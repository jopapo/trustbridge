use clap::ArgAction;
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
    #[value(name = "auto")]
    Auto,
    #[value(name = "rancher-desktop")]
    RancherDesktop,
    #[value(name = "colima")]
    Colima,
}

#[derive(Args, Debug)]
pub struct ScanArgs {
    #[arg(long, value_enum, default_value = "macos-keychain")]
    pub source: SourceKind,
    #[arg(long, default_value_t = false)]
    pub json: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Include non-self-signed certificates"
    )]
    pub all: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Include likely OS/public root CAs"
    )]
    pub include_public_roots: bool,
    #[arg(
        long,
        value_delimiter = ',',
        help = "Only include certs matching any subject keyword"
    )]
    pub only_keywords: Vec<String>,
    #[arg(
        long,
        value_delimiter = ',',
        help = "Exclude certs matching any subject keyword"
    )]
    pub exclude_keywords: Vec<String>,
}

#[derive(Args, Debug)]
pub struct PlanArgs {
    #[arg(long, value_enum, default_value = "macos-keychain")]
    pub source: SourceKind,
    #[arg(long, value_enum, default_value = "rancher-desktop")]
    pub target: TargetKind,
    #[arg(
        long,
        default_value_t = false,
        help = "Include likely OS/public root CAs"
    )]
    pub include_public_roots: bool,
    #[arg(
        long,
        value_delimiter = ',',
        help = "Only include certs matching any subject keyword"
    )]
    pub only_keywords: Vec<String>,
    #[arg(
        long,
        value_delimiter = ',',
        help = "Exclude certs matching any subject keyword"
    )]
    pub exclude_keywords: Vec<String>,
}

#[derive(Args, Debug)]
pub struct ApplyArgs {
    #[arg(long, value_enum, default_value = "macos-keychain")]
    pub source: SourceKind,
    #[arg(long, value_enum, default_value = "auto")]
    pub target: TargetKind,
    #[arg(long, default_value_t = false, action = ArgAction::SetTrue)]
    pub dry_run: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Include likely OS/public root CAs"
    )]
    pub include_public_roots: bool,
    #[arg(
        long,
        value_delimiter = ',',
        help = "Only include certs matching any subject keyword"
    )]
    pub only_keywords: Vec<String>,
    #[arg(
        long,
        value_delimiter = ',',
        help = "Exclude certs matching any subject keyword"
    )]
    pub exclude_keywords: Vec<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Prompt confirmation per container"
    )]
    pub interactive: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Include orchestrator/system containers and images"
    )]
    pub include_orchestrator: bool,
    #[arg(
        long,
        value_delimiter = ',',
        help = "Container names (default: all running)"
    )]
    pub containers: Vec<String>,
    #[arg(
        long,
        value_delimiter = ',',
        default_value = "runtime,containers,images",
        help = "Apply scopes: runtime,containers,images"
    )]
    pub scope: Vec<String>,
    #[arg(
        long,
        default_value = "user",
        help = "Image selection mode: user|all|none"
    )]
    pub images_mode: String,
    #[arg(long, default_value_t = 30, help = "Max number of images to patch")]
    pub images_limit: usize,
    #[arg(long, default_value_t = false, action = ArgAction::SetTrue, help = "Continuously keep targets in sync")]
    pub watch: bool,
    #[arg(long, default_value_t = 30, help = "Watch loop interval in seconds")]
    pub interval_secs: u64,
}

#[derive(Args, Debug)]
pub struct VerifyArgs {
    #[arg(long, value_enum, default_value = "rancher-desktop")]
    pub target: TargetKind,
    #[arg(long)]
    pub host: Option<String>,
}
