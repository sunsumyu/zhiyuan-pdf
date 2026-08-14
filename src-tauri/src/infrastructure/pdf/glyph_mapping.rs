//! Pure glyph-mapping logic for the embedded-font vector text pipeline.
//!
//! These functions lived in `vello_renderer` as `&self` methods that never
//! touched renderer state; extracting them makes the run-to-glyph decisions
//! unit-testable without a GPU or a font binary. Given a [`NativeTextModel`],
//! this module answers: how many glyphs the run has, where each sits on the
//! baseline, and which glyph id each resolves to.

use crate::infrastructure::pdf::models::NativeTextModel;

/// Effective font size in PDF units: prefer the text matrix's Y scale when it
/// exceeds 1.0, else the declared font size.
pub fn real_font_size(text: &NativeTextModel) -> f32 {
    if text.scale_y.abs() > 1.0 {
        text.scale_y.abs()
    } else {
        text.font_size
    }
}

/// Number of glyphs in the run: raw PDF char codes when present, else Unicode
/// characters.
pub fn glyph_count(text: &NativeTextModel) -> usize {
    if !text.pdf_char_codes.is_empty() {
        text.pdf_char_codes.len()
    } else {
        text.text.chars().count()
    }
}

/// Baseline origin `(x, y)` per glyph, in PDF user space.
///
/// Priority: explicit char origins when the count matches, else cumulative
/// advance from `tx` using char widths, else a single glyph at the run origin,
/// else empty (the caller falls back to another renderer).
pub fn build_glyph_positions(text: &NativeTextModel) -> Vec<(f32, f32)> {
    let count = glyph_count(text);
    if count == 0 {
        return Vec::new();
    }

    if text.char_origins.len() == count {
        return text
            .char_origins
            .iter()
            .map(|origin| (origin[0], origin[1]))
            .collect();
    }

    if text.char_widths.len() == count {
        let mut positions = Vec::with_capacity(count);
        let mut current_x = text.tx;
        for width in &text.char_widths {
            positions.push((current_x, text.ty));
            current_x += *width;
        }
        return positions;
    }

    if count == 1 {
        return vec![(text.tx, text.ty)];
    }

    Vec::new()
}

/// Whether the font subtype suggests PDF char codes are directly usable as
/// glyph ids (simple TrueType/OpenType/Type1 fonts).
pub fn prefers_pdf_code_glyph_mapping(text: &NativeTextModel) -> bool {
    let Some(subtype) = text.font_subtype.as_deref() else {
        return false;
    };
    let lower = subtype.trim().trim_start_matches('/').to_ascii_lowercase();
    matches!(lower.as_str(), "truetype" | "opentype" | "type1")
}

/// Resolve the glyph id for `glyph_index`, in priority order:
///
/// 1. Raw PDF char code mapped through the font's charmap.
/// 2. Unicode character mapped through the charmap (non-control, non-space).
/// 3. The raw code used directly as a glyph id, for simple font subtypes.
/// 4. `0` (notdef - the caller skips the glyph or falls back).
///
/// `map_charcode` abstracts `font_ref.charmap().map(code)`, keeping this pure
/// and testable without a font binary.
pub fn resolve_glyph_id(
    text: &NativeTextModel,
    glyph_index: usize,
    map_charcode: impl Fn(u32) -> u16,
) -> u16 {
    let ch_for_log = text.text.chars().nth(glyph_index);
    let raw_code_for_log = text.pdf_char_codes.get(glyph_index).copied();
    let is_suspect = ch_for_log.map(|c| c as u32 > 0x7F).unwrap_or(false);

    if let Some(raw_code) = text.pdf_char_codes.get(glyph_index).copied() {
        let mapped = map_charcode(raw_code);
        if mapped != 0 {
            if is_suspect {
                crate::pdf_log!(
                    3,
                    "[GLYPH-RESOLVE] font='{}' idx={} raw=0x{:04X} ch={:?}(U+{:04X}) -> RAW_CHARMAP gid={}",
                    text.font_name,
                    glyph_index,
                    raw_code,
                    ch_for_log,
                    ch_for_log.map(|c| c as u32).unwrap_or(0),
                    mapped
                );
            }
            return mapped;
        }
    }

    if let Some(ch) = text.text.chars().nth(glyph_index) {
        if !ch.is_control() && !ch.is_whitespace() {
            let glyph_id = map_charcode(ch as u32);
            if glyph_id != 0 {
                if is_suspect {
                    crate::pdf_log!(
                        3,
                        "[GLYPH-RESOLVE] font='{}' idx={} raw={:?} ch={:?}(U+{:04X}) -> UNICODE_CHARMAP gid={}",
                        text.font_name,
                        glyph_index,
                        raw_code_for_log,
                        ch,
                        ch as u32,
                        glyph_id
                    );
                }
                return glyph_id;
            }
        }
    }

    if let Some(raw_code) = text.pdf_char_codes.get(glyph_index).copied() {
        if prefers_pdf_code_glyph_mapping(text)
            && raw_code > 0
            && raw_code <= u16::MAX as u32
        {
            if is_suspect {
                crate::pdf_log!(
                    3,
                    "[GLYPH-RESOLVE] font='{}' idx={} raw=0x{:04X} ch={:?}(U+{:04X}) -> DIRECT_CODE gid={}",
                    text.font_name,
                    glyph_index,
                    raw_code,
                    ch_for_log,
                    ch_for_log.map(|c| c as u32).unwrap_or(0),
                    raw_code as u16
                );
            }
            return raw_code as u16;
        }
    }

    if is_suspect {
        crate::pdf_log!(
            3,
            "[GLYPH-RESOLVE] font='{}' idx={} raw={:?} ch={:?}(U+{:04X}) -> FAILED gid=0 (will skip or fallback to cosmic)",
            text.font_name,
            glyph_index,
            raw_code_for_log,
            ch_for_log,
            ch_for_log.map(|c| c as u32).unwrap_or(0)
        );
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_with(text: &str, codes: &[u32]) -> NativeTextModel {
        NativeTextModel {
            text: text.to_string(),
            pdf_char_codes: codes.to_vec(),
            ..Default::default()
        }
    }

    // -- real_font_size ------------------------------------------------------

    #[test]
    fn real_font_size_prefers_large_scale_y() {
        let mut t = NativeTextModel::default();
        t.scale_y = 8.0;
        t.font_size = 12.0;
        assert_eq!(real_font_size(&t), 8.0);
    }

    #[test]
    fn real_font_size_uses_font_size_for_small_scale() {
        let mut t = NativeTextModel::default();
        t.scale_y = 0.5;
        t.font_size = 12.0;
        assert_eq!(real_font_size(&t), 12.0);
    }

    #[test]
    fn real_font_size_takes_absolute_negative_scale() {
        let mut t = NativeTextModel::default();
        t.scale_y = -6.0;
        t.font_size = 12.0;
        assert_eq!(real_font_size(&t), 6.0);
    }

    // -- glyph_count ---------------------------------------------------------

    #[test]
    fn glyph_count_prefers_pdf_codes_when_present() {
        let t = text_with("ABC", &[0x41, 0x42]);
        assert_eq!(glyph_count(&t), 2);
    }

    #[test]
    fn glyph_count_falls_back_to_chars() {
        let t = text_with("A文b", &[]);
        assert_eq!(glyph_count(&t), 3);
    }

    // -- build_glyph_positions -------------------------------------------------

    #[test]
    fn positions_use_char_origins_when_counts_match() {
        let mut t = text_with("AB", &[1, 2]);
        t.char_origins = vec![[1.0, 2.0], [3.0, 4.0]];
        assert_eq!(build_glyph_positions(&t), vec![(1.0, 2.0), (3.0, 4.0)]);
    }

    #[test]
    fn positions_accumulate_widths_from_tx() {
        let mut t = text_with("AB", &[1, 2]);
        t.tx = 5.0;
        t.ty = 7.0;
        t.char_widths = vec![10.0, 20.0];
        assert_eq!(build_glyph_positions(&t), vec![(5.0, 7.0), (15.0, 7.0)]);
    }

    #[test]
    fn positions_single_glyph_defaults_to_run_origin() {
        let mut t = text_with("A", &[]);
        t.tx = 3.0;
        t.ty = 4.0;
        assert_eq!(build_glyph_positions(&t), vec![(3.0, 4.0)]);
    }

    #[test]
    fn positions_mismatched_metadata_yields_empty() {
        let t = text_with("ABC", &[]);
        assert!(build_glyph_positions(&t).is_empty());
    }

    #[test]
    fn positions_empty_run_yields_empty() {
        let t = text_with("", &[]);
        assert!(build_glyph_positions(&t).is_empty());
    }

    // -- prefers_pdf_code_glyph_mapping ----------------------------------------

    #[test]
    fn prefers_direct_codes_for_simple_subtypes() {
        for subtype in ["TrueType", "/OpenType", "TYPE1", " type1 "] {
            let mut t = NativeTextModel::default();
            t.font_subtype = Some(subtype.to_string());
            assert!(prefers_pdf_code_glyph_mapping(&t), "subtype {subtype}");
        }
    }

    #[test]
    fn rejects_composite_subtypes_and_missing_subtype() {
        for subtype in [Some("CIDFontType2"), Some("Type3"), None] {
            let mut t = NativeTextModel::default();
            t.font_subtype = subtype.map(|s| s.to_string());
            assert!(!prefers_pdf_code_glyph_mapping(&t));
        }
    }

    // -- resolve_glyph_id ------------------------------------------------------

    /// Charmap stub: maps 'A' (0x41) and U+4E2D, nothing else.
    fn stub_charmap(code: u32) -> u16 {
        match code {
            0x41 | 0x4E2D => 7,
            _ => 0,
        }
    }

    #[test]
    fn glyph_id_prefers_raw_code_through_charmap() {
        let t = text_with("A", &[0x41]);
        assert_eq!(resolve_glyph_id(&t, 0, stub_charmap), 7);
    }

    #[test]
    fn glyph_id_falls_back_to_unicode_charmap() {
        // Raw code 0xE000 is unmapped in the stub; the char 'A' maps.
        let t = text_with("A", &[0xE000]);
        assert_eq!(resolve_glyph_id(&t, 0, stub_charmap), 7);
    }

    #[test]
    fn glyph_id_skips_control_and_whitespace_chars() {
        // '\t' is a control char: stage 2 must not consult the charmap, and
        // without a simple-font subtype stage 3 cannot fire either.
        let t = text_with("\t", &[0xE000]);
        assert_eq!(resolve_glyph_id(&t, 0, stub_charmap), 0);
    }

    #[test]
    fn glyph_id_uses_direct_code_for_simple_fonts() {
        let mut t = text_with("?", &[35]);
        t.font_subtype = Some("TrueType".to_string());
        assert_eq!(resolve_glyph_id(&t, 0, stub_charmap), 35);
    }

    #[test]
    fn glyph_id_rejects_zero_direct_code() {
        let mut t = text_with("?", &[0]);
        t.font_subtype = Some("TrueType".to_string());
        assert_eq!(resolve_glyph_id(&t, 0, stub_charmap), 0);
    }

    #[test]
    fn glyph_id_returns_zero_when_all_stages_fail() {
        let t = text_with(" ", &[]);
        assert_eq!(resolve_glyph_id(&t, 0, stub_charmap), 0);
    }
}
