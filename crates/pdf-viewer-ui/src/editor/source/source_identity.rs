use std::collections::{BTreeSet, HashSet};

use pdf_viewer_core::models::{GlyphPaintRun, VectorPageModel, VectorRenderObject};

use crate::editor::session::ActiveEditorTarget;

pub(crate) fn collect_target_source_object_ids(
    target: &ActiveEditorTarget,
) -> HashSet<String> {
    let mut object_ids = target
        .scene
        .original_runs
        .iter()
        .flat_map(|run| run.object_ids.iter().cloned())
        .collect::<HashSet<_>>();

    object_ids.extend(
        target
            .scene
            .body_session
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

    object_ids
}

pub(crate) fn collect_target_source_object_indices_set(
    target: &ActiveEditorTarget,
) -> HashSet<usize> {
    let mut object_indices = target
        .scene
        .original_runs
        .iter()
        .flat_map(|run| run.object_indices.iter().copied())
        .collect::<HashSet<_>>();

    object_indices.extend(
        target
            .scene
            .body_session
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

    object_indices
}

pub(crate) fn collect_target_source_object_indices(target: &ActiveEditorTarget) -> Vec<usize> {
    let ordered = collect_target_source_object_indices_set(target)
        .into_iter()
        .collect::<BTreeSet<_>>();
    ordered.into_iter().collect()
}

pub(crate) fn collect_object_indices_from_runs(
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
