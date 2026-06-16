use crate::infrastructure::pdf::pdf_font::ParsedFont;
use std::sync::Arc;
#[derive(Clone, Debug)]
pub struct GraphicsState {
    pub ctm: [f32; 6],
    pub line_width: f32,
    pub line_cap: u8,
    pub line_join: u8,
    pub miter_limit: f32,
    pub stroke_color: Option<String>,
    pub fill_color: Option<String>,
    pub fill_alpha: f32,
    pub stroke_alpha: f32,
    pub font_size: f32,
    pub current_font: Option<Arc<ParsedFont>>,
    pub tm: [f32; 6],
    pub tlm: [f32; 6],
    pub tl: f32,
    pub char_spacing: f32,
    pub word_spacing: f32,
    pub horizontal_scaling: f32,
    pub text_rise: f32,
    pub render_mode: i64,
}

impl GraphicsState {
    pub fn new() -> Self {
        Self {
            ctm: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            line_width: 1.0,
            line_cap: 0,
            line_join: 0,
            miter_limit: 10.0,
            stroke_color: None,
            fill_color: None,
            fill_alpha: 1.0,
            stroke_alpha: 1.0,
            font_size: 12.0,
            current_font: None,
            tm: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            tlm: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            tl: 0.0,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            text_rise: 0.0,
            render_mode: 0,
        }
    }

    pub fn transform_point(&self, x: f32, y: f32) -> [f32; 2] {
        let (a, b, c, d, e, f) = (
            self.ctm[0],
            self.ctm[1],
            self.ctm[2],
            self.ctm[3],
            self.ctm[4],
            self.ctm[5],
        );
        [a * x + c * y + e, b * x + d * y + f]
    }
}

