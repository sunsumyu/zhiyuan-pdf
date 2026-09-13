//! Overlay 判定与预置：viewport 命中、来源抑制集合、绘制顺序。

use std::collections::HashSet;

use crate::edit::paragraph_overlay::{ParagraphRenderOverlay, ParagraphRenderOverlayOwner};
use crate::edit::replacement_region::paragraph_replacement_region;
use crate::edit::source_identity::{
    collect_target_source_object_ids, collect_target_source_object_indices_set,
};
use crate::models::BoundingBox;

use super::PreparedOverlay;

pub(super) fn overlay_paragraph_object_ids(overlay: &ParagraphRenderOverlay) -> HashSet<String> {
    collect_target_source_object_ids(&overlay.target)
}

pub(super) fn overlay_paragraph_object_indices(overlay: &ParagraphRenderOverlay) -> HashSet<usize> {
    let mut object_indices = collect_target_source_object_indices_set(&overlay.target);
    object_indices.extend(overlay.source_object_indices.iter().copied());
    object_indices
}

pub(super) fn overlay_renders_last(overlay: &ParagraphRenderOverlay) -> bool {
    matches!(
        overlay.owner,
        ParagraphRenderOverlayOwner::ActiveEditorShell
            | ParagraphRenderOverlayOwner::PersistedPageCanvas
    )
}

pub(super) fn overlay_suppresses_text_source(overlay: &ParagraphRenderOverlay) -> bool {
    overlay.replaces_source
}

pub(super) fn overlay_suppresses_row_paths(overlay: &ParagraphRenderOverlay) -> bool {
    matches!(
        overlay.owner,
        ParagraphRenderOverlayOwner::ActiveEditorShell
            | ParagraphRenderOverlayOwner::PersistedPageCanvas
    )
}

pub(super) fn overlay_intersects_viewport(
    overlay: &ParagraphRenderOverlay,
    viewport_bbox: &BoundingBox,
    page_width: f32,
) -> bool {
    let replacement_region = paragraph_replacement_region(&overlay.target);
    let cull_bbox = replacement_region.viewport_cull_bbox_for_page_width(page_width);
    cull_bbox.left <= viewport_bbox.right
        && cull_bbox.right >= viewport_bbox.left
        && cull_bbox.top <= viewport_bbox.bottom
        && cull_bbox.bottom >= viewport_bbox.top
}

pub(super) fn prepare_overlays(
    overlays: &[ParagraphRenderOverlay],
    viewport_bbox: &BoundingBox,
    page_width: f32,
) -> Vec<PreparedOverlay> {
    overlays
        .iter()
        .filter(|o| overlay_intersects_viewport(o, viewport_bbox, page_width))
        .cloned()
        .map(|overlay| {
            let rr = paragraph_replacement_region(&overlay.target);
            PreparedOverlay {
                object_ids: if overlay_suppresses_text_source(&overlay) {
                    overlay_paragraph_object_ids(&overlay)
                } else {
                    HashSet::new()
                },
                object_indices: if overlay_suppresses_text_source(&overlay) {
                    overlay_paragraph_object_indices(&overlay)
                } else {
                    HashSet::new()
                },
                path_suppression_bbox: if overlay_suppresses_row_paths(&overlay) {
                    rr.row_path_suppression_bbox_for_page_width(page_width)
                } else {
                    BoundingBox::default()
                },
                replacement_region: rr,
                overlay,
                inserted: false,
                suppressed_text_object_count: 0,
                suppressed_text_run_count: 0,
                object_intersect_count: 0,
                text_intersect_count: 0,
                path_intersect_count: 0,
                image_intersect_count: 0,
                thin_horizontal_path_count: 0,
                suppressed_path_count: 0,
                first_path_summary: None,
                object_summary_1: None,
                object_summary_2: None,
                object_summary_3: None,
            }
        })
        .collect::<Vec<_>>()
}

pub(super) fn insert_overlay_if_needed(
    o: &mut PreparedOverlay,
    e: &mut Vec<super::EffectiveVectorRenderEntry>,
) {
    if !o.inserted && !overlay_renders_last(&o.overlay) {
        e.push(super::EffectiveVectorRenderEntry::ParagraphOverlay(
            o.overlay.clone(),
        ));
        o.inserted = true;
    }
}
