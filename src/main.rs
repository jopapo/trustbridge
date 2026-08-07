mod cli;
mod commands;
mod core;
mod providers;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

fn main() -> Result<()> {
    init_tracing();

    let cli = Cli::parse();

    match cli.command {
        Commands::Scan(args) => commands::scan::run(args),
        Commands::Plan(args) => commands::plan::run(args),
        Commands::Apply(args) => commands::apply::run(args),
        Commands::Verify(args) => commands::verify::run(args),
    }
}

fn init_tracing() {
    use tracing_subscriber::{fmt, EnvFilter};

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .compact()
        .init();
}
