//! Draft reflow — 核心重排计算子模块。

use crate::common::debug::truncate_debug_text;
use crate::edit::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use crate::edit::document_plan::{EditContext, ParagraphEditorMarker};
use crate::geometry::layout_engine::{layout_paragraph, VisualLine};
use crate::models::{LayoutParagraph, LayoutRun};

use super::draft_geometry::{align_layout_baseline, build_editor_draft_caret_plan_from_layout};
use super::draft_init::build_empty_render_plan;
use super::draft_style::{
    build_draft_paragraph_with_policy, build_source_layout, paragraph_preserve_underline,
    source_baseline_y,
};
use super::draft_text_diff::{body_runs_match_source_text, remap_caret_indices_to_draft_space};
use super::draft_types::EditorDraftRenderPlan;

fn summarize_render_plan_lines(plan: &EditorDraftRenderPlan) -> String {
    plan.layout
        .lines
        .iter()
        .take(4)
        .enumerate()
        .map(|(line_index, line)| {
            let runs = line
                .runs
                .iter()
                .take(8)
                .enumerate()
                .map(|(run_index, run)| {
                    let first_origin = run.char_origins.first().copied().unwrap_or(f32::NAN);
                    let last_origin = run.char_origins.last().copied().unwrap_or(f32::NAN);
                    format!(
                        "r{run_index}('{}' x={:.2} origins={} first={:.2} last={:.2} font='{}')",
                        truncate_debug_text(&run.text, 18),
                        run.origin_x,
                        run.char_origins.len(),
                        first_origin,
                        last_origin,
                        truncate_debug_text(&run.style.font_name, 18),
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "line{line_index}(base={:.2}, off={:.2}, width={:.2}, text='{}', {runs})",
                line.baseline_y,
                line.offset_x,
                line.width,
                truncate_debug_text(&line.text, 40),
            )
        })
        .collect::<Vec<_>>()
        .join(" || ")
}

pub(super) fn build_draft_paragraph(
    document_plan: &EditContext,
    draft_text: &str,
    measure_width: &dyn Fn(&str, &LayoutRun) -> f32,
) -> LayoutParagraph {
    build_draft_paragraph_with_policy(
        document_plan,
        draft_text,
        measure_width,
        paragraph_preserve_underline(&document_plan.body_session.paragraph),
    )
}

pub(super) fn rebuild_layout_pipeline<F>(
    paragraph: LayoutParagraph,
    document_plan: &EditContext,
    draft_text: &str,
    measure_width: &F,
) -> EditorDraftRenderPlan
where
    F: Fn(&str, &LayoutRun) -> f32,
{
    let mut layout = layout_paragraph(&paragraph, paragraph.wrap_width, measure_width);
    align_layout_baseline(&mut layout, source_baseline_y(document_plan));
    let mut caret_lines = build_editor_draft_caret_plan_from_layout(&layout, measure_width);
    let draft_runs_text: String = paragraph.runs.iter().map(|r| r.text.as_str()).collect();
    remap_caret_indices_to_draft_space(
        &mut caret_lines,
        document_plan,
        &draft_runs_text,
        draft_text,
    );
    EditorDraftRenderPlan {
        layout,
        caret_lines,
    }
}

fn marker_bbox_left(marker: &ParagraphEditorMarker) -> Option<f32> {
    marker
        .runs
        .iter()
        .filter(|run| !run.text.is_empty())
        .map(|run| {
            if let Some(first_origin) = run.char_origins.first().copied() {
                run.origin_x + first_origin
            } else {
                run.bbox.left
            }
        })
        .reduce(f32::min)
}

fn marker_render_run(
    marker: &ParagraphEditorMarker,
    paragraph_id: &str,
    body_anchor_left: f32,
) -> Option<LayoutRun> {
    let template = marker.runs.first()?;
    let marker_left = marker_bbox_left(marker).unwrap_or(template.origin_x);
    let mut marker_run = template.clone();
    marker_run.text = marker.text.clone();
    marker_run.id = format!("{}-marker", paragraph_id);
    marker_run.origin_x = marker_left - body_anchor_left;
    marker_run.origin_y = 0.0;
    if !marker_run.char_origins.is_empty() {
        let first_origin = marker_run.char_origins.first().copied().unwrap_or(0.0);
        marker_run.char_origins = marker_run
            .char_origins
            .iter()
            .map(|origin| origin - first_origin)
            .collect();
    }
    Some(marker_run)
}

fn positive_finite_width(width: f32) -> Option<f32> {
    if width.is_finite() && width > 0.0 {
        Some(width)
    } else {
        None
    }
}

fn marker_char_width(marker: &ParagraphEditorMarker) -> Option<f32> {
    let marker_len = marker.text.chars().count();
    if marker_len == 0 {
        return None;
    }

    let mut remaining = marker_len;
    let mut total = 0.0;
    for run in marker.runs.iter().filter(|run| !run.text.is_empty()) {
        let run_len = run.text.chars().count().min(remaining);
        if run_len == 0 {
            continue;
        }
        if run.char_widths.len() < run_len {
            return None;
        }
        let run_width = run
            .char_widths
            .iter()
            .take(run_len)
            .try_fold(0.0, |acc, width| {
                positive_finite_width(*width).map(|w| acc + w)
            })?;
        total += run_width;
        remaining -= run_len;
        if remaining == 0 {
            break;
        }
    }

    if remaining == 0 {
        positive_finite_width(total)
    } else {
        None
    }
}

fn marker_bbox_width(marker: &ParagraphEditorMarker) -> Option<f32> {
    let mut left = f32::INFINITY;
    let mut right = f32::NEG_INFINITY;
    for run in marker.runs.iter().filter(|run| !run.text.is_empty()) {
        if run.bbox.left.is_finite() && run.bbox.right.is_finite() && run.bbox.right > run.bbox.left
        {
            left = left.min(run.bbox.left);
            right = right.max(run.bbox.right);
        }
    }
    positive_finite_width(right - left)
}

fn marker_source_width(marker: &ParagraphEditorMarker) -> Option<f32> {
    marker_char_width(marker).or_else(|| marker_bbox_width(marker))
}

fn prepend_marker_to_first_line(line: &mut VisualLine, marker_run: LayoutRun, marker_width: f32) {
    let marker_text = marker_run.text.clone();
    line.width = line.width.max(marker_run.origin_x + marker_width);
    line.text = format!("{}{}", marker_text, line.text);
    line.runs.insert(0, marker_run);
}

fn inject_fixed_marker<F>(
    plan: &mut EditorDraftRenderPlan,
    document_plan: &EditContext,
    measure_width: &F,
) where
    F: Fn(&str, &LayoutRun) -> f32,
{
    let Some(marker) = document_plan.marker.as_ref() else {
        return;
    };
    let Some(marker_run) = marker_render_run(
        marker,
        &document_plan.body_session.paragraph.id,
        document_plan.body_session.anchor_bbox.left,
    ) else {
        return;
    };
    let marker_width = marker_source_width(marker)
        .unwrap_or_else(|| measure_width(&marker_run.text, &marker_run))
        .max(1.0);
    let marker_text_len = marker_run.text.chars().count();
    if let Some(first_line) = plan.layout.lines.first_mut() {
        prepend_marker_to_first_line(first_line, marker_run, marker_width);
        for caret_line in &mut plan.caret_lines {
            for stop in &mut caret_line.stops {
                stop.index += marker_text_len;
            }
        }
    }
}

/// 构建 draft 渲染计划 — 编辑器 active editing 模式的核心入口。
pub fn build_draft_render_plan<F>(
    document_plan: &EditContext,
    draft_text: &str,
    measure_width: F,
) -> EditorDraftRenderPlan
where
    F: Fn(&str, &LayoutRun) -> f32,
{
    if draft_text == document_plan.source_body_text() && body_runs_match_source_text(document_plan)
    {
        let layout = build_source_layout(document_plan);
        let caret_lines = build_editor_draft_caret_plan_from_layout(&layout, measure_width);
        let plan = EditorDraftRenderPlan {
            layout,
            caret_lines,
        };
        dbg_event(
            "render-plan",
            "existing-layout",
            vec![
                dbg_field("paragraphId", &document_plan.body_session.paragraph.id),
                dbg_field("draftText", draft_text),
                dbg_field("bodyText", document_plan.source_body_text()),
                dbg_field("lineSummary", summarize_render_plan_lines(&plan)),
                dbg_field("visualLineCount", plan.layout.lines.len()),
                dbg_field("caretLineCount", plan.caret_lines.len()),
                dbg_field(
                    "caretStopCount",
                    plan.caret_lines
                        .iter()
                        .map(|line| line.stops.len())
                        .sum::<usize>(),
                ),
            ],
        );
        return plan;
    }

    if draft_text.is_empty() {
        let plan = build_empty_render_plan(document_plan);
        dbg_event(
            "render-plan",
            "uniform-layout-empty",
            vec![
                dbg_field("paragraphId", &document_plan.body_session.paragraph.id),
                dbg_field("draftText", draft_text),
                dbg_field("bodyText", document_plan.source_body_text()),
                dbg_field("lineSummary", summarize_render_plan_lines(&plan)),
            ],
        );
        return plan;
    }

    let paragraph = build_draft_paragraph(document_plan, draft_text, &measure_width);
    let mut layout = layout_paragraph(&paragraph, paragraph.wrap_width, &measure_width);
    align_layout_baseline(&mut layout, source_baseline_y(document_plan));
    let mut caret_lines = build_editor_draft_caret_plan_from_layout(&layout, measure_width);
    let draft_runs_text: String = paragraph.runs.iter().map(|r| r.text.as_str()).collect();
    remap_caret_indices_to_draft_space(
        &mut caret_lines,
        document_plan,
        &draft_runs_text,
        draft_text,
    );
    let plan = EditorDraftRenderPlan {
        layout,
        caret_lines,
    };

    dbg_event(
        "render-plan",
        "uniform-layout",
        vec![
            dbg_field("paragraphId", &document_plan.body_session.paragraph.id),
            dbg_field("draftText", draft_text),
            dbg_field("bodyText", document_plan.source_body_text()),
            dbg_field("lineSummary", summarize_render_plan_lines(&plan)),
            dbg_field("visualLineCount", plan.layout.lines.len()),
            dbg_field("caretLineCount", plan.caret_lines.len()),
            dbg_field(
                "caretStopCount",
                plan.caret_lines
                    .iter()
                    .map(|line| line.stops.len())
                    .sum::<usize>(),
            ),
        ],
    );

    plan
}

/// 构建 persisted overlay 渲染计划 — 用于提交/持久化编辑后的渲染。
pub fn build_persisted_overlay_render_plan<F>(
    document_plan: &EditContext,
    draft_text: &str,
    measure_width: F,
) -> EditorDraftRenderPlan
where
    F: Fn(&str, &LayoutRun) -> f32,
{
    dbg_event(
        "unified-layout",
        "build-persisted-overlay",
        vec![
            dbg_field("draftText", truncate_debug_text(draft_text, 50)),
            dbg_field("hasMarker", document_plan.marker.is_some()),
        ],
    );

    if draft_text.is_empty() && document_plan.marker.is_none() {
        let plan = build_empty_render_plan(document_plan);
        return plan;
    }

    let mut paragraph =
        build_draft_paragraph_with_policy(document_plan, draft_text, &measure_width, false);

    if let Some(marker) = &document_plan.marker {
        paragraph.style.left_indent = marker.advance.max(0.0);
        paragraph.style.first_line_indent = 0.0;
        dbg_event(
            "unified-layout",
            "marker-detected",
            vec![
                dbg_field("markerText", &marker.text),
                dbg_field("markerRuns", marker.runs.len()),
                dbg_field("markerAdvance", marker.advance),
            ],
        );
    }

    let mut plan = rebuild_layout_pipeline(paragraph, document_plan, draft_text, &measure_width);
    inject_fixed_marker(&mut plan, document_plan, &measure_width);
    dbg_event(
        "unified-layout",
        "plan-built",
        vec![dbg_field("lines", plan.layout.lines.len())],
    );
    plan
}
