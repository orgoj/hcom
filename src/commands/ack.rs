//! Explicit acknowledgement for reliable external receive adapters.

use clap::Parser;

use crate::db::HcomDb;
use crate::identity;
use crate::shared::CommandContext;

#[derive(Parser, Debug)]
#[command(name = "ack", about = "Acknowledge the first unread message")]
pub struct AckArgs {
    /// Event ID emitted by `hcom listen --manual-ack --json`
    pub event_id: i64,
}

pub fn cmd_ack(db: &HcomDb, args: &AckArgs, ctx: Option<&CommandContext>) -> i32 {
    let explicit_name = ctx.and_then(|c| c.explicit_name.as_deref());
    let resolved = ctx
        .and_then(|c| c.identity.clone())
        .map(Ok)
        .unwrap_or_else(|| {
            identity::resolve_identity(db, explicit_name, None, None, None, None, None)
        });
    let identity = match resolved {
        Ok(identity) => identity,
        Err(error) => {
            eprintln!("Error: {error}");
            return 1;
        }
    };
    if let Err(error) = db.ack_first_deliverable(&identity.name, args.event_id) {
        eprintln!("Error: {error}");
        return 1;
    }
    println!("Acknowledged event {} for {}", args.event_id, identity.name);
    0
}
