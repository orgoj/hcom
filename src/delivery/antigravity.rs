//! Antigravity (agy) PTY prompt delivery and exit cleanup.
//!
//! Delivery runs through the shared state machine in `delivery.rs`, but the PTY
//! types the message bodies themselves ([`build_antigravity_prompt`]) and commits
//! the ack once agy takes the prompt; hooks deliver nothing while a PTY endpoint
//! exists. The teardown path is special too: SessionEnd may have already
//! soft-stopped the instance, in which case we must not re-delete its row.

use crate::db::HcomDb;
use crate::hooks::DeliveryAck;
use crate::log::{log_info, log_warn};
use crate::shared::ST_INACTIVE;

/// Longest prompt hcom types into agy. agy 1.5.9 re-wraps the whole input per
/// typed character (~330 chars/s, slower as it grows), so longer text would miss
/// the render check under load; the cut-off rest is left to `hcom events`.
const AGY_PROMPT_MAX_CHARS: usize = 1500;

const AGY_REQUEST_NOTE: &str =
    " [intent=request: do the work and hcom send the result before ending this turn]";

/// Build the prompt that carries pending messages to agy, with the ack to
/// commit once agy has started a turn from it.
///
/// agy shows a hook's `ephemeralMessage` to one model call only, so a body
/// delivered by hooks is gone from the conversation after that call. Typed into
/// the prompt, it is a persisted user turn. The text is one line (a newline
/// would submit early); `@` and `>` pass unchanged (agy 1.5.9 keeps them).
/// Messages that do not fit stay pending for the next turn. Delivery exception:
/// a single message longer than the limit is acked cut, with a pointer to its
/// full text; left pending it could never render in time and would stall.
pub(crate) fn build_antigravity_prompt(db: &HcomDb, name: &str) -> Option<(String, DeliveryAck)> {
    let mut prepared = crate::hooks::common::prepare_pending_messages(db, name)?;
    let messages = &prepared.messages;
    let field = |m: &serde_json::Value, key: &str| m.get(key).cloned();
    let mut count = messages.len();
    let text = loop {
        let batch = &messages[..count];
        let is_request = batch
            .iter()
            .any(|m| m.get("intent").and_then(|v| v.as_str()) == Some("request"));
        let note = if is_request { AGY_REQUEST_NOTE } else { "" };
        let body = one_line(&crate::hooks::common::format_messages_json_for_instance(
            db, batch, name,
        ));
        if body.chars().count() + note.chars().count() <= AGY_PROMPT_MAX_CHARS {
            break format!("{body}{note}");
        }
        if count == 1 {
            let id = field(&batch[0], "event_id").and_then(|v| v.as_i64());
            break cut_prompt(&body, note, id.unwrap_or(0));
        }
        count -= 1;
    };
    let last = &messages[count - 1];
    if let Some(id) = field(last, "event_id").and_then(|v| v.as_i64()) {
        prepared.ack.last_event_id = id;
    }
    if let Some(ts) = field(last, "timestamp").and_then(|v| v.as_str().map(String::from)) {
        prepared.ack.msg_ts = ts;
    }
    Some((text, prepared.ack))
}

/// Commit the typed prompt's ack once agy started a turn from it. The caller
/// drops the ack when the prompt cleared without its Enter (a cancel); agy's
/// turn-start hook (gemini-beforeagent) then sets active:prompt.
pub(crate) fn ack_prompt_once_turn_started(
    db: &HcomDb,
    name: &str,
    prompt_ack: &mut Option<DeliveryAck>,
) {
    if prompt_ack.is_some()
        && db
            .get_status(name)
            .ok()
            .flatten()
            .is_some_and(|(status, _)| status == crate::shared::ST_ACTIVE)
        && let Some(ack) = prompt_ack.take()
    {
        crate::hooks::common::commit_delivery_ack(db, &ack);
    }
}

fn one_line(text: &str) -> String {
    text.trim().replace(['\r', '\n', '\t'], " ")
}

fn cut_prompt(body: &str, note: &str, id: i64) -> String {
    let cut = format!(" … [truncated; full text: hcom events --sql \"id={id}\"]{note}");
    let keep = AGY_PROMPT_MAX_CHARS.saturating_sub(cut.chars().count());
    let head: String = body.chars().take(keep).collect();
    format!("{}{cut}", head.trim_end())
}

/// PTY exit cleanup when SessionEnd already soft-stopped the instance.
pub(crate) fn cleanup_antigravity_pty_exit(
    db: &mut HcomDb,
    current_name: &str,
    process_id: &str,
    owns_instance: bool,
) {
    if !owns_instance {
        super::log_pty_cleanup_skipped(db, current_name);
    } else {
        let already_inactive = db
            .get_status(current_name)
            .ok()
            .flatten()
            .is_some_and(|(status, _)| status == ST_INACTIVE);

        if already_inactive {
            log_info(
                "native",
                "delivery.cleanup_soft_stopped",
                &format!(
                    "Instance {} already inactive (soft SessionEnd); skipping delete_instance",
                    current_name
                ),
            );
            if let Err(e) = db.delete_notify_endpoints(current_name) {
                log_warn(
                    "native",
                    "delivery.cleanup_endpoints_fail",
                    &format!("{}", e),
                );
            }
            // Subscriptions stay with the kept row. Its hard stop (dead-process
            // or stale-row cleanup, or `hcom r`) saves them for resume and
            // removes them.
        } else {
            super::cleanup_deleted_instance(db, current_name);
        }
    }

    if !process_id.is_empty()
        && let Err(e) = db.delete_process_binding(process_id)
    {
        log_warn("native", "delivery.cleanup_binding_fail", &format!("{}", e));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db_with_messages(texts: &[&str]) -> (tempfile::TempDir, HcomDb, Vec<i64>) {
        let dir = tempfile::tempdir().unwrap();
        let db = HcomDb::open_raw(&dir.path().join("test.db")).unwrap();
        db.init_db().unwrap();
        db.conn()
            .execute(
                "INSERT INTO instances (name, status, created_at, tool) VALUES ('vago', 'listening', 1.0, 'antigravity')",
                [],
            )
            .unwrap();
        let ids = texts
            .iter()
            .map(|text| {
                let data = serde_json::json!({"from": "homo", "text": text, "scope": "broadcast"});
                db.conn()
                    .execute(
                        "INSERT INTO events (type, timestamp, instance, data) VALUES ('message', '2026-01-01T00:00:01Z', 'homo', ?1)",
                        [data.to_string()],
                    )
                    .unwrap();
                db.conn().last_insert_rowid()
            })
            .collect();
        (dir, db, ids)
    }

    #[test]
    fn prompt_keeps_message_text_verbatim_on_one_line() {
        let (_dir, db, ids) = db_with_messages(&["run hcom send @luna -- hi\n> quoted  a >= 2"]);
        let (text, ack) = build_antigravity_prompt(&db, "vago").unwrap();
        assert!(!text.contains('\n'), "{text}");
        assert!(
            text.contains("run hcom send @luna -- hi > quoted  a >= 2"),
            "{text}"
        );
        assert_eq!(ack.last_event_id, ids[0]);
    }

    #[test]
    fn messages_that_do_not_fit_stay_pending() {
        let long = "x".repeat(AGY_PROMPT_MAX_CHARS - 50);
        let (_dir, db, ids) = db_with_messages(&["short one", &long]);
        let (text, ack) = build_antigravity_prompt(&db, "vago").unwrap();
        assert!(
            text.contains("short one") && !text.contains(&long),
            "{text}"
        );
        assert_eq!(ack.last_event_id, ids[0]);
    }

    #[test]
    fn a_single_long_message_is_cut_with_a_pointer_to_its_text() {
        let long = "word ".repeat(AGY_PROMPT_MAX_CHARS);
        let (_dir, db, ids) = db_with_messages(&[&long]);
        let (text, ack) = build_antigravity_prompt(&db, "vago").unwrap();
        assert!(text.chars().count() <= AGY_PROMPT_MAX_CHARS);
        assert!(text.ends_with(&format!(
            " … [truncated; full text: hcom events --sql \"id={}\"]",
            ids[0]
        )));
        assert_eq!(ack.last_event_id, ids[0]);
    }

    /// A cleared prompt while agy stays listening (user cancel) must not mark
    /// the typed messages read; the turn-start status does.
    #[test]
    fn prompt_ack_waits_for_the_turn_start() {
        let (_dir, db, ids) = db_with_messages(&["hello"]);
        let (_, ack) = build_antigravity_prompt(&db, "vago").unwrap();
        let mut prompt_ack = Some(ack);

        ack_prompt_once_turn_started(&db, "vago", &mut prompt_ack);
        assert!(prompt_ack.is_some());
        assert_eq!(db.get_unread_messages("vago").len(), 1);

        // A session end is not a turn start.
        db.conn()
            .execute(
                "UPDATE instances SET status = 'inactive' WHERE name = 'vago'",
                [],
            )
            .unwrap();
        ack_prompt_once_turn_started(&db, "vago", &mut prompt_ack);
        assert!(prompt_ack.is_some());

        db.conn()
            .execute(
                "UPDATE instances SET status = 'active' WHERE name = 'vago'",
                [],
            )
            .unwrap();
        ack_prompt_once_turn_started(&db, "vago", &mut prompt_ack);
        assert!(prompt_ack.is_none());
        assert_eq!(db.get_cursor("vago"), ids[0]);
        assert!(db.get_unread_messages("vago").is_empty());
    }
}
