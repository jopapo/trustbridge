mod cli;
mod commands;
mod core;
mod providers;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use std::io::{self, IsTerminal, Write};

fn main() -> Result<()> {
    init_tracing();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Scan(args)) => commands::scan::run(args),
        Some(Commands::Plan(args)) => commands::plan::run(args),
        Some(Commands::Apply(args)) => commands::apply::run(args),
        Some(Commands::Verify(args)) => commands::verify::run(args),
        None => {
            print_quick_start();
            maybe_wait_for_enter();
            Ok(())
        }
    }
}

fn print_quick_start() {
    println!("TrustBridge opened without a command.\n");
    println!("Run from terminal/PowerShell with one of these:");
    println!("  tbridge scan");
    println!("  tbridge plan");
    println!("  tbridge apply --dry-run");
    println!("  tbridge apply");
    println!("\nHelp:");
    println!("  tbridge --help");
    println!("  tbridge apply --help");
}

fn maybe_wait_for_enter() {
    if !cfg!(target_os = "windows") {
        return;
    }

    if std::env::var_os("TBRIDGE_NO_PAUSE").is_some() {
        return;
    }

    if io::stdin().is_terminal() && io::stdout().is_terminal() {
        print!("\nPress Enter to exit...");
        let _ = io::stdout().flush();
        let mut buffer = String::new();
        let _ = io::stdin().read_line(&mut buffer);
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
