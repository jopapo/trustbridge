use super::SourceProvider;
use crate::core::certificate::Certificate;
use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::process::Command;

pub struct MacosKeychainSource;

impl SourceProvider for MacosKeychainSource {
    fn name(&self) -> &'static str {
        "macos-keychain"
    }

    fn scan(&self) -> Result<Vec<Certificate>> {
        let output = Command::new("security")
            .args([
                "find-certificate",
                "-a",
                "-p",
                "/System/Library/Keychains/SystemRootCertificates.keychain",
            ])
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

                Certificate {
                    id: format!("macos-{index}"),
                    subject: format!("macOS Keychain certificate #{index}"),
                    fingerprint_sha256: fingerprint,
                    pem: block,
                    not_after: None,
                }
            })
            .collect();

        Ok(certs)
    }
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

#[cfg(test)]
mod tests {
    use super::split_pem_blocks;

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
}
