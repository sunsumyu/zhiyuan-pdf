use crate::infrastructure::pdf::models::{
    PdfMaterializationDecisionReport, PdfMaterializationReport, PdfMaterializationSourceStats,
    PersistableRegionPatch, PersistableSemanticOperation, TextReflowPatch,
};
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};

#[derive(Debug, Clone)]
pub struct RegionMaterializationDecision {
    pub region_id: String,
    pub source: String,
    pub status: &'static str,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct RegionMaterializationPlan {
    pub effective_text_reflows: Vec<TextReflowPatch>,
    pub decisions: Vec<RegionMaterializationDecision>,
}
impl RegionMaterializationPlan {
    pub fn to_report(
        &self,
        path: &str,
        region_patch_count: usize,
        explicit_text_reflow_count: usize,
    ) -> PdfMaterializationReport {
        let materialized_count = self
            .decisions
            .iter()
            .filter(|decision| decision.status == "materialized")
            .count();
        let skipped_count = self
            .decisions
            .iter()
            .filter(|decision| decision.status == "skipped")
            .count();
        let mut by_source_map: HashMap<String, (usize, usize)> = HashMap::new();
        for decision in &self.decisions {
            let entry = by_source_map
                .entry(decision.source.clone())
                .or_insert((0, 0));
            if decision.status == "materialized" {
                entry.0 += 1;
            } else {
                entry.1 += 1;
            }
        }
        let mut by_source: Vec<PdfMaterializationSourceStats> = by_source_map
            .into_iter()
            .map(
                |(source, (materialized, skipped))| PdfMaterializationSourceStats {
                    source,
                    materialized,
                    skipped,
                },
            )
            .collect();
        by_source.sort_by(|a, b| a.source.cmp(&b.source));
        PdfMaterializationReport {
            path: path.to_string(),
            region_patch_count,
            explicit_text_reflow_count,
            effective_text_reflow_count: self.effective_text_reflows.len(),
            materialized_count,
            skipped_count,
            by_source,
            decisions: self
                .decisions
                .iter()
                .map(|decision| PdfMaterializationDecisionReport {
                    region_id: decision.region_id.clone(),
                    source: decision.source.clone(),
                    status: decision.status.to_string(),
                    reason: decision.reason.clone(),
                })
                .collect(),
        }
    }
}
fn snapshot_text(snapshot: &Option<Value>) -> Option<String> {
    snapshot
        .as_ref()
        .and_then(|value| value.get("text"))
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
}
fn snapshot_lines_len(snapshot: &Option<Value>) -> usize {
    snapshot
        .as_ref()
        .and_then(|value| value.get("lines"))
        .and_then(|value| value.as_array())
        .map(|arr| arr.len())
        .unwrap_or(0)
}
fn snapshot_style_runs_len(snapshot: &Option<Value>) -> usize {
    snapshot
        .as_ref()
        .and_then(|value| value.get("styleRuns"))
        .and_then(|value| value.as_array())
        .map(|arr| arr.len())
        .unwrap_or(0)
}

#[derive(Debug, Clone)]
struct SnapshotLineReflow {
    target_indices: Vec<usize>,
    rendered_text: String,
}
fn snapshot_line_reflows(snapshot: &Option<Value>) -> Option<Vec<SnapshotLineReflow>> {
    let lines = snapshot
        .as_ref()
        .and_then(|value| value.get("lines"))
        .and_then(|value| value.as_array())?;

    let line_reflows = lines
        .iter()
        .map(|line| {
            let target_indices = line
                .get("objectIndices")
                .and_then(|value| value.as_array())
                .map(|entries| {
                    entries
                        .iter()
                        .filter_map(|entry| entry.as_u64().map(|value| value as usize))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let rendered_text = line
                .get("renderedText")
                .and_then(|value| value.as_str())
                .or_else(|| line.get("text").and_then(|value| value.as_str()))
                .unwrap_or("")
                .to_string();
            SnapshotLineReflow {
                target_indices,
                rendered_text,
            }
        })
        .collect::<Vec<_>>();

    if line_reflows.is_empty()
        || line_reflows
            .iter()
            .any(|line| line.target_indices.is_empty())
    {
        return None;
    }

    Some(line_reflows)
}
fn snapshot_field_texts(snapshot: &Option<Value>) -> Option<(String, String)> {
    let snapshot = snapshot.as_ref()?;
    let key_text = snapshot.get("keyText")?.as_str()?.to_string();
    let value_text = snapshot.get("valueText")?.as_str()?.to_string();
    Some((key_text, value_text))
}
fn rebuild_field_row_text_from_value_patch(patch: &PersistableRegionPatch) -> Option<String> {
    let original_value = patch.original_value_text.as_ref()?;
    let new_value = patch.new_value_text.as_ref()?;
    if original_value == new_value {
        return Some(patch.original_text.clone());
    }
    if patch.original_text.ends_with(original_value) {
        let prefix = &patch.original_text[..patch
            .original_text
            .len()
            .saturating_sub(original_value.len())];
        return Some(format!("{prefix}{new_value}"));
    }
    None
}
fn normalize_region_text(text: String) -> String {
    text.replace("\r\n", "\n")
}
fn snapshot_list_item_texts(snapshot: &Option<Value>) -> Option<(String, String)> {
    let snapshot = snapshot.as_ref()?;
    let marker_text = snapshot
        .get("markerText")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string();
    let body_text = snapshot
        .get("bodyText")
        .and_then(|value| value.as_str())
        .or_else(|| snapshot.get("text").and_then(|value| value.as_str()))
        .unwrap_or("")
        .to_string();
    Some((marker_text, body_text))
}
fn combine_list_item_text(marker_text: &str, body_text: &str) -> String {
    let marker = normalize_region_text(marker_text.to_string());
    let body = normalize_region_text(body_text.to_string());
    if marker.is_empty() {
        body
    } else if body.is_empty() {
        marker
    } else {
        format!("{marker}{body}")
    }
}
fn semantic_list_item_unit_text_reflow(
    patch: &PersistableRegionPatch,
) -> Option<(Vec<TextReflowPatch>, RegionMaterializationDecision)> {
    let summary = patch.semantic_block.as_ref()?;
    if summary.kind != "list-item" {
        return None;
    }
    let marker_text = patch
        .new_marker_text
        .clone()
        .or_else(|| summary.marker_text.clone())
        .or_else(|| patch.marker_text.clone())
        .filter(|text| !text.is_empty())?;
    if summary.marker_object_indices.is_empty() || summary.body_object_indices.is_empty() {
        return None;
    }

    let mut target_indices = summary
        .marker_object_indices
        .iter()
        .chain(summary.body_object_indices.iter())
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if target_indices.is_empty() {
        return None;
    }
    target_indices.sort_unstable();

    let body_text = patch
        .snapshot
        .as_ref()
        .and_then(|value| value.get("bodyText"))
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
        .unwrap_or_else(|| patch.new_text.clone());
    let new_text = combine_list_item_text(&marker_text, &body_text);

    Some((
        vec![TextReflowPatch {
            page_index: patch.page_index,
            target_indices,
            new_text: normalize_region_text(new_text),
            // Semantic text-marker list items are materialized as one full text unit.
            // Legacy patch.new_runs are body-only in the current editor pipeline, so carrying
            // them here would mismatch marker+body text and can corrupt save output.
            new_runs: None,
            alignment: patch.align,
            line_height: patch.line_height,
            displacement_y: patch.displacement_y,
            wrap_width: patch.wrap_width,
            char_spacing: patch.char_spacing,
            horizontal_scaling: patch.horizontal_scaling,
        }],
        RegionMaterializationDecision {
            region_id: patch.region_id.clone(),
            source: patch.source.clone(),
            status: "materialized",
            reason: "semantic-list-item-text-marker-unit".to_string(),
        },
    ))
}

fn is_valid_patch_target(patch: &PersistableRegionPatch) -> bool {
    !patch.target_indices.is_empty()
}
fn has_non_empty_snapshot_text(patch: &PersistableRegionPatch) -> bool {
    snapshot_text(&patch.snapshot).is_some_and(|v| !v.trim().is_empty())
}
fn has_structured_paragraph_snapshot(patch: &PersistableRegionPatch) -> bool {
    snapshot_lines_len(&patch.snapshot) > 0
}
fn is_valid_field_row_patch(patch: &PersistableRegionPatch) -> bool {
    is_valid_patch_target(patch)
        && patch.pair_id.as_ref().is_some_and(|v| !v.is_empty())
        && patch.group_id.as_ref().is_some_and(|v| !v.is_empty())
}
fn is_valid_paragraph_patch(patch: &PersistableRegionPatch) -> bool {
    is_valid_patch_target(patch)
        && (has_structured_paragraph_snapshot(patch)
            || has_non_empty_snapshot_text(patch)
            || !patch.new_text.trim().is_empty())
}
fn merge_text_reflows(
    region_reflows: Vec<TextReflowPatch>,
    explicit_reflows: &[TextReflowPatch],
) -> Vec<TextReflowPatch> {
    let mut merged: HashMap<String, TextReflowPatch> = HashMap::new();

    for reflow in region_reflows {
        let key = format!(
            "{}::{}",
            reflow.page_index,
            reflow
                .target_indices
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
        merged.insert(key, reflow);
    }

    for reflow in explicit_reflows {
        let key = format!(
            "{}::{}",
            reflow.page_index,
            reflow
                .target_indices
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
        merged.insert(key, reflow.clone());
    }

    merged.into_values().collect()
}
fn materialize_field_row_patch_to_text_reflow(
    patch: &PersistableRegionPatch,
) -> (Vec<TextReflowPatch>, RegionMaterializationDecision) {
    if !is_valid_field_row_patch(patch) {
        let reason = format!(
            "invalid-structure(pairId={}, groupId={}, targets={})",
            patch.pair_id.as_deref().unwrap_or("-"),
            patch.group_id.as_deref().unwrap_or("-"),
            patch.target_indices.len()
        );
        crate::log_step!(
            "[PDF][materialize][field-row][skip] region_id={} reason={}",
            patch.region_id,
            reason
        );
        return (
            Vec::new(),
            RegionMaterializationDecision {
                region_id: patch.region_id.clone(),
                source: patch.source.clone(),
                status: "skipped",
                reason,
            },
        );
    }
    let new_text = snapshot_field_texts(&patch.snapshot)
        .map(|(key_text, value_text)| format!("{key_text}{value_text}"))
        .or_else(|| rebuild_field_row_text_from_value_patch(patch))
        .unwrap_or_else(|| patch.new_text.clone());
    let reason = if snapshot_field_texts(&patch.snapshot).is_some() {
        "snapshot-key-value".to_string()
    } else if patch.new_value_text.is_some() {
        "value-patch-rebuild".to_string()
    } else {
        "fallback-new-text".to_string()
    };
    (
        vec![TextReflowPatch {
            page_index: patch.page_index,
            target_indices: patch.target_indices.clone(),
            new_text,
            new_runs: patch.new_runs.clone(),
            alignment: patch.align,
            line_height: patch.line_height,
            displacement_y: patch.displacement_y,
            wrap_width: patch.wrap_width,
            char_spacing: patch.char_spacing,
            horizontal_scaling: patch.horizontal_scaling,
        }],
        RegionMaterializationDecision {
            region_id: patch.region_id.clone(),
            source: patch.source.clone(),
            status: "materialized",
            reason,
        },
    )
}
fn materialize_snapshot_lines_to_text_reflows(
    patch: &PersistableRegionPatch,
) -> Option<Vec<TextReflowPatch>> {
    let line_reflows = snapshot_line_reflows(&patch.snapshot)?;
    Some(
        line_reflows
            .into_iter()
            .map(|line| TextReflowPatch {
                page_index: patch.page_index,
                target_indices: line.target_indices,
                new_text: normalize_region_text(line.rendered_text),
                new_runs: patch.new_runs.clone(), // Propagation from root patch if available
                alignment: patch.align,
                line_height: patch.line_height,
                displacement_y: None,
                wrap_width: None,
                char_spacing: patch.char_spacing,
                horizontal_scaling: patch.horizontal_scaling,
            })
            .collect(),
    )
}
fn materialize_paragraph_patch_to_text_reflow(
    patch: &PersistableRegionPatch,
) -> (Vec<TextReflowPatch>, RegionMaterializationDecision) {
    if !is_valid_paragraph_patch(patch) {
        let reason = format!(
            "invalid-structure(targets={}, snapshotText={}, snapshotLines={}, snapshotStyleRuns={}, newText={})",
            patch.target_indices.len(),
            has_non_empty_snapshot_text(patch),
            snapshot_lines_len(&patch.snapshot),
            snapshot_style_runs_len(&patch.snapshot),
            !patch.new_text.trim().is_empty()
        );
        crate::log_step!(
            "[PDF][materialize][paragraph][skip] region_id={} reason={}",
            patch.region_id,
            reason
        );
        return (
            Vec::new(),
            RegionMaterializationDecision {
                region_id: patch.region_id.clone(),
                source: patch.source.clone(),
                status: "skipped",
                reason,
            },
        );
    }
    if let Some(line_reflows) = materialize_snapshot_lines_to_text_reflows(patch) {
        let reason = format!(
            "snapshot-line-targets(lines={}, styleRuns={})",
            line_reflows.len(),
            snapshot_style_runs_len(&patch.snapshot)
        );
        return (
            line_reflows,
            RegionMaterializationDecision {
                region_id: patch.region_id.clone(),
                source: patch.source.clone(),
                status: "materialized",
                reason,
            },
        );
    }
    let new_text = snapshot_text(&patch.snapshot).unwrap_or_else(|| patch.new_text.clone());
    let reason = if has_non_empty_snapshot_text(patch) && has_structured_paragraph_snapshot(patch) {
        format!(
            "snapshot-text(lines={}, styleRuns={})",
            snapshot_lines_len(&patch.snapshot),
            snapshot_style_runs_len(&patch.snapshot)
        )
    } else {
        "fallback-new-text".to_string()
    };
    (
        vec![TextReflowPatch {
            page_index: patch.page_index,
            target_indices: patch.target_indices.clone(),
            new_text: normalize_region_text(new_text),
            new_runs: patch.new_runs.clone(),
            alignment: patch.align,
            line_height: patch.line_height,
            displacement_y: patch.displacement_y,
            wrap_width: patch.wrap_width,
            char_spacing: patch.char_spacing,
            horizontal_scaling: patch.horizontal_scaling,
        }],
        RegionMaterializationDecision {
            region_id: patch.region_id.clone(),
            source: patch.source.clone(),
            status: "materialized",
            reason,
        },
    )
}
fn materialize_list_item_patch_to_text_reflow(
    patch: &PersistableRegionPatch,
) -> (Vec<TextReflowPatch>, RegionMaterializationDecision) {
    if !is_valid_paragraph_patch(patch) {
        let reason = format!(
            "invalid-structure(targets={}, snapshotText={}, snapshotLines={}, snapshotStyleRuns={}, newText={})",
            patch.target_indices.len(),
            has_non_empty_snapshot_text(patch),
            snapshot_lines_len(&patch.snapshot),
            snapshot_style_runs_len(&patch.snapshot),
            !patch.new_text.trim().is_empty()
        );
        crate::log_step!(
            "[PDF][materialize][list-item][skip] region_id={} reason={}",
            patch.region_id,
            reason
        );
        return (
            Vec::new(),
            RegionMaterializationDecision {
                region_id: patch.region_id.clone(),
                source: patch.source.clone(),
                status: "skipped",
                reason,
            },
        );
    }
    if let Some(semantic_result) = semantic_list_item_unit_text_reflow(patch) {
        return semantic_result;
    }

    let has_marker_update = match (
        patch.marker_text.as_deref(),
        patch.new_marker_text.as_deref(),
    ) {
        (_, None) => false,
        (Some(current), Some(next)) => current != next,
        (None, Some(next)) => !next.is_empty(),
    };
    let target_indices = if has_marker_update && !patch.full_target_indices.is_empty() {
        patch.full_target_indices.clone()
    } else {
        patch.target_indices.clone()
    };
    let new_text = if has_marker_update {
        let marker_text = patch
            .new_marker_text
            .clone()
            .or_else(|| patch.marker_text.clone())
            .or_else(|| snapshot_list_item_texts(&patch.snapshot).map(|(marker, _)| marker))
            .unwrap_or_default();
        let body_text = patch
            .snapshot
            .as_ref()
            .and_then(|value| value.get("bodyText"))
            .and_then(|value| value.as_str())
            .map(|value| value.to_string())
            .unwrap_or_else(|| patch.new_text.clone());
        combine_list_item_text(&marker_text, &body_text)
    } else {
        patch.new_text.clone()
    };
    let reason = if has_marker_update {
        "list-marker-body-new-text".to_string()
    } else {
        "list-body-new-text".to_string()
    };
    (
        vec![TextReflowPatch {
            page_index: patch.page_index,
            target_indices,
            new_text: normalize_region_text(new_text),
            new_runs: patch.new_runs.clone(),
            alignment: patch.align,
            line_height: patch.line_height,
            displacement_y: patch.displacement_y,
            wrap_width: patch.wrap_width,
            char_spacing: patch.char_spacing,
            horizontal_scaling: patch.horizontal_scaling,
        }],
        RegionMaterializationDecision {
            region_id: patch.region_id.clone(),
            source: patch.source.clone(),
            status: "materialized",
            reason,
        },
    )
}
fn materialize_region_patch_to_text_reflow(
    patch: &PersistableRegionPatch,
) -> (Vec<TextReflowPatch>, RegionMaterializationDecision) {
    match patch.source.as_str() {
        "field-row" => {
            crate::log_step!(
                "[PDF][materialize][field-row] region_id={} group_id={} pair_id={} field_kind={}",
                patch.region_id,
                patch.group_id.as_deref().unwrap_or("-"),
                patch.pair_id.as_deref().unwrap_or("-"),
                patch.field_kind.as_deref().unwrap_or("-")
            );
            materialize_field_row_patch_to_text_reflow(patch)
        }
        "paragraph-region" => {
            crate::log_step!(
                "[PDF][materialize][paragraph] region_id={} kind={}",
                patch.region_id,
                patch.kind.as_deref().unwrap_or("paragraph")
            );
            materialize_paragraph_patch_to_text_reflow(patch)
        }
        "list-item-region" => {
            crate::log_step!(
                "[PDF][materialize][list-item] region_id={} kind={}",
                patch.region_id,
                patch.kind.as_deref().unwrap_or("list-item")
            );
            materialize_list_item_patch_to_text_reflow(patch)
        }
        _ => {
            crate::log_step!(
                "[PDF][materialize][fallback] region_id={} source={}",
                patch.region_id,
                patch.source
            );
            if !is_valid_patch_target(patch) {
                let reason = "empty-targets".to_string();
                crate::log_step!(
                    "[PDF][materialize][fallback][skip] region_id={} reason={}",
                    patch.region_id,
                    reason
                );
                return (
                    Vec::new(),
                    RegionMaterializationDecision {
                        region_id: patch.region_id.clone(),
                        source: patch.source.clone(),
                        status: "skipped",
                        reason,
                    },
                );
            }
            (
                vec![TextReflowPatch {
                    page_index: patch.page_index,
                    target_indices: patch.target_indices.clone(),
                    new_text: patch.new_text.clone(),
                    new_runs: patch.new_runs.clone(),
                    alignment: patch.align,
                    line_height: patch.line_height,
                    displacement_y: patch.displacement_y,
                    wrap_width: patch.wrap_width,
                    char_spacing: patch.char_spacing,
                    horizontal_scaling: patch.horizontal_scaling,
                }],
                RegionMaterializationDecision {
                    region_id: patch.region_id.clone(),
                    source: patch.source.clone(),
                    status: "materialized",
                    reason: "fallback-new-text".to_string(),
                },
            )
        }
    }
}
pub fn build_region_materialization_plan_v2(
    region_patches: &[PersistableRegionPatch],
    semantic_ops: &[PersistableSemanticOperation],
    text_reflows: &[TextReflowPatch],
) -> RegionMaterializationPlan {
    let mut decisions = Vec::with_capacity(region_patches.len());
    let mut region_reflows = Vec::new();
    let embedded_semantic_op_count = region_patches
        .iter()
        .map(|patch| patch.semantic_ops.len())
        .sum::<usize>();
    if !semantic_ops.is_empty() || embedded_semantic_op_count > 0 {
        crate::log_step!(
            "[PDF][materialize][semantic-v2] explicit_ops={} embedded_ops={}",
            semantic_ops.len(),
            embedded_semantic_op_count
        );
    }
    for patch in region_patches {
        let (reflows, decision) = materialize_region_patch_to_text_reflow(patch);
        decisions.push(decision);
        region_reflows.extend(reflows);
    }
    RegionMaterializationPlan {
        effective_text_reflows: merge_text_reflows(region_reflows, text_reflows),
        decisions,
    }
}

pub fn build_region_materialization_plan(
    region_patches: &[PersistableRegionPatch],
    text_reflows: &[TextReflowPatch],
) -> RegionMaterializationPlan {
    build_region_materialization_plan_v2(region_patches, &[], text_reflows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdf_viewer_core::persistence::models::PersistableSemanticBlockSummary;
    use serde_json::json;

    fn list_patch(
        marker_text: Option<&str>,
        new_marker_text: Option<&str>,
    ) -> PersistableRegionPatch {
        PersistableRegionPatch {
            patch_key: "list:p1".to_string(),
            page_index: 0,
            region_id: "p1".to_string(),
            original_text: "Body".to_string(),
            new_text: "Body edited".to_string(),
            source: "list-item-region".to_string(),
            marker_text: marker_text.map(|text| text.to_string()),
            new_marker_text: new_marker_text.map(|text| text.to_string()),
            snapshot: Some(json!({
                "bodyText": "Body edited",
                "markerText": marker_text.unwrap_or(""),
                "newMarkerText": new_marker_text.unwrap_or(""),
            })),
            target_indices: vec![2],
            full_target_indices: vec![1, 2],
            wrap_width: Some(100.0),
            ..Default::default()
        }
    }

    fn semantic_text_marker_list_patch() -> PersistableRegionPatch {
        let mut patch = list_patch(Some("●"), None);
        patch.semantic_block = Some(PersistableSemanticBlockSummary {
            block_id: "p1".to_string(),
            region_id: "p1".to_string(),
            kind: "list-item".to_string(),
            body_text: "Body edited".to_string(),
            marker_text: Some("●".to_string()),
            body_object_indices: vec![2],
            marker_object_indices: vec![1],
            graphic_marker_object_indices: Vec::new(),
        });
        patch.new_runs = Some(Vec::new());
        patch
    }

    fn semantic_graphic_marker_list_patch() -> PersistableRegionPatch {
        let mut patch = list_patch(None, None);
        patch.semantic_block = Some(PersistableSemanticBlockSummary {
            block_id: "p1".to_string(),
            region_id: "p1".to_string(),
            kind: "list-item".to_string(),
            body_text: "Body edited".to_string(),
            marker_text: None,
            body_object_indices: vec![2],
            marker_object_indices: Vec::new(),
            graphic_marker_object_indices: vec![1],
        });
        patch
    }

    #[test]
    fn semantic_text_marker_list_item_materializes_marker_and_body_as_unit() {
        let plan =
            build_region_materialization_plan_v2(&[semantic_text_marker_list_patch()], &[], &[]);
        assert_eq!(plan.effective_text_reflows.len(), 1);
        let reflow = &plan.effective_text_reflows[0];
        assert_eq!(reflow.target_indices, vec![1, 2]);
        assert_eq!(reflow.new_text, "●Body edited");
        assert!(
            reflow.new_runs.is_none(),
            "body-only legacy runs must not be reused for marker+body text"
        );
        assert_eq!(
            plan.decisions[0].reason,
            "semantic-list-item-text-marker-unit"
        );
    }

    #[test]
    fn semantic_graphic_marker_list_item_keeps_body_only_legacy_materialization() {
        let plan =
            build_region_materialization_plan_v2(&[semantic_graphic_marker_list_patch()], &[], &[]);
        assert_eq!(plan.effective_text_reflows.len(), 1);
        let reflow = &plan.effective_text_reflows[0];
        assert_eq!(reflow.target_indices, vec![2]);
        assert_eq!(reflow.new_text, "Body edited");
        assert_eq!(plan.decisions[0].reason, "list-body-new-text");
    }

    #[test]
    fn body_only_list_patch_materializes_body_targets_only() {
        let plan = build_region_materialization_plan(&[list_patch(Some("●"), None)], &[]);
        assert_eq!(plan.effective_text_reflows.len(), 1);
        let reflow = &plan.effective_text_reflows[0];
        assert_eq!(reflow.target_indices, vec![2]);
        assert_eq!(reflow.new_text, "Body edited");
    }

    #[test]
    fn marker_update_uses_full_targets_and_combines_marker_before_body() {
        let plan = build_region_materialization_plan(&[list_patch(Some("1."), Some("2."))], &[]);
        assert_eq!(plan.effective_text_reflows.len(), 1);
        let reflow = &plan.effective_text_reflows[0];
        assert_eq!(reflow.target_indices, vec![1, 2]);
        assert_eq!(reflow.new_text, "2.Body edited");
    }

    #[test]
    fn v2_keeps_legacy_materialization_compatible_when_semantic_ops_exist() {
        let mut patch = list_patch(Some("●"), None);
        patch.semantic_ops = vec![PersistableSemanticOperation::ReplaceBodyText {
            block_id: "p1".to_string(),
            old_text: "Body".to_string(),
            new_text: "Body edited".to_string(),
        }];
        let plan = build_region_materialization_plan_v2(&[patch], &[], &[]);
        assert_eq!(plan.effective_text_reflows.len(), 1);
        assert_eq!(plan.effective_text_reflows[0].target_indices, vec![2]);
        assert_eq!(plan.effective_text_reflows[0].new_text, "Body edited");
    }
}
