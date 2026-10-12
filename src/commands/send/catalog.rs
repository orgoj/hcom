//! Catalog fallback for `hcom send`.
//!
//! Running agents route exactly as upstream, and the catalog is read only when
//! a send could reach a stopped or missing catalog agent: an unmatched name, a
//! tag (which also covers stopped agents), a non-running thread member, or
//! `--spawn-as`. Those agents are started before upstream routing runs and the
//! targets are rewritten to their instance names, so the upstream resolver
//! sees them as running. An unreadable catalog only warns, so routing degrades
//! to upstream behavior.

use std::collections::HashSet;
use std::time::{Duration, Instant};

use super::{deliverable_instances, get_recipient_feedback};
use crate::commands::agent::{self, CatalogRoute};
use crate::db::HcomDb;
use crate::identity;
use crate::messages::{InstanceInfo, MessageEnvelope, resolve_targets};
use crate::shared::{SenderIdentity, status_icon};

const AUTOSTART_DELIVERY_ACK_WAIT: Duration = Duration::from_secs(15);
const AUTOSTART_DELIVERY_ACK_POLL: Duration = Duration::from_millis(50);

/// What the catalog fallback did for one send.
#[derive(Debug, Default)]
pub(super) struct CatalogSend {
    autostarted: Vec<String>,
    scope_note: bool,
}

/// Start the catalog agents this send addresses and rewrite `targets` to
/// their instance names. Returns the error text for a send that must fail.
///
/// `bundle` is the inline bundle upstream validates only after routing; it
/// is checked here first so invalid input never starts an agent.
#[allow(clippy::too_many_arguments)]
pub(super) fn prepare(
    db: &HcomDb,
    identity: &SenderIdentity,
    message: &str,
    envelope: &MessageEnvelope,
    bundle: Option<&serde_json::Value>,
    has_separator: bool,
    targets: &mut Vec<String>,
    spawn_as: Option<&str>,
) -> Result<CatalogSend, String> {
    let has_explicit_targets = has_separator || !targets.is_empty();
    let requested = if has_explicit_targets {
        targets.clone()
    } else {
        crate::shared::constants::extract_mentions(message)
    };
    let running = deliverable_instances(db)?;
    // Upstream delivers a thread-only send to the thread's members.
    let stopped_members: Vec<String> = match envelope.thread.as_deref() {
        Some(thread) if requested.is_empty() => db
            .get_thread_members(thread)
            .into_iter()
            .filter(|member| !is_running(&running, member))
            .collect(),
        _ => Vec::new(),
    };
    let needs_catalog = spawn_as.is_some()
        || requested.iter().any(|target| target.ends_with('-'))
        || resolve_targets(&requested, &running).is_ok_and(|(_, unmatched)| !unmatched.is_empty())
        || !stopped_members.is_empty();
    if !needs_catalog {
        return Ok(CatalogSend::default());
    }

    let (mut routes, catalog_found) = match agent::catalog_message_targets(db, identity) {
        Ok(found) => {
            if let Some(agent) = found.unroutable.iter().find(|agent| {
                requested.iter().any(|target| {
                    agent
                        .names
                        .iter()
                        .any(|name| target.eq_ignore_ascii_case(name))
                        || target.strip_suffix('-').is_some_and(|tag| {
                            agent
                                .tag
                                .as_deref()
                                .is_some_and(|agent_tag| agent_tag.eq_ignore_ascii_case(tag))
                        })
                })
            }) {
                return Err(agent.reason.clone());
            }
            let catalog_found = !found.routes.is_empty() || !found.unroutable.is_empty();
            (found.routes, catalog_found)
        }
        Err(e) => {
            eprintln!("Warning: cannot load agent catalog; routing to running agents only: {e:#}");
            (Vec::new(), false)
        }
    };

    if let Some(spawn_name) = spawn_as {
        apply_spawn_as(&running, spawn_name, targets, &mut routes)?;
    }
    let requested = if has_explicit_targets {
        targets.clone()
    } else {
        requested
    };

    let plan = plan(
        db,
        &requested,
        &stopped_members,
        &running,
        &routes,
        catalog_found,
    )?;
    if !plan.starts.is_empty()
        && let Some(bundle) = bundle
    {
        // Upstream validates the bundle again before creating it and warns
        // there about missing files, so this probe must stay silent: an
        // existing path stands in for a non-empty file list.
        let mut probe = bundle.clone();
        if let Some(files) = probe
            .pointer_mut("/refs/files")
            .filter(|files| files.as_array().is_some_and(|list| !list.is_empty()))
        {
            *files = serde_json::json!(["."]);
        }
        crate::core::bundles::validate_bundle(&mut probe)?;
    }
    for route in &plan.starts {
        agent::autostart_catalog_agent(db, route).map_err(|e| e.to_string())?;
    }
    if let Some(rewritten) = plan.targets {
        *targets = rewritten;
    }
    Ok(CatalogSend {
        autostarted: plan
            .starts
            .into_iter()
            .map(|route| route.instance)
            .collect(),
        scope_note: plan.scope_note,
    })
}

/// `--spawn-as`: address one catalog agent under another instance name.
fn apply_spawn_as(
    running: &[InstanceInfo],
    spawn_name: &str,
    targets: &mut Vec<String>,
    routes: &mut Vec<CatalogRoute>,
) -> Result<(), String> {
    if !identity::is_valid_base_name(spawn_name) {
        return Err(identity::base_name_error(spawn_name));
    }
    if targets.len() != 1 {
        return Err(
            "--spawn-as requires exactly one target agent (e.g. hcom send @agent --spawn-as clone_name)"
                .to_string(),
        );
    }
    let target = targets[0].clone();
    if spawn_name.eq_ignore_ascii_case(&target) {
        return Ok(());
    }
    if !is_running(running, spawn_name) {
        let Some(target_route) = routes.iter().find(|r| {
            r.alias.eq_ignore_ascii_case(&target) || r.instance.eq_ignore_ascii_case(&target)
        }) else {
            return Err(format!(
                "Unknown catalog agent '@{target}'. --spawn-as requires a configured catalog agent."
            ));
        };
        let clone_route = CatalogRoute {
            alias: spawn_name.to_string(),
            instance: spawn_name.to_string(),
            tag: target_route.tag.clone(),
            launch_name: target_route.launch_name.clone(),
            launch_dir: target_route.launch_dir.clone(),
        };
        routes.push(clone_route);
    }
    *targets = vec![spawn_name.to_string()];
    Ok(())
}

#[derive(Debug, Default)]
struct Plan {
    /// Catalog agents to start before the message is written.
    starts: Vec<CatalogRoute>,
    /// Replacement explicit targets (canonical instance names), when needed.
    targets: Option<Vec<String>>,
    /// A target stays unmatched after the catalog fallback.
    scope_note: bool,
}

fn plan(
    db: &HcomDb,
    requested: &[String],
    stopped_members: &[String],
    running: &[InstanceInfo],
    routes: &[CatalogRoute],
    catalog_found: bool,
) -> Result<Plan, String> {
    let mut plan = Plan::default();
    if !requested.is_empty() {
        let mut routing_rows = running.to_vec();
        for route in routes {
            for name in [&route.alias, &route.instance] {
                if !is_running(&routing_rows, name) {
                    routing_rows.push(InstanceInfo {
                        name: name.clone(),
                        tag: route.tag.clone(),
                    });
                }
            }
        }
        // A malformed target is upstream's error to report.
        let Ok((matched, unmatched)) = resolve_targets(requested, &routing_rows) else {
            return Ok(plan);
        };
        if !unmatched.is_empty() {
            // Start nothing: upstream rejects the send, and the note explains
            // that catalog scope depends on the directory.
            plan.scope_note = catalog_found;
            return Ok(plan);
        }
        let mut canonical_targets: Vec<String> = Vec::new();
        let mut rewritten = false;
        for name in matched {
            let canonical = routes
                .iter()
                .find(|route| route.alias.eq_ignore_ascii_case(&name))
                .map(|route| route.instance.clone())
                .unwrap_or_else(|| name.clone());
            rewritten |= canonical != name;
            if !is_running(running, &canonical)
                && let Some(route) = routes
                    .iter()
                    .find(|route| route.instance.eq_ignore_ascii_case(&canonical))
            {
                push_start(&mut plan.starts, route.clone());
            }
            if !canonical_targets
                .iter()
                .any(|target| target.eq_ignore_ascii_case(&canonical))
            {
                canonical_targets.push(canonical);
            }
        }
        // Started agents are addressed by name: a tag or alias would not match
        // a row that is still registering.
        if rewritten || !plan.starts.is_empty() {
            plan.targets = Some(canonical_targets);
        }
    }
    for member in stopped_members {
        if let Some(route) = routes
            .iter()
            .find(|route| route.instance.eq_ignore_ascii_case(member))
        {
            push_start(&mut plan.starts, route.clone());
            continue;
        }
        for route in routes {
            if let Some(retargeted) =
                agent::retarget_roaming_route(db, route, member).map_err(|e| e.to_string())?
            {
                push_start(&mut plan.starts, retargeted);
                break;
            }
        }
    }
    Ok(plan)
}

fn push_start(starts: &mut Vec<CatalogRoute>, route: CatalogRoute) {
    if !starts
        .iter()
        .any(|start| start.instance.eq_ignore_ascii_case(&route.instance))
    {
        starts.push(route);
    }
}

fn is_running(rows: &[InstanceInfo], name: &str) -> bool {
    rows.iter().any(|inst| inst.name.eq_ignore_ascii_case(name))
}

impl CatalogSend {
    /// Explain an unmatched target once the catalog could not resolve it.
    pub(super) fn print_scope_note(&self) {
        if self.scope_note {
            eprintln!("{}", agent::OUT_OF_SCOPE_NOTE);
        }
    }

    /// Briefly wait for started agents to acknowledge `event_id`; returns the
    /// ones still pending.
    pub(super) fn wait_for_ack(&self, db: &HcomDb, event_id: i64) -> HashSet<String> {
        wait_for_ack_until(db, event_id, &self.autostarted, AUTOSTART_DELIVERY_ACK_WAIT)
    }
}

fn wait_for_ack_until(
    db: &HcomDb,
    event_id: i64,
    autostarted: &[String],
    timeout: Duration,
) -> HashSet<String> {
    if autostarted.is_empty() {
        return HashSet::new();
    }

    let deadline = Instant::now() + timeout;
    loop {
        let pending: HashSet<String> = autostarted
            .iter()
            .filter(|name| db.get_cursor(name) < event_id)
            .cloned()
            .collect();
        if pending.is_empty() || Instant::now() >= deadline {
            return pending;
        }
        std::thread::sleep(AUTOSTART_DELIVERY_ACK_POLL);
    }
}

/// Upstream recipient feedback, with started agents that have not yet
/// acknowledged the message reported as pending instead of delivered.
pub(super) fn recipient_feedback(
    db: &HcomDb,
    delivered_to: &[String],
    pending: &HashSet<String>,
    thread: Option<&str>,
) -> String {
    if pending.is_empty() {
        return get_recipient_feedback(db, delivered_to, thread);
    }
    let (waiting, delivered): (Vec<String>, Vec<String>) = delivered_to
        .iter()
        .cloned()
        .partition(|name| pending.contains(name));
    let mut lines = Vec::new();
    if !delivered.is_empty() {
        lines.push(get_recipient_feedback(db, &delivered, thread));
    }
    if waiting.len() > 10 {
        lines.push(format!(
            "Queued; delivery pending for {} agents",
            waiting.len()
        ));
    } else if !waiting.is_empty() {
        let parts: Vec<String> = waiting
            .iter()
            .map(|name| match db.get_instance_full(name) {
                Ok(Some(data)) => format!(
                    "{} {}",
                    status_icon(&data.status),
                    identity::get_display_name(db, name)
                ),
                _ => format!("◌ {name}"),
            })
            .collect();
        lines.push(format!("Queued; delivery pending: {}", parts.join(", ")));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::path::PathBuf;

    fn route(alias: &str, instance: &str, tag: Option<&str>) -> CatalogRoute {
        CatalogRoute {
            alias: alias.into(),
            instance: instance.into(),
            tag: tag.map(Into::into),
            launch_name: alias.into(),
            launch_dir: (alias != instance).then(|| PathBuf::from("/projects/elsewhere")),
        }
    }

    fn targets(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| name.to_string()).collect()
    }

    fn running(names: &[&str]) -> Vec<InstanceInfo> {
        names
            .iter()
            .map(|name| InstanceInfo {
                name: name.to_string(),
                tag: None,
            })
            .collect()
    }

    fn with_db(test: impl FnOnce(&HcomDb, &std::path::Path)) {
        let (_dir, hcom_dir, _home, _guard) = crate::hooks::test_helpers::isolated_test_env();
        let path = hcom_dir.join("test.db");
        let db = HcomDb::open_at(&path).unwrap();
        test(&db, &path);
    }

    #[test]
    #[serial]
    fn targeted_and_tagged_catalog_agents_start_and_are_addressed_by_name() {
        with_db(|db, _| {
            let routes = [route("reviewer", "reviewer", Some("review"))];
            for requested in [targets(&["reviewer"]), targets(&["review-"])] {
                let plan = plan(db, &requested, &[], &[], &routes, true).unwrap();
                assert_eq!(plan.starts[0].instance, "reviewer");
                assert_eq!(plan.targets, Some(targets(&["reviewer"])));
                assert!(!plan.scope_note);
            }
            let broadcast = plan(db, &[], &[], &[], &routes, true).unwrap();
            assert!(broadcast.starts.is_empty());
            assert!(broadcast.targets.is_none());
        });
    }

    #[test]
    #[serial]
    fn running_target_is_left_to_upstream_routing() {
        with_db(|db, _| {
            let routes = [route("reviewer", "reviewer", None)];
            let plan = plan(
                db,
                &targets(&["reviewer"]),
                &[],
                &running(&["reviewer"]),
                &routes,
                true,
            )
            .unwrap();
            assert!(plan.starts.is_empty());
            assert!(plan.targets.is_none());
        });
    }

    #[test]
    #[serial]
    fn roaming_alias_is_rewritten_to_the_materialized_instance() {
        with_db(|db, _| {
            let routes = [route("reviewer", "reviewer_weather", None)];
            let requested = targets(&["reviewer"]);
            let stopped = plan(db, &requested, &[], &[], &routes, true).unwrap();
            assert_eq!(stopped.targets, Some(targets(&["reviewer_weather"])));
            assert_eq!(stopped.starts[0].launch_name, "reviewer");

            let up = running(&["reviewer_weather"]);
            let started = plan(db, &requested, &[], &up, &routes, true).unwrap();
            assert!(started.starts.is_empty());
            assert_eq!(started.targets, Some(targets(&["reviewer_weather"])));
        });
    }

    #[test]
    #[serial]
    fn unmatched_target_starts_nothing_and_notes_scope_only_with_a_catalog() {
        with_db(|db, _| {
            let routes = [route("reviewer", "reviewer", None)];
            let requested = targets(&["reviewer", "typo"]);
            let plan_with = plan(db, &requested, &[], &[], &routes, true).unwrap();
            assert!(plan_with.starts.is_empty());
            assert!(plan_with.scope_note);
            let plan_without = plan(db, &targets(&["typo"]), &[], &[], &[], false).unwrap();
            assert!(!plan_without.scope_note);
        });
    }

    #[test]
    #[serial]
    fn stopped_catalog_thread_member_is_started() {
        with_db(|db, _| {
            let routes = [route("reviewer", "reviewer", None)];
            let plan = plan(db, &[], &targets(&["reviewer"]), &[], &routes, true).unwrap();
            assert_eq!(plan.starts[0].instance, "reviewer");
            assert!(plan.targets.is_none());
        });
    }

    #[test]
    #[serial]
    fn stopped_roaming_thread_member_restarts_in_its_original_project() {
        with_db(|db, _| {
            let tmp = tempfile::tempdir().unwrap();
            let project = tmp.path().join("project");
            std::fs::create_dir_all(project.join(".git")).unwrap();
            db.conn()
                .execute(
                    "INSERT INTO instances (name, status, directory, created_at) \
                     VALUES ('reviewer_project', 'stopped', ?1, 1000.0)",
                    [project.to_string_lossy().as_ref()],
                )
                .unwrap();
            let routes = [route("reviewer", "reviewer_elsewhere", None)];
            let plan = plan(db, &[], &targets(&["reviewer_project"]), &[], &routes, true).unwrap();
            assert_eq!(plan.starts[0].instance, "reviewer_project");
            assert_eq!(
                plan.starts[0].launch_dir.as_deref(),
                Some(std::fs::canonicalize(&project).unwrap().as_path())
            );
        });
    }

    #[test]
    #[serial]
    fn ack_wait_tracks_the_specific_message_event() {
        with_db(|db, _| {
            db.conn()
                .execute(
                    "INSERT INTO instances (name, status, last_event_id, created_at) \
                     VALUES ('reviewer', 'listening', 7, 0)",
                    [],
                )
                .unwrap();
            let started = targets(&["reviewer"]);
            assert!(wait_for_ack_until(db, 7, &started, Duration::ZERO).is_empty());
            assert_eq!(
                wait_for_ack_until(db, 8, &started, Duration::ZERO),
                HashSet::from(["reviewer".to_string()])
            );
        });
    }

    #[test]
    #[serial]
    fn ack_wait_observes_a_later_cursor_advance() {
        with_db(|db, path| {
            db.conn()
                .execute(
                    "INSERT INTO instances (name, status, last_event_id, created_at) \
                     VALUES ('reviewer', 'listening', 7, 0)",
                    [],
                )
                .unwrap();
            let writer_path = path.to_path_buf();
            let writer = std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(30));
                HcomDb::open_at(&writer_path)
                    .unwrap()
                    .conn()
                    .execute(
                        "UPDATE instances SET last_event_id = 8 WHERE name = 'reviewer'",
                        [],
                    )
                    .unwrap();
            });
            let pending =
                wait_for_ack_until(db, 8, &targets(&["reviewer"]), Duration::from_secs(1));
            writer.join().unwrap();
            assert!(pending.is_empty());
        });
    }

    #[test]
    #[serial]
    fn pending_feedback_does_not_claim_delivery() {
        with_db(|db, _| {
            db.conn()
                .execute(
                    "INSERT INTO instances (name, status, created_at) \
                     VALUES ('reviewer', 'listening', 0)",
                    [],
                )
                .unwrap();
            let feedback = recipient_feedback(
                db,
                &targets(&["reviewer"]),
                &HashSet::from(["reviewer".to_string()]),
                None,
            );
            assert!(feedback.contains("Queued; delivery pending:"));
            assert!(!feedback.contains("Sent to:"));
        });
    }
}
