use serde::{Deserialize, Serialize};

use crate::viewer::viewer_controller::{
    get_session, reset_session, set_document, set_page_size, set_page, set_zoom,
};
use crate::zoom::zoom_controller::{reset_zoom_runtime, set_target_zoom};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OpenDocumentSessionRequest {
    pub path: String,
    pub page_count: u16,
    pub initial_zoom: f32,
    pub default_page_width: f32,
    pub default_page_height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HostActionResult {
    pub changed: bool,
    pub current_page: u16,
    pub current_zoom: f32,
}

pub fn open_document_session(request: OpenDocumentSessionRequest) -> HostActionResult {
    let initial_zoom = if request.initial_zoom.is_finite() && request.initial_zoom > 0.0 {
        request.initial_zoom
    } else {
        1.0
    };
    set_document(Some(request.path), request.page_count, initial_zoom);
    set_page_size(request.default_page_width, request.default_page_height);
    reset_zoom_runtime(initial_zoom);
    let session = get_session();
    HostActionResult {
        changed: true,
        current_page: session.current_page,
        current_zoom: session.current_zoom,
    }
}

pub fn reset_host_document_session(
    default_page_width: f32,
    default_page_height: f32,
) -> HostActionResult {
    reset_session();
    reset_zoom_runtime(1.0);
    set_page_size(default_page_width, default_page_height);
    let session = get_session();
    HostActionResult {
        changed: true,
        current_page: session.current_page,
        current_zoom: session.current_zoom,
    }
}

pub fn navigate_prev_page() -> HostActionResult {
    let session = get_session();
    if session.path.is_none() || session.current_page == 0 {
        return HostActionResult {
            changed: false,
            current_page: session.current_page,
            current_zoom: session.current_zoom,
        };
    }
    let next_page = session.current_page.saturating_sub(1);
    set_page(next_page);
    HostActionResult {
        changed: true,
        current_page: next_page,
        current_zoom: session.current_zoom,
    }
}

pub fn navigate_next_page() -> HostActionResult {
    let session = get_session();
    if session.path.is_none() || session.current_page + 1 >= session.page_count {
        return HostActionResult {
            changed: false,
            current_page: session.current_page,
            current_zoom: session.current_zoom,
        };
    }
    let next_page = session.current_page + 1;
    set_page(next_page);
    HostActionResult {
        changed: true,
        current_page: next_page,
        current_zoom: session.current_zoom,
    }
}

pub fn apply_zoom_selection(zoom: f32) -> HostActionResult {
    let session = get_session();
    if session.path.is_none() {
        return HostActionResult {
            changed: false,
            current_page: session.current_page,
            current_zoom: session.current_zoom,
        };
    }
    set_target_zoom(zoom);
    set_zoom(zoom);
    HostActionResult {
        changed: true,
        current_page: session.current_page,
        current_zoom: zoom,
    }
}
