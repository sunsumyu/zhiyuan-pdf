//! glyph 左右物理边界、视觉宽度、同行判定、典型 advance、间隙插入判定。

use super::caret::infer_run_advance;
use super::text_predicates::{should_allow_synthetic_gap, should_insert_gap_from_origin_delta};
use crate::models::LayoutRun;

pub(crate) fn glyph_left(run: &LayoutRun, glyph_index: usize) -> f32 {
    run.origin_x
        + run
            .char_origins
            .get(glyph_index)
            .copied()
            .unwrap_or_else(|| infer_run_advance(run) * glyph_index as f32)
}

pub(crate) fn glyph_right(run: &LayoutRun, glyph_index: usize, glyph_count: usize) -> f32 {
    if glyph_index + 1 < run.char_origins.len() {
        return run.origin_x + run.char_origins[glyph_index + 1];
    }
    if let Some(width) = run
        .char_widths
        .get(glyph_index)
        .copied()
        .filter(|value| value.is_finite() && *value > 0.0)
    {
        return glyph_left(run, glyph_index) + width;
    }
    if glyph_count == 1 && run.bbox.right > run.bbox.left {
        return run.bbox.right;
    }
    glyph_left(run, glyph_index) + typical_contiguous_advance(run)
}

pub(crate) fn glyph_visual_width(run: &LayoutRun, glyph_index: usize, glyph_count: usize) -> f32 {
    (glyph_right(run, glyph_index, glyph_count) - glyph_left(run, glyph_index)).max(1.0)
}

pub(crate) fn same_visual_line(prev: &LayoutRun, next: &LayoutRun) -> bool {
    let tolerance = (prev.style.font_size.max(next.style.font_size) * 0.45).max(2.0);
    (prev.origin_y - next.origin_y).abs() <= tolerance
}

pub(crate) fn typical_contiguous_advance(run: &LayoutRun) -> f32 {
    let mut deltas: Vec<f32> = run
        .char_origins
        .windows(2)
        .map(|pair| pair[1] - pair[0])
        .filter(|delta| delta.is_finite() && *delta > 0.0)
        .collect();
    if deltas.is_empty() {
        return infer_run_advance(run).max(1.0);
    }

    deltas.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    // Use the lower-middle advance so large PDF-positioned word gaps do not become the baseline.
    let index = ((deltas.len().saturating_sub(1)) as f32 * 0.35).round() as usize;
    deltas[index.min(deltas.len() - 1)].max(1.0)
}

pub(crate) fn should_insert_internal_gap_space(
    run: &LayoutRun,
    glyph_index: usize,
    chars: &[char],
) -> bool {
    if glyph_index + 1 >= chars.len() || glyph_index + 1 >= run.char_origins.len() {
        return false;
    }

    let current = chars[glyph_index];
    let next = chars[glyph_index + 1];
    if current.is_whitespace() || next.is_whitespace() {
        return false;
    }
    if !should_allow_synthetic_gap(current, next) {
        return false;
    }

    let next_left = glyph_left(run, glyph_index + 1);
    let current_left = glyph_left(run, glyph_index);
    let origin_delta = next_left - current_left;
    if !origin_delta.is_finite() || origin_delta <= 0.0 {
        return false;
    }

    let typical_advance = typical_contiguous_advance(run);
    should_insert_gap_from_origin_delta(current, next, origin_delta, typical_advance)
}

pub(crate) fn should_insert_visual_gap_space(prev: &LayoutRun, next: &LayoutRun) -> bool {
    if !same_visual_line(prev, next) {
        return false;
    }

    let prev_text = prev.text.trim_end();
    let next_text = next.text.trim_start();
    if prev_text.is_empty() || next_text.is_empty() {
        return false;
    }
    let prev_last = prev_text.chars().last().unwrap_or(' ');
    let next_first = next_text.chars().next().unwrap_or(' ');
    if prev_last.is_whitespace() || next_first.is_whitespace() {
        return false;
    }
    if !should_allow_synthetic_gap(prev_last, next_first) {
        return false;
    }

    let prev_glyph_count = prev.text.chars().count().max(1);
    let next_glyph_count = next.text.chars().count().max(1);
    let prev_right = glyph_right(prev, prev_glyph_count.saturating_sub(1), prev_glyph_count);
    let next_left = glyph_left(next, 0);
    let geometric_gap = next_left - prev_right;
    if !geometric_gap.is_finite() || geometric_gap <= 0.0 {
        return false;
    }
    let prev_width = glyph_visual_width(prev, prev_glyph_count.saturating_sub(1), prev_glyph_count);
    let next_width = glyph_visual_width(next, 0, next_glyph_count);
    let reference_width = prev_width.min(next_width).max(1.0);
    let contiguous_join_gap = (prev.style.font_size * 0.08)
        .max(reference_width * 0.18)
        .max(0.9);
    geometric_gap > contiguous_join_gap
}
