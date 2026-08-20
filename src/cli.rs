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
    #[value(
        name = "macos-keychain",
        help = "macOS System + login Keychains (default on macOS)"
    )]
    MacosKeychain,
    #[value(
        name = "windows-certstore",
        help = "Windows Certificate Store, LocalMachine/CurrentUser Root, via PowerShell (default on Windows/WSL)"
    )]
    WindowsCertStore,
}

/// Picks a sensible default source for the host OS the CLI is running on.
pub fn default_source_kind() -> SourceKind {
    if cfg!(target_os = "windows") || is_wsl() {
        SourceKind::WindowsCertStore
    } else {
        SourceKind::MacosKeychain
    }
}

fn is_wsl() -> bool {
    std::env::var_os("WSL_DISTRO_NAME").is_some() || std::env::var_os("WSL_INTEROP").is_some()
}

impl std::fmt::Display for SourceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            SourceKind::MacosKeychain => "macos-keychain",
            SourceKind::WindowsCertStore => "windows-certstore",
        };
        write!(f, "{name}")
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum TargetKind {
    #[value(
        help = "Auto-detect available runtime targets for the current OS (macOS: rancher-desktop, colima; Windows/WSL: rancher-desktop, docker-desktop, wsl)"
    )]
    Auto,
    #[value(
        name = "rancher-desktop",
        help = "Rancher Desktop (Lima VM on macOS; WSL2 distro on Windows/WSL)"
    )]
    RancherDesktop,
    #[value(name = "colima", help = "Colima (Lima VM, macOS only)")]
    Colima,
    #[value(
        name = "docker-desktop",
        help = "Docker Desktop's WSL2 backend (Windows/WSL only)"
    )]
    DockerDesktop,
    #[value(name = "wsl", help = "A WSL2 distro reached directly or via wsl.exe")]
    Wsl,
}

#[derive(Args, Debug)]
pub struct ScanArgs {
    #[arg(
        long,
        value_enum,
        default_value_t = default_source_kind(),
        help = "Certificate source (default: OS-detected)"
    )]
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
    #[arg(
        long,
        value_enum,
        default_value_t = default_source_kind(),
        help = "Certificate source (default: OS-detected)"
    )]
    pub source: SourceKind,
    #[arg(
        long,
        value_enum,
        default_value = "auto",
        help = "Runtime target (default: auto-detected)"
    )]
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
    #[arg(
        long,
        value_enum,
        default_value_t = default_source_kind(),
        help = "Certificate source (default: OS-detected)"
    )]
    pub source: SourceKind,
    #[arg(
        long,
        value_enum,
        default_value = "auto",
        help = "Runtime target (default: auto-detected; tries all compatible targets, tolerating unavailable ones)"
    )]
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
    #[arg(
        long,
        value_enum,
        default_value = "auto",
        help = "Runtime target (default: auto-detected)"
    )]
    pub target: TargetKind,
    #[arg(long)]
    pub host: Option<String>,
}
