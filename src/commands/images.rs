use crate::commands::container_runtime::ContainerRuntime;
use crate::core::certificate::Certificate;
use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::io::Write;
use std::process::{Command, Stdio};

pub struct ImagePatchOptions {
    pub dry_run: bool,
    pub verbose: bool,
    pub mode: String,
    pub include_orchestrator: bool,
    pub limit: usize,
    pub bundle_hash: String,
    pub known_hashes: HashMap<String, String>,
    pub retag_original: bool,
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
        if options.verbose {
            println!("image patch skipped (mode=none)");
        }
        return Ok(result);
    }

    if certs.is_empty() {
        if options.verbose {
            println!("image patch: nothing to patch (filtered set is empty)");
        }
        return Ok(result);
    }

    let runtime = ContainerRuntime::detect()?;
    let mut images = list_images(&runtime)?;
    // don't re-patch our own derived images, or the suffix compounds and eventually
    // produces an invalid (too long) tag
    images.retain(|image| !is_trustbridge_derived_tag(&image.tag));
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
        return Ok(result);
    }

    for image in images {
        let image_key = image.ref_name();
        // keyed by image ID, not repo:tag: a tag can be silently repointed to a fresh
        // (unpatched) pull without us ever seeing that as a change otherwise
        if options
            .known_hashes
            .get(&image.id)
            .is_some_and(|hash| hash == &options.bundle_hash)
        {
            result.skipped += 1;
            if !options.dry_run {
                if options.retag_original {
                    if let Err(error) = retag_image(&runtime, &image.id, &image_key) {
                        if options.verbose {
                            println!("- {}: retag failed ({error})", image.ref_name());
                        }
                    }
                } else if let Err(error) =
                    retag_image(&runtime, &image.id, &image.stable_alias_tag())
                {
                    if options.verbose {
                        println!("- {}: alias retag failed ({error})", image.ref_name());
                    }
                }
            }
            continue;
        }

        match patch_single_image(
            &runtime,
            &image,
            certs,
            options.dry_run,
            options.retag_original,
        ) {
            Ok((tag, new_id)) => {
                result.patched += 1;
                if options.dry_run && options.verbose {
                    println!("- {}: dry-run -> {}", image.ref_name(), tag);
                } else if options.verbose {
                    println!("- {}: patched -> {}", image.ref_name(), tag);
                }
                if !options.dry_run {
                    result
                        .updated_hashes
                        .insert(new_id, options.bundle_hash.clone());
                }
            }
            Err(error) => {
                result.failed += 1;
                if options.verbose {
                    println!("- {}: failed ({error})", image.ref_name());
                }
            }
        }
    }

    if options.verbose {
        println!(
            "image patch summary: patched={}, skipped={}, failed={}",
            result.patched, result.skipped, result.failed
        );
    }
    Ok(result)
}

#[derive(Clone)]
struct LocalImage {
    repository: String,
    tag: String,
    id: String,
}

impl LocalImage {
    fn ref_name(&self) -> String {
        format!("{}:{}", self.repository, self.tag)
    }

    // hash-free alias that always points at the latest patched build; unlike retagging over
    // the original tag, nothing else (e.g. an upstream `docker pull`) will ever overwrite this
    fn stable_alias_tag(&self) -> String {
        format!("{}:{}-tbridge", self.repository, self.tag)
    }
}

fn list_images(runtime: &ContainerRuntime) -> Result<Vec<LocalImage>> {
    let output = Command::new(runtime.command())
        .args([
            "image",
            "ls",
            "--no-trunc",
            "--format",
            "{{.Repository}}|{{.Tag}}|{{.ID}}",
        ])
        .output()
        .with_context(|| format!("failed to execute {} image ls", runtime.name()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("{} image ls failed: {stderr}", runtime.name()));
    }

    let stdout = String::from_utf8(output.stdout)
        .with_context(|| format!("invalid UTF-8 from {} image ls", runtime.name()))?;
    let mut images = Vec::new();

    for line in stdout.lines() {
        let mut parts = line.splitn(3, '|');
        let repository = parts.next().unwrap_or_default().trim();
        let tag = parts.next().unwrap_or_default().trim();
        let id = parts.next().unwrap_or_default().trim();

        if repository.is_empty() || tag.is_empty() || repository == "<none>" || tag == "<none>" {
            continue;
        }

        images.push(LocalImage {
            repository: repository.to_string(),
            tag: tag.to_string(),
            id: id.trim_start_matches("sha256:").to_string(),
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

fn is_trustbridge_derived_tag(tag: &str) -> bool {
    if tag.ends_with("-tbridge") {
        return true;
    }
    match tag.rsplit_once("-tb-") {
        Some((_, suffix)) => suffix.len() == 8 && suffix.chars().all(|c| c.is_ascii_hexdigit()),
        None => false,
    }
}

fn json_string_array(values: &[String]) -> String {
    serde_json::to_string(values).unwrap_or_else(|_| "[]".to_string())
}

fn patch_single_image(
    runtime: &ContainerRuntime,
    image: &LocalImage,
    certs: &[Certificate],
    dry_run: bool,
    retag_original: bool,
) -> Result<(String, String)> {
    let source_ref = image.ref_name();
    let target_ref = if retag_original {
        source_ref.clone()
    } else {
        image.stable_alias_tag()
    };

    if dry_run {
        return Ok((target_ref, image.id.clone()));
    }

    let container_id = create_patch_container(runtime, &source_ref)?;
    let result = patch_image_container(runtime, &container_id, certs)
        .and_then(|_| original_entrypoint_cmd(runtime, &source_ref))
        .and_then(|(entrypoint, cmd)| {
            commit_image(runtime, &container_id, &target_ref, &entrypoint, &cmd)
        });
    let cleanup_result = remove_container(runtime, &container_id);

    if let Err(error) = result {
        let _ = cleanup_result;
        return Err(error);
    }

    cleanup_result?;
    Ok((target_ref, result.unwrap()))
}

fn create_patch_container(runtime: &ContainerRuntime, image_ref: &str) -> Result<String> {
    let output = Command::new(runtime.command())
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
        return Err(anyhow!(
            "{} create failed for `{image_ref}`: {stderr}",
            runtime.name()
        ));
    }

    let container_id = String::from_utf8(output.stdout)
        .with_context(|| format!("invalid UTF-8 from {} create output", runtime.name()))?
        .trim()
        .to_string();

    let status = Command::new(runtime.command())
        .args(["start", &container_id])
        .stdout(Stdio::null())
        .status()
        .with_context(|| format!("failed to start temp container `{container_id}`"))?;

    if !status.success() {
        return Err(anyhow!(
            "{} start failed for temp container `{container_id}`",
            runtime.name()
        ));
    }

    Ok(container_id)
}

fn retag_image(runtime: &ContainerRuntime, patched_ref: &str, original_ref: &str) -> Result<()> {
    let status = Command::new(runtime.command())
        .args(["tag", patched_ref, original_ref])
        .stdout(Stdio::null())
        .status()
        .with_context(|| format!("failed to tag `{patched_ref}` as `{original_ref}`"))?;

    if !status.success() {
        return Err(anyhow!(
            "{} tag failed for `{patched_ref}` -> `{original_ref}`",
            runtime.name()
        ));
    }

    Ok(())
}

fn patch_image_container(
    runtime: &ContainerRuntime,
    container_id: &str,
    certs: &[Certificate],
) -> Result<()> {
    ensure_ca_update_tool(runtime, container_id)?;
    let (cert_dir, update_cmd) = detect_patch_strategy(runtime, container_id)?;
    exec_in_container(
        runtime,
        container_id,
        &["sh", "-lc", &format!("mkdir -p '{cert_dir}'")],
    )?;

    for certificate in certs {
        let path = format!("{cert_dir}/{}.crt", certificate.fingerprint_sha256);
        write_file_in_container(runtime, container_id, &path, &certificate.pem)?;
    }

    exec_in_container(runtime, container_id, &["sh", "-lc", &update_cmd])?;

    // JVM ships its own cacerts store; update-ca-certificates never touches it.
    import_into_java_truststore(runtime, container_id, &cert_dir)
}

fn import_into_java_truststore(
    runtime: &ContainerRuntime,
    container_id: &str,
    cert_dir: &str,
) -> Result<()> {
    let script = format!(
        "KEYTOOL=''; \
         for candidate in \"$JAVA_HOME/bin/keytool\" /opt/jdk/*/bin/keytool /opt/java/openjdk/bin/keytool /usr/lib/jvm/*/bin/keytool /opt/openjdk*/bin/keytool; do \
           if [ -x \"$candidate\" ]; then KEYTOOL=\"$candidate\"; break; fi; \
         done; \
         if [ -z \"$KEYTOOL\" ]; then KEYTOOL=$(command -v keytool 2>/dev/null || true); fi; \
         if [ -n \"$KEYTOOL\" ]; then \
           CACERTS=''; \
           for candidate in \"$JAVA_HOME/lib/security/cacerts\" /opt/jdk/*/lib/security/cacerts /opt/java/openjdk/lib/security/cacerts /usr/lib/jvm/*/lib/security/cacerts /opt/openjdk*/lib/security/cacerts; do \
             if [ -f \"$candidate\" ]; then CACERTS=\"$candidate\"; break; fi; \
           done; \
           if [ -z \"$CACERTS\" ]; then CACERTS=$(find /usr/lib/jvm /opt -maxdepth 5 -name cacerts 2>/dev/null | head -n1); fi; \
           if [ -n \"$CACERTS\" ]; then \
             for f in {cert_dir}/*.crt; do \
               [ -f \"$f\" ] || continue; \
               alias=$(basename \"$f\" .crt); \
               \"$KEYTOOL\" -importcert -noprompt -trustcacerts -alias \"tbridge-$alias\" -file \"$f\" -keystore \"$CACERTS\" -storepass changeit >/dev/null 2>&1 || true; \
             done; \
           fi; \
         fi"
    );
    exec_in_container(runtime, container_id, &["sh", "-lc", &script])
}

fn ensure_ca_update_tool(runtime: &ContainerRuntime, container_id: &str) -> Result<()> {
    let install_script = "if command -v update-ca-certificates >/dev/null 2>&1 || command -v update-ca-trust >/dev/null 2>&1; then exit 0; fi; if command -v apt-get >/dev/null 2>&1; then apt-get update && apt-get install -y ca-certificates; elif command -v apk >/dev/null 2>&1; then apk add --no-cache ca-certificates; elif command -v dnf >/dev/null 2>&1; then dnf install -y ca-certificates; elif command -v yum >/dev/null 2>&1; then yum install -y ca-certificates; elif command -v microdnf >/dev/null 2>&1; then microdnf install -y ca-certificates; elif command -v zypper >/dev/null 2>&1; then zypper --non-interactive install ca-certificates; elif command -v pacman >/dev/null 2>&1; then pacman -Sy --noconfirm ca-certificates; else echo \"unsupported package manager for ca-certificates install\"; exit 1; fi";
    exec_in_container(runtime, container_id, &["sh", "-lc", install_script])
}

fn detect_patch_strategy(
    runtime: &ContainerRuntime,
    container_id: &str,
) -> Result<(String, String)> {
    let script = "if command -v update-ca-certificates >/dev/null 2>&1; then if [ -d /usr/local/share/ca-certificates ]; then echo '/usr/local/share/ca-certificates|update-ca-certificates'; else echo '/etc/ssl/certs|update-ca-certificates'; fi; elif command -v update-ca-trust >/dev/null 2>&1; then echo '/etc/pki/ca-trust/source/anchors|update-ca-trust extract'; else echo 'UNSUPPORTED'; fi";

    let output = exec_in_container_capture(runtime, container_id, &["sh", "-lc", script])?;
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

// docker's own config JSON uses `null` for an unset Entrypoint/Cmd
fn original_entrypoint_cmd(
    runtime: &ContainerRuntime,
    image_ref: &str,
) -> Result<(Option<Vec<String>>, Option<Vec<String>>)> {
    let output = Command::new(runtime.command())
        .args([
            "inspect",
            "--format",
            "{{json .Config.Entrypoint}}|||{{json .Config.Cmd}}",
            image_ref,
        ])
        .output()
        .with_context(|| format!("failed to inspect image `{image_ref}`"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "{} inspect failed for `{image_ref}`: {stderr}",
            runtime.name()
        ));
    }

    let stdout = String::from_utf8(output.stdout)
        .with_context(|| format!("invalid UTF-8 from {} inspect output", runtime.name()))?;
    let mut parts = stdout.trim().splitn(2, "|||");
    let entrypoint_json = parts.next().unwrap_or("null");
    let cmd_json = parts.next().unwrap_or("null");

    let entrypoint: Option<Vec<String>> = serde_json::from_str(entrypoint_json)
        .with_context(|| format!("invalid Entrypoint JSON for `{image_ref}`: {entrypoint_json}"))?;
    let cmd: Option<Vec<String>> = serde_json::from_str(cmd_json)
        .with_context(|| format!("invalid Cmd JSON for `{image_ref}`: {cmd_json}"))?;

    Ok((entrypoint, cmd))
}

fn commit_image(
    runtime: &ContainerRuntime,
    container_id: &str,
    target_ref: &str,
    entrypoint: &Option<Vec<String>>,
    cmd: &Option<Vec<String>>,
) -> Result<String> {
    let mut command = Command::new(runtime.command());
    command.arg("commit");

    // restore the source image's own Entrypoint/Cmd: our temp patch container overrode both
    // with an infinite sleep, and `docker commit` would otherwise bake that in permanently
    command.arg("--change").arg(format!(
        "ENTRYPOINT {}",
        entrypoint
            .as_ref()
            .map_or("[]".to_string(), |value| json_string_array(value))
    ));
    command.arg("--change").arg(format!(
        "CMD {}",
        cmd.as_ref()
            .map_or("[]".to_string(), |value| json_string_array(value))
    ));

    let output = command
        .args([container_id, target_ref])
        .output()
        .with_context(|| format!("failed to commit temp container `{container_id}`"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "{} commit failed for container `{container_id}`: {stderr}",
            runtime.name()
        ));
    }

    let new_id = String::from_utf8(output.stdout)
        .with_context(|| format!("invalid UTF-8 from {} commit output", runtime.name()))?
        .trim()
        .trim_start_matches("sha256:")
        .to_string();

    Ok(new_id)
}

fn remove_container(runtime: &ContainerRuntime, container_id: &str) -> Result<()> {
    let status = Command::new(runtime.command())
        .args(["rm", "-f", container_id])
        .stdout(Stdio::null())
        .status()
        .with_context(|| format!("failed to remove temp container `{container_id}`"))?;

    if !status.success() {
        return Err(anyhow!(
            "{} rm failed for temp container `{container_id}`",
            runtime.name()
        ));
    }

    Ok(())
}

fn exec_in_container(runtime: &ContainerRuntime, container_id: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(runtime.command())
        .arg("exec")
        .arg("-u")
        .arg("0")
        .arg(container_id)
        .args(args)
        .stdout(Stdio::null())
        .status()
        .with_context(|| {
            format!(
                "failed to execute {} exec for `{container_id}`",
                runtime.name()
            )
        })?;

    if !status.success() {
        return Err(anyhow!(
            "{} exec failed for `{container_id}` with status {status}",
            runtime.name()
        ));
    }

    Ok(())
}

fn exec_in_container_capture(
    runtime: &ContainerRuntime,
    container_id: &str,
    args: &[&str],
) -> Result<String> {
    let output = Command::new(runtime.command())
        .arg("exec")
        .arg("-u")
        .arg("0")
        .arg(container_id)
        .args(args)
        .output()
        .with_context(|| {
            format!(
                "failed to execute {} exec for `{container_id}`",
                runtime.name()
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "{} exec capture failed for `{container_id}`: {stderr}",
            runtime.name()
        ));
    }

    String::from_utf8(output.stdout)
        .with_context(|| format!("invalid UTF-8 from {} exec output", runtime.name()))
}

fn write_file_in_container(
    runtime: &ContainerRuntime,
    container_id: &str,
    path: &str,
    content: &str,
) -> Result<()> {
    let script = format!("cat > '{path}'");
    let mut child = Command::new(runtime.command())
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
        .with_context(|| {
            format!(
                "failed to execute {} exec for `{container_id}`",
                runtime.name()
            )
        })?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(content.as_bytes()).with_context(|| {
            format!(
                "failed writing cert content to {} exec stdin",
                runtime.name()
            )
        })?;
    }

    let output = child.wait_with_output().with_context(|| {
        format!(
            "failed waiting {} exec for `{container_id}`",
            runtime.name()
        )
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "{} exec write failed for `{container_id}`: {stderr}",
            runtime.name()
        ));
    }

    Ok(())
}
