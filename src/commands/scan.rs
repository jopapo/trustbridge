use crate::cli::ScanArgs;
use crate::commands::resolve_source;
use anyhow::Result;

pub fn run(args: ScanArgs) -> Result<()> {
    let source = resolve_source(args.source);
    let certs = source.scan()?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&certs)?);
        return Ok(());
    }

    println!("source: {}", source.name());
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
