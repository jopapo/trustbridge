use crate::cli::FilterProfile;
use crate::core::certificate::Certificate;
use std::io::Write;
use std::process::{Command, Stdio};

pub struct FilterOptions {
    pub include_public_roots: bool,
    pub only_keywords: Vec<String>,
    pub exclude_keywords: Vec<String>,
}

pub struct FilterStats {
    pub kept: usize,
    pub dropped: usize,
}

const CORP_KEYWORDS: &[&str] = &["netskope", "inbev", "zscaler", "corp", "internal root"];

pub fn apply_profile_overrides(mut opts: FilterOptions, profile: FilterProfile) -> FilterOptions {
    match profile {
        FilterProfile::Default => opts,
        FilterProfile::Corp => {
            if opts.only_keywords.is_empty() {
                opts.only_keywords = CORP_KEYWORDS.iter().map(|v| v.to_string()).collect();
            }
            opts
        }
    }
}

pub fn apply_default_filter(
    certs: Vec<Certificate>,
    opts: &FilterOptions,
) -> (Vec<Certificate>, FilterStats) {
    let mut kept = Vec::new();
    let mut dropped = 0usize;

    let include_keywords: Vec<String> = opts
        .only_keywords
        .iter()
        .map(|value| value.to_ascii_lowercase())
        .collect();
    let exclude_keywords: Vec<String> = opts
        .exclude_keywords
        .iter()
        .map(|value| value.to_ascii_lowercase())
        .collect();

    for cert in certs {
        let subject_lower = cert.subject.to_ascii_lowercase();

        let is_self_signed = is_self_signed_from_pem(&cert.pem)
            .unwrap_or_else(|| is_probably_root_or_corporate_ca(&subject_lower));
        if !is_self_signed {
            dropped += 1;
            continue;
        }

        if !opts.include_public_roots && is_likely_public_or_os_ca(&subject_lower) {
            dropped += 1;
            continue;
        }

        if !include_keywords.is_empty()
            && !include_keywords
                .iter()
                .any(|keyword| subject_lower.contains(keyword))
        {
            dropped += 1;
            continue;
        }

        if exclude_keywords
            .iter()
            .any(|keyword| subject_lower.contains(keyword))
        {
            dropped += 1;
            continue;
        }

        kept.push(cert);
    }

    let kept_len = kept.len();
    (
        kept,
        FilterStats {
            kept: kept_len,
            dropped,
        },
    )
}

pub fn is_likely_public_or_os_ca(subject_lowercase: &str) -> bool {
    const ROOT_MARKERS: &[&str] = &[
        "apple",
        "digicert",
        "globalsign",
        "lets encrypt",
        "isrg",
        "microsoft",
        "google trust",
        "amazon root",
        "entrust",
        "sectigo",
        "godaddy",
        "verisign",
        "symantec",
        "geotrust",
        "usertrust",
        "comodo",
        "baltimore",
        "ssl.com",
    ];

    ROOT_MARKERS
        .iter()
        .any(|marker| subject_lowercase.contains(marker))
}

pub fn is_self_signed_from_pem(pem: &str) -> Option<bool> {
    let mut child = Command::new("openssl")
        .args([
            "x509", "-noout", "-subject", "-issuer", "-nameopt", "RFC2253",
        ])
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

    let output = String::from_utf8(output.stdout).ok()?;
    parse_subject_issuer(&output).map(|(subject, issuer)| subject == issuer)
}

fn parse_subject_issuer(output: &str) -> Option<(String, String)> {
    let mut subject: Option<String> = None;
    let mut issuer: Option<String> = None;

    for line in output.lines() {
        if let Some(value) = line.strip_prefix("subject=") {
            subject = Some(value.trim().to_string());
            continue;
        }

        if let Some(value) = line.strip_prefix("issuer=") {
            issuer = Some(value.trim().to_string());
        }
    }

    Some((subject?, issuer?))
}

fn is_probably_root_or_corporate_ca(subject_lowercase: &str) -> bool {
    const ROOT_HINTS: &[&str] = &[
        "root ca",
        "internal root",
        "certificate authority",
        "corporate",
        "corp",
        "netskope",
        "inbev",
        "zscaler",
        "blue coat",
        "palo alto",
    ];

    ROOT_HINTS
        .iter()
        .any(|hint| subject_lowercase.contains(hint))
}

#[cfg(test)]
mod tests {
    use super::{
        is_likely_public_or_os_ca, is_probably_root_or_corporate_ca, parse_subject_issuer,
    };

    #[test]
    fn parses_subject_and_issuer() {
        let input = "subject=CN=Root CA,O=Corp,C=US\nissuer=CN=Root CA,O=Corp,C=US\n";
        let parsed = parse_subject_issuer(input);
        assert_eq!(
            parsed,
            Some((
                "CN=Root CA,O=Corp,C=US".to_string(),
                "CN=Root CA,O=Corp,C=US".to_string()
            ))
        );
    }

    #[test]
    fn returns_none_when_subject_or_issuer_missing() {
        let input = "subject=CN=Root CA,O=Corp,C=US\n";
        assert_eq!(parse_subject_issuer(input), None);
    }

    #[test]
    fn identifies_public_roots() {
        assert!(is_likely_public_or_os_ca("cn=apple root ca"));
        assert!(is_likely_public_or_os_ca("cn=digicert global root ca"));
    }

    #[test]
    fn keeps_custom_corporate_cas() {
        assert!(!is_likely_public_or_os_ca(
            "cn=netskope root ca,o=netskope inc"
        ));
        assert!(!is_likely_public_or_os_ca("cn=ab inbev internal root ca"));
    }

    #[test]
    fn infers_corporate_roots_by_subject() {
        assert!(is_probably_root_or_corporate_ca(
            "cn=one internal root ca, o=anheuser-busch inbev"
        ));
        assert!(is_probably_root_or_corporate_ca(
            "cn=caadmin.netskope.com, ou=cert management"
        ));
    }
}
