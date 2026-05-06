use serde::{Deserialize, Serialize};

pub use crate::state_manager::{
    accept_all_review_changes, accept_review_change, collect_review_changes,
    current_patch_revision, reject_all_review_changes, reject_review_change,
    ReviewBulkChangeResult, ReviewChangeEntry,
};

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

pub fn get_review_feed() -> ReviewFeedResult {
    let changes = collect_review_changes();
    ReviewFeedResult {
        revision: current_patch_revision(),
        pending_count: changes.len(),
        changes,
    }
}

