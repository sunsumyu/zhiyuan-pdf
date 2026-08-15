use crate::infrastructure::pdf::pdf_font::ParsedFont;
use crate::infrastructure::pdf::text_state::TextState;
use std::sync::Arc;

/// Graphics state for the read-path content-stream parser.
///
/// Embeds the shared [`TextState`] for text-state fields (font size, spacing,
/// horizontal scaling, render mode, leading) and matrix operations. The
/// remaining fields carry read-path-only state (color, font, line properties,
/// transparency) with no cross-field invariants, so they stay public and are
/// mutated inline by the operator dispatch.
#[derive(Clone, Debug)]
pub struct GraphicsState {
    /// Shared text state: matrix trio + text-state parameters.
    pub text: TextState,
    pub line_width: f32,
    pub line_cap: u8,
    pub line_join: u8,
    pub miter_limit: f32,
    pub stroke_color: Option<String>,
    pub fill_color: Option<String>,
    pub fill_alpha: f32,
    pub stroke_alpha: f32,
    pub current_font: Option<Arc<ParsedFont>>,
    pub text_rise: f32,
}

impl GraphicsState {
    pub fn new() -> Self {
        Self {
            text: TextState::default(),
            line_width: 1.0,
            line_cap: 0,
            line_join: 0,
            miter_limit: 10.0,
            stroke_color: None,
            fill_color: None,
            fill_alpha: 1.0,
            stroke_alpha: 1.0,
            current_font: None,
            text_rise: 0.0,
        }
    }
}
