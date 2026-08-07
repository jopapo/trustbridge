use crate::commands::filtering::FilterStats;
use crate::core::certificate::Certificate;
use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::io::{self, Write};
use std::process::{Command, Stdio};

pub struct WorkloadPatchOptions {
    pub dry_run: bool,
    pub interactive: bool,
    pub containers: Vec<String>,
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
    println!("workload patch filter result: kept {} / dropped {}", stats.kept, stats.dropped);
    println!("workload patch certificates selected: {}", certs.len());

    let mut result = WorkloadPatchResult {
        updated_hashes: options.known_hashes.clone(),
        patched: 0,
        skipped: 0,
        failed: 0,
    };

    if certs.is_empty() {
        println!("workload patch: nothing to patch (filtered set is empty)");
        return Ok(result);
    }

    let containers = if options.containers.is_empty() {
        list_running_containers()?
    } else {
        options.containers.clone()
    };

    if containers.is_empty() {
        println!("workload patch: no running containers selected");
        return Ok(result);
    }

    println!("workload patch containers selected: {}", containers.len());

    for container in containers {
        if options
            .known_hashes
            .get(&container)
            .is_some_and(|hash| hash == &options.bundle_hash)
        {
            println!("- {container}: skipped (already in sync)");
            result.skipped += 1;
            continue;
        }

        if options.interactive && !confirm_container(&container)? {
            println!("- {container}: skipped by user");
            result.skipped += 1;
            continue;
        }

        match patch_container(&container, certs, options.dry_run) {
            Ok(_) => {
                result.patched += 1;
                println!("- {container}: patched");
                if !options.dry_run {
                    result
                        .updated_hashes
                        .insert(container.clone(), options.bundle_hash.clone());
                }
            }
            Err(error) => {
                result.failed += 1;
                println!("- {container}: failed ({error})");
            }
        }
    }

    println!(
        "workload patch summary: patched={}, skipped={}, failed={}",
        result.patched, result.skipped, result.failed
    );

    if result.failed > 0 {
        return Err(anyhow!("one or more containers failed to patch"));
    }

    Ok(result)
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

fn patch_container(container: &str, certs: &[Certificate], dry_run: bool) -> Result<()> {
    let (cert_dir, update_cmd) = detect_patch_strategy(container)?;

    if dry_run {
        println!(
            "  [dry-run] strategy: dir={} update='{}' certs={}",
            cert_dir,
            update_cmd,
            certs.len()
        );
        return Ok(());
    }

    exec_in_container_root(container, &["sh", "-lc", &format!("mkdir -p '{cert_dir}'")])?;

    for certificate in certs {
        let path = format!("{cert_dir}/{}.crt", certificate.fingerprint_sha256);
        write_file_in_container(container, &path, &certificate.pem)?;
    }

    exec_in_container_root(container, &["sh", "-lc", &update_cmd])?;
    Ok(())
}

fn detect_patch_strategy(container: &str) -> Result<(String, String)> {
    let script = "if command -v update-ca-certificates >/dev/null 2>&1; then if [ -d /usr/local/share/ca-certificates ]; then echo '/usr/local/share/ca-certificates|update-ca-certificates'; else echo '/etc/ssl/certs|update-ca-certificates'; fi; elif command -v update-ca-trust >/dev/null 2>&1; then echo '/etc/pki/ca-trust/source/anchors|update-ca-trust extract'; else echo 'UNSUPPORTED'; fi";

    let output = exec_in_container_root_capture(container, &["sh", "-lc", script])?;
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

fn list_running_containers() -> Result<Vec<String>> {
    let output = Command::new("docker")
        .args(["ps", "--format", "{{.Names}}"])
        .output()
        .context("failed to execute docker ps")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("docker ps failed: {stderr}"));
    }

    let names = String::from_utf8(output.stdout).context("invalid UTF-8 from docker ps output")?;
    Ok(names
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.to_string())
        .collect())
}

fn exec_in_container_root(container: &str, args: &[&str]) -> Result<()> {
    let status = Command::new("docker")
        .arg("exec")
        .arg("-u")
        .arg("0")
        .arg(container)
        .args(args)
        .status()
        .with_context(|| format!("failed to execute docker exec for container `{container}`"))?;

    if !status.success() {
        return Err(anyhow!(
            "docker exec failed for container `{container}` with status {status}"
        ));
    }

    Ok(())
}

fn exec_in_container_root_capture(container: &str, args: &[&str]) -> Result<String> {
    let output = Command::new("docker")
        .arg("exec")
        .arg("-u")
        .arg("0")
        .arg(container)
        .args(args)
        .output()
        .with_context(|| format!("failed to execute docker exec for container `{container}`"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "docker exec capture failed for container `{container}`: {stderr}"
        ));
    }

    String::from_utf8(output.stdout).context("invalid UTF-8 from docker exec output")
}

fn write_file_in_container(container: &str, path: &str, content: &str) -> Result<()> {
    let script = format!("cat > '{path}'");
    let mut child = Command::new("docker")
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
        .with_context(|| format!("failed to execute docker exec for container `{container}`"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(content.as_bytes())
            .context("failed writing certificate content to docker exec stdin")?;
    }

    let output = child
        .wait_with_output()
        .with_context(|| format!("failed waiting docker exec for container `{container}`"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "docker exec write failed for container `{container}`: {stderr}"
        ));
    }

    Ok(())
}
