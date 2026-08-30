pub mod comment_review_state;
pub mod effective_page_plan;
pub mod facade_types;
pub mod find_state;
pub mod frame_cache;
pub mod layer;
pub mod paint_plan;
pub mod path_suppression;
pub mod plan_builder;
pub mod prepared_scene;
pub mod present_plan;
pub mod preview;
pub mod progressive;
pub mod quality;
pub mod renderer;
pub mod scheduler;
pub mod snapshot_paint_plan;
pub mod source_suppression;
pub mod tile_cache;
pub mod tile_cache_legacy;
pub mod tile_manager;
pub mod tile_v2;
pub mod viewer_session;
pub mod viewport_culling;
pub mod viewport_refresh;
pub mod workflow;
pub mod zoom;

// Consolidated zoom subsystem — compatibility aliases so consumers (UI crate,
// zoom/ submodules) can keep addressing symbols through the pre-consolidation
// module paths.
pub use zoom as zoom_host;
pub use zoom::animation as zoom_interaction;
pub use zoom::state as zoom_state;
