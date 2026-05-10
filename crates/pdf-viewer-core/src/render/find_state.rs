use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum HostFindScope {
    #[default]
    Page,
    Document,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HostFindSession {
    pub query: String,
    pub scope: HostFindScope,
    pub active_index: usize,
    pub total_matches: usize,
    pub match_pages: Vec<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HostFindNavigationResult {
    pub has_matches: bool,
    pub active_index: usize,
    pub active_page: Option<u16>,
    pub wrapped: bool,
}
