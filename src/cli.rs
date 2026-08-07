use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(name = "tbridge", version, about = "TrustBridge: host truststore sync")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Scan(ScanArgs),
    Plan(PlanArgs),
    Apply(ApplyArgs),
    Verify(VerifyArgs),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum SourceKind {
    #[value(name = "macos-keychain")]
    MacosKeychain,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum TargetKind {
    #[value(name = "rancher-desktop")]
    RancherDesktop,
}

#[derive(Args, Debug)]
pub struct ScanArgs {
    #[arg(long, value_enum, default_value = "macos-keychain")]
    pub source: SourceKind,
    #[arg(long, default_value_t = false)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct PlanArgs {
    #[arg(long, value_enum, default_value = "macos-keychain")]
    pub source: SourceKind,
    #[arg(long, value_enum, default_value = "rancher-desktop")]
    pub target: TargetKind,
}

#[derive(Args, Debug)]
pub struct ApplyArgs {
    #[arg(long, value_enum, default_value = "macos-keychain")]
    pub source: SourceKind,
    #[arg(long, value_enum, default_value = "rancher-desktop")]
    pub target: TargetKind,
    #[arg(long, default_value_t = true)]
    pub dry_run: bool,
}

#[derive(Args, Debug)]
pub struct VerifyArgs {
    #[arg(long, value_enum, default_value = "rancher-desktop")]
    pub target: TargetKind,
    #[arg(long)]
    pub host: Option<String>,
}
