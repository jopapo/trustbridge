use crate::core::certificate::Certificate;
use anyhow::{anyhow, Context, Result};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::Write;
use std::process::{Command, Stdio};

pub struct ImagePatchOptions {
    pub dry_run: bool,
    pub mode: String,
    pub include_orchestrator: bool,
    pub limit: usize,
    pub bundle_hash: String,
    pub known_hashes: HashMap<String, String>,
}

pub struct ImagePatchResult {
    pub updated_hashes: HashMap<String, String>,
    pub patched: usize,
    pub skipped: usize,
    pub failed: usize,
}

pub fn patch_images(
    certs: &[Certificate],
    options: &ImagePatchOptions,
) -> Result<ImagePatchResult> {
    let mut result = ImagePatchResult {
        updated_hashes: options.known_hashes.clone(),
        patched: 0,
        skipped: 0,
        failed: 0,
    };

    if options.mode.eq_ignore_ascii_case("none") {
        println!("image patch skipped (mode=none)");
        return Ok(result);
    }

    if certs.is_empty() {
        println!("image patch: nothing to patch (filtered set is empty)");
        return Ok(result);
    }

    let mut images = list_images()?;
    if options.mode.eq_ignore_ascii_case("user") {
        images.retain(|image| is_user_image(&image.repository));
    } else if !options.mode.eq_ignore_ascii_case("all") {
        return Err(anyhow!(
            "invalid images mode: {} (expected user|all|none)",
            options.mode
        ));
    }

    if !options.include_orchestrator {
        images.retain(|image| !is_orchestrator_image(&image.repository));
    }

    if options.limit > 0 && images.len() > options.limit {
        images.truncate(options.limit);
    }

    if images.is_empty() {
        println!("image patch: no images selected");
        return Ok(result);
    }

    println!("image patch selected images: {}", images.len());

    let suffix = bundle_suffix(certs);

    for image in images {
        let image_key = image.ref_name();
        if options
            .known_hashes
            .get(&image_key)
            .is_some_and(|hash| hash == &options.bundle_hash)
        {
            println!("- {}: skipped (already in sync)", image.ref_name());
            result.skipped += 1;
            continue;
        }

        match patch_single_image(&image, certs, options.dry_run, &suffix) {
            Ok(tag) => {
                result.patched += 1;
                if options.dry_run {
                    println!("- {}: dry-run -> {}", image.ref_name(), tag);
                } else {
                    println!("- {}: patched -> {}", image.ref_name(), tag);
                    result
                        .updated_hashes
                        .insert(image_key, options.bundle_hash.clone());
                }
            }
            Err(error) => {
                result.failed += 1;
                println!("- {}: failed ({error})", image.ref_name());
            }
        }
    }

    println!(
        "image patch summary: patched={}, skipped={}, failed={}",
        result.patched, result.skipped, result.failed
    );
    Ok(result)
}

#[derive(Clone)]
struct LocalImage {
    repository: String,
    tag: String,
}

impl LocalImage {
    fn ref_name(&self) -> String {
        format!("{}:{}", self.repository, self.tag)
    }

    fn patched_tag(&self, suffix: &str) -> String {
        format!("{}:{}-tb-{}", self.repository, self.tag, suffix)
    }
}

fn list_images() -> Result<Vec<LocalImage>> {
    let output = Command::new("docker")
        .args(["image", "ls", "--format", "{{.Repository}}|{{.Tag}}"])
        .output()
        .context("failed to execute docker image ls")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("docker image ls failed: {stderr}"));
    }

    let stdout = String::from_utf8(output.stdout).context("invalid UTF-8 from docker image ls")?;
    let mut images = Vec::new();

    for line in stdout.lines() {
        let mut parts = line.splitn(2, '|');
        let repository = parts.next().unwrap_or_default().trim();
        let tag = parts.next().unwrap_or_default().trim();

        if repository.is_empty() || tag.is_empty() || repository == "<none>" || tag == "<none>" {
            continue;
        }

        images.push(LocalImage {
            repository: repository.to_string(),
            tag: tag.to_string(),
        });
    }

    Ok(images)
}

fn is_user_image(repository: &str) -> bool {
    let lower = repository.to_ascii_lowercase();
    let blocked_exact = [
        "alpine", "ubuntu", "debian", "python", "node", "busybox", "nginx", "redis", "postgres",
        "mysql", "golang", "rust", "openjdk",
    ];

    if blocked_exact.contains(&lower.as_str()) {
        return false;
    }

    let blocked_prefixes = [
        "mcr.microsoft.com/",
        "gcr.io/",
        "k8s.gcr.io/",
        "registry.k8s.io/",
        "quay.io/",
        "ghcr.io/",
    ];

    !blocked_prefixes
        .iter()
        .any(|prefix| lower.starts_with(prefix))
}

fn is_orchestrator_image(repository: &str) -> bool {
    let lower = repository.to_ascii_lowercase();
    lower.contains("rancher")
        || lower.contains("k3s")
        || lower.contains("kubernetes")
        || lower.contains("coredns")
        || lower.contains("traefik")
        || lower.starts_with("registry.k8s.io/")
        || lower.starts_with("k8s.gcr.io/")
}

fn bundle_suffix(certs: &[Certificate]) -> String {
    let mut fingerprints: Vec<String> = certs
        .iter()
        .map(|certificate| certificate.fingerprint_sha256.clone())
        .collect();
    fingerprints.sort();

    let mut hasher = Sha256::new();
    for fingerprint in fingerprints {
        hasher.update(fingerprint.as_bytes());
    }

    let digest = format!("{:x}", hasher.finalize());
    digest[..8].to_string()
}

fn patch_single_image(
    image: &LocalImage,
    certs: &[Certificate],
    dry_run: bool,
    suffix: &str,
) -> Result<String> {
    let source_ref = image.ref_name();
    let target_ref = image.patched_tag(suffix);

    if dry_run {
        return Ok(target_ref);
    }

    let container_id = create_patch_container(&source_ref)?;
    let result = patch_image_container(&container_id, certs)
        .and_then(|_| commit_image(&container_id, &target_ref));
    let cleanup_result = remove_container(&container_id);

    if let Err(error) = result {
        let _ = cleanup_result;
        return Err(error);
    }

    cleanup_result?;
    Ok(target_ref)
}

fn create_patch_container(image_ref: &str) -> Result<String> {
    let output = Command::new("docker")
        .args([
            "create",
            "--entrypoint",
            "/bin/sh",
            image_ref,
            "-c",
            "while true; do sleep 3600; done",
        ])
        .output()
        .with_context(|| format!("failed to create temp container for image `{image_ref}`"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("docker create failed for `{image_ref}`: {stderr}"));
    }

    let container_id = String::from_utf8(output.stdout)
        .context("invalid UTF-8 from docker create output")?
        .trim()
        .to_string();

    let status = Command::new("docker")
        .args(["start", &container_id])
        .status()
        .with_context(|| format!("failed to start temp container `{container_id}`"))?;

    if !status.success() {
        return Err(anyhow!(
            "docker start failed for temp container `{container_id}`"
        ));
    }

    Ok(container_id)
}

fn patch_image_container(container_id: &str, certs: &[Certificate]) -> Result<()> {
    ensure_ca_update_tool(container_id)?;
    let (cert_dir, update_cmd) = detect_patch_strategy(container_id)?;
    exec_in_container(
        container_id,
        &["sh", "-lc", &format!("mkdir -p '{cert_dir}'")],
    )?;

    for certificate in certs {
        let path = format!("{cert_dir}/{}.crt", certificate.fingerprint_sha256);
        write_file_in_container(container_id, &path, &certificate.pem)?;
    }

    exec_in_container(container_id, &["sh", "-lc", &update_cmd])
}

fn ensure_ca_update_tool(container_id: &str) -> Result<()> {
    let install_script = "if command -v update-ca-certificates >/dev/null 2>&1 || command -v update-ca-trust >/dev/null 2>&1; then exit 0; fi; if command -v apt-get >/dev/null 2>&1; then apt-get update && apt-get install -y ca-certificates; elif command -v apk >/dev/null 2>&1; then apk add --no-cache ca-certificates; elif command -v dnf >/dev/null 2>&1; then dnf install -y ca-certificates; elif command -v yum >/dev/null 2>&1; then yum install -y ca-certificates; elif command -v microdnf >/dev/null 2>&1; then microdnf install -y ca-certificates; elif command -v zypper >/dev/null 2>&1; then zypper --non-interactive install ca-certificates; elif command -v pacman >/dev/null 2>&1; then pacman -Sy --noconfirm ca-certificates; else echo \"unsupported package manager for ca-certificates install\"; exit 1; fi";
    exec_in_container(container_id, &["sh", "-lc", install_script])
}

fn detect_patch_strategy(container_id: &str) -> Result<(String, String)> {
    let script = "if command -v update-ca-certificates >/dev/null 2>&1; then if [ -d /usr/local/share/ca-certificates ]; then echo '/usr/local/share/ca-certificates|update-ca-certificates'; else echo '/etc/ssl/certs|update-ca-certificates'; fi; elif command -v update-ca-trust >/dev/null 2>&1; then echo '/etc/pki/ca-trust/source/anchors|update-ca-trust extract'; else echo 'UNSUPPORTED'; fi";

    let output = exec_in_container_capture(container_id, &["sh", "-lc", script])?;
    let line = output.trim();
    if line == "UNSUPPORTED" || line.is_empty() {
        return Err(anyhow!(
            "image does not expose update-ca-certificates/update-ca-trust"
        ));
    }

    let mut parts = line.splitn(2, '|');
    let cert_dir = parts.next().unwrap_or_default().trim();
    let update_cmd = parts.next().unwrap_or_default().trim();
    if cert_dir.is_empty() || update_cmd.is_empty() {
        return Err(anyhow!("invalid patch strategy detected in image"));
    }

    Ok((cert_dir.to_string(), update_cmd.to_string()))
}

fn commit_image(container_id: &str, target_ref: &str) -> Result<()> {
    let status = Command::new("docker")
        .args(["commit", container_id, target_ref])
        .status()
        .with_context(|| format!("failed to commit temp container `{container_id}`"))?;

    if !status.success() {
        return Err(anyhow!(
            "docker commit failed for container `{container_id}`"
        ));
    }

    Ok(())
}

fn remove_container(container_id: &str) -> Result<()> {
    let status = Command::new("docker")
        .args(["rm", "-f", container_id])
        .status()
        .with_context(|| format!("failed to remove temp container `{container_id}`"))?;

    if !status.success() {
        return Err(anyhow!(
            "docker rm failed for temp container `{container_id}`"
        ));
    }

    Ok(())
}

fn exec_in_container(container_id: &str, args: &[&str]) -> Result<()> {
    let status = Command::new("docker")
        .arg("exec")
        .arg("-u")
        .arg("0")
        .arg(container_id)
        .args(args)
        .status()
        .with_context(|| format!("failed to execute docker exec for `{container_id}`"))?;

    if !status.success() {
        return Err(anyhow!(
            "docker exec failed for `{container_id}` with status {status}"
        ));
    }

    Ok(())
}

fn exec_in_container_capture(container_id: &str, args: &[&str]) -> Result<String> {
    let output = Command::new("docker")
        .arg("exec")
        .arg("-u")
        .arg("0")
        .arg(container_id)
        .args(args)
        .output()
        .with_context(|| format!("failed to execute docker exec for `{container_id}`"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "docker exec capture failed for `{container_id}`: {stderr}"
        ));
    }

    String::from_utf8(output.stdout).context("invalid UTF-8 from docker exec output")
}

fn write_file_in_container(container_id: &str, path: &str, content: &str) -> Result<()> {
    let script = format!("cat > '{path}'");
    let mut child = Command::new("docker")
        .arg("exec")
        .arg("-i")
        .arg("-u")
        .arg("0")
        .arg(container_id)
        .args(["sh", "-lc", &script])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to execute docker exec for `{container_id}`"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(content.as_bytes())
            .context("failed writing cert content to docker exec stdin")?;
    }

    let output = child
        .wait_with_output()
        .with_context(|| format!("failed waiting docker exec for `{container_id}`"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "docker exec write failed for `{container_id}`: {stderr}"
        ));
    }

    Ok(())
}
