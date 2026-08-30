//! Decision module — re-export hub for all zoom decision sub-modules.
//!
//! The actual implementations live in focused sub-modules:
//! - `zoom_decide`: Wheel render decisions, commit/flush staleness guards
//! - `zoom_css`: CSS transform computation, canvas box, layout geometry
//! - `zoom_render`: Render timing engine, blur thresholds, reknock gating
//! - `zoom_tick`: State machine orchestrator (tick_zoom_state_core)
//!
//! This module re-exports everything so existing import paths
//! (`pdf_viewer_core::render::zoom::decision::*`) continue to work.

pub use super::zoom_decide::*;
pub use super::zoom_css::*;
pub use super::zoom_render::*;
pub use super::zoom_tick::*;
