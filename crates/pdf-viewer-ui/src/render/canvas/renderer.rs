//! impl PdfRenderer for CanvasRenderer — primitive draw commands (text, rect, line)
//! and the PdfRenderer trait implementation.

use super::draw::draw_text_run_core;
use super::CanvasRenderer;
use super::CoordinateMode;
use crate::editor::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use pdf_viewer_core::render::renderer::{DrawCommand, PdfRenderer};

impl CanvasRenderer {
    #[allow(clippy::too_many_arguments)]
    pub fn draw_text_run(
        &self,
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
        let _current_canvas_height = self.canvas_height.get();

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
