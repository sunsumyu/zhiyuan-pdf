use serde::{Deserialize, Serialize};

use crate::persistence::patch_state::ReviewChangeEntry;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReviewFeedResult {
    pub revision: u64,
    pub pending_count: usize,
    pub changes: Vec<ReviewChangeEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RejectReviewChangeResult {
    pub changed: bool,
    pub revision: u64,
    pub patch_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AcceptReviewChangeResult {
    pub changed: bool,
    pub revision: u64,
    pub patch_key: String,
}
