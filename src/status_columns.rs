//! Agent status columns: values taken from an agent's last reply by
//! `[[status_column]]` regexes in config.toml, shown by `hcom agent list --status`.
//!
//! Values are stored per instance in the kv table, so they outlive the
//! instance row and need no schema change. Nothing here runs unless at least
//! one column is configured.

use regex::{Regex, RegexBuilder};
use serde_json::{Value, json};

use crate::db::HcomDb;
use crate::shared::time::now_epoch_i64;
use crate::shared::{ST_ACTIVE, ST_LISTENING};

pub const KEY_PREFIX: &str = "status_columns:";

/// Stored values of one instance: column name -> {value, updated_at}.
pub type StoredColumns = serde_json::Map<String, Value>;

/// Tools whose Stop hook passes the reply text (`last_assistant_message`);
/// every other tool falls back to its transcript.
const PAYLOAD_TOOLS: [&str; 2] = ["claude", "codex"];

pub struct Column {
    pub name: String,
    regex: Regex,
    /// Clear the value when the agent starts a new turn.
    reset_on_turn: bool,
}

/// Configured columns in config order, plus one message per invalid entry.
pub fn load() -> (Vec<Column>, Vec<String>) {
    match std::fs::read_to_string(crate::paths::config_toml_path()) {
        Ok(content) => parse(&content),
        Err(_) => (Vec::new(), Vec::new()),
    }
}

/// Configured columns; invalid entries are left out.
pub fn columns() -> Vec<Column> {
    load().0
}

fn parse(content: &str) -> (Vec<Column>, Vec<String>) {
    let mut columns: Vec<Column> = Vec::new();
    let mut errors = Vec::new();
    let Ok(table) = content.parse::<toml::Table>() else {
        return (columns, errors); // reported by the config loader
    };
    let Some(entries) = table.get("status_column") else {
        return (columns, errors);
    };
    let Some(entries) = entries.as_array() else {
        errors.push("status_column must be an array of tables ([[status_column]])".to_string());
        return (columns, errors);
    };
    for (i, entry) in entries.iter().enumerate() {
        let n = i + 1;
        let name = entry.get("name").and_then(|v| v.as_str()).unwrap_or("");
        if name.is_empty() {
            errors.push(format!("status_column #{n}: name is missing"));
            continue;
        }
        if columns.iter().any(|c| c.name == name) {
            errors.push(format!("status_column '{name}': duplicate name"));
            continue;
        }
        let Some(pattern) = entry.get("regex").and_then(|v| v.as_str()) else {
            errors.push(format!("status_column '{name}': regex is missing"));
            continue;
        };
        let regex = match RegexBuilder::new(pattern).multi_line(true).build() {
            Ok(regex) => regex,
            Err(e) => {
                errors.push(format!("status_column '{name}': invalid regex: {e}"));
                continue;
            }
        };
        let reset_on_turn = match entry.get("reset").map(|v| v.as_str()) {
            None | Some(Some("keep")) => false,
            Some(Some("turn")) => true,
            Some(_) => {
                errors.push(format!(
                    "status_column '{name}': reset must be \"keep\" or \"turn\""
                ));
                continue;
            }
        };
        columns.push(Column {
            name: name.to_string(),
            regex,
            reset_on_turn,
        });
    }
    (columns, errors)
}

/// Value of the column's last match in text: capture group 1, or the whole
/// match without a group, trimmed. None when nothing matches.
fn extract(column: &Column, text: &str) -> Option<String> {
    let caps = column.regex.captures_iter(text).last()?;
    let m = caps.get(1).or_else(|| caps.get(0))?;
    Some(m.as_str().trim().to_string())
}

fn key(instance: &str) -> String {
    format!("{KEY_PREFIX}{instance}")
}

fn load_entry(db: &HcomDb, instance: &str) -> Value {
    db.kv_get(&key(instance))
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_else(|| json!({"agent": null, "columns": {}}))
}

/// Read-modify-write of one instance's entry inside a write scope.
fn update_entry(db: &HcomDb, instance: &str, f: impl FnOnce(&mut Value) -> bool) {
    let result = db.with_write_scope(|| {
        let mut entry = load_entry(db, instance);
        if !f(&mut entry) {
            return Ok(());
        }
        db.kv_set(&key(instance), Some(&entry.to_string()))
    });
    if let Err(e) = result {
        crate::log::log_error(
            "status_columns",
            "db.error",
            &format!("instance={instance} - {e}"),
        );
    }
}

/// Store the values the configured columns find in an agent's reply text.
pub fn record_reply(db: &HcomDb, instance: &str, text: &str) {
    let columns = columns();
    if columns.is_empty() || text.is_empty() {
        return;
    }
    record_with(db, instance, text, &columns);
}

fn record_with(db: &HcomDb, instance: &str, text: &str, columns: &[Column]) {
    let found: Vec<(&str, String)> = columns
        .iter()
        .filter_map(|c| extract(c, text).map(|v| (c.name.as_str(), v)))
        .collect();
    if found.is_empty() {
        return;
    }
    let agent = db
        .get_instance_full(instance)
        .ok()
        .flatten()
        .and_then(|row| row.catalog_agent_name());
    let now = now_epoch_i64();
    update_entry(db, instance, |entry| {
        if agent.is_some() {
            entry["agent"] = json!(agent);
        }
        for (name, value) in found {
            entry["columns"][name] = json!({"value": value, "updated_at": now});
        }
        true
    });
}

/// Status-change hook: a new turn clears `reset = "turn"` columns; going idle
/// takes the reply from the transcript for tools without a payload source.
pub fn on_status_change(db: &HcomDb, instance: &str, status: &str) {
    if status != ST_ACTIVE && status != ST_LISTENING {
        return;
    }
    let columns = columns();
    if columns.is_empty() {
        return;
    }
    if status == ST_ACTIVE {
        clear_turn_columns(db, instance, &columns);
        return;
    }
    let Some(row) = db.get_instance_full(instance).ok().flatten() else {
        return;
    };
    if PAYLOAD_TOOLS.contains(&row.tool.as_str()) || row.transcript_path.is_empty() {
        return;
    }
    let text = crate::commands::transcript::get_exchanges(
        &row.transcript_path,
        &row.tool,
        1,
        false,
        row.session_id.as_deref(),
        false,
    )
    .ok()
    .and_then(|exchanges| exchanges.into_iter().last())
    .map(|ex| {
        if ex.final_text.is_empty() {
            ex.action
        } else {
            ex.final_text
        }
    })
    .unwrap_or_default();
    if !text.is_empty() {
        record_with(db, instance, &text, &columns);
    }
}

fn clear_turn_columns(db: &HcomDb, instance: &str, columns: &[Column]) {
    let turn: Vec<&str> = columns
        .iter()
        .filter(|c| c.reset_on_turn)
        .map(|c| c.name.as_str())
        .collect();
    if turn.is_empty() || db.kv_get(&key(instance)).ok().flatten().is_none() {
        return;
    }
    update_entry(db, instance, |entry| {
        let Some(stored) = entry["columns"].as_object_mut() else {
            return false;
        };
        let before = stored.len();
        stored.retain(|name, _| !turn.contains(&name.as_str()));
        stored.len() != before
    });
}

/// All stored entries: (instance, catalog agent, column name -> {value, updated_at}).
pub fn stored(db: &HcomDb) -> Vec<(String, Option<String>, StoredColumns)> {
    db.kv_prefix(KEY_PREFIX)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(k, raw)| {
            let instance = k.strip_prefix(KEY_PREFIX)?.to_string();
            let entry: Value = serde_json::from_str(&raw).ok()?;
            let agent = entry["agent"].as_str().map(str::to_string);
            let columns = entry["columns"].as_object().cloned().unwrap_or_default();
            Some((instance, agent, columns))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: &str = r#"
[[status_column]]
name = "status"
regex = '^STATUS:\s*(\S+)'

[[status_column]]
name = "next"
regex = '^Next:\s*(.*)$'
reset = "turn"
"#;

    #[test]
    fn parse_keeps_config_order_and_reset() {
        let (columns, errors) = parse(CONFIG);
        assert!(errors.is_empty(), "{errors:?}");
        let names: Vec<_> = columns.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["status", "next"]);
        assert!(!columns[0].reset_on_turn);
        assert!(columns[1].reset_on_turn);
    }

    #[test]
    fn parse_reports_invalid_entries_and_keeps_valid_ones() {
        let (columns, errors) = parse(
            r#"
[[status_column]]
name = "bad"
regex = '('

[[status_column]]
regex = 'x'

[[status_column]]
name = "r"
regex = 'x'
reset = "never"

[[status_column]]
name = "ok"
regex = 'x'
"#,
        );
        assert_eq!(columns.len(), 1);
        assert_eq!(columns[0].name, "ok");
        assert_eq!(errors.len(), 3, "{errors:?}");
        assert!(errors[0].contains("invalid regex"));
        assert!(errors[1].contains("name is missing"));
        assert!(errors[2].contains("reset must be"));
    }

    #[test]
    fn parse_without_columns_is_empty() {
        let (columns, errors) = parse("terminal = \"default\"\n");
        assert!(columns.is_empty());
        assert!(errors.is_empty());
    }

    #[test]
    fn extract_takes_group_of_last_match_multiline() {
        let (columns, _) = parse(CONFIG);
        let text = "STATUS: DONE\nearlier\n\nSTATUS: WAITING\nDone: x\nNext:  ask bin  \n";
        assert_eq!(extract(&columns[0], text).as_deref(), Some("WAITING"));
        assert_eq!(extract(&columns[1], text).as_deref(), Some("ask bin"));
        assert_eq!(extract(&columns[0], "no block"), None);
    }

    #[test]
    fn extract_without_group_uses_whole_match() {
        let (columns, _) = parse("[[status_column]]\nname = \"n\"\nregex = 'NEEDS_\\w+'\n");
        assert_eq!(
            extract(&columns[0], "STATUS: NEEDS_USER").as_deref(),
            Some("NEEDS_USER")
        );
    }

    fn values(db: &HcomDb, instance: &str) -> Vec<(String, String)> {
        let entry = load_entry(db, instance);
        entry["columns"]
            .as_object()
            .map(|m| {
                m.iter()
                    .map(|(k, v)| (k.clone(), v["value"].as_str().unwrap_or("").to_string()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// A tool without a payload source gets its values from the transcript
    /// when it goes idle; the next turn clears reset = "turn" columns.
    #[test]
    #[serial_test::serial]
    fn idle_reads_transcript_and_new_turn_clears_turn_columns() {
        use crate::instance_lifecycle::set_status;
        let (dir, hcom_dir, _home, _guard) = crate::hooks::test_helpers::isolated_test_env();
        std::fs::write(hcom_dir.join("config.toml"), CONFIG).unwrap();
        let transcript = dir.path().join("pi.jsonl");
        std::fs::write(
            &transcript,
            concat!(
                r#"{"type":"session","id":"abc"}"#,
                "\n",
                r#"{"type":"message","message":{"role":"user","content":"hello"}}"#,
                "\n",
                r#"{"type":"message","message":{"role":"assistant","content":"ok\n\nSTATUS: NEEDS_USER\nNext: approve"}}"#,
                "\n"
            ),
        )
        .unwrap();
        let db = HcomDb::open_raw(&hcom_dir.join("test.db")).unwrap();
        db.init_db().unwrap();
        db.conn()
            .execute(
                "INSERT INTO instances (name, tool, status, transcript_path, created_at)
                 VALUES ('luna', 'pi', 'active', ?, 0)",
                [transcript.to_string_lossy()],
            )
            .unwrap();

        set_status(&db, "luna", ST_LISTENING, "", Default::default());
        assert_eq!(
            values(&db, "luna"),
            [
                ("next".to_string(), "approve".to_string()),
                ("status".to_string(), "NEEDS_USER".to_string())
            ]
        );
        set_status(&db, "luna", ST_ACTIVE, "", Default::default());
        assert_eq!(
            values(&db, "luna"),
            [("status".to_string(), "NEEDS_USER".to_string())]
        );

        // Without configured columns nothing is read or stored.
        std::fs::write(hcom_dir.join("config.toml"), "").unwrap();
        db.kv_delete_prefix(KEY_PREFIX).unwrap();
        set_status(&db, "luna", ST_LISTENING, "", Default::default());
        record_reply(&db, "luna", "STATUS: DONE");
        assert!(db.kv_prefix(KEY_PREFIX).unwrap().is_empty());
    }

    #[test]
    fn keep_survives_a_reply_without_match_and_turn_is_cleared() {
        let dir = tempfile::tempdir().unwrap();
        let db = HcomDb::open_raw(&dir.path().join("test.db")).unwrap();
        db.init_db().unwrap();
        let (columns, _) = parse(CONFIG);
        record_with(&db, "luna", "STATUS: WAITING\nNext: wait for bin", &columns);
        record_with(&db, "luna", "ok, noted", &columns);
        assert_eq!(
            values(&db, "luna"),
            [
                ("next".to_string(), "wait for bin".to_string()),
                ("status".to_string(), "WAITING".to_string())
            ]
        );
        clear_turn_columns(&db, "luna", &columns);
        assert_eq!(
            values(&db, "luna"),
            [("status".to_string(), "WAITING".to_string())]
        );
        record_with(&db, "luna", "STATUS: DONE", &columns);
        assert_eq!(
            values(&db, "luna"),
            [("status".to_string(), "DONE".to_string())]
        );
    }
}
