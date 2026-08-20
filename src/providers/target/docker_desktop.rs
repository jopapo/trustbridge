use super::vm_backend::{is_wsl, list_wsl_distros, VmBackend};
use super::TargetProvider;
use crate::core::plan::SyncPlan;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::env;

const MANAGED_CERT_DIR: &str = "/usr/local/share/ca-certificates/tbridge";

/// Docker Desktop's WSL2-backed Linux VM (Windows only). On macOS/Linux,
/// Docker Desktop does not expose an equivalent shell-accessible VM.
pub struct DockerDesktopTarget;

impl TargetProvider for DockerDesktopTarget {
    fn name(&self) -> &'static str {
        "docker-desktop"
    }

    fn current_fingerprints(&self) -> Result<Vec<String>> {
        let backend = resolve_backend()?;
        let script = format!(
            "if [ -d '{dir}' ]; then for f in '{dir}'/*.crt; do [ -e \"$f\" ] || continue; b=$(basename \"$f\" .crt); echo \"$b\"; done; fi",
            dir = MANAGED_CERT_DIR
        );

        let output = backend.capture(&script)?;
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
                "[dry-run] docker-desktop: would add {} cert(s), remove {} cert(s)",
                plan.to_add.len(),
                plan.to_remove.len()
            );
            return Ok(());
        }

        let backend = resolve_backend()?;
        ensure_managed_dir(&backend)?;

        let mut added_paths = Vec::new();
        let mut removed_backups: HashMap<String, String> = HashMap::new();

        let apply_result: Result<()> = (|| {
            for fingerprint in &plan.to_remove {
                let path = cert_path(fingerprint);
                if let Some(content) = read_remote_file(&backend, &path)? {
                    removed_backups.insert(path.clone(), content);
                }
                remove_remote_file(&backend, &path)?;
            }

            for certificate in &plan.to_add {
                let path = cert_path(&certificate.fingerprint_sha256);
                write_remote_file(&backend, &path, &certificate.pem)?;
                added_paths.push(path);
            }

            ensure_ca_update_tool(&backend)?;
            refresh_trust_store(&backend)
        })();

        if let Err(error) = apply_result {
            rollback(&backend, &added_paths, &removed_backups)?;
            return Err(error).context("docker-desktop apply failed and rollback executed");
        }

        println!(
            "docker-desktop apply complete: add={}, remove={}",
            plan.to_add.len(),
            plan.to_remove.len()
        );

        Ok(())
    }

    fn verify(&self, host: Option<&str>) -> Result<()> {
        let backend = resolve_backend()?;

        if let Some(host) = host {
            let script = format!(
                "if command -v openssl >/dev/null 2>&1; then echo | openssl s_client -connect {host} -brief >/dev/null && echo 'verify: ok'; else echo 'openssl unavailable in VM'; exit 1; fi"
            );
            backend.capture(&script)?;
            println!("verify: docker-desktop TLS check succeeded for host: {host}");
        } else {
            let script = "if [ -d '/etc/ssl/certs' ]; then echo 'verify: trust store present'; else echo 'verify: trust store missing'; exit 1; fi";
            backend.capture(script)?;
            println!("verify: docker-desktop trust store accessible");
        }

        Ok(())
    }
}

fn resolve_backend() -> Result<VmBackend> {
    if !(cfg!(target_os = "windows") || is_wsl()) {
        anyhow::bail!(
            "docker-desktop target is only supported on Windows or WSL (Docker Desktop's WSL2 backend)"
        );
    }

    Ok(VmBackend::WslDistro {
        distro: instance_name(),
    })
}

fn instance_name() -> String {
    if let Ok(distro) = env::var("TBRIDGE_DOCKER_DESKTOP_INSTANCE") {
        if !distro.trim().is_empty() {
            return distro;
        }
    }

    if let Ok(distros) = list_wsl_distros() {
        if distros.iter().any(|d| d == "docker-desktop") {
            return "docker-desktop".to_string();
        }
    }

    "docker-desktop".to_string()
}

fn cert_path(fingerprint: &str) -> String {
    format!("{MANAGED_CERT_DIR}/{fingerprint}.crt")
}

fn ensure_managed_dir(backend: &VmBackend) -> Result<()> {
    backend.run(&format!("mkdir -p '{MANAGED_CERT_DIR}'"))
}

fn remove_remote_file(backend: &VmBackend, path: &str) -> Result<()> {
    backend.run(&format!("rm -f '{path}'"))
}

fn write_remote_file(backend: &VmBackend, path: &str, content: &str) -> Result<()> {
    backend.run_with_stdin(&format!("cat > '{path}'"), content)
}

fn read_remote_file(backend: &VmBackend, path: &str) -> Result<Option<String>> {
    let script = format!("if [ -f '{path}' ]; then cat '{path}'; fi");
    let output = backend.capture(&script)?;
    if output.trim().is_empty() {
        return Ok(None);
    }

    Ok(Some(output))
}

fn refresh_trust_store(backend: &VmBackend) -> Result<()> {
    let script = "if command -v update-ca-certificates >/dev/null 2>&1; then update-ca-certificates; elif command -v update-ca-trust >/dev/null 2>&1; then update-ca-trust extract; else echo 'no supported CA update command found'; exit 1; fi";
    backend.run(script)
}

fn ensure_ca_update_tool(backend: &VmBackend) -> Result<()> {
    let script = "if command -v update-ca-certificates >/dev/null 2>&1 || command -v update-ca-trust >/dev/null 2>&1; then exit 0; fi; if command -v apt-get >/dev/null 2>&1; then apt-get update && apt-get install -y ca-certificates; elif command -v apk >/dev/null 2>&1; then apk add --no-cache ca-certificates; elif command -v dnf >/dev/null 2>&1; then dnf install -y ca-certificates; elif command -v yum >/dev/null 2>&1; then yum install -y ca-certificates; elif command -v microdnf >/dev/null 2>&1; then microdnf install -y ca-certificates; elif command -v zypper >/dev/null 2>&1; then zypper --non-interactive install ca-certificates; elif command -v pacman >/dev/null 2>&1; then pacman -Sy --noconfirm ca-certificates; else echo \"no supported package manager found to install ca-certificates\"; exit 1; fi";
    backend.run(script)
}

fn rollback(
    backend: &VmBackend,
    added_paths: &[String],
    removed_backups: &HashMap<String, String>,
) -> Result<()> {
    for path in added_paths {
        remove_remote_file(backend, path)?;
    }

    for (path, content) in removed_backups {
        write_remote_file(backend, path, content)?;
    }

    let _ = refresh_trust_store(backend);
    Ok(())
}
