//! Idle autokill: kill an agent once it has idled for its `idle_kill` seconds.
//!
//! The value is stored per instance name in `kv` as `idle_kill:<name>` =
//! `{"secs": N, "set_at": <epoch>}`, written by a catalog launch or by
//! `hcom config -i <name> idle_kill N`. Any write replaces the whole value,
//! which also re-arms a fired policy.
//!
//! Every long-running hcom loop (PTY delivery, hook poll, `hcom listen`) and
//! every CLI command runs [`sweep_throttled`]; while any key exists those loops
//! cap their waits with [`cap_wait`], so one running loop enforces the policy
//! for every agent. The kill itself runs in a detached worker process
//! (`hcom __idle-kill`): the sweep may run inside the victim's own process group.

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use rusqlite::OptionalExtension;
use serde_json::{Value, json};

use crate::db::{HcomDb, InstanceRow};
use crate::shared::time::now_epoch_f64;
use crate::shared::{ST_INACTIVE, ST_LISTENING};
use crate::terminal::{self, KillResult};

const KEY_PREFIX: &str = "idle_kill:";
/// First key after every `idle_kill:` key (`;` follows `:`), for range scans:
/// `_` in the prefix is a LIKE wildcard.
const KEY_END: &str = "idle_kill;";
const SWEEP_KV: &str = "idle_kill_sweep_ts";
const SWEEP_INTERVAL_SECS: f64 = 5.0;
const WAIT_CAP: Duration = Duration::from_secs(5);
/// A catalog launch writes the key before its instance row exists.
const NO_ROW_GRACE_SECS: f64 = 120.0;
/// A fired kill that left the same incarnation running is retried after this.
const FIRING_EXPIRY_SECS: f64 = 60.0;
/// Initiator recorded on the stopped event.
const INITIATOR: &str = "idle_kill";
/// Internal argv[1] of the detached kill worker (see [`run_worker`]).
pub const WORKER_COMMAND: &str = "__idle-kill";

fn key(name: &str) -> String {
    format!("{KEY_PREFIX}{name}")
}

/// Parse a user-supplied `idle_kill` value: whole seconds, `0` disables.
pub fn parse_secs(value: &str) -> Result<u64, String> {
    value
        .trim()
        .parse::<u64>()
        .map_err(|_| format!("idle_kill must be a non-negative integer (seconds), got '{value}'"))
}

/// Store `secs` for `name` (`0` clears it) and wake every waiting loop so it
/// picks up the shorter wait cap now instead of after its current wait.
pub fn set(db: &HcomDb, name: &str, secs: u64) -> anyhow::Result<()> {
    if secs == 0 {
        db.kv_set(&key(name), None)?;
    } else {
        let value = json!({"secs": secs, "set_at": now_epoch_f64()}).to_string();
        db.kv_set(&key(name), Some(&value))?;
    }
    crate::notify::wake_all(db);
    Ok(())
}

/// Stored seconds for `name`, `None` when unset.
pub fn get(db: &HcomDb, name: &str) -> Option<u64> {
    let raw = db.kv_get(&key(name)).ok()??;
    serde_json::from_str::<Value>(&raw)
        .ok()?
        .get("secs")?
        .as_u64()
}

fn any_key(db: &HcomDb) -> bool {
    db.conn()
        .query_row(
            "SELECT 1 FROM kv WHERE key >= ? AND key < ? LIMIT 1",
            rusqlite::params![KEY_PREFIX, KEY_END],
            |_| Ok(()),
        )
        .is_ok()
}

/// Shorten a loop's wait while any agent has an `idle_kill` value, so the
/// sweep runs within seconds of an agent becoming eligible.
pub fn cap_wait(db: &HcomDb, wait: Duration) -> Duration {
    if wait > WAIT_CAP && any_key(db) {
        WAIT_CAP
    } else {
        wait
    }
}

/// Run [`sweep`] at most once per interval across all hcom processes.
pub fn sweep_throttled(db: &HcomDb) {
    let now = now_epoch_f64();
    let last = db
        .kv_get(SWEEP_KV)
        .ok()
        .flatten()
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.0);
    // A last-sweep time in the future means the clock moved backwards.
    if last <= now && now - last < SWEEP_INTERVAL_SECS {
        return;
    }
    if !any_key(db) {
        return;
    }
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    // Concurrent duplicate sweeps are harmless: every claim is a compare-and-swap.
    let _ = db.kv_set(SWEEP_KV, Some(&now.to_string()));
    sweep(db, now, &exe);
}

/// Identifies one process incarnation of a row: PID reuse alone must not let a
/// stale firing state touch a relaunched agent.
fn incarnation(row: &InstanceRow) -> String {
    let token = row
        .pid_identity()
        .unwrap_or_else(|| row.created_at.to_string());
    format!("{}:{token}", row.pid.unwrap_or(0))
}

fn delete_if_unchanged(db: &HcomDb, key: &str, raw: &str) {
    let _ = db.conn().execute(
        "DELETE FROM kv WHERE key = ? AND value = ?",
        rusqlite::params![key, raw],
    );
}

fn sweep(db: &HcomDb, now: f64, exe: &Path) {
    let Ok(entries) = db.kv_prefix(KEY_PREFIX) else {
        return;
    };
    for (key, raw) in entries {
        let name = &key[KEY_PREFIX.len()..];
        let Ok(policy) = serde_json::from_str::<Value>(&raw) else {
            delete_if_unchanged(db, &key, &raw);
            continue;
        };
        let Some(secs) = policy.get("secs").and_then(Value::as_u64) else {
            delete_if_unchanged(db, &key, &raw);
            continue;
        };
        let row = match db.get_instance_full(name) {
            Ok(Some(row)) => row,
            Ok(None) => {
                let set_at = policy.get("set_at").and_then(Value::as_f64).unwrap_or(0.0);
                if policy.get("firing_at").is_some() || now - set_at > NO_ROW_GRACE_SECS {
                    delete_if_unchanged(db, &key, &raw);
                }
                continue;
            }
            Err(_) => continue,
        };
        let current = incarnation(&row);
        if let Some(fired) = policy.get("incarnation").and_then(Value::as_str) {
            if fired != current {
                // The fired incarnation is gone and a new one runs under the name.
                delete_if_unchanged(db, &key, &raw);
                continue;
            }
            let firing_at = policy
                .get("firing_at")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            if now - firing_at < FIRING_EXPIRY_SECS {
                continue;
            }
        }
        if !matches!(is_idle(db, &row, secs, now), Ok(true)) {
            continue;
        }
        if let Some(claimed) = claim(db, &key, &raw, name, &current, secs, now) {
            fire(
                db,
                exe,
                &key,
                &raw,
                &claimed,
                name,
                secs,
                now - row.status_time as f64,
            );
        }
    }
}

/// Whether `row` has idled for `secs`: listening that long with no gate (a
/// human at the prompt), no message delivered to it in that span, nothing
/// unread, and no own `intent=request` still waiting for a reply. Any read
/// error is an error, never "idle": the caller must not kill on it.
fn is_idle(db: &HcomDb, row: &InstanceRow, secs: u64, now: f64) -> anyhow::Result<bool> {
    if row.status != ST_LISTENING
        || row.status_context.starts_with("tui:")
        || row.pid.is_none()
        || row.origin_device_id.is_some()
        || row.parent_name.is_some()
        || crate::instances::is_launching_placeholder(row)
    {
        return Ok(false);
    }
    let since = now - secs as f64;
    if row.status_time as f64 > since {
        return Ok(false);
    }
    if !db.try_get_unread_messages(&row.name)?.is_empty() {
        return Ok(false);
    }
    let cutoff = chrono::DateTime::from_timestamp(since as i64, 0)
        .unwrap_or_default()
        .format("%Y-%m-%dT%H:%M:%S%.6f+00:00")
        .to_string();
    let recent_message = db
        .conn()
        .query_row(
            "SELECT 1 FROM events_v WHERE type = 'message' AND timestamp > ?
               AND EXISTS (SELECT 1 FROM json_each(msg_delivered_to) WHERE value = ?)
             LIMIT 1",
            rusqlite::params![cutoff, row.name],
            |_| Ok(()),
        )
        .optional()?;
    if recent_message.is_some() {
        return Ok(false);
    }
    // Request watches exist from an `intent=request` send until each recipient
    // replies or stops (see `create_request_watches`).
    let pending_request = db
        .conn()
        .query_row(
            "SELECT 1 FROM kv WHERE key LIKE 'events_sub:reqwatch-%'
               AND json_extract(value, '$.caller') = ? LIMIT 1",
            rusqlite::params![row.name],
            |_| Ok(()),
        )
        .optional()?;
    Ok(pending_request.is_none())
}

/// Mark the policy as firing, only if it still holds the evaluated value and
/// the row is still the evaluated incarnation and still idle. Returns the
/// stored value.
fn claim(
    db: &HcomDb,
    key: &str,
    raw: &str,
    name: &str,
    incarnation_seen: &str,
    secs: u64,
    now: f64,
) -> Option<String> {
    let mut claimed = serde_json::from_str::<Value>(raw).ok()?;
    claimed["firing_at"] = json!(now);
    claimed["incarnation"] = json!(incarnation_seen);
    let claimed = claimed.to_string();
    let won = db.with_write_scope(|| {
        let Some(row) = db.get_instance_full(name)? else {
            return Ok(false);
        };
        if incarnation(&row) != incarnation_seen || !is_idle(db, &row, secs, now)? {
            return Ok(false);
        }
        let changed = db.conn().execute(
            "UPDATE kv SET value = ? WHERE key = ? AND value = ?",
            rusqlite::params![claimed, key, raw],
        )?;
        Ok(changed == 1)
    });
    matches!(won, Ok(true)).then_some(claimed)
}

#[allow(clippy::too_many_arguments)]
fn fire(
    db: &HcomDb,
    exe: &Path,
    key: &str,
    raw: &str,
    claimed: &str,
    name: &str,
    secs: u64,
    idle: f64,
) {
    crate::log::log_info(
        "idle_kill",
        "idle_kill.fire",
        &format!("name={name} secs={secs} idle={idle:.0}s"),
    );
    let mut cmd = Command::new(exe);
    cmd.args([WORKER_COMMAND, name, claimed])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    crate::sys::process::detach_session(&mut cmd);
    match cmd.spawn() {
        Ok(mut child) => {
            // Reap it so a long-running loop does not keep a zombie.
            std::thread::spawn(move || child.wait());
        }
        Err(e) => {
            crate::log::log_warn(
                "idle_kill",
                "idle_kill.spawn_failed",
                &format!("name={name} exe={} err={e}", exe.display()),
            );
            // Re-arm only our own claim: a newer source write or claim wins.
            let _ = db.conn().execute(
                "UPDATE kv SET value = ? WHERE key = ? AND value = ?",
                rusqlite::params![raw, key, claimed],
            );
        }
    }
}

/// Entry point of the detached `hcom __idle-kill <name> <claimed value>`.
pub fn run_worker(args: &[String]) -> i32 {
    let [name, claimed] = args else {
        return 2;
    };
    let db = match HcomDb::open() {
        Ok(db) => db,
        Err(e) => {
            crate::log::log_warn("idle_kill", "idle_kill.worker_db", &e.to_string());
            return 1;
        }
    };
    match kill_claimed(&db, name, claimed, now_epoch_f64()) {
        Ok(true) => 0,
        Ok(false) => 1,
        Err(e) => {
            crate::log::log_warn(
                "idle_kill",
                "idle_kill.worker_failed",
                &format!("name={name} err={e}"),
            );
            1
        }
    }
}

/// Kill the instance a sweep claimed, and nothing else: no target syntax
/// (`all`, groups, tags, PIDs) is involved. The claim must still be stored,
/// and the row must still be the claimed incarnation and still idle. The kill
/// works from that row snapshot, so a later reuse of the name is untouched.
/// Mirrors `hcom kill <name>`: record the kill, signal, close the pane, stop.
fn kill_claimed(db: &HcomDb, name: &str, claimed: &str, now: f64) -> anyhow::Result<bool> {
    let policy: Value = serde_json::from_str(claimed)?;
    let secs = policy
        .get("secs")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("claim without secs"))?;
    let expected = policy
        .get("incarnation")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("claim without incarnation"))?;
    let snapshot = db.with_write_scope(|| {
        if db.kv_get(&key(name))?.as_deref() != Some(claimed) {
            return Ok(None);
        }
        let Some(row) = db.get_instance_full(name)? else {
            return Ok(None);
        };
        if incarnation(&row) != expected || !is_idle(db, &row, secs, now)? {
            return Ok(None);
        }
        // Recorded before the signal, like `hcom kill`: the PTY cleanup the
        // signal triggers then reports the stop as killed by idle_kill.
        db.conn().execute(
            "UPDATE instances SET status = ?, status_context = 'exit:killed',
                 status_detail = ?, status_time = ?
             WHERE name = ? AND pid IS ? AND created_at = ?",
            rusqlite::params![
                ST_INACTIVE,
                INITIATOR,
                now as i64,
                name,
                row.pid,
                row.created_at
            ],
        )?;
        Ok(Some(row))
    })?;
    let Some(row) = snapshot else {
        return Ok(false);
    };
    let pid = row.pid.unwrap_or_default() as u32;
    let result = if row.pid_reused(pid) {
        KillResult::AlreadyDead
    } else if row.background != 0 {
        terminal::kill_process(pid, &terminal::TerminalInfo::default()).0
    } else {
        let ti = terminal::resolve_terminal_info(
            row.terminal_preset_effective.as_deref(),
            row.launch_context.as_deref(),
        );
        terminal::kill_process(pid, &ti).0
    };
    crate::log::log_info(
        "idle_kill",
        "idle_kill.kill",
        &format!("name={name} pid={pid} result={result:?}"),
    );
    if result == KillResult::PermissionDenied {
        // Undo only our own marker; the policy retries after its expiry.
        db.conn().execute(
            "UPDATE instances SET status = ?, status_context = ?, status_detail = ?, status_time = ?
             WHERE name = ? AND pid IS ? AND created_at = ? AND status_context = 'exit:killed'
               AND status_detail = ?",
            rusqlite::params![
                row.status,
                row.status_context,
                row.status_detail,
                row.status_time,
                name,
                row.pid,
                row.created_at,
                INITIATOR
            ],
        )?;
        return Ok(false);
    }
    // Stop the row only while it still belongs to the killed incarnation.
    match row.pid_identity() {
        Some(identity) => {
            crate::hooks::common::stop_instance_if_pid_identity(
                db, name, INITIATOR, "killed", pid, &identity,
            );
        }
        None => {
            db.with_write_scope(|| {
                let same = db
                    .get_instance_full(name)?
                    .is_some_and(|current| incarnation(&current) == expected);
                if same {
                    crate::hooks::common::stop_instance(db, name, INITIATOR, "killed");
                }
                Ok(())
            })?;
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn setup_test_db() -> (HcomDb, PathBuf) {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let db_path = std::env::temp_dir().join(format!(
            "test_idle_kill_{}_{}.db",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        (HcomDb::open_at(&db_path).unwrap(), db_path)
    }

    fn cleanup(path: PathBuf) {
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("db-wal"));
        let _ = std::fs::remove_file(path.with_extension("db-shm"));
    }

    /// A PID above every platform's pid_max, so it names no live process.
    const DEAD_PID: i64 = 4_194_305;

    fn insert_listening(db: &HcomDb, name: &str, idle_for: i64) {
        let now = crate::shared::time::now_epoch_i64();
        db.conn()
            .execute(
                "INSERT INTO instances (name, tool, status, status_time, created_at, pid, session_id)
                 VALUES (?, 'claude', 'listening', ?, ?, ?, ?)",
                rusqlite::params![name, now - idle_for, 1.0, DEAD_PID, format!("s-{name}")],
            )
            .unwrap();
    }

    fn row(db: &HcomDb, name: &str) -> InstanceRow {
        db.get_instance_full(name).unwrap().unwrap()
    }

    fn raw(db: &HcomDb, name: &str) -> Option<String> {
        db.kv_get(&key(name)).unwrap()
    }

    #[test]
    fn parse_secs_accepts_whole_seconds_only() {
        assert_eq!(parse_secs("0"), Ok(0));
        assert_eq!(parse_secs("600"), Ok(600));
        for bad in ["-1", "1.5", "abc", ""] {
            let err = parse_secs(bad).unwrap_err();
            assert!(err.contains("non-negative integer"), "{bad}: {err}");
        }
    }

    #[test]
    fn set_get_and_zero_clears() {
        let (db, path) = setup_test_db();
        set(&db, "luna", 30).unwrap();
        assert_eq!(get(&db, "luna"), Some(30));
        set(&db, "luna", 0).unwrap();
        assert_eq!(get(&db, "luna"), None);
        assert!(!any_key(&db));
        cleanup(path);
    }

    #[test]
    fn cap_wait_caps_only_while_a_key_exists() {
        let (db, path) = setup_test_db();
        let long = Duration::from_secs(30);
        assert_eq!(cap_wait(&db, long), long);
        // A key that only matches the LIKE wildcard must not count.
        db.kv_set("idleXkill:luna", Some("{}")).unwrap();
        assert_eq!(cap_wait(&db, long), long);
        set(&db, "luna", 30).unwrap();
        assert_eq!(cap_wait(&db, long), WAIT_CAP);
        assert_eq!(
            cap_wait(&db, Duration::from_secs(1)),
            Duration::from_secs(1)
        );
        cleanup(path);
    }

    #[test]
    fn idle_requires_listening_age_and_no_gate() {
        let (db, path) = setup_test_db();
        let now = now_epoch_f64();
        insert_listening(&db, "luna", 100);
        assert!(is_idle(&db, &row(&db, "luna"), 60, now).unwrap());
        assert!(!is_idle(&db, &row(&db, "luna"), 200, now).unwrap());

        db.set_gate_status("luna", "tui:user-active", "user is typing")
            .unwrap();
        assert!(!is_idle(&db, &row(&db, "luna"), 60, now).unwrap());

        db.conn()
            .execute(
                "UPDATE instances SET status = 'blocked', status_context = '' WHERE name = 'luna'",
                [],
            )
            .unwrap();
        assert!(!is_idle(&db, &row(&db, "luna"), 60, now).unwrap());
        cleanup(path);
    }

    fn log_message(db: &HcomDb, from: &str, to: &str, intent: &str) -> i64 {
        db.log_event(
            "message",
            from,
            &json!({
                "from": from,
                "scope": "mentions",
                "mentions": [to],
                "delivered_to": [to],
                "intent": intent,
                "text": "hi",
            }),
        )
        .unwrap()
    }

    #[test]
    fn unread_or_recent_message_blocks_idle() {
        let (db, path) = setup_test_db();
        let now = now_epoch_f64();
        insert_listening(&db, "luna", 100);
        let id = log_message(&db, "nova", "luna", "inform");
        // Unread.
        assert!(!is_idle(&db, &row(&db, "luna"), 60, now).unwrap());
        // Read, but arrived inside the idle span: the clock restarts.
        db.conn()
            .execute(
                "UPDATE instances SET last_event_id = ? WHERE name = 'luna'",
                rusqlite::params![id],
            )
            .unwrap();
        assert!(!is_idle(&db, &row(&db, "luna"), 60, now).unwrap());
        // The same message seen from far enough in the future is old.
        assert!(is_idle(&db, &row(&db, "luna"), 60, now + 120.0).unwrap());
        cleanup(path);
    }

    #[test]
    fn pending_own_request_blocks_idle() {
        let (db, path) = setup_test_db();
        let now = now_epoch_f64();
        insert_listening(&db, "luna", 100);
        crate::db::subscriptions::create_request_watches(&db, "luna", 7, &["nova".to_string()]);
        assert!(!is_idle(&db, &row(&db, "luna"), 60, now).unwrap());
        // Someone else's request does not hold luna.
        db.conn()
            .execute("DELETE FROM kv WHERE key LIKE 'events_sub:reqwatch-%'", [])
            .unwrap();
        crate::db::subscriptions::create_request_watches(&db, "nova", 8, &["luna".to_string()]);
        assert!(is_idle(&db, &row(&db, "luna"), 60, now).unwrap());
        cleanup(path);
    }

    #[test]
    fn claim_marks_firing_once_and_keeps_the_key() {
        let (db, path) = setup_test_db();
        let now = now_epoch_f64();
        insert_listening(&db, "luna", 100);
        set(&db, "luna", 60).unwrap();
        let before = raw(&db, "luna").unwrap();
        let inc = incarnation(&row(&db, "luna"));
        let claimed =
            claim(&db, &key("luna"), &before, "luna", &inc, 60, now).expect("first claim");
        assert_eq!(raw(&db, "luna").as_deref(), Some(claimed.as_str()));
        let stored: Value = serde_json::from_str(&claimed).unwrap();
        assert_eq!(stored["incarnation"], inc.as_str());
        assert_eq!(stored["secs"], 60);
        // A second claimer that evaluated the same value loses.
        assert!(claim(&db, &key("luna"), &before, "luna", &inc, 60, now).is_none());
        cleanup(path);
    }

    #[test]
    fn stale_claim_loses_to_a_policy_rewrite_or_rebind() {
        let (db, path) = setup_test_db();
        let now = now_epoch_f64();
        insert_listening(&db, "luna", 100);
        set(&db, "luna", 1).unwrap();
        let evaluated = raw(&db, "luna").unwrap();
        let inc = incarnation(&row(&db, "luna"));

        // A source rewrites the policy between eligibility and claim.
        set(&db, "luna", 100).unwrap();
        let rewritten = raw(&db, "luna").unwrap();
        assert!(claim(&db, &key("luna"), &evaluated, "luna", &inc, 60, now).is_none());
        assert_eq!(raw(&db, "luna").as_deref(), Some(rewritten.as_str()));

        // The row is rebound to a new incarnation under the same PID.
        db.conn()
            .execute(
                "UPDATE instances SET created_at = 2.0 WHERE name = 'luna'",
                [],
            )
            .unwrap();
        assert!(claim(&db, &key("luna"), &rewritten, "luna", &inc, 60, now).is_none());
        assert_eq!(raw(&db, "luna").as_deref(), Some(rewritten.as_str()));
        cleanup(path);
    }

    #[test]
    fn spawn_failure_rearms_only_its_own_claim() {
        let (db, path) = setup_test_db();
        let now = now_epoch_f64();
        insert_listening(&db, "luna", 100);
        set(&db, "luna", 60).unwrap();
        let armed = raw(&db, "luna").unwrap();
        let missing = Path::new("/nonexistent/hcom-idle-kill-test");

        sweep(&db, now, missing);
        assert_eq!(raw(&db, "luna").as_deref(), Some(armed.as_str()));

        // A newer source write landed after the claim: the failed worker must
        // not overwrite it.
        let inc = incarnation(&row(&db, "luna"));
        let claimed = claim(&db, &key("luna"), &armed, "luna", &inc, 60, now).unwrap();
        set(&db, "luna", 90).unwrap();
        let newer = raw(&db, "luna").unwrap();
        fire(
            &db,
            missing,
            &key("luna"),
            &armed,
            &claimed,
            "luna",
            60,
            100.0,
        );
        assert_eq!(raw(&db, "luna").as_deref(), Some(newer.as_str()));
        cleanup(path);
    }

    #[test]
    fn fired_policy_retries_after_expiry_for_the_same_incarnation() {
        let (db, path) = setup_test_db();
        let now = now_epoch_f64();
        insert_listening(&db, "luna", 100);
        set(&db, "luna", 60).unwrap();
        let armed = raw(&db, "luna").unwrap();
        let inc = incarnation(&row(&db, "luna"));
        let claimed = claim(&db, &key("luna"), &armed, "luna", &inc, 60, now).unwrap();
        let missing = Path::new("/nonexistent/hcom-idle-kill-test");

        // Inside the expiry the fired policy is left alone.
        sweep(&db, now + 10.0, missing);
        assert_eq!(raw(&db, "luna").as_deref(), Some(claimed.as_str()));
        // After it, the kill evidently failed: fire again.
        let retry = now + FIRING_EXPIRY_SECS + 1.0;
        sweep(&db, retry, Path::new("/bin/true"));
        let stored: Value = serde_json::from_str(&raw(&db, "luna").unwrap()).unwrap();
        assert_eq!(stored["firing_at"].as_f64(), Some(retry));
        assert_eq!(stored["incarnation"], inc.as_str());
        cleanup(path);
    }

    #[test]
    fn fired_key_of_a_previous_incarnation_is_deleted() {
        let (db, path) = setup_test_db();
        let now = now_epoch_f64();
        insert_listening(&db, "luna", 100);
        set(&db, "luna", 60).unwrap();
        let armed = raw(&db, "luna").unwrap();
        let inc = incarnation(&row(&db, "luna"));
        claim(&db, &key("luna"), &armed, "luna", &inc, 60, now).unwrap();
        // Same name and same PID, new incarnation.
        db.conn()
            .execute(
                "UPDATE instances SET created_at = 2.0 WHERE name = 'luna'",
                [],
            )
            .unwrap();
        sweep(
            &db,
            now + 1.0,
            Path::new("/nonexistent/hcom-idle-kill-test"),
        );
        assert_eq!(raw(&db, "luna"), None);
        cleanup(path);
    }

    #[test]
    fn keys_without_a_row_are_pruned_after_grace() {
        let (db, path) = setup_test_db();
        let now = now_epoch_f64();
        let missing = Path::new("/nonexistent/hcom-idle-kill-test");
        set(&db, "luna", 60).unwrap();
        sweep(&db, now + 10.0, missing);
        assert_eq!(get(&db, "luna"), Some(60), "launch window keeps the key");
        sweep(&db, now + NO_ROW_GRACE_SECS + 1.0, missing);
        assert_eq!(get(&db, "luna"), None);
        cleanup(path);
    }

    #[test]
    fn read_errors_are_never_idle() {
        let (db, path) = setup_test_db();
        let now = now_epoch_f64();
        insert_listening(&db, "luna", 100);
        set(&db, "luna", 60).unwrap();
        let armed = raw(&db, "luna").unwrap();

        // A malformed request watch makes the pending-request query fail.
        db.kv_set("events_sub:reqwatch-1-nova", Some("not json"))
            .unwrap();
        assert!(is_idle(&db, &row(&db, "luna"), 60, now).is_err());
        sweep(&db, now, Path::new("/bin/true"));
        assert_eq!(raw(&db, "luna").as_deref(), Some(armed.as_str()));
        db.kv_set("events_sub:reqwatch-1-nova", None).unwrap();

        // An unreadable event log makes the unread query fail.
        db.conn()
            .execute_batch("DROP VIEW events_v; ALTER TABLE events RENAME TO events_gone;")
            .unwrap();
        assert!(is_idle(&db, &row(&db, "luna"), 60, now).is_err());
        sweep(&db, now, Path::new("/bin/true"));
        assert_eq!(raw(&db, "luna").as_deref(), Some(armed.as_str()));
        cleanup(path);
    }

    /// A live process group to stand in for the agent.
    fn sleeper() -> std::process::Child {
        use std::os::unix::process::CommandExt;
        Command::new("sleep")
            .arg("60")
            .process_group(0)
            .spawn()
            .unwrap()
    }

    fn bind(db: &HcomDb, name: &str, pid: u32) {
        db.conn()
            .execute(
                "UPDATE instances SET pid = ?, background = 1 WHERE name = ?",
                rusqlite::params![pid, name],
            )
            .unwrap();
    }

    /// Claim `name` the way a sweep does and return the claimed value.
    fn claim_now(db: &HcomDb, name: &str, now: f64) -> String {
        let armed = raw(db, name).unwrap();
        let inc = incarnation(&row(db, name));
        let secs = get(db, name).unwrap();
        claim(db, &key(name), &armed, name, &inc, secs, now).expect("claim")
    }

    fn exited(child: &mut std::process::Child) -> bool {
        for _ in 0..50 {
            if child.try_wait().unwrap().is_some() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        false
    }

    #[test]
    fn worker_kills_the_claimed_incarnation() {
        let (db, path) = setup_test_db();
        let now = now_epoch_f64();
        let mut agent = sleeper();
        insert_listening(&db, "luna", 100);
        bind(&db, "luna", agent.id());
        set(&db, "luna", 60).unwrap();
        let claimed = claim_now(&db, "luna", now);

        assert!(kill_claimed(&db, "luna", &claimed, now).unwrap());
        assert!(exited(&mut agent), "agent process survived");
        assert!(db.get_instance_full("luna").unwrap().is_none());
        cleanup(path);
    }

    #[test]
    fn worker_spares_a_changed_policy_incarnation_or_activity() {
        let (db, path) = setup_test_db();
        let now = now_epoch_f64();
        let mut agent = sleeper();
        insert_listening(&db, "luna", 100);
        bind(&db, "luna", agent.id());

        // Policy rewritten after the claim.
        set(&db, "luna", 60).unwrap();
        let claimed = claim_now(&db, "luna", now);
        set(&db, "luna", 600).unwrap();
        assert!(!kill_claimed(&db, "luna", &claimed, now).unwrap());

        // Policy cleared after the claim.
        set(&db, "luna", 60).unwrap();
        let claimed = claim_now(&db, "luna", now);
        set(&db, "luna", 0).unwrap();
        assert!(!kill_claimed(&db, "luna", &claimed, now).unwrap());

        // The agent became active after the claim.
        set(&db, "luna", 60).unwrap();
        let claimed = claim_now(&db, "luna", now);
        db.conn()
            .execute(
                "UPDATE instances SET status = 'active' WHERE name = 'luna'",
                [],
            )
            .unwrap();
        assert!(!kill_claimed(&db, "luna", &claimed, now).unwrap());
        db.conn()
            .execute(
                "UPDATE instances SET status = 'listening' WHERE name = 'luna'",
                [],
            )
            .unwrap();

        // The name now belongs to a new incarnation with the same PID.
        set(&db, "luna", 60).unwrap();
        let claimed = claim_now(&db, "luna", now);
        db.conn()
            .execute(
                "UPDATE instances SET created_at = 2.0 WHERE name = 'luna'",
                [],
            )
            .unwrap();
        assert!(!kill_claimed(&db, "luna", &claimed, now).unwrap());

        assert!(agent.try_wait().unwrap().is_none(), "agent was killed");
        assert_eq!(row(&db, "luna").status, ST_LISTENING);
        let _ = agent.kill();
        let _ = agent.wait();
        cleanup(path);
    }

    #[test]
    fn worker_rejects_malformed_arguments() {
        assert_eq!(run_worker(&[]), 2);
        assert_eq!(run_worker(&["all".to_string()]), 2);
    }
}
