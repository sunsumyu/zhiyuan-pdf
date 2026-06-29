use super::{preview_text, should_trace_text_render, text_is_non_painting, VelloRenderer};
use crate::infrastructure::pdf::models::NativeTextModel;
use crate::infrastructure::pdf::path_utils;
use pdf_viewer_core::typography::models::ResolvedPdfFont;
use swash::proxy::MetricsProxy;
use swash::scale::ScaleContext;
use vello::kurbo::Affine;
use vello::Scene;

impl VelloRenderer {
    pub(super) fn resolve_pdf_font(&mut self, text: &NativeTextModel) -> ResolvedPdfFont {
        self.font_matcher.resolve_native_text(text)
    }

    pub(super) fn draw_embedded_text_vector(
        &mut self,
        scene: &mut Scene,
        scale_context: &mut ScaleContext,
        text: &NativeTextModel,
        resolved_font: &ResolvedPdfFont,
        flip_y: Affine,
    ) -> bool {
        if text_is_non_painting(text.rendering_mode) {
            return true;
        }
        if !resolved_font.can_attempt_embedded_render {
            if should_trace_text_render(text) {
                println!(
                    "[PDF-EMBEDDED] skip can_attempt=false text='{}' font='{}' key={:?} subtype={:?}",
                    preview_text(&text.text),
                    text.font_name,
                    text.embedded_font_key,
                    text.font_subtype
                );
            }
            return false;
        }

        let Some(font_key) = text.embedded_font_key.as_deref() else {
            println!(
                "[PDF-EMBEDDED] skip missing-key text='{}' font='{}' subtype={:?}",
                preview_text(&text.text),
                text.font_name,
                text.font_subtype
            );
            return false;
        };
        let font_bytes = {
            let cache = crate::infrastructure::pdf::cache::PDF_FONT_PROGRAM_CACHE
                .lock()
                .unwrap();
            cache.get(font_key).cloned()
        };
        let Some(font_bytes) = font_bytes else {
            println!(
                "[PDF-EMBEDDED] skip missing-cache-entry key='{}' text='{}' font='{}'",
                font_key,
                preview_text(&text.text),
                text.font_name
            );
            return false;
        };

        let Some(font_ref) = swash::FontRef::from_index(font_bytes.as_slice(), 0) else {
            println!(
                "[PDF-EMBEDDED] skip invalid-font-ref key='{}' text='{}' font='{}'",
                font_key,
                preview_text(&text.text),
                text.font_name
            );
            return false;
        };
        let units_per_em = MetricsProxy::from_font(&font_ref).units_per_em() as f64;
        if units_per_em <= 0.0 {
            println!(
                "[PDF-EMBEDDED] skip invalid-upem key='{}' text='{}' font='{}'",
                font_key,
                preview_text(&text.text),
                text.font_name
            );
            return false;
        }

        let glyph_positions = self.build_embedded_glyph_positions(text);
        if glyph_positions.is_empty() {
            println!(
                "[PDF-EMBEDDED] skip no-glyph-positions text='{}' font='{}' char_origins={} char_widths={} codes={}",
                preview_text(&text.text),
                text.font_name,
                text.char_origins.len(),
                text.char_widths.len(),
                text.pdf_char_codes.len()
            );
            return false;
        }

        let real_font_size = if text.scale_y.abs() > 1.0 {
            text.scale_y.abs()
        } else {
            text.font_size
        };
        let mut scaler = scale_context.builder(font_ref).hint(false).build();

        let mut drew_any_glyph = false;
        for (index, (baseline_x, baseline_y)) in glyph_positions.into_iter().enumerate() {
            let glyph_id = self.resolve_embedded_glyph_id(text, &font_ref, index);
            if glyph_id == 0 {
                continue;
            }
            let Some(outline) = scaler.scale_outline(glyph_id) else {
                continue;
            };

            let bez_path = path_utils::outline_to_bez_path(&outline);

            let final_transform = self.raw_outline_transform(
                flip_y,
                baseline_x,
                baseline_y,
                real_font_size,
                units_per_em,
            );

            if self.paint_text_outline(scene, bez_path, final_transform, text) {
                drew_any_glyph = true;
            }
        }

        if !drew_any_glyph {
            println!(
                "[PDF-EMBEDDED] skip no-outlines text='{}' font='{}' key='{}' subtype={:?} codes={:?}",
                preview_text(&text.text),
                text.font_name,
                font_key,
                text.font_subtype,
                text.pdf_char_codes
            );
        } else if should_trace_text_render(text) {
            println!(
                "[PDF-EMBEDDED] success text='{}' font='{}' key='{}' subtype={:?} codes={:?}",
                preview_text(&text.text),
                text.font_name,
                font_key,
                text.font_subtype,
                text.pdf_char_codes
            );
        }

        drew_any_glyph
    }

    pub(super) fn build_embedded_glyph_positions(&self, text: &NativeTextModel) -> Vec<(f32, f32)> {
        let glyph_count = self.embedded_glyph_count(text);
        if glyph_count == 0 {
            return Vec::new();
        }

        if text.char_origins.len() == glyph_count {
            return text
                .char_origins
                .iter()
                .map(|origin| (origin[0], origin[1]))
                .collect();
        }

        if text.char_widths.len() == glyph_count {
            let mut positions = Vec::with_capacity(glyph_count);
            let mut current_x = text.tx;
            for width in &text.char_widths {
                positions.push((current_x, text.ty));
                current_x += *width;
            }
            return positions;
        }

        if glyph_count == 1 {
            return vec![(text.tx, text.ty)];
        }

        Vec::new()
    }

    pub(super) fn embedded_glyph_count(&self, text: &NativeTextModel) -> usize {
        if !text.pdf_char_codes.is_empty() {
            return text.pdf_char_codes.len();
        }
        text.text.chars().count()
    }

    pub(super) fn resolve_embedded_glyph_id(
        &self,
        text: &NativeTextModel,
        font_ref: &swash::FontRef<'_>,
        glyph_index: usize,
    ) -> u16 {
        let ch_for_log = text.text.chars().nth(glyph_index);
        let raw_code_for_log = text.pdf_char_codes.get(glyph_index).copied();
        let is_suspect = ch_for_log.map(|c| c as u32 > 0x7F).unwrap_or(false);

        if let Some(raw_code) = text.pdf_char_codes.get(glyph_index).copied() {
            if let Some(mapped) = self.resolve_cached_cid_glyph_id(text, raw_code) {
                if is_suspect {
                    crate::pdf_log!(
                        3,
                        "[GLYPH-RESOLVE] font='{}' idx={} raw=0x{:04X} ch={:?}(U+{:04X}) -> CID_MAP gid={}",
                        text.font_name, glyph_index, raw_code,
                        ch_for_log, ch_for_log.map(|c| c as u32).unwrap_or(0), mapped
                    );
                }
                return mapped;
            }
            let charmap_gid = font_ref.charmap().map(raw_code);
            if charmap_gid != 0 {
                if is_suspect {
                    crate::pdf_log!(
                        3,
                        "[GLYPH-RESOLVE] font='{}' idx={} raw=0x{:04X} ch={:?}(U+{:04X}) -> RAW_CHARMAP gid={}",
                        text.font_name, glyph_index, raw_code,
                        ch_for_log, ch_for_log.map(|c| c as u32).unwrap_or(0), charmap_gid
                    );
                }
                return charmap_gid;
            }
        }

        if let Some(ch) = text.text.chars().nth(glyph_index) {
            if !ch.is_control() && !ch.is_whitespace() {
                let glyph_id = font_ref.charmap().map(ch);
                if glyph_id != 0 {
                    if is_suspect {
                        crate::pdf_log!(
                            3,
                            "[GLYPH-RESOLVE] font='{}' idx={} raw={:?} ch={:?}(U+{:04X}) -> UNICODE_CHARMAP gid={}",
                            text.font_name, glyph_index, raw_code_for_log, ch, ch as u32, glyph_id
                        );
                    }
                    return glyph_id;
                }
            }
        }

        if let Some(raw_code) = text.pdf_char_codes.get(glyph_index).copied() {
            if self.prefers_pdf_code_glyph_mapping(text)
                && raw_code > 0
                && raw_code <= u16::MAX as u32
            {
                if is_suspect {
                    crate::pdf_log!(
                        3,
                        "[GLYPH-RESOLVE] font='{}' idx={} raw=0x{:04X} ch={:?}(U+{:04X}) -> DIRECT_CODE gid={}",
                        text.font_name, glyph_index, raw_code,
                        ch_for_log, ch_for_log.map(|c| c as u32).unwrap_or(0), raw_code as u16
                    );
                }
                return raw_code as u16;
            }
        }

        if is_suspect {
            crate::pdf_log!(
                3,
                "[GLYPH-RESOLVE] font='{}' idx={} raw={:?} ch={:?}(U+{:04X}) -> FAILED gid=0 (will skip or fallback to cosmic)",
                text.font_name, glyph_index, raw_code_for_log,
                ch_for_log, ch_for_log.map(|c| c as u32).unwrap_or(0)
            );
        }
        0
    }

    pub(super) fn resolve_cached_cid_glyph_id(
        &self,
        text: &NativeTextModel,
        raw_code: u32,
    ) -> Option<u16> {
        let font_key = text.embedded_font_key.as_deref()?;
        let cache = crate::infrastructure::pdf::cache::PDF_FONT_GLYPH_MAP_CACHE
            .lock()
            .ok()?;
        let glyph_map = cache.get(font_key)?;

        if let Some(gid) = glyph_map.cid_to_gid.get(&raw_code).copied() {
            return Some(gid);
        }
        if glyph_map.identity && raw_code > 0 && raw_code <= u16::MAX as u32 {
            return Some(raw_code as u16);
        }

        None
    }

    pub(super) fn prefers_pdf_code_glyph_mapping(&self, text: &NativeTextModel) -> bool {
        let Some(subtype) = text.font_subtype.as_deref() else {
            return false;
        };
        let lower = subtype.trim().trim_start_matches('/').to_ascii_lowercase();
        matches!(lower.as_str(), "truetype" | "opentype" | "type1")
    }

    pub(super) fn resolve_cosmic_family<'a>(
        &self,
        text: &NativeTextModel,
        resolved_font: &'a ResolvedPdfFont,
    ) -> cosmic_text::Family<'a> {
        if let Some(matched_family) = resolved_font.matched_family.as_deref() {
            return cosmic_text::Family::Name(matched_family);
        }

        if text
            .font_hints
            .as_ref()
            .map(|value| value.is_serif)
            .unwrap_or(false)
            || text.font_name.to_ascii_lowercase().contains("serif")
            || text.font_name.to_ascii_lowercase().contains("roman")
        {
            return cosmic_text::Family::Serif;
        }

        cosmic_text::Family::SansSerif
    }
}
