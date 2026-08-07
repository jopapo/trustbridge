use crate::core::certificate::Certificate;

#[derive(Debug, Clone)]
pub struct SyncPlan {
    pub source_total: usize,
    pub target_total: usize,
    pub to_add: Vec<Certificate>,
    pub to_remove: Vec<String>,
}

impl SyncPlan {
    pub fn is_noop(&self) -> bool {
        self.to_add.is_empty() && self.to_remove.is_empty()
    }
}
