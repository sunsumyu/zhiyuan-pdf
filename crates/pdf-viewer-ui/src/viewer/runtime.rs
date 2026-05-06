use serde::{Deserialize, Serialize};

use crate::editor::host_runtime::reset_state as reset_editor_host_state;
use crate::editor::session::{
    close_active_editor as host_close_active_editor,
    reset_editor_mode as host_reset_editor_mode,
};
use crate::page::runtime::reset_progressive_render_task as host_reset_progressive_render_task;
use crate::present::runtime::reset_present_runtime as host_reset_present_runtime;
use crate::render::host_runtime::reset_render_loop_runtime as host_reset_render_loop_runtime;
use crate::render::scheduler::reset_render_state as host_reset_render_state;
use crate::state_manager::clear_persistable_patches;
use crate::viewer::comment_review::clear_comment_review_session as host_clear_comment_review_session;
use crate::viewer::find::clear_find_session as host_clear_find_session;
use crate::viewer::session::{
    bump_document_revision as host_bump_document_revision,
    reset_viewer_session as host_reset_viewer_session,
    set_current_page as host_set_current_page,
    set_current_zoom as host_set_current_zoom,
    set_page_dimensions as host_set_page_dimensions,
    set_viewer_document as host_set_viewer_document, HostViewerSession,
    HOST_VIEWER_SESSION,
};
use crate::zoom::preview_host::{
    clear_zoom_preview_host_state as host_clear_zoom_preview_host_state,
    settle_zoom_preview_at_target as host_settle_zoom_preview_at_target,
};
use crate::zoom::runtime::reset_zoom_runtime as host_reset_zoom_runtime;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ViewerRuntimeResetOptions {
    pub reset_cache: bool,
    pub reset_refresh: bool,
    pub reset_editor_mode: bool,
    pub clear_patches: bool,
}

pub fn reset_viewer_runtime(options: ViewerRuntimeResetOptions) {
    host_reset_render_state();
    host_reset_present_runtime(options.reset_cache, options.reset_refresh);
    host_reset_progressive_render_task();
    host_reset_render_loop_runtime();
    reset_editor_host_state();
    host_clear_zoom_preview_host_state(true);
    if options.reset_editor_mode {
        host_reset_editor_mode();
    } else {
        host_close_active_editor();
    }
    if options.clear_patches {
        clear_persistable_patches(true);
    }
    host_clear_find_session();
    host_clear_comment_review_session();
}

pub fn reset_session() {
    host_reset_viewer_session();
    reset_viewer_runtime(ViewerRuntimeResetOptions {
        reset_cache: true,
        reset_refresh: true,
        reset_editor_mode: true,
        clear_patches: true,
    });
}

pub fn reset_zoom_view(initial_zoom: f32) {
    host_reset_zoom_runtime(initial_zoom);
    reset_viewer_runtime(ViewerRuntimeResetOptions {
        reset_cache: true,
        reset_refresh: true,
        reset_editor_mode: true,
        clear_patches: false,
    });
}

pub fn note_document_mutation(_reason: &str) -> u64 {
    let revision = host_bump_document_revision();
    host_reset_render_state();
    host_reset_present_runtime(true, true);
    host_reset_progressive_render_task();
    host_reset_render_loop_runtime();
    host_settle_zoom_preview_at_target();
    revision
}

pub fn get_session() -> HostViewerSession {
    HOST_VIEWER_SESSION.with(|session| session.borrow().clone())
}

pub fn set_document(path: Option<String>, page_count: u16, initial_zoom: f32) {
    host_set_viewer_document(path, page_count, initial_zoom);
    reset_viewer_runtime(ViewerRuntimeResetOptions {
        reset_cache: true,
        reset_refresh: true,
        reset_editor_mode: false,
        clear_patches: true,
    });
}

pub fn set_page(page_index: u16) {
    host_set_current_page(page_index);
    reset_viewer_runtime(ViewerRuntimeResetOptions {
        reset_cache: false,
        reset_refresh: true,
        reset_editor_mode: false,
        clear_patches: false,
    });
}

pub fn set_zoom(zoom: f32) {
    host_set_current_zoom(zoom);
}

pub fn set_page_size(page_width: f32, page_height: f32) {
    log::info!(
        "[PAGE-SIZE] set_page_size called. Width={}, Height={}",
        page_width,
        page_height
    );
    host_set_page_dimensions(page_width, page_height);
}
