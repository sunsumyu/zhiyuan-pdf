//! build_effective_vector_render_plan — 覆盖层抑制 + 可见对象渲染计划构建。

use crate::geometry::bbox_ops::bbox_intersects;
use crate::models::{BoundingBox, VectorPageModel, VectorRenderObject};
use crate::render::path_suppression::should_suppress;
use crate::render::prepared_scene::PreparedPageScene;
use crate::render::source_suppression::{matching_text_run_refs, text_object_should_be_suppressed};
use crate::render::viewport_culling::{path_object_bbox, vector_object_intersects_viewport};

use super::object_summary::{
    record_overlay_object_summary, vector_object_bbox, vector_object_summary,
};
use super::overlay_predicates::{
    insert_overlay_if_needed, overlay_suppresses_row_paths, overlay_suppresses_text_source,
    prepare_overlays,
};
use super::trace::{trace_overlay_identity, trace_overlay_summary};
use super::{EffectiveVectorRenderEntry, PreparedOverlay, SuppressedVectorTextRuns};

fn resolve_visible_indices(
    vector_model: &VectorPageModel,
    prepared_scene: Option<&PreparedPageScene>,
    viewport_bbox: &BoundingBox,
) -> Vec<usize> {
    prepared_scene
        .map(|scene| scene.visible_vector_indices(viewport_bbox))
        .unwrap_or_else(|| {
            vector_model
                .objects
                .iter()
                .enumerate()
                .filter_map(|(index, obj)| {
                    if vector_object_intersects_viewport(obj, viewport_bbox) {
                        Some(index)
                    } else {
                        None
                    }
                })
                .collect()
        })
}

fn build_entries_without_overlays(
    vi: Vec<usize>,
    vm: &VectorPageModel,
) -> Vec<EffectiveVectorRenderEntry> {
    vi.into_iter()
        .filter(|&oi| {
            if let Some(VectorRenderObject::Text(t)) = vm.objects.get(oi) {
                !t.runs.iter().all(|r| r.render_mode == 3)
            } else {
                true
            }
        })
        .map(|oi| EffectiveVectorRenderEntry::Object {
            object_index: oi,
            suppressed_text_runs: SuppressedVectorTextRuns::default(),
        })
        .collect()
}

enum TextSuppressionOutcome {
    RunLevel(SuppressedVectorTextRuns),
    NonMarkerRuns,
    NoMatch,
}

fn decide_text_suppression(
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
        crate::edit::debug_trace::record_editor_debug_event(
            "effective-plan",
            "suppress-check",
            vec![
                crate::edit::debug_trace::editor_debug_field("objectIndex", object_index),
                crate::edit::debug_trace::editor_debug_field("textZIndex", text_z),
                crate::edit::debug_trace::editor_debug_field("textId", text_id),
                crate::edit::debug_trace::editor_debug_field(
                    "overlayParagraphId",
                    overlay.overlay.target.paragraph_id.as_str(),
                ),
                crate::edit::debug_trace::editor_debug_field("zIndexHit", z_index_hit),
                crate::edit::debug_trace::editor_debug_field("arrayIndexHit", array_index_hit),
                crate::edit::debug_trace::editor_debug_field("idHit", id_hit),
                crate::edit::debug_trace::editor_debug_field("matched", text_object_index_match),
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

fn apply_text_suppression(
    outcome: TextSuppressionOutcome,
    object: &VectorRenderObject,
    overlay: &mut PreparedOverlay,
    suppressed_text_runs: &mut SuppressedVectorTextRuns,
) -> bool {
    match outcome {
        TextSuppressionOutcome::RunLevel(refs) => {
            let matched_run_count = if let VectorRenderObject::Text(text) = object {
                refs.suppressed_count_for_text_object(text)
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
                for (run_index, run) in text.runs.iter().enumerate() {
                    if !crate::render::source_suppression::run_text_is_list_marker_only(&run.text) {
                        suppressed_text_runs.run_indices.insert(run_index);
                    }
                }
            }
            true
        }
        TextSuppressionOutcome::NoMatch => false,
    }
}

fn check_path_suppression(
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
        object_index,
        &overlay.overlay.graphic_markers,
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
        if let Some(path_bbox) = path_object_bbox(path) {
            if bbox_intersects(&path_bbox, &overlay.path_suppression_bbox)
                && overlay.first_path_summary.is_none()
            {
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
    false
}

fn process_visible_objects(
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
            if suppress_row_paths && check_path_suppression(object, object_index, overlay) {
                suppress_entire_object = true;
                continue;
            }
        }
        let should_skip_entire_object = match object {
            VectorRenderObject::Text(text) => {
                suppress_entire_object
                    || (!text.runs.is_empty()
                        && suppressed_text_runs.suppressed_count_for_text_object(text)
                            == text.runs.len())
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

pub fn build_effective_vector_render_plan(
    vector_model: &VectorPageModel,
    prepared_scene: Option<&PreparedPageScene>,
    viewport_bbox: &BoundingBox,
    overlays: &[crate::edit::paragraph_overlay::ParagraphRenderOverlay],
) -> Vec<EffectiveVectorRenderEntry> {
    let visible_indices = resolve_visible_indices(vector_model, prepared_scene, viewport_bbox);
    let mut prepared_overlays = prepare_overlays(overlays, viewport_bbox, vector_model.width);
    trace_overlay_identity(&prepared_overlays, &visible_indices, vector_model);

    if prepared_overlays.is_empty() {
        return build_entries_without_overlays(visible_indices, vector_model);
    }

    let mut entries =
        process_visible_objects(visible_indices, vector_model, &mut prepared_overlays);

    for overlay in prepared_overlays {
        trace_overlay_summary(&overlay);
        if !overlay.inserted {
            entries.push(EffectiveVectorRenderEntry::ParagraphOverlay(
                overlay.overlay,
            ));
        }
    }

    entries
}
