use crate::infrastructure::pdf::models::*;
use crate::infrastructure::pdf::pdf_font::{
    parse_font_from_dict, resolve_glyph_geom, ParsedFont, ResourceCache,
};
use crate::infrastructure::pdf::pdf_read::{
    multiply_matrices, operands_to_f32, read_resources, FlatResources,
};
use crate::infrastructure::pdf::pdf_write_font_resolver::resolve_text_write_font;
use crate::infrastructure::pdf::save_text_write_plan::PersistedTextLinePlan;
use lopdf::{content::Content, Dictionary, Document, Object, Stream, StringFormat};
use crate::infrastructure::pdf::pdf_utils;
use pdf_viewer_core::geometry::coordinate_transform::PdfCoordinateSpace;
use std::collections::HashMap;
use std::sync::Arc;

pub trait PdfDocExt {
    fn apply_text_patch(
        &mut self,
        page_num: u32,
        old_text: &str,
        new_text: &str,
        target_index: Option<usize>,
        offset_x: Option<f32>,
    ) -> Result<(), String>;
    fn apply_atomic_reflow_to_doc(
        &mut self,
        page_num: u32,
        target_indices: &[usize],
        new_text: &str,
        new_runs: Option<Vec<pdf_viewer_core::models::LayoutRun>>,
        displacement_y: Option<f32>,
        wrap_width: Option<f32>,
        align: Option<pdf_viewer_core::models::LayoutAlignment>,
        line_height: Option<f32>,
        char_spacing: f32,
        horizontal_scaling: f32,
    ) -> Result<(), String>;
    fn apply_batch_reflow_to_doc(
        &mut self,
        page_num: u32,
        patches: &[TextReflowPatch],
    ) -> Result<(), String>;
    fn replace_image_xobject(
        &mut self,
        object_id: (u32, u16),
        new_bytes: &[u8],
    ) -> Result<(), String>;
    fn delete_page(&mut self, page_num: u32) -> Result<(), String>;
    fn rotate_page(&mut self, page_num: u32, rotation: i32) -> Result<(), String>;
    fn insert_blank_page(&mut self, at_index: u32) -> Result<(), String>;
    fn add_highlight(
        &mut self,
        page_num: u32,
        rect: [f32; 4],
        color: [f32; 3],
    ) -> Result<(), String>;
    fn add_text_comment(
        &mut self,
        page_num: u32,
        rect: [f32; 4],
        color: [f32; 3],
        contents: &str,
    ) -> Result<(), String>;
    fn update_text_comment(
        &mut self,
        page_num: u32,
        annot_id: (u32, u16),
        contents: &str,
    ) -> Result<(), String>;
    fn delete_annotation(&mut self, page_num: u32, annot_id: (u32, u16)) -> Result<(), String>;
    fn update_metadata(
        &mut self,
        title: &str,
        author: &str,
        subject: &str,
        keywords: &str,
    ) -> Result<(), String>;
}

impl PdfDocExt for Document {
    fn apply_text_patch(
        &mut self,
        page_num: u32,
        old_text: &str,
        new_text: &str,
        target_index: Option<usize>,
        offset_x: Option<f32>,
    ) -> Result<(), String> {
        let page_id = *self
            .get_pages()
            .get(&page_num)
            .ok_or_else(|| format!("Page {} not found", page_num))?;
        let resources = read_resources(self, page_id);
        let mut cache = ResourceCache::new();
        let content_data = self.get_page_content(page_id).map_err(|e| e.to_string())?;
        let mut content = Content::decode(&content_data).map_err(|e| e.to_string())?;

        let mut obj_counter = 0;
        if patch_content_recursive(
            self,
            &mut content,
            &resources,
            &mut cache,
            old_text,
            new_text,
            target_index,
            offset_x,
            &mut obj_counter,
        )? {
            let new_content = content.encode().map_err(|e| e.to_string())?;
            let stream_id = self.new_object_id();
            self.objects.insert(
                stream_id,
                Object::Stream(Stream::new(Dictionary::new(), new_content)),
            );
            let page_dict = self
                .get_object_mut(page_id)
                .and_then(|obj| obj.as_dict_mut())
                .map_err(|e| e.to_string())?;
            page_dict.set("Contents", Object::Reference(stream_id));
            Ok(())
        } else {
            Err("Text not found for patching".into())
        }
    }

    fn apply_atomic_reflow_to_doc(
        &mut self,
        page_num: u32,
        target_indices: &[usize],
        new_text: &str,
        new_runs: Option<Vec<pdf_viewer_core::models::LayoutRun>>,
        displacement_y: Option<f32>,
        wrap_width: Option<f32>,
        align: Option<pdf_viewer_core::models::LayoutAlignment>,
        line_height: Option<f32>,
        char_spacing: f32,
        horizontal_scaling: f32,
    ) -> Result<(), String> {
        let single_patch = TextReflowPatch {
            page_index: page_num as u16,
            target_indices: target_indices.to_vec(),
            new_text: new_text.to_string(),
            new_runs,
            alignment: align,
            line_height,
            displacement_y,
            wrap_width,
            char_spacing,
            horizontal_scaling,
        };
        self.apply_batch_reflow_to_doc(page_num, &[single_patch])
    }

    fn apply_batch_reflow_to_doc(
        &mut self,
        page_num: u32,
        patches: &[TextReflowPatch],
    ) -> Result<(), String> {
        let page_id = *self
            .get_pages()
            .get(&page_num)
            .ok_or_else(|| format!("Page {} not found", page_num))?;
        let flat_resources = read_resources(self, page_id);
        let mut res_cache = ResourceCache::new();

        let _page_dict = self.get_dictionary(page_id).map_err(|e| e.to_string())?;
        let page_size = pdf_utils::read_page_size(self, page_id);
        let page_height = page_size.effective_height();

        let content_data = self
            .get_page_content(page_id)
            .map_err(|e| format!("Failed to get page content: {}", e))?;
        let mut content = Content::decode(&content_data)
            .map_err(|e| format!("Failed to decode content: {}", e))?;

        let user_unit = self
            .get_dictionary(page_id)
            .ok()
            .and_then(|dict| dict.get(b"UserUnit").ok())
            .and_then(|o| {
                o.as_float()
                    .ok()
                    .or_else(|| o.as_i64().ok().map(|i| i as f32))
            })
            .unwrap_or(1.0);

        content
            .operations
            .insert(0, lopdf::content::Operation::new("q", vec![]));

        let clusters = ReflowCluster::build(patches);
        let mut cluster_map = HashMap::new();
        for cluster in &clusters {
            cluster_map.insert(cluster.min_idx, cluster.clone());
        }

        let mut state = PdfTextState::default();
        let patch_font_name = b"Helvetica".to_vec();
        let mut obj_counter = 0;

        let changed = patch_atomic_reflow_recursive(
            self,
            page_id,
            &mut content,
            &flat_resources,
            &mut res_cache,
            &cluster_map,
            page_height,
            &patch_font_name,
            &mut obj_counter,
            &mut state,
        )?;

        if changed || !state.deferred_lines.is_empty() {
            content
                .operations
                .push(lopdf::content::Operation::new("Q", vec![]));
        }

        if !state.deferred_lines.is_empty() {
            content
                .operations
                .extend(emit_deferred_text_block(&state.deferred_lines, page_height, user_unit));
        }

        if changed || !state.deferred_lines.is_empty() {
            let new_content = content
                .encode()
                .map_err(|e| format!("Failed to encode: {}", e))?;
            let new_id = self.new_object_id();
            self.objects.insert(
                new_id,
                Object::Stream(Stream::new(Dictionary::new(), new_content)),
            );
            let p_dict = self
                .get_object_mut(page_id)
                .and_then(|o| o.as_dict_mut())
                .map_err(|e| e.to_string())?;
            p_dict.set("Contents", Object::Reference(new_id));
        }
        Ok(())
    }

    fn replace_image_xobject(
        &mut self,
        _object_id: (u32, u16),
        _new_bytes: &[u8],
    ) -> Result<(), String> {
        Err("replace_image_xobject not yet implemented".to_string())
    }

    fn delete_page(&mut self, page_num: u32) -> Result<(), String> {
        self.delete_pages(&[page_num]);
        Ok(())
    }

    fn rotate_page(&mut self, page_num: u32, rotation: i32) -> Result<(), String> {
        let page_id = *self
            .get_pages()
            .get(&page_num)
            .ok_or_else(|| format!("Page {} not found", page_num))?;
        let page_dict = self
            .get_object_mut(page_id)
            .and_then(|obj| obj.as_dict_mut())
            .map_err(|e| format!("Get page dict error: {}", e))?;
        page_dict.set("Rotate", rotation);
        Ok(())
    }

    fn insert_blank_page(&mut self, _at_index: u32) -> Result<(), String> {
        Err("insert_blank_page not yet implemented".to_string())
    }

    fn add_highlight(
        &mut self,
        page_num: u32,
        rect: [f32; 4],
        color: [f32; 3],
    ) -> Result<(), String> {
        let page_id = *self
            .get_pages()
            .get(&page_num)
            .ok_or_else(|| format!("Page {} not found", page_num))?;
        let page_height = read_page_height(self, page_id)?;
        let (left, top, width, height) = (rect[0], rect[1], rect[2].max(1.0), rect[3].max(1.0));
        let (right, p_top, p_bot) = (
            left + width,
            page_height - top,
            page_height - (top + height),
        );

        let annot_id = self.new_object_id();
        let mut dict = Dictionary::new();
        dict.set("Type", Object::Name(b"Annot".to_vec()));
        dict.set("Subtype", Object::Name(b"Highlight".to_vec()));
        dict.set(
            "Rect",
            Object::Array(vec![
                Object::Real(left),
                Object::Real(p_bot),
                Object::Real(right),
                Object::Real(p_top),
            ]),
        );
        dict.set(
            "QuadPoints",
            Object::Array(vec![
                Object::Real(left),
                Object::Real(p_top),
                Object::Real(right),
                Object::Real(p_top),
                Object::Real(left),
                Object::Real(p_bot),
                Object::Real(right),
                Object::Real(p_bot),
            ]),
        );
        dict.set(
            "C",
            Object::Array(vec![
                Object::Real(color[0]),
                Object::Real(color[1]),
                Object::Real(color[2]),
            ]),
        );
        dict.set("CA", Object::Real(0.35));
        dict.set("F", Object::Integer(4));
        dict.set("P", Object::Reference(page_id));
        self.objects.insert(annot_id, Object::Dictionary(dict));
        append_page_annotation(self, page_id, annot_id)
    }

    fn add_text_comment(
        &mut self,
        page_num: u32,
        rect: [f32; 4],
        color: [f32; 3],
        contents: &str,
    ) -> Result<(), String> {
        let page_id = *self
            .get_pages()
            .get(&page_num)
            .ok_or_else(|| format!("Page {} not found", page_num))?;
        let page_height = read_page_height(self, page_id)?;
        let (left, top, width, height) = (
            rect[0].max(0.0),
            rect[1].max(0.0),
            rect[2].max(14.0),
            rect[3].max(14.0),
        );
        let size = width.min(height).clamp(16.0, 24.0);
        let (n_left, n_top) = (left + width - size, top);
        let (n_right, p_top, p_bot) = (
            n_left + size,
            page_height - n_top,
            page_height - (n_top + size),
        );

        let annot_id = self.new_object_id();
        let mut dict = Dictionary::new();
        dict.set("Type", Object::Name(b"Annot".to_vec()));
        dict.set("Subtype", Object::Name(b"Text".to_vec()));
        dict.set(
            "Rect",
            Object::Array(vec![
                Object::Real(n_left),
                Object::Real(p_bot),
                Object::Real(n_right),
                Object::Real(p_top),
            ]),
        );
        dict.set("Contents", Object::string_literal(contents));
        dict.set("Name", Object::Name(b"Comment".to_vec()));
        dict.set(
            "C",
            Object::Array(vec![
                Object::Real(color[0]),
                Object::Real(color[1]),
                Object::Real(color[2]),
            ]),
        );
        dict.set("Open", Object::Boolean(false));
        dict.set("F", Object::Integer(4));
        dict.set("P", Object::Reference(page_id));
        self.objects.insert(annot_id, Object::Dictionary(dict));
        append_page_annotation(self, page_id, annot_id)
    }

    fn update_text_comment(
        &mut self,
        page_num: u32,
        annot_id: (u32, u16),
        contents: &str,
    ) -> Result<(), String> {
        let page_id = *self
            .get_pages()
            .get(&page_num)
            .ok_or_else(|| format!("Page {} not found", page_num))?;
        if !read_page_annotation_refs(self, page_id)?.contains(&annot_id) {
            return Err(format!(
                "Annotation {:?} not found on page {}",
                annot_id, page_num
            ));
        }
        let dict = self
            .get_object_mut(annot_id)
            .and_then(|obj| obj.as_dict_mut())
            .map_err(|e| e.to_string())?;
        if dict
            .get(b"Subtype")
            .ok()
            .and_then(|v| v.as_name().ok())
            .unwrap_or(b"")
            != b"Text"
        {
            return Err(format!("Annotation {:?} is not a text comment", annot_id));
        }
        dict.set("Contents", Object::string_literal(contents));
        Ok(())
    }

    fn delete_annotation(&mut self, page_num: u32, annot_id: (u32, u16)) -> Result<(), String> {
        let page_id = *self
            .get_pages()
            .get(&page_num)
            .ok_or_else(|| format!("Page {} not found", page_num))?;
        remove_page_annotation(self, page_id, annot_id)?;
        self.objects.remove(&annot_id);
        Ok(())
    }

    fn update_metadata(
        &mut self,
        title: &str,
        author: &str,
        subject: &str,
        keywords: &str,
    ) -> Result<(), String> {
        let info_id = self
            .trailer
            .get(b"Info")
            .ok()
            .and_then(|obj| obj.as_reference().ok())
            .ok_or("No Info dict")?;
        let dict = self
            .get_object_mut(info_id)
            .and_then(|obj| obj.as_dict_mut())
            .map_err(|e| e.to_string())?;
        dict.set("Title", Object::string_literal(title));
        dict.set("Author", Object::string_literal(author));
        dict.set("Subject", Object::string_literal(subject));
        dict.set("Keywords", Object::string_literal(keywords));
        Ok(())
    }
}

fn patch_content_recursive(
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

#[derive(Clone, Default)]
struct PdfTextState {
    font_alias: Vec<u8>,
    font_size: f32,
    char_spacing: f32,
    word_spacing: f32,
    horizontal_scaling: f32,
    render_mode: i32,
    in_text_block: bool,
    last_tm: [f32; 6],
    tlm: [f32; 6],
    ctm: [f32; 6],
    state_stack: Vec<([f32; 6], [f32; 6], [f32; 6])>,
    deferred_lines: Vec<PersistedTextLinePlan>,
}

#[derive(Clone, Debug)]
struct ReflowCluster<'a> {
    min_idx: usize,
    max_idx: usize,
    patches: Vec<&'a TextReflowPatch>,
}

impl<'a> ReflowCluster<'a> {
    pub fn build(patches: &'a [TextReflowPatch]) -> Vec<Self> {
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

// ── PDF operation emitters (command pattern) ──────────────────────────────
// Each function returns a Vec<Operation> expressing one rendering intent,
// decoupling the high-level "draw a text line / underline" semantics from
// the low-level PDF operator sequencing in `apply_batch_reflow_to_doc`.

/// Geometry of a single underline stroke, ready to emit as PDF path operators.
struct UnderlineSpec {
    x: f32,
    y: f32,
    width: f32,
    stroke_width: f32,
    color: String,
}

/// Emit the PDF text operators for one reflow line: color, text matrix, font, show.
/// Returns the operations plus an optional underline spec when the line needs one.
fn emit_text_line_ops(run: &PersistedTextLinePlan, user_unit: f32) -> (Vec<lopdf::content::Operation>, Option<UnderlineSpec>) {
    let mut ops = Vec::new();
    let h_scale = run.horizontal_scaling / 100.0;
    let adj_tx = run.tx / user_unit;
    let adj_ty = run.ty / user_unit;
    let adj_width = run.width / user_unit;
    let adj_font_size = run.font_size / user_unit;

    if let Some([red, green, blue]) = parse_pdf_hex_color(&run.color) {
        ops.push(lopdf::content::Operation::new(
            "rg",
            vec![Object::Real(red), Object::Real(green), Object::Real(blue)],
        ));
        ops.push(lopdf::content::Operation::new(
            "RG",
            vec![Object::Real(red), Object::Real(green), Object::Real(blue)],
        ));
    }

    ops.push(lopdf::content::Operation::new(
        "Tm",
        vec![
            Object::Real(h_scale),
            Object::Real(0.0),
            Object::Real(0.0),
            Object::Real(1.0),
            Object::Real(adj_tx),
            Object::Real(adj_ty),
        ],
    ));
    ops.push(lopdf::content::Operation::new(
        "Tf",
        vec![
            Object::Name(run.font_alias.clone()),
            Object::Real(adj_font_size),
        ],
    ));
    ops.push(lopdf::content::Operation::new(
        "Tj",
        vec![Object::String(
            run.encoded_bytes.clone(),
            lopdf::StringFormat::Hexadecimal,
        )],
    ));

    let underline = if run.is_underline && adj_width > 0.0 {
        Some(UnderlineSpec {
            x: adj_tx,
            y: adj_ty + (adj_font_size * 0.12),
            width: adj_width,
            stroke_width: (adj_font_size * 0.055).max(0.6),
            color: run.color.clone(),
        })
    } else {
        None
    };

    (ops, underline)
}

/// Emit the PDF path operators for one underline stroke: color, width, move, line, stroke.
fn emit_underline_ops(spec: &UnderlineSpec) -> Vec<lopdf::content::Operation> {
    let mut ops = Vec::new();
    if let Some([r, g, b]) = parse_pdf_hex_color(&spec.color) {
        ops.push(lopdf::content::Operation::new(
            "RG",
            vec![Object::Real(r), Object::Real(g), Object::Real(b)],
        ));
    }
    ops.push(lopdf::content::Operation::new("w", vec![Object::Real(spec.stroke_width)]));
    ops.push(lopdf::content::Operation::new(
        "m",
        vec![Object::Real(spec.x), Object::Real(spec.y)],
    ));
    ops.push(lopdf::content::Operation::new(
        "l",
        vec![Object::Real(spec.x + spec.width), Object::Real(spec.y)],
    ));
    ops.push(lopdf::content::Operation::new("S", vec![]));
    ops
}

/// Emit the full deferred-text block: graphics state setup, text lines, underlines, teardown.
/// `page_height` drives the Y-flip cm matrix; `user_unit` scales coordinates.
fn emit_deferred_text_block(
    lines: &[PersistedTextLinePlan],
    page_height: f32,
    user_unit: f32,
) -> Vec<lopdf::content::Operation> {
    let mut ops = Vec::new();
    // Graphics state: save, flip-Y, reset char/word spacing + horizontal scaling, begin text.
    ops.push(lopdf::content::Operation::new("q", vec![]));
    ops.push(lopdf::content::Operation::new(
        "cm",
        vec![
            Object::Real(1.0),
            Object::Real(0.0),
            Object::Real(0.0),
            Object::Real(-1.0),
            Object::Real(0.0),
            Object::Real(page_height),
        ],
    ));
    ops.push(lopdf::content::Operation::new("Tc", vec![Object::Real(0.0)]));
    ops.push(lopdf::content::Operation::new("Tw", vec![Object::Real(0.0)]));
    ops.push(lopdf::content::Operation::new("Tz", vec![Object::Real(100.0)]));
    ops.push(lopdf::content::Operation::new("BT", vec![]));

    let mut rendered = std::collections::HashSet::new();
    let mut underlines: Vec<UnderlineSpec> = Vec::new();
    for run in lines {
        if !rendered.insert((run.patch_idx, run.line_seq)) {
            continue;
        }
        let (line_ops, underline) = emit_text_line_ops(run, user_unit);
        ops.extend(line_ops);
        if let Some(spec) = underline {
            underlines.push(spec);
        }
    }
    ops.push(lopdf::content::Operation::new("ET", vec![]));

    for spec in &underlines {
        ops.extend(emit_underline_ops(spec));
    }
    ops.push(lopdf::content::Operation::new("Q", vec![]));
    ops
}

fn patch_atomic_reflow_recursive(
    doc: &mut Document,
    page_id: lopdf::ObjectId,
    content: &mut Content,
    resources: &FlatResources,
    res_cache: &mut ResourceCache,
    cluster_map: &HashMap<usize, ReflowCluster>,
    page_height: f32,
    _font_name: &[u8],
    obj_counter: &mut usize,
    state: &mut PdfTextState,
) -> Result<bool, String> {
    let mut modified = false;
    let mut current_font = None;
    let mut silenced = std::collections::HashSet::new();
    for c in cluster_map.values() {
        for p in &c.patches {
            for idx in &p.target_indices {
                silenced.insert(*idx);
            }
        }
    }
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
                            doc,
                            page_id,
                            &state.font_alias,
                            current_font.as_ref().map(|f: &Arc<ParsedFont>| f.as_ref()),
                            &patch.new_text,
                        )?;
                        let active_font = Arc::new(font_info.parsed_font.clone());
                        let layout = break_text_into_lines(
                            &patch.new_text,
                            patch.new_runs.as_ref(),
                            &active_font,
                            state.font_size,
                            patch.wrap_width.unwrap_or(0.0),
                            patch.alignment,
                            patch.line_height,
                            patch.char_spacing,
                            patch.horizontal_scaling,
                        );

                        let trm = multiply_matrices(state.ctm, state.last_tm);
                        let (psx, psy) = (
                            (trm[0].powi(2) + trm[1].powi(2)).sqrt(),
                            (trm[2].powi(2) + trm[3].powi(2)).sqrt(),
                        );
                        let (ax, ay) =
                            (trm[4], PdfCoordinateSpace::normalize_y(trm[5], page_height));

                        let first_base = layout
                            .lines
                            .first()
                            .map(|l| l.baseline_y)
                            .unwrap_or(state.font_size);
                        for (idx, line) in layout.lines.iter().enumerate() {
                            let ly = ay + patch.displacement_y.unwrap_or(0.0)
                                - ((line.baseline_y - first_base) * psy);
                            let lx = ax + (line.offset_x as f32 * psx);
                            state.deferred_lines.push(PersistedTextLinePlan {
                                font_alias: font_info.font_alias.clone(),
                                font_size: state.font_size * psy,
                                encoded_bytes: font_info.encode_text(&line.text)?,
                                tx: lx,
                                ty: ly,
                                width: line.width * psx,
                                color: resolve_line_color(line),
                                is_underline: resolve_line_underline(line),
                                horizontal_scaling: patch.horizontal_scaling,
                                patch_idx: target_idx,
                                line_seq: idx,
                            });
                        }
                        modified = true;
                    }
                }
            }
        }

        if is_show && silenced.contains(&target_idx) {
            *obj_counter += 1;
            let mut muted = op.clone();
            match op_str {
                "Tj" | "'" => muted.operands[0] = Object::String(vec![], StringFormat::Literal),
                "TJ" => muted.operands[0] = Object::Array(vec![]),
                "\"" => muted.operands[2] = Object::String(vec![], StringFormat::Literal),
                _ => {}
            }
            new_ops.push(muted);
            continue;
        }

        match op_str {
            "BT" => {
                state.in_text_block = true;
                state.last_tm = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
                state.tlm = state.last_tm;
            }
            "ET" => state.in_text_block = false,
            "Tc" | "Tw" | "Tz" | "Tr" => {
                if let Some(f) = op.operands.get(0).and_then(|o| {
                    o.as_float()
                        .ok()
                        .or_else(|| o.as_i64().ok().map(|i| i as f32))
                }) {
                    match op_str {
                        "Tc" => state.char_spacing = f,
                        "Tw" => state.word_spacing = f,
                        "Tz" => state.horizontal_scaling = f,
                        "Tr" => state.render_mode = f as i32,
                        _ => {}
                    }
                }
            }
            "Tm" => {
                if let Ok(m) = operands_to_f32(&op.operands) {
                    if m.len() >= 6 {
                        state.last_tm = [m[0], m[1], m[2], m[3], m[4], m[5]];
                        state.tlm = state.last_tm;
                    }
                }
            }
            "Td" | "TD" => {
                if let Ok(p) = operands_to_f32(&op.operands) {
                    state.tlm = multiply_matrices(state.tlm, [1.0, 0.0, 0.0, 1.0, p[0], p[1]]);
                    state.last_tm = state.tlm;
                }
            }
            "T*" => {
                state.tlm =
                    multiply_matrices(state.tlm, [1.0, 0.0, 0.0, 1.0, 0.0, -state.font_size]);
                state.last_tm = state.tlm;
            }
            "q" => state
                .state_stack
                .push((state.last_tm, state.tlm, state.ctm)),
            "Q" => {
                if let Some((tm, tlm, ctm)) = state.state_stack.pop() {
                    state.last_tm = tm;
                    state.tlm = tlm;
                    state.ctm = ctm;
                }
            }
            "cm" => {
                if let Ok(m) = operands_to_f32(&op.operands) {
                    if m.len() >= 6 {
                        state.ctm =
                            multiply_matrices(state.ctm, [m[0], m[1], m[2], m[3], m[4], m[5]]);
                    }
                }
            }
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
                        o.as_float()
                            .ok()
                            .or_else(|| o.as_i64().ok().map(|i| i as f32))
                    }) {
                        state.font_size = s;
                    }
                }
            }
            "Tj" | "TJ" | "'" | "\"" => *obj_counter += 1,
            "Do" => {
                if let Some(name) = op.operands.get(0).and_then(|o| o.as_name().ok()) {
                    if let Some(xid) = resources.get(b"XObject" as &[u8]).and_then(|m| m.get(name))
                    {
                        if let Ok(mut xstream) =
                            doc.get_object(*xid).and_then(|o| o.as_stream().cloned())
                        {
                            if xstream
                                .dict
                                .get(b"Subtype")
                                .ok()
                                .and_then(|o| o.as_name().ok())
                                == Some(b"Form")
                            {
                                if let Ok(data) = xstream.decompressed_content() {
                                    if let Ok(mut sub) = Content::decode(&data) {
                                        let sub_res = read_resources(doc, *xid);
                                        let mut sub_state = state.clone();
                                        if let Ok(m_obj) = xstream.dict.get(b"Matrix") {
                                            if let Ok(m_arr) = m_obj.as_array() {
                                                if let Ok(m) = operands_to_f32(m_arr) {
                                                    if m.len() >= 6 {
                                                        sub_state.ctm = multiply_matrices(
                                                            state.ctm,
                                                            [m[0], m[1], m[2], m[3], m[4], m[5]],
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                        if patch_atomic_reflow_recursive(
                                            doc,
                                            page_id,
                                            &mut sub,
                                            &sub_res,
                                            res_cache,
                                            cluster_map,
                                            page_height,
                                            _font_name,
                                            obj_counter,
                                            &mut sub_state,
                                        )? {
                                            state.deferred_lines.extend(sub_state.deferred_lines);
                                            xstream.set_content(
                                                sub.encode().map_err(|e| e.to_string())?,
                                            );
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
        new_ops.push(op.clone());
    }
    content.operations = new_ops;
    Ok(modified)
}

fn break_text_into_lines(
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
    use pdf_viewer_core::models::{
        LayoutAlignment, LayoutParagraph, LayoutRun, ParagraphStyle, RunStyle,
    };

    let runs = runs.cloned().unwrap_or_else(|| {
        vec![LayoutRun {
            id: "patch-run-0".into(),
            text: text.to_string(),
            style: RunStyle {
                font_size,
                char_spacing,
                scale_x,
                ..Default::default()
            },
            ..Default::default()
        }]
    });

    let para = LayoutParagraph {
        id: "patch-para-0".into(),
        runs,
        style: ParagraphStyle {
            align: align.unwrap_or(LayoutAlignment::Left),
            line_height: line_height.unwrap_or(1.2).max(0.8),
            ..Default::default()
        },
        ..Default::default()
    };

    layout_paragraph(&para, max_width, |t, _| {
        font.resolve_text_width(t, font_size, char_spacing, scale_x)
    })
}

fn read_page_height(doc: &Document, id: lopdf::ObjectId) -> Result<f32, String> {
    let page_size = pdf_utils::read_page_size(doc, id);
    Ok(page_size.effective_height())
}

fn append_page_annotation(
    doc: &mut Document,
    page_id: lopdf::ObjectId,
    annot_id: lopdf::ObjectId,
) -> Result<(), String> {
    let annots = {
        let dict = doc
            .get_object(page_id)
            .map_err(|e| e.to_string())?
            .as_dict()
            .map_err(|e| e.to_string())?;
        dict.get(b"Annots").ok().cloned()
    };
    match annots {
        Some(Object::Reference(id)) => {
            doc.get_object_mut(id)
                .and_then(|v| v.as_array_mut())
                .map_err(|e| e.to_string())?
                .push(Object::Reference(annot_id));
        }
        Some(Object::Array(mut arr)) => {
            arr.push(Object::Reference(annot_id));
            doc.get_object_mut(page_id)
                .and_then(|o| o.as_dict_mut())
                .map_err(|e| e.to_string())?
                .set("Annots", Object::Array(arr));
        }
        _ => {
            doc.get_object_mut(page_id)
                .and_then(|o| o.as_dict_mut())
                .map_err(|e| e.to_string())?
                .set("Annots", Object::Array(vec![Object::Reference(annot_id)]));
        }
    }
    Ok(())
}

fn remove_page_annotation(
    doc: &mut Document,
    page_id: lopdf::ObjectId,
    annot_id: lopdf::ObjectId,
) -> Result<(), String> {
    let annots = {
        doc.get_object(page_id)
            .map_err(|e| e.to_string())?
            .as_dict()
            .map_err(|e| e.to_string())?
            .get(b"Annots")
            .ok()
            .cloned()
    };
    match annots {
        Some(Object::Reference(id)) => {
            doc.get_object_mut(id)
                .and_then(|v| v.as_array_mut())
                .map_err(|e| e.to_string())?
                .retain(|i| i.as_reference().ok() != Some(annot_id));
        }
        Some(Object::Array(arr)) => {
            let filtered = arr
                .into_iter()
                .filter(|i| i.as_reference().ok() != Some(annot_id))
                .collect::<Vec<_>>();
            let dict = doc
                .get_object_mut(page_id)
                .and_then(|o| o.as_dict_mut())
                .map_err(|e| e.to_string())?;
            if filtered.is_empty() {
                dict.remove(b"Annots");
            } else {
                dict.set("Annots", Object::Array(filtered));
            }
        }
        _ => {}
    }
    Ok(())
}

fn read_page_annotation_refs(
    doc: &Document,
    page_id: lopdf::ObjectId,
) -> Result<Vec<lopdf::ObjectId>, String> {
    let dict = doc
        .get_object(page_id)
        .and_then(|o| o.as_dict())
        .map_err(|e| e.to_string())?;
    match dict.get(b"Annots") {
        Ok(Object::Array(arr)) => Ok(arr.iter().filter_map(|i| i.as_reference().ok()).collect()),
        Ok(Object::Reference(id)) => Ok(doc
            .get_object(*id)
            .and_then(|v| v.as_array())
            .map_err(|e| e.to_string())?
            .iter()
            .filter_map(|i| i.as_reference().ok())
            .collect()),
        _ => Ok(vec![]),
    }
}

fn resolve_line_color(line: &pdf_viewer_core::geometry::layout_engine::VisualLine) -> String {
    line.runs
        .iter()
        .find(|r| !r.text.is_empty())
        .map(|r| r.style.color.clone())
        .filter(|c| !c.trim().is_empty())
        .unwrap_or_else(|| "#000000".to_string())
}
fn resolve_line_underline(line: &pdf_viewer_core::geometry::layout_engine::VisualLine) -> bool {
    line.runs.iter().any(|r| r.style.is_underline)
}
fn parse_pdf_hex_color(color: &str) -> Option<[f32; 3]> {
    let hex = color.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some([r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0])
}
