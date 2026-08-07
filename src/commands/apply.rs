use crate::cli::ApplyArgs;
use crate::commands::{resolve_source, resolve_target};
use crate::core::engine::SyncEngine;
use crate::core::state::StateSnapshot;
use anyhow::Result;
use std::path::PathBuf;
use time::OffsetDateTime;

pub fn run(args: ApplyArgs) -> Result<()> {
    let source = resolve_source(args.source);
    let target = resolve_target(args.target);

    let plan = SyncEngine::build_plan(source.as_ref(), target.as_ref())?;

    println!("applying plan to {}", target.name());
    println!("- add: {}", plan.to_add.len());
    println!("- remove: {}", plan.to_remove.len());

    target.apply_plan(&plan, args.dry_run)?;

    if !args.dry_run {
        let state_path = default_state_path();
        let mut snapshot = StateSnapshot::load(&state_path)?;
        snapshot.applied_fingerprints = plan
            .to_add
            .iter()
            .map(|certificate| certificate.fingerprint_sha256.clone())
            .collect();
        snapshot.last_apply_at = Some(OffsetDateTime::now_utc().to_string());
        snapshot.save(&state_path)?;

        println!("state updated at {}", state_path.display());
    }

    Ok(())
}

fn default_state_path() -> PathBuf {
    PathBuf::from(".tbridge/state.json")
}
