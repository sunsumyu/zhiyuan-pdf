//! source/runs 字符索引映射与 caret 索引重映射。

use crate::edit::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use crate::edit::document_plan::EditorDocumentPlan;

use super::DraftCaretLine;

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
pub(super) fn build_source_to_runs_index_map(source_text: &str, runs_text: &str) -> Vec<usize> {
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
pub(super) fn build_runs_to_source_index_map(source_text: &str, runs_text: &str) -> Vec<usize> {
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

/// Result of comparing source text against draft text.
/// Identifies the unchanged prefix and suffix so only the
/// genuinely edited middle segment needs re-measurement.
pub(super) struct TextDiff {
    pub(super) prefix_len: usize,
    pub(super) suffix_len: usize,
    pub(super) source_len: usize,
    pub(super) draft_len: usize,
}

impl TextDiff {
    /// Start of the inserted/edited segment in draft char space.
    pub(super) fn inserted_start(&self) -> usize {
        self.prefix_len
    }
    /// End of the inserted/edited segment in draft char space.
    pub(super) fn inserted_end(&self) -> usize {
        self.draft_len.saturating_sub(self.suffix_len)
    }
    /// True if there is an edited middle segment.
    pub(super) fn has_inserted(&self) -> bool {
        self.inserted_start() < self.inserted_end()
    }
}

/// Compute the common prefix and suffix lengths between source and draft.
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
