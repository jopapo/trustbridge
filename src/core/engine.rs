use crate::core::certificate::Certificate;
use crate::core::plan::SyncPlan;
use std::collections::HashSet;

pub struct SyncEngine;

impl SyncEngine {
    pub fn build_plan_from_data(
        source_certs: Vec<Certificate>,
        target_fingerprints: Vec<String>,
        managed_fingerprints: Vec<String>,
    ) -> SyncPlan {
        let source_set: HashSet<_> = source_certs
            .iter()
            .map(|certificate| certificate.fingerprint_sha256.clone())
            .collect();

        let target_set: HashSet<_> = target_fingerprints.into_iter().collect();
        let managed_set: HashSet<_> = managed_fingerprints.into_iter().collect();

        let to_add: Vec<Certificate> = source_certs
            .iter()
            .filter(|certificate| !target_set.contains(&certificate.fingerprint_sha256))
            .cloned()
            .collect();

        let to_remove: Vec<String> = target_set
            .iter()
            .filter(|fingerprint| managed_set.contains(*fingerprint))
            .filter(|fingerprint| !source_set.contains(*fingerprint))
            .cloned()
            .collect();

        SyncPlan {
            source_total: source_set.len(),
            target_total: target_set.len(),
            to_add,
            to_remove,
        }
    }
}
