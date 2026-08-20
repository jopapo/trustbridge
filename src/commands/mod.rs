pub mod apply;
pub mod container_runtime;
pub mod filtering;
pub mod images;
pub mod plan;
pub mod scan;
pub mod verify;
pub mod workloads;

use crate::cli::{SourceKind, TargetKind};
use crate::providers::source::{
    macos_keychain::MacosKeychainSource, windows_certstore::WindowsCertStoreSource, SourceProvider,
};
use crate::providers::target::{
    colima::ColimaTarget, docker_desktop::DockerDesktopTarget,
    rancher_desktop::RancherDesktopTarget, vm_backend::is_wsl, wsl::WslTarget, TargetProvider,
};

pub fn resolve_source(source: SourceKind) -> Box<dyn SourceProvider> {
    match source {
        SourceKind::MacosKeychain => Box::new(MacosKeychainSource),
        SourceKind::WindowsCertStore => Box::new(WindowsCertStoreSource),
    }
}

pub fn resolve_target(target: TargetKind) -> Box<dyn TargetProvider> {
    match target {
        TargetKind::Auto if cfg!(target_os = "windows") || is_wsl() => {
            Box::new(DockerDesktopTarget)
        }
        TargetKind::Auto => Box::new(RancherDesktopTarget),
        TargetKind::RancherDesktop => Box::new(RancherDesktopTarget),
        TargetKind::Colima => Box::new(ColimaTarget),
        TargetKind::DockerDesktop => Box::new(DockerDesktopTarget),
        TargetKind::Wsl => Box::new(WslTarget),
    }
}
