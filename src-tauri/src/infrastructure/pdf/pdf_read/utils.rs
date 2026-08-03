use crate::infrastructure::pdf::models::PathSegment;
use crate::infrastructure::pdf::pdf_font::{resolve_glyph_geom, ParsedFont};
use lopdf::Object;
pub fn operands_to_f32(ops: &[Object]) -> Result<Vec<f32>, String> {
    let mut res = Vec::new();
    for op in ops {
        if let Ok(f) = op.as_float() {
            res.push(f);
        } else if let Ok(i) = op.as_i64() {
            res.push(i as f32);
        }
    }
    Ok(res)
}

pub fn multiply_matrices(current: [f32; 6], new: [f32; 6]) -> [f32; 6] {
    let (a1, b1, c1, d1, e1, f1) = (
        current[0], current[1], current[2], current[3], current[4], current[5],
    );
    let (a2, b2, c2, d2, e2, f2) = (new[0], new[1], new[2], new[3], new[4], new[5]);
    [
        a1 * a2 + c1 * b2,
        b1 * a2 + d1 * b2,
        a1 * c2 + c1 * d2,
        b1 * c2 + d1 * d2,
        a1 * e2 + c1 * f2 + e1,
        b1 * e2 + d1 * f2 + f1,
    ]
}

/// Apply alpha into a `#rrggbb` color, producing `#rrggbbaa` when alpha < 1.0.
/// Fully-opaque colors are returned unchanged to preserve existing output.
pub(crate) fn apply_alpha_to_color(color: &Option<String>, alpha: f32) -> Option<String> {
    let base = color.as_ref()?;
    if alpha >= 0.999 {
        return Some(base.clone());
    }
    if base.starts_with('#') && base.len() == 7 {
        let alpha_byte = (alpha.clamp(0.0, 1.0) * 255.0).round() as u8;
        Some(format!("{}{:02x}", base, alpha_byte))
    } else {
        Some(base.clone())
    }
}

/// Axis-aligned bounding box of a path segment list, or `None` if empty.
pub(crate) fn compute_segments_bbox(segments: &[PathSegment]) -> Option<(f32, f32, f32, f32)> {
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for seg in segments {
        for pt in &seg.points {
            if pt[0] < min_x {
                min_x = pt[0];
            }
            if pt[0] > max_x {
                max_x = pt[0];
            }
            if pt[1] < min_y {
                min_y = pt[1];
            }
            if pt[1] > max_y {
                max_y = pt[1];
            }
        }
    }
    if min_x.is_infinite() {
        None
    } else {
        Some((min_x, min_y, max_x, max_y))
    }
}

/// Resolve a TJ array (mixed strings and kerning adjustments) into unified text geometry.
/// Each string element is resolved via `resolve_glyph_geom`; numeric elements adjust the
/// horizontal offset (negative kern). Returns (text, origins, widths, codes, total_advance).
pub(crate) fn resolve_tj_array_text(
    items: &[lopdf::Object],
    font: &ParsedFont,
    font_size: f32,
    h_scale: f32,
    char_spacing: f32,
    word_spacing: f32,
) -> (String, Vec<f32>, Vec<f32>, Vec<u32>, f32) {
    let mut combined = String::new();
    let mut all_origins = Vec::new();
    let mut all_widths = Vec::new();
    let mut all_codes = Vec::new();
    let mut offset = 0.0f32;
    for item in items {
        if let Ok(bytes) = item.as_str() {
            let (t, o, w, c, adv) =
                resolve_glyph_geom(bytes, font, font_size, h_scale, char_spacing, word_spacing);
            for ori in o {
                all_origins.push(offset + ori);
            }
            all_widths.extend(w);
            all_codes.extend(c);
            combined.push_str(&t);
            offset += adv;
        } else if let Ok(kern) = item.as_float().or_else(|_| item.as_i64().map(|i| i as f32)) {
            offset -= (kern / 1000.0) * font_size * h_scale;
        }
    }
    (combined, all_origins, all_widths, all_codes, offset)
}

lazy_static::lazy_static! {
    static ref PAGE_LOCKS: std::sync::Mutex<std::collections::HashMap<String, std::sync::Arc<std::sync::Mutex<()>>>> =
        std::sync::Mutex::new(std::collections::HashMap::new());
}
