use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct StateSnapshot {
    pub last_apply_at: Option<String>,
    pub applied_fingerprints: Vec<String>,
    pub last_bundle_hash: Option<String>,
    pub container_bundle_hashes: HashMap<String, String>,
    pub image_bundle_hashes: HashMap<String, String>,
}

impl StateSnapshot {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read state file: {}", path.display()))?;

        serde_json::from_str(&content)
            .with_context(|| format!("failed to parse state file: {}", path.display()))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create state dir: {}", parent.display()))?;
        }

        let content = serde_json::to_string_pretty(self).context("failed to serialize state")?;
        fs::write(path, content)
            .with_context(|| format!("failed to write state file: {}", path.display()))?;

        Ok(())
    }
}
