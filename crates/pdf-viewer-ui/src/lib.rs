pub mod api;
pub mod application;
pub mod events;
pub mod geometry_api;

pub mod annotation;
pub mod app_controller;
pub mod comment;
pub mod document;
pub mod editor;
pub mod find;
pub mod host;
pub mod page;
pub mod present;
pub mod presentation;
pub mod render;
pub mod review;
pub mod runtime;
pub mod style_mapper;
pub mod ui_state_store;
pub mod common;
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
