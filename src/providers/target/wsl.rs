use super::vm_backend::{current_wsl_distro, is_wsl, list_wsl_distros, VmBackend};
use super::TargetProvider;
use crate::core::plan::SyncPlan;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::env;

const MANAGED_CERT_DIR: &str = "/usr/local/share/ca-certificates/tbridge";

pub struct WslTarget;

impl TargetProvider for WslTarget {
    fn name(&self) -> &'static str {
        "wsl"
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
            .map(str::to_string)
            .collect())
    }

    fn apply_plan(&self, plan: &SyncPlan, dry_run: bool) -> Result<()> {
        if dry_run {
            println!(
                "[dry-run] wsl: would add {} cert(s), remove {} cert(s)",
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
            return Err(error).context("wsl apply failed and rollback executed");
        }

        println!(
            "wsl apply complete: add={}, remove={}",
            plan.to_add.len(),
            plan.to_remove.len()
        );

        Ok(())
    }

    fn verify(&self, host: Option<&str>) -> Result<()> {
        let backend = resolve_backend()?;

        if let Some(host) = host {
            let script = format!(
                "if command -v openssl >/dev/null 2>&1; then echo | openssl s_client -connect {host} -brief >/dev/null && echo 'verify: ok'; else echo 'openssl unavailable in WSL'; exit 1; fi"
            );
            backend.capture(&script)?;
            println!("verify: wsl TLS check succeeded for host: {host}");
        } else {
            let script = "if [ -d '/etc/ssl/certs' ]; then echo 'verify: trust store present'; else echo 'verify: trust store missing'; exit 1; fi";
            backend.capture(script)?;
            println!("verify: wsl trust store accessible");
        }

        Ok(())
    }
}

fn resolve_backend() -> Result<VmBackend> {
    if let Ok(distro) = env::var("TBRIDGE_WSL_DISTRO") {
        if !distro.trim().is_empty() {
            return Ok(VmBackend::WslDistro { distro });
        }
    }

    if is_wsl() && current_wsl_distro().is_some() {
        return Ok(VmBackend::Local);
    }

    if let Some(distro) = infer_wsl_distro()? {
        return Ok(VmBackend::WslDistro { distro });
    }

    anyhow::bail!(
        "wsl target requires running inside WSL, setting TBRIDGE_WSL_DISTRO, or having an installed WSL distro"
    )
}

fn infer_wsl_distro() -> Result<Option<String>> {
    let distros = list_wsl_distros()?;
    if distros.is_empty() {
        return Ok(None);
    }

    let preferred = distros.iter().find(|name| {
        let lower = name.to_ascii_lowercase();
        lower != "docker-desktop"
            && lower != "docker-desktop-data"
            && lower != "rancher-desktop"
            && lower != "rancher-desktop-data"
    });

    Ok(preferred.cloned().or_else(|| distros.first().cloned()))
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
    let output = backend.capture(&format!("if [ -f '{path}' ]; then cat '{path}'; fi"))?;
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
