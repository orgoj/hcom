//! `hcom handoff` — replace the calling agent with a fresh session of itself.
//!
//! The agent writes the prompt for its successor. hcom relaunches the same
//! catalog agent under the same instance name in a clean session, with that
//! prompt as the initial prompt. The relaunch runs detached because it kills
//! the agent that asked for it.

use std::io::{IsTerminal, Read};
use std::process::{Command, Stdio};

use crate::db::HcomDb;
use crate::shared::{CommandContext, SenderKind};

/// Parsed arguments for `hcom handoff`.
#[derive(clap::Parser, Debug)]
#[command(
    name = "handoff",
    about = "Restart yourself in a clean session with a prompt you write"
)]
pub struct HandoffArgs {
    /// Initial prompt for the successor session; read from stdin when omitted
    pub prompt: Vec<String>,
    /// Restart on another AI CLI (claude, codex, ...)
    #[arg(long)]
    pub cli: Option<String>,
    /// Print the relaunch without stopping anything
    #[arg(long)]
    pub dry_run: bool,
}

pub fn cmd_handoff(_db: &HcomDb, args: &HandoffArgs, ctx: Option<&CommandContext>) -> i32 {
    match run(args, ctx) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Error: {e:#}");
            1
        }
    }
}

fn run(args: &HandoffArgs, ctx: Option<&CommandContext>) -> anyhow::Result<i32> {
    let Some((name, data)) = ctx
        .and_then(|c| c.identity.as_ref())
        .filter(|id| matches!(id.kind, SenderKind::Instance))
        .and_then(|id| Some((id.name.clone(), id.instance_data.clone()?)))
    else {
        anyhow::bail!("hcom handoff must be run by an hcom agent, for itself");
    };
    let agent_name = data
        .get("launch_context")
        .and_then(|v| v.as_str())
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
        .and_then(|ctx| ctx.get("agent_name")?.as_str().map(str::to_string))
        .filter(|n| !n.is_empty())
        .or_else(|| env_agent_name(&name))
        .ok_or_else(|| {
            anyhow::anyhow!("'{name}' has no recorded agent name; relaunch it with hcom agent")
        })?;
    let dir = data
        .get("directory")
        .and_then(|v| v.as_str())
        .filter(|d| !d.is_empty())
        .ok_or_else(|| anyhow::anyhow!("'{name}' has no recorded directory"))?
        .to_string();

    let prompt = read_prompt(&args.prompt)?;
    let mut argv = vec![
        "agent".to_string(),
        agent_name,
        "--as".to_string(),
        name.clone(),
        "--restart".to_string(),
        "--clean".to_string(),
        "--hcom-prompt".to_string(),
        prompt,
    ];
    if let Some(cli) = &args.cli {
        argv.extend(["--cli".to_string(), cli.clone()]);
    }

    // Validate synchronously: the detached relaunch has nobody to report to.
    let mut check = relaunch_command(&argv, &dir);
    check.arg("--dry-run");
    let output = check.output()?;
    if !output.status.success() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
        anyhow::bail!("handoff not started; '{name}' keeps running");
    }
    if args.dry_run {
        print!("{}", String::from_utf8_lossy(&output.stdout));
        return Ok(0);
    }

    let log_path = crate::paths::hcom_dir()
        .join(crate::paths::LOGS_DIR)
        .join(format!("handoff-{name}.log"));
    let log = std::fs::File::create(&log_path)?;
    let mut relaunch = relaunch_command(&argv, &dir);
    relaunch
        .stdin(Stdio::null())
        .stdout(log.try_clone()?)
        .stderr(log);
    crate::sys::process::detach_session(&mut relaunch);
    relaunch.spawn()?;
    println!(
        "Handoff started: '{name}' stops now and restarts in a clean session with your prompt. \
         End your turn. Log: {}",
        log_path.display()
    );
    Ok(0)
}

/// The catalog name from the agent's own environment, for sessions whose
/// recorded name is missing. Trusted only when the environment belongs to
/// the instance being handed off.
fn env_agent_name(name: &str) -> Option<String> {
    let instance = std::env::var("HCOM_INSTANCE_NAME").ok()?;
    let agent = std::env::var(crate::commands::agent::AGENT_NAME_ENV).ok()?;
    (instance == name && !agent.is_empty()).then_some(agent)
}

fn read_prompt(words: &[String]) -> anyhow::Result<String> {
    let prompt = if words.is_empty() && !std::io::stdin().is_terminal() {
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        buf
    } else {
        words.join(" ")
    };
    let prompt = prompt.trim().to_string();
    if prompt.is_empty() {
        anyhow::bail!("handoff needs a prompt for the successor (argument or stdin)");
    }
    Ok(prompt)
}

/// `hcom agent ...` as a fresh launch from the agent's directory, not as a
/// child of the agent being replaced.
fn relaunch_command(argv: &[String], dir: &str) -> Command {
    let exe = std::env::current_exe().unwrap_or_else(|_| "hcom".into());
    let mut cmd = Command::new(exe);
    cmd.args(argv).current_dir(dir);
    for var in crate::launcher::env_strip_set() {
        cmd.env_remove(var);
    }
    for var in [
        "HCOM_INSTANCE_NAME",
        crate::commands::agent::AGENT_NAME_ENV,
        "HCOM_TOOL",
    ] {
        cmd.env_remove(var);
    }
    cmd
}
