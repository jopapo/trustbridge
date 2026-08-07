use super::SourceProvider;
use crate::core::certificate::Certificate;
use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::env;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

pub struct MacosKeychainSource;

impl SourceProvider for MacosKeychainSource {
    fn name(&self) -> &'static str {
        "macos-keychain"
    }

    fn scan(&self) -> Result<Vec<Certificate>> {
        let keychains = scan_keychains();
        if keychains.is_empty() {
            anyhow::bail!("no keychains found to scan");
        }

        let mut args = vec![
            "find-certificate".to_string(),
            "-a".to_string(),
            "-p".to_string(),
        ];
        args.extend(keychains);

        let output = Command::new("security")
            .args(args)
            .output()
            .context("failed to execute macOS security command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("security command failed: {stderr}");
        }

        let pem = String::from_utf8(output.stdout).context("invalid UTF-8 in security output")?;
        let cert_blocks = split_pem_blocks(&pem);

        let certs = cert_blocks
            .into_iter()
            .enumerate()
            .map(|(index, block)| {
                let mut hasher = Sha256::new();
                hasher.update(block.as_bytes());
                let fingerprint = format!("{:x}", hasher.finalize());
                let subject = extract_subject_from_pem(&block)
                    .unwrap_or_else(|| format!("macOS Keychain certificate #{index}"));

                Certificate {
                    id: format!("macos-{index}"),
                    subject,
                    fingerprint_sha256: fingerprint,
                    pem: block,
                    not_after: None,
                }
            })
            .collect();

        Ok(certs)
    }
}

fn scan_keychains() -> Vec<String> {
    let mut keychains = Vec::new();

    let system = "/Library/Keychains/System.keychain";
    if Path::new(system).exists() {
        keychains.push(system.to_string());
    }

    if let Ok(home) = env::var("HOME") {
        let login = format!("{home}/Library/Keychains/login.keychain-db");
        if Path::new(&login).exists() {
            keychains.push(login);
        }
    }

    keychains
}

fn split_pem_blocks(bundle: &str) -> Vec<String> {
    let begin = "-----BEGIN CERTIFICATE-----";
    let end = "-----END CERTIFICATE-----";

    let mut blocks = Vec::new();
    let mut current = Vec::new();
    let mut in_block = false;

    for line in bundle.lines() {
        if line.contains(begin) {
            in_block = true;
            current.clear();
        }

        if in_block {
            current.push(line);
        }

        if line.contains(end) && in_block {
            blocks.push(current.join("\n") + "\n");
            in_block = false;
        }
    }

    blocks
}

fn extract_subject_from_pem(pem: &str) -> Option<String> {
    let mut child = Command::new("openssl")
        .args(["x509", "-noout", "-subject", "-nameopt", "RFC2253"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    if let Some(mut stdin) = child.stdin.take() {
        if stdin.write_all(pem.as_bytes()).is_err() {
            return None;
        }
    }

    let output = child.wait_with_output().ok()?;
    if !output.status.success() {
        return None;
    }

    parse_subject_output(&String::from_utf8(output.stdout).ok()?)
}

fn parse_subject_output(output: &str) -> Option<String> {
    let subject = output.trim();
    let stripped = subject.strip_prefix("subject=").unwrap_or(subject);
    if stripped.is_empty() {
        return None;
    }

    Some(stripped.to_string())
}

#[cfg(test)]
mod tests {
    use super::{parse_subject_output, scan_keychains, split_pem_blocks};
    use std::path::Path;

    #[test]
    fn parses_pem_bundle() {
        let input = "\
-----BEGIN CERTIFICATE-----\n\
ABC\n\
-----END CERTIFICATE-----\n\
-----BEGIN CERTIFICATE-----\n\
DEF\n\
-----END CERTIFICATE-----\n";

        let blocks = split_pem_blocks(input);
        assert_eq!(blocks.len(), 2);
    }

    #[test]
    fn parses_subject_output_with_prefix() {
        let input = "subject=CN=Example Root CA,O=Example Corp,C=US\n";
        let parsed = parse_subject_output(input);
        assert_eq!(
            parsed.as_deref(),
            Some("CN=Example Root CA,O=Example Corp,C=US")
        );
    }

    #[test]
    fn parses_subject_output_without_prefix() {
        let input = "CN=Example Root CA,O=Example Corp,C=US\n";
        let parsed = parse_subject_output(input);
        assert_eq!(
            parsed.as_deref(),
            Some("CN=Example Root CA,O=Example Corp,C=US")
        );
    }

    #[test]
    fn scan_keychains_excludes_system_root_store() {
        let keychains = scan_keychains();
        assert!(!keychains
            .iter()
            .any(|path| path.contains("SystemRootCertificates.keychain")));
    }

    #[test]
    fn scan_keychains_only_returns_existing_paths() {
        let keychains = scan_keychains();
        for keychain in keychains {
            assert!(Path::new(&keychain).exists());
        }
    }
}
