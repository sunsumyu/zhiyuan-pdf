pub mod free_api;
pub mod raf_committed;
pub mod raf_dispatch;
pub mod raf_dom_cache;
pub mod raf_loop;
pub mod raf_settle;
pub mod raf_transform;
pub mod zoom_anchor;
pub mod zoom_authority;
pub mod zoom_controller;
pub mod zoom_frame;
pub mod zoom_preview;
pub mod zoom_store;

#[cfg(test)]
mod authority_tests;

// Backward-compatible re-exports
pub use zoom_controller::{
    execute_wheel_zoom, step_preview_host, WheelZoomHostRequest, WheelZoomHostResult,
    PreviewHostStepRequest, PreviewHostStepResult,
    reset_zoom_preview_host, clear_preview_host_with_anchor, settle_zoom_preview_at_target,
    set_wheel_render_pending, set_preview_active, set_cancel_pending_render,
    take_cancel_pending_render, is_preview_active, is_wheel_render_pending,
    queue_committed_frame, take_ready_committed_frame,
    resolve_wheel_zoom, resolve_anchor_scroll,
};
