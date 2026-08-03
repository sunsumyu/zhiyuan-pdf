use crate::document::page_region_context::{
    FieldGroupSnapshot, ParagraphRegionSnapshot, StyleRunSnapshot,
};
use crate::models::{FontHints, GlyphPaintRun, PaintMode, ResolvedFontFace};
use crate::typography::font_resolver::resolve_font_face;

fn to_paint_mode(render_mode: Option<i64>) -> PaintMode {
    match render_mode {
        Some(1) => PaintMode::Stroke,
        Some(2) => PaintMode::FillStroke,
        _ => PaintMode::Fill,
    }
}

// In a real WASM env, this might need an external resolver. For core, we provide a fallback generator
// or we can pass a trait / closure for resolving fonts.
pub fn build_resolved_font_face(
    font_name: &str,
    is_bold: bool,
    is_italic: bool,
    font_hints: Option<&FontHints>,
) -> ResolvedFontFace {
    let synthesized_hints;
    let hints = match font_hints {
        Some(existing) => existing,
        None => {
            synthesized_hints = FontHints {
                flags: 0,
                weight: if is_bold { 700 } else { 400 },
                italic_angle: if is_italic { -12.0 } else { 0.0 },
                ascent: 0.0,
                descent: 0.0,
                cap_height: 0.0,
                x_height: 0.0,
                is_fixed_pitch: false,
                is_serif: false,
                is_italic,
                is_bold,
                ..Default::default()
            };
            &synthesized_hints
        }
    };

    resolve_font_face(font_name, Some(hints))
}

fn build_run_bbox(
    left: f32,
    bottom: f32,
    font_size: f32,
    width: f32,
    char_origins: &[f32],
    char_widths: &[f32],
) -> crate::models::BoundingBox {
    let final_width = if !char_origins.is_empty() {
        let last_index = char_origins.len() - 1;
        let last_origin = char_origins[last_index];
        let last_width = if last_index < char_widths.len() {
            char_widths[last_index]
        } else {
            width / (char_origins.len().max(1) as f32)
        };
        width.max(last_origin + last_width)
    } else {
        width
    };

    crate::models::BoundingBox {
        left,
        top: bottom - font_size,
        right: left + final_width,
        bottom,
    }
}

fn build_snapshot_paint_run(
    page_index: u16,
    region_id: &str,
    paragraph_id: &str,
    object_ids: &[String],
    text: &str,
    left: f32,
    bottom: f32,
    run: &StyleRunSnapshot,
) -> GlyphPaintRun {
    let scale_x = run.style.scale_x;
    GlyphPaintRun {
        id: run.id.clone(),
        page_index,
        region_id: region_id.to_string(),
        paragraph_id: paragraph_id.to_string(),
        text: text.to_string(),
        bbox: build_run_bbox(
            left,
            bottom,
            run.style.font_size,
            run.width,
            &run.char_origins,
            &run.char_widths,
        ),
        origin_x: left,
        origin_y: bottom,
        char_origins: run.char_origins.clone(),
        color: run.style.color.clone(),
        resolved_font: build_resolved_font_face(
            &run.style.font_name,
            run.style.is_bold,
            run.style.is_italic,
            run.style.font_hints.as_ref(),
        ),
        font_size: run.style.font_size,
        scale_x,
        is_bold: run.style.is_bold,
        is_italic: run.style.is_italic,
        is_underline: run.style.is_underline,
        paint_mode: to_paint_mode(Some(run.style.render_mode)),
        object_ids: object_ids.to_vec(),
        object_indices: run.object_indices.clone(),
    }
}

pub struct RunLayout {
    /// run 的绝对页面 X 坐标（不再相对于 line_left）
    pub absolute_left: f32,
    pub width: f32,
    /// glyph 的绝对页面 X 坐标（不再归零）
    pub absolute_glyph_positions: Vec<f32>,
}

/// 计算 run 的绝对坐标布局
/// 输入：StyleRunSnapshot.char_origins 是相对于 line_left 的偏移
/// 输出：RunLayout.absolute_left 和 absolute_glyph_positions 都是绝对页面坐标
pub fn resolve_run_layout<F>(
    line_left: f32,
    cursor_left: f32,
    run: &StyleRunSnapshot,
    measure_text: &F,
) -> RunLayout
where
    F: Fn(&str, f32, &str, bool, bool) -> f32,
{
    let measured_width = measure_text(
        &run.text,
        run.style.font_size,
        &run.style.font_name,
        run.style.is_bold,
        run.style.is_italic,
    );

    if !run.char_origins.is_empty() {
        // 直接转换为绝对坐标，不再归零
        let absolute_glyph_positions: Vec<f32> = run
            .char_origins
            .iter()
            .map(|origin| line_left + *origin)
            .collect();

        let absolute_left = absolute_glyph_positions[0];
        let last_index = absolute_glyph_positions.len() - 1;
        let last_glyph_x = absolute_glyph_positions[last_index];
        let last_width = if last_index < run.char_widths.len() {
            run.char_widths[last_index]
        } else {
            if run.width > 0.0 {
                run.width / (run.char_origins.len().max(1) as f32)
            } else {
                measured_width / (run.char_origins.len().max(1) as f32)
            }
        };
        let final_width = run
            .width
            .max(measured_width)
            .max(last_glyph_x + last_width - absolute_left)
            .max(1.0);
        return RunLayout {
            absolute_left,
            width: final_width,
            absolute_glyph_positions,
        };
    }

    // 没有 char_origins 时，使用 cursor_left 作为起点
    RunLayout {
        absolute_left: cursor_left,
        width: run.width.max(measured_width).max(1.0),
        absolute_glyph_positions: vec![],
    }
}

pub fn build_paragraph_snapshot_paint_runs<F>(
    snapshot: &ParagraphRegionSnapshot,
    page_index: u16,
    measure_text: &F,
) -> Vec<GlyphPaintRun>
where
    F: Fn(&str, f32, &str, bool, bool) -> f32,
{
    let mut runs = Vec::new();
    for line in &snapshot.lines {
        let paragraph_id = format!("{}::line::{}", snapshot.region_id, line.line_index);

        if let Some(marker_runs) = &line.marker_runs {
            if !marker_runs.is_empty() {
                let mut marker_cursor_left = line.left;
                crate::common::trace::emit(
                    crate::common::trace::TraceLevel::Debug,
                    "marker-render".to_string(),
                    "start".to_string(),
                    vec![
                        crate::common::trace::field("lineLeft", line.left),
                        crate::common::trace::field(
                            "bodyLeft",
                            line.body_left.unwrap_or(line.left),
                        ),
                        crate::common::trace::field("markerRunCount", marker_runs.len()),
                    ],
                );
                for (run_idx, run) in marker_runs.iter().enumerate() {
                    let layout =
                        resolve_run_layout(line.left, marker_cursor_left, run, measure_text);
                    crate::common::trace::emit(
                        crate::common::trace::TraceLevel::Debug,
                        "marker-render".to_string(),
                        "run-layout".to_string(),
                        vec![
                            crate::common::trace::field("runIdx", run_idx),
                            crate::common::trace::field("runText", run.text.as_str()),
                            crate::common::trace::field("runWidth", run.width),
                            crate::common::trace::field(
                                "runCharOrigins",
                                format!("{:?}", run.char_origins),
                            ),
                            crate::common::trace::field("lineLeft", line.left),
                            crate::common::trace::field("markerCursorLeft", marker_cursor_left),
                            crate::common::trace::field("layoutAbsoluteLeft", layout.absolute_left),
                            crate::common::trace::field("layoutWidth", layout.width),
                        ],
                    );
                    // 使用绝对坐标构建 GlyphPaintRun
                    let mut synth_run = run.clone();
                    synth_run.width = layout.width;
                    // char_origins 存储绝对坐标（相对于 origin_x 的偏移 = 绝对坐标 - origin_x）
                    let absolute_left = layout.absolute_left;
                    synth_run.char_origins = layout
                        .absolute_glyph_positions
                        .iter()
                        .map(|x| x - absolute_left)
                        .collect();
                    runs.push(build_snapshot_paint_run(
                        page_index,
                        &snapshot.region_id,
                        &format!("{}::marker", paragraph_id),
                        &line.object_ids,
                        &run.text,
                        layout.absolute_left,
                        line.bottom,
                        &synth_run,
                    ));
                    marker_cursor_left =
                        marker_cursor_left.max(layout.absolute_left + layout.width);
                }
            }
        }

        if !line.style_runs.is_empty() {
            let body_left = line.body_left.unwrap_or(line.left);
            let mut cursor_left = body_left;
            for run in &line.style_runs {
                let layout = resolve_run_layout(body_left, cursor_left, run, measure_text);
                // 使用绝对坐标构建 GlyphPaintRun
                let mut synth_run = run.clone();
                synth_run.width = layout.width;
                let absolute_left = layout.absolute_left;
                synth_run.char_origins = layout
                    .absolute_glyph_positions
                    .iter()
                    .map(|x| x - absolute_left)
                    .collect();
                runs.push(build_snapshot_paint_run(
                    page_index,
                    &snapshot.region_id,
                    &paragraph_id,
                    &line.object_ids,
                    &run.text,
                    layout.absolute_left,
                    line.bottom,
                    &synth_run,
                ));
                cursor_left = cursor_left.max(layout.absolute_left + layout.width);
            }
            continue;
        }

        if !line.rendered_text.is_empty() {
            let synthetic_run = StyleRunSnapshot {
                id: format!("{}::synthetic", paragraph_id),
                text: line.rendered_text.clone(),
                start: 0,
                end: line.rendered_text.chars().count(),
                style: crate::document::page_region_context::StyleSource {
                    font_name: line.font_name.clone(),
                    font_size: line.font_size,
                    color: line.color.clone(),
                    is_bold: line.is_bold,
                    is_italic: line.is_italic,
                    is_underline: line.is_underline,
                    font_hints: line.font_hints.clone(),
                    render_mode: line.render_mode.unwrap_or(0),
                    char_spacing: line.char_spacing,
                    scale_x: line.scale_x,
                },
                width: line.width,
                char_origins: line.char_origins.clone(),
                char_widths: line.char_widths.clone(),
                object_ids: line.object_ids.clone(),
                object_indices: line.object_indices.clone(),
            };
            runs.push(build_snapshot_paint_run(
                page_index,
                &snapshot.region_id,
                &paragraph_id,
                &line.object_ids,
                &line.rendered_text,
                line.body_left.unwrap_or(line.left),
                line.bottom,
                &synthetic_run,
            ));
        }
    }
    runs
}

pub fn build_field_group_snapshot_paint_runs(
    snapshot: &FieldGroupSnapshot,
    baseline_y: f32,
    page_index: u16,
) -> Vec<GlyphPaintRun> {
    let mut runs = Vec::new();
    let region_id = &snapshot.group_id;
    let key_paragraph_id = format!("{}::key", snapshot.group_id);
    let value_paragraph_id = format!("{}::value", snapshot.group_id);

    for run in &snapshot.key_runs {
        runs.push(build_snapshot_paint_run(
            page_index,
            region_id,
            &key_paragraph_id,
            &snapshot.object_ids,
            &run.text,
            snapshot.key_box.left,
            baseline_y,
            run,
        ));
    }

    for run in &snapshot.value_runs {
        runs.push(build_snapshot_paint_run(
            page_index,
            region_id,
            &value_paragraph_id,
            &snapshot.object_ids,
            &run.text,
            snapshot.value_box.left,
            baseline_y,
            run,
        ));
    }

    runs
}
