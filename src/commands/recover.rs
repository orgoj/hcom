//! `hcom recover` — resume agents whose process vanished (crash, power loss).
//!
//! A candidate is a local agent whose latest stop event has reason
//! `process_exit` (the tracked process disappeared without an exit hook), that
//! is not running now, and that stopped within the last `--hours`.

use std::collections::HashSet;

use anyhow::Result;

use crate::db::HcomDb;
use crate::instance_lifecycle::cleanup_stale_instances;
use crate::router::GlobalFlags;
use crate::shared::shorten_path_max;
use crate::shared::time::{format_age, now_epoch_f64};

/// Parsed arguments for `hcom recover`.
#[derive(clap::Parser, Debug)]
#[command(name = "recover", about = "Resume agents whose process vanished")]
pub struct RecoverArgs {
    /// Agents to resume; without names, list the candidates
    #[arg(conflicts_with = "all")]
    pub names: Vec<String>,
    /// Resume every candidate
    #[arg(long)]
    pub all: bool,
    /// Look back this many hours
    #[arg(long, default_value_t = 24)]
    pub hours: u64,
    /// List candidates as JSON
    #[arg(long)]
    pub json: bool,
}

/// An agent interrupted by a vanished process.
#[derive(serde::Serialize, Debug, PartialEq)]
pub struct Candidate {
    pub name: String,
    pub tool: String,
    pub stopped_at: String,
    /// hcom status before the stop (`listening`, `active`, `blocked`, ...).
    pub last_status: String,
    pub directory: String,
    pub session_id: String,
    pub transcript_path: String,
    /// Stored `[[status_column]]` values: column name -> value.
    pub status_columns: serde_json::Map<String, serde_json::Value>,
}

/// Stored status column values of `name` (see `crate::status_columns`).
fn status_column_values(db: &HcomDb, name: &str) -> serde_json::Map<String, serde_json::Value> {
    let key = format!("{}{name}", crate::status_columns::KEY_PREFIX);
    let entry: serde_json::Value = db
        .kv_get(&key)
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default();
    entry["columns"]
        .as_object()
        .into_iter()
        .flatten()
        .map(|(k, v)| (k.clone(), v["value"].clone()))
        .collect()
}

/// Agents whose latest stop is `process_exit` within `hours` and that are not running.
pub fn candidates(db: &HcomDb, hours: u64) -> Vec<Candidate> {
    let cutoff = now_epoch_f64() as i64 - (hours as i64) * 3600;
    let cutoff_ts = chrono::DateTime::from_timestamp(cutoff, 0)
        .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S").to_string())
        .unwrap_or_default();
    let Ok(mut stmt) = db.conn().prepare(
        "SELECT id, instance, timestamp, data FROM events
         WHERE type = 'life' AND json_extract(data, '$.action') = 'stopped'
         AND timestamp > ?
         ORDER BY id DESC",
    ) else {
        return vec![];
    };
    let rows: Vec<(i64, String, String, String)> = stmt
        .query_map([cutoff_ts], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|r| r.ok())
        .collect();

    let mut seen = HashSet::new();
    let mut out = vec![];
    // Newest first: the first stop seen per instance is its latest one.
    for (id, name, ts, data) in rows {
        if !seen.insert(name.clone()) {
            continue;
        }
        let data: serde_json::Value = serde_json::from_str(&data).unwrap_or_default();
        let snapshot = &data["snapshot"];
        let remote = snapshot["origin_device_id"]
            .as_str()
            .is_some_and(|d| !d.is_empty());
        if data["reason"] != "process_exit" || remote {
            continue;
        }
        if !matches!(db.get_instance_full(&name), Ok(None)) {
            continue;
        }
        let last_status = db
            .conn()
            .query_row(
                "SELECT json_extract(data, '$.status') FROM events
                 WHERE type = 'status' AND instance = ? AND id < ?
                 ORDER BY id DESC LIMIT 1",
                rusqlite::params![name, id],
                |r| r.get::<_, Option<String>>(0),
            )
            .ok()
            .flatten()
            .unwrap_or_default();
        let field = |k: &str| snapshot[k].as_str().unwrap_or("").to_string();
        out.push(Candidate {
            status_columns: status_column_values(db, &name),
            tool: field("tool"),
            directory: field("directory"),
            session_id: field("session_id"),
            transcript_path: field("transcript_path"),
            name,
            stopped_at: ts,
            last_status,
        });
    }
    out
}

/// Run `hcom recover`. `argv` is the full argv[1..].
pub fn run(argv: &[String], flags: &GlobalFlags) -> Result<i32> {
    let mut filtered = vec!["recover".to_string()];
    let mut skip_next = false;
    for arg in argv {
        if skip_next {
            skip_next = false;
            continue;
        }
        match arg.as_str() {
            "recover" | "--go" => continue,
            "--name" => skip_next = true,
            _ => filtered.push(arg.clone()),
        }
    }
    use clap::Parser;
    let args = match RecoverArgs::try_parse_from(&filtered) {
        Ok(a) => a,
        Err(e) => {
            e.print().ok();
            return Ok(if e.use_stderr() { 1 } else { 0 });
        }
    };

    let db = HcomDb::open()?;
    let _ = cleanup_stale_instances(&db, 3600, 3600);

    let names: Vec<String> = if args.all {
        candidates(&db, args.hours)
            .into_iter()
            .map(|c| c.name)
            .collect()
    } else if !args.names.is_empty() {
        args.names.clone()
    } else {
        print_candidates(&candidates(&db, args.hours), args.hours, args.json);
        return Ok(0);
    };
    drop(db);

    let mut failed = 0;
    for name in &names {
        match super::resume::do_resume(name, false, &[], flags) {
            Ok(0) => {}
            Ok(_) => failed += 1,
            Err(e) => {
                eprintln!("Error: {name}: {e}");
                failed += 1;
            }
        }
    }
    if names.is_empty() {
        println!("No interrupted agents in the last {}h", args.hours);
    }
    Ok(if failed > 0 { 1 } else { 0 })
}

fn print_candidates(list: &[Candidate], hours: u64, json: bool) {
    if json {
        println!(
            "{}",
            serde_json::to_string(list).unwrap_or_else(|_| "[]".into())
        );
        return;
    }
    if list.is_empty() {
        println!("No interrupted agents in the last {hours}h");
        return;
    }
    println!("Interrupted agents (process vanished, last {hours}h):\n");
    let now = now_epoch_f64();
    for c in list {
        let age = chrono::DateTime::parse_from_rfc3339(&c.stopped_at)
            .map(|dt| format!("{} ago", format_age((now - dt.timestamp() as f64) as i64)))
            .unwrap_or_else(|_| c.stopped_at.clone());
        let status = if c.last_status.is_empty() {
            String::new()
        } else {
            format!("  last:{}", c.last_status)
        };
        println!(
            "  {} ({}) {age}{status}  {}",
            c.name,
            c.tool,
            shorten_path_max(&c.directory, 40)
        );
    }
    println!("\n  Resume: hcom recover <name>...  |  hcom recover --all");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::HcomDb;

    fn stop(db: &HcomDb, name: &str, by: &str, reason: &str, snapshot: serde_json::Value) {
        db.log_life_event(name, "stopped", by, reason, Some(snapshot))
            .unwrap();
    }

    #[test]
    fn candidates_keep_only_vanished_local_stopped_agents() {
        let dir = tempfile::tempdir().unwrap();
        let db = HcomDb::open_raw(&dir.path().join("test.db")).unwrap();
        db.init_db().unwrap();
        let snap =
            |dir: &str| serde_json::json!({"tool": "claude", "directory": dir, "session_id": "s1"});
        db.log_event("status", "luna", &serde_json::json!({"status": "blocked"}))
            .unwrap();
        stop(&db, "luna", "system", "process_exit", snap("/a"));
        stop(&db, "kill", "cli", "killed", snap("/b"));
        stop(&db, "redo", "system", "process_exit", snap("/c"));
        stop(&db, "redo", "cli", "killed", snap("/c"));
        stop(
            &db,
            "away",
            "system",
            "process_exit",
            serde_json::json!({"tool": "claude", "origin_device_id": "dev1"}),
        );
        // Running again: its instance row exists.
        stop(&db, "back", "system", "process_exit", snap("/d"));
        db.conn()
            .execute(
                "INSERT INTO instances (name, status, created_at, tool) VALUES ('back', 'listening', 0, 'claude')",
                [],
            )
            .unwrap();
        // Outside the look-back window.
        db.log_event_with_ts(
            "life",
            "old1",
            &serde_json::json!({"action": "stopped", "by": "system", "reason": "process_exit", "snapshot": snap("/e")}),
            Some("2020-01-01T00:00:00+00:00"),
        )
        .unwrap();
        db.kv_set(
            "status_columns:luna",
            Some(r#"{"columns":{"status":{"value":"NEEDS_USER","updated_at":1}}}"#),
        )
        .unwrap();

        let got = candidates(&db, 24);
        assert_eq!(got.len(), 1, "{got:?}");
        assert_eq!(got[0].name, "luna");
        assert_eq!(got[0].last_status, "blocked");
        assert_eq!(got[0].directory, "/a");
        assert_eq!(got[0].session_id, "s1");
        assert_eq!(got[0].status_columns["status"], "NEEDS_USER");
    }
}
