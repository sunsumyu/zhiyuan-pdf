//! Text and path suppression logic — 从 effective_page_plan.rs 拆分。
//!
//! 处理 overlay 对 vector objects 的文本压制和路径压制决策。

use crate::edit::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use crate::geometry::bbox_ops::bbox_intersects;
use crate::models::{VectorPageModel, VectorRenderObject};
use crate::render::path_suppression::should_suppress;
use crate::render::source_suppression::{matching_text_run_refs, text_object_should_be_suppressed};
use crate::render::viewport_culling::path_bbox;

use super::overlay_ops::{
    insert_overlay_if_needed, overlay_suppresses_row_paths, overlay_suppresses_text_source,
    record_overlay_object_summary, vector_object_bbox, vector_object_summary, PreparedOverlay,
};
use super::{EffectiveVectorRenderEntry, SuppressedVectorTextRuns};

/// 文本压制决策结果。
pub(super) enum TextSuppressionOutcome {
    RunLevel(SuppressedVectorTextRuns),
    NonMarkerRuns,
    NoMatch,
}

pub(super) fn decide_text_suppression(
    object: &VectorRenderObject,
    object_index: usize,
    overlay: &PreparedOverlay,
) -> TextSuppressionOutcome {
    let z_index_hit = matches!(object, VectorRenderObject::Text(text) if overlay.object_indices.contains(&text.z_index));
    let array_index_hit = overlay.object_indices.contains(&object_index);
    let index_hit = z_index_hit || array_index_hit;
    let id_hit =
        matches!(object, VectorRenderObject::Text(text) if overlay.object_ids.contains(&text.id));
    let text_object_index_match =
        matches!(object, VectorRenderObject::Text(_)) && (index_hit || id_hit);
    if matches!(object, VectorRenderObject::Text(_)) {
        let (text_id, text_z) = if let VectorRenderObject::Text(text) = object {
            (text.id.as_str(), text.z_index)
        } else {
            ("", 0)
        };
        dbg_event(
            "effective-plan",
            "suppress-check",
            vec![
                dbg_field("objectIndex", object_index),
                dbg_field("textZIndex", text_z),
                dbg_field("textId", text_id),
                dbg_field(
                    "overlayParagraphId",
                    overlay.overlay.target.paragraph_id.as_str(),
                ),
                dbg_field("zIndexHit", z_index_hit),
                dbg_field("arrayIndexHit", array_index_hit),
                dbg_field("idHit", id_hit),
                dbg_field("matched", text_object_index_match),
            ],
        );
    }
    if text_object_index_match {
        let refs = matching_text_run_refs(object, &overlay.object_ids, &overlay.replacement_region);
        return TextSuppressionOutcome::RunLevel(refs);
    }
    if text_object_should_be_suppressed(object, &overlay.object_ids) {
        return TextSuppressionOutcome::NonMarkerRuns;
    }
    let refs = matching_text_run_refs(object, &overlay.object_ids, &overlay.replacement_region);
    if refs.run_indices.is_empty() && refs.object_ids.is_empty() {
        TextSuppressionOutcome::NoMatch
    } else {
        TextSuppressionOutcome::RunLevel(refs)
    }
}

pub(super) fn apply_text_suppression(
    outcome: TextSuppressionOutcome,
    object: &VectorRenderObject,
    overlay: &mut PreparedOverlay,
    suppressed_text_runs: &mut SuppressedVectorTextRuns,
) -> bool {
    match outcome {
        TextSuppressionOutcome::RunLevel(refs) => {
            let matched_run_count = if let VectorRenderObject::Text(text) = object {
                refs.text_suppressed_count(text)
            } else {
                0
            };
            overlay.suppressed_text_run_count = overlay
                .suppressed_text_run_count
                .saturating_add(matched_run_count);
            overlay.suppressed_text_object_count =
                overlay.suppressed_text_object_count.saturating_add(1);
            suppressed_text_runs.run_indices.extend(refs.run_indices);
            suppressed_text_runs.object_ids.extend(refs.object_ids);
            true
        }
        TextSuppressionOutcome::NonMarkerRuns => {
            overlay.suppressed_text_object_count =
                overlay.suppressed_text_object_count.saturating_add(1);
            if let VectorRenderObject::Text(text) = object {
                for (run_index, _) in text.runs.iter().enumerate() {
                    suppressed_text_runs.run_indices.insert(run_index);
                }
            }
            true
        }
        TextSuppressionOutcome::NoMatch => false,
    }
}

pub(super) fn check_path_suppression(
    object: &VectorRenderObject,
    object_index: usize,
    overlay: &mut PreparedOverlay,
) -> bool {
    if let Some(object_bbox) = vector_object_bbox(object) {
        if bbox_intersects(&object_bbox, &overlay.path_suppression_bbox) {
            overlay.object_intersect_count = overlay.object_intersect_count.saturating_add(1);
            match object {
                VectorRenderObject::Text(_) => {
                    overlay.text_intersect_count = overlay.text_intersect_count.saturating_add(1)
                }
                VectorRenderObject::Path(_) => {
                    overlay.path_intersect_count = overlay.path_intersect_count.saturating_add(1)
                }
                VectorRenderObject::Image(_) => {
                    overlay.image_intersect_count = overlay.image_intersect_count.saturating_add(1)
                }
            }
            record_overlay_object_summary(overlay, vector_object_summary(object, object_index));
        }
    }
    if let Some(path_summary) = should_suppress(
        object,
        &overlay.replacement_region,
        &overlay.path_suppression_bbox,
    ) {
        overlay.thin_horizontal_path_count = overlay.thin_horizontal_path_count.saturating_add(1);
        overlay.suppressed_path_count = overlay.suppressed_path_count.saturating_add(1);
        if overlay.first_path_summary.is_none() {
            overlay.first_path_summary = Some(path_summary);
        }
        return true;
    }
    if let VectorRenderObject::Path(path) = object {
        if let Some(path_bbox) = path_bbox(path) {
            if bbox_intersects(&path_bbox, &overlay.path_suppression_bbox) {
                if overlay.first_path_summary.is_none() {
                    overlay.first_path_summary = Some(format!(
                        "id={} bbox={:.1},{:.1},{:.1},{:.1} stroke={} color={}",
                        path.id,
                        path_bbox.left,
                        path_bbox.top,
                        path_bbox.right,
                        path_bbox.bottom,
                        path.stroke_width,
                        path.stroke_color.as_deref().unwrap_or("none")
                    ));
                }
            }
        }
    }
    false
}

/// 遍历所有可见 objects，应用 text/path 压制，生成 vector render entries。
pub(super) fn process_visible_objects(
    visible_indices: Vec<usize>,
    vector_model: &VectorPageModel,
    prepared_overlays: &mut [PreparedOverlay],
) -> Vec<EffectiveVectorRenderEntry> {
    let mut entries = Vec::with_capacity(visible_indices.len() + prepared_overlays.len());
    for object_index in visible_indices {
        let Some(object) = vector_model.objects.get(object_index) else {
            continue;
        };
        if let VectorRenderObject::Text(text) = object {
            if text.runs.iter().all(|run| run.render_mode == 3) {
                continue;
            }
        }
        let mut suppressed_text_runs = SuppressedVectorTextRuns::default();
        let mut suppress_entire_object = false;
        for overlay in &mut *prepared_overlays {
            let suppress_text_source = overlay_suppresses_text_source(&overlay.overlay);
            let suppress_row_paths = overlay_suppresses_row_paths(&overlay.overlay);
            if suppress_text_source {
                let outcome = decide_text_suppression(object, object_index, overlay);
                if apply_text_suppression(outcome, object, overlay, &mut suppressed_text_runs) {
                    insert_overlay_if_needed(overlay, &mut entries);
                    continue;
                }
            }
            if suppress_row_paths {
                if check_path_suppression(object, object_index, overlay) {
                    suppress_entire_object = true;
                    continue;
                }
            }
        }
        let should_skip_entire_object = match object {
            VectorRenderObject::Text(text) => {
                suppress_entire_object
                    || (!text.runs.is_empty()
                        && suppressed_text_runs.text_suppressed_count(text) == text.runs.len())
            }
            _ => suppress_entire_object,
        };
        if should_skip_entire_object {
            continue;
        }
        entries.push(EffectiveVectorRenderEntry::Object {
            object_index,
            suppressed_text_runs,
        });
    }
    entries
}
