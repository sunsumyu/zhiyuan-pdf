use crate::{log_audit, log_step};
use std::sync::Arc;
use lopdf::{Document, Object};
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct FontHints {
    pub is_bold: bool,
    pub is_italic: bool,
    pub is_fixed_pitch: bool,
    pub is_serif: bool,
    pub is_symbolic: bool,
    pub is_script: bool,
    pub is_all_cap: bool,
    pub is_small_cap: bool,
    pub is_force_bold: bool,
}

#[derive(Debug, Clone, Default)]
pub struct CMap {
    pub mappings: HashMap<u16, String>,
    pub rev_mappings: HashMap<String, u16>,
}

impl CMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_codepoint_pairs(pairs: Vec<(u16, String)>) -> Self {
        let mut mappings = HashMap::new();
        let mut rev_mappings = HashMap::new();
        for (code, s) in pairs {
            mappings.insert(code, s.clone());
            rev_mappings.insert(s, code);
        }
        Self { mappings, rev_mappings }
    }
}

#[derive(Debug, Clone)]
pub struct ParsedFont {
    pub name: String,
    pub base_font: String,
    pub font_subtype: Option<String>,
    pub cmap: Option<CMap>,
    pub widths: HashMap<u32, f32>,
    pub default_width: f32,
    pub hints: Option<FontHints>,
    pub post_script_name: Option<String>,
    pub family_hint: Option<String>,
    pub embedded_font_key: Option<String>,
    pub has_embedded_program: bool,
    pub has_to_unicode_cmap: bool,
}

impl ParsedFont {
    pub fn is_multibyte(&self) -> bool {
        self.font_subtype.as_deref() == Some("Type0")
    }

    pub fn can_encode(&self, ch: char) -> bool {
        if let Some(ref cmap) = self.cmap {
            cmap.rev_mappings.contains_key(&ch.to_string())
        } else {
            ch.is_ascii()
        }
    }

    pub fn encode_text(&self, text: &str) -> Vec<u8> {
        let is_multi = self.is_multibyte();
        if let Some(ref cmap) = self.cmap {
            let mut result = Vec::new();
            for c in text.chars() {
                let char_str = c.to_string();
                if let Some(&code) = cmap.rev_mappings.get(&char_str) {
                    if is_multi {
                        result.push((code >> 8) as u8);
                        result.push((code & 0xFF) as u8);
                    } else {
                        result.push((code & 0xFF) as u8);
                    }
                } else if char_str.is_ascii() {
                    let b = char_str.as_bytes()[0];
                    if is_multi {
                        result.push(0x00);
                        result.push(b);
                    } else {
                        result.push(b);
                    }
                } else {
                    if is_multi {
                        result.push(0x00);
                        result.push(0x00);
                    } else {
                        result.push(b'?');
                    }
                }
            }
            result
        } else {
            text.as_bytes().to_vec()
        }
    }

    pub fn get_text_width(
        &self,
        text: &str,
        font_size: f32,
        char_spacing: f32,
        horizontal_scaling: f32,
    ) -> f32 {
        let mut total_width = 0.0;
        let h_scale = horizontal_scaling / 100.0;
        for c in text.chars() {
            let code = if let Some(ref cmap) = self.cmap {
                cmap.rev_mappings
                    .get(&c.to_string())
                    .copied()
                    .unwrap_or(c as u32 as u16) as u32
            } else {
                c as u32
            };

            let width = self.widths.get(&code).copied().unwrap_or(self.default_width);
            total_width += ((width / 1000.0) * font_size + char_spacing) * h_scale;
        }
        total_width
    }
}

/// [Precise & Simple] Resolve glyph geometry from raw PDF bytes
pub fn resolve_glyph_geom(
    data: &[u8],
    font: &ParsedFont,
    font_size: f32,
    h_scale: f32,
    char_spacing: f32,
    word_spacing: f32,
) -> (String, Vec<f32>, Vec<f32>, Vec<u32>, f32) {
    let mut combined_text = String::new();
    let mut origins = Vec::new();
    let mut widths = Vec::new();
    let mut pdf_char_codes = Vec::new();
    let mut current_offset = 0.0;

    let mut i = 0;
    let is_symbol = {
        let lower = font.name.to_lowercase();
        lower.contains("symbol") || lower.contains("wingdings") || lower.contains("dingbats")
    };
    let multibyte = font.is_multibyte();
    let has_cmap = font.cmap.is_some();
    crate::pdf_log!(2, "[GLYPH-DECODE] font='{}' subtype={:?} multibyte={} has_cmap={} is_symbol={} data_len={}",
        font.name, font.font_subtype, multibyte, has_cmap, is_symbol, data.len());

    while i < data.len() {
        let code: u32;
        let mut unicode: String;

        if font.is_multibyte() {
            let hi = *data.get(i).unwrap_or(&0) as u32;
            let lo = *data.get(i + 1).unwrap_or(&0) as u32;
            code = (hi << 8) | lo;
            let cmap_hit = font.cmap.as_ref().and_then(|m| m.mappings.get(&(code as u16))).cloned();
            let had_hit = cmap_hit.is_some();
            unicode = cmap_hit
                .unwrap_or_else(|| char::from_u32(code).map(|c| c.to_string()).unwrap_or_else(|| format!("[0x{:04X}]", code)));
            crate::pdf_log!(2, "[GLYPH-DECODE] 2byte code=0x{:04X} cmap_hit={} unicode={:?} (U+{:04X})",
                code, had_hit, unicode, unicode.chars().next().map(|c| c as u32).unwrap_or(0));
            i += 2;
        } else {
            code = data[i] as u32;
            let cmap_hit = font.cmap.as_ref().and_then(|m| m.mappings.get(&(code as u16))).cloned();
            let had_hit = cmap_hit.is_some();
            unicode = cmap_hit
                .unwrap_or_else(|| char::from_u32(code).map(|c| c.to_string()).unwrap_or_else(|| "".to_string()));
            
            // Symbol Patching: patch when no CMap, CMap result is ASCII, or CMap result is PUA (U+E000-U+F8FF)
            let cp = unicode.chars().next().map(|c| c as u32).unwrap_or(0);
            if is_symbol && (unicode.is_empty() || cp <= 127 || (cp >= 0xE000 && cp <= 0xF8FF)) {
                let patched = match code {
                    0x7A | 0x6C | 0x6A | 0x6B | 0xB7 => "●",
                    0xA7 => "●",
                    0x6E | 0x73 => "■",
                    0x75 => "◆",
                    0xFC | 0xFE => "✔",
                    _ => "",
                };
                if !patched.is_empty() { unicode = patched.to_string(); }
            }
            crate::pdf_log!(2, "[GLYPH-DECODE] 1byte code=0x{:02X} cmap_hit={} symbol_patched={} unicode={:?} (U+{:04X})",
                code, had_hit, is_symbol, unicode, unicode.chars().next().map(|c| c as u32).unwrap_or(0));
            i += 1;
        }

        // Secondary Patching for CID-mapped symbols (0xF000/0xE000 range)
        if unicode.chars().count() == 1 {
            let cp = unicode.chars().next().unwrap() as u32;
            if (cp >= 0xF000 && cp <= 0xF0FF) || (cp >= 0xE000 && cp <= 0xE0FF) {
                let patched = match cp & 0xFF {
                    0x6A | 0x6B | 0x6C | 0xB7 => Some("●"),
                    0x6E => Some("■"),
                    0xFC => Some("✓"),
                    _ => None,
                };
                if let Some(p) = patched {
                    unicode = p.to_string();
                }
            }
        }

        let w0 = font.widths.get(&code).cloned().unwrap_or(font.default_width);
        let spacing = if unicode == " " { word_spacing } else { 0.0 };
        let advance = ((w0 / 1000.0) * font_size + char_spacing + spacing) * h_scale;

        origins.push(current_offset);
        widths.push(advance);
        pdf_char_codes.push(code);
        current_offset += advance;
        combined_text.push_str(&unicode);
    }

    // Always-on diagnostic: log any suspicious decoding results
    let has_empty_or_nonbmp = combined_text.is_empty()
        || combined_text.contains('\u{FFFD}')
        || combined_text.chars().any(|c| { let cp = c as u32; cp > 0xFFFF || (cp < 0x20 && cp != 0x0A && cp != 0x0D) });
    if has_empty_or_nonbmp || data.len() <= 4 || is_symbol {
        let decoded_codes: Vec<String> = (0..data.len().min(32))
            .map(|idx| format!("0x{:02X}", data[idx]))
            .collect();
        let result_codes: Vec<String> = combined_text.chars()
            .take(32)
            .map(|c| format!("U+{:04X}({})", c as u32, c))
            .collect();
        crate::pdf_log!(
            3,
            "[GLYPH-DECODE-RESULT] font='{}' subtype={:?} multibyte={} is_symbol={} data=[{}] => text={:?} chars=[{}]",
            font.name, font.font_subtype, multibyte, is_symbol,
            decoded_codes.join(","),
            combined_text,
            result_codes.join(",")
        );
    }

    (combined_text, origins, widths, pdf_char_codes, current_offset)
}

/// [Precise & Simple] Read CMap from raw PDF bytes
pub fn read_cmap(data: &[u8]) -> CMap {
    let mut cmap = CMap::default();
    let content = String::from_utf8_lossy(data);
    let mut lines = content.lines();

    while let Some(line) = lines.next() {
        let line = line.trim();
        if line.contains("beginbfchar") {
            while let Some(mapping_line) = lines.next() {
                let mapping_line = mapping_line.trim();
                if mapping_line.contains("endbfchar") { break; }
                let parts: Vec<&str> = mapping_line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let code = u16::from_str_radix(parts[0].trim_matches(|c| c == '<' || c == '>'), 16).unwrap_or(0);
                    let val = hex_to_string(parts[1].trim_matches(|c| c == '<' || c == '>'));
                    cmap.rev_mappings.insert(val.clone(), code);
                    cmap.mappings.insert(code, val);
                }
            }
        } else if line.contains("beginbfrange") {
            while let Some(mapping_line) = lines.next() {
                let mapping_line = mapping_line.trim();
                if mapping_line.contains("endbfrange") { break; }
                let parts: Vec<&str> = mapping_line.split_whitespace().collect();
                if parts.len() >= 3 {
                    let start = u16::from_str_radix(parts[0].trim_matches(|c| c == '<' || c == '>'), 16).unwrap_or(0);
                    let end = u16::from_str_radix(parts[1].trim_matches(|c| c == '<' || c == '>'), 16).unwrap_or(0);
                    if parts[2].starts_with('[') {
                        let array_content = parts[2..].join(" ");
                        let items: Vec<&str> = array_content.trim_matches(|c| c == '[' || c == ']').split_whitespace().collect();
                        for (idx, v_hex) in items.iter().enumerate() {
                            let code = start + idx as u16;
                            if code <= end {
                                cmap.mappings.insert(code, hex_to_string(v_hex.trim_matches(|c| c == '<' || c == '>')));
                            }
                        }
                    } else {
                        let base_val = u16::from_str_radix(parts[2].trim_matches(|c| c == '<' || c == '>'), 16).unwrap_or(0);
                        for code in start..=end {
                            let mapped_val = base_val + (code - start);
                            let val = char::from_u32(mapped_val as u32).map(|c| c.to_string()).unwrap_or_default();
                            cmap.rev_mappings.insert(val.clone(), code);
                            cmap.mappings.insert(code, val);
                        }
                    }
                }
            }
        }
    }
    cmap
}

fn hex_to_string(hex: &str) -> String {
    let mut res = String::new();
    for i in (0..hex.len()).step_by(4) {
        if i + 4 <= hex.len() {
            if let Ok(u) = u16::from_str_radix(&hex[i..i + 4], 16) {
                if let Some(c) = char::from_u32(u as u32) { res.push(c); }
            }
        }
    }
    if res.is_empty() && !hex.is_empty() {
        for i in (0..hex.len()).step_by(2) {
            if i + 2 <= hex.len() {
                if let Ok(u) = u8::from_str_radix(&hex[i..i + 2], 16) { res.push(u as char); }
            }
        }
    }
    res
}
#[derive(Debug, Clone)]
pub struct ParsedImage {
    pub data_url: String,
    pub mime: String,
    pub extraction_method: String,
}

pub struct ResourceCache {
    pub fonts: HashMap<lopdf::ObjectId, Arc<ParsedFont>>,
    pub images: HashMap<lopdf::ObjectId, Arc<ParsedImage>>,
}

impl ResourceCache {
    pub fn new() -> Self {
        Self {
            fonts: HashMap::new(),
            images: HashMap::new(),
        }
    }
}

pub fn break_text_into_lines(
    text: &str,
    runs: Option<&Vec<pdf_viewer_core::models::LayoutRun>>,
    font: &ParsedFont,
    font_size: f32,
    max_width: f32,
    align: Option<pdf_viewer_core::models::LayoutAlignment>,
    line_height: Option<f32>,
    char_spacing: f32,
    scale_x: f32,
) -> pdf_viewer_core::geometry::layout_engine::ParagraphLayout {
    use pdf_viewer_core::geometry::layout_engine::layout_paragraph;
    use pdf_viewer_core::models::{LayoutAlignment, LayoutParagraph, LayoutRun, ParagraphStyle, RunStyle};

    let layout_runs = if let Some(r) = runs { r.clone() } else {
        vec![LayoutRun {
            id: "patch-run-0".into(),
            text: text.to_string(),
            style: RunStyle { font_size, char_spacing, scale_x, ..Default::default() },
            ..Default::default()
        }]
    };

    let paragraph = LayoutParagraph {
        id: "patch-para-0".into(),
        runs: layout_runs,
        style: ParagraphStyle {
            align: align.unwrap_or(LayoutAlignment::Left),
            line_height: line_height.unwrap_or(1.2).max(0.8),
            ..Default::default()
        },
        ..Default::default()
    };

    layout_paragraph(&paragraph, max_width, |run_text, _| {
        font.get_text_width(run_text, font_size, char_spacing, scale_x)
    })
}

use crate::infrastructure::pdf::models::PathSegment;

pub fn simplify_path_segments(segments: Vec<PathSegment>, epsilon: f32) -> Vec<PathSegment> {
    if segments.is_empty() { return segments; }
    let mut result = Vec::with_capacity(segments.len());
    let mut current_poly: Vec<[f32; 2]> = Vec::new();

    let mut flush_poly = |poly: &mut Vec<[f32; 2]>, res: &mut Vec<PathSegment>| {
        if poly.is_empty() { return; }
        if poly.len() > 2 {
            let simplified = simplify_points(poly, epsilon);
            for (i, pt) in simplified.into_iter().enumerate() {
                res.push(PathSegment { command: if i == 0 { "move".into() } else { "line".into() }, points: vec![pt] });
            }
        } else {
            for (i, pt) in poly.drain(..).enumerate() {
                res.push(PathSegment { command: if i == 0 { "move".into() } else { "line".into() }, points: vec![pt] });
            }
        }
        poly.clear();
    };

    for seg in segments {
        if seg.command == "move" || seg.command == "line" {
            current_poly.push(seg.points[0]);
        } else {
            flush_poly(&mut current_poly, &mut result);
            result.push(seg);
        }
    }
    flush_poly(&mut current_poly, &mut result);
    result
}

fn simplify_points(points: &[[f32; 2]], epsilon: f32) -> Vec<[f32; 2]> {
    if points.len() < 3 { return points.to_vec(); }
    let mut dmax = 0.0;
    let mut index = 0;
    let end = points.len() - 1;
    for i in 1..end {
        let d = perpendicular_distance(points[i], points[0], points[end]);
        if d > dmax { index = i; dmax = d; }
    }
    if dmax > epsilon {
        let mut res1 = simplify_points(&points[0..=index], epsilon);
        let mut res2 = simplify_points(&points[index..=end], epsilon);
        res1.pop();
        res1.append(&mut res2);
        res1
    } else { vec![points[0], points[end]] }
}

fn perpendicular_distance(p: [f32; 2], p1: [f32; 2], p2: [f32; 2]) -> f32 {
    let [x, y] = p;
    let [x1, y1] = p1;
    let [x2, y2] = p2;
    let dx = x2 - x1;
    let dy = y2 - y1;
    let den = (dy * dy + dx * dx).sqrt();
    if den < 0.0001 { return ((x - x1).powi(2) + (y - y1).powi(2)).sqrt(); }
    (dy * x - dx * y + x2 * y1 - y2 * x1).abs() / den
}
pub fn parse_font_from_dict(
    doc: &Document,
    font_id: lopdf::ObjectId,
    name_bytes: &[u8],
) -> Result<ParsedFont, String> {
    let fd = doc.get_dictionary(font_id).map_err(|e| e.to_string())?;
    let mut real_name = String::from_utf8_lossy(name_bytes).into_owned();
    let mut post_script_name = None;
    let font_subtype = fd.get(b"Subtype").ok().and_then(|value| value.as_name().ok()).map(|value| {
        String::from_utf8_lossy(value).trim_start_matches('/').to_string()
    });

    if let Ok(base_font) = fd.get(b"BaseFont").and_then(|n| n.as_name()) {
        real_name = String::from_utf8_lossy(base_font).trim_start_matches('/').to_string();
        post_script_name = Some(real_name.clone());
        if let Some(plus_pos) = real_name.find('+') {
            if plus_pos == 6 { real_name = real_name[(plus_pos + 1)..].to_string(); }
        }
    }

    let descendant_dict = fd.get(b"DescendantFonts").ok()
        .and_then(|o| o.as_array().ok())
        .and_then(|descendants| descendants.get(0))
        .and_then(|o| {
            o.as_dict().ok().cloned().or_else(|| o.as_reference().ok().and_then(|r| doc.get_dictionary(r).ok()).cloned())
        });

    let mut widths = HashMap::new();
    let mut default_width = 1000.0;

    if let Some(desc_dict) = descendant_dict.as_ref() {
        default_width = desc_dict.get(b"DW").ok().and_then(|o| o.as_float().ok().or_else(|| o.as_i64().ok().map(|i| i as f32))).unwrap_or(1000.0);
        if let Some(w_array) = desc_dict.get(b"W").ok().and_then(|o| o.as_array().ok()) {
            let mut i = 0;
            while i < w_array.len() {
                if let (Some(first), Some(next)) = (w_array.get(i), w_array.get(i+1)) {
                    let c_first = first.as_i64().unwrap_or(0) as u32;
                    if let Ok(ws) = next.as_array() {
                        for (idx, w_obj) in ws.iter().enumerate() {
                            let w = w_obj.as_float().ok().or_else(|| w_obj.as_i64().map(|i| i as f32).ok()).unwrap_or(0.0);
                            widths.insert(c_first + idx as u32, w);
                        }
                        i += 2;
                    } else if let Some(last_obj) = w_array.get(i+1) {
                        if let Some(w_obj) = w_array.get(i+2) {
                            let c_last = last_obj.as_i64().unwrap_or(0) as u32;
                            let w = w_obj.as_float().ok().or_else(|| w_obj.as_i64().map(|i| i as f32).ok()).unwrap_or(0.0);
                            for c in c_first..=c_last { widths.insert(c, w); }
                            i += 3;
                        } else { i += 2; }
                    } else { i += 2; }
                } else { break; }
            }
        }
    }

    let mut hints = None;
    let mut family_hint = None;
    let mut has_embedded_program = false;
    let mut embedded_font_key: Option<String> = None;
    let font_desc_dict = fd.get(b"FontDescriptor").ok()
        .and_then(|o| o.as_dict().ok().cloned().or_else(|| o.as_reference().ok().and_then(|r| doc.get_dictionary(r).ok()).cloned()))
        .or_else(|| descendant_dict.clone().and_then(|d| d.get(b"FontDescriptor").ok().and_then(|o| o.as_dict().ok().cloned().or_else(|| o.as_reference().ok().and_then(|r| doc.get_dictionary(r).ok()).cloned()))));

    if let Some(font_desc) = font_desc_dict {
        family_hint = font_desc.get(b"FontFamily").ok().and_then(|o| o.as_str().ok()).map(|v| String::from_utf8_lossy(v).to_string());
        let flags = font_desc.get(b"Flags").and_then(|o| o.as_i64()).unwrap_or(0) as i32;
        let weight = font_desc.get(b"FontWeight").and_then(|o| o.as_i64()).unwrap_or(400) as i32;
        hints = Some(FontHints {
            is_bold: (flags & 262144) != 0 || weight >= 700,
            is_italic: (flags & 64) != 0,
            is_fixed_pitch: (flags & 1) != 0,
            is_serif: (flags & 2) != 0,
            ..Default::default()
        });
    }

    let mut cmap = None;
    let mut has_to_unicode_cmap = false;
    if let Ok(to_unicode) = fd.get(b"ToUnicode") {
        has_to_unicode_cmap = true;
        if let Some(stream) = to_unicode.as_reference().ok().and_then(|r| doc.get_object(r).ok()).and_then(|o| o.as_stream().ok()) {
            if let Ok(data) = stream.decompressed_content() {
                cmap = Some(read_cmap(&data));
            }
        }
    }

    Ok(ParsedFont {
        name: real_name,
        base_font: post_script_name.clone().unwrap_or_default(),
        font_subtype,
        cmap,
        widths,
        default_width,
        hints,
        post_script_name,
        family_hint,
        embedded_font_key: None,
        has_embedded_program: false,
        has_to_unicode_cmap,
    })
}
