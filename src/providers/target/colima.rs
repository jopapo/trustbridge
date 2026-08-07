use super::TargetProvider;
use crate::core::plan::SyncPlan;
use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::env;
use std::io::Write;
use std::process::{Command, Stdio};

const MANAGED_CERT_DIR: &str = "/usr/local/share/ca-certificates/tbridge";

pub struct ColimaTarget;

impl TargetProvider for ColimaTarget {
    fn name(&self) -> &'static str {
        "colima"
    }

    fn current_fingerprints(&self) -> Result<Vec<String>> {
        let instance = instance_name();
        let script = format!(
            "if [ -d '{dir}' ]; then for f in '{dir}'/*.crt; do [ -e \"$f\" ] || continue; b=$(basename \"$f\" .crt); echo \"$b\"; done; fi",
            dir = MANAGED_CERT_DIR
        );

        let output = run_lima_capture(&instance, &["sh", "-lc", &script])?;
        Ok(output
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|line| line.to_string())
            .collect())
    }

    fn apply_plan(&self, plan: &SyncPlan, dry_run: bool) -> Result<()> {
        if dry_run {
            println!(
                "[dry-run] colima: would add {} cert(s), remove {} cert(s)",
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
            return Err(error).context("colima apply failed and rollback executed");
        }

        println!(
            "colima apply complete: add={}, remove={}",
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
            run_lima_capture(&instance, &["sh", "-lc", &script])?;
            println!("verify: colima TLS check succeeded for host: {host}");
        } else {
            let script = "if [ -d '/etc/ssl/certs' ]; then echo 'verify: trust store present'; else echo 'verify: trust store missing'; exit 1; fi";
            run_lima_capture(&instance, &["sh", "-lc", script])?;
            println!("verify: colima trust store accessible");
        }

        Ok(())
    }
}

fn instance_name() -> String {
    env::var("TBRIDGE_COLIMA_INSTANCE").unwrap_or_else(|_| "colima".to_string())
}

fn cert_path(fingerprint: &str) -> String {
    format!("{MANAGED_CERT_DIR}/{fingerprint}.crt")
}

fn ensure_managed_dir(instance: &str) -> Result<()> {
    run_lima(instance, &["sudo", "mkdir", "-p", MANAGED_CERT_DIR])
}

fn remove_remote_file(instance: &str, path: &str) -> Result<()> {
    run_lima(instance, &["sudo", "rm", "-f", path])
}

fn write_remote_file(instance: &str, path: &str, content: &str) -> Result<()> {
    let script = format!("cat > '{path}'");
    run_lima_with_stdin(instance, &["sudo", "sh", "-lc", &script], content)
}

fn read_remote_file(instance: &str, path: &str) -> Result<Option<String>> {
    let script = format!("if [ -f '{path}' ]; then cat '{path}'; fi");
    let output = run_lima_capture(instance, &["sh", "-lc", &script])?;
    if output.trim().is_empty() {
        return Ok(None);
    }

    Ok(Some(output))
}

fn ensure_ca_update_tool(instance: &str) -> Result<()> {
    let script = "if command -v update-ca-certificates >/dev/null 2>&1 || command -v update-ca-trust >/dev/null 2>&1; then exit 0; fi; if command -v apt-get >/dev/null 2>&1; then sudo apt-get update && sudo apt-get install -y ca-certificates; elif command -v apk >/dev/null 2>&1; then sudo apk add --no-cache ca-certificates; elif command -v dnf >/dev/null 2>&1; then sudo dnf install -y ca-certificates; elif command -v yum >/dev/null 2>&1; then sudo yum install -y ca-certificates; elif command -v microdnf >/dev/null 2>&1; then sudo microdnf install -y ca-certificates; elif command -v zypper >/dev/null 2>&1; then sudo zypper --non-interactive install ca-certificates; elif command -v pacman >/dev/null 2>&1; then sudo pacman -Sy --noconfirm ca-certificates; else echo 'no supported package manager found to install ca-certificates'; exit 1; fi";
    run_lima(instance, &["sh", "-lc", script])
}

fn refresh_trust_store(instance: &str) -> Result<()> {
    let script = "if command -v update-ca-certificates >/dev/null 2>&1; then sudo update-ca-certificates; elif command -v update-ca-trust >/dev/null 2>&1; then sudo update-ca-trust extract; else echo 'no supported CA update command found'; exit 1; fi";
    run_lima(instance, &["sh", "-lc", script])
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

fn run_lima(instance: &str, args: &[&str]) -> Result<()> {
    let output = Command::new("limactl")
        .arg("shell")
        .arg(instance)
        .args(args)
        .output()
        .context("failed to execute limactl shell")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "limactl shell failed for instance `{instance}`: {stderr}"
        ));
    }

    Ok(())
}

fn run_lima_capture(instance: &str, args: &[&str]) -> Result<String> {
    let output = Command::new("limactl")
        .arg("shell")
        .arg(instance)
        .args(args)
        .output()
        .context("failed to execute limactl shell")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "limactl shell failed for instance `{instance}`: {stderr}"
        ));
    }

    String::from_utf8(output.stdout).context("invalid UTF-8 from limactl output")
}

fn run_lima_with_stdin(instance: &str, args: &[&str], stdin_content: &str) -> Result<()> {
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
            "limactl shell failed for instance `{instance}`: {stderr}"
        ));
    }

    Ok(())
}
