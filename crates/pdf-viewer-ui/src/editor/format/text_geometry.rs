use pdf_viewer_core::glyph_layout::{build_editor_session_text_plan, EditorSessionTextPlan};
use pdf_viewer_core::models::{EditorSession, LayoutRun};
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

use crate::editor::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use crate::editor::draft_layout::build_draft_render_plan;
use crate::editor::session::ActiveEditorTarget;

#[derive(Debug, Clone, Copy)]
pub struct EditorCaretVisualPosition {
    pub left: f32,
    pub baseline_y: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Copy)]
struct CaretStop {
    index: usize,
    left: f32,
}

#[derive(Debug, Clone, Default)]
struct CaretLine {
    baseline_y: f32,
    height: f32,
    stops: Vec<CaretStop>,
}

pub fn measure_editor_layout_text_width(
    ctx: &CanvasRenderingContext2d,
    text: &str,
    run: &LayoutRun,
) -> f32 {
    let font_weight = if run.style.is_bold { "bold" } else { "normal" };
    let font_style = if run.style.is_italic {
        "italic"
    } else {
        "normal"
    };
    ctx.set_font(&format!(
        "{} {} {}px {}",
        font_style, font_weight, run.style.font_size, run.style.font_name
    ));
    let measured_width = ctx
        .measure_text(text)
        .ok()
        .map(|metrics| metrics.width() as f32)
        .unwrap_or(0.0);

    if !run.char_origins.is_empty() {
        let last_index = run.char_origins.len().saturating_sub(1);
        let last_origin = run.char_origins.get(last_index).copied().unwrap_or(0.0);
        let last_width = if run.char_widths.len() > last_index {
            run.char_widths[last_index]
        } else if last_index > 0 {
            run.char_origins[last_index] - run.char_origins[last_index - 1]
        } else {
            measured_width / (run.char_origins.len().max(1) as f32)
        };
        return (last_origin + last_width).max(measured_width).max(1.0);
    }

    let spacing = run.style.char_spacing.max(0.0) * text.chars().count().saturating_sub(1) as f32;
    (measured_width * run.style.scale_x.max(0.01)) + spacing
}

fn create_measure_context() -> Option<CanvasRenderingContext2d> {
    let window = web_sys::window()?;
    let document = window.document()?;
    let canvas = document
        .create_element("canvas")
        .ok()?
        .dyn_into::<HtmlCanvasElement>()
        .ok()?;
    canvas
        .get_context("2d")
        .ok()??
        .dyn_into::<CanvasRenderingContext2d>()
        .ok()
}

fn same_existing_session_line(
    reference_baseline_y: f32,
    run: &LayoutRun,
    anchor_top: f32,
) -> bool {
    let baseline_y = (run.origin_y - anchor_top).max(0.0);
    let tolerance = (run.style.font_size * 0.45).max(2.0);
    (reference_baseline_y - baseline_y).abs() <= tolerance
}

fn resolve_caret_index_from_lines(
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

fn dedupe_caret_stops(line: &mut CaretLine) {
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

fn build_session_caret_lines(
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

fn populate_line_stops_from_text_plan(
    line: &mut CaretLine,
    text_plan: &pdf_viewer_core::glyph_layout::EditorSessionTextPlan,
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

fn convert_render_plan_caret_lines(
    render_plan: crate::editor::draft_layout::EditorDraftRenderPlan,
) -> Vec<CaretLine> {
    let mut lines = Vec::new();
    for line in render_plan.caret_lines {
        let mut caret_line = CaretLine {
            baseline_y: line.baseline_y,
            height: line.height,
            stops: Vec::new(),
        };
        for stop in line.stops {
            caret_line.stops.push(CaretStop {
                index: stop.index,
                left: stop.left,
            });
        }
        dedupe_caret_stops(&mut caret_line);
        lines.push(caret_line);
    }
    lines
}

fn build_unified_draft_caret_lines(
    active_target: &ActiveEditorTarget,
    draft_text: &str,
) -> Option<Vec<CaretLine>> {
    let document_plan = &active_target.scene.document_plan;
    let ctx = create_measure_context()?;
    let render_plan = build_draft_render_plan(document_plan, draft_text, |text, run| {
        measure_editor_layout_text_width(&ctx, text, run)
    });
    Some(convert_render_plan_caret_lines(render_plan))
}

fn resolve_caret_index_for_draft_point(
    active_target: &ActiveEditorTarget,
    draft_text: &str,
    page_x: f32,
    page_y: f32,
) -> usize {
    let Some(lines) = build_unified_draft_caret_lines(active_target, draft_text) else {
        let resolved = caret_index_at_page_point_with_plan(
            &active_target.scene.body_session,
            &active_target.scene.document_plan.body_text_plan,
            page_x,
            page_y,
        );
        dbg_event(
            "caret.resolve",
            "fallback-source-text-plan-no-measure-context",
            vec![
                dbg_field("paragraphId", &active_target.paragraph_id),
                dbg_field("pageX", page_x),
                dbg_field("pageY", page_y),
                dbg_field("draftText", draft_text),
                dbg_field("resolvedCaretIndex", resolved),
            ],
        );
        return resolved;
    };

    let session = &active_target.scene.document_plan.body_session;
    let local_click_x = (page_x - session.anchor_bbox.left).max(0.0);
    let local_click_y = (page_y - session.anchor_bbox.top).max(0.0);
    let resolved = resolve_caret_index_from_lines(&lines, local_click_x, local_click_y)
        .min(draft_text.chars().count());
    dbg_event(
        "caret.resolve",
        "unified-draft-point",
        vec![
            dbg_field("paragraphId", &active_target.paragraph_id),
            dbg_field("pageX", page_x),
            dbg_field("pageY", page_y),
            dbg_field("localClickX", local_click_x),
            dbg_field("localClickY", local_click_y),
            dbg_field("draftText", draft_text),
            dbg_field("resolvedCaretIndex", resolved),
            dbg_field("caretLineCount", lines.len()),
            dbg_field(
                "caretStopCount",
                lines.iter().map(|line| line.stops.len()).sum::<usize>(),
            ),
        ],
    );
    resolved
}

fn build_draft_caret_lines(
    active_target: &ActiveEditorTarget,
    draft_text: &str,
) -> Option<Vec<CaretLine>> {
    build_unified_draft_caret_lines(active_target, draft_text)
}

fn resolve_caret_visual_from_draft(
    active_target: &ActiveEditorTarget,
    draft_text: &str,
    caret_index: usize,
) -> EditorCaretVisualPosition {
    let Some(lines) = build_draft_caret_lines(active_target, draft_text) else {
        return caret_visual_for_session_plan(
            &active_target.scene.body_session,
            &active_target.scene.document_plan.body_text_plan,
            caret_index,
            active_target.font_size.max(1.0),
        );
    };
    for line in &lines {
        if let Some(stop) = line.stops.iter().find(|stop| stop.index == caret_index) {
            return EditorCaretVisualPosition {
                left: stop.left,
                baseline_y: line.baseline_y,
                height: line.height.max(active_target.font_size.max(1.0)),
            };
        }
    }
    lines
        .last()
        .and_then(|line| line.stops.last().map(|stop| (line, stop)))
        .map(|(line, stop)| EditorCaretVisualPosition {
            left: stop.left,
            baseline_y: line.baseline_y,
            height: line.height.max(active_target.font_size.max(1.0)),
        })
        .unwrap_or(EditorCaretVisualPosition {
            left: 0.0,
            baseline_y: 0.0,
            height: active_target.font_size.max(1.0),
        })
}

pub fn active_caret_visual(
    active_target: &ActiveEditorTarget,
    draft_text: &str,
    caret_index: usize,
) -> EditorCaretVisualPosition {
    resolve_caret_visual_from_draft(active_target, draft_text, caret_index)
}

fn resolve_navigation_from_lines(
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

pub fn move_caret_by_key(
    active_target: &ActiveEditorTarget,
    draft_text: &str,
    caret_index: usize,
    key: &str,
) -> Option<usize> {
    let char_count = draft_text.chars().count();
    match key {
        "ArrowLeft" => Some(caret_index.saturating_sub(1)),
        "ArrowRight" => Some((caret_index + 1).min(char_count)),
        "Home" | "End" | "ArrowUp" | "ArrowDown" => {
            let lines = build_draft_caret_lines(active_target, draft_text)?;
            resolve_navigation_from_lines(&lines, caret_index, key)
        }
        _ => None,
    }
}

pub fn active_caret_index_at_page_point(
    active_target: &ActiveEditorTarget,
    draft_text: &str,
    page_x: f32,
    page_y: f32,
) -> usize {
    let body_left = active_target.scene.body_session.anchor_bbox.left;
    if page_x <= body_left {
        dbg_event(
            "caret.resolve",
            "before-body-left",
            vec![
                dbg_field("paragraphId", &active_target.paragraph_id),
                dbg_field("pageX", page_x),
                dbg_field("pageY", page_y),
                dbg_field("bodyLeft", body_left),
                dbg_field("resolvedCaretIndex", 0),
            ],
        );
        return 0;
    }

    resolve_caret_index_for_draft_point(active_target, draft_text, page_x, page_y)
}

pub fn active_caret_index_at_shell_point(
    active_target: &ActiveEditorTarget,
    draft_text: &str,
    shell_x: f32,
    shell_y: f32,
) -> usize {
    let shell_left = active_target.scene.shell_bbox.left;
    let shell_top = active_target.scene.shell_bbox.top;
    let body_left = active_target.scene.body_session.anchor_bbox.left;
    let body_top = active_target.scene.body_session.anchor_bbox.top;
    let body_offset_x = (body_left - shell_left).max(0.0);
    let body_offset_y = (body_top - shell_top).max(0.0);
    let page_x = body_left + (shell_x - body_offset_x).max(0.0);
    let page_y = body_top + (shell_y - body_offset_y).max(0.0);
    dbg_event(
        "caret.resolve",
        "shell-to-page",
        vec![
            dbg_field("paragraphId", &active_target.paragraph_id),
            dbg_field("shellX", shell_x),
            dbg_field("shellY", shell_y),
            dbg_field("bodyOffsetX", body_offset_x),
            dbg_field("bodyOffsetY", body_offset_y),
            dbg_field("pageX", page_x),
            dbg_field("pageY", page_y),
        ],
    );
    active_caret_index_at_page_point(active_target, draft_text, page_x, page_y)
}
