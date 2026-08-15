//! Shared text-state fields and operator semantics for the content-stream pipeline.
//!
//! Owned by both the read path ([`GraphicsState`][crate::infrastructure::pdf::pdf_read::graphics_state::GraphicsState])
//! and the write-reflow path (`PdfTextState` in `pdf_write`). Consolidates the
//! text-state fields that both paths track identically (font size, character/word
//! spacing, horizontal scaling, render mode, text leading) plus the matrix
//! operations delegated to [`TextMatrixCore`].
//!
//! The `Tf` operator is intentionally **not** a method here — the read path
//! resolves `ParsedFont` while the write path resolves `font_alias` bytes, so
//! each path handles `Tf` in its own dispatch.

use super::text_matrix::TextMatrixCore;

/// Shared PDF text state: the matrix trio plus the text-state parameters
/// that both the read and write paths track identically.
///
/// Fields are `pub(crate)` — no cross-field invariants, matching the
/// codebase convention for state structs.
#[derive(Clone, Debug)]
pub struct TextState {
    /// The matrix trio (`ctm`/`tm`/`tlm`) and invariant-bearing operations.
    pub core: TextMatrixCore,
    /// Current font size (set by `Tf`).
    pub font_size: f32,
    /// Character spacing (set by `Tc`).
    pub char_spacing: f32,
    /// Word spacing (set by `Tw`).
    pub word_spacing: f32,
    /// Horizontal scaling percentage (set by `Tz`). 100 = normal.
    pub horizontal_scaling: f32,
    /// Text render mode (set by `Tr`). 0=fill, 1=stroke, 2=fill+stroke, 3=invisible.
    pub render_mode: i32,
    /// Text leading — vertical distance between baselines (set by `TL`/`TD`, consumed by `T*`).
    pub tl: f32,
}

impl Default for TextState {
    fn default() -> Self {
        Self {
            core: TextMatrixCore::default(),
            font_size: 12.0,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            render_mode: 0,
            tl: 0.0,
        }
    }
}

impl TextState {
    // ── Stateful operator methods ────────────────────────────────────────
    // These have side effects beyond a simple field write and earn their
    // place as named methods.

    /// `cm`: concatenate `m` onto the CTM.
    pub fn op_cm(&mut self, m: [f32; 6]) {
        self.core.concat_ctm(m);
    }

    /// `BT`: reset the text and line matrices to identity.
    pub fn op_bt(&mut self) {
        self.core.begin_text();
    }

    /// `Tm`: set both the text and line matrices to `m`.
    pub fn op_tm(&mut self, m: [f32; 6]) {
        self.core.set_text_matrix(m);
    }

    /// `Td tx ty`: translate the line matrix by `(tx, ty)`; text matrix follows.
    pub fn op_td(&mut self, tx: f32, ty: f32) {
        self.core.translate_text(tx, ty);
    }

    /// `TD tx ty`: set leading to `-ty`, then translate by `(tx, ty)`.
    ///
    /// Per the PDF spec: `TD` is equivalent to `−ty TL` followed by `Td tx ty`.
    pub fn op_td_with_leading(&mut self, tx: f32, ty: f32) {
        self.tl = -ty;
        self.core.translate_text(tx, ty);
    }

    /// `T*`: move to the next line by the current text leading.
    pub fn op_t_star(&mut self) {
        self.core.translate_text(0.0, -self.tl);
    }

    // ── Pass-through accessors ──────────────────────────────────────────
    // These have real semantics ("get the rendering matrix", "transform a
    // point") and are called from both paths.

    /// Text rendering matrix (`ctm × tm`), mapping text space to device space.
    pub fn text_render_matrix(&self) -> [f32; 6] {
        self.core.text_render_matrix()
    }

    /// Transform a point by the CTM.
    pub fn transform_point(&self, x: f32, y: f32) -> [f32; 2] {
        self.core.transform_point(x, y)
    }

    /// Current CTM.
    pub fn ctm(&self) -> [f32; 6] {
        self.core.ctm()
    }

    /// Current text matrix.
    pub fn tm(&self) -> [f32; 6] {
        self.core.tm()
    }

    /// Current line matrix.
    pub fn tlm(&self) -> [f32; 6] {
        self.core.tlm()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_values() {
        let s = TextState::default();
        assert_eq!(s.font_size, 12.0);
        assert_eq!(s.char_spacing, 0.0);
        assert_eq!(s.word_spacing, 0.0);
        assert_eq!(s.horizontal_scaling, 100.0);
        assert_eq!(s.render_mode, 0);
        assert_eq!(s.tl, 0.0);
        assert_eq!(s.tm(), [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
    }

    #[test]
    fn op_td_updates_tlm_and_tm() {
        let mut s = TextState::default();
        s.op_td(10.0, -20.0);
        let tm = s.tm();
        let tlm = s.tlm();
        assert_eq!(tm[4], 10.0);
        assert_eq!(tm[5], -20.0);
        assert_eq!(tlm, tm);
    }

    #[test]
    fn op_td_with_leading_sets_tl_and_translates() {
        let mut s = TextState::default();
        s.op_td_with_leading(5.0, -14.0);
        assert_eq!(s.tl, 14.0); // -(-14) = 14
        let tm = s.tm();
        assert_eq!(tm[4], 5.0);
        assert_eq!(tm[5], -14.0);
    }

    #[test]
    fn op_t_star_uses_tl() {
        let mut s = TextState::default();
        s.tl = 14.0;
        s.op_t_star();
        // T* translates by (0, -tl) = (0, -14)
        assert_eq!(s.tm()[5], -14.0);
        assert_eq!(s.tlm()[5], -14.0);
    }

    #[test]
    fn op_t_star_with_zero_leading() {
        let mut s = TextState::default();
        s.op_t_star();
        assert_eq!(s.tm(), [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
    }

    #[test]
    fn op_cm_modifies_ctm() {
        let mut s = TextState::default();
        s.op_cm([2.0, 0.0, 0.0, 3.0, 10.0, 20.0]);
        assert_eq!(s.ctm(), [2.0, 0.0, 0.0, 3.0, 10.0, 20.0]);
    }

    #[test]
    fn op_bt_resets_tm_and_tlm() {
        let mut s = TextState::default();
        s.op_td(10.0, 20.0);
        s.op_bt();
        assert_eq!(s.tm(), [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
        assert_eq!(s.tlm(), [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
    }

    #[test]
    fn op_tm_sets_both_matrices() {
        let mut s = TextState::default();
        let m = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        s.op_tm(m);
        assert_eq!(s.tm(), m);
        assert_eq!(s.tlm(), m);
    }

    #[test]
    fn text_render_matrix_delegates() {
        let mut s = TextState::default();
        s.op_cm([2.0, 0.0, 0.0, 2.0, 0.0, 0.0]);
        s.op_tm([1.0, 0.0, 0.0, 1.0, 10.0, 20.0]);
        let trm = s.text_render_matrix();
        assert_eq!(trm[0], 2.0); // a scaled by 2
        assert_eq!(trm[4], 20.0); // e = 2*10
    }

    #[test]
    fn transform_point_delegates() {
        let mut s = TextState::default();
        s.op_cm([2.0, 0.0, 0.0, 3.0, 10.0, 20.0]);
        assert_eq!(s.transform_point(1.0, 1.0), [12.0, 23.0]);
    }

    #[test]
    fn clone_is_independent() {
        let mut s = TextState::default();
        s.font_size = 16.0;
        s.tl = 20.0;
        let mut d = s.clone();
        d.font_size = 8.0;
        d.tl = 10.0;
        assert_eq!(s.font_size, 16.0);
        assert_eq!(s.tl, 20.0);
        assert_eq!(d.font_size, 8.0);
        assert_eq!(d.tl, 10.0);
    }

    #[test]
    fn field_direct_access() {
        let mut s = TextState::default();
        s.char_spacing = 1.5;
        s.word_spacing = 2.0;
        s.horizontal_scaling = 80.0;
        s.render_mode = 2;
        assert_eq!(s.char_spacing, 1.5);
        assert_eq!(s.word_spacing, 2.0);
        assert_eq!(s.horizontal_scaling, 80.0);
        assert_eq!(s.render_mode, 2);
    }
}
