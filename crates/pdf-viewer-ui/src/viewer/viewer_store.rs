use serde::{Deserialize, Serialize};
use std::cell::RefCell;

// Re-export pure data structure from core.
pub use pdf_viewer_core::render::viewer_session::*;

// ─── ViewerSessionState (Batch 2 sec 4) ─────────────────────────
//
// Explicit enum for the Viewer session state, complementing
// EditorSession's SessionState and FindSession's FindSessionState.
// Derived from `path: Option<String>` — no redundant storage needed.
//
// Semantics
//
//   NoDocument    no PDF loaded (path is None)
//   DocumentOpen  a PDF is loaded and active (path is Some)

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ViewerSessionState {
    NoDocument,
    DocumentOpen,
}

impl ViewerSessionState {
    pub fn as_str(&self) -> &'static str {
        match self {
            ViewerSessionState::NoDocument => "NoDocument",
            ViewerSessionState::DocumentOpen => "DocumentOpen",
        }
    }
}

/// Snapshot of the current viewer session state.
pub fn read_viewer_state() -> ViewerSessionState {
    VIEWER_SESSION.with(|session| {
        if session.borrow().path.is_some() {
            ViewerSessionState::DocumentOpen
        } else {
            ViewerSessionState::NoDocument
        }
    })
}

thread_local! {
    pub static VIEWER_SESSION: RefCell<HostViewerSession> =
        RefCell::new(HostViewerSession::default());
}

pub fn reset_viewer_session() {
    VIEWER_SESSION.with(|session| {
        *session.borrow_mut() = HostViewerSession::default();
    });
}

pub fn set_viewer_document(path: Option<String>, page_count: u16, initial_zoom: f32) {
    // zoom 不在此处存储 —— 权威是 ZOOM_STATE（ADR-0001）。
    log::debug!("[AUTHORITY] set_viewer_document: path={:?}, page_count={}, initial_zoom={}", path, page_count, initial_zoom);
    crate::zoom::zoom_controller::set_target_zoom_authoritative(initial_zoom);
    VIEWER_SESSION.with(|session| {
        let mut session = session.borrow_mut();
        session.path = path;
        session.current_page = 0;
        session.page_count = page_count;
        session.document_revision = 0;
    });
}

pub fn set_current_page(page_index: u16) {
    VIEWER_SESSION.with(|session| {
        session.borrow_mut().current_page = page_index;
    });
}

pub fn set_zoom_and_page_dimensions(zoom: f32, page_width: Option<f32>, page_height: Option<f32>) {
    // zoom 走权威单入口；页面尺寸仍归 session 存储。
    crate::zoom::zoom_controller::set_target_zoom_authoritative(zoom);
    VIEWER_SESSION.with(|session| {
        let mut session = session.borrow_mut();
        if let Some(w) = page_width {
            session.page_width = w.max(1.0);
        }
        if let Some(h) = page_height {
            session.page_height = h.max(1.0);
        }
    });
}

/// Session 快照。`current_zoom` 是派生投影：从缩放权威 ZOOM_STATE 的
/// target_zoom 填充，本存储不再持有该字段（ADR-0001）。
pub fn read_viewer_session() -> HostViewerSession {
    let authority_zoom = crate::zoom::zoom_controller::read_zoom_state().target_zoom;
    let mut snapshot = VIEWER_SESSION.with(|session| session.borrow().clone());
    snapshot.current_zoom = authority_zoom;
    log::debug!("[AUTHORITY] read_viewer_session: path={:?}, authority_zoom={}, page_count={}", snapshot.path, authority_zoom, snapshot.page_count);
    snapshot
}

pub fn set_page_dimensions(page_width: f32, page_height: f32) {
    VIEWER_SESSION.with(|session| {
        let mut session = session.borrow_mut();
        session.page_width = page_width.max(1.0);
        session.page_height = page_height.max(1.0);
    });
}

pub fn bump_document_revision() -> u64 {
    VIEWER_SESSION.with(|session| {
        let mut session = session.borrow_mut();
        session.document_revision = session.document_revision.wrapping_add(1).max(1);
        session.document_revision
    })
}

pub fn current_document_revision() -> u64 {
    VIEWER_SESSION.with(|session| session.borrow().document_revision)
}
