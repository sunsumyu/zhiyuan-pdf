//! 构建入口：build_draft_render_plan / build_persisted_overlay_render_plan。

use crate::common::debug::truncate_debug_text;
use crate::edit::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use crate::edit::document_plan::EditorDocumentPlan;
use crate::geometry::layout_engine::{layout_paragraph, ParagraphLayout, VisualLine};
use crate::models::{LayoutParagraph, LayoutRun};

use super::caret_plan::build_editor_draft_caret_plan_from_layout;
use super::source_layout::{build_source_layout, source_baseline_y};
use super::styles::{
    build_styles, paragraph_preserve_underline, resolve_draft_template_run, resolve_template,
    shell_width,
};
use super::text_mapping::{body_runs_match_source_text, remap_caret_indices_to_draft_space};
use super::{DraftCaretLine, DraftCaretStop, EditorDraftRenderPlan};

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

fn build_draft_paragraph(
    document_plan: &EditorDocumentPlan,
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

fn build_draft_paragraph_with_policy(
    document_plan: &EditorDocumentPlan,
    draft_text: &str,
    _measure_width: &dyn Fn(&str, &LayoutRun) -> f32,
    preserve_underline: bool,
) -> LayoutParagraph {
    let mut paragraph = document_plan.body_session.paragraph.clone();
    let mut runs = build_styles(document_plan, draft_text, preserve_underline);
    if runs.is_empty() {
        let mut template_run = resolve_template(document_plan, preserve_underline);
        template_run.id = format!("editor-draft-{}", document_plan.body_session.paragraph.id);
        template_run.text = draft_text.to_string();
        runs.push(template_run);
    }
    for (index, run) in runs.iter_mut().enumerate() {
        run.id = format!(
            "editor-draft-{}-{}",
            document_plan.body_session.paragraph.id, index
        );
    }
    // ── draft paragraph diagnostic ──
    let run_summary: String = runs
        .iter()
        .enumerate()
        .take(8)
        .map(|(i, r)| {
            format!(
                "r{}(co={} cw={} ox={:.1} text='{}')",
                i,
                r.char_origins.len(),
                r.char_widths.len(),
                r.origin_x,
                truncate_debug_text(&r.text, 15)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let anchor = &document_plan.body_session.anchor_bbox;
    let src_wrap = paragraph.wrap_width;
    let shell_w = shell_width(&document_plan.body_session);
    // ── end diagnostic ──
    paragraph.runs = runs;
    paragraph.wrap_width = paragraph
        .wrap_width
        .max(shell_width(&document_plan.body_session));
    paragraph.bbox = document_plan.body_session.anchor_bbox;
    paragraph.origin_x = document_plan.body_session.anchor_bbox.left;
    paragraph.origin_y = document_plan.body_session.anchor_bbox.top;
    dbg_event(
        "draft-paragraph",
        "built",
        vec![
            dbg_field("paragraphId", &paragraph.id),
            dbg_field("draftText", truncate_debug_text(draft_text, 50)),
            dbg_field("runCount", paragraph.runs.len()),
            dbg_field("srcWrapWidth", format!("{:.2}", src_wrap)),
            dbg_field("shellWidth", format!("{:.2}", shell_w)),
            dbg_field("finalWrapWidth", format!("{:.2}", paragraph.wrap_width)),
            dbg_field(
                "anchorBBox",
                format!(
                    "[{:.2},{:.2},{:.2},{:.2}]",
                    anchor.left, anchor.top, anchor.right, anchor.bottom
                ),
            ),
            dbg_field("runs", run_summary),
        ],
    );
    paragraph
}

fn align_layout_baseline(layout: &mut ParagraphLayout, target_baseline_y: f32) {
    let Some(first_line) = layout.lines.first() else {
        return;
    };
    let baseline_offset = target_baseline_y - first_line.baseline_y;
    if baseline_offset.abs() <= f32::EPSILON {
        return;
    }
    for line in &mut layout.lines {
        line.baseline_y += baseline_offset;
    }
}

fn build_empty_render_plan(document_plan: &EditorDocumentPlan) -> EditorDraftRenderPlan {
    let template_run = resolve_draft_template_run(document_plan);
    let baseline_y = source_baseline_y(document_plan);
    let height = template_run.style.font_size.max(1.0);
    let line = VisualLine {
        text: String::new(),
        runs: vec![template_run],
        width: 0.0,
        height,
        baseline_y,
        offset_x: 0.0,
    };
    let caret_line = DraftCaretLine {
        baseline_y,
        height,
        stops: vec![DraftCaretStop {
            index: 0,
            left: 0.0,
        }],
    };
    EditorDraftRenderPlan {
        layout: ParagraphLayout {
            lines: vec![line],
            height: baseline_y + height,
        },
        caret_lines: vec![caret_line],
    }
}

fn rebuild_layout_pipeline<F>(
    paragraph: LayoutParagraph,
    document_plan: &EditorDocumentPlan,
    draft_text: &str,
    measure_width: &F,
) -> EditorDraftRenderPlan
where
    F: Fn(&str, &LayoutRun) -> f32,
{
    let mut layout = layout_paragraph(&paragraph, paragraph.wrap_width, measure_width);
    align_layout_baseline(&mut layout, source_baseline_y(document_plan));
    let mut caret_lines = build_editor_draft_caret_plan_from_layout(&layout, measure_width);
    remap_caret_indices_to_draft_space(&mut caret_lines, document_plan, draft_text);
    EditorDraftRenderPlan {
        layout,
        caret_lines,
    }
}

fn trace_render_plan(
    action: &str,
    paragraph_id: &str,
    draft_text: &str,
    body_text: &str,
    plan: &EditorDraftRenderPlan,
) {
    dbg_event(
        "render-plan",
        action,
        vec![
            dbg_field("paragraphId", paragraph_id),
            dbg_field("draftText", draft_text),
            dbg_field("bodyText", body_text),
            dbg_field("lineSummary", summarize_render_plan_lines(plan)),
            dbg_field("visualLineCount", plan.layout.lines.len()),
            dbg_field("caretLineCount", plan.caret_lines.len()),
            dbg_field(
                "caretStopCount",
                plan.caret_lines
                    .iter()
                    .map(|l| l.stops.len())
                    .sum::<usize>(),
            ),
        ],
    );
}

pub fn build_draft_render_plan<F>(
    document_plan: &EditorDocumentPlan,
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
    remap_caret_indices_to_draft_space(&mut caret_lines, document_plan, draft_text);
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

pub fn build_persisted_overlay_render_plan<F>(
    document_plan: &EditorDocumentPlan,
    draft_text: &str,
    measure_width: F,
) -> EditorDraftRenderPlan
where
    F: Fn(&str, &LayoutRun) -> f32,
{
    if draft_text.is_empty() {
        let plan = build_empty_render_plan(document_plan);
        dbg_event(
            "render-plan",
            "persisted-overlay-empty",
            vec![
                dbg_field("paragraphId", &document_plan.body_session.paragraph.id),
                dbg_field("draftText", draft_text),
                dbg_field("bodyText", document_plan.source_body_text()),
                dbg_field("lineSummary", summarize_render_plan_lines(&plan)),
            ],
        );
        return plan;
    }

    let paragraph =
        build_draft_paragraph_with_policy(document_plan, draft_text, &measure_width, false);
    build_draft_paragraph_with_policy(document_plan, draft_text, &measure_width, false);
    let plan = rebuild_layout_pipeline(paragraph, document_plan, draft_text, &measure_width);
    trace_render_plan(
        "persisted-overlay-uniform-layout",
        &document_plan.body_session.paragraph.id,
        draft_text,
        document_plan.source_body_text(),
        &plan,
    );

    plan
}
