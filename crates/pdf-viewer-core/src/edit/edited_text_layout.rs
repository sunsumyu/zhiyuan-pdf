use crate::models::LayoutRun;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditedTextGeometryPolicy {
    PreserveSourceGeometry,
    MeasureEditedText,
}

pub fn resolve_edited_text_geometry_policy(
    source_text: &str,
    draft_text: &str,
    source_runs_match_text: bool,
) -> EditedTextGeometryPolicy {
    if draft_text == source_text && source_runs_match_text {
        EditedTextGeometryPolicy::PreserveSourceGeometry
    } else {
        EditedTextGeometryPolicy::MeasureEditedText
    }
}

pub fn strip_source_geometry_for_edited_text(runs: &mut [LayoutRun]) {
    for run in runs {
        run.char_origins.clear();
        run.char_widths.clear();
        run.object_ids.clear();
        run.object_indices.clear();
    }
}
