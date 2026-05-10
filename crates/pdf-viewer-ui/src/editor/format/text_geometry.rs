//! 编辑器光标几何 — 仍涉及 WASM canvas 测量与 ActiveEditorTarget 的部分留在 ui 侧。
//! 纯计算函数（光标行/导航/索引解析）已迁至 pdf_viewer_core::text::caret_geometry。

use pdf_viewer_core::models::LayoutRun;
// 重新导出 core 提供的纯计算 API，保持 ui 内的旧调用路径不变。
pub use pdf_viewer_core::text::caret_geometry::{
    caret_index_at_page_point, caret_index_at_page_point_with_plan, caret_visual_for_session,
    caret_visual_for_session_plan, resolve_caret_index_from_lines, resolve_navigation_from_lines,
    CaretLine, CaretStop, EditorCaretVisualPosition,
};

use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

use crate::editor::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use crate::editor::draft_layout::build_draft_render_plan;
use crate::editor::session::ActiveEditorTarget;

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

fn dedupe_caret_stops_local(line: &mut CaretLine) {
    line.stops
        .dedup_by(|a, b| a.index == b.index && (a.left - b.left).abs() <= 0.5);
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
        dedupe_caret_stops_local(&mut caret_line);
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
