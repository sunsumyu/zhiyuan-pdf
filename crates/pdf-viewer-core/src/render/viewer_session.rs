use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostViewerSession {
    pub path: Option<String>,
    pub current_page: u16,
    pub page_count: u16,
    pub current_zoom: f32,
    pub page_width: f32,
    pub page_height: f32,
    pub document_revision: u64,
}

impl Default for HostViewerSession {
    fn default() -> Self {
        Self {
            path: None,
            current_page: 0,
            page_count: 0,
            current_zoom: 1.0,
            page_width: 595.0,
            page_height: 842.0,
            document_revision: 0,
        }
    }
}
