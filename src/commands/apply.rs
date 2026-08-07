use crate::cli::{ApplyArgs, TargetKind};
use crate::commands::filtering::{apply_default_filter, FilterOptions};
use crate::commands::images::{patch_images, ImagePatchOptions};
use crate::commands::workloads::{patch_workloads, WorkloadPatchOptions};
use crate::commands::{resolve_source, resolve_target};
use crate::core::engine::SyncEngine;
use crate::core::paths;
use crate::core::state::StateSnapshot;
use anyhow::{anyhow, Result};
use sha2::{Digest, Sha256};
use std::thread;
use std::time::Duration;
use time::OffsetDateTime;

pub fn run(args: ApplyArgs) -> Result<()> {
    let source = resolve_source(args.source);

    if args.dry_run {
        println!("mode: DRY-RUN (no changes will be persisted)");
    }

    loop {
        if let Err(error) = apply_once(&args, source.as_ref()) {
            if args.watch {
                println!("watch: cycle failed: {error}");
            } else {
                return Err(error);
            }
        }

        if !args.watch {
            break;
        }

        println!("watch: sleeping {}s before next sync", args.interval_secs);
        thread::sleep(Duration::from_secs(args.interval_secs.max(1)));
    }

    Ok(())
}

fn apply_once(
    args: &ApplyArgs,
    source: &dyn crate::providers::source::SourceProvider,
) -> Result<()> {
    let state_path = paths::state_path();
    let mut snapshot = StateSnapshot::load(&state_path)?;
    let source_certs = source.scan()?;
    let (source_certs, filter_stats) = apply_default_filter(
        source_certs,
        &FilterOptions {
            include_public_roots: args.include_public_roots,
            only_keywords: args.only_keywords.clone(),
            exclude_keywords: args.exclude_keywords.clone(),
        },
    );

    let bundle_hash = bundle_hash(&source_certs);
    if snapshot.last_bundle_hash.as_deref() == Some(&bundle_hash) {
        println!("bundle unchanged ({bundle_hash}); applying incremental sync");
    } else {
        println!("bundle changed -> {bundle_hash}");
    }

    let desired_fingerprints: Vec<String> = source_certs
        .iter()
        .map(|certificate| certificate.fingerprint_sha256.clone())
        .collect();

    if has_scope(&args.scope, "runtime") {
        let mut runtime_available = 0usize;
        for target in resolve_runtime_targets(args.target) {
            let target_fingerprints = match target.current_fingerprints() {
                Ok(value) => {
                    runtime_available += 1;
                    value
                }
                Err(error) => {
                    if target.name() == "colima" {
                        println!(
                            "warning: runtime target `colima` unavailable: {error}\n  hint: colima uses Lima; verify profile/instance (default `colima`) or set TBRIDGE_COLIMA_INSTANCE=<profile>."
                        );
                    } else {
                        println!(
                            "warning: runtime target `{}` unavailable: {error}",
                            target.name()
                        );
                    }
                    continue;
                }
            };

            let plan = SyncEngine::build_plan_from_data(
                source_certs.clone(),
                target_fingerprints,
                snapshot.applied_fingerprints.clone(),
            );

            println!("applying runtime plan to {}", target.name());
            println!("- add: {}", plan.to_add.len());
            println!("- remove: {}", plan.to_remove.len());
            println!(
                "- managed fingerprints in state: {}",
                snapshot.applied_fingerprints.len()
            );

            target.apply_plan(&plan, args.dry_run)?;

            if !args.dry_run {
                snapshot.applied_fingerprints = desired_fingerprints.clone();
            }
        }

        if runtime_available == 0 {
            if has_scope(&args.scope, "containers") || has_scope(&args.scope, "images") {
                println!(
                    "warning: no compatible runtime target detected; continuing with non-runtime scopes"
                );
            } else {
                return Err(anyhow!(
                    "no compatible runtime target detected; start rancher-desktop/colima or pass --scope containers,images"
                ));
            }
        }
    } else {
        println!("runtime patch skipped (scope)");
    }

    if has_scope(&args.scope, "containers") {
        let container_result = patch_workloads(
            &source_certs,
            &filter_stats,
            &WorkloadPatchOptions {
                dry_run: args.dry_run,
                interactive: args.interactive,
                containers: args.containers.clone(),
                include_orchestrator: args.include_orchestrator,
                bundle_hash: bundle_hash.clone(),
                known_hashes: snapshot.container_bundle_hashes.clone(),
            },
        )?;

        if !args.dry_run {
            snapshot.container_bundle_hashes = container_result.updated_hashes;
        }
    } else {
        println!("container patch skipped (scope)");
    }

    if has_scope(&args.scope, "images") {
        let image_result = patch_images(
            &source_certs,
            &ImagePatchOptions {
                dry_run: args.dry_run,
                mode: args.images_mode.clone(),
                include_orchestrator: args.include_orchestrator,
                limit: args.images_limit,
                bundle_hash: bundle_hash.clone(),
                known_hashes: snapshot.image_bundle_hashes.clone(),
            },
        )?;

        if !args.dry_run {
            snapshot.image_bundle_hashes = image_result.updated_hashes;
        }
    } else {
        println!("image patch skipped (scope)");
    }

    if !args.dry_run {
        snapshot.last_bundle_hash = Some(bundle_hash);
        snapshot.last_apply_at = Some(OffsetDateTime::now_utc().to_string());
        snapshot.save(&state_path)?;
        println!("state updated at {}", state_path.display());
    }

    Ok(())
}

fn has_scope(scopes: &[String], name: &str) -> bool {
    scopes.iter().any(|scope| scope.eq_ignore_ascii_case(name))
}

fn resolve_runtime_targets(
    target: TargetKind,
) -> Vec<Box<dyn crate::providers::target::TargetProvider>> {
    match target {
        TargetKind::Auto => vec![
            resolve_target(TargetKind::RancherDesktop),
            resolve_target(TargetKind::Colima),
        ],
        _ => vec![resolve_target(target)],
    }
}

fn bundle_hash(certs: &[crate::core::certificate::Certificate]) -> String {
    let mut fingerprints: Vec<String> = certs
        .iter()
        .map(|certificate| certificate.fingerprint_sha256.clone())
        .collect();
    fingerprints.sort();

    let mut hasher = Sha256::new();
    for fingerprint in fingerprints {
        hasher.update(fingerprint.as_bytes());
    }

    format!("{:x}", hasher.finalize())
}
