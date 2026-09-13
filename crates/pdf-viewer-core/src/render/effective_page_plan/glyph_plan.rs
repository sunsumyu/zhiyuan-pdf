//! build_effective_glyph_render_plan — 字形回退渲染计划构建。

use std::collections::HashSet;

use crate::edit::paragraph_overlay::ParagraphRenderOverlay;
use crate::edit::replacement_region::paragraph_replacement_region;
use crate::models::{BoundingBox, GlyphPaintPlan};
use crate::render::source_suppression::{
    glyph_paragraph_matches_overlay_source_text, glyph_run_spatially_matches_replacement_region,
};
use crate::render::viewport_culling::{
    glyph_run_intersects_viewport, paragraph_intersects_viewport, region_intersects_viewport,
};

use super::overlay_predicates::{
    overlay_paragraph_object_ids, overlay_paragraph_object_indices, overlay_renders_last,
    overlay_suppresses_text_source,
};
use super::{EffectiveGlyphRenderEntry, GlyphParagraphRef};

fn process_glyph_paragraph(
    region_index: usize,
    paragraph_index: usize,
    paragraph: &crate::models::glyph::GlyphPaintParagraph,
    overlays: &[ParagraphRenderOverlay],
    viewport_bbox: &BoundingBox,
    entries: &mut Vec<EffectiveGlyphRenderEntry>,
) {
    let paragraph_object_ids = paragraph
        .runs
        .iter()
        .flat_map(|run| run.object_ids.iter().cloned())
        .collect::<HashSet<_>>();
    let paragraph_object_indices = paragraph
        .runs
        .iter()
        .flat_map(|run| run.object_indices.iter().copied())
        .collect::<HashSet<_>>();
    let matching_overlays = overlays
        .iter()
        .filter(|overlay| overlay_suppresses_text_source(overlay))
        .filter(|overlay| {
            let overlay_ids = overlay_paragraph_object_ids(overlay);
            let overlay_indices = overlay_paragraph_object_indices(overlay);
            let object_id_match = !overlay_ids.is_empty()
                && overlay_ids
                    .iter()
                    .any(|object_id| paragraph_object_ids.contains(object_id));
            let object_index_match = !overlay_indices.is_empty()
                && overlay_indices
                    .iter()
                    .any(|object_index| paragraph_object_indices.contains(object_index));
            let replacement_region = paragraph_replacement_region(&overlay.target);
            let spatial_match = paragraph.runs.iter().any(|run| {
                glyph_run_spatially_matches_replacement_region(run, &replacement_region)
            });
            let source_text_match = glyph_paragraph_matches_overlay_source_text(paragraph, overlay);
            object_id_match || object_index_match || spatial_match || source_text_match
        })
        .collect::<Vec<_>>();
    let mut suppressed_run_object_ids = HashSet::<String>::new();
    let mut suppressed_run_indices = HashSet::<usize>::new();
    for overlay in &matching_overlays {
        let overlay_ids = overlay_paragraph_object_ids(overlay);
        let overlay_indices = overlay_paragraph_object_indices(overlay);
        let source_text_match = glyph_paragraph_matches_overlay_source_text(paragraph, overlay);
        suppressed_run_object_ids.extend(overlay_ids.iter().cloned());
        let replacement_region = paragraph_replacement_region(&overlay.target);
        for (run_index, run) in paragraph.runs.iter().enumerate() {
            let object_id_match = !overlay_ids.is_empty()
                && run
                    .object_ids
                    .iter()
                    .any(|object_id| overlay_ids.contains(object_id));
            let object_index_match = !overlay_indices.is_empty()
                && run
                    .object_indices
                    .iter()
                    .any(|object_index| overlay_indices.contains(object_index));
            if source_text_match
                || object_id_match
                || object_index_match
                || glyph_run_spatially_matches_replacement_region(run, &replacement_region)
            {
                suppressed_run_indices.insert(run_index);
            }
        }
    }

    let mut deferred_overlays = Vec::new();
    for overlay in matching_overlays {
        if overlay_renders_last(overlay) {
            deferred_overlays.push((*overlay).clone());
        } else {
            entries.push(EffectiveGlyphRenderEntry::ParagraphOverlay(
                (*overlay).clone(),
            ));
        }
    }

    if paragraph
        .runs
        .iter()
        .any(|run| glyph_run_intersects_viewport(run, viewport_bbox))
    {
        entries.push(EffectiveGlyphRenderEntry::Paragraph(GlyphParagraphRef {
            region_index,
            paragraph_index,
            suppressed_run_object_ids,
            suppressed_run_indices,
        }));
    }

    entries.extend(
        deferred_overlays
            .into_iter()
            .map(EffectiveGlyphRenderEntry::ParagraphOverlay),
    );
}

pub fn build_effective_glyph_render_plan(
    paint_plan: &GlyphPaintPlan,
    viewport_bbox: &BoundingBox,
    overlays: &[ParagraphRenderOverlay],
) -> Vec<EffectiveGlyphRenderEntry> {
    let mut entries = Vec::new();

    for (region_index, region) in paint_plan.regions.iter().enumerate() {
        if !region_intersects_viewport(region, viewport_bbox) {
            continue;
        }
        for (paragraph_index, paragraph) in region.paragraphs.iter().enumerate() {
            if !paragraph_intersects_viewport(paragraph, viewport_bbox) {
                continue;
            }

            process_glyph_paragraph(
                region_index,
                paragraph_index,
                paragraph,
                overlays,
                viewport_bbox,
                &mut entries,
            );
        }
    }

    entries
}
