//! 源对象身份收集 — 从 ui::editor::source::source_identity 迁入。
//! 纯计算，无 wasm 依赖。

use std::collections::{BTreeSet, HashSet};

use crate::edit::active_target::ActiveEditorTarget;
use crate::models::{GlyphPaintRun, VectorPageModel, VectorRenderObject};

pub fn collect_target_source_object_ids(target: &ActiveEditorTarget) -> HashSet<String> {
    let mut object_ids = target
        .scene
        .original_runs()
        .iter()
        .flat_map(|run| run.object_ids.iter().cloned())
        .collect::<HashSet<_>>();

    object_ids.extend(
        target
            .scene
            .body_session()
            .paragraph
            .runs
            .iter()
            .flat_map(|run| run.object_ids.iter().cloned()),
    );
    object_ids.extend(
        target
            .scene
            .document_plan
            .body_session
            .paragraph
            .runs
            .iter()
            .flat_map(|run| run.object_ids.iter().cloned()),
    );
    object_ids.extend(
        target
            .editor_session
            .paragraph
            .runs
            .iter()
            .flat_map(|run| run.object_ids.iter().cloned()),
    );

    object_ids.extend(
        target
            .scene
            .marker()
            .into_iter()
            .flat_map(|marker| marker.runs.iter())
            .flat_map(|run| run.object_ids.iter().cloned()),
    );

    object_ids
}

pub fn collect_object_index_set(target: &ActiveEditorTarget) -> HashSet<usize> {
    let mut object_indices = target
        .scene
        .original_runs()
        .iter()
        .flat_map(|run| run.object_indices.iter().copied())
        .collect::<HashSet<_>>();

    object_indices.extend(
        target
            .scene
            .body_session()
            .paragraph
            .runs
            .iter()
            .flat_map(|run| run.object_indices.iter().copied()),
    );
    object_indices.extend(
        target
            .scene
            .document_plan
            .body_session
            .paragraph
            .runs
            .iter()
            .flat_map(|run| run.object_indices.iter().copied()),
    );
    object_indices.extend(
        target
            .editor_session
            .paragraph
            .runs
            .iter()
            .flat_map(|run| run.object_indices.iter().copied()),
    );

    object_indices.extend(
        target
            .scene
            .marker()
            .into_iter()
            .flat_map(|marker| marker.runs.iter())
            .flat_map(|run| run.object_indices.iter().copied()),
    );

    object_indices
}

pub fn collect_target_source_object_indices(target: &ActiveEditorTarget) -> Vec<usize> {
    let ordered = collect_object_index_set(target)
        .into_iter()
        .collect::<BTreeSet<_>>();
    ordered.into_iter().collect()
}

pub fn collect_run_indices(
    runs: &[GlyphPaintRun],
    vector_model: Option<&VectorPageModel>,
) -> Vec<usize> {
    let direct_indices = runs
        .iter()
        .flat_map(|run| run.object_indices.iter().copied())
        .collect::<BTreeSet<_>>();
    if !direct_indices.is_empty() {
        return direct_indices.into_iter().collect();
    }

    let object_ids = runs
        .iter()
        .flat_map(|run| run.object_ids.iter().cloned())
        .collect::<BTreeSet<_>>();
    if object_ids.is_empty() {
        return Vec::new();
    }
    let Some(vector_model) = vector_model else {
        return Vec::new();
    };

    let mut target_indices = BTreeSet::new();
    for object in &vector_model.objects {
        if let VectorRenderObject::Text(text) = object {
            if object_ids.contains(&text.id) {
                target_indices.insert(text.z_index);
            }
        }
    }

    target_indices.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::{collect_object_index_set, collect_target_source_object_ids};
    use crate::edit::active_target::ActiveEditorTarget;
    use crate::edit::document_plan::ParagraphEditorMarker;
    use crate::models::{BoundingBox, LayoutRun, RunStyle};
    use crate::text::list_semantics::ListMarkerKind;

    fn test_run(id: &str, object_id: &str, object_index: usize) -> LayoutRun {
        LayoutRun {
            id: id.to_string(),
            text: "•".to_string(),
            style: RunStyle {
                font_name: "Arial".to_string(),
                font_size: 12.0,
                color: "#111111".to_string(),
                is_bold: false,
                is_italic: false,
                is_underline: false,
                char_spacing: 0.0,
                scale_x: 1.0,
            },
            bbox: BoundingBox {
                left: 40.0,
                top: 100.0,
                right: 48.0,
                bottom: 112.0,
            },
            origin_x: 40.0,
            origin_y: 112.0,
            char_origins: Vec::new(),
            char_widths: Vec::new(),
            object_ids: vec![object_id.to_string()],
            object_indices: vec![object_index],
        }
    }

    #[test]
    fn collects_marker_run_source_identity() {
        let mut target = ActiveEditorTarget::default();
        *target.scene.marker_mut() = Some(ParagraphEditorMarker {
            kind: ListMarkerKind::Bullet,
            text: "•".to_string(),
            advance: 0.0,
            runs: vec![test_run("marker-run", "marker-object", 41)],
        });

        let object_ids = collect_target_source_object_ids(&target);
        let object_indices = collect_object_index_set(&target);

        assert!(object_ids.contains("marker-object"));
        assert!(object_indices.contains(&41));
    }
}
