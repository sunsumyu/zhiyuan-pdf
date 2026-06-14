use crate::infrastructure::pdf::models::*;
use crate::infrastructure::pdf::pdf_font::{
    resolve_glyph_geom, simplify_path_segments, ParsedFont, ResourceCache,
};
use lopdf::{content::Content, Document, Object};
use std::collections::HashMap;
use std::sync::Arc;

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
        let (a, b, c, d, e, f) = (
            self.ctm[0],
            self.ctm[1],
            self.ctm[2],
            self.ctm[3],
            self.ctm[4],
            self.ctm[5],
        );
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
            if let Ok(res_dict) = res_obj
                .as_dict()
                .or_else(|_| res_obj.as_reference().and_then(|r| doc.get_dictionary(r)))
            {
                for (cat_key, cat_val) in res_dict.iter() {
                    let cat_map = flat.entry(cat_key.clone()).or_insert_with(HashMap::new);
                    if let Ok(sub_dict) = cat_val
                        .as_dict()
                        .or_else(|_| cat_val.as_reference().and_then(|r| doc.get_dictionary(r)))
                    {
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
            if visited.contains(&parent_ref) {
                break;
            }
            visited.insert(parent_ref);
            curr_id = parent_ref;
        } else {
            break;
        }
    }
    flat
}

pub fn find_xobject_by_name(
    doc: &Document,
    flat_resources: &FlatResources,
    name: &[u8],
) -> Option<lopdf::ObjectId> {
    if let Some(xobjects) = flat_resources.get(b"XObject" as &[u8]) {
        if let Some(&id) = xobjects.get(name) {
            return Some(id);
        }
    }
    // Fallback: search all other pages' resources for this XObject name
    for (_, page_obj_id) in doc.get_pages() {
        let other_resources = read_resources(doc, page_obj_id);
        if let Some(xobjects) = other_resources.get(b"XObject" as &[u8]) {
            if let Some(&id) = xobjects.get(name) {
                return Some(id);
            }
        }
    }
    None
}

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
    let (a1, b1, c1, d1, e1, f1) = (new[0], new[1], new[2], new[3], new[4], new[5]);
    let (a2, b2, c2, d2, e2, f2) = (
        current[0], current[1], current[2], current[3], current[4], current[5],
    );
    [
        a1 * a2 + b1 * c2,
        a1 * b2 + b1 * d2,
        c1 * a2 + d1 * c2,
        c1 * b2 + d1 * d2,
        e1 * a2 + f1 * c2 + e2,
        e1 * b2 + f1 * d2 + f2,
    ]
}

lazy_static::lazy_static! {
    static ref PAGE_LOCKS: std::sync::Mutex<std::collections::HashMap<String, std::sync::Arc<std::sync::Mutex<()>>>> =
        std::sync::Mutex::new(std::collections::HashMap::new());
}

pub fn resolve_paths(
    doc: &Document,
    page_index: u32,
) -> Result<(Vec<RenderObject>, Vec<StyledRun>, f32, f32), String> {
    let doc_id = doc as *const Document as usize;
    let cache_key = format!("{}_{}", doc_id, page_index);

    // 1. Fast check without lock (for already-cached pages)
    if let Some(cached) = {
        let cache = crate::infrastructure::pdf::cache::PDF_RESOLVE_PATHS_CACHE
            .lock()
            .unwrap();
        cache.get(&cache_key).cloned()
    } {
        crate::log_step!(
            "[PDF-Vector][Cache] HIT for resolve_paths: key={}",
            cache_key
        );
        return Ok((*cached).clone());
    }

    // 2. Lock for this specific page to serialize concurrent duplicate requests
    let page_lock = {
        let mut locks = PAGE_LOCKS.lock().unwrap();
        locks
            .entry(cache_key.clone())
            .or_insert_with(|| std::sync::Arc::new(std::sync::Mutex::new(())))
            .clone()
    };

    let _guard = page_lock.lock().unwrap();

    // 3. Double-check cache inside the lock (in case another thread resolved it while we were waiting)
    if let Some(cached) = {
        let cache = crate::infrastructure::pdf::cache::PDF_RESOLVE_PATHS_CACHE
            .lock()
            .unwrap();
        cache.get(&cache_key).cloned()
    } {
        crate::log_step!(
            "[PDF-Vector][Cache] HIT (after lock wait) for resolve_paths: key={}",
            cache_key
        );
        return Ok((*cached).clone());
    }

    crate::log_audit!("[PDF-AUDIT] resolve_paths START page={}", page_index);
    let page_id = *doc
        .get_pages()
        .get(&(page_index + 1))
        .ok_or("Page not found")?;
    let page_dict = doc.get_dictionary(page_id).map_err(|e| e.to_string())?;

    let (mut width, mut height) = if let Ok(box_obj) = page_dict.get(b"MediaBox") {
        let arr = box_obj.as_array().map_err(|e| e.to_string())?;
        if arr.len() >= 4 {
            let w = arr[2]
                .as_float()
                .or_else(|_| arr[2].as_i64().map(|v| v as f32))
                .unwrap_or(595.0);
            let h = arr[3]
                .as_float()
                .or_else(|_| arr[3].as_i64().map(|v| v as f32))
                .unwrap_or(842.0);
            let y0 = arr[1]
                .as_float()
                .or_else(|_| arr[1].as_i64().map(|v| v as f32))
                .unwrap_or(0.0);
            ((w).abs(), (h - y0).abs())
        } else {
            (595.0, 842.0)
        }
    } else {
        (595.0, 842.0)
    };

    // Support inherited /Rotate attribute in page dictionary tree
    let mut rotation = 0i64;
    let mut current_id = page_id;
    while let Ok(dict) = doc.get_dictionary(current_id) {
        if let Ok(rotate_obj) = dict.get(b"Rotate") {
            if let Ok(r) = rotate_obj.as_i64() {
                rotation = r;
                break;
            }
        }
        if let Ok(parent_id) = dict.get(b"Parent").and_then(|o| o.as_reference()) {
            current_id = parent_id;
        } else {
            break;
        }
    }

    // Normalize rotation to 0, 90, 180, 270
    let normalized_rotation = ((rotation % 360) + 360) % 360;
    if normalized_rotation == 90 || normalized_rotation == 270 {
        std::mem::swap(&mut width, &mut height);
        crate::log_step!(
            "[PDF-Vector] swapped page size due to {} deg rotation. final={}x{}",
            normalized_rotation,
            width,
            height
        );
    }

    let flat_resources = read_resources(doc, page_id);
    let mut res_cache = ResourceCache::new();

    let content_data = doc.get_page_content(page_id).map_err(|e| e.to_string())?;
    crate::pdf_log!(
        3,
        "[PDF-DIAG] resolve_paths page={} content_bytes={}",
        page_index,
        content_data.len()
    );

    let content = Content::decode(&content_data).map_err(|e| e.to_string())?;
    crate::pdf_log!(
        3,
        "[PDF-DIAG] resolve_paths page={} ops_count={}",
        page_index,
        content.operations.len()
    );

    // Log first 20 operators for diagnostics
    let ops_preview: Vec<String> = content
        .operations
        .iter()
        .take(20)
        .map(|op| format!("{}({})", op.operator, op.operands.len()))
        .collect();
    crate::pdf_log!(
        3,
        "[PDF-DIAG] resolve_paths page={} first_ops={:?}",
        page_index,
        ops_preview
    );

    // Log XObject resource keys
    if let Some(xobjects) = flat_resources.get(b"XObject" as &[u8]) {
        let xobj_keys: Vec<String> = xobjects
            .keys()
            .map(|k| String::from_utf8_lossy(k).to_string())
            .collect();
        crate::pdf_log!(
            3,
            "[PDF-DIAG] resolve_paths page={} xobject_keys={:?}",
            page_index,
            xobj_keys
        );
    } else {
        crate::pdf_log!(
            3,
            "[PDF-DIAG] resolve_paths page={} NO XObject resources",
            page_index
        );
    }

    let mut objects = Vec::new();
    let mut text_runs = Vec::new();
    let mut obj_counter = 0;

    parse_content_stream(
        doc,
        &content,
        &flat_resources,
        &mut res_cache,
        GraphicsState::new(),
        &mut objects,
        &mut text_runs,
        &mut obj_counter,
    )?;

    crate::pdf_log!(
        2,
        "[PDF-DIAG] resolve_paths page={} RESULT objects={} text_runs={} w={} h={}",
        page_index,
        objects.len(),
        text_runs.len(),
        width,
        height
    );

    let res = (objects, text_runs, width, height);
    {
        let mut cache = crate::infrastructure::pdf::cache::PDF_RESOLVE_PATHS_CACHE
            .lock()
            .unwrap();
        cache.insert(cache_key, Arc::new(res.clone()));
    }
    Ok(res)
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
            "Q" => {
                if let Some(s) = state_stack.pop() {
                    state = s;
                }
            }
            "cm" => {
                if let Ok(m) = operands_to_f32(&op.operands) {
                    if m.len() == 6 {
                        state.ctm =
                            multiply_matrices(state.ctm, [m[0], m[1], m[2], m[3], m[4], m[5]]);
                    }
                }
            }
            "w" => {
                if let Some(w) = op.operands.get(0).and_then(|o| o.as_float().ok()) {
                    state.line_width = w;
                }
            }
            "j" => {
                if let Some(v) = op.operands.get(0).and_then(|o| o.as_i64().ok()) {
                    state.line_join = v as u8;
                }
            }
            "J" => {
                if let Some(v) = op.operands.get(0).and_then(|o| o.as_i64().ok()) {
                    state.line_cap = v as u8;
                }
            }
            "M" => {
                if let Some(v) = op.operands.get(0).and_then(|o| o.as_float().ok()) {
                    state.miter_limit = v;
                }
            }
            "rg" | "sc" | "scn" => {
                if let Ok(p) = operands_to_f32(&op.operands) {
                    if p.len() >= 3 {
                        state.fill_color = Some(format!(
                            "#{:02x}{:02x}{:02x}",
                            (p[0] * 255.0) as u8,
                            (p[1] * 255.0) as u8,
                            (p[2] * 255.0) as u8
                        ));
                    }
                }
            }
            "RG" | "SC" | "SCN" => {
                if let Ok(p) = operands_to_f32(&op.operands) {
                    if p.len() >= 3 {
                        state.stroke_color = Some(format!(
                            "#{:02x}{:02x}{:02x}",
                            (p[0] * 255.0) as u8,
                            (p[1] * 255.0) as u8,
                            (p[2] * 255.0) as u8
                        ));
                    }
                }
            }
            // Alpha (transparency) operators - critical for correct PDF rendering
            "ca" => {
                if let Some(v) = op.operands.get(0).and_then(|o| {
                    o.as_float()
                        .ok()
                        .or_else(|| o.as_i64().ok().map(|i| i as f32))
                }) {
                    state.fill_alpha = v.clamp(0.0, 1.0);
                }
            }
            "CA" => {
                if let Some(v) = op.operands.get(0).and_then(|o| {
                    o.as_float()
                        .ok()
                        .or_else(|| o.as_i64().ok().map(|i| i as f32))
                }) {
                    state.stroke_alpha = v.clamp(0.0, 1.0);
                }
            }
            // Named graphics state - look up ExtGState dictionary for ca/CA values
            "gs" => {
                if let Some(name) = op.operands.get(0).and_then(|o| o.as_name().ok()) {
                    if let Some(extgstate_id) = flat_resources
                        .get(b"ExtGState" as &[u8])
                        .and_then(|m| m.get(name))
                    {
                        if let Ok(dict) = doc.get_dictionary(*extgstate_id) {
                            if let Ok(ca) = dict.get(b"ca").and_then(|o| {
                                o.as_float().or_else(|_| o.as_i64().map(|i| i as f32))
                            }) {
                                state.fill_alpha = ca.clamp(0.0, 1.0);
                            }
                            if let Ok(ca_upper) = dict.get(b"CA").and_then(|o| {
                                o.as_float().or_else(|_| o.as_i64().map(|i| i as f32))
                            }) {
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
            "n" => {
                current_segments.clear();
            }
            "m" => {
                if let Ok(p) = operands_to_f32(&op.operands) {
                    if p.len() >= 2 {
                        let pt = state.transform_point(p[0], p[1]);
                        current_segments.push(PathSegment {
                            command: "move".into(),
                            points: vec![[pt[0], pt[1]]],
                        });
                    }
                }
            }
            "l" => {
                if let Ok(p) = operands_to_f32(&op.operands) {
                    if p.len() >= 2 {
                        let pt = state.transform_point(p[0], p[1]);
                        current_segments.push(PathSegment {
                            command: "line".into(),
                            points: vec![[pt[0], pt[1]]],
                        });
                    }
                }
            }
            "h" => current_segments.push(PathSegment {
                command: "close".into(),
                points: vec![],
            }),
            "re" => {
                if let Ok(p) = operands_to_f32(&op.operands) {
                    if p.len() >= 4 {
                        let (x, y, w, h) = (p[0], p[1], p[2], p[3]);
                        let p1 = state.transform_point(x, y);
                        let p2 = state.transform_point(x + w, y);
                        let p3 = state.transform_point(x + w, y + h);
                        let p4 = state.transform_point(x, y + h);
                        current_segments.push(PathSegment {
                            command: "move".into(),
                            points: vec![[p1[0], p1[1]]],
                        });
                        current_segments.push(PathSegment {
                            command: "line".into(),
                            points: vec![[p2[0], p2[1]]],
                        });
                        current_segments.push(PathSegment {
                            command: "line".into(),
                            points: vec![[p3[0], p3[1]]],
                        });
                        current_segments.push(PathSegment {
                            command: "line".into(),
                            points: vec![[p4[0], p4[1]]],
                        });
                        current_segments.push(PathSegment {
                            command: "close".into(),
                            points: vec![],
                        });
                    }
                }
            }
            "BT" => {
                state.tm = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
                state.tlm = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
            }
            "Tf" => {
                if let Some(name) = op.operands.get(0).and_then(|o| o.as_name().ok()) {
                    let size = op
                        .operands
                        .get(1)
                        .and_then(|o| {
                            o.as_float()
                                .ok()
                                .or_else(|| o.as_i64().ok().map(|i| i as f32))
                        })
                        .unwrap_or(state.font_size);
                    state.font_size = size;
                    if let Some(font_id) = flat_resources
                        .get(b"Font" as &[u8])
                        .and_then(|m| m.get(name))
                    {
                        if let Some(cached) = res_cache.fonts.get(font_id) {
                            state.current_font = Some(cached.clone());
                        } else if let Ok(parsed) =
                            crate::infrastructure::pdf::pdf_font::parse_font_from_dict(
                                doc, *font_id, name,
                            )
                        {
                            let arc = Arc::new(parsed);
                            res_cache.fonts.insert(*font_id, arc.clone());
                            state.current_font = Some(arc);
                        }
                    }
                }
            }
            "Tr" => {
                if let Some(v) = op.operands.get(0).and_then(|o| o.as_i64().ok()) {
                    state.render_mode = v;
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
                    if m.len() >= 6 {
                        state.tm = [m[0], m[1], m[2], m[3], m[4], m[5]];
                        state.tlm = state.tm;
                    }
                }
            }
            "Tj" | "TJ" => {
                *obj_counter += 1;
                if let Some(ref font) = state.current_font {
                    let h_scale = state.horizontal_scaling / 100.0;
                    let (text, origins, widths, codes, advance) = if op_str == "Tj" {
                        resolve_glyph_geom(
                            op.operands[0].as_str().unwrap_or(&[]),
                            font,
                            state.font_size,
                            h_scale,
                            state.char_spacing,
                            state.word_spacing,
                        )
                    } else {
                        let mut combined = String::new();
                        let mut all_origins = Vec::new();
                        let mut all_widths = Vec::new();
                        let mut all_codes = Vec::new();
                        let mut offset = 0.0;
                        if let Ok(arr) = op.operands[0].as_array() {
                            for item in arr {
                                if let Ok(bytes) = item.as_str() {
                                    let (t, o, w, c, adv) = resolve_glyph_geom(
                                        bytes,
                                        font,
                                        state.font_size,
                                        h_scale,
                                        state.char_spacing,
                                        state.word_spacing,
                                    );
                                    for ori in o {
                                        all_origins.push(offset + ori);
                                    }
                                    all_widths.extend(w);
                                    all_codes.extend(c);
                                    combined.push_str(&t);
                                    offset += adv;
                                } else if let Ok(kern) =
                                    item.as_float().or_else(|_| item.as_i64().map(|i| i as f32))
                                {
                                    offset -= (kern / 1000.0) * state.font_size * h_scale;
                                }
                            }
                        }
                        (combined, all_origins, all_widths, all_codes, offset)
                    };

                    let trm = multiply_matrices(state.ctm, state.tm);
                    // char_origins and char_widths from resolve_glyph_geom are in TEXT
                    // SPACE (pre-matrix).  The rest of the pipeline (LayoutRun, caret
                    // stops, overlay rendering) expects PAGE SPACE values.  Scale by the
                    // horizontal component of the text rendering matrix to bridge the gap.
                    let h_page_scale = trm[0].abs().max(f32::EPSILON);
                    let page_origins: Vec<f32> = origins.iter().map(|o| o * h_page_scale).collect();
                    let page_widths: Vec<f32> = widths.iter().map(|w| w * h_page_scale).collect();
                    let page_advance = advance * h_page_scale;
                    text_runs.push(StyledRun {
                        text,
                        tx: trm[4],
                        ty: trm[5],
                        width: page_advance,
                        font_size: (state.font_size * trm[3]).abs(),
                        font_name: font.name.clone(),
                        char_origins: page_origins,
                        char_widths: page_widths,
                        pdf_char_codes: codes,
                        z_index: *obj_counter,
                        color: state.fill_color.clone().unwrap_or("#000000".into()),
                        a: trm[0],
                        b: trm[1],
                        c: trm[2],
                        d: trm[3],
                        horizontal_scaling: state.horizontal_scaling,
                        char_spacing: state.char_spacing,
                        word_spacing: state.word_spacing,
                        render_mode: state.render_mode,
                        ..Default::default()
                    });
                    // Tm update uses the original text-space advance (not page-scaled)
                    state.tm = multiply_matrices(state.tm, [1.0, 0.0, 0.0, 1.0, advance, 0.0]);
                }
            }
            "S" | "s" | "f" | "F" | "f*" | "B" | "b" | "B*" | "b*" => {
                *obj_counter += 1;
                if !current_segments.is_empty() {
                    let fill =
                        op_str.to_lowercase().contains('f') || op_str.to_lowercase().contains('b');
                    let stroke =
                        op_str.to_lowercase().contains('s') || op_str.to_lowercase().contains('b');

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
                    let mut min_x = f32::INFINITY;
                    let mut min_y = f32::INFINITY;
                    let mut max_x = f32::NEG_INFINITY;
                    let mut max_y = f32::NEG_INFINITY;
                    for seg in &current_segments {
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
                    crate::pdf_log!(
                        3,
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
                        fill,
                        stroke,
                        stroke_width: state.line_width,
                        z_index: *obj_counter,
                        ..Default::default()
                    }));
                }
            }
            "Do" => {
                if let Some(name) = op.operands.get(0).and_then(|o| o.as_name().ok()) {
                    crate::pdf_log!(
                        3,
                        "[PDF-DIAG][Do] operator name={:?}",
                        String::from_utf8_lossy(name)
                    );
                    if let Some(id) = find_xobject_by_name(doc, flat_resources, name) {
                        crate::pdf_log!(3, "[PDF-DIAG][Do] found XObject id={:?}", id);
                        if let Ok(stream) = doc.get_object(id).and_then(|o| o.as_stream()) {
                                let subtype = stream
                                    .dict
                                    .get(b"Subtype")
                                    .ok()
                                    .and_then(|o| o.as_name().ok());
                                crate::pdf_log!(
                                    3,
                                    "[PDF-DIAG][Do] subtype={:?} keys={:?}",
                                    subtype.map(|s| String::from_utf8_lossy(s).into_owned()),
                                    stream
                                        .dict
                                        .iter()
                                        .map(|(k, _)| String::from_utf8_lossy(k).to_string())
                                        .collect::<Vec<_>>()
                                );
                                if subtype == Some(b"Form") {
                                    if let Ok(data) = stream.decompressed_content() {
                                        crate::pdf_log!(
                                            3,
                                            "[PDF-DIAG][Do] Form content bytes={} name={:?}",
                                            data.len(),
                                            String::from_utf8_lossy(name)
                                        );
                                        if let Ok(sub) = Content::decode(&data) {
                                            let sub_res = read_resources(doc, id);
                                            let mut sub_state = state.clone();
                                            if let Ok(m_obj) = stream.dict.get(b"Matrix") {
                                                if let Ok(m_arr) = m_obj.as_array() {
                                                    if let Ok(m) = operands_to_f32(m_arr) {
                                                        if m.len() == 6 {
                                                            sub_state.ctm = multiply_matrices(
                                                                state.ctm,
                                                                [
                                                                    m[0], m[1], m[2], m[3], m[4],
                                                                    m[5],
                                                                ],
                                                            );
                                                        }
                                                    }
                                                }
                                            }
                                            parse_content_stream(
                                                doc,
                                                &sub,
                                                &sub_res,
                                                res_cache,
                                                sub_state,
                                                objects,
                                                text_runs,
                                                obj_counter,
                                            )?;
                                        }
                                    }
                                } else if subtype == Some(b"Image") {
                                    *obj_counter += 1;
                                    let img_w = stream
                                        .dict
                                        .get(b"Width")
                                        .and_then(|o| o.as_i64())
                                        .unwrap_or(0);
                                    let img_h = stream
                                        .dict
                                        .get(b"Height")
                                        .and_then(|o| o.as_i64())
                                        .unwrap_or(0);
                                    crate::pdf_log!(3, "[PDF-DIAG][Do-Image] name={:?} width={} height={} filters={:?}", String::from_utf8_lossy(name), img_w, img_h, stream.dict.get(b"Filter").ok());
                                    if img_w > 0 && img_h > 0 {
                                        let filter_name =
                                            stream.dict.get(b"Filter").ok().and_then(|o| {
                                                o.as_name().ok().map(|n| n.to_vec()).or_else(|| {
                                                    o.as_array()
                                                        .ok()?
                                                        .first()?
                                                        .as_name()
                                                        .ok()
                                                        .map(|n| n.to_vec())
                                                })
                                            });
                                        let is_jpeg = filter_name.as_deref() == Some(b"DCTDecode");
                                        let img_data: Option<Arc<[u8]>> = if is_jpeg {
                                            crate::pdf_log!(
                                                3,
                                                "[PDF-DIAG][Do-Image] JPEG content_len={}",
                                                stream.content.len()
                                            );
                                            Some(Arc::from(stream.content.as_slice()))
                                        } else {
                                            crate::pdf_log!(3, "[PDF-DIAG][Do-Image] Non-JPEG, attempting JPEG encode filter={:?}", filter_name.as_deref().map(|s| String::from_utf8_lossy(s).into_owned()));
                                            // Non-JPEG: decompress raw samples and encode as JPEG
                                            build_image_as_jpeg(
                                                doc,
                                                stream,
                                                img_w as u32,
                                                img_h as u32,
                                            )
                                        };
                                        if let Some(data) = img_data {
                                            if !data.is_empty() {
                                                let asset_id = ::uuid::Uuid::new_v4().to_string();
                                                {
                                                    let mut cache = crate::infrastructure::pdf::cache::PDF_IMAGE_CACHE.lock().unwrap();
                                                    cache.insert(asset_id.clone(), data);
                                                }
                                                let ctm = state.ctm;
                                                crate::pdf_log!(
                                                    3,
                                                    "[PDF-IMG] Do image id={} {}x{} ctm=[{:.1},{:.1},{:.1},{:.1},{:.1},{:.1}] jpeg={}",
                                                    asset_id, img_w, img_h, ctm[0], ctm[1], ctm[2], ctm[3], ctm[4], ctm[5], is_jpeg
                                                );
                                                let corners = [
                                                    state.transform_point(0.0, 0.0),
                                                    state.transform_point(1.0, 0.0),
                                                    state.transform_point(0.0, 1.0),
                                                    state.transform_point(1.0, 1.0),
                                                ];
                                                let min_x = corners
                                                    .iter()
                                                    .map(|p| p[0])
                                                    .fold(f32::INFINITY, f32::min);
                                                let max_x = corners
                                                    .iter()
                                                    .map(|p| p[0])
                                                    .fold(f32::NEG_INFINITY, f32::max);
                                                let min_y = corners
                                                    .iter()
                                                    .map(|p| p[1])
                                                    .fold(f32::INFINITY, f32::min);
                                                let max_y = corners
                                                    .iter()
                                                    .map(|p| p[1])
                                                    .fold(f32::NEG_INFINITY, f32::max);
                                                objects.push(RenderObject::Image(NativeImageModel {
                                                    data_url: format!("http://pdfasset.localhost/{}", asset_id),
                                                    id: asset_id,
                                                    x: min_x,
                                                    y: min_y,
                                                    width: (max_x - min_x).abs(),
                                                    height: (max_y - min_y).abs(),
                                                    a: ctm[0],
                                                    b: ctm[1],
                                                    c: ctm[2],
                                                    d: ctm[3],
                                                    e: ctm[4],
                                                    f: ctm[5],
                                                    z_index: *obj_counter,
                                                    extraction_method: if is_jpeg {
                                                        "JPEG".into()
                                                    } else {
                                                        "PNG".into()
                                                    },
                                                }));
                                            } else {
                                                crate::pdf_log!(3, "[PDF-DIAG][Do-Image] image data empty name={:?}", String::from_utf8_lossy(name));
                                            }
                                        } else {
                                            crate::pdf_log!(3, "[PDF-DIAG][Do-Image] failed to extract image data name={:?}", String::from_utf8_lossy(name));
                                        }
                                    } else {
                                        crate::pdf_log!(3, "[PDF-DIAG][Do-Image] invalid dimensions name={:?} w={} h={}", String::from_utf8_lossy(name), img_w, img_h);
                                    }
                                } else {
                                    crate::pdf_log!(
                                        3,
                                        "[PDF-DIAG][Do] Unsupported subtype={:?}",
                                        subtype.map(|s| String::from_utf8_lossy(s).into_owned())
                                    );
                            }
                        } else {
                            crate::pdf_log!(
                                3,
                                "[PDF-DIAG][Do] name={:?} not found in XObject resources",
                                String::from_utf8_lossy(name)
                            );
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

pub fn extract_metadata(
    doc: &Document,
) -> Result<crate::infrastructure::pdf::models::PdfMetadata, String> {
    let mut meta = crate::infrastructure::pdf::models::PdfMetadata::default();

    if let Ok(info_dict) = doc
        .trailer
        .get(b"Info")
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

pub fn read_page_count(doc: &Document) -> Result<u32, String> {
    Ok(doc.get_pages().len() as u32)
}

pub fn extract_page_bbox(doc: &Document, page_index: u16) -> Result<[f32; 4], String> {
    let pages = doc.get_pages();
    let page_id = pages
        .get(&(page_index as u32 + 1))
        .copied()
        .ok_or_else(|| format!("Page {} not found", page_index))?;
    let page_dict = doc.get_dictionary(page_id).map_err(|e| e.to_string())?;
    let media = page_dict
        .get(b"MediaBox")
        .map_err(|_| "No MediaBox".to_string())?;
    let arr = media.as_array().map_err(|e| e.to_string())?;
    let mut result = [0.0f32, 0.0, 595.0, 842.0];
    for (i, obj) in arr.iter().enumerate().take(4) {
        result[i] = obj
            .as_float()
            .or_else(|_| obj.as_i64().map(|v| v as f32))
            .unwrap_or(result[i]);
    }
    Ok(result)
}

pub fn extract_vector_page_model(
    doc: &Document,
    page_index: u16,
) -> Result<crate::infrastructure::pdf::models::NativeVectorPageModel, String> {
    crate::infrastructure::pdf::vector_engine::resolve_model(doc, page_index)
}

pub fn extract_layout_inference(
    doc: &Document,
    page_index: u16,
) -> Result<crate::infrastructure::pdf::models::LayoutInferenceResult, String> {
    crate::infrastructure::pdf::vector_engine::resolve_layout_inference(doc, page_index)
}

pub fn extract_glyph_paint_plan(
    doc: &Document,
    page_index: u16,
) -> Result<crate::infrastructure::pdf::models::GlyphPaintPlan, String> {
    crate::log_step!(
        "[PDF][extract_glyph_paint_plan] Extracting glyph paint plan for page {}",
        page_index
    );

    let pages = doc.get_pages();
    let page_id = pages
        .get(&(page_index as u32 + 1))
        .copied()
        .ok_or_else(|| format!("Page {} not found", page_index))?;
    let page_dict = doc.get_dictionary(page_id).map_err(|e| e.to_string())?;

    let (width, height) = if let Ok(box_obj) = page_dict.get(b"MediaBox") {
        if let Ok(arr) = box_obj.as_array() {
            if arr.len() >= 4 {
                let w = arr[2]
                    .as_float()
                    .or_else(|_| arr[2].as_i64().map(|v| v as f32))
                    .unwrap_or(595.0);
                let h = arr[3]
                    .as_float()
                    .or_else(|_| arr[3].as_i64().map(|v| v as f32))
                    .unwrap_or(842.0);
                let y0 = arr[1]
                    .as_float()
                    .or_else(|_| arr[1].as_i64().map(|v| v as f32))
                    .unwrap_or(0.0);
                (w, h - y0)
            } else {
                (595.0, 842.0)
            }
        } else {
            (595.0, 842.0)
        }
    } else {
        (595.0, 842.0)
    };

    let glyph_paint_plan = crate::infrastructure::pdf::models::GlyphPaintPlan {
        page_index,
        width,
        height,
        ..Default::default()
    };

    let content_data = doc
        .get_page_content(page_id)
        .map_err(|e| format!("Failed to get page content: {}", e))?;
    let content_str = String::from_utf8_lossy(&content_data);

    if content_str.contains("BT") && content_str.contains("ET") {
        crate::log_step!(
            "[PDF][extract_glyph_paint_plan] Found text objects in page {}",
            page_index
        );
    }

    crate::log_step!(
        "[PDF][extract_glyph_paint_plan] Successfully extracted glyph paint plan for page {}",
        page_index
    );
    Ok(glyph_paint_plan)
}

/// Apply PNG-style predictor (PDF Predictor values 10-15) to unfilter the data.
/// Each row begins with a filter type byte (0=None, 1=Sub, 2=Up, 3=Average, 4=Paeth).
fn apply_png_predictor(raw: &[u8], bytes_per_row: usize, bpp: usize) -> Option<Vec<u8>> {
    let row_with_filter = bytes_per_row + 1;
    if raw.is_empty() || raw.len() % row_with_filter != 0 {
        // Try to be lenient - PDFs sometimes have extra/missing bytes
        if raw.len() < row_with_filter {
            return None;
        }
    }
    let bpp = bpp.max(1);
    let rows = raw.len() / row_with_filter;
    let mut out = vec![0u8; rows * bytes_per_row];
    let mut prev_row = vec![0u8; bytes_per_row];
    for r in 0..rows {
        let row_start = r * row_with_filter;
        if row_start + row_with_filter > raw.len() {
            break;
        }
        let filter = raw[row_start];
        let row_data = &raw[row_start + 1..row_start + row_with_filter];
        let out_row_start = r * bytes_per_row;
        let cur_row = &mut out[out_row_start..out_row_start + bytes_per_row];
        for c in 0..bytes_per_row {
            let left = if c < bpp { 0 } else { cur_row[c - bpp] };
            let up = prev_row[c];
            let up_left = if c < bpp { 0 } else { prev_row[c - bpp] };
            let val = match filter {
                0 => row_data[c],                                                     // None
                1 => row_data[c].wrapping_add(left),                                  // Sub
                2 => row_data[c].wrapping_add(up),                                    // Up
                3 => row_data[c].wrapping_add(((left as u16 + up as u16) / 2) as u8), // Average
                4 => {
                    // Paeth
                    let a = left as i32;
                    let b = up as i32;
                    let c2 = up_left as i32;
                    let p = a + b - c2;
                    let pa = (p - a).abs();
                    let pb = (p - b).abs();
                    let pc = (p - c2).abs();
                    let predictor = if pa <= pb && pa <= pc {
                        a
                    } else if pb <= pc {
                        b
                    } else {
                        c2
                    };
                    row_data[c].wrapping_add(predictor as u8)
                }
                _ => row_data[c],
            };
            cur_row[c] = val;
        }
        prev_row.copy_from_slice(cur_row);
    }
    Some(out)
}

/// Read DecodeParms dictionary and extract Predictor / Columns / Colors / BitsPerComponent.
/// Accepts `doc` to resolve indirect references in the DecodeParms entry.
fn read_decode_params(doc: &lopdf::Document, stream: &lopdf::Stream) -> (i64, i64, i64, i64) {
    let mut predictor = 1i64;
    let mut columns = 1i64;
    let mut colors = 1i64;
    let mut bpc = 8i64;
    if let Ok(obj) = stream.dict.get(b"DecodeParms") {
        // Resolve if it's an indirect reference
        let resolved = match obj {
            lopdf::Object::Reference(r) => doc.get_object(*r).ok(),
            other => Some(other),
        };
        let dict_opt = resolved.and_then(|o| {
            o.as_dict().ok().cloned().or_else(|| {
                o.as_array().ok()?.first().and_then(|item| match item {
                    lopdf::Object::Reference(r) => doc.get_object(*r).ok()?.as_dict().ok().cloned(),
                    other => other.as_dict().ok().cloned(),
                })
            })
        });
        if let Some(dict) = dict_opt {
            if let Ok(v) = dict.get(b"Predictor").and_then(|o| o.as_i64()) {
                predictor = v;
            }
            if let Ok(v) = dict.get(b"Columns").and_then(|o| o.as_i64()) {
                columns = v;
            }
            if let Ok(v) = dict.get(b"Colors").and_then(|o| o.as_i64()) {
                colors = v;
            }
            if let Ok(v) = dict.get(b"BitsPerComponent").and_then(|o| o.as_i64()) {
                bpc = v;
            }
        }
    }
    (predictor, columns, colors, bpc)
}

/// Manually decompress FlateDecode data using flate2 as fallback when lopdf fails.
fn manual_flate_decompress(compressed: &[u8]) -> Option<Vec<u8>> {
    use std::io::Read;
    // Try zlib (with header) first, then raw deflate
    if let Ok(decoder) = flate2::read::ZlibDecoder::new(compressed)
        .bytes()
        .collect::<Result<Vec<u8>, _>>()
    {
        if !decoder.is_empty() {
            return Some(decoder);
        }
    }
    // Fallback: raw deflate
    let mut decoder = flate2::read::DeflateDecoder::new(compressed);
    let mut out = Vec::new();
    if decoder.read_to_end(&mut out).is_ok() && !out.is_empty() {
        return Some(out);
    }
    None
}

/// Extract a non-JPEG image XObject's raw samples, convert to JPEG bytes.
/// Handles DeviceRGB, DeviceGray, CMYK, and FlateDecode with PNG/TIFF predictors.
pub(crate) fn build_image_as_jpeg(
    doc: &lopdf::Document,
    stream: &lopdf::Stream,
    w: u32,
    h: u32,
) -> Option<Arc<[u8]>> {
    // Try lopdf's built-in decompression first; fall back to manual flate2
    let raw_decompressed = match stream.decompressed_content() {
        Ok(d) => {
            crate::log_step!("[PDF-IMG] lopdf decompress OK len={}", d.len());
            d
        }
        Err(e) => {
            crate::log_step!(
                "[PDF-IMG] lopdf decompress failed: {} — trying manual flate2 on {} raw bytes",
                e,
                stream.content.len()
            );
            match manual_flate_decompress(&stream.content) {
                Some(d) => {
                    crate::log_step!("[PDF-IMG] manual flate2 OK len={}", d.len());
                    d
                }
                None => {
                    crate::log_step!("[PDF-IMG] manual flate2 also failed");
                    return None;
                }
            }
        }
    };
    if raw_decompressed.is_empty() {
        crate::log_step!("[PDF-IMG] decompressed empty");
        return None;
    }

    let cs_name = stream.dict.get(b"ColorSpace").ok().and_then(|o| {
        o.as_name().ok().map(|n| n.to_vec()).or_else(|| {
            o.as_array()
                .ok()?
                .first()?
                .as_name()
                .ok()
                .map(|n| n.to_vec())
        })
    });
    let bpc = stream
        .dict
        .get(b"BitsPerComponent")
        .and_then(|o| o.as_i64())
        .unwrap_or(8) as u32;
    let cs = cs_name.as_deref().unwrap_or(b"DeviceRGB");

    let (predictor, dp_columns, dp_colors, dp_bpc) = read_decode_params(doc, stream);
    crate::log_step!(
        "[PDF-IMG] decoding {}x{} cs={} bpc={} decompressed={} predictor={} dp_cols={} dp_colors={} dp_bpc={}",
        w, h, String::from_utf8_lossy(cs), bpc, raw_decompressed.len(),
        predictor, dp_columns, dp_colors, dp_bpc
    );

    // Apply predictor if needed
    let raw: Vec<u8> = if predictor >= 10 {
        let columns = if dp_columns > 0 {
            dp_columns as usize
        } else {
            w as usize
        };
        let colors = if dp_colors > 0 {
            dp_colors as usize
        } else {
            match cs {
                b"DeviceRGB" => 3,
                b"DeviceCMYK" => 4,
                _ => 1,
            }
        };
        let pbpc = if dp_bpc > 0 {
            dp_bpc as usize
        } else {
            bpc as usize
        };
        let bytes_per_row = (columns * colors * pbpc + 7) / 8;
        let bpp = ((colors * pbpc) + 7) / 8;
        match apply_png_predictor(&raw_decompressed, bytes_per_row, bpp) {
            Some(unfiltered) => {
                crate::log_step!(
                    "[PDF-IMG] PNG predictor applied: bytes_per_row={} bpp={} unfiltered_len={}",
                    bytes_per_row,
                    bpp,
                    unfiltered.len()
                );
                unfiltered
            }
            None => {
                crate::log_step!("[PDF-IMG] PNG predictor failed, using raw");
                raw_decompressed
            }
        }
    } else {
        raw_decompressed
    };

    let expected_len = match cs {
        b"DeviceRGB" => (w * h * 3 * bpc / 8) as usize,
        b"DeviceGray" => (w * h * bpc / 8) as usize,
        b"DeviceCMYK" => (w * h * 4 * bpc / 8) as usize,
        _ => (w * h * 3 * bpc / 8) as usize,
    };

    if raw.len() < expected_len {
        crate::log_step!(
            "[PDF-IMG] raw data too short after predictor: have={} expected={} cs={} bpc={} {}x{}",
            raw.len(),
            expected_len,
            String::from_utf8_lossy(cs),
            bpc,
            w,
            h
        );
        return None;
    }

    // Build RGBA buffer
    let pixel_count = (w * h) as usize;
    let mut rgba = vec![255u8; pixel_count * 4];
    match cs {
        b"DeviceRGB" if bpc == 8 => {
            for i in 0..pixel_count {
                rgba[i * 4] = raw[i * 3];
                rgba[i * 4 + 1] = raw[i * 3 + 1];
                rgba[i * 4 + 2] = raw[i * 3 + 2];
            }
        }
        b"DeviceGray" if bpc == 8 => {
            for i in 0..pixel_count {
                let g = raw[i];
                rgba[i * 4] = g;
                rgba[i * 4 + 1] = g;
                rgba[i * 4 + 2] = g;
            }
        }
        b"DeviceCMYK" if bpc == 8 => {
            for i in 0..pixel_count {
                let c = raw[i * 4] as f32 / 255.0;
                let m = raw[i * 4 + 1] as f32 / 255.0;
                let y = raw[i * 4 + 2] as f32 / 255.0;
                let k = raw[i * 4 + 3] as f32 / 255.0;
                rgba[i * 4] = (255.0 * (1.0 - c) * (1.0 - k)) as u8;
                rgba[i * 4 + 1] = (255.0 * (1.0 - m) * (1.0 - k)) as u8;
                rgba[i * 4 + 2] = (255.0 * (1.0 - y) * (1.0 - k)) as u8;
            }
        }
        b"DeviceGray" if bpc == 1 => {
            // 1-bit monochrome (common in B&W scans)
            for i in 0..pixel_count {
                let byte_idx = i / 8;
                let bit_idx = 7 - (i % 8);
                let g = if byte_idx < raw.len() && (raw[byte_idx] >> bit_idx) & 1 == 1 {
                    255u8
                } else {
                    0u8
                };
                rgba[i * 4] = g;
                rgba[i * 4 + 1] = g;
                rgba[i * 4 + 2] = g;
            }
        }
        _ => {
            // Best-effort: treat as RGB if enough data, else grayscale
            if raw.len() >= pixel_count * 3 {
                for i in 0..pixel_count {
                    rgba[i * 4] = raw[i * 3];
                    rgba[i * 4 + 1] = raw[i * 3 + 1];
                    rgba[i * 4 + 2] = raw[i * 3 + 2];
                }
            } else {
                for i in 0..pixel_count.min(raw.len()) {
                    rgba[i * 4] = raw[i];
                    rgba[i * 4 + 1] = raw[i];
                    rgba[i * 4 + 2] = raw[i];
                }
            }
        }
    }

    // Convert RGBA8 to RGB8 for JPEG encoding (JPEG has no Alpha channel)
    let mut rgb = Vec::with_capacity(pixel_count * 3);
    for i in 0..pixel_count {
        rgb.push(rgba[i * 4]);
        rgb.push(rgba[i * 4 + 1]);
        rgb.push(rgba[i * 4 + 2]);
    }

    // Encode to JPEG
    let mut jpeg_buf = Vec::new();
    {
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg_buf, 80);
        use image::ImageEncoder;
        if encoder
            .write_image(&rgb, w, h, image::ColorType::Rgb8)
            .is_err()
        {
            return None;
        }
    }
    Some(Arc::from(jpeg_buf.as_slice()))
}
