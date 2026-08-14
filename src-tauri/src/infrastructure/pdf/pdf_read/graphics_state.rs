use crate::infrastructure::pdf::pdf_font::ParsedFont;
use crate::infrastructure::pdf::text_matrix::TextMatrixCore;
use std::sync::Arc;

/// Graphics state for the read-path content-stream parser.
///
/// The text-matrix trio (`ctm`/`tm`/`tlm`) and its invariant-bearing operations
/// live in the shared [`TextMatrixCore`], delegated through pass-through
/// methods. The remaining fields carry read-path-only state (color, font, line
/// properties, text parameters) with no cross-field invariants, so they stay
/// public and are mutated inline by the operator dispatch.
#[derive(Clone, Debug)]
pub struct GraphicsState {
    pub core: TextMatrixCore,
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
            core: TextMatrixCore::new(),
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
            tl: 0.0,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            text_rise: 0.0,
            render_mode: 0,
        }
    }

    // -- text-matrix operations (delegate to the shared core) ----------------

    /// `cm`: concatenate `m` onto the CTM.
    pub fn concat_ctm(&mut self, m: [f32; 6]) {
        self.core.concat_ctm(m);
    }

    /// `BT`: reset the text and line matrices.
    pub fn begin_text(&mut self) {
        self.core.begin_text();
    }

    /// `Tm`: set the text and line matrices.
    pub fn set_text_matrix(&mut self, m: [f32; 6]) {
        self.core.set_text_matrix(m);
    }

    /// `Td`: translate the line matrix; the text matrix follows.
    pub fn translate_text(&mut self, tx: f32, ty: f32) {
        self.core.translate_text(tx, ty);
    }

    /// Advance the text matrix by a horizontal displacement (post-`Tj`/`TJ`).
    pub fn advance_text(&mut self, dx: f32) {
        self.core.advance_text(dx);
    }

    /// Text rendering matrix (`ctm × tm`).
    pub fn text_render_matrix(&self) -> [f32; 6] {
        self.core.text_render_matrix()
    }

    /// Transform a point by the CTM.
    pub fn transform_point(&self, x: f32, y: f32) -> [f32; 2] {
        self.core.transform_point(x, y)
    }

    pub fn ctm(&self) -> [f32; 6] {
        self.core.ctm()
    }
    pub fn tm(&self) -> [f32; 6] {
        self.core.tm()
    }
    pub fn tlm(&self) -> [f32; 6] {
        self.core.tlm()
    }
}
