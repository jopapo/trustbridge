use crate::core::certificate::Certificate;
use anyhow::Result;

pub mod macos_keychain;

pub trait SourceProvider {
    fn name(&self) -> &'static str;
    fn scan(&self) -> Result<Vec<Certificate>>;
}
