use crate::common::bbox::bbox_intersects;
use crate::editor::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use crate::editor::mode::read_state;
use crate::editor::replacement_region::build_region;
use js_sys;
use pdf_viewer_core::models::{BoundingBox, PageState};
use pdf_viewer_core::render::renderer::{DrawCommand, PdfRenderer};
use std::cell::Cell;
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

pub mod vector_draw;

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

pub struct TextMetricsSnapshot {
    pub width: f32,
    pub _ascent: f32,
    pub _descent: f32,
}

pub(super) fn current_time_ms() -> f64 {
    js_sys::Date::now()
}

pub(super) fn active_shell_bbox_for_debug() -> Option<BoundingBox> {
    read_state().map(|state| build_region(&state.target).text_clear_bbox)
}

pub(super) fn debug_bbox_intersects_active_shell(bbox: &BoundingBox) -> bool {
    active_shell_bbox_for_debug()
        .map(|shell| bbox_intersects(bbox, &shell))
        .unwrap_or(false)
}

pub(super) fn debug_log_canvas_method(
    action: &str,
    object_type: &str,
    object_index: Option<usize>,
    object_id: Option<&str>,
    bbox: Option<BoundingBox>,
    extra: Vec<crate::editor::debug_trace::EditorDebugField>,
) {
    let intersects_shell = bbox
        .as_ref()
        .map(debug_bbox_intersects_active_shell)
        .unwrap_or(false);
    if !intersects_shell {
        return;
    }
    let mut details = vec![
        dbg_field("objectType", object_type),
        dbg_field("intersectsShell", intersects_shell),
    ];
    if let Some(index) = object_index {
        details.push(dbg_field("objectIndex", index));
    }
    if let Some(id) = object_id {
        details.push(dbg_field("objectId", id));
    }
    if let Some(bounds) = bbox {
        details.push(dbg_field(
            "bbox",
            format!(
                "{:.1},{:.1},{:.1},{:.1}",
                bounds.left, bounds.top, bounds.right, bounds.bottom
            ),
        ));
    }
    details.extend(extra);
    dbg_event("canvas.draw", action, details);
}

impl CanvasRenderer {
    pub fn new_overlay(canvas: HtmlCanvasElement) -> Self {
        let attrs = web_sys::ContextAttributes2d::new();
        attrs.set_alpha(true);

        let ctx = canvas
            .get_context_with_context_options("2d", &attrs.into())
            .unwrap()
            .unwrap()
            .dyn_into::<CanvasRenderingContext2d>()
            .unwrap();

        let dpr = web_sys::window().unwrap().device_pixel_ratio() as f32;

        Self {
            ctx,
            canvas,
            dpr,
            canvas_height: Cell::new(0.0),
            is_hijacked: false,
            transparent_surface: true,
        }
    }

    pub fn new_hijacked(target_id: &str) -> Option<Self> {
        let window = web_sys::window()?;
        let document = window.document()?;
        let canvas = document
            .get_element_by_id(target_id)?
            .dyn_into::<HtmlCanvasElement>()
            .ok()?;

        let ctx = canvas
            .get_context("2d")
            .ok()?
            .and_then(|c| c.dyn_into::<CanvasRenderingContext2d>().ok())?;

        let dpr = window.device_pixel_ratio() as f32;

        Some(Self {
            ctx,
            canvas,
            dpr,
            canvas_height: Cell::new(0.0),
            is_hijacked: true,
            transparent_surface: false,
        })
    }

    pub fn new_offscreen(canvas_js: JsValue, dpr: f32) -> Option<Self> {
        let canvas: HtmlCanvasElement = canvas_js.unchecked_into();
        let ctx_val = canvas.get_context("2d").ok()??;
        let ctx: CanvasRenderingContext2d = ctx_val.unchecked_into();

        Some(Self {
            ctx,
            canvas,
            dpr,
            canvas_height: Cell::new(0.0),
            is_hijacked: true, // don't resize DOM
            transparent_surface: false,
        })
    }

    /// 根据当前容器尺寸同步 Canvas 大小
    pub fn sync_size(&self, width: f32, height: f32, zoom: f32) {
        if self.is_hijacked {
            return;
        }
        self.canvas_height.set(height);
        let _ = self.canvas.set_width((width * self.dpr) as u32);
        let _ = self.canvas.set_height((height * self.dpr) as u32);
        let style = self.canvas.style();
        style
            .set_property("width", &format!("{}px", width))
            .unwrap();
        style
            .set_property("height", &format!("{}px", height))
            .unwrap();

        let combined_scale = (self.dpr * zoom) as f64;
        let _ = self
            .ctx
            .set_transform(combined_scale, 0.0, 0.0, combined_scale, 0.0, 0.0);
    }

    pub fn measure_text_metrics(
        &self,
        text: &str,
        font_size: f32,
        font_name: &str,
        font_weight: &str,
        font_style: &str,
    ) -> TextMetricsSnapshot {
        self.ctx.set_font(&format!(
            "{} {} {}px {}",
            font_style, font_weight, font_size, font_name
        ));
        let measure_target = if text.is_empty() { "Hg" } else { text };
        match self.ctx.measure_text(measure_target) {
            Ok(metrics) => TextMetricsSnapshot {
                width: if text.is_empty() {
                    0.0
                } else {
                    metrics.width() as f32
                },
                _ascent: metrics.actual_bounding_box_ascent() as f32,
                _descent: metrics.actual_bounding_box_descent() as f32,
            },
            Err(_) => TextMetricsSnapshot {
                width: 0.0,
                _ascent: font_size * 0.8,
                _descent: font_size * 0.2,
            },
        }
    }

    pub fn draw_text_run(
        &self,
        text: &str,
        x: f32,
        baseline_y: f32,
        font_size: f32,
        color: &str,
        font_name: &str,
        font_weight: u16,
        font_style: &str,
        is_underline: bool,
        scale_x: f32,
        render_mode: i32,
        char_origins: Option<&[f32]>,
    ) {
        draw_text_run_core(
            &self.ctx,
            self.dpr,
            text,
            x,
            baseline_y,
            font_size,
            color,
            font_name,
            font_weight,
            font_style,
            is_underline,
            scale_x,
            render_mode,
            char_origins,
            CoordinateMode::EditorLocal,
        );
    }

    pub fn clear_dirty_rect(&self, x: f32, y: f32, w: f32, h: f32) {
        if self.transparent_surface {
            self.ctx.clear_rect(
                x as f64 - 0.5,
                y as f64 - 0.5,
                (w + 1.0) as f64,
                (h + 1.0) as f64,
            );
            return;
        }
        self.ctx.set_fill_style_str("#ffffff");
        self.ctx.fill_rect(
            x as f64 - 0.5,
            y as f64 - 0.5,
            (w + 1.0) as f64,
            (h + 1.0) as f64,
        );
    }

    pub(super) fn prepare_page_surface(
        &self,
        state: &PageState,
        _page_width: f32,
        _page_height: f32,
    ) {
        let page_scale = (state.zoom * state.dpr) as f64;
        let viewport_left_px = (state.viewport_left * state.dpr).max(0.0) as f64;
        let viewport_top_px = (state.viewport_top * state.dpr).max(0.0) as f64;

        let _ = self.ctx.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);
        self.ctx.set_fill_style_str("#ffffff");
        self.ctx.fill_rect(
            0.0,
            0.0,
            self.canvas.width() as f64,
            self.canvas.height() as f64,
        );
        let _ = self.ctx.set_transform(
            page_scale,
            0.0,
            0.0,
            page_scale,
            -viewport_left_px,
            -viewport_top_px,
        );
    }

    pub(super) fn apply_page_transform(
        &self,
        state: &PageState,
        _page_width: f32,
        _page_height: f32,
    ) {
        let page_scale = (state.zoom * state.dpr) as f64;
        let viewport_left_px = (state.viewport_left * state.dpr).max(0.0) as f64;
        let viewport_top_px = (state.viewport_top * state.dpr).max(0.0) as f64;
        let _ = self.ctx.set_transform(
            page_scale,
            0.0,
            0.0,
            page_scale,
            -viewport_left_px,
            -viewport_top_px,
        );
    }

    fn draw_text_command(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        font_size: f32,
        color: &str,
        font_name: &str,
    ) {
        self.ctx.set_fill_style_str(color);
        self.ctx.set_font(&format!("{}px {}", font_size, font_name));
        let _ = self.ctx.fill_text(text, x as f64, y as f64);
    }

    fn draw_rect_command(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: &str,
        is_fill: bool,
    ) {
        let is_suspicious_horizontal_rect = width >= 120.0 && height <= 6.0;
        if is_suspicious_horizontal_rect {
            dbg_event(
                "canvas.draw",
                if is_fill {
                    "draw-command-fill-rect"
                } else {
                    "draw-command-stroke-rect"
                },
                vec![
                    dbg_field("x1", x),
                    dbg_field("y1", y),
                    dbg_field("width", width),
                    dbg_field("height", height),
                    dbg_field("color", color),
                ],
            );
        }
        if is_fill {
            self.ctx.set_fill_style_str(color);
            self.ctx
                .fill_rect(x as f64, y as f64, width as f64, height as f64);
        } else {
            self.ctx.set_stroke_style_str(color);
            self.ctx
                .stroke_rect(x as f64, y as f64, width as f64, height as f64);
        }
    }

    fn draw_line_command(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, color: &str, width: f32) {
        let line_width = (x2 - x1).abs();
        let line_height = (y2 - y1).abs();
        let is_suspicious_horizontal_line = line_width >= 120.0 && line_height <= 6.0;
        if is_suspicious_horizontal_line {
            dbg_event(
                "canvas.draw",
                "draw-command-line",
                vec![
                    dbg_field("x1", x1),
                    dbg_field("y1", y1),
                    dbg_field("x2", x2),
                    dbg_field("y2", y2),
                    dbg_field("strokeWidth", width),
                    dbg_field("color", color),
                ],
            );
        }
        self.ctx.begin_path();
        self.ctx.set_stroke_style_str(color);
        self.ctx.set_line_width(width as f64);
        self.ctx.move_to(x1 as f64, y1 as f64);
        self.ctx.line_to(x2 as f64, y2 as f64);
        self.ctx.stroke();
    }
}

impl PdfRenderer for CanvasRenderer {
    fn render(&mut self, commands: &[DrawCommand]) {
        for cmd in commands {
            match cmd {
                DrawCommand::Text {
                    text,
                    x,
                    y,
                    font_size,
                    color,
                    font_name,
                } => self.draw_text_command(text, *x, *y, *font_size, color, font_name),
                DrawCommand::Rect {
                    x,
                    y,
                    width,
                    height,
                    color,
                    is_fill,
                } => self.draw_rect_command(*x, *y, *width, *height, color, *is_fill),
                DrawCommand::Line {
                    x1,
                    y1,
                    x2,
                    y2,
                    color,
                    width,
                } => self.draw_line_command(*x1, *y1, *x2, *y2, color, *width),
            }
        }
    }

    fn clear(&mut self) {
        if self.is_hijacked {
            return;
        }
        let w = self.canvas.width() as f64;
        let h = self.canvas.height() as f64;

        let _ = self.ctx.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);
        self.ctx.set_fill_style_str("#ffffff");
        self.ctx.fill_rect(0.0, 0.0, w, h);
        let _ = self
            .ctx
            .set_transform(self.dpr as f64, 0.0, 0.0, self.dpr as f64, 0.0, 0.0);
    }

    fn name(&self) -> &str {
        "Canvas2D"
    }
}

pub(crate) fn draw_text_run_core(
    ctx: &CanvasRenderingContext2d,
    dpr: f32,
    text: &str,
    x: f32,
    baseline_y: f32,
    font_size: f32,
    color: &str,
    font_name: &str,
    font_weight: u16,
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
    let weight_str = font_weight.to_string();

    ctx.save();
    ctx.set_font(&format!(
        "{} {} {}px {}",
        font_style, weight_str, font_size, font_name
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
        let _ = ctx.save();
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
        let _ = ctx.restore();
    } else {
        let _ = ctx.save();
        let _ = ctx.scale(scale_x as f64, y_scale);
        if render_mode == 1 || render_mode == 2 {
            let _ = ctx.stroke_text(text, 0.0, 0.0);
        }
        if render_mode == 0 || render_mode == 2 {
            let _ = ctx.fill_text(text, 0.0, 0.0);
        }
        let _ = ctx.restore();
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
            dbg_event(
                "canvas.draw",
                "underline-stroke",
                vec![
                    dbg_field("color", color),
                    dbg_field("width", underline_width),
                    dbg_field("height", (font_size * 0.055).max(0.8)),
                    dbg_field("strokeWidth", (font_size * 0.055).max(0.8)),
                ],
            );
            ctx.begin_path();
            ctx.set_line_width((font_size * 0.055).max(0.8) as f64);
            ctx.move_to(0.0, underline_y);
            ctx.line_to(snap_to_pixel(underline_width), underline_y);
            ctx.stroke();
        }
    }

    ctx.restore();
}
