//! Text diff and index mapping — 从 draft_layout.rs 拆分。
//!
//! 文本差异计算（公共前后缀检测）以及源文本/runs 文本之间的字符索引映射。

use crate::edit::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use crate::edit::document_plan::EditContext;

use super::draft_types::{DraftCaretLine, TextDiff};

/// 构建从 `source_text`（带合成空格的可视文本）到 `runs_text`（raw run 拼接）的双向字符索引映射。
/// 返回 `(source_to_runs, runs_to_source)` 两个映射表。
pub(super) fn build_index_map(source_text: &str, runs_text: &str) -> (Vec<usize>, Vec<usize>) {
    let source_chars: Vec<char> = source_text.chars().collect();
    let runs_chars: Vec<char> = runs_text.chars().collect();
    let source_len = source_chars.len();
    let runs_len = runs_chars.len();

    let mut source_to_runs = vec![0; source_len + 1];
    let mut runs_to_source = vec![0; runs_len + 1];

    let mut s = 0;
    let mut r = 0;

    while s < source_len && r < runs_len {
        let sc = source_chars[s];
        let rc = runs_chars[r];

        if sc == ' ' && rc == ' ' {
            source_to_runs[s] = r;
            runs_to_source[r] = s;
            s += 1;
            r += 1;
        } else if sc == ' ' {
            source_to_runs[s] = r;
            s += 1;
        } else if rc == ' ' {
            runs_to_source[r] = s;
            r += 1;
        } else if sc == rc {
            source_to_runs[s] = r;
            runs_to_source[r] = s;
            s += 1;
            r += 1;
        } else {
            // Mismatch of non-space characters: search for the next match to realign
            let mut found_in_source = None;
            for i in 1..(source_len - s) {
                if source_chars[s + i] == rc {
                    found_in_source = Some(i);
                    break;
                }
            }

            let mut found_in_runs = None;
            for j in 1..(runs_len - r) {
                if runs_chars[r + j] == sc {
                    found_in_runs = Some(j);
                    break;
                }
            }

            match (found_in_source, found_in_runs) {
                (Some(i), Some(j)) => {
                    if i <= j {
                        for k in 0..i {
                            source_to_runs[s + k] = r;
                        }
                        s += i;
                    } else {
                        for k in 0..j {
                            runs_to_source[r + k] = s;
                        }
                        r += j;
                    }
                }
                (Some(i), None) => {
                    for k in 0..i {
                        source_to_runs[s + k] = r;
                    }
                    s += i;
                }
                (None, Some(j)) => {
                    for k in 0..j {
                        runs_to_source[r + k] = s;
                    }
                    r += j;
                }
                (None, None) => {
                    source_to_runs[s] = r;
                    runs_to_source[r] = s;
                    s += 1;
                    r += 1;
                }
            }
        }
    }

    while s < source_len {
        source_to_runs[s] = r;
        s += 1;
    }
    while r < runs_len {
        runs_to_source[r] = s;
        r += 1;
    }

    source_to_runs[source_len] = runs_len;
    runs_to_source[runs_len] = source_len;

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

pub(super) fn body_runs_text(document_plan: &EditContext) -> String {
    document_plan
        .body_session
        .paragraph
        .runs
        .iter()
        .map(|run| run.text.as_str())
        .collect()
}

pub(super) fn body_runs_match_source_text(document_plan: &EditContext) -> bool {
    body_runs_text(document_plan) == document_plan.source_body_text()
}

/// 将 caret stop 索引从 runs-text 空间重映射到 draft-text (source_body_text) 空间。
pub(super) fn remap_caret_indices_to_draft_space(
    caret_lines: &mut [DraftCaretLine],
    document_plan: &EditContext,
    draft_runs_text: &str,
    draft_text: &str,
) {
    if draft_text == draft_runs_text {
        dbg_event(
            "caret.remap",
            "skipped-runs-match",
            vec![
                dbg_field("paragraphId", &document_plan.body_session.paragraph.id),
                dbg_field("draftLen", draft_text.chars().count()),
            ],
        );
        return; // 无需重映射
    }
    let runs_len = draft_runs_text.chars().count();
    let draft_len = draft_text.chars().count();
    let (_, inverse) = build_index_map(draft_text, draft_runs_text);
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

#[cfg(test)]
mod tests {
    use super::super::DraftCaretStop;
    use super::*;

    #[test]
    fn test_build_index_map_no_spaces() {
        let source_text = "HelloWorld";
        let runs_text = "HelloWorld";
        let (s_to_r, r_to_s) = build_index_map(source_text, runs_text);
        assert_eq!(s_to_r, (0..=10).collect::<Vec<_>>());
        assert_eq!(r_to_s, (0..=10).collect::<Vec<_>>());
    }

    #[test]
    fn test_build_index_map_with_synthetic_spaces() {
        let source_text = "Hello World";
        let runs_text = "HelloWorld";
        let (s_to_r, r_to_s) = build_index_map(source_text, runs_text);
        assert_eq!(s_to_r, vec![0, 1, 2, 3, 4, 5, 5, 6, 7, 8, 9, 10]);
        assert_eq!(r_to_s, vec![0, 1, 2, 3, 4, 6, 7, 8, 9, 10, 11]);
    }

    #[test]
    fn test_build_index_map_with_real_spaces() {
        let source_text = "Hello World";
        let runs_text = "Hello World";
        let (s_to_r, r_to_s) = build_index_map(source_text, runs_text);
        assert_eq!(s_to_r, (0..=11).collect::<Vec<_>>());
        assert_eq!(r_to_s, (0..=11).collect::<Vec<_>>());
    }

    #[test]
    fn test_build_index_map_mixed_spaces() {
        let source_text = "编程语言: Rust";
        let runs_text = "编程语言:Rust";
        let (s_to_r, r_to_s) = build_index_map(source_text, runs_text);
        assert_eq!(s_to_r.len(), 11);
        assert_eq!(r_to_s.len(), 10);
        assert_eq!(s_to_r[5], 5);
        assert_eq!(s_to_r[6], 5);
        assert_eq!(r_to_s[5], 6);
    }

    #[test]
    fn test_build_index_map_deletion_middle() {
        let source_text = "Rst";
        let runs_text = "Rust";
        let (s_to_r, r_to_s) = build_index_map(source_text, runs_text);
        assert_eq!(s_to_r, vec![0, 2, 3, 4]);
        assert_eq!(r_to_s, vec![0, 1, 1, 2, 3]);
    }

    #[test]
    fn test_build_index_map_insertion_middle() {
        let source_text = "Rust";
        let runs_text = "Rst";
        let (s_to_r, r_to_s) = build_index_map(source_text, runs_text);
        assert_eq!(s_to_r, vec![0, 1, 1, 2, 3]);
        assert_eq!(r_to_s, vec![0, 2, 3, 4]);
    }

    #[test]
    fn test_build_index_map_mixed_real_synthetic() {
        let source_text = "A  B"; // two spaces (first real, second synthetic)
        let runs_text = "A B"; // one space
        let (s_to_r, r_to_s) = build_index_map(source_text, runs_text);
        assert_eq!(s_to_r, vec![0, 1, 2, 2, 3]);
        assert_eq!(r_to_s, vec![0, 1, 3, 4]);
    }

    #[test]
    fn test_remap_caret_indices() {
        let document_plan = EditContext::default();
        let draft_runs_text = "HelloWorld";
        let draft_text = "Hello World";

        let mut caret_lines = vec![DraftCaretLine {
            baseline_y: 10.0,
            height: 12.0,
            stops: vec![
                DraftCaretStop {
                    index: 0,
                    left: 0.0,
                },
                DraftCaretStop {
                    index: 5,
                    left: 25.0,
                },
                DraftCaretStop {
                    index: 10,
                    left: 50.0,
                },
            ],
        }];

        remap_caret_indices_to_draft_space(
            &mut caret_lines,
            &document_plan,
            draft_runs_text,
            draft_text,
        );

        let stops = &caret_lines[0].stops;
        assert_eq!(stops[0].index, 0);
        assert_eq!(stops[1].index, 6);
        assert_eq!(stops[2].index, 11);
    }

    #[test]
    fn remap_caret_indices_clamps_deleted_suffix_stops() {
        let document_plan = EditContext::default();
        let mut caret_lines = vec![DraftCaretLine {
            baseline_y: 10.0,
            height: 12.0,
            stops: vec![
                DraftCaretStop {
                    index: 0,
                    left: 0.0,
                },
                DraftCaretStop {
                    index: 1,
                    left: 5.0,
                },
                DraftCaretStop {
                    index: 2,
                    left: 10.0,
                },
                DraftCaretStop {
                    index: 3,
                    left: 15.0,
                },
                DraftCaretStop {
                    index: 4,
                    left: 20.0,
                },
            ],
        }];

        remap_caret_indices_to_draft_space(&mut caret_lines, &document_plan, "Rust", "Ru");

        let indices = caret_lines[0]
            .stops
            .iter()
            .map(|stop| stop.index)
            .collect::<Vec<_>>();
        assert_eq!(indices, vec![0, 1, 2, 2, 2]);
    }

    #[test]
    fn remap_caret_indices_handles_multiple_synthetic_gaps() {
        let document_plan = EditContext::default();
        let mut caret_lines = vec![DraftCaretLine {
            baseline_y: 10.0,
            height: 12.0,
            stops: vec![
                DraftCaretStop {
                    index: 0,
                    left: 0.0,
                },
                DraftCaretStop {
                    index: 1,
                    left: 5.0,
                },
                DraftCaretStop {
                    index: 2,
                    left: 10.0,
                },
                DraftCaretStop {
                    index: 3,
                    left: 15.0,
                },
            ],
        }];

        remap_caret_indices_to_draft_space(&mut caret_lines, &document_plan, "ABC", "A B C");

        let indices = caret_lines[0]
            .stops
            .iter()
            .map(|stop| stop.index)
            .collect::<Vec<_>>();
        assert_eq!(indices, vec![0, 2, 4, 5]);
    }
}
