//! Narrow lifecycle callbacks for `hcom hermes` managed CLI sessions.

use crate::db::HcomDb;
use crate::hooks::common::finalize_session;
use crate::instance_binding;
use crate::instance_lifecycle as lifecycle;
use crate::shared::{HcomContext, ST_ACTIVE, ST_LISTENING};

fn instance_name(db: &HcomDb, process_id: Option<&str>) -> Result<String, String> {
    let process_id = process_id
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "HCOM_PROCESS_ID not set".to_string())?;
    db.get_process_binding(process_id)
        .map_err(|error| format!("hcom Hermes process binding error: {error}"))?
        .ok_or_else(|| format!("no hcom process binding for HCOM_PROCESS_ID '{process_id}'"))
}

pub fn dispatch_hermes_hook(hook: &str, argv: &[String]) -> (i32, String) {
    let ctx = HcomContext::from_os();
    let db = match HcomDb::open() {
        Ok(db) => db,
        Err(error) => return (1, format!("hcom Hermes hook DB error: {error}")),
    };
    let name = match instance_name(&db, ctx.process_id.as_deref()) {
        Ok(name) => name,
        Err(error) => return (1, error),
    };
    if db.get_instance_full(&name).ok().flatten().is_none() {
        return (1, format!("unknown hcom Hermes identity '{name}'"));
    }

    match hook {
        "hermes-start" => {
            lifecycle::set_status(&db, &name, ST_LISTENING, "start", Default::default());
            instance_binding::capture_and_store_launch_context(&db, &name);
            (0, String::new())
        }
        "hermes-status" => {
            let Some(status) = argv
                .iter()
                .position(|arg| arg == "hermes-status")
                .and_then(|index| argv.get(index + 1))
                .or_else(|| argv.first())
                .map(String::as_str)
            else {
                return (1, "usage: hcom hermes-status active|listening".to_string());
            };
            let status = match status {
                "active" => ST_ACTIVE,
                "listening" => ST_LISTENING,
                other => return (1, format!("invalid Hermes status '{other}'")),
            };
            lifecycle::set_status(&db, &name, status, status, Default::default());
            if status == ST_LISTENING {
                crate::notify::wake(&db, &name, &[]);
            }
            (0, String::new())
        }
        "hermes-stop" => {
            finalize_session(&db, &name, "exit", None);
            (0, String::new())
        }
        other => (1, format!("unknown Hermes hook '{other}'")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_is_resolved_from_the_process_binding() {
        let dir = tempfile::tempdir().unwrap();
        let db = HcomDb::open_at(&dir.path().join("state.db")).unwrap();
        db.set_process_binding("process-1", "", "kato").unwrap();

        assert_eq!(instance_name(&db, Some("process-1")).as_deref(), Ok("kato"));
        assert!(
            instance_name(&db, Some("unknown"))
                .unwrap_err()
                .contains("no hcom process binding")
        );
        assert_eq!(
            instance_name(&db, None).unwrap_err(),
            "HCOM_PROCESS_ID not set"
        );
    }
}
