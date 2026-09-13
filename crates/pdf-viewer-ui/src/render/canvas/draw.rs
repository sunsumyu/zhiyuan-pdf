//! Low-level canvas drawing primitives (text run core, metrics, standalone export).
//!
//! `draw_text_run_core` is the shared text rendering path used by both the
//! standalone WASM export (`render_run_standalone`) and
//! `CanvasRenderer::draw_text_run`.

use super::debug::debug_log_canvas_method;
use super::CoordinateMode;
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

pub struct TextMetricsSnapshot {
    pub width: f32,
    pub _ascent: f32,
    pub _descent: f32,
}

/// Standalone WASM entry point — renders a single text run at page space coords.
// Canvas primitives arrive individually by design; grouping them into a
// params struct would hurt the direct mapping to the 2D context API.
#[allow(clippy::too_many_arguments)]
#[wasm_bindgen]
pub fn render_run_standalone(
    ctx: CanvasRenderingContext2d,
    dpr: f32,
    text: String,
    x: f32,
    baseline_y: f32,
    font_size: f32,
    color: String,
    font_name: String,
    font_weight: String,
    font_style: String,
    is_underline: bool,
    scale_x: f32,
    render_mode: i32,
    char_origins: Option<Vec<f32>>,
) {
    draw_text_run_core(
        &ctx,
        dpr,
        &text,
        x,
        baseline_y,
        font_size,
        &color,
        &font_name,
        &font_weight,
        &font_style,
        is_underline,
        scale_x,
        render_mode,
        char_origins.as_deref(),
        CoordinateMode::PageSpace,
    );
}

/// Core text run renderer — shared by render_run_standalone and
/// CanvasRenderer::draw_text_run. Renders text with optional per-glyph
/// origins (for precise kerning) and underline.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_text_run_core(
    ctx: &CanvasRenderingContext2d,
    dpr: f32,
    text: &str,
    x: f32,
    baseline_y: f32,
    font_size: f32,
    color: &str,
    font_name: &str,
    font_weight: &str,
    font_style: &str,
    is_underline: bool,
    scale_x: f32,
    render_mode: i32,
    char_origins: Option<&[f32]>,
    coordinate_mode: CoordinateMode,
) {
    if render_mode == 3 {
        return;
    }
    let snap_to_pixel = |val: f32| -> f64 { (val * dpr).round() as f64 / dpr as f64 };
    let y_scale = match coordinate_mode {
        CoordinateMode::PageSpace => 1.0,
        CoordinateMode::EditorLocal => 1.0,
    };
    let effective_weight = if font_weight == "bold" { "600" } else { "400" };

    ctx.save();
    ctx.set_font(&format!(
        "{} {} {}px {}",
        font_style, effective_weight, font_size, font_name
    ));
    ctx.set_fill_style_str(color);
    ctx.set_stroke_style_str(color);
    ctx.set_text_baseline("alphabetic");
    ctx.set_line_join("round");
    ctx.set_miter_limit(2.0);
    ctx.set_line_width((font_size * 0.03).max(0.4) as f64);

    let snapped_x = snap_to_pixel(x);
    let snapped_baseline_y = snap_to_pixel(baseline_y);
    let _ = ctx.translate(snapped_x, snapped_baseline_y);

    if let Some(origins) = char_origins {
        ctx.save();
        let _ = ctx.scale(1.0, y_scale);
        for (index, ch) in text.chars().enumerate() {
            let mut glyph_buf = [0_u8; 4];
            let glyph = ch.encode_utf8(&mut glyph_buf);
            let origin_x = origins.get(index).copied().unwrap_or(0.0);
            let origin_x = snap_to_pixel(origin_x);
            if render_mode == 1 || render_mode == 2 {
                let _ = ctx.stroke_text(glyph, origin_x, 0.0);
            }
            if render_mode == 0 || render_mode == 2 {
                let _ = ctx.fill_text(glyph, origin_x, 0.0);
            }
        }
        ctx.restore();
    } else {
        ctx.save();
        let _ = ctx.scale(scale_x as f64, y_scale);
        if render_mode == 1 || render_mode == 2 {
            let _ = ctx.stroke_text(text, 0.0, 0.0);
        }
        if render_mode == 0 || render_mode == 2 {
            let _ = ctx.fill_text(text, 0.0, 0.0);
        }
        ctx.restore();
    }

    if is_underline {
        let measured_width = ctx
            .measure_text(text)
            .ok()
            .map(|metrics| metrics.width() as f32)
            .unwrap_or(0.0);
        let underline_width = if let Some(origins) = char_origins {
            let glyph_count = text.chars().count();
            if glyph_count <= 1 {
                measured_width.max(0.0)
            } else {
                let average_width = measured_width / glyph_count.max(1) as f32;
                origins.last().copied().unwrap_or(0.0) + average_width.max(0.0)
            }
        } else {
            measured_width * scale_x.max(0.01)
        };
        if underline_width > 0.0 {
            let underline_y = snap_to_pixel(font_size * 0.12);
            debug_log_canvas_method("underline-stroke", "text", None, None, None, vec![]);
            ctx.begin_path();
            ctx.set_line_width((font_size * 0.055).max(0.8) as f64);
            ctx.move_to(0.0, underline_y);
            ctx.line_to(snap_to_pixel(underline_width), underline_y);
            ctx.stroke();
        }
    }

    ctx.restore();
}
