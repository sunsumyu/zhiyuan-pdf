//! Text diff and index mapping — 从 draft_layout.rs 拆分。
//!
//! 文本差异计算（公共前后缀检测）以及源文本/runs 文本之间的字符索引映射。

use crate::edit::debug_trace::{editor_debug_field as dbg_field, record_editor_debug_event as dbg_event};
use crate::edit::document_plan::{EditContext, EditorDocumentPlan};

use super::draft_types::{DraftCaretLine, TextDiff};

/// 构建从 `source_text`（带合成空格的可视文本）到 `runs_text`（raw run 拼接）的双向字符索引映射。
/// 返回 `(source_to_runs, runs_to_source)` 两个映射表。
pub(super) fn build_index_map(source_text: &str, runs_text: &str) -> (Vec<usize>, Vec<usize>) {
    let source_chars: Vec<char> = source_text.chars().collect();
    let runs_chars: Vec<char> = runs_text.chars().collect();
    let source_len = source_chars.len();
    let runs_len = runs_chars.len();
    let mut source_to_runs = Vec::with_capacity(source_len + 1);
    let mut runs_to_source = Vec::with_capacity(runs_len + 1);
    let mut source_cursor = 0usize;
    let mut runs_cursor = 0usize;

    // 构建 source -> runs 映射
    for sc in &source_chars {
        while runs_cursor < runs_len && runs_chars[runs_cursor] != *sc {
            runs_cursor += 1;
        }
        source_to_runs.push(runs_cursor);
        if runs_cursor < runs_len && runs_chars[runs_cursor] == *sc {
            runs_cursor += 1;
        }
    }
    source_to_runs.push(runs_len);

    // 构建 runs -> source 映射
    runs_cursor = 0;
    for rc in &runs_chars {
        while source_cursor < source_len && source_chars[source_cursor] != *rc {
            source_cursor += 1;
        }
        if source_cursor >= source_len {
            runs_to_source.push(source_len);
        } else {
            runs_to_source.push(source_cursor);
            source_cursor += 1;
        }
    }
    runs_to_source.push(source_len);

    (source_to_runs, runs_to_source)
}

/// 计算源文本和 draft 文本的公共前后缀长度。
pub(super) fn compute_text_diff(source_text: &str, draft_text: &str) -> TextDiff {
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
    TextDiff {
        prefix_len,
        suffix_len,
        source_len: source_chars.len(),
        draft_len: draft_chars.len(),
    }
}

pub(super) fn body_runs_text(document_plan: &EditorDocumentPlan) -> String {
    document_plan
        .body_session
        .paragraph
        .runs
        .iter()
        .map(|run| run.text.as_str())
        .collect()
}

pub(super) fn body_runs_match_source_text(document_plan: &EditorDocumentPlan) -> bool {
    body_runs_text(document_plan) == document_plan.source_body_text()
}

/// 将 caret stop 索引从 runs-text 空间重映射到 draft-text (source_body_text) 空间。
pub(super) fn remap_caret_indices_to_draft_space(
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
    let (_, inverse) = build_index_map(draft_text, &runs_text);
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
