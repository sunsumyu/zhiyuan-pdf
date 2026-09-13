//! Overlay 调试事件追踪：身份、最小摘要、紧凑摘要、路径摘要。

use crate::edit::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use crate::models::{VectorPageModel, VectorRenderObject};

use super::PreparedOverlay;

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
