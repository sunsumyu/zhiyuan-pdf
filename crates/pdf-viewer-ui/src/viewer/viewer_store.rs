use std::cell::RefCell;

// Re-export pure data structure from core.
pub use pdf_viewer_core::render::viewer_session::*;

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

pub fn set_zoom_and_page_dimensions(zoom: f32, page_width: Option<f32>, page_height: Option<f32>) {
    HOST_VIEWER_SESSION.with(|session| {
        let mut session = session.borrow_mut();
        session.current_zoom = sanitize_zoom(zoom);
        if let Some(w) = page_width {
            session.page_width = w.max(1.0);
        }
        if let Some(h) = page_height {
            session.page_height = h.max(1.0);
        }
    });
}

pub fn get_viewer_session() -> HostViewerSession {
    HOST_VIEWER_SESSION.with(|session| session.borrow().clone())
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
