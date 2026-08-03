use serde::{Deserialize, Serialize};

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

use crate::app_context;

/// Snapshot of the current viewer session state.
pub fn read_viewer_state() -> ViewerSessionState {
    app_context::with_viewer(|viewer| {
        if viewer.path.is_some() {
            ViewerSessionState::DocumentOpen
        } else {
            ViewerSessionState::NoDocument
        }
    })
}

pub fn reset_viewer_session() {
    app_context::with_viewer_mut(|viewer| {
        *viewer = HostViewerSession::default();
    });
}

pub fn set_viewer_document(path: Option<String>, page_count: u16, initial_zoom: f32) {
    app_context::with_viewer_mut(|session| {
        session.path = path;
        session.current_page = 0;
        session.page_count = page_count;
        session.current_zoom = sanitize_zoom(initial_zoom);
        session.document_revision = 0;
    });
}

pub fn set_current_page(page_index: u16) {
    app_context::with_viewer_mut(|viewer| {
        viewer.current_page = page_index;
    });
}

pub fn set_current_zoom(zoom: f32) {
    app_context::with_viewer_mut(|viewer| {
        viewer.current_zoom = sanitize_zoom(zoom);
    });
}

pub fn set_zoom_and_page_dimensions(zoom: f32, page_width: Option<f32>, page_height: Option<f32>) {
    app_context::with_viewer_mut(|session| {
        session.current_zoom = sanitize_zoom(zoom);
        if let Some(w) = page_width {
            session.page_width = w.max(1.0);
        }
        if let Some(h) = page_height {
            session.page_height = h.max(1.0);
        }
    });
}

pub fn read_viewer_session() -> HostViewerSession {
    app_context::with_viewer(Clone::clone)
}

pub fn set_page_dimensions(page_width: f32, page_height: f32) {
    app_context::with_viewer_mut(|session| {
        session.page_width = page_width.max(1.0);
        session.page_height = page_height.max(1.0);
    });
}

pub fn bump_document_revision() -> u64 {
    app_context::with_viewer_mut(|session| {
        session.document_revision = session.document_revision.wrapping_add(1).max(1);
        session.document_revision
    })
}

pub fn current_document_revision() -> u64 {
    app_context::with_viewer(|viewer| viewer.document_revision)
}

pub fn update_mutable_fields(f: impl FnOnce(&mut HostViewerSession)) {
    app_context::with_viewer_mut(f);
}

fn sanitize_zoom(value: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        1.0
    }
}
