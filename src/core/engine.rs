use crate::core::certificate::Certificate;
use crate::core::plan::SyncPlan;
use crate::providers::source::SourceProvider;
use crate::providers::target::TargetProvider;
use anyhow::Result;
use std::collections::HashSet;

pub struct SyncEngine;

impl SyncEngine {
    pub fn build_plan(
        source: &dyn SourceProvider,
        target: &dyn TargetProvider,
    ) -> Result<SyncPlan> {
        let source_certs = source.scan()?;
        let target_fingerprints = target.current_fingerprints()?;

        let source_set: HashSet<_> = source_certs
            .iter()
            .map(|certificate| certificate.fingerprint_sha256.clone())
            .collect();

        let target_set: HashSet<_> = target_fingerprints.into_iter().collect();

        let to_add: Vec<Certificate> = source_certs
            .iter()
            .filter(|certificate| !target_set.contains(&certificate.fingerprint_sha256))
            .cloned()
            .collect();

        let to_remove: Vec<String> = target_set
            .iter()
            .filter(|fingerprint| !source_set.contains(*fingerprint))
            .cloned()
            .collect();

        Ok(SyncPlan {
            source_total: source_set.len(),
            target_total: target_set.len(),
            to_add,
            to_remove,
        })
    }
}
