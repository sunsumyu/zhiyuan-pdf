//! 行级上下文 delta + build_editor_session_text_plan（微排主循环）。

use super::glyph_geometry::{
    glyph_left, glyph_right, same_visual_line,
    should_insert_internal_gap_space, should_insert_visual_gap_space,
};
use super::text_predicates::{
    is_decorative_text, should_allow_synthetic_gap, should_insert_gap_from_origin_delta,
};
use super::{EditorGlyphSlot, EditorGlyphSlotKind, EditorSessionTextPlan};
use crate::models::{LayoutRun, ParagraphEditContext};

pub(crate) fn line_contextual_run_delta(runs: &[&LayoutRun], run_index: usize) -> Option<f32> {
    let target = runs.get(run_index)?;
    let mut deltas = Vec::new();

    for pair in runs.windows(2) {
        let [left, right] = pair else { continue };
        if !same_visual_line(left, right) {
            continue;
        }
        let delta = right.origin_x - left.origin_x;
        if delta.is_finite() && delta > 0.0 {
            deltas.push(delta);
        }
    }

    if deltas.is_empty() {
        let prev_delta = run_index
            .checked_sub(1)
            .and_then(|idx| runs.get(idx))
            .filter(|prev| same_visual_line(prev, target))
            .map(|prev| target.origin_x - prev.origin_x);
        let next_delta = runs
            .get(run_index + 1)
            .filter(|next| same_visual_line(target, next))
            .map(|next| next.origin_x - target.origin_x);
        return prev_delta
            .or(next_delta)
            .filter(|delta| delta.is_finite() && *delta > 0.0);
    }

    deltas.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let index = ((deltas.len().saturating_sub(1)) as f32 * 0.35).round() as usize;
    Some(deltas[index.min(deltas.len() - 1)].max(1.0))
}

pub(crate) fn needs_gap(
    prev: &LayoutRun,
    next: &LayoutRun,
    line_typical_delta: Option<f32>,
) -> bool {
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

    let prev_char_count = prev.text.chars().count();
    let next_char_count = next.text.chars().count();
    if prev_char_count == 1 && next_char_count == 1 {
        let origin_delta = next.origin_x - prev.origin_x;
        if !origin_delta.is_finite() || origin_delta <= 0.0 {
            return false;
        }
        let contextual_reference = line_typical_delta
            .map(|delta| delta.max(1.0))
            .unwrap_or_else(|| prev.style.font_size.max(next.style.font_size).max(1.0));
        return should_insert_gap_from_origin_delta(
            prev_last,
            next_first,
            origin_delta,
            contextual_reference,
        );
    }

    should_insert_visual_gap_space(prev, next)
}

pub fn build_editor_session_text_plan(session: &ParagraphEditContext) -> EditorSessionTextPlan {
    let ordered_runs: Vec<&LayoutRun> = session
        .paragraph
        .runs
        .iter()
        .filter(|run| !run.text.is_empty())
        .collect();
    let mut text = String::new();
    let mut slots = Vec::new();
    let raw_char_capacity = ordered_runs
        .iter()
        .map(|run| run.text.chars().count())
        .sum::<usize>();
    let mut raw_to_reconstructed = Vec::with_capacity(raw_char_capacity + 1);
    raw_to_reconstructed.push(0);
    let mut reconstructed_to_raw = Vec::new();
    reconstructed_to_raw.push(0);
    let mut raw_count = 0usize;
    let mut prev: Option<&LayoutRun> = None;
    let mut prev_right: Option<f32> = None;

    for (run_index, run) in ordered_runs.iter().enumerate() {
        if let Some(prev_run) = prev {
            let line_typical_delta = line_contextual_run_delta(&ordered_runs, run_index);
            if needs_gap(prev_run, run, line_typical_delta) {
                let gap_left = prev_right.unwrap_or(prev_run.bbox.right);
                let gap_right = run.bbox.left.max(gap_left);
                slots.push(EditorGlyphSlot {
                    kind: EditorGlyphSlotKind::Gap,
                    ch: ' ',
                    raw_char_index: None,
                    left: gap_left - session.anchor_bbox.left,
                    right: gap_right - session.anchor_bbox.left,
                });
                text.push(' ');
                reconstructed_to_raw.push(raw_count);
                if let Some(last) = raw_to_reconstructed.last_mut() {
                    *last = text.chars().count();
                }
            }
        }
        let chars: Vec<char> = run.text.chars().collect();
        if chars.is_empty() {
            prev = Some(run);
            continue;
        }

        if run.char_origins.len() < 2 {
            let glyph_count = chars.len();
            for (glyph_index, ch) in chars.into_iter().enumerate() {
                let left = if glyph_count == 1 {
                    run.bbox.left
                } else {
                    glyph_left(run, glyph_index)
                };
                let right = if glyph_count == 1 {
                    run.bbox.right
                } else {
                    glyph_right(run, glyph_index, glyph_count)
                };
                slots.push(EditorGlyphSlot {
                    kind: EditorGlyphSlotKind::Glyph,
                    ch,
                    raw_char_index: Some(raw_count),
                    left: left - session.anchor_bbox.left,
                    right: right - session.anchor_bbox.left,
                });
                text.push(ch);
                raw_count += 1;
                raw_to_reconstructed.push(text.chars().count());
                reconstructed_to_raw.push(raw_count);
                prev_right = Some(right);
            }
            prev = Some(run);
            continue;
        }

        for (index, ch) in chars.iter().enumerate() {
            let glyph_count = chars.len();
            let left = glyph_left(run, index);
            let right = glyph_right(run, index, glyph_count);
            slots.push(EditorGlyphSlot {
                kind: EditorGlyphSlotKind::Glyph,
                ch: *ch,
                raw_char_index: Some(raw_count),
                left: left - session.anchor_bbox.left,
                right: right - session.anchor_bbox.left,
            });
            text.push(*ch);
            raw_count += 1;
            raw_to_reconstructed.push(text.chars().count());
            reconstructed_to_raw.push(raw_count);
            if should_insert_internal_gap_space(run, index, &chars) {
                let next_left = glyph_left(run, index + 1);
                slots.push(EditorGlyphSlot {
                    kind: EditorGlyphSlotKind::Gap,
                    ch: ' ',
                    raw_char_index: None,
                    left: right - session.anchor_bbox.left,
                    right: next_left - session.anchor_bbox.left,
                });
                text.push(' ');
                reconstructed_to_raw.push(raw_count);
                if let Some(last) = raw_to_reconstructed.last_mut() {
                    *last = text.chars().count();
                }
            }
            prev_right = Some(right);
        }
        prev = Some(run);
    }

    EditorSessionTextPlan {
        text,
        slots,
        raw_to_reconstructed,
        reconstructed_to_raw,
    }
}

#[allow(dead_code)]
pub fn has_suspicious_run_geometry<F, G>(
    session: &ParagraphEditContext,
    is_symbol_font: F,
    measure_run_width: G,
) -> bool
where
    F: Fn(&str) -> bool,
    G: Fn(&LayoutRun) -> f32,
{
    session.paragraph.runs.iter().any(|run| {
        if is_decorative_text(&run.text) || is_symbol_font(&run.style.font_name) {
            return false;
        }
        let bbox_width = (run.bbox.right - run.bbox.left).abs().max(1.0);
        let measured_width = measure_run_width(run).max(1.0);
        bbox_width > measured_width * 3.0 || bbox_width < measured_width * 0.4
    })
}
