pub mod apply;
pub mod plan;
pub mod scan;
pub mod verify;

use crate::cli::{SourceKind, TargetKind};
use crate::providers::source::{macos_keychain::MacosKeychainSource, SourceProvider};
use crate::providers::target::{rancher_desktop::RancherDesktopTarget, TargetProvider};

pub fn resolve_source(source: SourceKind) -> Box<dyn SourceProvider> {
    match source {
        SourceKind::MacosKeychain => Box::new(MacosKeychainSource),
    }
}

pub fn resolve_target(target: TargetKind) -> Box<dyn TargetProvider> {
    match target {
        TargetKind::RancherDesktop => Box::new(RancherDesktopTarget),
    }
}
