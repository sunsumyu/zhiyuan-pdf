//! Text reflow logic: content-stream walkers, pure helpers, and state tracking.
//!
//! Extracted from `pdf_write/mod.rs`. Contains the two recursive content-stream
//! walkers (`patch_content_recursive` for simple replacement, `patch_atomic_reflow_recursive`
//! for layout-aware reflow), the pure helpers they depend on, and the write-path
//! text-state tracker (`PdfTextState`).

use crate::infrastructure::pdf::font::{
    break_text_into_lines, parse_font_from_dict, resolve_glyph_geom, resolve_text_write_font,
    ParsedFont, ResourceCache,
};
use crate::infrastructure::pdf::models::*;
use crate::infrastructure::pdf::pdf_read::{operands_to_f32, read_resources, FlatResources};
use crate::infrastructure::pdf::text_state::TextState;
use lopdf::{content::Content, Document, Object, StringFormat};
use pdf_viewer_core::geometry::coordinate_transform::PdfCoordinateSpace;
use std::collections::HashMap;
use std::sync::Arc;

// ── PersistedTextLinePlan (shared with emitters) ────────────────

#[derive(Clone)]
pub(crate) struct PersistedTextLinePlan {
    pub(crate) font_alias: Vec<u8>,
    pub(crate) font_size: f32,
    pub(crate) encoded_bytes: Vec<u8>,
    pub(crate) tx: f32,
    pub(crate) ty: f32,
    pub(crate) width: f32,
    pub(crate) color: String,
    pub(crate) is_underline: bool,
    pub(crate) horizontal_scaling: f32,
    pub(crate) render_mode: i32,
    pub(crate) patch_idx: usize,
    pub(crate) line_seq: usize,
}

// ── PdfTextState (write-path state tracker) ─────────────────────

#[derive(Clone)]
pub(crate) struct PdfTextState {
    /// Shared text state: matrix trio + text-state parameters.
    pub(crate) text: TextState,
    /// Write-only: the raw font name bytes from the `Tf` operator.
    pub(crate) font_alias: Vec<u8>,
}

impl PdfTextState {
    pub(crate) fn new() -> Self {
        Self {
            text: TextState::default(),
            font_alias: Vec::new(),
        }
    }
}

// ── ReflowCluster ───────────────────────────────────────────────

#[derive(Clone, Debug)]
pub(crate) struct ReflowCluster<'a> {
    pub(crate) min_idx: usize,
    pub(crate) max_idx: usize,
    pub(crate) patches: Vec<&'a TextReflowPatch>,
}

impl<'a> ReflowCluster<'a> {
    pub(crate) fn build(patches: &'a [TextReflowPatch]) -> Vec<Self> {
        let mut map: std::collections::BTreeMap<usize, ReflowCluster<'a>> =
            std::collections::BTreeMap::new();
        for p in patches.iter().filter(|p| !p.target_indices.is_empty()) {
            let anchor = p.target_indices.iter().min().copied().unwrap_or(0);
            let max_idx = p.target_indices.iter().max().copied().unwrap_or(anchor);
            map.entry(anchor)
                .and_modify(|c| {
                    c.max_idx = c.max_idx.max(max_idx);
                    c.patches.push(p);
                })
                .or_insert_with(|| ReflowCluster {
                    min_idx: anchor,
                    max_idx,
                    patches: vec![p],
                });
        }
        map.into_values().collect()
    }
}

// ── Pure helpers ────────────────────────────────────────────────

/// Collect all target indices that will be silenced (replaced) by patches.
pub(crate) fn compute_silenced_indices(
    cluster_map: &HashMap<usize, ReflowCluster<'_>>,
) -> std::collections::HashSet<usize> {
    let mut silenced = std::collections::HashSet::new();
    for c in cluster_map.values() {
        for p in &c.patches {
            for idx in &p.target_indices {
                silenced.insert(*idx);
            }
        }
    }
    silenced
}

/// Compute micro-fit adjustments so replacement text lands in the original slot.
/// Returns `(effective_h_scaling, effective_char_spacing)`.
pub(crate) fn compute_micro_fit(
    initial_layout: &pdf_viewer_core::geometry::layout_engine::ParagraphLayout,
    target_wrap: f32,
    new_text: &str,
    mut h_scaling: f32,
    mut char_spacing: f32,
) -> (f32, f32) {
    if target_wrap > 1.0 && initial_layout.lines.len() == 1 {
        let nat_w = initial_layout.lines[0].width;
        if nat_w > 0.1 && (nat_w - target_wrap).abs() > 0.5 {
            let ratio = target_wrap / nat_w;
            if ratio >= 0.85 && ratio <= 1.15 {
                h_scaling *= ratio;
            } else {
                let char_count = new_text.chars().count();
                if char_count > 1 {
                    let delta = (target_wrap - nat_w) / ((char_count - 1) as f32);
                    char_spacing += delta;
                }
            }
        }
    }
    (h_scaling, char_spacing)
}

/// Create a muted (no-op) copy of a show-operator by emptying its text operand.
pub(crate) fn mute_show_op(op: &lopdf::content::Operation, op_str: &str) -> lopdf::content::Operation {
    let mut muted = op.clone();
    match op_str {
        "Tj" | "'" => muted.operands[0] = Object::String(vec![], StringFormat::Literal),
        "TJ" => muted.operands[0] = Object::Array(vec![]),
        "\"" => muted.operands[2] = Object::String(vec![], StringFormat::Literal),
        _ => {}
    }
    muted
}

/// Apply a pure text-state operator (BT, ET, Tc/Tw/Tz/Tr/TL, Tm, Td/TD, T*, q/Q, cm)
/// to the write-path state. Returns `true` if the operator was handled here.
pub(crate) fn apply_text_state_op(
    op: &lopdf::content::Operation,
    op_str: &str,
    state: &mut PdfTextState,
    state_stack: &mut Vec<PdfTextState>,
) -> bool {
    match op_str {
        "BT" => { state.text.op_bt(); true }
        "ET" => true,
        "Tc" | "Tw" | "Tz" | "Tr" | "TL" => {
            if let Some(f) = op.operands.get(0).and_then(|o| {
                o.as_float().ok().or_else(|| o.as_i64().ok().map(|i| i as f32))
            }) {
                match op_str {
                    "Tc" => state.text.char_spacing = f,
                    "Tw" => state.text.word_spacing = f,
                    "Tz" => state.text.horizontal_scaling = f,
                    "Tr" => state.text.render_mode = f as i32,
                    "TL" => state.text.tl = f,
                    _ => {}
                }
            }
            true
        }
        "Tm" => {
            if let Ok(m) = operands_to_f32(&op.operands) {
                if m.len() >= 6 {
                    state.text.op_tm([m[0], m[1], m[2], m[3], m[4], m[5]]);
                }
            }
            true
        }
        "Td" => {
            if let Ok(p) = operands_to_f32(&op.operands) {
                if p.len() >= 2 {
                    state.text.op_td(p[0], p[1]);
                }
            }
            true
        }
        "TD" => {
            if let Ok(p) = operands_to_f32(&op.operands) {
                if p.len() >= 2 {
                    state.text.op_td_with_leading(p[0], p[1]);
                }
            }
            true
        }
        "T*" => { state.text.op_t_star(); true }
        "q" => { state_stack.push(state.clone()); true }
        "Q" => {
            if let Some(s) = state_stack.pop() {
                *state = s;
            }
            true
        }
        "cm" => {
            if let Ok(m) = operands_to_f32(&op.operands) {
                if m.len() >= 6 {
                    state.text.op_cm([m[0], m[1], m[2], m[3], m[4], m[5]]);
                }
            }
            true
        }
        _ => false,
    }
}

// ── Visual line helpers ─────────────────────────────────────────

pub(crate) fn resolve_line_color(line: &pdf_viewer_core::geometry::layout_engine::VisualLine) -> String {
    line.runs
        .iter()
        .find(|r| !r.text.is_empty())
        .map(|r| r.style.color.clone())
        .filter(|c| !c.trim().is_empty())
        .unwrap_or_else(|| "#000000".to_string())
}

pub(crate) fn resolve_line_underline(line: &pdf_viewer_core::geometry::layout_engine::VisualLine) -> bool {
    line.runs.iter().any(|r| r.style.is_underline)
}

// ── Content-stream walkers ──────────────────────────────────────

/// Simple text replacement walker: finds `old_text` in a Tj/TJ and replaces with `new_text`.
pub(crate) fn patch_content_recursive(
    doc: &mut Document,
    content: &mut Content,
    resources: &FlatResources,
    cache: &mut ResourceCache,
    old_text: &str,
    new_text: &str,
    target_index: Option<usize>,
    _offset_x: Option<f32>,
    obj_counter: &mut usize,
) -> Result<bool, String> {
    let mut modified = false;
    let mut current_font: Option<Arc<ParsedFont>> = None;
    let mut font_size = 12.0;
    let mut char_spacing = 0.0;
    let mut word_spacing = 0.0;
    let mut h_scaling = 100.0;

    for op in &mut content.operations {
        match op.operator.as_str() {
            "Tf" => {
                if let Some(name) = op.operands.get(0).and_then(|o| o.as_name().ok()) {
                    font_size = op
                        .operands
                        .get(1)
                        .and_then(|o| {
                            o.as_float()
                                .ok()
                                .or_else(|| o.as_i64().ok().map(|i| i as f32))
                        })
                        .unwrap_or(font_size);
                    if let Some(id) = resources.get(b"Font" as &[u8]).and_then(|m| m.get(name)) {
                        if let Some(f) = cache.fonts.get(id) {
                            current_font = Some(f.clone());
                        } else if let Ok(p) = parse_font_from_dict(doc, *id, name) {
                            let arc = Arc::new(p);
                            cache.fonts.insert(*id, arc.clone());
                            current_font = Some(arc);
                        }
                    }
                }
            }
            "Tc" => {
                if let Some(v) = op.operands.get(0).and_then(|o| {
                    o.as_float()
                        .ok()
                        .or_else(|| o.as_i64().ok().map(|i| i as f32))
                }) {
                    char_spacing = v;
                }
            }
            "Tw" => {
                if let Some(v) = op.operands.get(0).and_then(|o| {
                    o.as_float()
                        .ok()
                        .or_else(|| o.as_i64().ok().map(|i| i as f32))
                }) {
                    word_spacing = v;
                }
            }
            "Tz" => {
                if let Some(v) = op.operands.get(0).and_then(|o| {
                    o.as_float()
                        .ok()
                        .or_else(|| o.as_i64().ok().map(|i| i as f32))
                }) {
                    h_scaling = v;
                }
            }
            "Tj" | "TJ" => {
                *obj_counter += 1;
                if target_index.map_or(true, |t| *obj_counter == t) {
                    let decoded = if let Some(ref font) = current_font {
                        if op.operator == "Tj" {
                            resolve_glyph_geom(
                                op.operands[0].as_str().unwrap_or(&[]),
                                font,
                                font_size,
                                h_scaling / 100.0,
                                char_spacing,
                                word_spacing,
                            )
                            .0
                        } else {
                            let mut s = String::new();
                            if let Ok(arr) = op.operands[0].as_array() {
                                for item in arr {
                                    if let Ok(b) = item.as_str() {
                                        s.push_str(
                                            &resolve_glyph_geom(
                                                b,
                                                font,
                                                font_size,
                                                h_scaling / 100.0,
                                                char_spacing,
                                                word_spacing,
                                            )
                                            .0,
                                        );
                                    }
                                }
                            }
                            s
                        }
                    } else {
                        String::from_utf8_lossy(op.operands[0].as_str().unwrap_or(&[])).to_string()
                    };

                    if decoded == old_text {
                        let replacement = if let Some(ref font) = current_font {
                            font.encode_text(new_text)
                        } else {
                            new_text.as_bytes().to_vec()
                        };
                        if op.operator == "Tj" {
                            op.operands[0] = Object::String(replacement, StringFormat::Literal);
                        } else {
                            op.operands[0] = Object::Array(vec![Object::String(
                                replacement,
                                StringFormat::Literal,
                            )]);
                        }
                        modified = true;
                    }
                }
            }
            "Do" => {
                if let Some(name) = op.operands.get(0).and_then(|o| o.as_name().ok()) {
                    if let Some(id) = resources.get(b"XObject" as &[u8]).and_then(|m| m.get(name)) {
                        let id = *id;
                        if let Ok(mut stream) =
                            doc.get_object(id).and_then(|o| o.as_stream().cloned())
                        {
                            if stream
                                .dict
                                .get(b"Subtype")
                                .ok()
                                .and_then(|o| o.as_name().ok())
                                == Some(b"Form")
                            {
                                if let Ok(data) = stream.decompressed_content() {
                                    if let Ok(mut sub) = Content::decode(&data) {
                                        let sub_res = read_resources(doc, id);
                                        if patch_content_recursive(
                                            doc,
                                            &mut sub,
                                            &sub_res,
                                            cache,
                                            old_text,
                                            new_text,
                                            target_index,
                                            _offset_x,
                                            obj_counter,
                                        )? {
                                            stream.set_content(
                                                sub.encode().map_err(|e| e.to_string())?,
                                            );
                                            doc.set_object(id, stream);
                                            modified = true;
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
    Ok(modified)
}

/// Layout-aware reflow walker: silences original show-ops and builds deferred
/// text-line plans for the emitter to emit later.
pub(crate) fn patch_atomic_reflow_recursive(
    doc: &mut Document,
    page_id: lopdf::ObjectId,
    content: &mut Content,
    resources: &FlatResources,
    res_cache: &mut ResourceCache,
    cluster_map: &HashMap<usize, ReflowCluster>,
    page_height: f32,
    obj_counter: &mut usize,
    state: &mut PdfTextState,
    deferred_lines: &mut Vec<PersistedTextLinePlan>,
) -> Result<bool, String> {
    let mut modified = false;
    let mut current_font = None;
    let mut state_stack: Vec<PdfTextState> = Vec::new();
    let silenced = compute_silenced_indices(cluster_map);
    let mut injected = std::collections::HashSet::new();

    let mut new_ops = Vec::new();
    for op in &content.operations {
        let op_str = op.operator.as_str();
        let target_idx = obj_counter.wrapping_add(1);
        let is_show = matches!(op_str, "Tj" | "TJ" | "'" | "\"");

        if is_show {
            if let Some(cluster) = cluster_map.get(&target_idx) {
                if injected.insert(target_idx) {
                    for patch in &cluster.patches {
                        let font_info = resolve_text_write_font(
                            doc, page_id, &state.font_alias,
                            current_font.as_ref().map(|f: &Arc<ParsedFont>| f.as_ref()),
                            &patch.new_text,
                        )?;
                        let active_font = Arc::new(font_info.parsed_font.clone());
                        let target_wrap = patch.wrap_width.unwrap_or(0.0);

                        let initial_layout = break_text_into_lines(
                            &patch.new_text, patch.new_runs.as_ref(), &active_font,
                            state.text.font_size, target_wrap, patch.alignment,
                            patch.line_height, patch.char_spacing, patch.horizontal_scaling,
                        );
                        let (effective_h_scaling, effective_char_spacing) = compute_micro_fit(
                            &initial_layout, target_wrap, &patch.new_text,
                            patch.horizontal_scaling, patch.char_spacing,
                        );

                        let layout = break_text_into_lines(
                            &patch.new_text, patch.new_runs.as_ref(), &active_font,
                            state.text.font_size, target_wrap, patch.alignment,
                            patch.line_height, effective_char_spacing, effective_h_scaling,
                        );

                        let trm = state.text.text_render_matrix();
                        let (psx, psy) = (
                            (trm[0].powi(2) + trm[1].powi(2)).sqrt(),
                            (trm[2].powi(2) + trm[3].powi(2)).sqrt(),
                        );
                        let (ax, ay) = (trm[4], PdfCoordinateSpace::normalize_y(trm[5], page_height));
                        let first_base = layout.lines.first().map(|l| l.baseline_y).unwrap_or(state.text.font_size);

                        for (idx, line) in layout.lines.iter().enumerate() {
                            let ly = ay + patch.displacement_y.unwrap_or(0.0)
                                - ((line.baseline_y - first_base) * psy);
                            let lx = ax + (line.offset_x as f32 * psx);
                            deferred_lines.push(PersistedTextLinePlan {
                                font_alias: font_info.font_alias.clone(),
                                font_size: state.text.font_size * psy,
                                encoded_bytes: font_info.encode_text(&line.text)?,
                                tx: lx, ty: ly,
                                width: line.width * psx,
                                color: resolve_line_color(line),
                                is_underline: resolve_line_underline(line),
                                horizontal_scaling: effective_h_scaling
                                    * state.text.horizontal_scaling / 100.0,
                                render_mode: state.text.render_mode,
                                patch_idx: target_idx, line_seq: idx,
                            });
                        }
                        modified = true;
                    }
                }
            }
        }

        if is_show && silenced.contains(&target_idx) {
            *obj_counter += 1;
            new_ops.push(mute_show_op(op, op_str));
            continue;
        }

        if !apply_text_state_op(op, op_str, state, &mut state_stack) {
            match op_str {
                "Tf" => {
                    if let Some(name) = op.operands.get(0).and_then(|o| o.as_name().ok()) {
                        state.font_alias = name.to_vec();
                        if let Some(id) = resources.get(b"Font" as &[u8]).and_then(|m| m.get(name)) {
                            if let Some(f) = res_cache.fonts.get(id) {
                                current_font = Some(f.clone());
                            } else if let Ok(p) = parse_font_from_dict(doc, *id, name) {
                                let arc = Arc::new(p);
                                res_cache.fonts.insert(*id, arc.clone());
                                current_font = Some(arc);
                            }
                        }
                        if let Some(s) = op.operands.get(1).and_then(|o| {
                            o.as_float().ok().or_else(|| o.as_i64().ok().map(|i| i as f32))
                        }) {
                            state.text.font_size = s;
                        }
                    }
                }
                "Tj" | "TJ" | "'" | "\"" => *obj_counter += 1,
                "Do" => {
                    if let Some(name) = op.operands.get(0).and_then(|o| o.as_name().ok()) {
                        if let Some(xid) = resources.get(b"XObject" as &[u8]).and_then(|m| m.get(name)) {
                            if let Ok(mut xstream) = doc.get_object(*xid).and_then(|o| o.as_stream().cloned()) {
                                if xstream.dict.get(b"Subtype").ok().and_then(|o| o.as_name().ok()) == Some(b"Form") {
                                    if let Ok(data) = xstream.decompressed_content() {
                                        if let Ok(mut sub) = Content::decode(&data) {
                                            let sub_res = read_resources(doc, *xid);
                                            let mut sub_state = state.clone();
                                            if let Ok(m_obj) = xstream.dict.get(b"Matrix") {
                                                if let Ok(m_arr) = m_obj.as_array() {
                                                    if let Ok(m) = operands_to_f32(m_arr) {
                                                        if m.len() >= 6 {
                                                            sub_state.text.op_cm([m[0], m[1], m[2], m[3], m[4], m[5]]);
                                                        }
                                                    }
                                                }
                                            }
                                            if patch_atomic_reflow_recursive(
                                                doc, page_id, &mut sub, &sub_res, res_cache,
                                                cluster_map, page_height, obj_counter,
                                                &mut sub_state, deferred_lines,
                                            )? {
                                                xstream.set_content(sub.encode().map_err(|e| e.to_string())?);
                                                doc.set_object(*xid, xstream);
                                                modified = true;
                                            }
                                        }
                                    }
                                } else {
                                    *obj_counter += 1;
                                }
                            }
                        }
                    }
                }
                "S" | "s" | "f" | "F" | "f*" | "B" | "b" | "B*" | "b*" => *obj_counter += 1,
                _ => {}
            }
        }
        new_ops.push(op.clone());
    }
    content.operations = new_ops;
    Ok(modified)
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod reflow_tests {
    use super::*;
    use lopdf::content::Operation;

    fn run_ops(ops: &[(&str, &[lopdf::Object])]) -> PdfTextState {
        let operations = ops
            .iter()
            .map(|(op, operands)| Operation::new(*op, operands.to_vec()))
            .collect();
        let mut content = Content { operations };
        let mut doc = Document::with_version("1.4");
        let resources: FlatResources = HashMap::new();
        let mut res_cache = ResourceCache::new();
        let cluster_map: HashMap<usize, ReflowCluster> = HashMap::new();
        let mut state = PdfTextState::new();
        let mut deferred: Vec<PersistedTextLinePlan> = Vec::new();
        let mut obj_counter = 0;
        let _ = patch_atomic_reflow_recursive(
            &mut doc,
            (0, 0),
            &mut content,
            &resources,
            &mut res_cache,
            &cluster_map,
            1000.0,
            &mut obj_counter,
            &mut state,
            &mut deferred,
        );
        state
    }

    #[test]
    fn new_initializes_nonzero_defaults_and_identity_matrices() {
        let s = PdfTextState::new();
        assert_eq!(s.text.font_size, 12.0);
        assert_eq!(s.text.horizontal_scaling, 100.0);
        assert_eq!(s.text.tl, 0.0);
        assert_eq!(s.text.ctm(), [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
        assert_eq!(s.text.tm(), [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
    }

    #[test]
    fn q_save_q_restore_clones_whole_graphics_state() {
        let state = run_ops(&[
            ("BT", &[]),
            ("Tf", &[Object::Name(b"F1".to_vec()), Object::Real(20.0)]),
            ("q", &[]),
            ("Tf", &[Object::Name(b"F1".to_vec()), Object::Real(40.0)]),
            ("Q", &[]),
        ]);
        assert_eq!(state.text.font_size, 20.0);
    }

    #[test]
    fn q_save_q_restore_also_restores_leading() {
        let state = run_ops(&[
            ("BT", &[]),
            ("TL", &[Object::Real(12.0)]),
            ("q", &[]),
            ("TL", &[Object::Real(30.0)]),
            ("Q", &[]),
            ("T*", &[]),
        ]);
        assert_eq!(state.text.tl, 12.0);
        assert_eq!(state.text.core.tm()[5], -12.0);
    }

    #[test]
    fn td_sets_leading_then_translates() {
        let state = run_ops(&[
            ("BT", &[]),
            (
                "Tm",
                &[
                    Object::Real(1.0),
                    Object::Real(0.0),
                    Object::Real(0.0),
                    Object::Real(1.0),
                    Object::Real(50.0),
                    Object::Real(60.0),
                ],
            ),
            ("TD", &[Object::Real(10.0), Object::Real(-15.0)]),
            ("T*", &[]),
        ]);
        let tm = state.text.core.tm();
        assert_eq!(state.text.tl, 15.0);
        assert_eq!(tm[4], 60.0);
        assert_eq!(tm[5], 30.0);
    }

    #[test]
    fn t_star_without_leading_is_a_noop() {
        let state = run_ops(&[
            ("BT", &[]),
            (
                "Tm",
                &[
                    Object::Real(1.0),
                    Object::Real(0.0),
                    Object::Real(0.0),
                    Object::Real(1.0),
                    Object::Real(50.0),
                    Object::Real(60.0),
                ],
            ),
            ("T*", &[]),
        ]);
        let tm = state.text.core.tm();
        assert_eq!(tm[4], 50.0);
        assert_eq!(tm[5], 60.0);
        assert_eq!(state.text.tl, 0.0);
    }

    #[test]
    fn tl_op_sets_text_leading() {
        let state = run_ops(&[
            ("BT", &[]),
            ("TL", &[Object::Real(18.0)]),
            ("T*", &[]),
        ]);
        assert_eq!(state.text.tl, 18.0);
        assert_eq!(state.text.core.tm()[5], -18.0);
    }

    #[test]
    fn compute_silenced_indices_collects_all_target_indices() {
        use std::collections::HashMap;
        let p1 = TextReflowPatch {
            page_index: 0,
            target_indices: vec![1, 3],
            new_text: "a".into(),
            new_runs: None,
            alignment: None,
            line_height: None,
            displacement_y: None,
            wrap_width: None,
            char_spacing: 0.0,
            horizontal_scaling: 100.0,
        };
        let p2 = TextReflowPatch {
            page_index: 0,
            target_indices: vec![3, 5],
            new_text: "b".into(),
            new_runs: None,
            alignment: None,
            line_height: None,
            displacement_y: None,
            wrap_width: None,
            char_spacing: 0.0,
            horizontal_scaling: 100.0,
        };
        let c1 = ReflowCluster { min_idx: 1, max_idx: 3, patches: vec![&p1] };
        let c2 = ReflowCluster { min_idx: 3, max_idx: 5, patches: vec![&p2] };
        let mut cluster_map = HashMap::new();
        cluster_map.insert(1, c1);
        cluster_map.insert(3, c2);
        let silenced = compute_silenced_indices(&cluster_map);
        assert!(silenced.contains(&1));
        assert!(silenced.contains(&3));
        assert!(silenced.contains(&5));
        assert_eq!(silenced.len(), 3);
    }

    #[test]
    fn compute_micro_fit_applies_tz_ratio_for_close_fit() {
        use pdf_viewer_core::geometry::layout_engine::{ParagraphLayout, VisualLine};
        let layout = ParagraphLayout {
            lines: vec![VisualLine { width: 90.0, runs: vec![], height: 0.0, baseline_y: 0.0, offset_x: 0.0, text: String::new() }],
            height: 0.0,
        };
        let (h, c) = compute_micro_fit(&layout, 100.0, "hello", 100.0, 0.0);
        assert!((h - 100.0 * (100.0 / 90.0)).abs() < 0.01);
        assert_eq!(c, 0.0);
    }

    #[test]
    fn compute_micro_fit_applies_char_spacing_for_wide_gap() {
        use pdf_viewer_core::geometry::layout_engine::{ParagraphLayout, VisualLine};
        let layout = ParagraphLayout {
            lines: vec![VisualLine { width: 70.0, runs: vec![], height: 0.0, baseline_y: 0.0, offset_x: 0.0, text: String::new() }],
            height: 0.0,
        };
        let (h, c) = compute_micro_fit(&layout, 100.0, "ab", 100.0, 0.0);
        assert_eq!(h, 100.0);
        assert!(c > 0.0);
    }

    #[test]
    fn compute_micro_fit_no_adjustment_when_no_wrap() {
        use pdf_viewer_core::geometry::layout_engine::{ParagraphLayout, VisualLine};
        let layout = ParagraphLayout {
            lines: vec![VisualLine { width: 90.0, runs: vec![], height: 0.0, baseline_y: 0.0, offset_x: 0.0, text: String::new() }],
            height: 0.0,
        };
        let (h, c) = compute_micro_fit(&layout, 0.0, "hello", 100.0, 0.0);
        assert_eq!(h, 100.0);
        assert_eq!(c, 0.0);
    }

    #[test]
    fn mute_show_op_empties_tj_operand() {
        let op = Operation::new("Tj", vec![Object::String(b"hello".to_vec(), lopdf::StringFormat::Literal)]);
        let muted = mute_show_op(&op, "Tj");
        assert_eq!(muted.operator, "Tj");
        match &muted.operands[0] {
            Object::String(s, _) => assert!(s.is_empty()),
            _ => panic!("expected empty string operand"),
        }
    }

    #[test]
    fn mute_show_op_empties_tj_array_operand() {
        let op = Operation::new("TJ", vec![Object::Array(vec![Object::Real(100.0)])]);
        let muted = mute_show_op(&op, "TJ");
        match &muted.operands[0] {
            Object::Array(a) => assert!(a.is_empty()),
            _ => panic!("expected empty array operand"),
        }
    }

    #[test]
    fn apply_text_state_op_handles_bt() {
        let mut state = PdfTextState::new();
        let mut stack = Vec::new();
        let op = Operation::new("BT", vec![]);
        assert!(apply_text_state_op(&op, "BT", &mut state, &mut stack));
        assert_eq!(state.text.core.tm(), [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
    }

    #[test]
    fn apply_text_state_op_returns_false_for_unknown() {
        let mut state = PdfTextState::new();
        let mut stack = Vec::new();
        let op = Operation::new("Tf", vec![Object::Name(b"F1".to_vec()), Object::Real(12.0)]);
        assert!(!apply_text_state_op(&op, "Tf", &mut state, &mut stack));
    }

    #[test]
    fn text_line_plan_preserves_render_mode_and_scaling() {
        let line_plan = PersistedTextLinePlan {
            font_alias: b"F1".to_vec(),
            font_size: 12.0,
            encoded_bytes: vec![0, 65],
            tx: 10.0,
            ty: 20.0,
            width: 100.0,
            color: "#000000".to_string(),
            is_underline: false,
            horizontal_scaling: 105.0,
            render_mode: 2,
            patch_idx: 1,
            line_seq: 0,
        };
        assert_eq!(line_plan.render_mode, 2);
        assert!((line_plan.horizontal_scaling - 105.0).abs() < 0.001);
    }
}
