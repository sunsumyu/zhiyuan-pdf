use crate::typography::font_resolver::looks_like_symbolic_font;
use crate::text::glyph_layout::is_decorative_text;
use crate::geometry::layout_engine::{layout_paragraph, ParagraphLayout, VisualLine};
use crate::models::{BoundingBox, ParagraphEditContext, LayoutParagraph, LayoutRun};

use crate::edit::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use crate::edit::document_plan::EditorDocumentPlan;
use crate::text::style_mapper::should_preserve_editor_underline;
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

fn shell_width(session: &ParagraphEditContext) -> f32 {
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

/// 构建从 `source_text`（带合成空格的可视文本）到 `runs_text`（raw run 拼接）的字符索引映射。
///
/// `session_source_text` 在 raw run 之间/内部插入合成空格（CJK 间距、缩写规则、
/// run 间空隙），所以 `source_text` 的字符位置无法直接用于切片 raw runs。
/// 本函数返回长度为 `source_text.chars().count() + 1` 的映射表：
///   `mapping[i]` = `source_text` 的第 i 个字符在 `runs_text` 中对应的字符索引（左闭右开边界）。
/// 合成空格不消耗 raw 游标，跳过即可。最后一个元素是 `runs_text` 总长度，方便用作右开边界。
///
/// 对齐策略：贪心顺序匹配。源串字符若与 raw 当前字符相等则同步推进；
/// 否则视为合成字符（典型场景：合成空格），raw 游标保持。这与 `session_source_text`
/// 仅 *插入* 字符、不 *修改/删除* 字符的语义一致。
fn build_source_to_runs_index_map(source_text: &str, runs_text: &str) -> Vec<usize> {
    let source_chars: Vec<char> = source_text.chars().collect();
    let runs_chars: Vec<char> = runs_text.chars().collect();
    let mut mapping = Vec::with_capacity(source_chars.len() + 1);
    let mut runs_cursor = 0usize;
    for sc in &source_chars {
        if runs_cursor < runs_chars.len() && runs_chars[runs_cursor] == *sc {
            mapping.push(runs_cursor);
            runs_cursor += 1;
        } else {
            // 源串中存在但 raw runs 中不存在 —— 合成字符（如 normalize 出的空格）。
            // 不推进 raw 游标，但仍记录"该位置在 runs 中等价于 runs_cursor"。
            mapping.push(runs_cursor);
        }
    }
    mapping.push(runs_chars.len());
    mapping
}

/// 构建从 `runs_text` 字符索引到 `source_text`（draft）字符索引的逆映射。
/// 当 `source_text` 含合成空格时，runs_text 索引 < source_text 索引。
/// 返回长度 `runs_text.chars().count() + 1` 的向量。
fn build_runs_to_source_index_map(source_text: &str, runs_text: &str) -> Vec<usize> {
    let source_chars: Vec<char> = source_text.chars().collect();
    let runs_chars: Vec<char> = runs_text.chars().collect();
    let source_len = source_chars.len();
    let mut inverse = Vec::with_capacity(runs_chars.len() + 1);
    let mut source_cursor = 0usize;
    for rc in &runs_chars {
        // Skip synthetic chars in source until we find matching real char.
        while source_cursor < source_len && source_chars[source_cursor] != *rc {
            source_cursor += 1;
        }
        // 编辑后 runs 可能含有 draft 已删除的字符，找不到匹配时
        // source_cursor 会停在 source_len。此时仍把映射 clamp 到 source_len（句末），
        // 并且 *不* 越界递增，避免后续映射值漂移到 draft_len 之外。
        if source_cursor >= source_len {
            inverse.push(source_len);
            // 不再递增 source_cursor —— 后续 runs 字符也都映射到 source_len。
        } else {
            inverse.push(source_cursor);
            source_cursor += 1;
        }
    }
    inverse.push(source_len);
    inverse
}

/// 将 caret stop 索引从 runs-text 空间重映射到 draft-text (source_body_text) 空间。
fn remap_caret_indices_to_draft_space(
    caret_lines: &mut [DraftCaretLine],
    document_plan: &EditorDocumentPlan,
    draft_text: &str,
) {
    if body_runs_match_source_text(document_plan) {
        dbg_event(
            "caret.remap",
            "skipped-runs-match",
            vec![
                dbg_field("paragraphId", &document_plan.body_session.paragraph.id),
                dbg_field("draftLen", draft_text.chars().count()),
            ],
        );
        return; // 无合成空格，索引空间一致
    }
    let runs_text = body_runs_text(document_plan);
    let runs_len = runs_text.chars().count();
    let draft_len = draft_text.chars().count();
    let inverse = build_runs_to_source_index_map(draft_text, &runs_text);
    let first_stop_before = caret_lines
        .first()
        .and_then(|l| l.stops.first())
        .map(|s| s.index);
    let last_stop_before = caret_lines
        .last()
        .and_then(|l| l.stops.last())
        .map(|s| s.index);
    let mut total_stops = 0usize;
    let mut out_of_range_stops = 0usize;
    let mut max_stop_index_seen = 0usize;
    for line in caret_lines.iter_mut() {
        for stop in line.stops.iter_mut() {
            total_stops += 1;
            if stop.index > max_stop_index_seen {
                max_stop_index_seen = stop.index;
            }
            if stop.index >= inverse.len() {
                out_of_range_stops += 1;
            }
            stop.index = inverse.get(stop.index).copied().unwrap_or(stop.index);
        }
    }
    dbg_event(
        "caret.remap",
        "stop-stats",
        vec![
            dbg_field("paragraphId", &document_plan.body_session.paragraph.id),
            dbg_field("totalStops", total_stops),
            dbg_field("outOfRangeStops", out_of_range_stops),
            dbg_field("maxStopIndexSeen", max_stop_index_seen),
            dbg_field("inverseLen", inverse.len()),
            dbg_field(
                "lastStopBefore",
                last_stop_before.map(|v| v.to_string()).unwrap_or_default(),
            ),
        ],
    );
    let first_stop_after = caret_lines
        .first()
        .and_then(|l| l.stops.first())
        .map(|s| s.index);
    let last_stop_after = caret_lines
        .last()
        .and_then(|l| l.stops.last())
        .map(|s| s.index);
    dbg_event(
        "caret.remap",
        "applied",
        vec![
            dbg_field("paragraphId", &document_plan.body_session.paragraph.id),
            dbg_field("runsLen", runs_len),
            dbg_field("draftLen", draft_len),
            dbg_field("inverseLen", inverse.len()),
            dbg_field(
                "firstStopBefore",
                first_stop_before.map(|v| v.to_string()).unwrap_or_default(),
            ),
            dbg_field(
                "firstStopAfter",
                first_stop_after.map(|v| v.to_string()).unwrap_or_default(),
            ),
            dbg_field(
                "lastStopAfter",
                last_stop_after.map(|v| v.to_string()).unwrap_or_default(),
            ),
        ],
    );
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

    // 架构原则（统一渲染链）：不论是否真的发生编辑、不论 runs 是否含合成空格，
    // 都走同一条 diff + 切片 + 映射路径。这样可以：
    //   * 编辑前打开编辑器时 (draft_text == source_text)：diff 得到 prefix=full、
    //     insert=∅、suffix=∅，切片返回完整 raw runs（保留 PDF char_origins），
    //     渲染结果与原 PDF 像素级一致。
    //   * 编辑发生后：未改前后缀切片仍保留 PDF char_origins；只有真正"被插入"的
    //     中间片段用 measureText 度量。
    //
    // 这取代了过去三条分叉：
    //   (a) `draft==source && runs_match`：用 normalize_style_run（错误地 clear 了 origins）
    //   (b) `draft==source && !runs_match`：用 reconstructed-fallback（单 run、无 origins）
    //   (c) `draft!=source && !runs_match`：reconstructed-fallback
    // 这些都会让 PDF char_origins 丢失，导致字体/字距与编辑前/原 PDF 出现可见漂移。

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

    // 架构关键点：切片索引必须基于 raw runs 的字符空间，而非 `source_body_text` 的可视空间。
    // `session_source_text` 注入的合成空格在 runs 中并不存在；若直接用 source_text 的索引切片
    // 会导致越界或错位，进而触发 reconstructed-fallback，丢失 PDF char_origins，
    // 用户即看到"删除后字体显示有变化"。
    //
    // 用 `build_source_to_runs_index_map` 把 prefix/suffix 边界换算到 raw runs 索引空间。
    // 当 `source_runs_match_text == true`（runs 与 source_text 完全一致），mapping 是恒等映射，
    // 行为与旧实现一致；当为 false（含合成空格），mapping 跳过合成位置正确切片。
    let source_runs = &document_plan.body_session.paragraph.runs;
    let runs_text = body_runs_text(document_plan);
    let runs_total_chars = runs_text.chars().count();
    let mapping = if source_runs_match_text {
        Vec::new() // 不需要 — 走恒等路径
    } else {
        build_source_to_runs_index_map(source_text, &runs_text)
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

    if inserted_start < inserted_end {
        let anchor_source_index = prefix_len
            .saturating_sub(1)
            .min(source_len.saturating_sub(1));
        let anchor_runs_index = map_to_runs_index(anchor_source_index);
        let mut template = select_insert_style_run_with_policy(
            document_plan,
            source_runs,
            anchor_runs_index,
            preserve_underline,
        );
        template.text = draft_chars[inserted_start..inserted_end].iter().collect();
        runs.push(normalize_style_run(&template, preserve_underline));
    }

    runs.extend(slice_runs_by_char_range(
        source_runs,
        suffix_runs_start,
        runs_total_chars,
    ));

    // 架构原则（编辑前后视觉完全一致 — single rendering chain）：
    // 编辑发生时 *不* 全局 strip 前后缀的 PDF char_origins。
    // 未修改的前缀/后缀通过 `slice_runs_by_char_range` + `normalize_preserved_geometry_run`
    // 已保留 run-local PDF 度量，按 PDF 原始字形位置绘制，与编辑前一致。
    // 仅"新插入"中间片段（由 `select_insert_style_run_with_policy` + `normalize_style_run`
    // 产生，自然 char_origins 为空）回退到 canvas measureText —— 这是唯一可行选择，
    // 因为新文本不在原 PDF content-stream 中、无任何 glyph 度量可用。
    //
    // 旧版本曾通过 `strip_source_geometry_for_edited_text` 把整段 runs 的 origins 全部清空，
    // 使"未修改字符"在编辑后切换到浏览器 measureText —— 字体/字距与编辑前出现可见漂移，
    // 即"删除后字体显示有变化"分叉症状。该策略和 `edited_text_layout.rs` 整个模块已废弃。

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
    // ── draft paragraph diagnostic ──
    let run_summary: String = runs.iter().enumerate().take(8).map(|(i, r)| {
        format!("r{}(co={} cw={} ox={:.1} text='{}')",
            i, r.char_origins.len(), r.char_widths.len(),
            r.origin_x, truncate_debug_text(&r.text, 15))
    }).collect::<Vec<_>>().join(", ");
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
            dbg_field("anchorBBox", format!("[{:.2},{:.2},{:.2},{:.2}]",
                anchor.left, anchor.top, anchor.right, anchor.bottom)),
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
    use crate::edit::document_plan::EditorDocumentPlan;
    use crate::text::glyph_layout::build_editor_session_text_plan;
    use crate::models::{
        BoundingBox, ParagraphEditContext, LayoutParagraph, LayoutRun, RunStyle,
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
        let body_session = ParagraphEditContext {
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
        let body_session = ParagraphEditContext {
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
    fn draft_layout_renders_compact_pdf_text_when_runs_have_no_spaces() {
        // 架构原则（单一渲染链）：编辑器 overlay 渲染的是 *PDF 真实 compact 形态*，
        // 不是 `source_body_text` 的 visual 形态（visual 形态包含 normalize 注入的
        // 合成空格，那些字符并不存在于 PDF content-stream 中）。如此渲染才能让
        // overlay 与 PDF 主画布像素级一致 —— 这是"编辑前后视觉完全一致"的前提。
        // 编辑器 textarea 仍展示 visual 形态供用户输入；overlay 与 textarea 是
        // 各自独立的视图，无需输出同一字符串。
        let runs = vec![
            test_run("r1", "编程语言:", 10.0, 60.0, false),
            test_run("r2", "Rust", 80.0, 110.0, false),
        ];
        let body_session = ParagraphEditContext {
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

        // PDF compact form — 合成空格不出现在 overlay 渲染输出里。
        assert_eq!(rendered_text, "编程语言:Rust");
    }

    #[test]
    fn changed_active_draft_layout_preserves_source_geometry_for_unchanged_parts() {
        // 架构原则（编辑前后视觉完全一致）：
        // 编辑后未修改的前后缀必须继续使用 PDF 原始 char_origins，
        // 仅"新插入"中间片段（无 PDF 度量可用）回退 measureText。
        // 这保证用户删除/插入文字后，未改动的字符像素级与编辑前一致。
        let document_plan = changed_text_document_plan();
        let draft_text = "智能合约: Anchor Framwork, Solana Program Library (SPL), ERC-20/721";

        let plan = build_draft_render_plan(&document_plan, draft_text, |text, run| {
            text.chars().count() as f32 * run.style.font_size.max(1.0) * 0.5
        });

        assert_eq!(rendered_text(&plan), draft_text);
        assert!(
            plan_has_source_char_origins(&plan),
            "edited draft layout must preserve PDF char_origins for unchanged prefix/suffix runs \
             so visual matches pre-edit (single-rendering-chain principle)"
        );
    }

    #[test]
    fn changed_persisted_overlay_layout_preserves_source_geometry_for_unchanged_parts() {
        // 同上：persisted/commit overlay 与 active editing 共享同一布局逻辑，
        // 必须保留未修改前后缀的 PDF 度量，避免提交后字体/字距漂移。
        let document_plan = changed_text_document_plan();
        let draft_text = "智能合约: Anchor Framwork, Solana Program Library (SPL), ERC-20/721";

        let plan =
            build_persisted_overlay_render_plan(&document_plan, draft_text, |text, run| {
                text.chars().count() as f32 * run.style.font_size.max(1.0) * 0.5
            });

        assert_eq!(rendered_text(&plan), draft_text);
        assert!(
            plan_has_source_char_origins(&plan),
            "persisted overlay must preserve PDF char_origins for unchanged prefix/suffix runs \
             so visual matches pre-edit (single-rendering-chain principle)"
        );
    }

    #[test]
    fn edited_draft_preserves_origins_when_runs_lack_synthetic_spaces() {
        // 真实 PDF 场景回归：raw runs 文本 = compact "智能合约:AnchorFramework,..."（无空格），
        // session_source_text 注入合成空格 → "智能合约: Anchor Framework, ..."。
        // 旧实现因 body_runs_match_source_text==false 直接走 reconstructed-fallback，
        // 整段单 run 无 char_origins，触发字体漂移。
        // 新实现通过 source→runs 索引映射继续走 slicing，保留前后缀 PDF 度量。
        let raw_runs_text =
            "智能合约:AnchorFramework,SolanaProgramLibrary(SPL),ERC-20/721".to_string();
        let runs = vec![test_run_with_origins("r1", &raw_runs_text, 10.0, false)];
        let body_session = ParagraphEditContext {
            anchor_bbox: BoundingBox {
                left: 10.0,
                top: 40.0,
                right: 430.0,
                bottom: 52.0,
            },
            paragraph: LayoutParagraph {
                id: "p-compact-pdf".to_string(),
                runs,
                wrap_width: 420.0,
                ..Default::default()
            },
        };
        // 编辑器实际显示给用户的文本（带合成空格），与 raw runs 字符长度不同。
        let visual_source_text =
            "智能合约: Anchor Framework, Solana Program Library (SPL), ERC-20/721".to_string();
        let document_plan = EditorDocumentPlan {
            source_body_text: visual_source_text.clone(),
            body_text_plan: build_editor_session_text_plan(&body_session),
            body_session,
            ..Default::default()
        };
        // 用户在 visual 文本基础上把 "Framework" 改成 "Framwork"（删一个 e）。
        let draft_text = "智能合约: Anchor Framwork, Solana Program Library (SPL), ERC-20/721";

        let plan = build_persisted_overlay_render_plan(&document_plan, draft_text, |text, run| {
            text.chars().count() as f32 * run.style.font_size.max(1.0) * 0.5
        });

        // 架构原则：渲染输出是 PDF 真实 compact 形态（synthetic 空格仅为编辑器
        // textarea 显示用，并不在原 PDF content-stream 内）。如此渲染才能让编辑后
        // 像素级匹配编辑前的 PDF 视觉，否则会插入 PDF 中根本不存在的空格 → 字体漂移。
        let expected_compact =
            "智能合约:AnchorFramwork,SolanaProgramLibrary(SPL),ERC-20/721";
        assert_eq!(rendered_text(&plan), expected_compact);
        assert!(
            plan_has_source_char_origins(&plan),
            "compact-PDF (synthetic-space) scenario must still preserve PDF char_origins \
             for unchanged prefix/suffix via source→runs index mapping"
        );
    }

    #[test]
    fn active_draft_layout_keeps_source_geometry_for_unchanged_split_words() {
        let runs = vec![
            test_run("r1", "A", 0.0, 5.0, false),
            test_run("r2", "nchor", 8.0, 33.0, false),
        ];
        let body_session = ParagraphEditContext {
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

    #[test]
    fn runs_to_source_index_map_accounts_for_synthetic_spaces() {
        // source_text has synthesized spaces; runs_text is compact PDF text.
        let source = "编程语言: Rust";  // 10 chars (space after colon)
        let runs   = "编程语言:Rust";   // 9 chars (no space)
        let inv = super::build_runs_to_source_index_map(source, runs);
        // inv[0]=0(编), inv[1]=1(程), inv[2]=2(语), inv[3]=3(言),
        // inv[4]=4(:), inv[5]=6(R, skips space@5), inv[6]=7(u), inv[7]=8(s), inv[8]=9(t)
        // inv[9]=10 (end sentinel = source.chars().count())
        assert_eq!(inv, vec![0, 1, 2, 3, 4, 6, 7, 8, 9, 10]);
    }

    /// Regression: 删除 draft 中的字符后，runs 仍包含被删字符。
    /// 此前实现会让 source_cursor 越过 source_len 后无界递增，
    /// 导致 inverse 表里出现 > source_len 的非法值（caret 跳到末尾之外）。
    #[test]
    fn runs_to_source_index_map_clamps_when_runs_has_chars_missing_in_source() {
        // 模拟：draft 已被删除最后两个字符 ('s', 't')，但 runs 仍是完整 "Rust"。
        let source = "Ru";   // 2 chars
        let runs   = "Rust"; // 4 chars
        let inv = super::build_runs_to_source_index_map(source, runs);
        // inv 长度 = runs.chars().count() + 1 = 5
        // inv[0]=0(R), inv[1]=1(u),
        // inv[2]=2(s 找不到，clamp 到 source_len=2),
        // inv[3]=2(t 找不到，仍 clamp 到 2 —— 不再 += 1 越界),
        // inv[4]=2 (end sentinel)
        assert_eq!(inv.len(), 5);
        assert_eq!(inv, vec![0, 1, 2, 2, 2]);
        // 关键不变量: 所有映射值都 <= source.chars().count()
        let source_len = source.chars().count();
        for v in &inv {
            assert!(
                *v <= source_len,
                "inverse value {} exceeds source_len {}",
                v,
                source_len
            );
        }
    }
}
