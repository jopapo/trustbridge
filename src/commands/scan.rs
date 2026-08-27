use crate::cli::ScanArgs;
use crate::commands::filtering::{apply_default_filter, apply_profile_overrides, FilterOptions};
use crate::commands::resolve_source;
use anyhow::Result;

pub fn run(args: ScanArgs) -> Result<()> {
    let source = resolve_source(args.source);
    let certs = source.scan()?;

    let (certs, stats) = if args.all {
        let total = certs.len();
        (
            certs,
            crate::commands::filtering::FilterStats {
                kept: total,
                dropped: 0,
            },
        )
    } else {
        let options = apply_profile_overrides(
            FilterOptions {
                include_public_roots: args.include_public_roots,
                only_keywords: args.only_keywords.clone(),
                exclude_keywords: args.exclude_keywords.clone(),
            },
            args.profile,
        );
        apply_default_filter(certs, &options)
    };

    if args.json {
        println!("{}", serde_json::to_string_pretty(&certs)?);
        return Ok(());
    }

    println!("source: {}", source.name());
    if !args.all {
        println!(
            "filter: self-signed custom CAs (profile: {:?})",
            args.profile
        );
        println!(
            "filter result: kept {} / dropped {}",
            stats.kept, stats.dropped
        );
        if stats.kept == 0 && stats.dropped > 0 {
            println!(
                "hint: no certs matched default filter; try --all or --only-keywords netskope,inbev"
            );
        }
    }
    println!("certificates found: {}", certs.len());

    for certificate in certs.iter().take(5) {
        println!(
            "- {} {}",
            &certificate.fingerprint_sha256[..12],
            certificate.subject
        );
    }

    if certs.len() > 5 {
        println!("... and {} more", certs.len() - 5);
    }

    Ok(())
}
