use super::SourceProvider;
use crate::core::certificate::Certificate;
use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::env;
use std::process::Command;

pub struct WindowsCertStoreSource;

impl SourceProvider for WindowsCertStoreSource {
    fn name(&self) -> &'static str {
        "windows-certstore"
    }

    fn scan(&self) -> Result<Vec<Certificate>> {
        if !(cfg!(target_os = "windows") || is_wsl()) {
            anyhow::bail!("windows-certstore source requires Windows or WSL with Windows interop");
        }

        let output = run_powershell(CERT_EXPORT_SCRIPT)?;
        let exported = parse_exported_certs(&output)?;

        Ok(exported
            .into_iter()
            .enumerate()
            .map(|(index, exported)| build_certificate(index, exported))
            .collect())
    }
}

/// Converts a single PowerShell-exported certificate into the domain
/// `Certificate` model, computing its fingerprint and a human-readable
/// subject fallback when the store did not report one.
fn build_certificate(index: usize, exported: ExportedCertificate) -> Certificate {
    let mut hasher = Sha256::new();
    hasher.update(exported.pem.as_bytes());

    Certificate {
        id: format!("windows-{index}"),
        subject: exported.subject.unwrap_or_else(|| {
            exported
                .thumbprint
                .map(|thumbprint| format!("Windows certificate {thumbprint}"))
                .unwrap_or_else(|| format!("Windows certificate #{index}"))
        }),
        fingerprint_sha256: format!("{:x}", hasher.finalize()),
        pem: exported.pem,
        not_after: exported.not_after,
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ExportedCertificate {
    subject: Option<String>,
    thumbprint: Option<String>,
    pem: String,
    not_after: Option<String>,
}

const CERT_EXPORT_SCRIPT: &str = r#"
$OutputEncoding = [System.Text.UTF8Encoding]::new($false)
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
$stores = @('Cert:\LocalMachine\Root', 'Cert:\CurrentUser\Root')
$certs = foreach ($store in $stores) {
  Get-ChildItem -Path $store -ErrorAction SilentlyContinue | ForEach-Object {
    if ($_.RawData) {
      [pscustomobject]@{
        Subject = $_.Subject
        Thumbprint = $_.Thumbprint
        Pem = "-----BEGIN CERTIFICATE-----`n$([Convert]::ToBase64String($_.RawData, [Base64FormattingOptions]::InsertLineBreaks))`n-----END CERTIFICATE-----`n"
        NotAfter = $_.NotAfter.ToUniversalTime().ToString('O')
      }
    }
  }
}
@($certs) | ConvertTo-Json -Depth 3
"#;

fn run_powershell(script: &str) -> Result<String> {
    let candidates = if cfg!(target_os = "windows") {
        ["powershell.exe", "pwsh.exe", "powershell", "pwsh"]
    } else {
        ["powershell.exe", "pwsh.exe", "pwsh", "powershell"]
    };

    let mut last_error = None;
    for candidate in candidates {
        let output = Command::new(candidate)
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .output();

        let output = match output {
            Ok(output) => output,
            Err(error) => {
                last_error = Some(error.to_string());
                continue;
            }
        };

        if output.status.success() {
            return decode_command_output(&output.stdout)
                .context("invalid text from Windows certificate export");
        }

        last_error = Some(String::from_utf8_lossy(&output.stderr).to_string());
    }

    Err(anyhow!(
        "failed to export Windows certificates with PowerShell: {}",
        last_error.unwrap_or_else(|| "no PowerShell executable found".to_string())
    ))
}

fn decode_command_output(bytes: &[u8]) -> Result<String> {
    if bytes.starts_with(&[0xFF, 0xFE]) || looks_like_utf16le(bytes) {
        let start = if bytes.starts_with(&[0xFF, 0xFE]) {
            2
        } else {
            0
        };
        let utf16: Vec<u16> = bytes[start..]
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();
        return Ok(String::from_utf16_lossy(&utf16));
    }

    if let Ok(text) = String::from_utf8(bytes.to_vec()) {
        return Ok(text);
    }

    Ok(String::from_utf8_lossy(bytes).into_owned())
}

fn looks_like_utf16le(bytes: &[u8]) -> bool {
    if bytes.len() < 4 || bytes.len() % 2 != 0 {
        return false;
    }

    let sample_pairs = bytes.chunks_exact(2).take(64);
    let mut total = 0usize;
    let mut zero_high_bytes = 0usize;

    for pair in sample_pairs {
        total += 1;
        if pair[1] == 0 {
            zero_high_bytes += 1;
        }
    }

    total >= 4 && zero_high_bytes * 2 >= total
}

fn parse_exported_certs(output: &str) -> Result<Vec<ExportedCertificate>> {
    let trimmed = output.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    serde_json::from_str::<Vec<ExportedCertificate>>(trimmed)
        .or_else(|_| serde_json::from_str::<ExportedCertificate>(trimmed).map(|cert| vec![cert]))
        .context("failed to parse Windows certificate export")
}

fn is_wsl() -> bool {
    if env::var_os("WSL_DISTRO_NAME").is_some() || env::var_os("WSL_INTEROP").is_some() {
        return true;
    }

    std::fs::read_to_string("/proc/version")
        .map(|version| version.to_ascii_lowercase().contains("microsoft"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::{
        build_certificate, decode_command_output, parse_exported_certs, ExportedCertificate,
    };

    const SAMPLE_PEM: &str = "-----BEGIN CERTIFICATE-----\nABC\n-----END CERTIFICATE-----\n";

    #[test]
    fn parses_single_cert_export() {
        let json = r#"{"Subject":"CN=Corp Root","Thumbprint":"ABCD","Pem":"-----BEGIN CERTIFICATE-----\nABC\n-----END CERTIFICATE-----\n","NotAfter":"2026-01-01T00:00:00Z"}"#;
        let certs = parse_exported_certs(json).unwrap();
        assert_eq!(certs.len(), 1);
        assert_eq!(certs[0].subject.as_deref(), Some("CN=Corp Root"));
    }

    #[test]
    fn parses_multiple_cert_export() {
        let json = r#"[
            {"Subject":"CN=Root One","Thumbprint":"AAAA","Pem":"pem-one","NotAfter":"2026-01-01T00:00:00Z"},
            {"Subject":"CN=Root Two","Thumbprint":"BBBB","Pem":"pem-two","NotAfter":null}
        ]"#;
        let certs = parse_exported_certs(json).unwrap();
        assert_eq!(certs.len(), 2);
        assert_eq!(certs[0].subject.as_deref(), Some("CN=Root One"));
        assert_eq!(certs[1].subject.as_deref(), Some("CN=Root Two"));
        assert_eq!(certs[1].not_after, None);
    }

    #[test]
    fn parses_empty_export_as_no_certs() {
        assert!(parse_exported_certs("").unwrap().is_empty());
        assert!(parse_exported_certs("   \n  ").unwrap().is_empty());
    }

    #[test]
    fn rejects_invalid_json() {
        assert!(parse_exported_certs("not json").is_err());
    }

    #[test]
    fn decode_command_output_handles_plain_utf8() {
        let decoded = decode_command_output("hello".as_bytes()).unwrap();
        assert_eq!(decoded, "hello");
    }

    #[test]
    fn decode_command_output_handles_utf16le_with_bom() {
        // "hi" encoded as UTF-16LE with a leading BOM, as emitted by some
        // Windows PowerShell builds.
        let bytes: Vec<u8> = vec![0xFF, 0xFE, b'h', 0x00, b'i', 0x00];
        let decoded = decode_command_output(&bytes).unwrap();
        assert_eq!(decoded, "hi");
    }

    #[test]
    fn decode_command_output_falls_back_for_non_utf8_bytes() {
        let bytes = [b'{', b'"', b'a', b'"', b':', b'"', 0xE9, b'"', b'}'];
        let decoded = decode_command_output(&bytes).unwrap();
        assert!(decoded.starts_with("{\"a\":\""));
    }

    #[test]
    fn build_certificate_uses_subject_when_present() {
        let exported = ExportedCertificate {
            subject: Some("CN=Corp Root".to_string()),
            thumbprint: Some("ABCD".to_string()),
            pem: SAMPLE_PEM.to_string(),
            not_after: Some("2026-01-01T00:00:00Z".to_string()),
        };

        let certificate = build_certificate(0, exported);
        assert_eq!(certificate.subject, "CN=Corp Root");
        assert_eq!(certificate.id, "windows-0");
        assert_eq!(
            certificate.not_after.as_deref(),
            Some("2026-01-01T00:00:00Z")
        );
    }

    #[test]
    fn build_certificate_falls_back_to_thumbprint() {
        let exported = ExportedCertificate {
            subject: None,
            thumbprint: Some("ABCD".to_string()),
            pem: SAMPLE_PEM.to_string(),
            not_after: None,
        };

        let certificate = build_certificate(1, exported);
        assert_eq!(certificate.subject, "Windows certificate ABCD");
    }

    #[test]
    fn build_certificate_falls_back_to_index_placeholder() {
        let exported = ExportedCertificate {
            subject: None,
            thumbprint: None,
            pem: SAMPLE_PEM.to_string(),
            not_after: None,
        };

        let certificate = build_certificate(2, exported);
        assert_eq!(certificate.subject, "Windows certificate #2");
    }

    #[test]
    fn build_certificate_computes_sha256_fingerprint_of_pem() {
        use sha2::{Digest, Sha256};

        let exported = ExportedCertificate {
            subject: Some("CN=Corp Root".to_string()),
            thumbprint: None,
            pem: SAMPLE_PEM.to_string(),
            not_after: None,
        };

        let mut hasher = Sha256::new();
        hasher.update(SAMPLE_PEM.as_bytes());
        let expected = format!("{:x}", hasher.finalize());

        let certificate = build_certificate(0, exported);
        assert_eq!(certificate.fingerprint_sha256, expected);
        assert_eq!(certificate.pem, SAMPLE_PEM);
    }
}
