//! Draft reflow — 核心重排计算子模块。

use crate::edit::debug_trace::{editor_debug_field as dbg_field, record_editor_debug_event as dbg_event};
use crate::edit::document_plan::EditContext;
use crate::geometry::layout_engine::layout_paragraph;
use crate::models::{LayoutParagraph, LayoutRun};
use crate::common::debug::truncate_debug_text;

use super::draft_style::{
    build_draft_paragraph_with_policy, build_source_layout, paragraph_preserve_underline,
    source_baseline_y,
};
use super::draft_text_diff::{body_runs_match_source_text, remap_caret_indices_to_draft_space};
use super::draft_types::EditorDraftRenderPlan;
use super::draft_geometry::{align_layout_baseline, build_editor_draft_caret_plan_from_layout};
use super::draft_init::build_empty_render_plan;

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
    remap_caret_indices_to_draft_space(&mut caret_lines, document_plan, &draft_runs_text, draft_text);
    EditorDraftRenderPlan { layout, caret_lines }
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
    remap_caret_indices_to_draft_space(&mut caret_lines, document_plan, &draft_runs_text, draft_text);
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
        dbg_event(
            "unified-layout",
            "marker-detected",
            vec![
                dbg_field("markerText", &marker.text),
                dbg_field("markerRuns", marker.runs.len()),
            ],
        );
        if let Some(template) = marker.runs.first() {
            let mut marker_run = template.clone();
            marker_run.text = marker.text.clone();
            marker_run.id = format!("{}-marker", paragraph.id);
            marker_run.origin_x = 0.0;
            marker_run.char_origins.clear();
            marker_run.char_widths.clear();
            paragraph.runs.insert(0, marker_run);
            dbg_event(
                "unified-layout",
                "marker-inserted",
                vec![dbg_field("paragraphRuns", paragraph.runs.len())],
            );
        }
    }

    let plan = rebuild_layout_pipeline(paragraph, document_plan, draft_text, &measure_width);
    dbg_event(
        "unified-layout",
        "plan-built",
        vec![dbg_field("lines", plan.layout.lines.len())],
    );
    plan
}
