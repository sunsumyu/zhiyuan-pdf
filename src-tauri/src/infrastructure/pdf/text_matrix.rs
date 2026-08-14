//! Shared text-matrix state for the content-stream pipeline.
//!
//! Owned by both the read path (`GraphicsState` in `pdf_read::graphics_state`)
//! and the write-reflow path (`PdfTextState` in `pdf_write`). Encapsulates the
//! `ctm`/`tm`/`tlm` trio and the invariant-bearing matrix operations (`cm`,
//! `BT`, `Td`, `Tm`, text advance) so both parsers share one correct
//! implementation rather than re-deriving the matrix math independently.

use crate::infrastructure::pdf::pdf_read::utils::multiply_matrices;

const IDENTITY: [f32; 6] = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

/// The shared text-matrix core: the CTM plus the text and line matrices.
///
/// Fields are private; all mutation goes through the operator methods, which
/// preserve the invariants (e.g. `Td` updates both `tlm` and `tm`; `Tm` sets
/// both; `BT` resets both). Callers that only need to read the current
/// matrices use the `ctm()`/`tm()`/`tlm()` accessors.
#[derive(Clone, Debug)]
pub struct TextMatrixCore {
    ctm: [f32; 6],
    tm: [f32; 6],
    tlm: [f32; 6],
}

impl TextMatrixCore {
    pub fn new() -> Self {
        Self {
            ctm: IDENTITY,
            tm: IDENTITY,
            tlm: IDENTITY,
        }
    }

    /// `cm`: concatenate `m` onto the CTM (`ctm = m × ctm`).
    pub fn concat_ctm(&mut self, m: [f32; 6]) {
        self.ctm = multiply_matrices(self.ctm, m);
    }

    /// `BT`: reset the text and line matrices to the identity.
    pub fn begin_text(&mut self) {
        self.tm = IDENTITY;
        self.tlm = IDENTITY;
    }

    /// `Tm`: set both the text and line matrices to `m`.
    pub fn set_text_matrix(&mut self, m: [f32; 6]) {
        self.tm = m;
        self.tlm = m;
    }

    /// `Td`: translate the line matrix by `(tx, ty)`; the text matrix follows.
    pub fn translate_text(&mut self, tx: f32, ty: f32) {
        self.tlm = multiply_matrices(self.tlm, [1.0, 0.0, 0.0, 1.0, tx, ty]);
        self.tm = self.tlm;
    }

    /// Advance the text matrix by a horizontal displacement (post-`Tj`/`TJ`).
    pub fn advance_text(&mut self, dx: f32) {
        self.tm = multiply_matrices(self.tm, [1.0, 0.0, 0.0, 1.0, dx, 0.0]);
    }

    /// Text rendering matrix (`ctm × tm`), mapping text space to device space.
    pub fn text_render_matrix(&self) -> [f32; 6] {
        multiply_matrices(self.ctm, self.tm)
    }

    /// Transform a point by the CTM.
    pub fn transform_point(&self, x: f32, y: f32) -> [f32; 2] {
        let [a, b, c, d, e, f] = self.ctm;
        [a * x + c * y + e, b * x + d * y + f]
    }

    pub fn ctm(&self) -> [f32; 6] {
        self.ctm
    }
    pub fn tm(&self) -> [f32; 6] {
        self.tm
    }
    pub fn tlm(&self) -> [f32; 6] {
        self.tlm
    }
}

impl Default for TextMatrixCore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_identity() {
        let c = TextMatrixCore::new();
        assert_eq!(c.ctm(), IDENTITY);
        assert_eq!(c.tm(), IDENTITY);
        assert_eq!(c.tlm(), IDENTITY);
    }

    #[test]
    fn concat_ctm_identity_is_noop() {
        let mut c = TextMatrixCore::new();
        c.concat_ctm(IDENTITY);
        assert_eq!(c.ctm(), IDENTITY);
    }

    #[test]
    fn concat_ctm_with_identity_ctm_yields_operand() {
        // ctm = m × ctm; with ctm = identity, result is m.
        let mut c = TextMatrixCore::new();
        let m = [2.0, 0.0, 0.0, 3.0, 10.0, 20.0];
        c.concat_ctm(m);
        assert_eq!(c.ctm(), m);
    }

    #[test]
    fn concat_ctm_composes_in_multiply_order() {
        let mut c = TextMatrixCore::new();
        let m1 = [2.0, 0.0, 0.0, 2.0, 5.0, 5.0];
        let m2 = [1.0, 0.0, 0.0, 1.0, 10.0, 0.0];
        c.concat_ctm(m1);
        c.concat_ctm(m2);
        // concat_ctm(m) does ctm = multiply_matrices(ctm, m); two concats give
        // multiply_matrices(multiply_matrices(I, m1), m2) == multiply_matrices(m1, m2).
        assert_eq!(c.ctm(), multiply_matrices(m1, m2));
    }

    #[test]
    fn begin_text_resets_text_matrices_only() {
        let mut c = TextMatrixCore::new();
        c.concat_ctm([2.0, 0.0, 0.0, 2.0, 5.0, 5.0]);
        c.set_text_matrix([4.0, 0.0, 0.0, 4.0, 1.0, 1.0]);
        c.begin_text();
        assert_eq!(c.tm(), IDENTITY);
        assert_eq!(c.tlm(), IDENTITY);
        // BT must not touch the CTM.
        assert_eq!(c.ctm(), [2.0, 0.0, 0.0, 2.0, 5.0, 5.0]);
    }

    #[test]
    fn set_text_matrix_sets_both() {
        let mut c = TextMatrixCore::new();
        let m = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        c.set_text_matrix(m);
        assert_eq!(c.tm(), m);
        assert_eq!(c.tlm(), m);
    }

    #[test]
    fn translate_text_sets_tm_equal_to_tlm() {
        let mut c = TextMatrixCore::new();
        c.translate_text(10.0, 20.0);
        let expected = multiply_matrices(IDENTITY, [1.0, 0.0, 0.0, 1.0, 10.0, 20.0]);
        assert_eq!(c.tlm(), expected);
        assert_eq!(c.tm(), c.tlm());
    }

    #[test]
    fn translate_text_accumulates_displacement() {
        let mut c = TextMatrixCore::new();
        c.translate_text(10.0, 0.0);
        c.translate_text(5.0, 0.0);
        assert_eq!(c.tlm()[4], 15.0);
        assert_eq!(c.tm()[4], 15.0);
    }

    #[test]
    fn advance_text_moves_tm_not_tlm() {
        let mut c = TextMatrixCore::new();
        c.translate_text(10.0, 20.0);
        let tlm_before = c.tlm();
        c.advance_text(7.0);
        assert_eq!(c.tlm(), tlm_before);
        assert_eq!(c.tm()[4], 17.0); // 10 from translate + 7 advance
    }

    #[test]
    fn text_render_matrix_is_ctm_times_tm() {
        let mut c = TextMatrixCore::new();
        c.concat_ctm([2.0, 0.0, 0.0, 2.0, 0.0, 0.0]);
        c.set_text_matrix([1.0, 0.0, 0.0, 1.0, 10.0, 20.0]);
        assert_eq!(c.text_render_matrix(), multiply_matrices(c.ctm(), c.tm()));
    }

    #[test]
    fn transform_point_identity_is_identity() {
        let c = TextMatrixCore::new();
        assert_eq!(c.transform_point(3.0, 4.0), [3.0, 4.0]);
    }

    #[test]
    fn transform_point_with_scale_and_translate() {
        let mut c = TextMatrixCore::new();
        c.concat_ctm([2.0, 0.0, 0.0, 3.0, 10.0, 20.0]);
        // (x, y) -> (2x + 10, 3y + 20)
        assert_eq!(c.transform_point(1.0, 1.0), [12.0, 23.0]);
    }

    #[test]
    fn clone_is_independent() {
        let mut c = TextMatrixCore::new();
        c.concat_ctm([2.0, 0.0, 0.0, 2.0, 0.0, 0.0]); // scale by 2
        let mut d = c.clone();
        // Translating the clone by 100 composes after the scale-by-2, so the
        // device-space e component is 100 * 2 = 200.
        d.concat_ctm([1.0, 0.0, 0.0, 1.0, 100.0, 0.0]);
        assert_eq!(c.ctm()[4], 0.0); // original is untouched by the clone's mutation
        assert_eq!(d.ctm()[4], 200.0);
    }
}
