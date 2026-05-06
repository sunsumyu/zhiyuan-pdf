pub mod activation;
pub mod bridge;
pub mod command;
pub mod commit;
pub mod debug_trace;
pub mod engine_state;
pub mod host_mode;
pub mod host_runtime;
pub mod host_snapshot;
pub mod host_workflow;
pub mod mode;
pub mod render_transaction;
pub mod replace_pipeline;
pub mod replacement_region;
pub mod replacement_snapshot;
pub mod runtime;
pub mod workflow;

// Sub-modules (P2 reorganization)
pub mod source;
pub mod draft;
pub mod session;
pub mod overlay;
pub mod format;
pub mod facade;
pub mod search_facade;
pub mod review_facade;
pub mod ai_facade;
pub mod render_facade;
pub mod annotation_facade;

// Re-exports for backward compatibility
pub use source::source_geometry;
pub use source::source_identity;
pub use source::source_runs;
pub use source::source_text;

pub use draft::draft_layout;
pub use draft::edit_target;
pub use draft::edited_text_layout;

pub use session::document_edit_ops;
pub use session::document_plan;
pub use session::document_runtime;

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
