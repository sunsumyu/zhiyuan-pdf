//! Overlay helper functions — 从 effective_page_plan.rs 拆分。
//!
//! 包含 overlay 准备、追踪、bbox 计算等辅助函数。

use std::collections::HashSet;

use crate::edit::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use crate::edit::paragraph_overlay::{ParagraphRenderOverlay, ParagraphRenderOverlayOwner};
use crate::edit::replacement_region::{build_region, ParagraphReplacementRegion};
use crate::edit::source_identity::{object_ids, object_indices_set};
use crate::models::{BoundingBox, VectorPageModel, VectorRenderObject};
use crate::render::prepared_scene::PreparedPageScene;
use crate::render::viewport_culling::{path_bbox, run_bbox, vector_object_intersects_viewport};

use super::{EffectiveVectorRenderEntry, SuppressedVectorTextRuns};

/// Overlay 经过预计算后的状态，用于渲染计划构建。
pub(super) struct PreparedOverlay {
    pub overlay: ParagraphRenderOverlay,
    pub replacement_region: ParagraphReplacementRegion,
    pub object_ids: HashSet<String>,
    pub object_indices: HashSet<usize>,
    pub path_suppression_bbox: BoundingBox,
    pub inserted: bool,
    pub suppressed_text_object_count: usize,
    pub suppressed_text_run_count: usize,
    pub object_intersect_count: usize,
    pub text_intersect_count: usize,
    pub path_intersect_count: usize,
    pub image_intersect_count: usize,
    pub thin_horizontal_path_count: usize,
    pub suppressed_path_count: usize,
    pub first_path_summary: Option<String>,
    pub object_summary_1: Option<String>,
    pub object_summary_2: Option<String>,
    pub object_summary_3: Option<String>,
}

// --- Overlay 属性查询 ---

pub(super) fn overlay_paragraph_object_ids(overlay: &ParagraphRenderOverlay) -> HashSet<String> {
    object_ids(&overlay.target)
}

pub(super) fn overlay_paragraph_object_indices(overlay: &ParagraphRenderOverlay) -> HashSet<usize> {
    let mut object_indices = object_indices_set(&overlay.target);
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
    let replacement_region = build_region(&overlay.target);
    let cull_bbox = replacement_region.viewport_cull_bbox(page_width);
    cull_bbox.left <= viewport_bbox.right
        && cull_bbox.right >= viewport_bbox.left
        && cull_bbox.top <= viewport_bbox.bottom
        && cull_bbox.bottom >= viewport_bbox.top
}

// --- Vector object 辅助 ---

pub(super) fn vector_object_bbox(object: &VectorRenderObject) -> Option<BoundingBox> {
    match object {
        VectorRenderObject::Text(text) => {
            let mut combined: Option<BoundingBox> = None;
            for run in &text.runs {
                let run_bbox = run_bbox(run);
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
        VectorRenderObject::Path(path) => path_bbox(path),
        VectorRenderObject::Image(image) => Some(BoundingBox {
            left: image.x,
            top: image.y,
            right: image.x + image.width.max(0.0),
            bottom: image.y + image.height.max(0.0),
        }),
    }
}

pub(super) fn vector_object_summary(object: &VectorRenderObject, object_index: usize) -> String {
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

pub(super) fn record_overlay_object_summary(overlay: &mut PreparedOverlay, summary: String) {
    if overlay.object_summary_1.is_none() {
        overlay.object_summary_1 = Some(summary);
    } else if overlay.object_summary_2.is_none() {
        overlay.object_summary_2 = Some(summary);
    } else if overlay.object_summary_3.is_none() {
        overlay.object_summary_3 = Some(summary);
    }
}

// --- Overlay 准备与解析 ---

pub(super) fn resolve_visible_indices(
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
            let rr = build_region(&overlay.target);
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
                    rr.row_suppression_bbox(page_width)
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

// --- 追踪 ---

pub(super) fn trace_overlay_identity(po: &[PreparedOverlay], vi: &[usize], vm: &VectorPageModel) {
    for (i, ov) in po.iter().enumerate() {
        dbg_event(
            "effective-plan",
            "overlay-identity",
            vec![
                dbg_field("overlayIndex", i),
                dbg_field("paragraphId", ov.overlay.target.paragraph_id.as_str()),
                dbg_field("owner", format!("{:?}", ov.overlay.owner)),
                dbg_field("replacesSource", ov.overlay.replaces_source),
                dbg_field(
                    "objectIds",
                    format!("{:?}", ov.object_ids.iter().collect::<Vec<_>>()),
                ),
                dbg_field("objectIdCount", ov.object_ids.len()),
                dbg_field("objectIndices", format!("{:?}", ov.object_indices)),
                dbg_field("objectIndexCount", ov.object_indices.len()),
                dbg_field(
                    "sourceText",
                    crate::common::debug::truncate_debug_text(&ov.overlay.source_text, 40),
                ),
                dbg_field(
                    "draftText",
                    crate::common::debug::truncate_debug_text(&ov.overlay.draft_text, 40),
                ),
            ],
        );
    }
    for &idx in vi {
        if let Some(VectorRenderObject::Text(t)) = vm.objects.get(idx) {
            dbg_event(
                "effective-plan",
                "vector-text-object",
                vec![
                    dbg_field("objectIndex", idx),
                    dbg_field("objectId", t.id.as_str()),
                    dbg_field("runCount", t.runs.len()),
                    dbg_field(
                        "firstRunText",
                        t.runs
                            .first()
                            .map(|r| crate::common::debug::truncate_debug_text(&r.text, 30))
                            .unwrap_or_default(),
                    ),
                ],
            );
        }
    }
}

pub(super) fn trace_overlay_summary(o: &PreparedOverlay) {
    let sb = format!(
        "{:.1},{:.1},{:.1},{:.1}",
        o.replacement_region.source_bbox.left,
        o.replacement_region.source_bbox.top,
        o.replacement_region.source_bbox.right,
        o.replacement_region.source_bbox.bottom
    );
    let tcb = format!(
        "{:.1},{:.1},{:.1},{:.1}",
        o.replacement_region.text_clear_bbox.left,
        o.replacement_region.text_clear_bbox.top,
        o.replacement_region.text_clear_bbox.right,
        o.replacement_region.text_clear_bbox.bottom
    );
    let pb = format!(
        "{:.1},{:.1},{:.1},{:.1}",
        o.path_suppression_bbox.left,
        o.path_suppression_bbox.top,
        o.path_suppression_bbox.right,
        o.path_suppression_bbox.bottom
    );
    dbg_event(
        "effective-plan",
        "overlay-min",
        vec![dbg_field(
            "summary",
            format!(
                "owner={:?} repl={} sp={} pi={} ii={} sb={} pb={} first={}",
                o.overlay.owner,
                o.overlay.replaces_source,
                o.suppressed_path_count,
                o.path_intersect_count,
                o.image_intersect_count,
                sb,
                pb,
                o.first_path_summary.as_deref().unwrap_or("none")
            ),
        )],
    );
    dbg_event(
        "effective-plan",
        "overlay-compact",
        vec![
            dbg_field("paragraphId", o.overlay.target.paragraph_id.as_str()),
            dbg_field("owner", format!("{:?}", o.overlay.owner)),
            dbg_field("replacesSource", o.overlay.replaces_source),
            dbg_field("sourceBBox", sb.as_str()),
            dbg_field("textClearBBox", tcb.as_str()),
            dbg_field("pathSuppressionBBox", pb.as_str()),
            dbg_field("pathIntersectCount", o.path_intersect_count),
            dbg_field("imageIntersectCount", o.image_intersect_count),
            dbg_field("suppressedPathCount", o.suppressed_path_count),
            dbg_field(
                "firstPathSummary",
                o.first_path_summary.as_deref().unwrap_or("none"),
            ),
        ],
    );
    dbg_event(
        "effective-plan",
        "overlay-path-summary",
        vec![
            dbg_field("paragraphId", o.overlay.target.paragraph_id.as_str()),
            dbg_field("owner", format!("{:?}", o.overlay.owner)),
            dbg_field("replacesSource", o.overlay.replaces_source),
            dbg_field("sourceText", o.overlay.source_text.as_str()),
            dbg_field("draftText", o.overlay.draft_text.as_str()),
            dbg_field("sourceObjectIndexCount", o.object_indices.len()),
            dbg_field("sourceObjectIndices", format!("{:?}", o.object_indices)),
            dbg_field("textClearBBox", tcb.as_str()),
            dbg_field("sourceBBox", sb.as_str()),
            dbg_field("pathSuppressionBBox", pb.as_str()),
            dbg_field("objectIntersectCount", o.object_intersect_count),
            dbg_field("textIntersectCount", o.text_intersect_count),
            dbg_field("pathIntersectCount", o.path_intersect_count),
            dbg_field("imageIntersectCount", o.image_intersect_count),
            dbg_field("thinHorizontalPathCount", o.thin_horizontal_path_count),
            dbg_field("suppressedPathCount", o.suppressed_path_count),
            dbg_field("suppressedTextObjectCount", o.suppressed_text_object_count),
            dbg_field("suppressedTextRunCount", o.suppressed_text_run_count),
            dbg_field("sourceObjectIdCount", o.object_ids.len()),
            dbg_field(
                "firstPathSummary",
                o.first_path_summary.as_deref().unwrap_or("none"),
            ),
            dbg_field(
                "objectSummary1",
                o.object_summary_1.as_deref().unwrap_or("none"),
            ),
            dbg_field(
                "objectSummary2",
                o.object_summary_2.as_deref().unwrap_or("none"),
            ),
            dbg_field(
                "objectSummary3",
                o.object_summary_3.as_deref().unwrap_or("none"),
            ),
        ],
    );
}

pub(super) fn insert_overlay_if_needed(
    o: &mut PreparedOverlay,
    e: &mut Vec<EffectiveVectorRenderEntry>,
) {
    if !o.inserted && !overlay_renders_last(&o.overlay) {
        e.push(EffectiveVectorRenderEntry::ParagraphOverlay(
            o.overlay.clone(),
        ));
        o.inserted = true;
    }
}

pub(super) fn build_entries_without_overlays(
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
