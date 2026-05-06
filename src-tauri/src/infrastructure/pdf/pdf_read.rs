use crate::{log_audit, log_step};
use lopdf::{Document, Object, Dictionary, content::Content};
use std::collections::HashMap;
use std::sync::Arc;
use crate::infrastructure::pdf::models::*;
use crate::infrastructure::pdf::pdf_font::{ParsedFont, resolve_glyph_geom, ResourceCache, simplify_path_segments, read_cmap};

#[derive(Clone, Debug)]
pub struct GraphicsState {
    pub ctm: [f32; 6],
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
    pub tm: [f32; 6],
    pub tlm: [f32; 6],
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
            ctm: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
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
            tm: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            tlm: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            tl: 0.0,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            text_rise: 0.0,
            render_mode: 0,
        }
    }

    pub fn transform_point(&self, x: f32, y: f32) -> [f32; 2] {
        let (a, b, c, d, e, f) = (self.ctm[0], self.ctm[1], self.ctm[2], self.ctm[3], self.ctm[4], self.ctm[5]);
        [a * x + c * y + e, b * x + d * y + f]
    }
}

pub type FlatResources = HashMap<Vec<u8>, HashMap<Vec<u8>, lopdf::ObjectId>>;

pub fn read_resources(doc: &Document, page_id: lopdf::ObjectId) -> FlatResources {
    let mut flat: FlatResources = HashMap::new();
    let mut curr_id = page_id;
    let mut visited = std::collections::HashSet::new();

    while let Ok(dict) = doc.get_dictionary(curr_id) {
        if let Ok(res_obj) = dict.get(b"Resources") {
            if let Ok(res_dict) = res_obj.as_dict().or_else(|_| res_obj.as_reference().and_then(|r| doc.get_dictionary(r))) {
                for (cat_key, cat_val) in res_dict.iter() {
                    let cat_map = flat.entry(cat_key.clone()).or_insert_with(HashMap::new);
                    if let Ok(sub_dict) = cat_val.as_dict().or_else(|_| cat_val.as_reference().and_then(|r| doc.get_dictionary(r))) {
                        for (res_name, res_val) in sub_dict.iter() {
                            if let Ok(id) = res_val.as_reference() {
                                cat_map.entry(res_name.clone()).or_insert(id);
                            }
                        }
                    }
                }
            }
        }
        if let Ok(parent_ref) = dict.get(b"Parent").and_then(|o| o.as_reference()) {
            if visited.contains(&parent_ref) { break; }
            visited.insert(parent_ref);
            curr_id = parent_ref;
        } else { break; }
    }
    flat
}

pub fn operands_to_f32(ops: &[Object]) -> Result<Vec<f32>, String> {
    let mut res = Vec::new();
    for op in ops {
        if let Ok(f) = op.as_float() { res.push(f); }
        else if let Ok(i) = op.as_i64() { res.push(i as f32); }
    }
    Ok(res)
}

pub fn multiply_matrices(current: [f32; 6], new: [f32; 6]) -> [f32; 6] {
    let (a1, b1, c1, d1, e1, f1) = (new[0], new[1], new[2], new[3], new[4], new[5]);
    let (a2, b2, c2, d2, e2, f2) = (current[0], current[1], current[2], current[3], current[4], current[5]);
    [
        a1 * a2 + b1 * c2,
        a1 * b2 + b1 * d2,
        c1 * a2 + d1 * c2,
        c1 * b2 + d1 * d2,
        e1 * a2 + f1 * c2 + e2,
        e1 * b2 + f1 * d2 + f2,
    ]
}

pub fn resolve_paths(
    doc: &Document,
    page_index: u32,
) -> Result<(Vec<RenderObject>, Vec<StyledRun>, f32, f32), String> {
    log_audit!("[PDF-AUDIT] resolve_paths START page={}", page_index);
    let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found")?;
    let page_dict = doc.get_dictionary(page_id).map_err(|e| e.to_string())?;

    let (width, height) = if let Ok(box_obj) = page_dict.get(b"MediaBox") {
        let arr = box_obj.as_array().map_err(|e| e.to_string())?;
        if arr.len() >= 4 {
            let w = arr[2].as_float().or_else(|_| arr[2].as_i64().map(|v| v as f32)).unwrap_or(595.0);
            let h = arr[3].as_float().or_else(|_| arr[3].as_i64().map(|v| v as f32)).unwrap_or(842.0);
            let y0 = arr[1].as_float().or_else(|_| arr[1].as_i64().map(|v| v as f32)).unwrap_or(0.0);
            (w, h - y0)
        } else { (595.0, 842.0) }
    } else { (595.0, 842.0) };

    let flat_resources = read_resources(doc, page_id);
    let mut res_cache = ResourceCache::new();

    let content_data = doc.get_page_content(page_id).map_err(|e| e.to_string())?;
    let content = Content::decode(&content_data).map_err(|e| e.to_string())?;

    let mut objects = Vec::new();
    let mut text_runs = Vec::new();
    let mut obj_counter = 0;

    parse_content_stream(doc, &content, &flat_resources, &mut res_cache, GraphicsState::new(), &mut objects, &mut text_runs, &mut obj_counter)?;

    Ok((objects, text_runs, width, height))
}

pub fn parse_content_stream(
    doc: &Document,
    content: &Content,
    flat_resources: &FlatResources,
    res_cache: &mut ResourceCache,
    mut state: GraphicsState,
    objects: &mut Vec<RenderObject>,
    text_runs: &mut Vec<StyledRun>,
    obj_counter: &mut usize,
) -> Result<(), String> {
    let mut state_stack = Vec::new();
    let mut current_segments = Vec::new();

    for op in &content.operations {
        let op_str = op.operator.as_str();
        match op_str {
            "q" => state_stack.push(state.clone()),
            "Q" => { if let Some(s) = state_stack.pop() { state = s; } }
            "cm" => {
                if let Ok(m) = operands_to_f32(&op.operands) {
                    if m.len() == 6 {
                        state.ctm = multiply_matrices(state.ctm, [m[0], m[1], m[2], m[3], m[4], m[5]]);
                    }
                }
            }
            "w" => { if let Some(w) = op.operands.get(0).and_then(|o| o.as_float().ok()) { state.line_width = w; } }
            "j" => { if let Some(v) = op.operands.get(0).and_then(|o| o.as_i64().ok()) { state.line_join = v as u8; } }
            "J" => { if let Some(v) = op.operands.get(0).and_then(|o| o.as_i64().ok()) { state.line_cap = v as u8; } }
            "M" => { if let Some(v) = op.operands.get(0).and_then(|o| o.as_float().ok()) { state.miter_limit = v; } }
            "rg" | "sc" | "scn" => {
                if let Ok(p) = operands_to_f32(&op.operands) {
                    if p.len() >= 3 {
                        state.fill_color = Some(format!("#{:02x}{:02x}{:02x}", (p[0]*255.0) as u8, (p[1]*255.0) as u8, (p[2]*255.0) as u8));
                    }
                }
            }
            "RG" | "SC" | "SCN" => {
                if let Ok(p) = operands_to_f32(&op.operands) {
                    if p.len() >= 3 {
                        state.stroke_color = Some(format!("#{:02x}{:02x}{:02x}", (p[0]*255.0) as u8, (p[1]*255.0) as u8, (p[2]*255.0) as u8));
                    }
                }
            }
            // Alpha (transparency) operators - critical for correct PDF rendering
            "ca" => {
                if let Some(v) = op.operands.get(0).and_then(|o| o.as_float().ok().or_else(|| o.as_i64().ok().map(|i| i as f32))) {
                    state.fill_alpha = v.clamp(0.0, 1.0);
                }
            }
            "CA" => {
                if let Some(v) = op.operands.get(0).and_then(|o| o.as_float().ok().or_else(|| o.as_i64().ok().map(|i| i as f32))) {
                    state.stroke_alpha = v.clamp(0.0, 1.0);
                }
            }
            // Named graphics state - look up ExtGState dictionary for ca/CA values
            "gs" => {
                if let Some(name) = op.operands.get(0).and_then(|o| o.as_name().ok()) {
                    if let Some(extgstate_id) = flat_resources.get(b"ExtGState" as &[u8]).and_then(|m| m.get(name)) {
                        if let Ok(dict) = doc.get_dictionary(*extgstate_id) {
                            if let Ok(ca) = dict.get(b"ca").and_then(|o| o.as_float().or_else(|_| o.as_i64().map(|i| i as f32))) {
                                state.fill_alpha = ca.clamp(0.0, 1.0);
                            }
                            if let Ok(ca_upper) = dict.get(b"CA").and_then(|o| o.as_float().or_else(|_| o.as_i64().map(|i| i as f32))) {
                                state.stroke_alpha = ca_upper.clamp(0.0, 1.0);
                            }
                        }
                    }
                }
            }
            // Clipping path operators: don't paint, just hint the clip. The path stays
            // for the next painting operator (which may be `n` = no-op). Without `n`,
            // segments leaked into following draw operators creating spurious giant rectangles.
            "W" | "W*" => { /* clipping intent only - keep segments for next op */ }
            "n" => { current_segments.clear(); }
            "m" => {
                if let Ok(p) = operands_to_f32(&op.operands) {
                    if p.len() >= 2 {
                        let pt = state.transform_point(p[0], p[1]);
                        current_segments.push(PathSegment { command: "move".into(), points: vec![[pt[0], pt[1]]] });
                    }
                }
            }
            "l" => {
                if let Ok(p) = operands_to_f32(&op.operands) {
                    if p.len() >= 2 {
                        let pt = state.transform_point(p[0], p[1]);
                        current_segments.push(PathSegment { command: "line".into(), points: vec![[pt[0], pt[1]]] });
                    }
                }
            }
            "h" => current_segments.push(PathSegment { command: "close".into(), points: vec![] }),
            "re" => {
                if let Ok(p) = operands_to_f32(&op.operands) {
                    if p.len() >= 4 {
                        let (x, y, w, h) = (p[0], p[1], p[2], p[3]);
                        let p1 = state.transform_point(x, y);
                        let p2 = state.transform_point(x+w, y);
                        let p3 = state.transform_point(x+w, y+h);
                        let p4 = state.transform_point(x, y+h);
                        current_segments.push(PathSegment { command: "move".into(), points: vec![[p1[0], p1[1]]] });
                        current_segments.push(PathSegment { command: "line".into(), points: vec![[p2[0], p2[1]]] });
                        current_segments.push(PathSegment { command: "line".into(), points: vec![[p3[0], p3[1]]] });
                        current_segments.push(PathSegment { command: "line".into(), points: vec![[p4[0], p4[1]]] });
                        current_segments.push(PathSegment { command: "close".into(), points: vec![] });
                    }
                }
            }
            "BT" => { state.tm = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]; state.tlm = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]; }
            "Tf" => {
                if let Some(name) = op.operands.get(0).and_then(|o| o.as_name().ok()) {
                    let size = op.operands.get(1).and_then(|o| o.as_float().ok().or_else(|| o.as_i64().ok().map(|i| i as f32))).unwrap_or(state.font_size);
                    state.font_size = size;
                    if let Some(font_id) = flat_resources.get(b"Font" as &[u8]).and_then(|m| m.get(name)) {
                        if let Some(cached) = res_cache.fonts.get(font_id) { state.current_font = Some(cached.clone()); }
                        else if let Ok(parsed) = crate::infrastructure::pdf::pdf_font::parse_font_from_dict(doc, *font_id, name) {
                            let arc = Arc::new(parsed);
                            res_cache.fonts.insert(*font_id, arc.clone());
                            state.current_font = Some(arc);
                        }
                    }
                }
            }
            "Td" => {
                if let Ok(p) = operands_to_f32(&op.operands) {
                    if p.len() >= 2 {
                        state.tlm = multiply_matrices(state.tlm, [1.0, 0.0, 0.0, 1.0, p[0], p[1]]);
                        state.tm = state.tlm;
                    }
                }
            }
            "Tm" => {
                if let Ok(m) = operands_to_f32(&op.operands) {
                    if m.len() >= 6 { state.tm = [m[0], m[1], m[2], m[3], m[4], m[5]]; state.tlm = state.tm; }
                }
            }
            "Tj" | "TJ" => {
                *obj_counter += 1;
                if let Some(ref font) = state.current_font {
                    let h_scale = state.horizontal_scaling / 100.0;
                    let (text, origins, widths, codes, advance) = if op_str == "Tj" {
                        resolve_glyph_geom(op.operands[0].as_str().unwrap_or(&[]), font, state.font_size, h_scale, state.char_spacing, state.word_spacing)
                    } else {
                        let mut combined = String::new();
                        let mut all_origins = Vec::new();
                        let mut all_widths = Vec::new();
                        let mut all_codes = Vec::new();
                        let mut offset = 0.0;
                        if let Ok(arr) = op.operands[0].as_array() {
                            for item in arr {
                                if let Ok(bytes) = item.as_str() {
                                    let (t, o, w, c, adv) = resolve_glyph_geom(bytes, font, state.font_size, h_scale, state.char_spacing, state.word_spacing);
                                    for ori in o { all_origins.push(offset + ori); }
                                    all_widths.extend(w);
                                    all_codes.extend(c);
                                    combined.push_str(&t);
                                    offset += adv;
                                } else if let Ok(kern) = item.as_float().or_else(|_| item.as_i64().map(|i| i as f32)) {
                                    offset -= (kern / 1000.0) * state.font_size * h_scale;
                                }
                            }
                        }
                        (combined, all_origins, all_widths, all_codes, offset)
                    };

                    let trm = multiply_matrices(state.ctm, state.tm);
                    text_runs.push(StyledRun {
                        text, tx: trm[4], ty: trm[5], width: advance,
                        font_size: (state.font_size * trm[3]).abs(),
                        font_name: font.name.clone(),
                        char_origins: origins, char_widths: widths, pdf_char_codes: codes,
                        z_index: *obj_counter,
                        color: state.fill_color.clone().unwrap_or("#000000".into()),
                        ..Default::default()
                    });
                    state.tm = multiply_matrices(state.tm, [1.0, 0.0, 0.0, 1.0, advance, 0.0]);
                }
            }
            "S" | "s" | "f" | "F" | "f*" | "B" | "b" | "B*" | "b*" => {
                *obj_counter += 1;
                if !current_segments.is_empty() {
                    let fill = op_str.to_lowercase().contains('f') || op_str.to_lowercase().contains('b');
                    let stroke = op_str.to_lowercase().contains('s') || op_str.to_lowercase().contains('b');

                    // Apply alpha into the color hex (CSS supports 8-digit #rrggbbaa).
                    // Default alpha is 1.0 - we only append the alpha byte when < 1.0
                    // to keep existing colors unchanged for fully-opaque paths.
                    fn with_alpha(color: &Option<String>, alpha: f32) -> Option<String> {
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
                    let final_fill_color = with_alpha(&state.fill_color, state.fill_alpha);
                    let final_stroke_color = with_alpha(&state.stroke_color, state.stroke_alpha);

                    // DBG: print path bbox, color and alpha
                    let mut min_x = f32::INFINITY; let mut min_y = f32::INFINITY;
                    let mut max_x = f32::NEG_INFINITY; let mut max_y = f32::NEG_INFINITY;
                    for seg in &current_segments {
                        for pt in &seg.points {
                            if pt[0] < min_x { min_x = pt[0]; }
                            if pt[0] > max_x { max_x = pt[0]; }
                            if pt[1] < min_y { min_y = pt[1]; }
                            if pt[1] > max_y { max_y = pt[1]; }
                        }
                    }
                    log_step!(
                        "[PATH-DBG] op={} id=path_{} bbox=({:.1},{:.1})-({:.1},{:.1}) size={:.1}x{:.1} fill={:?} stroke={:?} fill_alpha={:.3} stroke_alpha={:.3} z={}",
                        op_str, *obj_counter, min_x, min_y, max_x, max_y,
                        max_x - min_x, max_y - min_y,
                        final_fill_color, final_stroke_color,
                        state.fill_alpha, state.stroke_alpha, *obj_counter
                    );
                    objects.push(RenderObject::Path(NativePathModel {
                        id: format!("path_{}", *obj_counter),
                        segments: simplify_path_segments(current_segments.drain(..).collect(), 0.1),
                        fill_color: if fill { final_fill_color } else { None },
                        stroke_color: if stroke { final_stroke_color } else { None },
                        fill, stroke, stroke_width: state.line_width, z_index: *obj_counter,
                        ..Default::default()
                    }));
                }
            }
            "Do" => {
                if let Some(name) = op.operands.get(0).and_then(|o| o.as_name().ok()) {
                    if let Some(xobjects) = flat_resources.get(b"XObject" as &[u8]) {
                        if let Some(id) = xobjects.get(name) {
                            if let Ok(stream) = doc.get_object(*id).and_then(|o| o.as_stream()) {
                                if stream.dict.get(b"Subtype").ok().and_then(|o| o.as_name().ok()) == Some(b"Form") {
                                    if let Ok(data) = stream.decompressed_content() {
                                        if let Ok(sub) = Content::decode(&data) {
                                            let sub_res = read_resources(doc, *id);
                                            let mut sub_state = state.clone();
                                            if let Ok(m_obj) = stream.dict.get(b"Matrix") {
                                                if let Ok(m_arr) = m_obj.as_array() {
                                                    if let Ok(m) = operands_to_f32(m_arr) {
                                                        if m.len() == 6 {
                                                            sub_state.ctm = multiply_matrices(state.ctm, [m[0], m[1], m[2], m[3], m[4], m[5]]);
                                                        }
                                                    }
                                                }
                                            }
                                            parse_content_stream(doc, &sub, &sub_res, res_cache, sub_state, objects, text_runs, obj_counter)?;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

// === Stub wrappers for renamed/relocated functions ===

pub fn extract_metadata(doc: &Document) -> Result<crate::infrastructure::pdf::models::PdfMetadata, String> {
    let mut meta = crate::infrastructure::pdf::models::PdfMetadata::default();

    if let Ok(info_dict) = doc.trailer.get(b"Info")
        .and_then(|o| o.as_reference())
        .and_then(|id| doc.get_object(id))
        .and_then(|o| o.as_dict())
    {
        if let Ok(t) = info_dict.get(b"Title").and_then(|o| o.as_string()) {
            meta.title = Some(t.to_string());
        }
        if let Ok(a) = info_dict.get(b"Author").and_then(|o| o.as_string()) {
            meta.author = Some(a.to_string());
        }
        if let Ok(s) = info_dict.get(b"Subject").and_then(|o| o.as_string()) {
            meta.subject = Some(s.to_string());
        }
    }

    meta.page_count = doc.get_pages().len();
    Ok(meta)
}

pub fn get_page_count(doc: &Document) -> Result<u32, String> {
    Ok(doc.get_pages().len() as u32)
}

pub fn extract_page_bbox(doc: &Document, page_index: u16) -> Result<[f32; 4], String> {
    let pages = doc.get_pages();
    let page_id = pages.get(&(page_index as u32 + 1))
        .copied()
        .ok_or_else(|| format!("Page {} not found", page_index))?;
    let page_dict = doc.get_dictionary(page_id)
        .map_err(|e| e.to_string())?;
    let media = page_dict.get(b"MediaBox")
        .map_err(|_| "No MediaBox".to_string())?;
    let arr = media.as_array()
        .map_err(|e| e.to_string())?;
    let mut result = [0.0f32, 0.0, 595.0, 842.0];
    for (i, obj) in arr.iter().enumerate().take(4) {
        result[i] = obj.as_float().or_else(|_| obj.as_i64().map(|v| v as f32)).unwrap_or(result[i]);
    }
    Ok(result)
}

pub fn extract_vector_page_model(doc: &Document, page_index: u16) -> Result<crate::infrastructure::pdf::models::VectorPageModel, String> {
    crate::infrastructure::pdf::vector_engine::get_vector_page_model_with_doc(doc, page_index)
}

pub fn extract_layout_inference(doc: &Document, page_index: u16) -> Result<crate::infrastructure::pdf::models::LayoutInferenceResult, String> {
    crate::infrastructure::pdf::vector_engine::get_layout_inference(doc, page_index)
}

pub fn extract_glyph_paint_plan(doc: &Document, page_index: u16) -> Result<crate::infrastructure::pdf::models::GlyphPaintPlan, String> {
    log_step!("[PDF][extract_glyph_paint_plan] Extracting glyph paint plan for page {}", page_index);

    let pages = doc.get_pages();
    let page_id = pages.get(&(page_index as u32 + 1))
        .copied()
        .ok_or_else(|| format!("Page {} not found", page_index))?;
    let page_dict = doc.get_dictionary(page_id)
        .map_err(|e| e.to_string())?;

    let (width, height) = if let Ok(box_obj) = page_dict.get(b"MediaBox") {
        if let Ok(arr) = box_obj.as_array() {
            if arr.len() >= 4 {
                let w = arr[2].as_float().or_else(|_| arr[2].as_i64().map(|v| v as f32)).unwrap_or(595.0);
                let h = arr[3].as_float().or_else(|_| arr[3].as_i64().map(|v| v as f32)).unwrap_or(842.0);
                let y0 = arr[1].as_float().or_else(|_| arr[1].as_i64().map(|v| v as f32)).unwrap_or(0.0);
                (w, h - y0)
            } else { (595.0, 842.0) }
        } else { (595.0, 842.0) }
    } else { (595.0, 842.0) };

    let mut glyph_paint_plan = crate::infrastructure::pdf::models::GlyphPaintPlan {
        page_index,
        width,
        height,
        ..Default::default()
    };

    let content_data = doc.get_page_content(page_id)
        .map_err(|e| format!("Failed to get page content: {}", e))?;
    let content_str = String::from_utf8_lossy(&content_data);

    if content_str.contains("BT") && content_str.contains("ET") {
        log_step!("[PDF][extract_glyph_paint_plan] Found text objects in page {}", page_index);
    }

    log_step!("[PDF][extract_glyph_paint_plan] Successfully extracted glyph paint plan for page {}", page_index);
    Ok(glyph_paint_plan)
}
