use std::collections::HashSet;

use pdf_viewer_core::models::{BoundingBox, GlyphPaintPlan, VectorPageModel, VectorRenderObject};

use crate::editor::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use crate::editor::replacement_region::{
    paragraph_replacement_region, ParagraphReplacementRegion,
};
use crate::editor::source_identity::{
    collect_target_source_object_ids, collect_target_source_object_indices_set,
};
use crate::editor::paragraph_overlay::{ParagraphRenderOverlayOwner, ParagraphRenderOverlay};
use crate::render::prepared_scene::PreparedPageScene;
use crate::render::path_suppression::decorative_object_should_be_suppressed_by_overlay;
use crate::render::source_suppression::{
    glyph_paragraph_matches_overlay_source_text,
    glyph_run_spatially_matches_replacement_region, matching_text_run_refs,
    text_object_matches_overlay_source_text, text_object_should_be_suppressed,
};
use crate::utils::bbox::bbox_intersects;
use crate::viewport_culling::{
    glyph_run_intersects_viewport, paragraph_intersects_viewport,
    path_object_bbox, region_intersects_viewport, styled_run_bbox,
    vector_object_intersects_viewport,
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

pub fn build_effective_vector_render_plan(
    vector_model: &VectorPageModel,
    prepared_scene: Option<&PreparedPageScene>,
    viewport_bbox: &BoundingBox,
    overlays: &[ParagraphRenderOverlay],
) -> Vec<EffectiveVectorRenderEntry> {
    let visible_indices = prepared_scene
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
        });

    let mut prepared_overlays = overlays
        .iter()
        .filter(|overlay| overlay_intersects_viewport(overlay, viewport_bbox, vector_model.width))
        .cloned()
        .map(|overlay| {
            let replacement_region = paragraph_replacement_region(&overlay.target);
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
                    replacement_region.row_path_suppression_bbox_for_page_width(vector_model.width)
                } else {
                    BoundingBox::default()
                },
                replacement_region,
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
        .collect::<Vec<_>>();

    if prepared_overlays.is_empty() {
        return visible_indices
            .into_iter()
            .map(|object_index| EffectiveVectorRenderEntry::Object {
                object_index,
                suppressed_text_runs: SuppressedVectorTextRuns::default(),
            })
            .collect();
    }

    let mut entries = Vec::with_capacity(visible_indices.len() + prepared_overlays.len());

    for object_index in visible_indices {
        let Some(object) = vector_model.objects.get(object_index) else {
            continue;
        };
        let mut suppressed_text_runs = SuppressedVectorTextRuns::default();
        let mut suppress_entire_object = false;
        for overlay in &mut prepared_overlays {
            let suppress_text_source = overlay_suppresses_text_source(&overlay.overlay);
            let suppress_row_paths = overlay_suppresses_row_paths(&overlay.overlay);
            if suppress_text_source {
                let text_object_index_match = overlay.object_indices.contains(&object_index)
                    && matches!(object, VectorRenderObject::Text(_));
                let text_object_source_match = text_object_matches_overlay_source_text(
                    &object,
                    &overlay.overlay.source_text,
                    &overlay.replacement_region,
                );
                if text_object_index_match || text_object_source_match {
                    overlay.suppressed_text_object_count =
                        overlay.suppressed_text_object_count.saturating_add(1);
                    suppress_entire_object = true;
                    if !overlay.inserted && !overlay_renders_last(&overlay.overlay) {
                        entries.push(EffectiveVectorRenderEntry::ParagraphOverlay(
                            overlay.overlay.clone(),
                        ));
                        overlay.inserted = true;
                    }
                    continue;
                }
            }
            if suppress_row_paths {
                if let Some(object_bbox) = vector_object_bbox(&object) {
                    if bbox_intersects(&object_bbox, &overlay.path_suppression_bbox) {
                        overlay.object_intersect_count =
                            overlay.object_intersect_count.saturating_add(1);
                        match object {
                            VectorRenderObject::Text(_) => {
                                overlay.text_intersect_count =
                                    overlay.text_intersect_count.saturating_add(1);
                            }
                            VectorRenderObject::Path(_) => {
                                overlay.path_intersect_count =
                                    overlay.path_intersect_count.saturating_add(1);
                            }
                            VectorRenderObject::Image(_) => {
                                overlay.image_intersect_count =
                                    overlay.image_intersect_count.saturating_add(1);
                            }
                        }
                        record_overlay_object_summary(
                            overlay,
                            vector_object_summary(&object, object_index),
                        );
                    }
                }
                if let Some(path_summary) = decorative_object_should_be_suppressed_by_overlay(
                    &object,
                    &overlay.replacement_region,
                    &overlay.path_suppression_bbox,
                ) {
                    overlay.thin_horizontal_path_count =
                        overlay.thin_horizontal_path_count.saturating_add(1);
                    overlay.suppressed_path_count = overlay.suppressed_path_count.saturating_add(1);
                    if overlay.first_path_summary.is_none() {
                        overlay.first_path_summary = Some(path_summary);
                    }
                    suppress_entire_object = true;
                    continue;
                }
                if let VectorRenderObject::Path(path) = object {
                    if let Some(path_bbox) = path_object_bbox(&path) {
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
            }
            if suppress_text_source {
                if text_object_should_be_suppressed(&object, &overlay.object_ids) {
                    overlay.suppressed_text_object_count =
                        overlay.suppressed_text_object_count.saturating_add(1);
                    suppress_entire_object = true;
                    if !overlay.inserted && !overlay_renders_last(&overlay.overlay) {
                        entries.push(EffectiveVectorRenderEntry::ParagraphOverlay(
                            overlay.overlay.clone(),
                        ));
                        overlay.inserted = true;
                    }
                    continue;
                }
                let matched_text_runs = matching_text_run_refs(
                    &object,
                    &overlay.object_ids,
                    &overlay.replacement_region,
                );
                if matched_text_runs.is_empty() {
                    continue;
                }
                let matched_run_count = if let VectorRenderObject::Text(text) = object {
                    matched_text_runs.suppressed_count_for_text_object(&text)
                } else {
                    0
                };
                overlay.suppressed_text_run_count = overlay
                    .suppressed_text_run_count
                    .saturating_add(matched_run_count);
                suppressed_text_runs.extend(matched_text_runs);
                if !overlay.inserted && !overlay_renders_last(&overlay.overlay) {
                    entries.push(EffectiveVectorRenderEntry::ParagraphOverlay(
                        overlay.overlay.clone(),
                    ));
                    overlay.inserted = true;
                }
            }
        }

        let should_skip_entire_object = match object {
            VectorRenderObject::Text(text) => {
                suppress_entire_object
                    || (!text.runs.is_empty()
                        && suppressed_text_runs.suppressed_count_for_text_object(&text)
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

    for overlay in prepared_overlays {
        let source_bbox = format!(
            "{:.1},{:.1},{:.1},{:.1}",
            overlay.replacement_region.source_bbox.left,
            overlay.replacement_region.source_bbox.top,
            overlay.replacement_region.source_bbox.right,
            overlay.replacement_region.source_bbox.bottom
        );
        let text_clear_bbox = format!(
            "{:.1},{:.1},{:.1},{:.1}",
            overlay.replacement_region.text_clear_bbox.left,
            overlay.replacement_region.text_clear_bbox.top,
            overlay.replacement_region.text_clear_bbox.right,
            overlay.replacement_region.text_clear_bbox.bottom
        );
        let path_suppression_bbox = format!(
            "{:.1},{:.1},{:.1},{:.1}",
            overlay.path_suppression_bbox.left,
            overlay.path_suppression_bbox.top,
            overlay.path_suppression_bbox.right,
            overlay.path_suppression_bbox.bottom
        );
        dbg_event(
            "effective-plan",
            "overlay-min",
            vec![dbg_field(
                "summary",
                format!(
                    "owner={:?} repl={} sp={} pi={} ii={} sb={} pb={} first={}",
                    overlay.overlay.owner,
                    overlay.overlay.replaces_source,
                    overlay.suppressed_path_count,
                    overlay.path_intersect_count,
                    overlay.image_intersect_count,
                    source_bbox,
                    path_suppression_bbox,
                    overlay.first_path_summary.as_deref().unwrap_or("none")
                ),
            )],
        );
        dbg_event(
            "effective-plan",
            "overlay-compact",
            vec![
                dbg_field("paragraphId", overlay.overlay.target.paragraph_id.as_str()),
                dbg_field("owner", format!("{:?}", overlay.overlay.owner)),
                dbg_field("replacesSource", overlay.overlay.replaces_source),
                dbg_field("sourceBBox", source_bbox.as_str()),
                dbg_field("textClearBBox", text_clear_bbox.as_str()),
                dbg_field("pathSuppressionBBox", path_suppression_bbox.as_str()),
                dbg_field("pathIntersectCount", overlay.path_intersect_count),
                dbg_field("imageIntersectCount", overlay.image_intersect_count),
                dbg_field("suppressedPathCount", overlay.suppressed_path_count),
                dbg_field(
                    "firstPathSummary",
                    overlay.first_path_summary.as_deref().unwrap_or("none"),
                ),
            ],
        );
        dbg_event(
            "effective-plan",
            "overlay-path-summary",
            vec![
                dbg_field("paragraphId", overlay.overlay.target.paragraph_id.as_str()),
                dbg_field("owner", format!("{:?}", overlay.overlay.owner)),
                dbg_field("replacesSource", overlay.overlay.replaces_source),
                dbg_field("sourceText", overlay.overlay.source_text.as_str()),
                dbg_field("draftText", overlay.overlay.draft_text.as_str()),
                dbg_field("sourceObjectIndexCount", overlay.object_indices.len()),
                dbg_field(
                    "sourceObjectIndices",
                    format!("{:?}", overlay.object_indices),
                ),
                dbg_field("textClearBBox", text_clear_bbox.as_str()),
                dbg_field("sourceBBox", source_bbox.as_str()),
                dbg_field("pathSuppressionBBox", path_suppression_bbox.as_str()),
                dbg_field("objectIntersectCount", overlay.object_intersect_count),
                dbg_field("textIntersectCount", overlay.text_intersect_count),
                dbg_field("pathIntersectCount", overlay.path_intersect_count),
                dbg_field("imageIntersectCount", overlay.image_intersect_count),
                dbg_field(
                    "thinHorizontalPathCount",
                    overlay.thin_horizontal_path_count,
                ),
                dbg_field("suppressedPathCount", overlay.suppressed_path_count),
                dbg_field(
                    "suppressedTextObjectCount",
                    overlay.suppressed_text_object_count,
                ),
                dbg_field("suppressedTextRunCount", overlay.suppressed_text_run_count),
                dbg_field("sourceObjectIdCount", overlay.object_ids.len()),
                dbg_field(
                    "firstPathSummary",
                    overlay.first_path_summary.as_deref().unwrap_or("none"),
                ),
                dbg_field(
                    "objectSummary1",
                    overlay.object_summary_1.as_deref().unwrap_or("none"),
                ),
                dbg_field(
                    "objectSummary2",
                    overlay.object_summary_2.as_deref().unwrap_or("none"),
                ),
                dbg_field(
                    "objectSummary3",
                    overlay.object_summary_3.as_deref().unwrap_or("none"),
                ),
            ],
        );
        if !overlay.inserted {
            entries.push(EffectiveVectorRenderEntry::ParagraphOverlay(
                overlay.overlay,
            ));
        }
    }

    entries
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
                        || glyph_run_spatially_matches_replacement_region(
                            run,
                            &replacement_region,
                        )
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
                entries.push(EffectiveGlyphRenderEntry::Paragraph(
                    GlyphParagraphRef {
                        region_index,
                        paragraph_index,
                        suppressed_run_object_ids,
                        suppressed_run_indices,
                    },
                ));
            }

            entries.extend(
                deferred_overlays
                    .into_iter()
                    .map(EffectiveGlyphRenderEntry::ParagraphOverlay),
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
    use crate::editor::session::ActiveEditorTarget;
    use crate::editor::paragraph_overlay::{
        ParagraphRenderOverlayOwner, ParagraphRenderOverlay,
    };
    use pdf_viewer_core::models::{
        BoundingBox, EditorControlStyle, EditorSession, GlyphPaintParagraph, GlyphPaintPlan,
        GlyphPaintRegion, GlyphPaintRun, LayoutMode, LayoutParagraph, LayoutRole, LayoutRun,
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
        target.scene.body_session = EditorSession {
            anchor_bbox: body_bbox,
            paragraph: LayoutParagraph::default(),
        };

        ParagraphRenderOverlay {
            owner: ParagraphRenderOverlayOwner::ActiveEditorShell,
            target,
            source_object_indices: Vec::new(),
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
                    editor_session: EditorSession {
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
    fn active_editor_suppresses_zero_height_stroked_row_path() {
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
    fn active_editor_keeps_section_divider_path_outside_text_row() {
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
    fn active_editor_keeps_nearby_divider_below_text_row() {
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
    fn active_editor_suppresses_row_path_touching_text_descenders() {
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
    fn active_editor_suppresses_text_object_when_runs_have_no_object_id() {
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
    fn active_editor_spatially_suppresses_text_run_when_source_ids_are_missing() {
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
    fn clean_active_editor_keeps_spatially_matching_text_visible() {
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
    fn clean_active_editor_keeps_source_text_object_visible() {
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
    fn clean_active_editor_suppresses_row_path_without_hiding_text() {
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
    fn active_editor_spatially_suppresses_glyph_run_when_source_ids_are_missing() {
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
    fn clean_active_editor_keeps_spatially_matching_glyph_run_visible() {
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
    fn persisted_overlay_spatially_suppresses_glyph_run_when_source_ids_are_missing() {
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
    fn persisted_overlay_renders_after_later_page_paths() {
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
    fn persisted_overlay_suppresses_row_path_after_commit() {
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
    fn replacement_region_keeps_right_tile_row_path_suppressed() {
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
}
