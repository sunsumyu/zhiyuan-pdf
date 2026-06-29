pub mod activation;
pub mod bridge;
pub mod command;
pub mod debug_trace;
pub mod editor_api;
pub mod editor_controller;
pub mod editor_format;
pub mod editor_store;
pub mod editor_types;
pub mod engine_state;
pub mod host_mode;
pub mod host_snapshot;
pub mod host_workflow;
pub mod mode;
pub mod orchestrator;
pub mod platform_bridge;
pub mod replacement_region;
pub mod replacement_snapshot;
pub mod workflow;

// Sub-modules
pub mod format;
pub mod overlay;
pub mod session;

// Direct re-exports from core (previously via draft/ and source/ shim dirs)
pub use pdf_viewer_core::edit::document_edit_ops;
pub use pdf_viewer_core::edit::document_plan;
pub use pdf_viewer_core::edit::document_runtime;
pub use pdf_viewer_core::edit::draft_layout;
pub use pdf_viewer_core::edit::edit_target;
pub use pdf_viewer_core::edit::source_identity;
pub use pdf_viewer_core::edit::source_runs;
pub use pdf_viewer_core::edit::source_text;
pub use pdf_viewer_core::geometry::source_geometry;

pub use overlay::navigation;
pub use overlay::paragraph_overlay;
pub use overlay::paragraph_scene;
pub use overlay::projection;
pub use overlay::visual;

pub use format::list_format;
pub use format::target_resolution;
pub use format::text_geometry;
pub use format::text_index;
pub use format::text_model;
