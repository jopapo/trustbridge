use crate::cli::{ApplyArgs, TargetKind};
use crate::commands::filtering::{apply_default_filter, apply_profile_overrides, FilterOptions};
use crate::commands::images::{patch_images, ImagePatchOptions};
use crate::commands::workloads::{patch_workloads, WorkloadPatchOptions};
use crate::commands::{resolve_source, resolve_target};
use crate::core::engine::SyncEngine;
use crate::core::paths;
use crate::core::state::StateSnapshot;
use crate::providers::target::vm_backend::is_wsl;
use anyhow::{anyhow, Result};
use sha2::{Digest, Sha256};
use std::io::{self, Write};
use std::thread;
use std::time::Duration;
use time::OffsetDateTime;

pub fn run(args: ApplyArgs) -> Result<()> {
    let source = resolve_source(args.source);

    if args.dry_run {
        println!("mode: DRY-RUN (no changes will be persisted)");
    }

    let mut watch_status = WatchStatus::default();

    loop {
        let cycle_result = apply_once(&args, source.as_ref(), args.watch);

        if args.watch {
            match cycle_result {
                Ok(status) => {
                    watch_status.last_check = status.last_check;
                    watch_status.last_sync = status.last_sync;
                }
                Err(error) => {
                    watch_status.last_sync = format!("failed: {error}");
                }
            }
            render_watch_status(&watch_status)?;
        } else if let Err(error) = cycle_result {
            return Err(error);
        }

        if !args.watch {
            break;
        }

        watch_status.last_check = format!("sleeping {}s", args.interval_secs);
        render_watch_status(&watch_status)?;
        thread::sleep(Duration::from_secs(args.interval_secs.max(1)));
    }

    Ok(())
}

fn apply_once(
    args: &ApplyArgs,
    source: &dyn crate::providers::source::SourceProvider,
    watch_mode: bool,
) -> Result<ApplyCycleStatus> {
    let state_path = paths::state_path();
    let mut snapshot = StateSnapshot::load(&state_path)?;
    let source_certs = source.scan()?;
    let filter_options = apply_profile_overrides(
        FilterOptions {
            include_public_roots: args.include_public_roots,
            only_keywords: args.only_keywords.clone(),
            exclude_keywords: args.exclude_keywords.clone(),
        },
        args.profile,
    );
    let (source_certs, filter_stats) = apply_default_filter(source_certs, &filter_options);

    let bundle_hash = bundle_hash(&source_certs);
    let check_status = if snapshot.last_bundle_hash.as_deref() == Some(&bundle_hash) {
        format!("bundle unchanged ({bundle_hash})")
    } else {
        format!("bundle changed -> {bundle_hash}")
    };

    if snapshot.last_bundle_hash.as_deref() == Some(&bundle_hash) {
        if !watch_mode {
            println!("bundle unchanged ({bundle_hash}); applying incremental sync");
        }
    } else {
        if !watch_mode {
            println!("bundle changed -> {bundle_hash}");
        }
    }

    let desired_fingerprints: Vec<String> = source_certs
        .iter()
        .map(|certificate| certificate.fingerprint_sha256.clone())
        .collect();

    let mut runtime_status = if has_scope(&args.scope, "runtime") {
        "done".to_string()
    } else {
        "skipped".to_string()
    };

    if has_scope(&args.scope, "runtime") {
        let mut runtime_available = 0usize;
        for target in resolve_runtime_targets(args.target) {
            let target_fingerprints = match target.current_fingerprints() {
                Ok(value) => {
                    runtime_available += 1;
                    value
                }
                Err(error) => {
                    if !watch_mode && target.name() == "colima" {
                        println!(
                            "warning: runtime target `colima` unavailable: {error}\n  hint: colima uses Lima; verify profile/instance (default `colima`) or set TBRIDGE_COLIMA_INSTANCE=<profile>."
                        );
                    } else if !watch_mode {
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

            if !watch_mode {
                println!("applying runtime plan to {}", target.name());
                println!("- add: {}", plan.to_add.len());
                println!("- remove: {}", plan.to_remove.len());
                println!(
                    "- managed fingerprints in state: {}",
                    snapshot.applied_fingerprints.len()
                );
            }

            if let Err(error) = target.apply_plan(&plan, args.dry_run) {
                if !watch_mode {
                    println!(
                        "warning: runtime target `{}` apply failed: {error}",
                        target.name()
                    );
                }
                continue;
            }

            if !args.dry_run {
                snapshot.applied_fingerprints = desired_fingerprints.clone();
            }
        }

        if runtime_available == 0 {
            if has_scope(&args.scope, "containers") || has_scope(&args.scope, "images") {
                runtime_status = "unavailable".to_string();
                if !watch_mode {
                    println!(
                        "warning: no compatible runtime target detected; continuing with non-runtime scopes"
                    );
                }
            } else {
                return Err(anyhow!(
                    "no compatible runtime target detected; start rancher-desktop/colima or pass --scope containers,images"
                ));
            }
        }
    } else {
        if !watch_mode {
            println!("runtime patch skipped (scope)");
        }
    }

    let mut container_result = None;
    if has_scope(&args.scope, "containers") {
        let result = patch_workloads(
            &source_certs,
            &filter_stats,
            &WorkloadPatchOptions {
                dry_run: args.dry_run,
                interactive: args.interactive,
                verbose: !watch_mode,
                containers: args.containers.clone(),
                include_orchestrator: args.include_orchestrator,
                bundle_hash: bundle_hash.clone(),
                known_hashes: snapshot.container_bundle_hashes.clone(),
            },
        );

        match result {
            Ok(result) => {
                container_result = Some(result);
            }
            Err(error) if is_missing_container_cli(&error) => {
                println!(
                    "warning: containers scope skipped: {}",
                    error.to_string().trim()
                );
            }
            Err(error) => return Err(error),
        }

        if !args.dry_run {
            snapshot.container_bundle_hashes = container_result
                .as_ref()
                .map(|value| value.updated_hashes.clone())
                .unwrap_or_default();
        }
    } else {
        if !watch_mode {
            println!("container patch skipped (scope)");
        }
    }

    let mut image_result = None;
    if has_scope(&args.scope, "images") {
        let result = patch_images(
            &source_certs,
            &ImagePatchOptions {
                dry_run: args.dry_run,
                verbose: !watch_mode,
                mode: args.images_mode.clone(),
                include_orchestrator: args.include_orchestrator,
                limit: args.images_limit,
                bundle_hash: bundle_hash.clone(),
                known_hashes: snapshot.image_bundle_hashes.clone(),
            },
        );

        match result {
            Ok(result) => {
                image_result = Some(result);
            }
            Err(error) if is_missing_container_cli(&error) => {
                println!(
                    "warning: images scope skipped: {}",
                    error.to_string().trim()
                );
            }
            Err(error) => return Err(error),
        }

        if !args.dry_run {
            snapshot.image_bundle_hashes = image_result
                .as_ref()
                .map(|value| value.updated_hashes.clone())
                .unwrap_or_default();
        }
    } else {
        if !watch_mode {
            println!("image patch skipped (scope)");
        }
    }

    if !args.dry_run {
        snapshot.last_bundle_hash = Some(bundle_hash);
        snapshot.last_apply_at = Some(OffsetDateTime::now_utc().to_string());
        snapshot.save(&state_path)?;
        if !watch_mode {
            println!("state updated at {}", state_path.display());
        }
    }

    let sync_status = format!(
        "runtime={}, containers(p/s/f)={}/{}/{}, images(p/s/f)={}/{}/{}",
        runtime_status,
        container_result
            .as_ref()
            .map(|r| r.patched)
            .unwrap_or_default(),
        container_result
            .as_ref()
            .map(|r| r.skipped)
            .unwrap_or_default(),
        container_result
            .as_ref()
            .map(|r| r.failed)
            .unwrap_or_default(),
        image_result.as_ref().map(|r| r.patched).unwrap_or_default(),
        image_result.as_ref().map(|r| r.skipped).unwrap_or_default(),
        image_result.as_ref().map(|r| r.failed).unwrap_or_default()
    );

    Ok(ApplyCycleStatus {
        last_check: check_status,
        last_sync: sync_status,
    })
}

#[derive(Default)]
struct WatchStatus {
    last_check: String,
    last_sync: String,
}

struct ApplyCycleStatus {
    last_check: String,
    last_sync: String,
}

fn render_watch_status(status: &WatchStatus) -> Result<()> {
    print!("\x1B[2J\x1B[1;1H");
    println!("watch: last check -> {}", status.last_check);
    println!("watch: last sync  -> {}", status.last_sync);
    io::stdout().flush()?;
    Ok(())
}

fn has_scope(scopes: &[String], name: &str) -> bool {
    scopes.iter().any(|scope| scope.eq_ignore_ascii_case(name))
}

fn resolve_runtime_targets(
    target: TargetKind,
) -> Vec<Box<dyn crate::providers::target::TargetProvider>> {
    match target {
        TargetKind::Auto if cfg!(target_os = "windows") || is_wsl() => vec![
            resolve_target(TargetKind::RancherDesktop),
            resolve_target(TargetKind::DockerDesktop),
            resolve_target(TargetKind::Wsl),
        ],
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

fn is_missing_container_cli(error: &anyhow::Error) -> bool {
    error
        .to_string()
        .contains("no compatible container CLI found")
}
