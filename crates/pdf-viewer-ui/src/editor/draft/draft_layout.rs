use pdf_viewer_core::font_resolver::looks_like_symbolic_font;
use pdf_viewer_core::glyph_layout::is_decorative_text;
use pdf_viewer_core::layout_engine::{layout_paragraph, ParagraphLayout, VisualLine};
use pdf_viewer_core::models::{BoundingBox, EditorSession, LayoutParagraph, LayoutRun};

use crate::editor::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use crate::editor::document_plan::EditorDocumentPlan;
use crate::editor::edited_text_layout::{
    resolve_edited_text_geometry_policy, strip_source_geometry_for_edited_text,
    EditedTextGeometryPolicy,
};
use crate::style_mapper::should_preserve_editor_underline;
use crate::utils::debug::truncate_debug_text;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftCaretStop {
    pub index: usize,
    pub left: f32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftCaretLine {
    pub baseline_y: f32,
    pub height: f32,
    pub stops: Vec<DraftCaretStop>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorDraftRenderPlan {
    pub layout: ParagraphLayout,
    pub caret_lines: Vec<DraftCaretLine>,
}

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

fn shell_width(session: &EditorSession) -> f32 {
    (session.anchor_bbox.right - session.anchor_bbox.left).max(1.0)
}

fn paragraph_preserve_underline(paragraph: &LayoutParagraph) -> bool {
    should_preserve_editor_underline(paragraph)
}

fn body_runs_text(document_plan: &EditorDocumentPlan) -> String {
    document_plan
        .body_session
        .paragraph
        .runs
        .iter()
        .map(|run| run.text.as_str())
        .collect()
}

fn body_runs_match_source_text(document_plan: &EditorDocumentPlan) -> bool {
    body_runs_text(document_plan) == document_plan.source_body_text()
}

fn build_reconstructed_text_run_with_policy(
    document_plan: &EditorDocumentPlan,
    text: &str,
    preserve_underline: bool,
) -> Vec<LayoutRun> {
    if text.is_empty() {
        return Vec::new();
    }

    let mut run = resolve_draft_template_run_with_policy(document_plan, preserve_underline);
    run.text = text.to_string();
    vec![normalize_style_run(&run, preserve_underline)]
}

fn same_existing_layout_line(
    reference_baseline_y: f32,
    run: &LayoutRun,
    anchor_top: f32,
) -> bool {
    let baseline_y = (run.origin_y - anchor_top).max(0.0);
    let tolerance = (run.style.font_size * 0.45).max(2.0);
    (reference_baseline_y - baseline_y).abs() <= tolerance
}

fn build_source_layout(document_plan: &EditorDocumentPlan) -> ParagraphLayout {
    let session = &document_plan.body_session;
    let anchor_left = session.anchor_bbox.left;
    let anchor_top = session.anchor_bbox.top;
    let mut lines: Vec<VisualLine> = Vec::new();
    let preserve_underline = paragraph_preserve_underline(&document_plan.body_session.paragraph);

    for run in session
        .paragraph
        .runs
        .iter()
        .filter(|run| !run.text.is_empty())
    {
        let mut normalized_run: LayoutRun = run.clone();
        sanitize_draft_run_style(&mut normalized_run, preserve_underline);
        normalized_run.origin_x = (run.origin_x - anchor_left).max(0.0);
        normalized_run.origin_y = 0.0;
        normalized_run.bbox.left = (run.bbox.left - anchor_left).max(0.0);
        normalized_run.bbox.right = (run.bbox.right - anchor_left).max(normalized_run.bbox.left);
        normalized_run.bbox.top = (run.bbox.top - anchor_top).max(0.0);
        normalized_run.bbox.bottom = (run.bbox.bottom - anchor_top).max(normalized_run.bbox.top);

        if let Some(line) = lines
            .last_mut()
            .filter(|line| same_existing_layout_line(line.baseline_y, run, anchor_top))
        {
            line.text.push_str(&run.text);
            line.width = line.width.max((run.bbox.right - anchor_left).max(0.0));
            line.height = line.height.max(run.style.font_size.max(1.0));
            line.runs.push(normalized_run);
        } else {
            lines.push(VisualLine {
                runs: vec![normalized_run],
                width: (run.bbox.right - anchor_left).max(0.0),
                height: run.style.font_size.max(1.0),
                baseline_y: (run.origin_y - anchor_top).max(0.0),
                offset_x: 0.0,
                text: run.text.clone(),
            });
        }
    }

    let height = lines
        .last()
        .map(|line| line.baseline_y + line.height)
        .unwrap_or(0.0);

    ParagraphLayout { lines, height }
}

fn resolve_draft_template_run(document_plan: &EditorDocumentPlan) -> LayoutRun {
    resolve_draft_template_run_with_policy(
        document_plan,
        paragraph_preserve_underline(&document_plan.body_session.paragraph),
    )
}

fn resolve_draft_template_run_with_policy(
    document_plan: &EditorDocumentPlan,
    preserve_underline: bool,
) -> LayoutRun {
    let mut run = if !document_plan.draft_template_run.id.is_empty()
        || !document_plan.draft_template_run.style.font_name.is_empty()
        || document_plan.draft_template_run.style.font_size > 0.0
    {
        document_plan.draft_template_run.clone()
    } else {
        document_plan
            .body_session
            .paragraph
            .runs
            .iter()
            .find(|run| !run.text.trim().is_empty())
            .cloned()
            .unwrap_or_default()
    };
    run.char_origins.clear();
    run.char_widths.clear();
    run.object_ids.clear();
    run.object_indices.clear();
    run.origin_x = 0.0;
    run.origin_y = 0.0;
    run.bbox = BoundingBox::default();
    sanitize_draft_run_style(&mut run, preserve_underline);
    run
}

fn sanitize_draft_run_style(run: &mut LayoutRun, preserve_underline: bool) {
    if !run.style.scale_x.is_finite() || run.style.scale_x < 0.5 || run.style.scale_x > 2.0 {
        run.style.scale_x = 1.0;
    }
    if !preserve_underline {
        run.style.is_underline = false;
    }
}

fn normalize_style_run(run: &LayoutRun, preserve_underline: bool) -> LayoutRun {
    let mut normalized = run.clone();
    normalized.char_origins.clear();
    normalized.char_widths.clear();
    normalized.object_ids.clear();
    normalized.object_indices.clear();
    normalized.origin_x = 0.0;
    normalized.origin_y = 0.0;
    normalized.bbox = BoundingBox::default();
    sanitize_draft_run_style(&mut normalized, preserve_underline);
    normalized
}

fn normalize_preserved_geometry_run(run: &LayoutRun, preserve_underline: bool) -> LayoutRun {
    let mut normalized = run.clone();
    normalized.object_ids.clear();
    normalized.object_indices.clear();
    normalized.origin_x = 0.0;
    normalized.origin_y = 0.0;
    normalized.bbox = BoundingBox::default();
    sanitize_draft_run_style(&mut normalized, preserve_underline);
    normalized
}

fn find_source_run_index_at_char(runs: &[LayoutRun], index: usize) -> Option<usize> {
    let mut cursor = 0usize;
    for (run_index, run) in runs.iter().enumerate() {
        let run_len = run.text.chars().count();
        if run_len == 0 {
            continue;
        }
        let run_start = cursor;
        let run_end = cursor + run_len;
        if index >= run_start && index < run_end {
            return Some(run_index);
        }
        cursor = run_end;
    }
    None
}

fn is_good_body_style(run: &LayoutRun) -> bool {
    !run.text.trim().is_empty()
        && !is_decorative_text(&run.text)
        && !looks_like_symbolic_font(&run.style.font_name)
}

fn select_insert_style_run_with_policy(
    document_plan: &EditorDocumentPlan,
    source_runs: &[LayoutRun],
    anchor_index: usize,
    preserve_underline: bool,
) -> LayoutRun {
    let Some(anchor_run_index) = find_source_run_index_at_char(source_runs, anchor_index) else {
        return resolve_draft_template_run_with_policy(document_plan, preserve_underline);
    };

    if let Some(run) = source_runs
        .get(anchor_run_index)
        .filter(|run| is_good_body_style(run))
    {
        return normalize_style_run(run, preserve_underline);
    }

    let mut left = anchor_run_index as isize - 1;
    let mut right = anchor_run_index + 1;
    while left >= 0 || right < source_runs.len() {
        if left >= 0 {
            if let Some(run) = source_runs
                .get(left as usize)
                .filter(|run| is_good_body_style(run))
            {
                return normalize_style_run(run, preserve_underline);
            }
            left -= 1;
        }
        if right < source_runs.len() {
            if let Some(run) = source_runs
                .get(right)
                .filter(|run| is_good_body_style(run))
            {
                return normalize_style_run(run, preserve_underline);
            }
            right += 1;
        }
    }

    resolve_draft_template_run_with_policy(document_plan, preserve_underline)
}

fn slice_runs_by_char_range(runs: &[LayoutRun], start: usize, end: usize) -> Vec<LayoutRun> {
    let preserve_underline = should_preserve_editor_underline(&LayoutParagraph {
        runs: runs.to_vec(),
        ..LayoutParagraph::default()
    });
    if start >= end {
        return Vec::new();
    }
    let mut output = Vec::new();
    let mut cursor = 0usize;
    for run in runs.iter().filter(|run| !run.text.is_empty()) {
        let chars: Vec<char> = run.text.chars().collect();
        let run_len = chars.len();
        let run_start = cursor;
        let run_end = cursor + run_len;
        cursor = run_end;

        if end <= run_start || start >= run_end {
            continue;
        }
        let slice_start = start.saturating_sub(run_start).min(run_len);
        let slice_end = end.saturating_sub(run_start).min(run_len);
        if slice_start >= slice_end {
            continue;
        }
        let mut sliced = run.clone();
        sliced.text = chars[slice_start..slice_end].iter().collect();
        if !run.char_origins.is_empty() && run.char_origins.len() >= slice_end {
            let origins = &run.char_origins[slice_start..slice_end];
            if let Some(first_origin) = origins.first().copied() {
                sliced.char_origins = origins.iter().map(|o| o - first_origin).collect();
            } else {
                sliced.char_origins.clear();
            }
            if run.char_widths.len() >= slice_end {
                sliced.char_widths = run.char_widths[slice_start..slice_end]
                    .iter()
                    .copied()
                    .collect();
            } else {
                sliced.char_widths.clear();
            }
        } else {
            sliced.char_origins.clear();
            sliced.char_widths.clear();
        }
        sliced = normalize_preserved_geometry_run(&sliced, preserve_underline);
        output.push(sliced);
    }
    output
}

fn build_style_runs_for_draft_text_with_policy(
    document_plan: &EditorDocumentPlan,
    draft_text: &str,
    preserve_underline: bool,
) -> Vec<LayoutRun> {
    let source_text = document_plan.source_body_text();
    let source_runs_match_text = body_runs_match_source_text(document_plan);
    let geometry_policy =
        resolve_edited_text_geometry_policy(source_text, draft_text, source_runs_match_text);
    if draft_text == source_text {
        if source_runs_match_text {
            return document_plan
                .body_session
                .paragraph
                .runs
                .iter()
                .filter(|run| !run.text.is_empty())
                .map(|run| normalize_style_run(run, preserve_underline))
                .collect();
        }

        return build_reconstructed_text_run_with_policy(
            document_plan,
            source_text,
            preserve_underline,
        );
    }

    if !source_runs_match_text {
        return build_reconstructed_text_run_with_policy(
            document_plan,
            draft_text,
            preserve_underline,
        );
    }

    let source_chars: Vec<char> = source_text.chars().collect();
    let draft_chars: Vec<char> = draft_text.chars().collect();
    let mut prefix_len = 0usize;
    while prefix_len < source_chars.len()
        && prefix_len < draft_chars.len()
        && source_chars[prefix_len] == draft_chars[prefix_len]
    {
        prefix_len += 1;
    }

    let mut suffix_len = 0usize;
    while suffix_len < source_chars.len().saturating_sub(prefix_len)
        && suffix_len < draft_chars.len().saturating_sub(prefix_len)
        && source_chars[source_chars.len() - 1 - suffix_len]
            == draft_chars[draft_chars.len() - 1 - suffix_len]
    {
        suffix_len += 1;
    }

    let source_len = source_chars.len();
    let draft_len = draft_chars.len();
    let inserted_start = prefix_len;
    let inserted_end = draft_len.saturating_sub(suffix_len);

    let mut runs = Vec::new();
    let source_runs = &document_plan.body_session.paragraph.runs;
    runs.extend(slice_runs_by_char_range(source_runs, 0, prefix_len));

    if inserted_start < inserted_end {
        let source_runs = &document_plan.body_session.paragraph.runs;
        let anchor_index = prefix_len
            .saturating_sub(1)
            .min(source_len.saturating_sub(1));
        let mut template = select_insert_style_run_with_policy(
            document_plan,
            source_runs,
            anchor_index,
            preserve_underline,
        );
        template.text = draft_chars[inserted_start..inserted_end].iter().collect();
        runs.push(normalize_style_run(&template, preserve_underline));
    }

    let suffix_start = source_len.saturating_sub(suffix_len);
    runs.extend(slice_runs_by_char_range(
        source_runs,
        suffix_start,
        source_len,
    ));

    if geometry_policy == EditedTextGeometryPolicy::MeasureEditedText {
        strip_source_geometry_for_edited_text(&mut runs);
    }

    if runs.is_empty() {
        let mut fallback =
            resolve_draft_template_run_with_policy(document_plan, preserve_underline);
        fallback.text = draft_text.to_string();
        runs.push(normalize_style_run(&fallback, preserve_underline));
    }

    let preserved_run_count = runs
        .iter()
        .filter(|run| !run.char_origins.is_empty())
        .count();
    let lost_origin_run_count = runs
        .iter()
        .filter(|run| !run.text.is_empty() && run.char_origins.is_empty())
        .count();
    dbg_event(
        "draft-runs",
        "resolved",
        vec![
            dbg_field("paragraphId", &document_plan.body_session.paragraph.id),
            dbg_field("sourceLen", source_len),
            dbg_field("draftLen", draft_len),
            dbg_field("prefixLen", prefix_len),
            dbg_field("suffixLen", suffix_len),
            dbg_field("preservedRunCount", preserved_run_count),
            dbg_field("measuredRunCount", lost_origin_run_count),
            dbg_field("lostOriginRunCount", lost_origin_run_count),
        ],
    );

    runs
}

fn source_baseline_y(document_plan: &EditorDocumentPlan) -> f32 {
    document_plan
        .body_session
        .paragraph
        .runs
        .iter()
        .find(|run| !run.text.is_empty())
        .map(|run| (run.origin_y - document_plan.body_session.anchor_bbox.top).max(0.0))
        .unwrap_or_else(|| {
            resolve_draft_template_run(document_plan)
                .style
                .font_size
                .max(1.0)
        })
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
    let mut runs = build_style_runs_for_draft_text_with_policy(
        document_plan,
        draft_text,
        preserve_underline,
    );
    if runs.is_empty() {
        let mut template_run =
            resolve_draft_template_run_with_policy(document_plan, preserve_underline);
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
    paragraph.runs = runs;
    paragraph.wrap_width = paragraph
        .wrap_width
        .max(shell_width(&document_plan.body_session));
    paragraph.bbox = document_plan.body_session.anchor_bbox;
    paragraph.origin_x = document_plan.body_session.anchor_bbox.left;
    paragraph.origin_y = document_plan.body_session.anchor_bbox.top;
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

fn build_editor_draft_caret_plan_from_layout<F>(
    layout: &ParagraphLayout,
    measure_width: F,
) -> Vec<DraftCaretLine>
where
    F: Fn(&str, &LayoutRun) -> f32,
{
    let mut lines = Vec::new();
    let mut consumed = 0usize;

    for line in &layout.lines {
        let mut caret_line = DraftCaretLine {
            baseline_y: line.baseline_y,
            height: line
                .runs
                .iter()
                .map(|run| run.style.font_size.max(1.0))
                .fold(1.0, f32::max),
            stops: Vec::new(),
        };

        for run in &line.runs {
            let start_index = consumed;
            let run_origin_x = line.offset_x + run.origin_x;
            let chars: Vec<char> = run.text.chars().collect();
            let glyph_count = chars.len();
            if glyph_count == 0 {
                continue;
            }

            if !run.char_origins.is_empty() {
                let first_origin = run.char_origins.first().copied().unwrap_or(0.0);
                caret_line.stops.push(DraftCaretStop {
                    index: start_index,
                    left: run_origin_x + first_origin,
                });
                for glyph_index in 0..glyph_count {
                    let origin = run
                        .char_origins
                        .get(glyph_index)
                        .copied()
                        .unwrap_or(first_origin);
                    let right = if let Some(width) = run.char_widths.get(glyph_index).copied() {
                        origin + width
                    } else if let Some(next_origin) = run.char_origins.get(glyph_index + 1).copied()
                    {
                        next_origin
                    } else {
                        let mut buf = [0_u8; 4];
                        let glyph = chars[glyph_index].encode_utf8(&mut buf);
                        origin + measure_width(glyph, run)
                    };
                    caret_line.stops.push(DraftCaretStop {
                        index: start_index + glyph_index + 1,
                        left: run_origin_x + right,
                    });
                }
                consumed += glyph_count;
                continue;
            }

            caret_line.stops.push(DraftCaretStop {
                index: start_index,
                left: run_origin_x,
            });
            let mut prefix = String::new();
            for (glyph_index, ch) in chars.iter().enumerate() {
                prefix.push(*ch);
                let prefix_width = measure_width(&prefix, run);
                caret_line.stops.push(DraftCaretStop {
                    index: start_index + glyph_index + 1,
                    left: run_origin_x + prefix_width,
                });
            }
            consumed += glyph_count;
        }

        if caret_line.stops.is_empty() {
            caret_line.stops.push(DraftCaretStop {
                index: consumed,
                left: line.offset_x,
            });
        }

        lines.push(caret_line);
    }

    if lines.is_empty() {
        lines.push(DraftCaretLine {
            baseline_y: 0.0,
            height: 1.0,
            stops: vec![DraftCaretStop {
                index: 0,
                left: 0.0,
            }],
        });
    }

    lines
}

pub fn build_draft_render_plan<F>(
    document_plan: &EditorDocumentPlan,
    draft_text: &str,
    measure_width: F,
) -> EditorDraftRenderPlan
where
    F: Fn(&str, &LayoutRun) -> f32,
{
    if draft_text == document_plan.source_body_text()
        && body_runs_match_source_text(document_plan)
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
    let caret_lines = build_editor_draft_caret_plan_from_layout(&layout, measure_width);
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
    let mut layout = layout_paragraph(&paragraph, paragraph.wrap_width, &measure_width);
    align_layout_baseline(&mut layout, source_baseline_y(document_plan));
    let caret_lines = build_editor_draft_caret_plan_from_layout(&layout, measure_width);
    let plan = EditorDraftRenderPlan {
        layout,
        caret_lines,
    };

    dbg_event(
        "render-plan",
        "persisted-overlay-uniform-layout",
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
            dbg_field("preserveUnderline", false),
        ],
    );

    plan
}

#[cfg(test)]
mod tests {
    use super::{
        build_draft_render_plan, build_persisted_overlay_render_plan, build_source_layout,
    };
    use crate::editor::document_plan::EditorDocumentPlan;
    use pdf_viewer_core::glyph_layout::build_editor_session_text_plan;
    use pdf_viewer_core::models::{
        BoundingBox, EditorSession, LayoutParagraph, LayoutRun, RunStyle,
    };

    fn test_run(id: &str, text: &str, left: f32, right: f32, underline: bool) -> LayoutRun {
        LayoutRun {
            id: id.to_string(),
            text: text.to_string(),
            style: RunStyle {
                font_name: "MicrosoftYaHei".to_string(),
                font_size: 10.0,
                color: "#000000".to_string(),
                is_bold: false,
                is_italic: false,
                is_underline: underline,
                char_spacing: 0.0,
                scale_x: 1.0,
            },
            bbox: BoundingBox {
                left,
                top: 40.0,
                right,
                bottom: 50.0,
            },
            origin_x: left,
            origin_y: 50.0,
            char_origins: Vec::new(),
            char_widths: Vec::new(),
            object_ids: Vec::new(),
            object_indices: Vec::new(),
        }
    }

    fn test_run_with_origins(id: &str, text: &str, left: f32, underline: bool) -> LayoutRun {
        let char_count = text.chars().count();
        let char_origins = (0..char_count)
            .map(|index| index as f32 * 5.0)
            .collect::<Vec<_>>();
        let char_widths = vec![5.0; char_count];
        let mut run = test_run(id, text, left, left + char_count as f32 * 5.0, underline);
        run.char_origins = char_origins;
        run.char_widths = char_widths;
        run.object_ids = vec!["source-text-object".to_string()];
        run.object_indices = vec![0];
        run
    }

    fn changed_text_document_plan() -> EditorDocumentPlan {
        let source_text =
            "智能合约: Anchor Framework, Solana Program Library (SPL), ERC-20/721".to_string();
        let runs = vec![test_run_with_origins("r1", &source_text, 10.0, false)];
        let body_session = EditorSession {
            anchor_bbox: BoundingBox {
                left: 10.0,
                top: 40.0,
                right: 430.0,
                bottom: 52.0,
            },
            paragraph: LayoutParagraph {
                id: "p-smart-contract".to_string(),
                runs,
                wrap_width: 420.0,
                ..Default::default()
            },
        };
        EditorDocumentPlan {
            source_body_text: source_text,
            body_text_plan: build_editor_session_text_plan(&body_session),
            body_session,
            ..Default::default()
        }
    }

    fn rendered_text(plan: &super::EditorDraftRenderPlan) -> String {
        plan.layout
            .lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<String>()
    }

    fn plan_has_source_char_origins(plan: &super::EditorDraftRenderPlan) -> bool {
        plan.layout
            .lines
            .iter()
            .flat_map(|line| line.runs.iter())
            .any(|run| !run.char_origins.is_empty() || !run.char_widths.is_empty())
    }

    #[test]
    fn source_layout_sanitizes_partial_underlines_for_editor_canvas() {
        let runs = vec![
            test_run("r1", "专业：", 10.0, 40.0, true),
            test_run("r2", "计算机科学与技术", 40.0, 130.0, false),
        ];
        let body_session = EditorSession {
            anchor_bbox: BoundingBox {
                left: 10.0,
                top: 40.0,
                right: 130.0,
                bottom: 50.0,
            },
            paragraph: LayoutParagraph {
                runs,
                ..Default::default()
            },
        };
        let document_plan = EditorDocumentPlan {
            source_body_text: "专业：计算机科学与技术".to_string(),
            body_text_plan: build_editor_session_text_plan(&body_session),
            body_session,
            ..Default::default()
        };

        let layout = build_source_layout(&document_plan);
        let underline_count = layout
            .lines
            .iter()
            .flat_map(|line| line.runs.iter())
            .filter(|run| run.style.is_underline)
            .count();

        assert_eq!(underline_count, 0);
    }

    #[test]
    fn draft_layout_uses_canonical_text_when_raw_runs_have_no_spaces() {
        let runs = vec![
            test_run("r1", "编程语言:", 10.0, 60.0, false),
            test_run("r2", "Rust", 80.0, 110.0, false),
        ];
        let body_session = EditorSession {
            anchor_bbox: BoundingBox {
                left: 10.0,
                top: 40.0,
                right: 300.0,
                bottom: 50.0,
            },
            paragraph: LayoutParagraph {
                runs,
                wrap_width: 290.0,
                ..Default::default()
            },
        };
        let document_plan = EditorDocumentPlan {
            source_body_text: "编程语言: Rust".to_string(),
            body_text_plan: build_editor_session_text_plan(&body_session),
            body_session,
            ..Default::default()
        };

        let plan = build_persisted_overlay_render_plan(
            &document_plan,
            "编程语言: Rust",
            |text, run| text.chars().count() as f32 * run.style.font_size.max(1.0) * 0.5,
        );
        let rendered_text = plan
            .layout
            .lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<String>();

        assert_eq!(rendered_text, "编程语言: Rust");
    }

    #[test]
    fn changed_active_draft_layout_drops_source_geometry() {
        let document_plan = changed_text_document_plan();
        let draft_text = "智能合约: Anchor Framwork, Solana Program Library (SPL), ERC-20/721";

        let plan = build_draft_render_plan(&document_plan, draft_text, |text, run| {
            text.chars().count() as f32 * run.style.font_size.max(1.0) * 0.5
        });

        assert_eq!(rendered_text(&plan), draft_text);
        assert!(
            !plan_has_source_char_origins(&plan),
            "edited active text must use one measured Rust layout chain, not sliced PDF char origins"
        );
    }

    #[test]
    fn changed_persisted_overlay_layout_uses_same_measured_geometry() {
        let document_plan = changed_text_document_plan();
        let draft_text = "智能合约: Anchor Framwork, Solana Program Library (SPL), ERC-20/721";

        let plan =
            build_persisted_overlay_render_plan(&document_plan, draft_text, |text, run| {
                text.chars().count() as f32 * run.style.font_size.max(1.0) * 0.5
            });

        assert_eq!(rendered_text(&plan), draft_text);
        assert!(
            !plan_has_source_char_origins(&plan),
            "committed preview must consume the same measured edited-text layout as active editing"
        );
    }

    #[test]
    fn active_draft_layout_keeps_source_geometry_for_unchanged_split_words() {
        let runs = vec![
            test_run("r1", "A", 0.0, 5.0, false),
            test_run("r2", "nchor", 8.0, 33.0, false),
        ];
        let body_session = EditorSession {
            anchor_bbox: BoundingBox {
                left: 0.0,
                top: 40.0,
                right: 33.0,
                bottom: 50.0,
            },
            paragraph: LayoutParagraph {
                runs,
                wrap_width: 33.0,
                ..Default::default()
            },
        };
        let document_plan = EditorDocumentPlan {
            source_body_text: "Anchor".to_string(),
            body_text_plan: build_editor_session_text_plan(&body_session),
            body_session,
            ..Default::default()
        };

        let plan = build_draft_render_plan(&document_plan, "Anchor", |text, _run| {
            // Deliberately wrong measurement: if active edit mode reflows the
            // split runs, "nchor" moves to x=20 and the visual gap regresses.
            if text == "A" {
                20.0
            } else {
                text.chars().count() as f32 * 5.0
            }
        });

        let line = plan.layout.lines.first().expect("expected one source line");
        assert_eq!(line.text, "Anchor");
        assert_eq!(line.runs.len(), 2);
        assert_eq!(line.runs[0].origin_x, 0.0);
        assert_eq!(line.runs[1].origin_x, 8.0);
    }
}
