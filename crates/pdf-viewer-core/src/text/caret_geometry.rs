//! 纯计算光标几何 — 从 ui::editor::format::text_geometry 拆分而来。
//! 不依赖 wasm_bindgen / web_sys / canvas。
//! 含 wasm 平台依赖的函数（canvas.measureText、ActiveEditorTarget 相关）仍留在 ui 侧。

use crate::text::glyph_layout::{build_editor_session_text_plan, EditorSessionTextPlan};
use crate::models::EditorSession;

#[derive(Debug, Clone, Copy)]
pub struct EditorCaretVisualPosition {
    pub left: f32,
    pub baseline_y: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct CaretStop {
    pub index: usize,
    pub left: f32,
}

#[derive(Debug, Clone, Default)]
pub struct CaretLine {
    pub baseline_y: f32,
    pub height: f32,
    pub stops: Vec<CaretStop>,
}

pub fn same_existing_session_line(
    reference_baseline_y: f32,
    run: &crate::models::LayoutRun,
    anchor_top: f32,
) -> bool {
    let baseline_y = (run.origin_y - anchor_top).max(0.0);
    let tolerance = (run.style.font_size * 0.45).max(2.0);
    (reference_baseline_y - baseline_y).abs() <= tolerance
}

pub fn resolve_caret_index_from_lines(
    lines: &[CaretLine],
    local_click_x: f32,
    local_click_y: f32,
) -> usize {
    let mut best_index = 0usize;
    let mut best_score = f32::INFINITY;
    for line in lines {
        for stop in &line.stops {
            let dx = local_click_x - stop.left;
            let dy = local_click_y - line.baseline_y;
            let score = (dx * dx) + (dy * dy * 4.0);
            if score < best_score {
                best_score = score;
                best_index = stop.index;
            }
        }
    }
    best_index
}

pub fn dedupe_caret_stops(line: &mut CaretLine) {
    line.stops
        .dedup_by(|a, b| a.index == b.index && (a.left - b.left).abs() <= 0.5);
}

pub fn caret_index_at_page_point(session: &EditorSession, page_x: f32, page_y: f32) -> usize {
    let text_plan = build_editor_session_text_plan(session);
    caret_index_at_page_point_with_plan(session, &text_plan, page_x, page_y)
}

pub fn caret_index_at_page_point_with_plan(
    session: &EditorSession,
    text_plan: &EditorSessionTextPlan,
    page_x: f32,
    page_y: f32,
) -> usize {
    let lines = build_session_caret_lines(session, text_plan, 1.0);
    let local_click_x = page_x - session.anchor_bbox.left;
    let local_click_y = page_y - session.anchor_bbox.top;
    resolve_caret_index_from_lines(&lines, local_click_x, local_click_y)
}

pub fn caret_visual_for_session(
    session: &EditorSession,
    caret_index: usize,
    fallback_height: f32,
) -> EditorCaretVisualPosition {
    let text_plan = build_editor_session_text_plan(session);
    caret_visual_for_session_plan(session, &text_plan, caret_index, fallback_height)
}

pub fn caret_visual_for_session_plan(
    session: &EditorSession,
    text_plan: &EditorSessionTextPlan,
    caret_index: usize,
    fallback_height: f32,
) -> EditorCaretVisualPosition {
    let lines = build_session_caret_lines(session, text_plan, fallback_height);
    for line in &lines {
        if let Some(stop) = line.stops.iter().find(|stop| stop.index == caret_index) {
            return EditorCaretVisualPosition {
                left: stop.left,
                baseline_y: line.baseline_y,
                height: line.height.max(fallback_height),
            };
        }
    }

    lines
        .last()
        .and_then(|line| line.stops.last().map(|stop| (line, stop)))
        .map(|(line, stop)| EditorCaretVisualPosition {
            left: stop.left,
            baseline_y: line.baseline_y,
            height: line.height.max(fallback_height),
        })
        .unwrap_or(EditorCaretVisualPosition {
            left: 0.0,
            baseline_y: 0.0,
            height: fallback_height.max(1.0),
        })
}

pub fn build_session_caret_lines(
    session: &EditorSession,
    text_plan: &EditorSessionTextPlan,
    fallback_height: f32,
) -> Vec<CaretLine> {
    let mut lines: Vec<CaretLine> = Vec::new();
    let mut current_line_raw_start = 0usize;
    let mut current_line_raw_end = 0usize;
    for run in session
        .paragraph
        .runs
        .iter()
        .filter(|run| !run.text.is_empty())
    {
        let glyph_count = run.text.chars().count();
        let baseline_y = (run.origin_y - session.anchor_bbox.top).max(0.0);
        let run_height = run.style.font_size.max(fallback_height).max(1.0);
        if let Some(line) = lines.last_mut().filter(|line| {
            same_existing_session_line(line.baseline_y, run, session.anchor_bbox.top)
        }) {
            line.height = line.height.max(run_height);
            current_line_raw_end += glyph_count;
        } else {
            if let Some(previous_line) = lines.last_mut() {
                populate_line_stops_from_text_plan(
                    previous_line,
                    text_plan,
                    current_line_raw_start,
                    current_line_raw_end,
                );
                dedupe_caret_stops(previous_line);
            }

            lines.push(CaretLine {
                baseline_y,
                height: run_height,
                stops: Vec::new(),
            });
            current_line_raw_start = current_line_raw_end;
            current_line_raw_end = current_line_raw_start + glyph_count;
        }
    }
    if let Some(last_line) = lines.last_mut() {
        populate_line_stops_from_text_plan(
            last_line,
            text_plan,
            current_line_raw_start,
            current_line_raw_end,
        );
        dedupe_caret_stops(last_line);
    }
    lines.sort_by(|a, b| {
        a.baseline_y
            .partial_cmp(&b.baseline_y)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    lines
}

pub fn populate_line_stops_from_text_plan(
    line: &mut CaretLine,
    text_plan: &EditorSessionTextPlan,
    raw_start: usize,
    raw_end: usize,
) {
    if raw_start >= raw_end {
        return;
    }
    let mut glyph_slots = text_plan.slots.iter().filter_map(|slot| {
        slot.raw_char_index
            .filter(|index| *index >= raw_start && *index < raw_end)
            .map(|index| (index, slot))
    });
    let Some((first_raw_index, first_slot)) = glyph_slots.next() else {
        return;
    };

    line.stops.push(CaretStop {
        index: first_raw_index,
        left: first_slot.left,
    });

    line.stops.push(CaretStop {
        index: first_raw_index + 1,
        left: first_slot.right,
    });

    for (raw_index, slot) in glyph_slots {
        line.stops.push(CaretStop {
            index: raw_index + 1,
            left: slot.right,
        });
    }
}

pub fn resolve_navigation_from_lines(
    lines: &[CaretLine],
    caret_index: usize,
    key: &str,
) -> Option<usize> {
    let current_line_index = lines
        .iter()
        .position(|line| line.stops.iter().any(|stop| stop.index == caret_index))?;
    let current_line = &lines[current_line_index];
    let current_left = current_line
        .stops
        .iter()
        .find(|stop| stop.index == caret_index)
        .or_else(|| current_line.stops.last())
        .map(|stop| stop.left)
        .unwrap_or(0.0);

    match key {
        "Home" => current_line.stops.first().map(|stop| stop.index),
        "End" => current_line.stops.last().map(|stop| stop.index),
        "ArrowUp" => lines
            .get(current_line_index.saturating_sub(1))
            .and_then(|line| {
                line.stops
                    .iter()
                    .min_by(|a, b| {
                        (a.left - current_left)
                            .abs()
                            .partial_cmp(&(b.left - current_left).abs())
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|stop| stop.index)
            }),
        "ArrowDown" => lines.get(current_line_index + 1).and_then(|line| {
            line.stops
                .iter()
                .min_by(|a, b| {
                    (a.left - current_left)
                        .abs()
                        .partial_cmp(&(b.left - current_left).abs())
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|stop| stop.index)
        }),
        _ => None,
    }
}
