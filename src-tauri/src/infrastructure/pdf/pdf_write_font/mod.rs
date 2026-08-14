mod embed;
mod face;
mod finder;

use crate::infrastructure::pdf::pdf_font::ParsedFont;
use lopdf::Document;
use std::sync::Arc;

#[derive(Clone)]
pub struct PdfTextWriteFont {
    pub font_alias: Vec<u8>,
    pub parsed_font: ParsedFont,
    encoding: PdfTextWriteEncoding,
    source_label: String,
}

#[derive(Clone)]
enum PdfTextWriteEncoding {
    OriginalPdfFont,
    TrueTypeGlyphIds {
        font_bytes: Arc<Vec<u8>>,
        face_index: u32,
    },
}
impl PdfTextWriteFont {
    pub fn encode_text(&self, text: &str) -> Result<Vec<u8>, String> {
        match &self.encoding {
            PdfTextWriteEncoding::OriginalPdfFont => Ok(self.parsed_font.encode_text(text)),
            PdfTextWriteEncoding::TrueTypeGlyphIds {
                font_bytes,
                face_index,
            } => face::encode_text_as_glyph_ids(font_bytes, *face_index, text),
        }
    }
    pub fn source_label(&self) -> &str {
        &self.source_label
    }
}

/// A font found in the system (or a known font file), selected for embedding
/// when the PDF's own font cannot encode the text. Carries the full font
/// binary plus the metrics and glyph subset derived from it for embedding.
pub(crate) struct SystemFont {
    pub(crate) family_name: String,
    pub(crate) post_script_name: String,
    pub(crate) source_label: String,
    pub(crate) font_bytes: Arc<Vec<u8>>,
    pub(crate) face_index: u32,
    pub(crate) glyphs: Vec<(char, u16, f32)>,
    pub(crate) bbox: [f32; 4],
    pub(crate) ascent: f32,
    pub(crate) descent: f32,
    pub(crate) cap_height: f32,
    pub(crate) italic_angle: f32,
    pub(crate) weight: i32,
}

pub fn resolve_text_write_font(
    doc: &mut Document,
    page_id: lopdf::ObjectId,
    current_font_alias: &[u8],
    current_font: Option<&ParsedFont>,
    text: &str,
) -> Result<PdfTextWriteFont, String> {
    if let Some(font) = current_font {
        if can_pdf_font_encode_text(font, text) {
            println!(
                "[PDF-WRITE-FONT][reuse-original] font='{}' alias={} text='{}'",
                font.name,
                String::from_utf8_lossy(current_font_alias),
                truncate_log(text, 80)
            );
            return Ok(PdfTextWriteFont {
                font_alias: current_font_alias.to_vec(),
                parsed_font: font.clone(),
                encoding: PdfTextWriteEncoding::OriginalPdfFont,
                source_label: format!("original-pdf-font:{}", font.name),
            });
        }
    }

    let candidate_names = finder::candidate_font_names(current_font);
    let system_font = finder::find_system_font(&candidate_names, text)?;
    let alias = embed::ensure_font_in_page(doc, page_id, &system_font, text)?;
    let parsed_font = face::parsed_font_from_system_font(&system_font);

    println!(
        "[PDF-WRITE-FONT][resolved] alias={} source={} family='{}' ps='{}' text='{}'",
        String::from_utf8_lossy(&alias),
        system_font.source_label,
        system_font.family_name,
        system_font.post_script_name,
        truncate_log(text, 80)
    );

    Ok(PdfTextWriteFont {
        font_alias: alias,
        parsed_font,
        encoding: PdfTextWriteEncoding::TrueTypeGlyphIds {
            font_bytes: system_font.font_bytes,
            face_index: system_font.face_index,
        },
        source_label: system_font.source_label,
    })
}
fn can_pdf_font_encode_text(font: &ParsedFont, text: &str) -> bool {
    let mut has_visible = false;
    for ch in text.chars() {
        if ch == '\n' || ch == '\r' || ch == '\t' {
            continue;
        }
        has_visible = true;
        if !font.can_encode(ch) {
            return false;
        }
    }
    has_visible
}

/// Filter a font name to characters valid inside a PDF name object,
/// falling back to a fixed name when nothing survives.
pub(crate) fn sanitize_pdf_name(value: &str) -> String {
    let mut out = value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-' || *ch == '_')
        .collect::<String>();
    if out.is_empty() {
        out = "HsaWriteFont".to_string();
    }
    if out.len() > 48 {
        out.truncate(48);
    }
    out
}
fn truncate_log(value: &str, limit: usize) -> String {
    let mut out = String::new();
    let mut chars = value.chars();
    for _ in 0..limit {
        if let Some(ch) = chars.next() {
            out.push(ch);
        } else {
            break;
        }
    }
    if chars.next().is_some() {
        out.push_str("...");
    }
    out
}

#[cfg(test)]
mod util_tests {
    use super::*;

    #[test]
    fn sanitize_pdf_name_filters_invalid_chars() {
        assert_eq!(sanitize_pdf_name("ABC 123!@#"), "ABC123");
        assert_eq!(sanitize_pdf_name("Font-Name_2"), "Font-Name_2");
    }

    #[test]
    fn sanitize_pdf_name_falls_back_when_empty() {
        assert_eq!(sanitize_pdf_name("!!!"), "HsaWriteFont");
    }

    #[test]
    fn sanitize_pdf_name_truncates_to_48() {
        assert_eq!(sanitize_pdf_name(&"A".repeat(60)).len(), 48);
    }

    #[test]
    fn truncate_log_cuts_and_marks_long_values() {
        assert_eq!(truncate_log("short", 80), "short");
        assert_eq!(truncate_log("abcdef", 3), "abc...");
    }
}
