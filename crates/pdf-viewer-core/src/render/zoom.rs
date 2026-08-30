//! Zoom subsystem — consolidated module for all zoom logic.
//!
//! Sub-modules:
//! - `state`: Data structures (HostZoomState, DrawingDelayState, etc.)
//! - `zoom_decide`: Wheel render decisions, commit/flush staleness guards
//! - `zoom_css`: CSS transform computation, canvas box, layout geometry
//! - `zoom_render`: Render timing engine, blur thresholds, reknock gating
//! - `zoom_tick`: State machine orchestrator (tick_zoom_state_core)
//! - `animation`: Animation interpolation and anchor computation
//! - `presentation`: Surface operation state machine (ADR-0002)
//! - `decision`: Re-export hub for backward compatibility

pub mod state;
pub mod zoom_decide;
pub mod zoom_css;
pub mod zoom_render;
pub mod zoom_tick;
pub mod animation;
pub mod presentation;
pub mod decision;

// Re-export everything from sub-modules so external code can use
// `pdf_viewer_core::render::zoom::HostZoomState` etc.
pub use state::*;
pub use zoom_decide::*;
pub use zoom_css::*;
pub use zoom_render::*;
pub use zoom_tick::*;
pub use animation::*;
