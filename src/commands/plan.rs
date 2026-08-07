use crate::cli::PlanArgs;
use crate::commands::filtering::{apply_default_filter, FilterOptions};
use crate::commands::{resolve_source, resolve_target};
use crate::core::engine::SyncEngine;
use crate::core::paths;
use crate::core::state::StateSnapshot;
use anyhow::Result;

pub fn run(args: PlanArgs) -> Result<()> {
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
    let target_fingerprints = target.current_fingerprints()?;

    let plan = SyncEngine::build_plan_from_data(
        source_certs,
        target_fingerprints,
        snapshot.applied_fingerprints.clone(),
    );

    println!("source: {}", source.name());
    println!("target: {}", target.name());
    println!("state path: {}", state_path.display());
    println!("config path: {}", paths::config_path().display());
    println!("source certs: {}", plan.source_total);
    println!("target certs: {}", plan.target_total);
    println!("managed in state: {}", snapshot.applied_fingerprints.len());
    println!("to add: {}", plan.to_add.len());
    println!("to remove: {}", plan.to_remove.len());
    println!("remove policy: state-managed fingerprints only");

    if plan.is_noop() {
        println!("plan is noop");
    }

    Ok(())
}
