//! 源文本抑制 — 从 ui::render::source_suppression 迁入。
//! 纯数据/纯计算，无 wasm 依赖。

use std::collections::HashSet;

use crate::edit::paragraph_overlay::ParagraphRenderOverlay;
use crate::edit::replacement_region::{build_region, ParagraphReplacementRegion};
use crate::geometry::bbox_ops::bbox_intersects;
use crate::models::{
    BoundingBox, GlyphPaintParagraph, GlyphPaintRun, StyledRun, VectorRenderObject,
    VectorTextObject,
};
use crate::render::viewport_culling::run_bbox;

#[derive(Debug, Clone, Default)]
pub struct SuppressedVectorTextRuns {
    pub object_ids: HashSet<String>,
    pub run_indices: HashSet<usize>,
}

impl SuppressedVectorTextRuns {
    pub fn is_empty(&self) -> bool {
        self.object_ids.is_empty() && self.run_indices.is_empty()
    }

    pub fn extend(&mut self, other: SuppressedVectorTextRuns) {
        self.object_ids.extend(other.object_ids);
        self.run_indices.extend(other.run_indices);
    }

    pub fn text_suppressed_count(&self, text: &VectorTextObject) -> usize {
        text.runs
            .iter()
            .enumerate()
            .filter(|(run_index, run)| self.suppresses_run(*run_index, run))
            .count()
    }

    pub fn suppresses_run(&self, run_index: usize, run: &StyledRun) -> bool {
        self.run_indices.contains(&run_index)
            || run
                .object_id
                .as_ref()
                .map(|object_id| self.object_ids.contains(object_id))
                .unwrap_or(false)
    }
}

fn bbox_overlap_width(left: &BoundingBox, right: &BoundingBox) -> f32 {
    (left.right.min(right.right) - left.left.max(right.left)).max(0.0)
}

fn bbox_overlap_height(left: &BoundingBox, right: &BoundingBox) -> f32 {
    (left.bottom.min(right.bottom) - left.top.max(right.top)).max(0.0)
}

pub fn text_matches_region(
    run: &StyledRun,
    replacement_region: &ParagraphReplacementRegion,
) -> bool {
    let run_bbox = run_bbox(run);
    if !bbox_intersects(&run_bbox, &replacement_region.text_clear_bbox) {
        return false;
    }

    let run_width = (run_bbox.right - run_bbox.left).max(1.0);
    let run_height = (run_bbox.bottom - run_bbox.top).max(1.0);
    let overlap_width = bbox_overlap_width(&run_bbox, &replacement_region.text_clear_bbox);
    let overlap_height = bbox_overlap_height(&run_bbox, &replacement_region.text_clear_bbox);
    let horizontal_threshold = (run_width * 0.08).clamp(1.0, 6.0);
    let vertical_threshold = (run_height * 0.18).clamp(0.75, 3.0);

    overlap_width >= horizontal_threshold && overlap_height >= vertical_threshold
}

pub fn glyph_matches_region(
    run: &GlyphPaintRun,
    replacement_region: &ParagraphReplacementRegion,
) -> bool {
    let run_bbox = run.bbox;
    if !bbox_intersects(&run_bbox, &replacement_region.text_clear_bbox) {
        return false;
    }

    let run_width = (run_bbox.right - run_bbox.left).max(1.0);
    let run_height = (run_bbox.bottom - run_bbox.top).max(1.0);
    let overlap_width = bbox_overlap_width(&run_bbox, &replacement_region.text_clear_bbox);
    let overlap_height = bbox_overlap_height(&run_bbox, &replacement_region.text_clear_bbox);
    let horizontal_threshold = (run_width * 0.08).clamp(1.0, 6.0);
    let vertical_threshold = (run_height * 0.18).clamp(0.75, 3.0);

    overlap_width >= horizontal_threshold && overlap_height >= vertical_threshold
}

fn normalize_source_match_text(text: &str) -> String {
    text.chars().filter(|ch| !ch.is_whitespace()).collect()
}

#[allow(dead_code)]
pub fn text_matches_overlay(
    object: &VectorRenderObject,
    source_text: &str,
    replacement_region: &ParagraphReplacementRegion,
) -> bool {
    let VectorRenderObject::Text(text) = object else {
        return false;
    };
    let object_text = text
        .runs
        .iter()
        .map(|run| run.text.as_str())
        .collect::<String>();
    let normalized_object_text = normalize_source_match_text(&object_text);
    let normalized_source_text = normalize_source_match_text(source_text);
    if normalized_object_text.is_empty() || normalized_source_text.is_empty() {
        return false;
    }
    if normalized_object_text != normalized_source_text
        && !normalized_object_text.contains(&normalized_source_text)
        && !normalized_source_text.contains(&normalized_object_text)
    {
        return false;
    }

    text.runs
        .iter()
        .any(|run| text_matches_region(run, replacement_region))
}

pub fn glyph_matches_overlay(
    paragraph: &GlyphPaintParagraph,
    overlay: &ParagraphRenderOverlay,
) -> bool {
    let paragraph_text = paragraph
        .runs
        .iter()
        .map(|run| run.text.as_str())
        .collect::<String>();
    let normalized_paragraph_text = normalize_source_match_text(&paragraph_text);
    let normalized_source_text = normalize_source_match_text(&overlay.source_text);
    if normalized_paragraph_text.is_empty() || normalized_source_text.is_empty() {
        return false;
    }
    if normalized_paragraph_text != normalized_source_text
        && !normalized_paragraph_text.contains(&normalized_source_text)
        && !normalized_source_text.contains(&normalized_paragraph_text)
    {
        return false;
    }

    let replacement_region = build_region(&overlay.target);
    paragraph
        .runs
        .iter()
        .any(|run| glyph_matches_region(run, &replacement_region))
}

pub fn matching_text_run_refs(
    object: &VectorRenderObject,
    active_object_ids: &HashSet<String>,
    replacement_region: &ParagraphReplacementRegion,
) -> SuppressedVectorTextRuns {
    match object {
        VectorRenderObject::Text(text) => {
            let mut refs = SuppressedVectorTextRuns::default();
            for (run_index, run) in text.runs.iter().enumerate() {
                if let Some(object_id) = run.object_id.as_ref() {
                    if active_object_ids.contains(object_id) {
                        refs.object_ids.insert(object_id.clone());
                        continue;
                    }
                }
                if text_matches_region(run, replacement_region) {
                    refs.run_indices.insert(run_index);
                }
            }
            refs
        }
        _ => SuppressedVectorTextRuns::default(),
    }
}

pub fn text_object_should_be_suppressed(
    object: &VectorRenderObject,
    active_object_ids: &HashSet<String>,
) -> bool {
    let VectorRenderObject::Text(text) = object else {
        return false;
    };

    // Some extractors only provide the object id on the editor/layout side while
    // individual text runs have no id. Suppress the whole source object here so
    // edit mode never paints the original text under the replacement text.
    active_object_ids.contains(&text.id)
}
