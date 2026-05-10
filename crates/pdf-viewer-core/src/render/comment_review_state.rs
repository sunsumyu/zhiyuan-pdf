use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum HostCommentReviewScope {
    #[default]
    Page,
    Document,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HostCommentReviewSession {
    pub panel_open: bool,
    pub scope: HostCommentReviewScope,
    pub query: String,
    pub selected_comment_id: Option<String>,
}
