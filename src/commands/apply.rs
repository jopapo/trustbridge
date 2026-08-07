use crate::cli::ApplyArgs;
use crate::commands::filtering::{apply_default_filter, FilterOptions};
use crate::commands::{resolve_source, resolve_target};
use crate::core::engine::SyncEngine;
use crate::core::paths;
use crate::core::state::StateSnapshot;
use anyhow::Result;
use time::OffsetDateTime;

pub fn run(args: ApplyArgs) -> Result<()> {
    let source = resolve_source(args.source);
    let target = resolve_target(args.target);
    let state_path = paths::state_path();
    let snapshot = StateSnapshot::load(&state_path)?;
    let source_certs = source.scan()?;
    let source_certs = apply_default_filter(
        source_certs,
        &FilterOptions {
            include_public_roots: args.include_public_roots,
            only_keywords: args.only_keywords,
            exclude_keywords: args.exclude_keywords,
        },
    )
    .0;
    let desired_fingerprints: Vec<String> = source_certs
        .iter()
        .map(|certificate| certificate.fingerprint_sha256.clone())
        .collect();
    let target_fingerprints = target.current_fingerprints()?;

    let plan = SyncEngine::build_plan_from_data(
        source_certs,
        target_fingerprints,
        snapshot.applied_fingerprints.clone(),
    );

    println!("applying plan to {}", target.name());
    println!("- add: {}", plan.to_add.len());
    println!("- remove: {}", plan.to_remove.len());
    println!(
        "- managed fingerprints in state: {}",
        snapshot.applied_fingerprints.len()
    );

    target.apply_plan(&plan, args.dry_run)?;

    if !args.dry_run {
        let mut snapshot = snapshot;
        snapshot.applied_fingerprints = desired_fingerprints;
        snapshot.last_apply_at = Some(OffsetDateTime::now_utc().to_string());
        snapshot.save(&state_path)?;

        println!("state updated at {}", state_path.display());
    }

    Ok(())
}
