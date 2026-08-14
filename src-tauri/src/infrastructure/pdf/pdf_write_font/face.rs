use crate::infrastructure::pdf::font::ttc::extract_ttc_face_as_ttf;
use crate::infrastructure::pdf::pdf_font::{CMap, ParsedFont};
use crate::infrastructure::pdf::pdf_write_font::SystemFont;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::Arc;
use ttf_parser::{Face, GlyphId};

use super::sanitize_pdf_name;

/// Turn raw font bytes into a [`SystemFont`], rejecting fonts that cannot
/// cover every visible character of `text`.
pub(crate) fn font_from_bytes(
    data: &[u8],
    face_index: u32,
    requested_family: &str,
    source_label: &str,
    text: &str,
) -> Option<SystemFont> {
    let standalone = standalone_ttf_bytes(data, face_index)?;
    let font_bytes = Arc::new(standalone);
    let face = Face::parse(font_bytes.as_slice(), 0).ok()?;
    if !font_covers_text(&face, text) {
        println!(
            "[PDF-WRITE-FONT][reject-coverage] family='{}' source={} missing={}",
            requested_family,
            source_label,
            missing_chars(&face, text)
        );
        return None;
    }

    let glyphs = glyph_subset(&face, text)?;
    let units = face.units_per_em().max(1) as f32;
    let bbox = face.global_bounding_box();
    let post_script_name =
        post_script_name(&face).unwrap_or_else(|| sanitize_pdf_name(requested_family));

    let ascent = face.ascender() as f32 / units * 1000.0;
    let descent = face.descender() as f32 / units * 1000.0;
    let italic_angle = face.italic_angle();
    let weight = face.weight().to_number() as i32;

    Some(SystemFont {
        family_name: requested_family.to_string(),
        post_script_name,
        source_label: source_label.to_string(),
        font_bytes,
        face_index: 0,
        glyphs,
        bbox: [
            bbox.x_min as f32 / units * 1000.0,
            bbox.y_min as f32 / units * 1000.0,
            bbox.x_max as f32 / units * 1000.0,
            bbox.y_max as f32 / units * 1000.0,
        ],
        ascent,
        descent,
        cap_height: ascent,
        italic_angle,
        weight,
    })
}

/// Extract a standalone TrueType program from `data`: pass through TTF,
/// pull a face out of a TTC, reject CFF-based OpenType (needs FontFile3).
pub(crate) fn standalone_ttf_bytes(data: &[u8], face_index: u32) -> Option<Vec<u8>> {
    match data.get(0..4) {
        Some(b"\x00\x01\x00\x00") | Some(b"true") => Some(data.to_vec()),
        Some(b"ttcf") => extract_ttc_face_as_ttf(data, face_index).ok(),
        Some(b"OTTO") => {
            println!("[PDF-WRITE-FONT][reject-cff] OpenType CFF requires FontFile3 writer");
            None
        }
        _ => None,
    }
}
fn font_covers_text(face: &Face<'_>, text: &str) -> bool {
    text.chars()
        .all(|ch| ch == '\n' || ch == '\r' || ch == '\t' || face.glyph_index(ch).is_some())
}
fn missing_chars(face: &Face<'_>, text: &str) -> String {
    let mut missing = BTreeSet::new();
    for ch in text.chars() {
        if ch == '\n' || ch == '\r' || ch == '\t' {
            continue;
        }
        if face.glyph_index(ch).is_none() {
            missing.insert(format!("'{}'(U+{:04X})", ch, ch as u32));
        }
    }
    missing.into_iter().collect::<Vec<_>>().join(", ")
}
fn glyph_subset(face: &Face<'_>, text: &str) -> Option<Vec<(char, u16, f32)>> {
    let mut seen = HashSet::<u16>::new();
    let mut out = Vec::new();
    let units = face.units_per_em().max(1) as f32;
    for ch in text.chars() {
        if ch == '\n' || ch == '\r' || ch == '\t' {
            continue;
        }
        let gid = face.glyph_index(ch)?.0;
        if seen.insert(gid) {
            let advance = face
                .glyph_hor_advance(GlyphId(gid))
                .unwrap_or(face.units_per_em());
            out.push((ch, gid, advance as f32 / units * 1000.0));
        }
    }
    Some(out)
}

/// Encode `text` as a 2-byte-per-glyph Identity-H string using the glyph
/// ids of the resolved write font.
pub(crate) fn encode_text_as_glyph_ids(
    font_bytes: &[u8],
    face_index: u32,
    text: &str,
) -> Result<Vec<u8>, String> {
    let face = Face::parse(font_bytes, face_index)
        .map_err(|err| format!("failed to parse resolved write font: {:?}", err))?;
    let mut encoded = Vec::new();
    for ch in text.chars() {
        if ch == '\n' || ch == '\r' || ch == '\t' {
            continue;
        }
        let gid = face
            .glyph_index(ch)
            .ok_or_else(|| {
                format!(
                    "resolved write font cannot encode '{}'(U+{:04X})",
                    ch, ch as u32
                )
            })?
            .0;
        encoded.push((gid >> 8) as u8);
        encoded.push((gid & 0xFF) as u8);
    }
    Ok(encoded)
}
pub(crate) fn parsed_font_from_system_font(font: &SystemFont) -> ParsedFont {
    let mut widths = HashMap::new();
    let mut pairs = Vec::new();
    for (ch, gid, width) in &font.glyphs {
        widths.insert(*gid as u32, *width);
        widths.insert(*ch as u32, *width);
        pairs.push((*gid, ch.to_string()));
    }
    ParsedFont {
        name: font.family_name.clone(),
        base_font: font.post_script_name.clone(),
        post_script_name: Some(font.post_script_name.clone()),
        family_hint: Some(font.family_name.clone()),
        font_subtype: Some("Type0".to_string()),
        embedded_font_key: Some(font.source_label.clone()),
        has_embedded_program: true,
        has_to_unicode_cmap: true,
        widths,
        default_width: 1000.0,
        cmap: Some(CMap::from_codepoint_pairs(pairs)),
        hints: None,
    }
}
fn post_script_name(face: &Face<'_>) -> Option<String> {
    face.names()
        .into_iter()
        .find(|name| name.name_id == ttf_parser::name_id::POST_SCRIPT_NAME)
        .and_then(|name| name.to_string())
}
