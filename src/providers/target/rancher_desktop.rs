use super::TargetProvider;
use crate::core::plan::SyncPlan;
use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::env;
use std::io::Write;
use std::process::{Command, ExitStatus, Stdio};

const MANAGED_CERT_DIR: &str = "/usr/local/share/ca-certificates/tbridge";

pub struct RancherDesktopTarget;

impl TargetProvider for RancherDesktopTarget {
    fn name(&self) -> &'static str {
        "rancher-desktop"
    }

    fn current_fingerprints(&self) -> Result<Vec<String>> {
        let instance = instance_name();
        let script = format!(
            "if [ -d '{dir}' ]; then for f in '{dir}'/*.crt; do [ -e \"$f\" ] || continue; b=$(basename \"$f\" .crt); echo \"$b\"; done; fi",
            dir = MANAGED_CERT_DIR
        );

        let output = run_vm_capture(&instance, &["sh", "-lc", &script])?;
        let fingerprints = output
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|line| line.to_string())
            .collect();

        Ok(fingerprints)
    }

    fn apply_plan(&self, plan: &SyncPlan, dry_run: bool) -> Result<()> {
        if dry_run {
            println!(
                "[dry-run] rancher-desktop: would add {} cert(s), remove {} cert(s)",
                plan.to_add.len(),
                plan.to_remove.len()
            );
            return Ok(());
        }

        let instance = instance_name();
        ensure_managed_dir(&instance)?;

        let mut added_paths = Vec::new();
        let mut removed_backups: HashMap<String, String> = HashMap::new();

        let apply_result: Result<()> = (|| {
            for fingerprint in &plan.to_remove {
                let path = cert_path(fingerprint);
                if let Some(content) = read_remote_file(&instance, &path)? {
                    removed_backups.insert(path.clone(), content);
                }
                remove_remote_file(&instance, &path)?;
            }

            for certificate in &plan.to_add {
                let path = cert_path(&certificate.fingerprint_sha256);
                write_remote_file(&instance, &path, &certificate.pem)?;
                added_paths.push(path);
            }

            ensure_ca_update_tool(&instance)?;
            refresh_trust_store(&instance)
        })();

        if let Err(error) = apply_result {
            rollback(&instance, &added_paths, &removed_backups)?;
            return Err(error).context("apply failed and rollback executed");
        }

        println!(
            "rancher-desktop apply complete: add={}, remove={}",
            plan.to_add.len(),
            plan.to_remove.len()
        );

        Ok(())
    }

    fn verify(&self, host: Option<&str>) -> Result<()> {
        let instance = instance_name();

        if let Some(host) = host {
            let script = format!(
                "if command -v openssl >/dev/null 2>&1; then echo | openssl s_client -connect {host} -brief >/dev/null && echo 'verify: ok'; else echo 'openssl unavailable in VM'; exit 1; fi"
            );
            run_vm_capture(&instance, &["sh", "-lc", &script])?;
            println!("verify: rancher-desktop TLS check succeeded for host: {host}");
        } else {
            let script = "if [ -d '/etc/ssl/certs' ]; then echo 'verify: trust store present'; else echo 'verify: trust store missing'; exit 1; fi";
            run_vm_capture(&instance, &["sh", "-lc", script])?;
            println!("verify: rancher-desktop trust store accessible");
        }

        Ok(())
    }
}

fn instance_name() -> String {
    if let Ok(instance) = env::var("TBRIDGE_RD_INSTANCE") {
        if !instance.trim().is_empty() {
            return instance;
        }
    }

    if let Ok(list) = run_lima_list() {
        if let Some(instance) = detect_rancher_desktop_instance(&list) {
            return instance;
        }
    }

    "0".to_string()
}

fn run_lima_list() -> Result<String> {
    let output = Command::new("limactl")
        .arg("list")
        .output()
        .context("failed to execute limactl list")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("limactl list failed: {stderr}"));
    }

    String::from_utf8(output.stdout).context("invalid UTF-8 from limactl list output")
}

fn detect_rancher_desktop_instance(list_output: &str) -> Option<String> {
    let mut first_candidate: Option<String> = None;

    for line in list_output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("NAME") {
            continue;
        }

        let name = trimmed.split_whitespace().next()?.to_string();
        if first_candidate.is_none() {
            first_candidate = Some(name.clone());
        }

        let lower = name.to_ascii_lowercase();
        if lower.contains("rancher") || lower.contains("desktop") {
            return Some(name);
        }
    }

    first_candidate
}

fn cert_path(fingerprint: &str) -> String {
    format!("{MANAGED_CERT_DIR}/{fingerprint}.crt")
}

fn ensure_managed_dir(instance: &str) -> Result<()> {
    run_vm(instance, &["sudo", "mkdir", "-p", MANAGED_CERT_DIR])
}

fn remove_remote_file(instance: &str, path: &str) -> Result<()> {
    run_vm(instance, &["sudo", "rm", "-f", path])
}

fn write_remote_file(instance: &str, path: &str, content: &str) -> Result<()> {
    let script = format!("cat > '{path}'");
    run_vm_with_stdin(instance, &["sudo", "sh", "-lc", &script], content)
}

fn read_remote_file(instance: &str, path: &str) -> Result<Option<String>> {
    let script = format!("if [ -f '{path}' ]; then cat '{path}'; fi");
    let output = run_vm_capture(instance, &["sh", "-lc", &script])?;
    if output.trim().is_empty() {
        return Ok(None);
    }

    Ok(Some(output))
}

fn refresh_trust_store(instance: &str) -> Result<()> {
    let script = "if command -v update-ca-certificates >/dev/null 2>&1; then sudo update-ca-certificates; elif command -v update-ca-trust >/dev/null 2>&1; then sudo update-ca-trust extract; else echo 'no supported CA update command found'; exit 1; fi";
    run_vm(instance, &["sh", "-lc", script])
}

fn ensure_ca_update_tool(instance: &str) -> Result<()> {
    let script = "if command -v update-ca-certificates >/dev/null 2>&1 || command -v update-ca-trust >/dev/null 2>&1; then exit 0; fi; if command -v apt-get >/dev/null 2>&1; then sudo apt-get update && sudo apt-get install -y ca-certificates; elif command -v apk >/dev/null 2>&1; then sudo apk add --no-cache ca-certificates; elif command -v dnf >/dev/null 2>&1; then sudo dnf install -y ca-certificates; elif command -v yum >/dev/null 2>&1; then sudo yum install -y ca-certificates; elif command -v microdnf >/dev/null 2>&1; then sudo microdnf install -y ca-certificates; elif command -v zypper >/dev/null 2>&1; then sudo zypper --non-interactive install ca-certificates; elif command -v pacman >/dev/null 2>&1; then sudo pacman -Sy --noconfirm ca-certificates; else echo \"no supported package manager found to install ca-certificates\"; exit 1; fi";
    run_vm(instance, &["sh", "-lc", script])
}

fn rollback(
    instance: &str,
    added_paths: &[String],
    removed_backups: &HashMap<String, String>,
) -> Result<()> {
    for path in added_paths {
        remove_remote_file(instance, path)?;
    }

    for (path, content) in removed_backups {
        write_remote_file(instance, path, content)?;
    }

    let _ = refresh_trust_store(instance);
    Ok(())
}

fn run_vm(instance: &str, args: &[&str]) -> Result<()> {
    let (status, stderr, backend) = run_vm_status(instance, args)?;
    if !status.success() {
        return Err(anyhow!(
            "{backend} command failed for instance `{instance}` with status {status}: {stderr}"
        ));
    }
    Ok(())
}

fn run_vm_capture(instance: &str, args: &[&str]) -> Result<String> {
    let (stdout, status, stderr, backend) = run_vm_capture_raw(instance, args)?;
    if !status.success() {
        return Err(anyhow!(
            "{backend} command failed for instance `{instance}`: {stderr}"
        ));
    }
    Ok(stdout)
}

fn run_vm_with_stdin(instance: &str, args: &[&str], stdin_content: &str) -> Result<()> {
    if should_use_rdctl_fallback(instance)? {
        let (status, stderr) = run_rdctl_with_stdin(args, stdin_content)?;
        if !status.success() {
            return Err(anyhow!(
                "rdctl shell command failed for instance `{instance}`: {stderr}"
            ));
        }
        return Ok(());
    }

    let mut child = Command::new("limactl")
        .arg("shell")
        .arg(instance)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to execute limactl shell")?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(stdin_content.as_bytes())
            .context("failed writing stdin to limactl")?;
    }

    let output = child
        .wait_with_output()
        .context("failed waiting limactl process")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "limactl shell command failed for instance `{instance}`: {stderr}"
        ));
    }

    Ok(())
}

fn run_vm_status(instance: &str, args: &[&str]) -> Result<(ExitStatus, String, &'static str)> {
    if should_use_rdctl_fallback(instance)? {
        let (status, stderr) = run_rdctl_status(args)?;
        return Ok((status, stderr, "rdctl shell"));
    }

    let output = Command::new("limactl")
        .arg("shell")
        .arg(instance)
        .args(args)
        .output()
        .context("failed to execute limactl shell")?;
    Ok((
        output.status,
        String::from_utf8_lossy(&output.stderr).to_string(),
        "limactl shell",
    ))
}

fn run_vm_capture_raw(
    instance: &str,
    args: &[&str],
) -> Result<(String, ExitStatus, String, &'static str)> {
    if should_use_rdctl_fallback(instance)? {
        let (stdout, status, stderr) = run_rdctl_capture(args)?;
        return Ok((stdout, status, stderr, "rdctl shell"));
    }

    let output = Command::new("limactl")
        .arg("shell")
        .arg(instance)
        .args(args)
        .output()
        .context("failed to execute limactl shell")?;

    Ok((
        String::from_utf8(output.stdout).context("invalid UTF-8 from limactl output")?,
        output.status,
        String::from_utf8_lossy(&output.stderr).to_string(),
        "limactl shell",
    ))
}

fn should_use_rdctl_fallback(instance: &str) -> Result<bool> {
    if matches!(
        env::var("TBRIDGE_VM_BACKEND").ok().as_deref(),
        Some("limactl") | Some("LIMACTL")
    ) {
        return Ok(false);
    }

    let _ = instance;
    Ok(Command::new("rdctl").arg("--help").output().is_ok())
}

fn run_rdctl_status(args: &[&str]) -> Result<(ExitStatus, String)> {
    let output = Command::new("rdctl")
        .arg("shell")
        .args(args)
        .output()
        .context("failed to execute rdctl shell")?;
    Ok((
        output.status,
        String::from_utf8_lossy(&output.stderr).to_string(),
    ))
}

fn run_rdctl_capture(args: &[&str]) -> Result<(String, ExitStatus, String)> {
    let output = Command::new("rdctl")
        .arg("shell")
        .args(args)
        .output()
        .context("failed to execute rdctl shell")?;
    Ok((
        String::from_utf8(output.stdout).context("invalid UTF-8 from rdctl output")?,
        output.status,
        String::from_utf8_lossy(&output.stderr).to_string(),
    ))
}

fn run_rdctl_with_stdin(args: &[&str], stdin_content: &str) -> Result<(ExitStatus, String)> {
    let mut child = Command::new("rdctl")
        .arg("shell")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to execute rdctl shell")?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(stdin_content.as_bytes())
            .context("failed writing stdin to rdctl")?;
    }

    let output = child
        .wait_with_output()
        .context("failed waiting rdctl process")?;
    Ok((
        output.status,
        String::from_utf8_lossy(&output.stderr).to_string(),
    ))
}
