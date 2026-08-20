use crate::commands::container_runtime::ContainerRuntime;
use crate::commands::filtering::FilterStats;
use crate::core::certificate::Certificate;
use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::io::{self, Write};
use std::process::{Command, Stdio};

pub struct WorkloadPatchOptions {
    pub dry_run: bool,
    pub interactive: bool,
    pub verbose: bool,
    pub containers: Vec<String>,
    pub include_orchestrator: bool,
    pub bundle_hash: String,
    pub known_hashes: HashMap<String, String>,
}

pub struct WorkloadPatchResult {
    pub updated_hashes: HashMap<String, String>,
    pub patched: usize,
    pub skipped: usize,
    pub failed: usize,
}

pub fn patch_workloads(
    certs: &[Certificate],
    stats: &FilterStats,
    options: &WorkloadPatchOptions,
) -> Result<WorkloadPatchResult> {
    if options.verbose {
        println!(
            "workload patch filter result: kept {} / dropped {}",
            stats.kept, stats.dropped
        );
        println!("workload patch certificates selected: {}", certs.len());
    }

    let mut result = WorkloadPatchResult {
        updated_hashes: options.known_hashes.clone(),
        patched: 0,
        skipped: 0,
        failed: 0,
    };

    if certs.is_empty() {
        if options.verbose {
            println!("workload patch: nothing to patch (filtered set is empty)");
        }
        return Ok(result);
    }

    let runtime = ContainerRuntime::detect()?;
    let containers = if options.containers.is_empty() {
        list_running_containers(&runtime)?
    } else {
        options.containers.clone()
    };

    let containers: Vec<String> = containers
        .into_iter()
        .filter(|container| options.include_orchestrator || !is_orchestrator_container(container))
        .collect();

    if containers.is_empty() {
        if options.verbose {
            println!("workload patch: no running containers selected");
        }
        return Ok(result);
    }

    if options.verbose {
        println!("workload patch containers selected: {}", containers.len());
    }

    for container in containers {
        if options
            .known_hashes
            .get(&container)
            .is_some_and(|hash| hash == &options.bundle_hash)
        {
            if options.verbose {
                println!("- {container}: skipped (already in sync)");
            }
            result.skipped += 1;
            continue;
        }

        if options.interactive && !confirm_container(&container)? {
            if options.verbose {
                println!("- {container}: skipped by user");
            }
            result.skipped += 1;
            continue;
        }

        match patch_container(&runtime, &container, certs, options.dry_run) {
            Ok(_) => {
                result.patched += 1;
                if options.verbose {
                    println!("- {container}: patched");
                }
                if !options.dry_run {
                    result
                        .updated_hashes
                        .insert(container.clone(), options.bundle_hash.clone());
                }
            }
            Err(error) => {
                result.failed += 1;
                if options.verbose {
                    println!("- {container}: failed ({error})");
                }
            }
        }
    }

    if options.verbose {
        println!(
            "workload patch summary: patched={}, skipped={}, failed={}",
            result.patched, result.skipped, result.failed
        );
    }

    Ok(result)
}

fn is_orchestrator_container(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.starts_with("k8s_")
        || lower.contains("_kube-system_")
        || lower.contains("_kube-public_")
        || lower.contains("_kube-node-lease_")
}

fn confirm_container(container: &str) -> Result<bool> {
    print!("Patch container `{container}`? [y/N]: ");
    io::stdout().flush().context("failed to flush stdout")?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .context("failed to read confirmation")?;

    let answer = input.trim().to_ascii_lowercase();
    Ok(answer == "y" || answer == "yes")
}

fn patch_container(
    runtime: &ContainerRuntime,
    container: &str,
    certs: &[Certificate],
    dry_run: bool,
) -> Result<()> {
    ensure_ca_update_tool(runtime, container, dry_run)?;
    let (cert_dir, update_cmd) = detect_patch_strategy(runtime, container)?;

    if dry_run {
        println!(
            "  [dry-run] strategy: dir={} update='{}' certs={}",
            cert_dir,
            update_cmd,
            certs.len()
        );
        return Ok(());
    }

    exec_in_container_root(
        runtime,
        container,
        &["sh", "-lc", &format!("mkdir -p '{cert_dir}'")],
    )?;

    for certificate in certs {
        let path = format!("{cert_dir}/{}.crt", certificate.fingerprint_sha256);
        write_file_in_container(runtime, container, &path, &certificate.pem)?;
    }

    exec_in_container_root(runtime, container, &["sh", "-lc", &update_cmd])?;
    Ok(())
}

fn ensure_ca_update_tool(runtime: &ContainerRuntime, container: &str, dry_run: bool) -> Result<()> {
    if dry_run {
        let check_script = "if command -v update-ca-certificates >/dev/null 2>&1 || command -v update-ca-trust >/dev/null 2>&1; then exit 0; fi; if command -v apt-get >/dev/null 2>&1 || command -v apk >/dev/null 2>&1 || command -v dnf >/dev/null 2>&1 || command -v yum >/dev/null 2>&1 || command -v microdnf >/dev/null 2>&1 || command -v zypper >/dev/null 2>&1 || command -v pacman >/dev/null 2>&1; then exit 0; fi; echo UNSUPPORTED";
        let output =
            exec_in_container_root_capture(runtime, container, &["sh", "-lc", check_script])?;
        if output.trim() == "UNSUPPORTED" {
            return Err(anyhow!(
                "container lacks CA update tool and supported package manager"
            ));
        }

        return Ok(());
    }

    let install_script = "if command -v update-ca-certificates >/dev/null 2>&1 || command -v update-ca-trust >/dev/null 2>&1; then exit 0; fi; if command -v apt-get >/dev/null 2>&1; then apt-get update && apt-get install -y ca-certificates; elif command -v apk >/dev/null 2>&1; then apk add --no-cache ca-certificates; elif command -v dnf >/dev/null 2>&1; then dnf install -y ca-certificates; elif command -v yum >/dev/null 2>&1; then yum install -y ca-certificates; elif command -v microdnf >/dev/null 2>&1; then microdnf install -y ca-certificates; elif command -v zypper >/dev/null 2>&1; then zypper --non-interactive install ca-certificates; elif command -v pacman >/dev/null 2>&1; then pacman -Sy --noconfirm ca-certificates; else echo \"unsupported package manager for ca-certificates install\"; exit 1; fi";
    exec_in_container_root(runtime, container, &["sh", "-lc", install_script])
}

fn detect_patch_strategy(runtime: &ContainerRuntime, container: &str) -> Result<(String, String)> {
    let script = "if command -v update-ca-certificates >/dev/null 2>&1; then if [ -d /usr/local/share/ca-certificates ]; then echo '/usr/local/share/ca-certificates|update-ca-certificates'; else echo '/etc/ssl/certs|update-ca-certificates'; fi; elif command -v update-ca-trust >/dev/null 2>&1; then echo '/etc/pki/ca-trust/source/anchors|update-ca-trust extract'; else echo 'UNSUPPORTED'; fi";

    let output = exec_in_container_root_capture(runtime, container, &["sh", "-lc", script])?;
    let line = output.trim();

    if line == "UNSUPPORTED" || line.is_empty() {
        return Err(anyhow!(
            "container does not expose update-ca-certificates/update-ca-trust"
        ));
    }

    let mut parts = line.splitn(2, '|');
    let cert_dir = parts.next().unwrap_or_default().trim();
    let update_cmd = parts.next().unwrap_or_default().trim();

    if cert_dir.is_empty() || update_cmd.is_empty() {
        return Err(anyhow!("invalid strategy detected from container"));
    }

    Ok((cert_dir.to_string(), update_cmd.to_string()))
}

fn list_running_containers(runtime: &ContainerRuntime) -> Result<Vec<String>> {
    let output = Command::new(runtime.command())
        .args(["ps", "--format", "{{.Names}}"])
        .output()
        .with_context(|| format!("failed to execute {} ps", runtime.name()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("{} ps failed: {stderr}", runtime.name()));
    }

    let names = String::from_utf8(output.stdout)
        .with_context(|| format!("invalid UTF-8 from {} ps output", runtime.name()))?;
    Ok(names
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.to_string())
        .collect())
}

fn exec_in_container_root(
    runtime: &ContainerRuntime,
    container: &str,
    args: &[&str],
) -> Result<()> {
    let status = Command::new(runtime.command())
        .arg("exec")
        .arg("-u")
        .arg("0")
        .arg(container)
        .args(args)
        .status()
        .with_context(|| {
            format!(
                "failed to execute {} exec for container `{container}`",
                runtime.name()
            )
        })?;

    if !status.success() {
        return Err(anyhow!(
            "{} exec failed for container `{container}` with status {status}",
            runtime.name()
        ));
    }

    Ok(())
}

fn exec_in_container_root_capture(
    runtime: &ContainerRuntime,
    container: &str,
    args: &[&str],
) -> Result<String> {
    let output = Command::new(runtime.command())
        .arg("exec")
        .arg("-u")
        .arg("0")
        .arg(container)
        .args(args)
        .output()
        .with_context(|| {
            format!(
                "failed to execute {} exec for container `{container}`",
                runtime.name()
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "{} exec capture failed for container `{container}`: {stderr}",
            runtime.name()
        ));
    }

    String::from_utf8(output.stdout)
        .with_context(|| format!("invalid UTF-8 from {} exec output", runtime.name()))
}

fn write_file_in_container(
    runtime: &ContainerRuntime,
    container: &str,
    path: &str,
    content: &str,
) -> Result<()> {
    let script = format!("cat > '{path}'");
    let mut child = Command::new(runtime.command())
        .arg("exec")
        .arg("-i")
        .arg("-u")
        .arg("0")
        .arg(container)
        .args(["sh", "-lc", &script])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| {
            format!(
                "failed to execute {} exec for container `{container}`",
                runtime.name()
            )
        })?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(content.as_bytes()).with_context(|| {
            format!(
                "failed writing certificate content to {} exec stdin",
                runtime.name()
            )
        })?;
    }

    let output = child.wait_with_output().with_context(|| {
        format!(
            "failed waiting {} exec for container `{container}`",
            runtime.name()
        )
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "{} exec write failed for container `{container}`: {stderr}",
            runtime.name()
        ));
    }

    Ok(())
}
