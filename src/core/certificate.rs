use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Certificate {
    pub id: String,
    pub subject: String,
    pub fingerprint_sha256: String,
    pub pem: String,
    pub not_after: Option<String>,
}
