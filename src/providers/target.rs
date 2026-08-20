use crate::core::plan::SyncPlan;
use anyhow::Result;

pub mod colima;
pub mod docker_desktop;
pub mod rancher_desktop;
pub mod vm_backend;
pub mod wsl;

pub trait TargetProvider {
    fn name(&self) -> &'static str;
    fn current_fingerprints(&self) -> Result<Vec<String>>;
    fn apply_plan(&self, plan: &SyncPlan, dry_run: bool) -> Result<()>;
    fn verify(&self, host: Option<&str>) -> Result<()>;
}
