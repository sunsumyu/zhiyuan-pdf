use std::cell::RefCell;

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

thread_local! {
    pub static HOST_VIEWER_SESSION: RefCell<HostViewerSession> =
        RefCell::new(HostViewerSession::default());
}

pub fn reset_viewer_session() {
    HOST_VIEWER_SESSION.with(|session| {
        *session.borrow_mut() = HostViewerSession::default();
    });
}

pub fn set_viewer_document(path: Option<String>, page_count: u16, initial_zoom: f32) {
    HOST_VIEWER_SESSION.with(|session| {
        let mut session = session.borrow_mut();
        session.path = path;
        session.current_page = 0;
        session.page_count = page_count;
        session.current_zoom = sanitize_zoom(initial_zoom);
        session.document_revision = 0;
    });
}

pub fn set_current_page(page_index: u16) {
    HOST_VIEWER_SESSION.with(|session| {
        session.borrow_mut().current_page = page_index;
    });
}

pub fn set_current_zoom(zoom: f32) {
    HOST_VIEWER_SESSION.with(|session| {
        session.borrow_mut().current_zoom = sanitize_zoom(zoom);
    });
}

pub fn set_page_dimensions(page_width: f32, page_height: f32) {
    HOST_VIEWER_SESSION.with(|session| {
        let mut session = session.borrow_mut();
        session.page_width = page_width.max(1.0);
        session.page_height = page_height.max(1.0);
    });
}

pub fn bump_document_revision() -> u64 {
    HOST_VIEWER_SESSION.with(|session| {
        let mut session = session.borrow_mut();
        session.document_revision = session.document_revision.wrapping_add(1).max(1);
        session.document_revision
    })
}

pub fn current_document_revision() -> u64 {
    HOST_VIEWER_SESSION.with(|session| session.borrow().document_revision)
}

fn sanitize_zoom(value: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        1.0
    }
}
