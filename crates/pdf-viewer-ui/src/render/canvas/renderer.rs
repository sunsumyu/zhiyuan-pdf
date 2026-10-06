//! Editor overlay text drawing on the shared CanvasRenderer (`draw_text_run`).

use super::draw::draw_text_run_core;
use super::CanvasRenderer;
use super::CoordinateMode;

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
}
