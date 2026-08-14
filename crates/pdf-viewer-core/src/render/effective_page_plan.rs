//! Effective page render plan — 从 ui::render::effective_page_plan 迁入。
//! 纯计算 + 调试事件追踪；无 wasm 依赖。

use std::collections::HashSet;

use crate::edit::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use crate::edit::paragraph_overlay::{ParagraphRenderOverlay, ParagraphRenderOverlayOwner};
use crate::edit::replacement_region::{paragraph_replacement_region, ParagraphReplacementRegion};
use crate::edit::source_identity::{
    collect_target_source_object_ids, collect_target_source_object_indices_set,
};
use crate::geometry::bbox_ops::bbox_intersects;
use crate::models::{BoundingBox, GlyphPaintPlan, VectorPageModel, VectorRenderObject};
use crate::render::path_suppression::should_suppress;
use crate::render::prepared_scene::PreparedPageScene;
use crate::render::source_suppression::{
    glyph_paragraph_matches_overlay_source_text, glyph_run_spatially_matches_replacement_region,
    matching_text_run_refs, text_object_should_be_suppressed,
};
use crate::render::viewport_culling::{
    glyph_run_intersects_viewport, paragraph_intersects_viewport, path_object_bbox,
    region_intersects_viewport, styled_run_bbox, vector_object_intersects_viewport,
};

pub use crate::render::source_suppression::SuppressedVectorTextRuns;

#[derive(Debug, Clone)]
pub enum EffectiveVectorRenderEntry {
    Object {
        object_index: usize,
        suppressed_text_runs: SuppressedVectorTextRuns,
    },
    ParagraphOverlay(ParagraphRenderOverlay),
}

#[derive(Debug, Clone)]
pub struct GlyphParagraphRef {
    pub region_index: usize,
    pub paragraph_index: usize,
    pub suppressed_run_object_ids: HashSet<String>,
    pub suppressed_run_indices: HashSet<usize>,
}

#[derive(Debug, Clone)]
pub enum EffectiveGlyphRenderEntry {
    Paragraph(GlyphParagraphRef),
    ParagraphOverlay(ParagraphRenderOverlay),
}

struct PreparedOverlay {
    overlay: ParagraphRenderOverlay,
    replacement_region: ParagraphReplacementRegion,
    object_ids: HashSet<String>,
    object_indices: HashSet<usize>,
    path_suppression_bbox: BoundingBox,
    inserted: bool,
    suppressed_text_object_count: usize,
    suppressed_text_run_count: usize,
    object_intersect_count: usize,
    text_intersect_count: usize,
    path_intersect_count: usize,
    image_intersect_count: usize,
    thin_horizontal_path_count: usize,
    suppressed_path_count: usize,
    first_path_summary: Option<String>,
    object_summary_1: Option<String>,
    object_summary_2: Option<String>,
    object_summary_3: Option<String>,
}

fn overlay_paragraph_object_ids(overlay: &ParagraphRenderOverlay) -> HashSet<String> {
    collect_target_source_object_ids(&overlay.target)
}

fn overlay_paragraph_object_indices(overlay: &ParagraphRenderOverlay) -> HashSet<usize> {
    let mut object_indices = collect_target_source_object_indices_set(&overlay.target);
    object_indices.extend(overlay.source_object_indices.iter().copied());
    object_indices
}

fn overlay_renders_last(overlay: &ParagraphRenderOverlay) -> bool {
    matches!(
        overlay.owner,
        ParagraphRenderOverlayOwner::ActiveEditorShell
            | ParagraphRenderOverlayOwner::PersistedPageCanvas
    )
}

fn overlay_suppresses_text_source(overlay: &ParagraphRenderOverlay) -> bool {
    overlay.replaces_source
}

fn overlay_suppresses_row_paths(overlay: &ParagraphRenderOverlay) -> bool {
    matches!(
        overlay.owner,
        ParagraphRenderOverlayOwner::ActiveEditorShell
            | ParagraphRenderOverlayOwner::PersistedPageCanvas
    )
}

fn overlay_intersects_viewport(
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

fn vector_object_bbox(object: &VectorRenderObject) -> Option<BoundingBox> {
    match object {
        VectorRenderObject::Text(text) => {
            let mut combined: Option<BoundingBox> = None;
            for run in &text.runs {
                let run_bbox = styled_run_bbox(run);
                combined = Some(match combined {
                    Some(current) => BoundingBox {
                        left: current.left.min(run_bbox.left),
                        top: current.top.min(run_bbox.top),
                        right: current.right.max(run_bbox.right),
                        bottom: current.bottom.max(run_bbox.bottom),
                    },
                    None => run_bbox,
                });
            }
            combined
        }
        VectorRenderObject::Path(path) => path_object_bbox(path),
        VectorRenderObject::Image(image) => Some(BoundingBox {
            left: image.x,
            top: image.y,
            right: image.x + image.width.max(0.0),
            bottom: image.y + image.height.max(0.0),
        }),
    }
}

fn vector_object_summary(object: &VectorRenderObject, object_index: usize) -> String {
    match object {
        VectorRenderObject::Text(text) => {
            let bbox = vector_object_bbox(object).unwrap_or_default();
            format!(
                "idx={} type=text id={} runs={} bbox={:.1},{:.1},{:.1},{:.1}",
                object_index,
                text.id,
                text.runs.len(),
                bbox.left,
                bbox.top,
                bbox.right,
                bbox.bottom,
            )
        }
        VectorRenderObject::Path(path) => {
            let bbox = vector_object_bbox(object).unwrap_or_default();
            format!(
                "idx={} type=path id={} stroke={} fill={} bbox={:.1},{:.1},{:.1},{:.1}",
                object_index,
                path.id,
                path.stroke_color.as_deref().unwrap_or("none"),
                path.fill_color.as_deref().unwrap_or("none"),
                bbox.left,
                bbox.top,
                bbox.right,
                bbox.bottom,
            )
        }
        VectorRenderObject::Image(image) => {
            let bbox = vector_object_bbox(object).unwrap_or_default();
            format!(
                "idx={} type=image id={} bbox={:.1},{:.1},{:.1},{:.1}",
                object_index, image.id, bbox.left, bbox.top, bbox.right, bbox.bottom,
            )
        }
    }
}

fn record_overlay_object_summary(overlay: &mut PreparedOverlay, summary: String) {
    if overlay.object_summary_1.is_none() {
        overlay.object_summary_1 = Some(summary);
    } else if overlay.object_summary_2.is_none() {
        overlay.object_summary_2 = Some(summary);
    } else if overlay.object_summary_3.is_none() {
        overlay.object_summary_3 = Some(summary);
    }
}

fn resolve_visible_indices(vector_model: &VectorPageModel, prepared_scene: Option<&PreparedPageScene>, viewport_bbox: &BoundingBox) -> Vec<usize> {
    prepared_scene.map(|scene| scene.visible_vector_indices(viewport_bbox)).unwrap_or_else(|| {
        vector_model.objects.iter().enumerate().filter_map(|(index, obj)| {
            if vector_object_intersects_viewport(obj, viewport_bbox) { Some(index) } else { None }
        }).collect()
    })
}
fn prepare_overlays(overlays: &[ParagraphRenderOverlay], viewport_bbox: &BoundingBox, page_width: f32) -> Vec<PreparedOverlay> {
    overlays.iter().filter(|o| overlay_intersects_viewport(o, viewport_bbox, page_width)).cloned().map(|overlay| {
        let rr = paragraph_replacement_region(&overlay.target);
        PreparedOverlay {
            object_ids: if overlay_suppresses_text_source(&overlay) { overlay_paragraph_object_ids(&overlay) } else { HashSet::new() },
            object_indices: if overlay_suppresses_text_source(&overlay) { overlay_paragraph_object_indices(&overlay) } else { HashSet::new() },
            path_suppression_bbox: if overlay_suppresses_row_paths(&overlay) { rr.row_path_suppression_bbox_for_page_width(page_width) } else { BoundingBox::default() },
            replacement_region: rr, overlay, inserted: false,
            suppressed_text_object_count: 0, suppressed_text_run_count: 0,
            object_intersect_count: 0, text_intersect_count: 0, path_intersect_count: 0, image_intersect_count: 0,
            thin_horizontal_path_count: 0, suppressed_path_count: 0,
            first_path_summary: None, object_summary_1: None, object_summary_2: None, object_summary_3: None,
        }
    }).collect::<Vec<_>>()
}
fn trace_overlay_identity(po: &[PreparedOverlay], vi: &[usize], vm: &VectorPageModel) {
    for (i, ov) in po.iter().enumerate() {
        dbg_event("effective-plan","overlay-identity",vec![
            dbg_field("overlayIndex",i),dbg_field("paragraphId",ov.overlay.target.paragraph_id.as_str()),
            dbg_field("owner",format!("{:?}",ov.overlay.owner)),dbg_field("replacesSource",ov.overlay.replaces_source),
            dbg_field("objectIds",format!("{:?}",ov.object_ids.iter().collect::<Vec<_>>())),
            dbg_field("objectIdCount",ov.object_ids.len()),
            dbg_field("objectIndices",format!("{:?}",ov.object_indices)),
            dbg_field("objectIndexCount",ov.object_indices.len()),
            dbg_field("sourceText",crate::common::debug::truncate_debug_text(&ov.overlay.source_text,40)),
            dbg_field("draftText",crate::common::debug::truncate_debug_text(&ov.overlay.draft_text,40))]);
    }
    for &idx in vi {
        if let Some(VectorRenderObject::Text(t)) = vm.objects.get(idx) {
            dbg_event("effective-plan","vector-text-object",vec![
                dbg_field("objectIndex",idx),dbg_field("objectId",t.id.as_str()),
                dbg_field("runCount",t.runs.len()),
                dbg_field("firstRunText",t.runs.first().map(|r|crate::common::debug::truncate_debug_text(&r.text,30)).unwrap_or_default())]);
        }
    }
}
fn build_entries_without_overlays(vi: Vec<usize>, vm: &VectorPageModel) -> Vec<EffectiveVectorRenderEntry> {
    vi.into_iter().filter(|&oi| {
        if let Some(VectorRenderObject::Text(t)) = vm.objects.get(oi) { !t.runs.iter().all(|r|r.render_mode==3) } else { true }
    }).map(|oi| EffectiveVectorRenderEntry::Object{object_index:oi,suppressed_text_runs:SuppressedVectorTextRuns::default()}).collect()
}
fn trace_overlay_summary(o: &PreparedOverlay) {
    let sb=format!("{:.1},{:.1},{:.1},{:.1}",o.replacement_region.source_bbox.left,o.replacement_region.source_bbox.top,o.replacement_region.source_bbox.right,o.replacement_region.source_bbox.bottom);
    let tcb=format!("{:.1},{:.1},{:.1},{:.1}",o.replacement_region.text_clear_bbox.left,o.replacement_region.text_clear_bbox.top,o.replacement_region.text_clear_bbox.right,o.replacement_region.text_clear_bbox.bottom);
    let pb=format!("{:.1},{:.1},{:.1},{:.1}",o.path_suppression_bbox.left,o.path_suppression_bbox.top,o.path_suppression_bbox.right,o.path_suppression_bbox.bottom);
    dbg_event("effective-plan","overlay-min",vec![dbg_field("summary",format!("owner={:?} repl={} sp={} pi={} ii={} sb={} pb={} first={}",o.overlay.owner,o.overlay.replaces_source,o.suppressed_path_count,o.path_intersect_count,o.image_intersect_count,sb,pb,o.first_path_summary.as_deref().unwrap_or("none")))]);
    dbg_event("effective-plan","overlay-compact",vec![dbg_field("paragraphId",o.overlay.target.paragraph_id.as_str()),dbg_field("owner",format!("{:?}",o.overlay.owner)),dbg_field("replacesSource",o.overlay.replaces_source),dbg_field("sourceBBox",sb.as_str()),dbg_field("textClearBBox",tcb.as_str()),dbg_field("pathSuppressionBBox",pb.as_str()),dbg_field("pathIntersectCount",o.path_intersect_count),dbg_field("imageIntersectCount",o.image_intersect_count),dbg_field("suppressedPathCount",o.suppressed_path_count),dbg_field("firstPathSummary",o.first_path_summary.as_deref().unwrap_or("none"))]);
    dbg_event("effective-plan","overlay-path-summary",vec![dbg_field("paragraphId",o.overlay.target.paragraph_id.as_str()),dbg_field("owner",format!("{:?}",o.overlay.owner)),dbg_field("replacesSource",o.overlay.replaces_source),dbg_field("sourceText",o.overlay.source_text.as_str()),dbg_field("draftText",o.overlay.draft_text.as_str()),dbg_field("sourceObjectIndexCount",o.object_indices.len()),dbg_field("sourceObjectIndices",format!("{:?}",o.object_indices)),dbg_field("textClearBBox",tcb.as_str()),dbg_field("sourceBBox",sb.as_str()),dbg_field("pathSuppressionBBox",pb.as_str()),dbg_field("objectIntersectCount",o.object_intersect_count),dbg_field("textIntersectCount",o.text_intersect_count),dbg_field("pathIntersectCount",o.path_intersect_count),dbg_field("imageIntersectCount",o.image_intersect_count),dbg_field("thinHorizontalPathCount",o.thin_horizontal_path_count),dbg_field("suppressedPathCount",o.suppressed_path_count),dbg_field("suppressedTextObjectCount",o.suppressed_text_object_count),dbg_field("suppressedTextRunCount",o.suppressed_text_run_count),dbg_field("sourceObjectIdCount",o.object_ids.len()),dbg_field("firstPathSummary",o.first_path_summary.as_deref().unwrap_or("none")),dbg_field("objectSummary1",o.object_summary_1.as_deref().unwrap_or("none")),dbg_field("objectSummary2",o.object_summary_2.as_deref().unwrap_or("none")),dbg_field("objectSummary3",o.object_summary_3.as_deref().unwrap_or("none"))]);
}
fn insert_overlay_if_needed(o: &mut PreparedOverlay, e: &mut Vec<EffectiveVectorRenderEntry>) {
    if !o.inserted && !overlay_renders_last(&o.overlay) {
        e.push(EffectiveVectorRenderEntry::ParagraphOverlay(o.overlay.clone()));
        o.inserted = true;
    }
}
pub fn build_effective_vector_render_plan(
    vector_model: &VectorPageModel,
    prepared_scene: Option<&PreparedPageScene>,
    viewport_bbox: &BoundingBox,
    overlays: &[ParagraphRenderOverlay],
) -> Vec<EffectiveVectorRenderEntry> {
    let visible_indices = resolve_visible_indices(vector_model, prepared_scene, viewport_bbox);
    let mut prepared_overlays = prepare_overlays(overlays, viewport_bbox, vector_model.width);
    trace_overlay_identity(&prepared_overlays, &visible_indices, vector_model);

    if prepared_overlays.is_empty() {
        return build_entries_without_overlays(visible_indices, vector_model);
    }

enum TextSuppressionOutcome {
    RunLevel(SuppressedVectorTextRuns),
    NonMarkerRuns,
    NoMatch,
}

fn decide_text_suppression(object: &VectorRenderObject, object_index: usize, overlay: &PreparedOverlay) -> TextSuppressionOutcome {
    let z_index_hit = matches!(object, VectorRenderObject::Text(text) if overlay.object_indices.contains(&text.z_index));
    let array_index_hit = overlay.object_indices.contains(&object_index);
    let index_hit = z_index_hit || array_index_hit;
    let id_hit = matches!(object, VectorRenderObject::Text(text) if overlay.object_ids.contains(&text.id));
    let text_object_index_match = matches!(object, VectorRenderObject::Text(_)) && (index_hit || id_hit);
    if matches!(object, VectorRenderObject::Text(_)) {
        let (text_id, text_z) = if let VectorRenderObject::Text(text) = object {
            (text.id.as_str(), text.z_index)
        } else { ("", 0) };
        dbg_event("effective-plan", "suppress-check", vec![
            dbg_field("objectIndex", object_index),
            dbg_field("textZIndex", text_z),
            dbg_field("textId", text_id),
            dbg_field("overlayParagraphId", overlay.overlay.target.paragraph_id.as_str()),
            dbg_field("zIndexHit", z_index_hit),
            dbg_field("arrayIndexHit", array_index_hit),
            dbg_field("idHit", id_hit),
            dbg_field("matched", text_object_index_match),
        ]);
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
            } else { 0 };
            overlay.suppressed_text_run_count = overlay.suppressed_text_run_count.saturating_add(matched_run_count);
            overlay.suppressed_text_object_count = overlay.suppressed_text_object_count.saturating_add(1);
            suppressed_text_runs.run_indices.extend(refs.run_indices);
            suppressed_text_runs.object_ids.extend(refs.object_ids);
            true
        }
        TextSuppressionOutcome::NonMarkerRuns => {
            overlay.suppressed_text_object_count = overlay.suppressed_text_object_count.saturating_add(1);
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

fn check_path_suppression(object: &VectorRenderObject, object_index: usize, overlay: &mut PreparedOverlay) -> bool {
    if let Some(object_bbox) = vector_object_bbox(object) {
        if bbox_intersects(&object_bbox, &overlay.path_suppression_bbox) {
            overlay.object_intersect_count = overlay.object_intersect_count.saturating_add(1);
            match object {
                VectorRenderObject::Text(_) => overlay.text_intersect_count = overlay.text_intersect_count.saturating_add(1),
                VectorRenderObject::Path(_) => overlay.path_intersect_count = overlay.path_intersect_count.saturating_add(1),
                VectorRenderObject::Image(_) => overlay.image_intersect_count = overlay.image_intersect_count.saturating_add(1),
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
        if overlay.first_path_summary.is_none() { overlay.first_path_summary = Some(path_summary); }
        return true;
    }
    if let VectorRenderObject::Path(path) = object {
        if let Some(path_bbox) = path_object_bbox(path) {
            if bbox_intersects(&path_bbox, &overlay.path_suppression_bbox) {
                if overlay.first_path_summary.is_none() {
                    overlay.first_path_summary = Some(format!(
                        "id={} bbox={:.1},{:.1},{:.1},{:.1} stroke={} color={}",
                        path.id, path_bbox.left, path_bbox.top, path_bbox.right, path_bbox.bottom,
                        path.stroke_width, path.stroke_color.as_deref().unwrap_or("none")));
                }
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
        let Some(object) = vector_model.objects.get(object_index) else { continue };
        if let VectorRenderObject::Text(text) = object {
            if text.runs.iter().all(|run| run.render_mode == 3) { continue }
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
                        && suppressed_text_runs.suppressed_count_for_text_object(text) == text.runs.len())
            }
            _ => suppress_entire_object,
        };
        if should_skip_entire_object { continue }
        entries.push(EffectiveVectorRenderEntry::Object {
            object_index,
            suppressed_text_runs,
        });
    }
    entries
}

    let mut entries = process_visible_objects(visible_indices, vector_model, &mut prepared_overlays);

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
            let source_text_match =
                glyph_paragraph_matches_overlay_source_text(paragraph, overlay);
            object_id_match || object_index_match || spatial_match || source_text_match
        })
        .collect::<Vec<_>>();
    let mut suppressed_run_object_ids = HashSet::<String>::new();
    let mut suppressed_run_indices = HashSet::<usize>::new();
    for overlay in &matching_overlays {
        let overlay_ids = overlay_paragraph_object_ids(overlay);
        let overlay_indices = overlay_paragraph_object_indices(overlay);
        let source_text_match =
            glyph_paragraph_matches_overlay_source_text(paragraph, overlay);
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

#[cfg(test)]
mod tests {
    use super::{
        build_effective_glyph_render_plan, build_effective_vector_render_plan,
        EffectiveGlyphRenderEntry, EffectiveVectorRenderEntry,
    };
    use crate::edit::active_target::ActiveEditorTarget;
    use crate::edit::paragraph_overlay::{ParagraphRenderOverlay, ParagraphRenderOverlayOwner};
    use crate::models::{
        BoundingBox, EditorControlStyle, GlyphPaintParagraph, GlyphPaintPlan, GlyphPaintRegion,
        GlyphPaintRun, LayoutMode, LayoutParagraph, LayoutRole, LayoutRun, ParagraphEditContext,
        StyledRun, VectorPageModel, VectorPathObject, VectorPathSegment, VectorRenderObject,
        VectorTextObject,
    };

    fn horizontal_stroked_path(id: &str, y: f32) -> VectorRenderObject {
        horizontal_stroked_path_between(id, 80.0, 340.0, y)
    }

    fn horizontal_stroked_path_between(
        id: &str,
        left: f32,
        right: f32,
        y: f32,
    ) -> VectorRenderObject {
        VectorRenderObject::Path(VectorPathObject {
            id: id.to_string(),
            segments: vec![
                VectorPathSegment {
                    command: "move".to_string(),
                    points: vec![[left, y]],
                },
                VectorPathSegment {
                    command: "line".to_string(),
                    points: vec![[right, y]],
                },
            ],
            stroke: true,
            stroke_width: 4.0,
            stroke_color: Some("#0070c0".to_string()),
            ..Default::default()
        })
    }

    fn active_overlay_for_body(body_bbox: BoundingBox) -> ParagraphRenderOverlay {
        let mut target = ActiveEditorTarget::default();
        target.paragraph_id = "p-1".to_string();
        target.scene.shell_bbox = BoundingBox {
            left: 40.0,
            top: 96.0,
            right: 360.0,
            bottom: 116.0,
        };
        target.scene.body_session = ParagraphEditContext {
            anchor_bbox: body_bbox,
            paragraph: LayoutParagraph::default(),
        };

        ParagraphRenderOverlay {
            owner: ParagraphRenderOverlayOwner::ActiveEditorShell,
            target,
            source_object_indices: Vec::new(),
            graphic_markers: Vec::new(),
            source_text: "body".to_string(),
            draft_text: "body".to_string(),
            replaces_source: true,
            marker_text_override: None,
        }
    }

    fn active_overlay_for_source_object(object_id: &str) -> ParagraphRenderOverlay {
        let mut overlay = active_overlay_for_body(BoundingBox {
            left: 90.0,
            top: 100.0,
            right: 330.0,
            bottom: 112.0,
        });
        overlay.target.scene.original_runs.clear();
        overlay.target.scene.body_session.paragraph.runs = vec![LayoutRun {
            id: "body-run".to_string(),
            text: "编程语言: Rust".to_string(),
            object_ids: vec![object_id.to_string()],
            bbox: overlay.target.scene.body_session.anchor_bbox,
            ..Default::default()
        }];
        overlay
    }

    fn persisted_overlay_for_source_object(object_id: &str) -> ParagraphRenderOverlay {
        let mut overlay = active_overlay_for_source_object(object_id);
        overlay.owner = ParagraphRenderOverlayOwner::PersistedPageCanvas;
        overlay
    }

    fn text_object_without_run_ids(id: &str) -> VectorRenderObject {
        VectorRenderObject::Text(VectorTextObject {
            id: id.to_string(),
            runs: vec![StyledRun {
                text: "编程语言: Rust".to_string(),
                tx: 90.0,
                ty: 112.0,
                width: 240.0,
                font_size: 12.0,
                object_id: None,
                ..Default::default()
            }],
            ..Default::default()
        })
    }

    fn glyph_plan_without_run_ids() -> GlyphPaintPlan {
        let bbox = BoundingBox {
            left: 90.0,
            top: 100.0,
            right: 330.0,
            bottom: 112.0,
        };
        GlyphPaintPlan {
            page_index: 0,
            width: 595.0,
            height: 842.0,
            regions: vec![GlyphPaintRegion {
                id: "r-1".to_string(),
                kind: LayoutRole::Paragraph,
                layout_mode: LayoutMode::Flow,
                bbox,
                paragraphs: vec![GlyphPaintParagraph {
                    id: "p-1".to_string(),
                    region_id: "r-1".to_string(),
                    bbox,
                    style: Default::default(),
                    editor_session: ParagraphEditContext {
                        anchor_bbox: bbox,
                        paragraph: LayoutParagraph::default(),
                    },
                    control_style: EditorControlStyle::default(),
                    semantic_role: Default::default(),
                    runs: vec![GlyphPaintRun {
                        id: "glyph-run-1".to_string(),
                        page_index: 0,
                        region_id: "r-1".to_string(),
                        paragraph_id: "p-1".to_string(),
                        text: "编程语言: Rust".to_string(),
                        bbox,
                        origin_x: 90.0,
                        origin_y: 112.0,
                        font_size: 12.0,
                        object_ids: Vec::new(),
                        ..Default::default()
                    }],
                }],
                object_ids: Vec::new(),
            }],
            external_objects: Vec::new(),
        }
    }

    #[test]
    fn suppresses_zero_height_path() {
        let model = VectorPageModel {
            width: 595.0,
            height: 842.0,
            objects: vec![horizontal_stroked_path("blue-row-bar", 106.0)],
            ..Default::default()
        };
        let overlay = active_overlay_for_body(BoundingBox {
            left: 90.0,
            top: 100.0,
            right: 330.0,
            bottom: 112.0,
        });
        let viewport = BoundingBox {
            left: 0.0,
            top: 0.0,
            right: 595.0,
            bottom: 842.0,
        };

        let entries = build_effective_vector_render_plan(&model, None, &viewport, &[overlay]);

        assert!(
            entries.iter().all(|entry| !matches!(
                entry,
                EffectiveVectorRenderEntry::Object { object_index: 0, .. }
            )),
            "active editor must remove stroked horizontal source paths that cross the editable body"
        );
        assert!(entries
            .iter()
            .any(|entry| matches!(entry, EffectiveVectorRenderEntry::ParagraphOverlay(_))));
    }

    #[test]
    fn keeps_section_divider() {
        let model = VectorPageModel {
            width: 595.0,
            height: 842.0,
            objects: vec![horizontal_stroked_path("section-divider", 118.0)],
            ..Default::default()
        };
        let overlay = active_overlay_for_body(BoundingBox {
            left: 90.0,
            top: 100.0,
            right: 330.0,
            bottom: 112.0,
        });
        let viewport = BoundingBox {
            left: 0.0,
            top: 0.0,
            right: 595.0,
            bottom: 842.0,
        };

        let entries = build_effective_vector_render_plan(&model, None, &viewport, &[overlay]);

        assert!(
            entries.iter().any(|entry| matches!(
                entry,
                EffectiveVectorRenderEntry::Object {
                    object_index: 0,
                    ..
                }
            )),
            "decorative divider paths outside the editable row must remain on the page canvas"
        );
    }

    #[test]
    fn keeps_nearby_divider() {
        let model = VectorPageModel {
            width: 595.0,
            height: 842.0,
            objects: vec![horizontal_stroked_path("near-divider", 116.0)],
            ..Default::default()
        };
        let overlay = active_overlay_for_body(BoundingBox {
            left: 90.0,
            top: 100.0,
            right: 330.0,
            bottom: 112.0,
        });
        let viewport = BoundingBox {
            left: 0.0,
            top: 0.0,
            right: 595.0,
            bottom: 842.0,
        };

        let entries = build_effective_vector_render_plan(&model, None, &viewport, &[overlay]);

        assert!(
            entries.iter().any(|entry| matches!(
                entry,
                EffectiveVectorRenderEntry::Object { object_index: 0, .. }
            )),
            "decorative divider paths below the editable row must not be reclassified as editable-row decoration"
        );
    }

    #[test]
    fn suppresses_descender_path() {
        let model = VectorPageModel {
            width: 595.0,
            height: 842.0,
            objects: vec![horizontal_stroked_path("descender-blue-row-bar", 113.8)],
            ..Default::default()
        };
        let overlay = active_overlay_for_body(BoundingBox {
            left: 90.0,
            top: 100.0,
            right: 330.0,
            bottom: 112.0,
        });
        let viewport = BoundingBox {
            left: 0.0,
            top: 0.0,
            right: 595.0,
            bottom: 842.0,
        };

        let entries = build_effective_vector_render_plan(&model, None, &viewport, &[overlay]);

        assert!(
            entries.iter().all(|entry| !matches!(
                entry,
                EffectiveVectorRenderEntry::Object { object_index: 0, .. }
            )),
            "row-level source decorations that overlap the glyph descender band must be removed with the edited text"
        );
    }

    #[test]
    fn suppresses_text_without_ids() {
        let model = VectorPageModel {
            width: 595.0,
            height: 842.0,
            objects: vec![text_object_without_run_ids("text-object-1")],
            ..Default::default()
        };
        let overlay = active_overlay_for_source_object("text-object-1");
        let viewport = BoundingBox {
            left: 0.0,
            top: 0.0,
            right: 595.0,
            bottom: 842.0,
        };

        let entries = build_effective_vector_render_plan(&model, None, &viewport, &[overlay]);

        assert!(
            entries.iter().all(|entry| !matches!(
                entry,
                EffectiveVectorRenderEntry::Object {
                    object_index: 0,
                    ..
                }
            )),
            "active editor must suppress the source text object when run-level ids are unavailable"
        );
        assert!(entries
            .iter()
            .any(|entry| matches!(entry, EffectiveVectorRenderEntry::ParagraphOverlay(_))));
    }

    #[test]
    fn spatially_suppresses_text() {
        let model = VectorPageModel {
            width: 595.0,
            height: 842.0,
            objects: vec![text_object_without_run_ids("unmatched-text-object")],
            ..Default::default()
        };
        let overlay = active_overlay_for_body(BoundingBox {
            left: 90.0,
            top: 100.0,
            right: 330.0,
            bottom: 112.0,
        });
        let viewport = BoundingBox {
            left: 0.0,
            top: 0.0,
            right: 595.0,
            bottom: 842.0,
        };

        let entries = build_effective_vector_render_plan(&model, None, &viewport, &[overlay]);

        assert!(
            entries.iter().all(|entry| !matches!(
                entry,
                EffectiveVectorRenderEntry::Object { object_index: 0, .. }
            )),
            "changed edit mode must still remove source text when PDF extraction cannot provide stable source object ids"
        );
    }

    #[test]
    fn keeps_matching_text() {
        let model = VectorPageModel {
            width: 595.0,
            height: 842.0,
            objects: vec![text_object_without_run_ids("unmatched-text-object")],
            ..Default::default()
        };
        let mut overlay = active_overlay_for_body(BoundingBox {
            left: 90.0,
            top: 100.0,
            right: 330.0,
            bottom: 112.0,
        });
        overlay.replaces_source = false;
        let viewport = BoundingBox {
            left: 0.0,
            top: 0.0,
            right: 595.0,
            bottom: 842.0,
        };

        let entries = build_effective_vector_render_plan(&model, None, &viewport, &[overlay]);

        assert!(
            entries.iter().any(|entry| matches!(
                entry,
                EffectiveVectorRenderEntry::Object {
                    object_index: 0,
                    ..
                }
            )),
            "clean caret-only edit mode must not spatially suppress source text"
        );
    }

    #[test]
    fn keeps_source_text() {
        let model = VectorPageModel {
            width: 595.0,
            height: 842.0,
            objects: vec![text_object_without_run_ids("text-object-1")],
            ..Default::default()
        };
        let mut overlay = active_overlay_for_source_object("text-object-1");
        overlay.replaces_source = false;
        let viewport = BoundingBox {
            left: 0.0,
            top: 0.0,
            right: 595.0,
            bottom: 842.0,
        };

        let entries = build_effective_vector_render_plan(&model, None, &viewport, &[overlay]);

        assert!(
            entries.iter().any(|entry| matches!(
                entry,
                EffectiveVectorRenderEntry::Object {
                    object_index: 0,
                    ..
                }
            )),
            "clean edit mode must keep the original PDF text painter visible"
        );
    }

    #[test]
    fn suppresses_path_only() {
        let model = VectorPageModel {
            width: 595.0,
            height: 842.0,
            objects: vec![
                horizontal_stroked_path("blue-row-bar", 106.0),
                text_object_without_run_ids("unmatched-text-object"),
            ],
            ..Default::default()
        };
        let mut overlay = active_overlay_for_body(BoundingBox {
            left: 90.0,
            top: 100.0,
            right: 330.0,
            bottom: 112.0,
        });
        overlay.replaces_source = false;
        let viewport = BoundingBox {
            left: 0.0,
            top: 0.0,
            right: 595.0,
            bottom: 842.0,
        };

        let entries = build_effective_vector_render_plan(&model, None, &viewport, &[overlay]);

        assert!(
            entries.iter().all(|entry| !matches!(
                entry,
                EffectiveVectorRenderEntry::Object {
                    object_index: 0,
                    ..
                }
            )),
            "clean edit mode must still remove source-row blue path artifacts"
        );
        assert!(
            entries.iter().any(|entry| matches!(
                entry,
                EffectiveVectorRenderEntry::Object {
                    object_index: 1,
                    ..
                }
            )),
            "clean edit mode must keep the original PDF text painter visible"
        );
    }

    #[test]
    fn spatially_suppresses_glyphs() {
        let plan = glyph_plan_without_run_ids();
        let overlay = active_overlay_for_body(BoundingBox {
            left: 90.0,
            top: 100.0,
            right: 330.0,
            bottom: 112.0,
        });
        let viewport = BoundingBox {
            left: 0.0,
            top: 0.0,
            right: 595.0,
            bottom: 842.0,
        };

        let entries = build_effective_glyph_render_plan(&plan, &viewport, &[overlay]);

        let paragraph = entries
            .iter()
            .find_map(|entry| match entry {
                EffectiveGlyphRenderEntry::Paragraph(reference) => Some(reference),
                _ => None,
            })
            .expect("glyph paragraph should remain in the render plan");
        assert!(
            paragraph.suppressed_run_indices.contains(&0),
            "changed edit mode must spatially suppress source glyph runs when source ids are missing"
        );
        assert!(entries
            .iter()
            .any(|entry| matches!(entry, EffectiveGlyphRenderEntry::ParagraphOverlay(_))));
    }

    #[test]
    fn keeps_matching_glyphs() {
        let plan = glyph_plan_without_run_ids();
        let mut overlay = active_overlay_for_body(BoundingBox {
            left: 90.0,
            top: 100.0,
            right: 330.0,
            bottom: 112.0,
        });
        overlay.replaces_source = false;
        let viewport = BoundingBox {
            left: 0.0,
            top: 0.0,
            right: 595.0,
            bottom: 842.0,
        };

        let entries = build_effective_glyph_render_plan(&plan, &viewport, &[overlay]);

        let paragraph = entries
            .iter()
            .find_map(|entry| match entry {
                EffectiveGlyphRenderEntry::Paragraph(reference) => Some(reference),
                _ => None,
            })
            .expect("glyph paragraph should remain in the render plan");
        assert!(
            paragraph.suppressed_run_indices.is_empty(),
            "clean caret-only edit mode must not spatially suppress source glyph text"
        );
    }

    #[test]
    fn overlay_suppresses_glyphs() {
        let plan = glyph_plan_without_run_ids();
        let mut overlay = active_overlay_for_body(BoundingBox {
            left: 90.0,
            top: 100.0,
            right: 330.0,
            bottom: 112.0,
        });
        overlay.owner = ParagraphRenderOverlayOwner::PersistedPageCanvas;
        let viewport = BoundingBox {
            left: 0.0,
            top: 0.0,
            right: 595.0,
            bottom: 842.0,
        };

        let entries = build_effective_glyph_render_plan(&plan, &viewport, &[overlay]);

        let paragraph = entries
            .iter()
            .find_map(|entry| match entry {
                EffectiveGlyphRenderEntry::Paragraph(reference) => Some(reference),
                _ => None,
            })
            .expect("glyph paragraph should remain in the render plan");
        assert!(
            paragraph.suppressed_run_indices.contains(&0),
            "committed replacement must spatially suppress fallback glyph text when source ids are missing"
        );
        assert!(
            matches!(
                entries.last(),
                Some(EffectiveGlyphRenderEntry::ParagraphOverlay(_))
            ),
            "persisted replacement overlay must render after the fallback glyph paragraph"
        );
    }

    #[test]
    fn overlay_renders_last() {
        let model = VectorPageModel {
            width: 595.0,
            height: 842.0,
            objects: vec![
                text_object_without_run_ids("text-object-1"),
                horizontal_stroked_path("later-blue-path", 106.0),
            ],
            ..Default::default()
        };
        let overlay = persisted_overlay_for_source_object("text-object-1");
        let viewport = BoundingBox {
            left: 0.0,
            top: 0.0,
            right: 595.0,
            bottom: 842.0,
        };

        let entries = build_effective_vector_render_plan(&model, None, &viewport, &[overlay]);

        assert!(
            matches!(entries.last(), Some(EffectiveVectorRenderEntry::ParagraphOverlay(_))),
            "persisted replacement text must be painted after later PDF paths so paths cannot cover edited text after leaving edit mode"
        );
    }

    #[test]
    fn overlay_suppresses_path() {
        let model = VectorPageModel {
            width: 595.0,
            height: 842.0,
            objects: vec![
                text_object_without_run_ids("text-object-1"),
                horizontal_stroked_path("committed-blue-row-bar", 106.0),
            ],
            ..Default::default()
        };
        let overlay = persisted_overlay_for_source_object("text-object-1");
        let viewport = BoundingBox {
            left: 0.0,
            top: 0.0,
            right: 595.0,
            bottom: 842.0,
        };

        let entries = build_effective_vector_render_plan(&model, None, &viewport, &[overlay]);

        assert!(
            entries.iter().all(|entry| !matches!(
                entry,
                EffectiveVectorRenderEntry::Object { object_index: 1, .. }
            )),
            "committed paragraph replacement must remove row-level PDF paths, not only active editor paths"
        );
    }

    #[test]
    fn keeps_right_tile_suppressed() {
        let model = VectorPageModel {
            width: 595.0,
            height: 842.0,
            objects: vec![horizontal_stroked_path_between(
                "right-tile-blue-row-bar",
                430.0,
                560.0,
                106.0,
            )],
            ..Default::default()
        };
        let overlay = persisted_overlay_for_source_object("text-object-1");
        let right_tile_viewport = BoundingBox {
            left: 400.0,
            top: 80.0,
            right: 595.0,
            bottom: 140.0,
        };

        let entries =
            build_effective_vector_render_plan(&model, None, &right_tile_viewport, &[overlay]);

        assert!(
            entries.iter().all(|entry| !matches!(
                entry,
                EffectiveVectorRenderEntry::Object { object_index: 0, .. }
            )),
            "replacement effect region must cover row-level path suppression even when the viewport tile is outside the editor shell"
        );
        assert!(entries
            .iter()
            .any(|entry| matches!(entry, EffectiveVectorRenderEntry::ParagraphOverlay(_))));
    }

    /// 关键回归测试：当 PDF 的 list-item 把 marker (●) 和 body 放在同一个文本对象里时，
    /// 编辑后 marker run 必须被保留（不能被 spatial suppress 干掉）。
    #[test]
    fn keeps_list_marker() {
        // 模拟真实 PDF：单个文本对象，runs[0] = "●", runs[1..] = body 字符
        let body_left = 90.0;
        let body_right = 330.0;
        let body_top = 100.0;
        let body_bottom = 112.0;
        let marker_x = 70.0; // marker 在 body 左侧 20px

        let mut runs = vec![StyledRun {
            text: "●".to_string(),
            tx: marker_x,
            ty: body_bottom,
            width: 10.0,
            font_size: 12.0,
            object_id: None,
            ..Default::default()
        }];
        // 模拟 body 的若干 run（每个字符一个）
        let body_chars = ["编", "程", "语", "言", ":", "R", "u", "s", "t"];
        let mut x = body_left;
        for ch in body_chars {
            runs.push(StyledRun {
                text: ch.to_string(),
                tx: x,
                ty: body_bottom,
                width: 12.0,
                font_size: 12.0,
                object_id: None,
                ..Default::default()
            });
            x += 12.0;
        }
        let total_run_count = runs.len();

        let model = VectorPageModel {
            width: 595.0,
            height: 842.0,
            objects: vec![VectorRenderObject::Text(VectorTextObject {
                id: "text-with-marker".to_string(),
                runs,
                ..Default::default()
            })],
            ..Default::default()
        };
        let overlay = persisted_overlay_for_source_object("text-with-marker");
        let viewport = BoundingBox {
            left: 0.0,
            top: 0.0,
            right: 595.0,
            bottom: 842.0,
        };
        let _ = (body_left, body_right, body_top); // silence unused

        let entries = build_effective_vector_render_plan(&model, None, &viewport, &[overlay]);

        // 整对象不应该被 skip — 应该有一个 Object entry
        let obj_entry = entries.iter().find_map(|e| {
            if let EffectiveVectorRenderEntry::Object {
                object_index,
                suppressed_text_runs,
            } = e
            {
                if *object_index == 0 {
                    Some(suppressed_text_runs)
                } else {
                    None
                }
            } else {
                None
            }
        });
        let suppressed = obj_entry.expect(
            "marker text object must remain in render plan (entire object got suppressed!)",
        );

        // marker run (index 0) 不能被 suppress
        let marker_run = match &model.objects[0] {
            VectorRenderObject::Text(t) => &t.runs[0],
            _ => unreachable!(),
        };
        assert!(
            !suppressed.suppresses_run(0, marker_run),
            "marker run (●) must NOT be suppressed; suppressed_runs={:?}",
            suppressed
        );

        // body runs 应该被 suppress
        let body_run_1 = match &model.objects[0] {
            VectorRenderObject::Text(t) => &t.runs[1],
            _ => unreachable!(),
        };
        assert!(
            suppressed.suppresses_run(1, body_run_1),
            "body run must be suppressed"
        );

        // 不能全部 run 都被 suppress（否则整对象会被 should_skip_entire_object 干掉）
        let suppressed_count = (0..total_run_count)
            .filter(|i| {
                let run = match &model.objects[0] {
                    VectorRenderObject::Text(t) => &t.runs[*i],
                    _ => unreachable!(),
                };
                suppressed.suppresses_run(*i, run)
            })
            .count();
        assert!(
            suppressed_count < total_run_count,
            "not all runs should be suppressed; suppressed {}/{}",
            suppressed_count,
            total_run_count
        );
    }

    /// 回归测试：当文本对象前有非文本对象（path/image）时，
    /// z_index 和数组位置不同，suppression 必须仍然生效。
    /// 这是 z_index vs array-position mismatch bug 的精确回归保护。
    #[test]
    fn handles_z_index_order() {
        // objects[0] = Path (z_index=0)
        // objects[1] = Text (z_index=5)  ← array pos 1, z_index 5
        let model = VectorPageModel {
            width: 595.0,
            height: 842.0,
            objects: vec![
                VectorRenderObject::Path(VectorPathObject::default()),
                VectorRenderObject::Text(VectorTextObject {
                    id: "text-z5".to_string(),
                    z_index: 5,
                    runs: vec![StyledRun {
                        text: "Hello world".to_string(),
                        tx: 90.0,
                        ty: 112.0,
                        width: 200.0,
                        font_size: 12.0,
                        ..Default::default()
                    }],
                }),
            ],
            ..Default::default()
        };
        // overlay has object_indices = {5} (the z_index, NOT the array position 1)
        let mut overlay = active_overlay_for_body(BoundingBox {
            left: 90.0,
            top: 100.0,
            right: 330.0,
            bottom: 112.0,
        });
        overlay.source_object_indices = vec![5];
        overlay.target.scene.body_session.paragraph.runs = vec![LayoutRun {
            id: "run-0".to_string(),
            text: "Hello world".to_string(),
            object_ids: vec!["text-z5".to_string()],
            object_indices: vec![5],
            bbox: BoundingBox {
                left: 90.0,
                top: 100.0,
                right: 290.0,
                bottom: 112.0,
            },
            ..Default::default()
        }];

        let viewport = BoundingBox {
            left: 0.0,
            top: 0.0,
            right: 595.0,
            bottom: 842.0,
        };
        let entries = build_effective_vector_render_plan(&model, None, &viewport, &[overlay]);

        // The text object (array pos 1, z_index 5) must be suppressed
        let has_unsuppressed_text = entries.iter().any(|e| {
            matches!(
                e,
                EffectiveVectorRenderEntry::Object { object_index: 1, suppressed_text_runs }
                if suppressed_text_runs.run_indices.is_empty()
            )
        });
        assert!(
            !has_unsuppressed_text,
            "text object at array position 1 / z_index 5 must be suppressed; \
             entries: {:?}",
            entries
                .iter()
                .map(|e| match e {
                    EffectiveVectorRenderEntry::Object {
                        object_index,
                        suppressed_text_runs,
                    } => format!(
                        "Object(idx={}, suppressed_runs={:?})",
                        object_index, suppressed_text_runs.run_indices
                    ),
                    EffectiveVectorRenderEntry::ParagraphOverlay(_) =>
                        "ParagraphOverlay".to_string(),
                })
                .collect::<Vec<_>>()
        );
        // overlay must have been inserted
        assert!(
            entries
                .iter()
                .any(|e| matches!(e, EffectiveVectorRenderEntry::ParagraphOverlay(_))),
            "overlay must be inserted"
        );
    }
}
