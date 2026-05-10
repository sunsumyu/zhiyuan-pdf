pub mod api;

pub mod annotation;
pub mod comment;
pub mod document;
pub mod editor;
pub mod find;
pub mod host;
pub mod review;
pub mod page;
pub mod present;
pub mod render;
pub mod app_controller;
pub mod runtime;
pub mod state_manager;
pub mod style_mapper;
pub mod utils;
pub mod viewer;
pub mod viewport_culling;
pub mod viewport_refresh;
pub mod zoom;

pub mod bridge;
pub mod dom_projection;
pub mod models;
pub mod projection_workflow;

// All legacy `pub use ... as ..._workflow` aliases were removed in Phase 3.
// New code uses canonical paths (e.g. `crate::editor::session`).

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

