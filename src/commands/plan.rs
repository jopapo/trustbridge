use crate::cli::PlanArgs;
use crate::commands::{resolve_source, resolve_target};
use crate::core::engine::SyncEngine;
use anyhow::Result;

pub fn run(args: PlanArgs) -> Result<()> {
    let source = resolve_source(args.source);
    let target = resolve_target(args.target);

    let plan = SyncEngine::build_plan(source.as_ref(), target.as_ref())?;

    println!("source: {}", source.name());
    println!("target: {}", target.name());
    println!("source certs: {}", plan.source_total);
    println!("target certs: {}", plan.target_total);
    println!("to add: {}", plan.to_add.len());
    println!("to remove: {}", plan.to_remove.len());

    if plan.is_noop() {
        println!("plan is noop");
    }

    Ok(())
}
