use crate::infrastructure::pdf::models::{NativePathModel, NativeImageModel, PathSegment, RenderObject, StyledRun};
use crate::infrastructure::pdf::font::{resolve_glyph_geom, ResourceCache, ParsedFont};
use crate::infrastructure::pdf::font::path::simplify_path_segments;
use crate::infrastructure::pdf::pdf_read::graphics_state::GraphicsState;
use crate::infrastructure::pdf::pdf_read::resource_reader::{read_resources, FlatResources, find_xobject_by_name};
use crate::infrastructure::pdf::pdf_read::utils::operands_to_f32;
use lopdf::{content::Content, Document, Object};
use std::sync::Arc;
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
                        state.text.op_cm([m[0], m[1], m[2], m[3], m[4], m[5]]);
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
            "g" => {
                if let Ok(p) = operands_to_f32(&op.operands) {
                    if let Some(&gray) = p.first() {
                        state.fill_color = Some(crate::infrastructure::pdf::color::gray_to_hex(gray));
                    }
                }
            }
            "G" => {
                if let Ok(p) = operands_to_f32(&op.operands) {
                    if let Some(&gray) = p.first() {
                        state.stroke_color = Some(crate::infrastructure::pdf::color::gray_to_hex(gray));
                    }
                }
            }
            "k" => {
                if let Ok(p) = operands_to_f32(&op.operands) {
                    if p.len() >= 4 {
                        let (r, g, b) = crate::infrastructure::pdf::color::cmyk_to_rgb(p[0], p[1], p[2], p[3]);
                        state.fill_color = Some(format!("#{:02x}{:02x}{:02x}", r, g, b));
                    }
                }
            }
            "K" => {
                if let Ok(p) = operands_to_f32(&op.operands) {
                    if p.len() >= 4 {
                        let (r, g, b) = crate::infrastructure::pdf::color::cmyk_to_rgb(p[0], p[1], p[2], p[3]);
                        state.stroke_color = Some(format!("#{:02x}{:02x}{:02x}", r, g, b));
                    }
                }
            }
            "rg" | "sc" | "scn" => {
                if let Ok(p) = operands_to_f32(&op.operands) {
                    if p.len() == 1 {
                        state.fill_color = Some(crate::infrastructure::pdf::color::gray_to_hex(p[0]));
                    } else if p.len() >= 3 {
                        let (r, g, b) = if p.len() >= 4 {
                            crate::infrastructure::pdf::color::cmyk_to_rgb(p[0], p[1], p[2], p[3])
                        } else {
                            (
                                (p[0].clamp(0.0, 1.0) * 255.0) as u8,
                                (p[1].clamp(0.0, 1.0) * 255.0) as u8,
                                (p[2].clamp(0.0, 1.0) * 255.0) as u8,
                            )
                        };
                        state.fill_color = Some(format!("#{:02x}{:02x}{:02x}", r, g, b));
                    }
                }
            }
            "RG" | "SC" | "SCN" => {
                if let Ok(p) = operands_to_f32(&op.operands) {
                    if p.len() == 1 {
                        state.stroke_color = Some(crate::infrastructure::pdf::color::gray_to_hex(p[0]));
                    } else if p.len() >= 3 {
                        let (r, g, b) = if p.len() >= 4 {
                            crate::infrastructure::pdf::color::cmyk_to_rgb(p[0], p[1], p[2], p[3])
                        } else {
                            (
                                (p[0].clamp(0.0, 1.0) * 255.0) as u8,
                                (p[1].clamp(0.0, 1.0) * 255.0) as u8,
                                (p[2].clamp(0.0, 1.0) * 255.0) as u8,
                            )
                        };
                        state.stroke_color = Some(format!("#{:02x}{:02x}{:02x}", r, g, b));
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
                        let pt = state.text.transform_point(p[0], p[1]);
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
                        let pt = state.text.transform_point(p[0], p[1]);
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
                        let p1 = state.text.transform_point(x, y);
                        let p2 = state.text.transform_point(x + w, y);
                        let p3 = state.text.transform_point(x + w, y + h);
                        let p4 = state.text.transform_point(x, y + h);
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
                state.text.op_bt();
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
                        .unwrap_or(state.text.font_size);
                    state.text.font_size = size;
                    if let Some(font_id) = flat_resources
                        .get(b"Font" as &[u8])
                        .and_then(|m| m.get(name))
                    {
                        if let Some(cached) = res_cache.fonts.get(font_id) {
                            state.current_font = Some(cached.clone());
                        } else if let Ok(parsed) =
                            crate::infrastructure::pdf::font::parse_font_from_dict(
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
            "TL" | "Tc" | "Tw" | "Tz" | "Ts" => {
                if let Some(v) = op.operands.get(0).and_then(|o| {
                    o.as_float()
                        .ok()
                        .or_else(|| o.as_i64().ok().map(|i| i as f32))
                }) {
                    match op_str {
                        "TL" => state.text.tl = v,
                        "Tc" => state.text.char_spacing = v,
                        "Tw" => state.text.word_spacing = v,
                        "Tz" => state.text.horizontal_scaling = v,
                        _ => state.text_rise = v,
                    }
                }
            }
            "Tr" => {
                if let Some(v) = op.operands.get(0).and_then(|o| o.as_i64().ok()) {
                    state.text.render_mode = v as i32;
                }
            }
            "Td" => {
                if let Ok(p) = operands_to_f32(&op.operands) {
                    if p.len() >= 2 {
                        state.text.op_td(p[0], p[1]);
                    }
                }
            }
            // `TD tx ty` is `TL -ty` followed by `Td tx ty` (PDF spec).
            "TD" => {
                if let Ok(p) = operands_to_f32(&op.operands) {
                    if p.len() >= 2 {
                        state.text.op_td_with_leading(p[0], p[1]);
                    }
                }
            }
            // `T*` moves to the next line by the text leading (`TL`).
            "T*" => state.text.op_t_star(),
            "Tm" => {
                if let Ok(m) = operands_to_f32(&op.operands) {
                    if m.len() >= 6 {
                        state.text.op_tm([m[0], m[1], m[2], m[3], m[4], m[5]]);
                    }
                }
            }
            "Tj" | "TJ" => {
                *obj_counter += 1;
                if let Some(ref font) = state.current_font {
                    let h_scale = state.text.horizontal_scaling / 100.0;
                    let (text, origins, widths, codes, advance) = if op_str == "Tj" {
                        resolve_glyph_geom(
                            op.operands[0].as_str().unwrap_or(&[]),
                            font,
                            state.text.font_size,
                            h_scale,
                            state.text.char_spacing,
                            state.text.word_spacing,
                        )
                    } else {
                        op.operands[0]
                            .as_array()
                            .map(|arr| {
                                resolve_tj_array_text(
                                    arr,
                                    font,
                                    state.text.font_size,
                                    h_scale,
                                    state.text.char_spacing,
                                    state.text.word_spacing,
                                )
                            })
                            .unwrap_or_default()
                    };

                    let trm = state.text.text_render_matrix();
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
                        font_size: (state.text.font_size * trm[3]).abs(),
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
                        horizontal_scaling: state.text.horizontal_scaling,
                        char_spacing: state.text.char_spacing,
                        word_spacing: state.text.word_spacing,
                        render_mode: state.text.render_mode as i64,
                        ..Default::default()
                    });
                    // Tm update uses the original text-space advance (not page-scaled)
                    state.text.core.advance_text(advance);
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
                    let final_fill_color = apply_alpha_to_color(&state.fill_color, state.fill_alpha);
                    let final_stroke_color = apply_alpha_to_color(&state.stroke_color, state.stroke_alpha);

                    let bbox @ (min_x, min_y, max_x, max_y) =
                        compute_segments_bbox(&current_segments).unwrap_or((0.0, 0.0, 0.0, 0.0));
                    let _ = bbox;
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
                                                            sub_state.text.op_cm([
                                                                m[0], m[1], m[2], m[3], m[4],
                                                                m[5],
                                                            ]);
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
                                            crate::infrastructure::pdf::pdf_read::image_builder::build_image_as_jpeg(
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
                                                let ctm = state.text.ctm();
                                                crate::pdf_log!(
                                                    3,
                                                    "[PDF-IMG] Do image id={} {}x{} ctm=[{:.1},{:.1},{:.1},{:.1},{:.1},{:.1}] jpeg={}",
                                                    asset_id, img_w, img_h, ctm[0], ctm[1], ctm[2], ctm[3], ctm[4], ctm[5], is_jpeg
                                                );
                                                let corners = [
                                                    state.text.transform_point(0.0, 0.0),
                                                    state.text.transform_point(1.0, 0.0),
                                                    state.text.transform_point(0.0, 1.0),
                                                    state.text.transform_point(1.0, 1.0),
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

// ── content-stream helpers (relocated from pdf_read::utils; single-consumer) ──

/// Apply alpha into a `#rrggbb` color, producing `#rrggbbaa` when alpha < 1.0.
/// Fully-opaque colors are returned unchanged to preserve existing output.
fn apply_alpha_to_color(color: &Option<String>, alpha: f32) -> Option<String> {
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
fn compute_segments_bbox(segments: &[PathSegment]) -> Option<(f32, f32, f32, f32)> {
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for seg in segments {
        for pt in &seg.points {
            if pt[0] < min_x { min_x = pt[0]; }
            if pt[0] > max_x { max_x = pt[0]; }
            if pt[1] < min_y { min_y = pt[1]; }
            if pt[1] > max_y { max_y = pt[1]; }
        }
    }
    if min_x.is_infinite() { None } else { Some((min_x, min_y, max_x, max_y)) }
}

/// Resolve a TJ array (mixed strings and kerning adjustments) into unified text geometry.
/// Each string element is resolved via `resolve_glyph_geom`; numeric elements adjust the
/// horizontal offset (negative kern). Returns (text, origins, widths, codes, total_advance).
fn resolve_tj_array_text(
    items: &[Object],
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



