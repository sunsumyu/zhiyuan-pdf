use std::collections::BTreeSet;

use crate::typography::font_resolver::looks_like_symbolic_font;
use crate::text::list_semantics::derive_list_text_semantics;
use crate::models::{BoundingBox, ParagraphEditContext, LayoutParagraph, LayoutRun};

use crate::geometry::source_geometry::{source_run_visual_bbox, source_visual_bbox_from_runs};

const EDIT_SEGMENT_DELIMITER: &str = "::edit-segment::";

#[derive(Debug, Clone)]
pub struct EditorEditTarget {
    pub target_id: String,
    pub base_paragraph_id: String,
    pub session: ParagraphEditContext,
    pub source_run_indices: Vec<usize>,
    pub source_object_ids: BTreeSet<String>,
}

pub fn make_edit_segment_target_id(base_paragraph_id: &str, segment_key: &str) -> String {
    format!("{base_paragraph_id}{EDIT_SEGMENT_DELIMITER}{segment_key}")
}

pub fn edit_target_base_paragraph_id(target_id: &str) -> &str {
    target_id
        .split_once(EDIT_SEGMENT_DELIMITER)
        .map(|(base, _)| base)
        .unwrap_or(target_id)
}

pub fn edit_target_segment_key(target_id: &str) -> Option<&str> {
    target_id
        .split_once(EDIT_SEGMENT_DELIMITER)
        .map(|(_, segment_key)| segment_key)
}

pub fn collect_edit_targets_from_session(
    base_paragraph_id: &str,
    session: &ParagraphEditContext,
) -> Vec<EditorEditTarget> {
    let segments = build_visual_segments(session);
    if segments.is_empty() {
        return vec![whole_session_target(base_paragraph_id, session)];
    }

    let full_run_count = session
        .paragraph
        .runs
        .iter()
        .filter(|run| !run.text.is_empty())
        .count();
    if segments.len() == 1 && segments[0].run_indices.len() == full_run_count {
        return vec![whole_session_target(base_paragraph_id, session)];
    }

    segments
        .into_iter()
        .filter_map(|segment| build_segment_target(base_paragraph_id, session, segment))
        .collect()
}

pub fn resolve_edit_target_from_session(
    base_paragraph_id: &str,
    requested_target_id: &str,
    session: &ParagraphEditContext,
    click_page_point: Option<(f32, f32)>,
) -> EditorEditTarget {
    let targets = collect_edit_targets_from_session(base_paragraph_id, session);
    if targets.is_empty() {
        return whole_session_target(base_paragraph_id, session);
    }

    if let Some(segment_key) = edit_target_segment_key(requested_target_id) {
        if let Some(target) = targets
            .iter()
            .find(|target| edit_target_segment_key(&target.target_id) == Some(segment_key))
        {
            return target.clone();
        }
    }

    if requested_target_id == base_paragraph_id && targets.len() == 1 {
        return targets[0].clone();
    }

    if let Some((click_x, click_y)) = click_page_point {
        if let Some(target) = targets.iter().min_by(|a, b| {
            let score_a = target_hit_score(&a.session.anchor_bbox, click_x, click_y);
            let score_b = target_hit_score(&b.session.anchor_bbox, click_x, click_y);
            score_a
                .partial_cmp(&score_b)
                .unwrap_or(std::cmp::Ordering::Equal)
        }) {
            return target.clone();
        }
    }

    targets
        .into_iter()
        .find(|target| target.target_id == requested_target_id)
        .unwrap_or_else(|| whole_session_target(base_paragraph_id, session))
}

#[derive(Debug, Clone)]
struct VisualSegment {
    key: String,
    run_indices: Vec<usize>,
}

type IndexedRunRef<'a> = (usize, &'a LayoutRun);

fn build_visual_segments(session: &ParagraphEditContext) -> Vec<VisualSegment> {
    let mut indexed_runs = session
        .paragraph
        .runs
        .iter()
        .enumerate()
        .filter(|(_, run)| !run.text.is_empty())
        .collect::<Vec<_>>();
    indexed_runs.sort_by(|(left_index, left): &(usize, &LayoutRun), (right_index, right)| {
        line_sort_key(left)
            .partial_cmp(&line_sort_key(right))
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                left.origin_x
                    .partial_cmp(&right.origin_x)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| left_index.cmp(right_index))
    });

    let line_groups = group_runs_by_visual_line(indexed_runs);
    let mut segments = Vec::new();
    for line_runs in line_groups {
        if line_runs.is_empty() {
            continue;
        }
        if line_is_list_like(&line_runs) {
            segments.push(visual_segment_from_indices(
                line_runs.iter().map(|(run_index, _)| *run_index).collect(),
            ));
            continue;
        }

        let mut current_indices: Vec<usize> = Vec::new();
        let mut current_right: Option<f32> = None;
        let mut previous_run: Option<&LayoutRun> = None;

        for (run_index, run) in line_runs {
            let run_bbox = source_run_visual_bbox(run).unwrap_or(run.bbox);
            let gap = current_right
                .map(|right| run_bbox.left - right)
                .unwrap_or(0.0);
            let starts_new_segment = previous_run
                .map(|previous| gap > segment_break_gap(previous, run))
                .unwrap_or(false);

            if starts_new_segment && !current_indices.is_empty() {
                segments.push(visual_segment_from_indices(std::mem::take(
                    &mut current_indices,
                )));
                current_right = None;
            }

            current_right = Some(current_right.unwrap_or(run_bbox.left).max(run_bbox.right));
            previous_run = Some(run);
            current_indices.push(run_index);
        }

        if !current_indices.is_empty() {
            segments.push(visual_segment_from_indices(current_indices));
        }
    }

    segments
}

fn group_runs_by_visual_line<'a>(
    indexed_runs: Vec<IndexedRunRef<'a>>,
) -> Vec<Vec<IndexedRunRef<'a>>> {
    let mut lines = Vec::<Vec<IndexedRunRef<'a>>>::new();
    let mut current_line: Vec<IndexedRunRef<'a>> = Vec::new();
    let mut current_line_y: Option<f32> = None;

    for (run_index, run) in indexed_runs {
        let starts_new_line = current_line_y
            .map(|line_y| !same_visual_line(line_y, run))
            .unwrap_or(false);
        if starts_new_line && !current_line.is_empty() {
            lines.push(std::mem::take(&mut current_line));
        }
        if current_line_y.is_none() || starts_new_line {
            current_line_y = Some(run.origin_y);
        }
        current_line.push((run_index, run));
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    lines
}

fn line_is_list_like(line_runs: &[IndexedRunRef<'_>]) -> bool {
    let line_text = line_runs
        .iter()
        .map(|(_, run)| run.text.as_str())
        .collect::<String>();
    if derive_list_text_semantics(&line_text).has_marker {
        return true;
    }

    line_runs
        .iter()
        .map(|(_, run)| run)
        .find(|run| !run.text.trim().is_empty())
        .map(|run| {
            let trimmed = run.text.trim_start();
            let first_char = trimmed.chars().next();
            looks_like_symbolic_font(&run.style.font_name)
                || first_char
                    .map(|ch| matches!(ch, '•' | '●' | '▪' | '◦' | '·' | '○' | '-' | '▶' | '➤'))
                    .unwrap_or(false)
        })
        .unwrap_or(false)
}

fn visual_segment_from_indices(run_indices: Vec<usize>) -> VisualSegment {
    let start = run_indices.first().copied().unwrap_or(0);
    let end = run_indices.last().copied().unwrap_or(start);
    VisualSegment {
        key: format!("r{start}-{end}"),
        run_indices,
    }
}

fn build_segment_target(
    base_paragraph_id: &str,
    session: &ParagraphEditContext,
    segment: VisualSegment,
) -> Option<EditorEditTarget> {
    let runs = segment
        .run_indices
        .iter()
        .filter_map(|index| session.paragraph.runs.get(*index).cloned())
        .collect::<Vec<_>>();
    if runs.is_empty() {
        return None;
    }

    let anchor_bbox = bbox_from_layout_runs(&runs)?;
    let mut paragraph = session.paragraph.clone();
    paragraph.id = make_edit_segment_target_id(base_paragraph_id, &segment.key);
    paragraph.runs = runs;
    normalize_paragraph_to_bbox(&mut paragraph, anchor_bbox);

    let source_object_ids = paragraph
        .runs
        .iter()
        .flat_map(|run| run.object_ids.iter().cloned())
        .collect::<BTreeSet<_>>();

    Some(EditorEditTarget {
        target_id: paragraph.id.clone(),
        base_paragraph_id: base_paragraph_id.to_string(),
        session: ParagraphEditContext {
            anchor_bbox,
            paragraph,
        },
        source_run_indices: segment.run_indices,
        source_object_ids,
    })
}

fn whole_session_target(
    base_paragraph_id: &str,
    session: &ParagraphEditContext,
) -> EditorEditTarget {
    let mut session = session.clone();
    session.paragraph.id = base_paragraph_id.to_string();
    let source_object_ids = session
        .paragraph
        .runs
        .iter()
        .flat_map(|run| run.object_ids.iter().cloned())
        .collect::<BTreeSet<_>>();
    EditorEditTarget {
        target_id: base_paragraph_id.to_string(),
        base_paragraph_id: base_paragraph_id.to_string(),
        source_run_indices: (0..session.paragraph.runs.len()).collect(),
        source_object_ids,
        session,
    }
}

fn normalize_paragraph_to_bbox(paragraph: &mut LayoutParagraph, bbox: BoundingBox) {
    paragraph.bbox = bbox;
    paragraph.origin_x = bbox.left;
    paragraph.origin_y = bbox.top;
    paragraph.wrap_width = (bbox.right - bbox.left).max(1.0);
}

fn bbox_from_layout_runs(runs: &[LayoutRun]) -> Option<BoundingBox> {
    if let Some(source_bbox) = source_visual_bbox_from_runs(runs) {
        return Some(source_bbox);
    }
    let first = runs.first()?;
    let mut bbox = first.bbox;
    for run in runs.iter().skip(1) {
        bbox.left = bbox.left.min(run.bbox.left);
        bbox.top = bbox.top.min(run.bbox.top);
        bbox.right = bbox.right.max(run.bbox.right);
        bbox.bottom = bbox.bottom.max(run.bbox.bottom);
    }
    Some(bbox)
}

fn line_sort_key(run: &LayoutRun) -> f32 {
    let tolerance = (run.style.font_size * 0.5).max(2.0);
    (run.origin_y / tolerance).round() * tolerance
}

fn same_visual_line(reference_origin_y: f32, run: &LayoutRun) -> bool {
    let tolerance = (run.style.font_size * 0.55).max(2.0);
    (reference_origin_y - run.origin_y).abs() <= tolerance
}

fn segment_break_gap(previous: &LayoutRun, next: &LayoutRun) -> f32 {
    let previous_char_count = previous.text.chars().count().max(1) as f32;
    let previous_bbox = source_run_visual_bbox(previous).unwrap_or(previous.bbox);
    let previous_width = (previous_bbox.right - previous_bbox.left).max(0.0);
    let previous_avg = (previous_width / previous_char_count).max(1.0);
    let font_size = previous.style.font_size.max(next.style.font_size).max(1.0);
    font_size
        .mul_add(3.0, 0.0)
        .max(previous_avg * 4.0)
        .max(24.0)
}

fn target_hit_score(bbox: &BoundingBox, click_x: f32, click_y: f32) -> f32 {
    let vertical_padding = ((bbox.bottom - bbox.top).abs() * 0.8).max(4.0);
    let horizontal_padding = 6.0;
    let dx = if click_x < bbox.left - horizontal_padding {
        bbox.left - horizontal_padding - click_x
    } else if click_x > bbox.right + horizontal_padding {
        click_x - bbox.right - horizontal_padding
    } else {
        0.0
    };
    let dy = if click_y < bbox.top - vertical_padding {
        bbox.top - vertical_padding - click_y
    } else if click_y > bbox.bottom + vertical_padding {
        click_y - bbox.bottom - vertical_padding
    } else {
        0.0
    };
    dy * 1000.0 + dx
}

#[cfg(test)]
mod tests {
    use super::collect_edit_targets_from_session;
    use crate::models::{
        BoundingBox, ParagraphEditContext, LayoutParagraph, LayoutRun, RunStyle,
    };

    fn test_run(id: &str, text: &str, left: f32, baseline_y: f32) -> LayoutRun {
        let width = text.chars().count() as f32 * 6.0;
        LayoutRun {
            id: id.to_string(),
            text: text.to_string(),
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
                left,
                top: baseline_y,
                right: left + width,
                bottom: baseline_y + 12.0,
            },
            origin_x: left,
            origin_y: baseline_y,
            char_origins: Vec::new(),
            char_widths: Vec::new(),
            object_ids: Vec::new(),
            object_indices: Vec::new(),
        }
    }

    #[test]
    fn segmented_targets_use_baseline_font_visual_bbox() {
        let session = ParagraphEditContext {
            anchor_bbox: BoundingBox {
                left: 0.0,
                top: 100.0,
                right: 400.0,
                bottom: 112.0,
            },
            paragraph: LayoutParagraph {
                id: "p1".to_string(),
                runs: vec![
                    test_run("r0", "编程语言:", 60.0, 112.0),
                    test_run("r1", "Rust", 220.0, 112.0),
                ],
                ..LayoutParagraph::default()
            },
        };

        let targets = collect_edit_targets_from_session("p1", &session);

        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].session.anchor_bbox.top, 100.0);
        assert_eq!(targets[0].session.anchor_bbox.bottom, 112.0);
        assert_eq!(targets[1].session.anchor_bbox.top, 100.0);
        assert_eq!(targets[1].session.anchor_bbox.bottom, 112.0);
    }
}
