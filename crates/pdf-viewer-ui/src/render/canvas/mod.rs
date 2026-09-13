//! Canvas rendering module — split from the monolithic canvas.rs (zoom closeout #05).
//!
//! Sub-modules by cohesion:
//! - `debug`:      debug tracing helpers (active shell bbox, draw call logging)
//! - `surface`:    CanvasRenderer construction, size/sync, page surface transforms
//! - `draw`:       draw_text_run_core, TextMetricsSnapshot, render_run_standalone
//! - `vector`:     draw_vector_object dispatch + path/image/text object drawing
//! - `page`:       render_page, render_vector_slice (progressive pipeline)
//! - `renderer`:   impl PdfRenderer trait + primitive draw commands (text/rect/line)

mod debug;
mod draw;
mod page;
mod renderer;
mod surface;
mod vector;

use std::cell::Cell;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

// ─── Public types (struct/enum definitions live in mod.rs) ────────

#[derive(Clone, Copy)]
pub(crate) enum CoordinateMode {
    PageSpace,
    EditorLocal,
}

pub struct CanvasRenderer {
    pub ctx: CanvasRenderingContext2d,
    pub canvas: HtmlCanvasElement,
    pub dpr: f32,
    pub canvas_height: Cell<f32>,
    pub is_hijacked: bool,
    pub transparent_surface: bool,
}

// Re-exports for existing import paths.
pub(crate) use draw::draw_text_run_core;
pub use draw::{render_run_standalone, TextMetricsSnapshot};
