use serde::{Deserialize, Serialize};

use crate::editor::platform_bridge::reset_state as reset_editor_host_state;
use crate::editor::session::{close_active_editor, reset_editor_mode};
use crate::find::find_store::clear_find_session;
use crate::page::page_store::reset_progressive_render_task;
use crate::present::present_store::reset_present_runtime;
use crate::render::platform_bridge::reset_runtime as reset_render_loop_runtime;
use crate::render::render_store::reset_render_state;
use crate::review::review_store::clear_review_session as clear_comment_review_session;
use crate::ui_state_store::clear_persistable_patches;
use crate::viewer::viewer_store::{
    bump_document_revision, read_viewer_session, reset_viewer_session, set_current_page,
    set_current_zoom, set_page_dimensions, set_viewer_document, HostViewerSession,
};
use crate::zoom::preview_host::{clear_preview_state, settle_at_target};
use crate::zoom::zoom_controller::reset_zoom_runtime;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ViewerRuntimeResetOptions {
    pub reset_cache: bool,
    pub reset_refresh: bool,
    pub reset_editor_mode: bool,
    pub clear_patches: bool,
}

pub fn reset_viewer_runtime(options: ViewerRuntimeResetOptions) {
    reset_render_state();
    reset_present_runtime(options.reset_cache, options.reset_refresh);
    reset_progressive_render_task();
    reset_render_loop_runtime();
    reset_editor_host_state();
    clear_preview_state(true);
    if options.reset_editor_mode {
        reset_editor_mode();
    } else {
        close_active_editor();
    }
    if options.clear_patches {
        clear_persistable_patches(true);
    }
    clear_find_session();
    clear_comment_review_session();
}

pub fn reset_session() {
    reset_viewer_session();
    reset_viewer_runtime(ViewerRuntimeResetOptions {
        reset_cache: true,
        reset_refresh: true,
        reset_editor_mode: true,
        clear_patches: true,
    });
}

pub fn reset_zoom_view(initial_zoom: f32) {
    reset_zoom_runtime(initial_zoom);
    reset_viewer_runtime(ViewerRuntimeResetOptions {
        reset_cache: true,
        reset_refresh: true,
        reset_editor_mode: true,
        clear_patches: false,
    });
}

pub fn note_document_mutation(_reason: &str) -> u64 {
    let revision = bump_document_revision();
    reset_render_state();
    reset_present_runtime(true, true);
    reset_progressive_render_task();
    reset_render_loop_runtime();
    settle_at_target();
    revision
}

pub fn read_session() -> HostViewerSession {
    read_viewer_session()
}

pub fn set_document(path: Option<String>, page_count: u16, initial_zoom: f32) {
    set_viewer_document(path, page_count, initial_zoom);
    reset_viewer_runtime(ViewerRuntimeResetOptions {
        reset_cache: true,
        reset_refresh: true,
        reset_editor_mode: false,
        clear_patches: true,
    });
}

pub fn set_page(page_index: u16) {
    set_current_page(page_index);
    reset_viewer_runtime(ViewerRuntimeResetOptions {
        reset_cache: false,
        reset_refresh: true,
        reset_editor_mode: false,
        clear_patches: false,
    });
    // Unified EventBus (Nutrient borrowing #1)
    #[cfg(target_arch = "wasm32")]
    crate::events::emit(
        crate::events::event_names::VIEWER_PAGE_CHANGE,
        &wasm_bindgen::JsValue::from(page_index),
    );
}

pub fn set_zoom(zoom: f32) {
    set_current_zoom(zoom);
    // Unified EventBus (Nutrient borrowing #1)
    #[cfg(target_arch = "wasm32")]
    crate::events::emit(
        crate::events::event_names::VIEWER_ZOOM_CHANGE,
        &wasm_bindgen::JsValue::from(zoom),
    );
}

pub fn set_page_size(page_width: f32, page_height: f32) {
    log::info!(
        "[PAGE-SIZE] set_page_size called. Width={}, Height={}",
        page_width,
        page_height
    );
    set_page_dimensions(page_width, page_height);
}
