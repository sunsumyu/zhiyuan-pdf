pub use crate::state_manager::{
    accept_all_review_changes, accept_review_change, collect_review_changes,
    current_patch_revision, reject_all_review_changes, reject_review_change,
    ReviewBulkChangeResult, ReviewChangeEntry,
};

// Re-export pure DTO types from core.
pub use pdf_viewer_core::persistence::review_types::*;

pub fn get_review_feed() -> ReviewFeedResult {
    let changes = collect_review_changes();
    ReviewFeedResult {
        revision: current_patch_revision(),
        pending_count: changes.len(),
        changes,
    }
}

