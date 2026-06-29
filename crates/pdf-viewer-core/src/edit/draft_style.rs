//! Style building logic — 从 draft_layout.rs 拆分。
//!
//! 构建 draft runs 的样式，包括源布局构建、模板选择、run 切片等。

use crate::common::debug::truncate_debug_text;
use crate::edit::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use crate::edit::document_plan::EditContext;
use crate::geometry::layout_engine::{ParagraphLayout, VisualLine};
use crate::models::{BoundingBox, LayoutParagraph, LayoutRun, ParagraphEditContext};
use crate::text::glyph_layout::is_decorative_text;
use crate::text::style_mapper::should_preserve_editor_underline;
use crate::typography::font_resolver::looks_like_symbolic_font;

use super::draft_text_diff::{
    body_runs_match_source_text, body_runs_text, build_index_map, compute_text_diff,
};

pub(super) fn shell_width(session: &ParagraphEditContext) -> f32 {
    (session.anchor_bbox.right - session.anchor_bbox.left).max(1.0)
}

pub(super) fn paragraph_preserve_underline(paragraph: &LayoutParagraph) -> bool {
    should_preserve_editor_underline(paragraph)
}

pub(super) fn same_existing_layout_line(
    reference_baseline_y: f32,
    run: &LayoutRun,
    anchor_top: f32,
) -> bool {
    let baseline_y = (run.origin_y - anchor_top).max(0.0);
    let tolerance = (run.style.font_size * 0.45).max(2.0);
    (reference_baseline_y - baseline_y).abs() <= tolerance
}

pub(super) fn build_source_layout(document_plan: &EditContext) -> ParagraphLayout {
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
        let mut normalized_run = run.clone();
        if !normalized_run.style.scale_x.is_finite()
            || normalized_run.style.scale_x < 0.5
            || normalized_run.style.scale_x > 2.0
        {
            normalized_run.style.scale_x = 1.0;
        }
        if !preserve_underline {
            normalized_run.style.is_underline = false;
        }
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

pub(super) fn resolve_draft_template_run(document_plan: &EditContext) -> LayoutRun {
    resolve_template(
        document_plan,
        paragraph_preserve_underline(&document_plan.body_session.paragraph),
    )
}

pub(super) fn resolve_template(document_plan: &EditContext, preserve_underline: bool) -> LayoutRun {
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
    if !run.style.scale_x.is_finite() || run.style.scale_x < 0.5 || run.style.scale_x > 2.0 {
        run.style.scale_x = 1.0;
    }
    if !preserve_underline {
        run.style.is_underline = false;
    }
    run
}

pub(super) fn find_source_run_index_at_char(runs: &[LayoutRun], index: usize) -> Option<usize> {
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

pub(super) fn is_good_body_style(run: &LayoutRun) -> bool {
    !run.text.trim().is_empty()
        && !is_decorative_text(&run.text)
        && !looks_like_symbolic_font(&run.style.font_name)
}

pub(super) fn select_style(
    document_plan: &EditContext,
    source_runs: &[LayoutRun],
    anchor_index: usize,
    preserve_underline: bool,
) -> LayoutRun {
    let Some(anchor_run_index) = find_source_run_index_at_char(source_runs, anchor_index) else {
        return resolve_template(document_plan, preserve_underline);
    };

    if let Some(run) = source_runs
        .get(anchor_run_index)
        .filter(|run| is_good_body_style(run))
    {
        return run.cleared_style(false, preserve_underline, true);
    }

    let mut left = anchor_run_index as isize - 1;
    let mut right = anchor_run_index + 1;
    while left >= 0 || right < source_runs.len() {
        if left >= 0 {
            if let Some(run) = source_runs
                .get(left as usize)
                .filter(|run| is_good_body_style(run))
            {
                return run.cleared_style(false, preserve_underline, true);
            }
            left -= 1;
        }
        if right < source_runs.len() {
            if let Some(run) = source_runs.get(right).filter(|run| is_good_body_style(run)) {
                return run.cleared_style(false, preserve_underline, true);
            }
            right += 1;
        }
    }

    resolve_template(document_plan, preserve_underline)
}

pub(super) fn slice_runs_by_char_range(
    runs: &[LayoutRun],
    start: usize,
    end: usize,
) -> Vec<LayoutRun> {
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
        let run_len = run.text.chars().count();
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

        // 使用 TextRun::split_at 实现零变换切片
        // 先从 slice_start 处切一刀，取右侧
        // 再从 (slice_end - slice_start) 处切一刀，取左侧
        let text_run = run.to_text_run();
        let (_, right) = text_run.split_at(slice_start);
        let Some(middle_text_run) = right else {
            continue;
        };
        let (middle, _) = middle_text_run.split_at(slice_end - slice_start);
        let Some(middle) = middle else {
            continue;
        };

        let mut sliced = middle.to_layout_run();
        sliced = sliced.cleared_style(true, preserve_underline, true);
        output.push(sliced);
    }
    output
}

/// 构建 draft runs 的样式。
///
/// 架构原则（统一渲染链）：不论是否真的发生编辑、不论 runs 是否含合成空格，
/// 都走同一条 diff + 切片 + 映射路径。这样可以：
///   * 编辑前打开编辑器时 (draft_text == source_text)：diff 得到 prefix=full、
///     insert=∅、suffix=∅，切片返回完整 raw runs（保留 PDF char_origins），
///     渲染结果与原 PDF 像素级一致。
///   * 编辑发生后：未改前后缀切片仍保留 PDF char_origins；只有真正"被插入"的
///     中间片段用 measureText 度量。
///
/// 这取代了过去三条分叉：
///   (a) `draft==source && runs_match`：用 cleared_style(false, _, true)（错误地 clear 了 origins）
///   (b) `draft==source && !runs_match`：用 reconstructed-fallback（单 run、无 origins）
///   (c) `draft!=source && !runs_match`：reconstructed-fallback
/// 这些都会让 PDF char_origins 丢失，导致字体/字距与编辑前/原 PDF 出现可见漂移。
pub(super) fn build_styles(
    document_plan: &EditContext,
    draft_text: &str,
    preserve_underline: bool,
) -> Vec<LayoutRun> {
    let source_text = document_plan.source_body_text();
    let source_runs_match_text = body_runs_match_source_text(document_plan);

    let diff = compute_text_diff(source_text, draft_text);
    let source_len = diff.source_len;
    let draft_len = diff.draft_len;
    let prefix_len = diff.prefix_len;
    let suffix_len = diff.suffix_len;
    let inserted_start = diff.inserted_start();
    let inserted_end = diff.inserted_end();

    // 架构关键点：切片索引必须基于 raw runs 的字符空间，而非 `source_body_text` 的可视空间。
    // `session_source_text` 注入的合成空格在 runs 中并不存在；若直接用 source_text 的索引切片
    // 会导致越界或错位，进而触发 reconstructed-fallback，丢失 PDF char_origins，
    // 用户即看到"删除后字体显示有变化"。
    //
    // 用 `build_index_map` 把 prefix/suffix 边界换算到 raw runs 索引空间。
    // 当 `source_runs_match_text == true`（runs 与 source_text 完全一致），mapping 是恒等映射，
    // 行为与旧实现一致；当为 false（含合成空格），mapping 跳过合成位置正确切片。
    let source_runs = &document_plan.body_session.paragraph.runs;
    let runs_text = body_runs_text(document_plan);
    let runs_total_chars = runs_text.chars().count();
    let mapping = if source_runs_match_text {
        Vec::new() // 不需要 — 走恒等路径
    } else {
        build_index_map(source_text, &runs_text).0
    };
    let map_to_runs_index = |source_index: usize| -> usize {
        if source_runs_match_text {
            source_index.min(runs_total_chars)
        } else {
            mapping
                .get(source_index)
                .copied()
                .unwrap_or(runs_total_chars)
        }
    };

    let prefix_runs_end = map_to_runs_index(prefix_len);
    let suffix_runs_start = map_to_runs_index(source_len.saturating_sub(suffix_len));

    let mut runs = Vec::new();
    runs.extend(slice_runs_by_char_range(source_runs, 0, prefix_runs_end));

    if diff.has_inserted() {
        let anchor_source_index = prefix_len
            .saturating_sub(1)
            .min(source_len.saturating_sub(1));
        let anchor_runs_index = map_to_runs_index(anchor_source_index);
        let mut template = select_style(
            document_plan,
            source_runs,
            anchor_runs_index,
            preserve_underline,
        );
        template.text = draft_text
            .chars()
            .skip(inserted_start)
            .take(inserted_end - inserted_start)
            .collect();
        runs.push(template.cleared_style(false, preserve_underline, true));
    }

    runs.extend(slice_runs_by_char_range(
        source_runs,
        suffix_runs_start,
        runs_total_chars,
    ));

    // 架构原则（编辑前后视觉完全一致 — single rendering chain）：
    // 编辑发生时 *不* 全局 strip 前后缀的 PDF char_origins。
    // 未修改的前缀/后缀通过 `slice_runs_by_char_range` + `cleared_style(true, _, true)`
    // 已保留 run-local PDF 度量，按 PDF 原始字形位置绘制，与编辑前一致。
    // 仅"新插入"中间片段（由 `select_insert_style_run_with_policy` + `cleared_style(false, _, true)`
    // 产生，自然 char_origins 为空）回退到 canvas measureText —— 这是唯一可行选择，
    // 因为新文本不在原 PDF content-stream 中、无任何 glyph 度量可用。
    //
    // 旧版本曾通过 `strip_source_geometry_for_edited_text` 把整段 runs 的 origins 全部清空，
    // 使"未修改字符"在编辑后切换到浏览器 measureText —— 字体/字距与编辑前出现可见漂移，
    // 即"删除后字体显示有变化"分叉症状。该策略和 `edited_text_layout.rs` 整个模块已废弃。

    if runs.is_empty() {
        let mut fallback = resolve_template(document_plan, preserve_underline);
        fallback.text = draft_text.to_string();
        runs.push(fallback.cleared_style(false, preserve_underline, true));
    }

    let preserved_run_count = runs
        .iter()
        .filter(|run| !run.char_origins.is_empty())
        .count();
    let lost_origin_run_count = runs
        .iter()
        .filter(|run| !run.text.is_empty() && run.char_origins.is_empty())
        .count();
    let final_runs_text: String = runs.iter().map(|r| r.text.as_str()).collect();
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
            // ── 编辑前后文字不一致诊断: 把三份关键文本都 dump 出来 ──
            dbg_field("sourceText", source_text),
            dbg_field("draftText", draft_text),
            dbg_field("rawRunsText", runs_text.as_str()),
            dbg_field("finalRunsText", final_runs_text.as_str()),
            dbg_field("sourceRunsMatchText", source_runs_match_text),
            dbg_field("prefixRunsEnd", prefix_runs_end),
            dbg_field("suffixRunsStart", suffix_runs_start),
            dbg_field("runsTotalChars", runs_total_chars),
        ],
    );

    runs
}

pub(super) fn source_baseline_y(document_plan: &EditContext) -> f32 {
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

pub(super) fn build_draft_paragraph_with_policy(
    document_plan: &EditContext,
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
            dbg_field("draftText", &truncate_debug_text(draft_text, 50)),
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
