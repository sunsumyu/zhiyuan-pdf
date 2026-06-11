use crate::models::{
    BoundingBox, GlyphPaintParagraph, GlyphPaintRun, LayoutRun, ParagraphEditContext, StyledRun,
    VectorPageModel, VectorRenderObject,
};

use crate::edit::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use crate::geometry::bbox_ops::{bbox_height, bbox_width};
use crate::geometry::source_geometry::{source_run_visual_bbox, source_visual_bbox_from_runs};
use crate::common::debug::truncate_debug_text;

pub fn original_paint_runs_for_target(
    paragraph: &GlyphPaintParagraph,
    body_session: &ParagraphEditContext,
    target: &crate::edit::edit_target::EditorEditTarget,
) -> Vec<GlyphPaintRun> {
    let body_object_indices = body_session
        .paragraph
        .runs
        .iter()
        .flat_map(|run| run.object_indices.iter().copied())
        .collect::<std::collections::BTreeSet<_>>();
    if !body_object_indices.is_empty() {
        let runs = paragraph
            .runs
            .iter()
            .filter(|run| {
                run.object_indices
                    .iter()
                    .any(|index| body_object_indices.contains(index))
            })
            .cloned()
            .collect::<Vec<_>>();
        if !runs.is_empty() {
            return runs;
        }
    }

    let body_object_ids = body_session
        .paragraph
        .runs
        .iter()
        .flat_map(|run| run.object_ids.iter().cloned())
        .collect::<std::collections::BTreeSet<_>>();
    if !body_object_ids.is_empty() {
        let runs = paragraph
            .runs
            .iter()
            .filter(|run| {
                run.object_ids
                    .iter()
                    .any(|object_id| body_object_ids.contains(object_id))
            })
            .cloned()
            .collect::<Vec<_>>();
        if !runs.is_empty() {
            return runs;
        }
    }

    if !target.source_object_ids.is_empty() {
        let runs = paragraph
            .runs
            .iter()
            .filter(|run| {
                run.object_ids
                    .iter()
                    .any(|object_id| target.source_object_ids.contains(object_id))
            })
            .cloned()
            .collect::<Vec<_>>();
        if !runs.is_empty() {
            return runs;
        }
    }

    let runs = target
        .source_run_indices
        .iter()
        .filter_map(|index| paragraph.runs.get(*index).cloned())
        .collect::<Vec<_>>();
    if !runs.is_empty() {
        return runs;
    }

    paragraph.runs.clone()
}

fn summarize_layout_runs(runs: &[LayoutRun]) -> String {
    runs
        .iter()
        .take(10)
        .enumerate()
        .map(|(index, run)| {
            let first_origin = run.char_origins.first().copied().unwrap_or(f32::NAN);
            let last_origin = run.char_origins.last().copied().unwrap_or(f32::NAN);
            format!(
                "#{index} text='{}' x={:.2} y={:.2} bbox=[{:.2},{:.2},{:.2},{:.2}] origins={} first={:.2} last={:.2} font='{}' objs={:?} idx={:?}",
                truncate_debug_text(&run.text, 24),
                run.origin_x,
                run.origin_y,
                run.bbox.left,
                run.bbox.top,
                run.bbox.right,
                run.bbox.bottom,
                run.char_origins.len(),
                first_origin,
                last_origin,
                truncate_debug_text(&run.style.font_name, 24),
                run.object_ids,
                run.object_indices,
            )
        })
        .collect::<Vec<_>>()
        .join(" || ")
}

pub fn resolve_preferred_editor_session(
    paragraph: &GlyphPaintParagraph,
    vector_model: Option<&VectorPageModel>,
) -> Option<ParagraphEditContext> {
    let vector_runs = vector_model
        .and_then(|model| resolve_vector_model_source_runs(paragraph, model))
        .filter(|item| !item.1.is_empty());
    let paint_runs = resolve_glyph_paint_runs(paragraph).filter(|runs| !runs.is_empty());
    let (source, exact_runs) = match (vector_runs, paint_runs) {
        // Vector-model runs are the canonical PDF text source. Paint-plan runs
        // are a rendered projection and may already contain visual gap fixes.
        (Some((source, runs)), _) => (source, runs),
        (None, Some(runs)) => ("paint-plan", runs),
        (None, None) => {
            dbg_event(
                "document-plan.source-runs",
                "missing",
                vec![
                    dbg_field("paragraphId", &paragraph.id),
                    dbg_field("paintRunCount", paragraph.runs.len()),
                    dbg_field("hasVectorModel", vector_model.is_some()),
                ],
            );
            return None;
        }
    };

    let anchor_bbox = source_visual_bbox_from_runs(&exact_runs).unwrap_or_else(|| {
        exact_runs.iter().fold(
            BoundingBox {
                left: f32::INFINITY,
                top: f32::INFINITY,
                right: f32::NEG_INFINITY,
                bottom: f32::NEG_INFINITY,
            },
            |acc, run| BoundingBox {
                left: acc.left.min(run.bbox.left),
                top: acc.top.min(run.bbox.top),
                right: acc.right.max(run.bbox.right),
                bottom: acc.bottom.max(run.bbox.bottom),
            },
        )
    });

    if !anchor_bbox.left.is_finite()
        || !anchor_bbox.top.is_finite()
        || !anchor_bbox.right.is_finite()
        || !anchor_bbox.bottom.is_finite()
    {
        dbg_event(
            "document-plan.source-runs",
            "invalid-anchor",
            vec![
                dbg_field("paragraphId", &paragraph.id),
                dbg_field("source", source),
                dbg_field("runCount", exact_runs.len()),
                dbg_field("runSummary", summarize_layout_runs(&exact_runs)),
            ],
        );
        return None;
    }

    let mut paragraph_layout = paragraph.editor_session.paragraph.clone();
    paragraph_layout.runs = exact_runs;
    paragraph_layout.bbox = anchor_bbox;
    paragraph_layout.origin_x = anchor_bbox.left;
    paragraph_layout.origin_y = anchor_bbox.top;
    paragraph_layout.wrap_width = paragraph_layout
        .wrap_width
        .max((anchor_bbox.right - anchor_bbox.left).max(1.0));

    dbg_event(
        "document-plan.source-runs",
        "resolved",
        vec![
            dbg_field("paragraphId", &paragraph.id),
            dbg_field("source", source),
            dbg_field("runCount", paragraph_layout.runs.len()),
            dbg_field(
                "anchor",
                format!(
                    "[{:.2},{:.2},{:.2},{:.2}]",
                    anchor_bbox.left, anchor_bbox.top, anchor_bbox.right, anchor_bbox.bottom
                ),
            ),
            dbg_field("runSummary", summarize_layout_runs(&paragraph_layout.runs)),
        ],
    );

    Some(ParagraphEditContext {
        anchor_bbox,
        paragraph: paragraph_layout,
    })
}

fn resolve_vector_model_source_runs(
    paragraph: &GlyphPaintParagraph,
    vector_model: &VectorPageModel,
) -> Option<(&'static str, Vec<LayoutRun>)> {
    if let Some(runs) = resolve_vector_model_runs_by_object_id(paragraph, vector_model) {
        return Some(("vector-model", runs));
    }
    resolve_vector_model_runs_by_geometry(paragraph, vector_model)
        .map(|runs| ("vector-geometry", runs))
}

fn resolve_vector_model_runs_by_object_id(
    paragraph: &GlyphPaintParagraph,
    vector_model: &VectorPageModel,
) -> Option<Vec<LayoutRun>> {
    let object_order = resolve_vector_source_object_order(paragraph);
    if object_order.is_empty() {
        return None;
    }
    let object_ids = object_order
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();

    let mut runs_by_object: std::collections::BTreeMap<String, Vec<LayoutRun>> =
        std::collections::BTreeMap::new();
    for object in &vector_model.objects {
        let VectorRenderObject::Text(text) = object else {
            continue;
        };
        if !object_ids.contains(&text.id) {
            continue;
        }
        let mapped_runs = text
            .runs
            .iter()
            .enumerate()
            .filter(|(_, run)| !run.text.is_empty())
            .map(|(run_index, run)| build_layout(run, &text.id, run_index))
            .collect::<Vec<_>>();
        if !mapped_runs.is_empty() {
            runs_by_object.insert(text.id.clone(), mapped_runs);
        }
    }

    let mut exact_runs: Vec<LayoutRun> = Vec::new();
    let mut used_object_ids = std::collections::BTreeSet::new();
    for object_id in object_order {
        if !used_object_ids.insert(object_id.clone()) {
            continue;
        }
        if let Some(mapped_runs) = runs_by_object.get(&object_id) {
            exact_runs.extend(mapped_runs.iter().cloned());
        }
    }

    if exact_runs.is_empty() {
        None
    } else {
        Some(exact_runs)
    }
}

fn bbox_intersection_width(a: BoundingBox, b: BoundingBox) -> f32 {
    (a.right.min(b.right) - a.left.max(b.left)).max(0.0)
}

fn bbox_intersection_height(a: BoundingBox, b: BoundingBox) -> f32 {
    (a.bottom.min(b.bottom) - a.top.max(b.top)).max(0.0)
}

fn expand_bbox(bbox: BoundingBox, x_pad: f32, y_pad: f32) -> BoundingBox {
    BoundingBox {
        left: bbox.left - x_pad,
        top: bbox.top - y_pad,
        right: bbox.right + x_pad,
        bottom: bbox.bottom + y_pad,
    }
}

fn vector_run_matches_paragraph_geometry(run: &LayoutRun, target_bbox: BoundingBox) -> bool {
    let run_bbox = source_run_visual_bbox(run).unwrap_or(run.bbox);
    let run_height = bbox_height(&run_bbox).max(run.style.font_size.max(1.0));
    let vertical_overlap = bbox_intersection_height(run_bbox, target_bbox);
    if vertical_overlap < (run_height.min(bbox_height(&target_bbox).max(1.0)) * 0.25).max(0.8) {
        return false;
    }

    let horizontal_overlap = bbox_intersection_width(run_bbox, target_bbox);
    if horizontal_overlap > 0.0 {
        return true;
    }

    let run_center_x = (run_bbox.left + run_bbox.right) * 0.5;
    run_center_x >= target_bbox.left && run_center_x <= target_bbox.right
}

fn resolve_vector_model_runs_by_geometry(
    paragraph: &GlyphPaintParagraph,
    vector_model: &VectorPageModel,
) -> Option<Vec<LayoutRun>> {
    let y_pad = bbox_height(&paragraph.bbox)
        .max(paragraph.editor_session.anchor_bbox.bottom - paragraph.editor_session.anchor_bbox.top)
        .max(12.0)
        * 0.35;
    let x_pad = bbox_width(&paragraph.bbox).max(24.0) * 0.08;
    let target_bbox = expand_bbox(paragraph.bbox, x_pad, y_pad);
    let mut matched_runs = Vec::new();

    for object in &vector_model.objects {
        let VectorRenderObject::Text(text) = object else {
            continue;
        };
        for (run_index, run) in text.runs.iter().enumerate() {
            if run.text.is_empty() {
                continue;
            }
            let layout_run = build_layout(run, &text.id, run_index);
            if vector_run_matches_paragraph_geometry(&layout_run, target_bbox) {
                matched_runs.push(layout_run);
            }
        }
    }

    if matched_runs.is_empty() {
        return None;
    }

    matched_runs.sort_by(|a, b| {
        a.bbox
            .top
            .partial_cmp(&b.bbox.top)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                a.bbox
                    .left
                    .partial_cmp(&b.bbox.left)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| a.id.cmp(&b.id))
    });
    Some(matched_runs)
}

fn resolve_vector_source_object_order(paragraph: &GlyphPaintParagraph) -> Vec<String> {
    let mut seen = std::collections::BTreeSet::new();
    let mut object_order = Vec::new();

    for object_id in paragraph
        .runs
        .iter()
        .flat_map(|run| run.object_ids.iter().cloned())
    {
        if seen.insert(object_id.clone()) {
            object_order.push(object_id);
        }
    }
    if !object_order.is_empty() {
        return object_order;
    }

    // Patched display runs are a projection and may intentionally drop source
    // object ids. The editor session remains the canonical original-PDF source
    // for rebuilding persisted overlays after commit.
    for object_id in paragraph
        .editor_session
        .paragraph
        .runs
        .iter()
        .flat_map(|run| run.object_ids.iter().cloned())
    {
        if seen.insert(object_id.clone()) {
            object_order.push(object_id);
        }
    }

    object_order
}

fn resolve_glyph_paint_runs(paragraph: &GlyphPaintParagraph) -> Option<Vec<LayoutRun>> {
    let runs = paragraph
        .runs
        .iter()
        .filter(|run| !run.text.is_empty())
        .enumerate()
        .map(|(run_index, run)| layout_run_from_glyph_paint(run, run_index))
        .collect::<Vec<_>>();
    if runs.is_empty() {
        None
    } else {
        Some(runs)
    }
}

fn build_layout(
    run: &StyledRun,
    owner_object_id: &str,
    run_index: usize,
) -> LayoutRun {
    let mut layout_run = LayoutRun::from_styled(run);
    if layout_run.id.is_empty() {
        layout_run.id = format!("{owner_object_id}::run::{run_index}");
    }
    if layout_run.object_ids.is_empty() {
        layout_run.object_ids.push(owner_object_id.to_string());
    }
    layout_run
}

fn layout_run_from_glyph_paint(run: &GlyphPaintRun, run_index: usize) -> LayoutRun {
    LayoutRun {
        id: if run.id.is_empty() {
            format!("paint-run::{run_index}")
        } else {
            run.id.clone()
        },
        text: run.text.clone(),
        style: crate::models::RunStyle {
            font_name: run.resolved_font.render_family.clone(),
            font_size: run.font_size,
            color: run.color.clone(),
            is_bold: run.is_bold,
            is_italic: run.is_italic,
            is_underline: run.is_underline,
            char_spacing: 0.0,
            scale_x: run.scale_x.max(0.01),
        },
        bbox: run.bbox,
        origin_x: run.origin_x,
        origin_y: run.origin_y,
        char_origins: run.char_origins.clone(),
        char_widths: Vec::new(),
        object_ids: run.object_ids.clone(),
        object_indices: run.object_indices.clone(),
    }
}
