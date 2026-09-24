pub mod free_api;
pub mod raf_committed;
pub mod raf_dispatch;
pub mod raf_dom_cache;
pub mod raf_loop;
pub mod raf_settle;
pub mod zoom_authority;
pub mod zoom_controller;
pub mod zoom_frame;
pub mod zoom_preview;
pub mod zoom_store;

#[cfg(test)]
mod authority_tests;

#[cfg(test)]
mod frame_tests;

// Backward-compatible re-exports
pub use zoom_controller::{
    execute_wheel_zoom, is_preview_active, is_wheel_render_pending, queue_committed_frame,
    reset_zoom_preview_host, resolve_wheel_zoom, set_cancel_pending_render, set_preview_active,
    set_wheel_render_pending, settle_zoom_preview_at_target, step_preview_host,
    take_cancel_pending_render, take_ready_committed_frame, PreviewHostStepRequest,
    PreviewHostStepResult, WheelZoomHostRequest, WheelZoomHostResult,
};
