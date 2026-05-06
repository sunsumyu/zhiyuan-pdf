use crate::editor::session::ActiveEditorTarget;
use crate::models::PersistableRegionPatch;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeSet;

const REPLACEMENT_SNAPSHOT_SCHEMA: &str = "editReplacementSnapshot.v3";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditReplacementSnapshot {
    pub schema: String,
    pub id: String,
    pub region_id: String,
    pub kind: String,
    pub text: String,
    pub body_text: String,
    #[serde(default)]
    pub marker_text: Option<String>,
    #[serde(default)]
    pub new_marker_text: Option<String>,
    #[serde(default)]
    pub object_indices: Vec<usize>,
    #[serde(default)]
    pub wrap_width: f32,
    // Kept only for backward compatibility with snapshots produced before the
    // in-memory replacement-target cache. New patches must not serialize this.
    #[serde(default)]
    pub replacement_target: ActiveEditorTarget,
}

pub fn build_edit_replacement_snapshot(
    replacement_target: ActiveEditorTarget,
    kind: &str,
    body_text: String,
    marker_text: Option<String>,
    new_marker_text: Option<String>,
) -> Value {
    let object_indices = replacement_object_indices(&replacement_target);
    let wrap_width = replacement_target
        .scene
        .body_session
        .paragraph
        .wrap_width
        .max(replacement_target.scene.shell_bbox.right - replacement_target.scene.shell_bbox.left);
    let effective_marker = new_marker_text.clone().or(marker_text.clone());
    let combined_text = effective_marker
        .as_deref()
        .filter(|marker| !marker.is_empty())
        .map(|marker| {
            if body_text.is_empty() {
                marker.to_string()
            } else {
                format!("{marker}{body_text}")
            }
        })
        .unwrap_or_else(|| body_text.clone());

    json!({
        "schema": REPLACEMENT_SNAPSHOT_SCHEMA,
        "id": replacement_target.paragraph_id,
        "regionId": replacement_target.region_id,
        "kind": kind,
        "text": combined_text,
        "bodyText": body_text,
        "markerText": marker_text,
        "newMarkerText": new_marker_text,
        "objectIndices": object_indices,
        "wrapWidth": wrap_width,
    })
}

pub fn replacement_target_from_patch_snapshot(
    patch: &PersistableRegionPatch,
) -> Option<ActiveEditorTarget> {
    let snapshot = patch.snapshot.as_ref()?;
    snapshot
        .get("replacementTarget")
        .cloned()
        .and_then(|value| serde_json::from_value::<ActiveEditorTarget>(value).ok())
}

fn replacement_object_indices(target: &ActiveEditorTarget) -> Vec<usize> {
    let mut indices = target
        .scene
        .original_runs
        .iter()
        .flat_map(|run| run.object_indices.iter().copied())
        .collect::<BTreeSet<_>>();
    if let Some(marker) = target.scene.marker.as_ref() {
        indices.extend(
            marker
                .runs
                .iter()
                .flat_map(|run| run.object_indices.iter().copied()),
        );
    }
    if indices.is_empty() {
        indices.extend(
            target
                .scene
                .body_session
                .paragraph
                .runs
                .iter()
                .flat_map(|run| run.object_indices.iter().copied()),
        );
    }
    indices.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::{build_edit_replacement_snapshot, replacement_target_from_patch_snapshot};
    use crate::editor::session::ActiveEditorTarget;
    use crate::models::PersistableRegionPatch;
    use pdf_viewer_core::models::BoundingBox;

    #[test]
    fn replacement_snapshot_stays_lightweight() {
        let mut target = ActiveEditorTarget {
            paragraph_id: "p-1".to_string(),
            region_id: "r-1".to_string(),
            text: "old".to_string(),
            ..Default::default()
        };
        target.scene.shell_bbox = BoundingBox {
            left: 10.0,
            top: 20.0,
            right: 200.0,
            bottom: 40.0,
        };
        target.scene.body_session.anchor_bbox = BoundingBox {
            left: 30.0,
            top: 20.0,
            right: 190.0,
            bottom: 36.0,
        };

        let snapshot = build_edit_replacement_snapshot(
            target.clone(),
            "paragraph",
            "new".to_string(),
            None,
            None,
        );
        let patch = PersistableRegionPatch {
            region_id: "p-1".to_string(),
            snapshot: Some(snapshot),
            ..Default::default()
        };

        assert!(
            replacement_target_from_patch_snapshot(&patch).is_none(),
            "new persisted patch snapshots must not carry the full editor target"
        );
        let snapshot = patch.snapshot.as_ref().expect("snapshot should exist");
        assert!(snapshot.get("replacementTarget").is_none());
        assert_eq!(
            snapshot
                .get("wrapWidth")
                .and_then(|value| value.as_f64())
                .unwrap_or_default(),
            190.0
        );
    }
}
