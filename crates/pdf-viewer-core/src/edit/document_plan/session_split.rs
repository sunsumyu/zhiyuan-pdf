//! marker/body 会话切分：SessionSplit、run 级拆分、symbolic 字体检测、几何合成、切分策略链。

use crate::edit::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use crate::models::VisualMarker;
use crate::models::{
    BoundingBox, GlyphPaintParagraph, LayoutParagraph, LayoutRun, ParagraphEditContext,
};
use crate::text::glyph_layout::EditorSessionTextPlan;
use crate::text::list_semantics::{derive_list_text_semantics, ListMarkerKind};
use crate::typography::font_resolver::looks_like_symbolic_font;

use super::line_plans::{bbox_from_runs, split_run_at_char_index};
use super::ParagraphEditorMarker;

// ── Build functions ─────────────────────────────────────────────

pub(super) fn resolve_shell_bbox(
    target_session: &ParagraphEditContext,
    split: &SessionSplit,
    graphic_markers: &[VisualMarker],
) -> BoundingBox {
    let mut shell_bbox = split.body_session.anchor_bbox;
    let mut expanded = false;

    if let Some(marker) = split.marker.as_ref() {
        if let Some(marker_bbox) = bbox_from_runs(&marker.runs) {
            shell_bbox.left = shell_bbox.left.min(marker_bbox.left);
            shell_bbox.top = shell_bbox.top.min(marker_bbox.top);
            shell_bbox.right = shell_bbox.right.max(marker_bbox.right);
            shell_bbox.bottom = shell_bbox.bottom.max(marker_bbox.bottom);
            expanded = true;
        }
    }

    for marker in graphic_markers {
        shell_bbox.left = shell_bbox.left.min(marker.bbox.left);
        shell_bbox.top = shell_bbox.top.min(marker.bbox.top);
        shell_bbox.right = shell_bbox.right.max(marker.bbox.right);
        shell_bbox.bottom = shell_bbox.bottom.max(marker.bbox.bottom);
        expanded = true;
    }

    if expanded {
        shell_bbox
    } else {
        target_session.anchor_bbox
    }
}

#[derive(Debug, Clone)]
pub(super) struct SessionSplit {
    pub(super) body_session: ParagraphEditContext,
    pub(super) marker: Option<ParagraphEditorMarker>,
}

pub(super) fn split_editor_session(
    session: &ParagraphEditContext,
    body_char_start: usize,
    marker_kind: ListMarkerKind,
) -> Option<SessionSplit> {
    let para_text_len: usize = session
        .paragraph
        .runs
        .iter()
        .map(|r| r.text.chars().count())
        .sum();
    dbg_event(
        "split-marker",
        "entry",
        vec![
            dbg_field("paragraphId", session.paragraph.id.as_str()),
            dbg_field("bodyCharStart", body_char_start),
            dbg_field("paragraphTextLen", para_text_len),
            dbg_field("runCount", session.paragraph.runs.len()),
            dbg_field("markerKind", format!("{:?}", marker_kind)),
        ],
    );
    if body_char_start == 0 {
        dbg_event(
            "split-marker",
            "no-marker-zero-start",
            vec![dbg_field("paragraphId", session.paragraph.id.as_str())],
        );
        return Some(SessionSplit {
            body_session: session.clone(),
            marker: None,
        });
    }

    let mut consumed = 0usize;
    let mut marker_runs = Vec::new();
    let mut body_runs = Vec::new();

    for run in &session.paragraph.runs {
        let glyph_count = run.text.chars().count();
        let run_start = consumed;
        let run_end = consumed + glyph_count;

        if body_char_start >= run_end {
            marker_runs.push(run.clone());
        } else if body_char_start <= run_start {
            body_runs.push(run.clone());
        } else {
            let split_index = body_char_start.saturating_sub(run_start);
            let (marker_run, body_run) = split_run_at_char_index(run, split_index);
            if let Some(marker_run) = marker_run {
                marker_runs.push(marker_run);
            }
            if let Some(body_run) = body_run {
                body_runs.push(body_run);
            }
        }

        consumed = run_end;
    }

    if body_runs.is_empty() {
        return None;
    }

    let body_bbox = bbox_from_runs(&body_runs)?;
    let marker_text = marker_runs
        .iter()
        .map(|run| run.text.as_str())
        .collect::<String>();
    let marker_advance = (body_bbox.left - session.anchor_bbox.left).max(0.0);

    let mut body_paragraph: LayoutParagraph = session.paragraph.clone();
    body_paragraph.bbox = body_bbox;
    body_paragraph.origin_x = body_runs
        .first()
        .map(|run| run.origin_x)
        .unwrap_or(body_paragraph.origin_x);
    body_paragraph.origin_y = body_runs
        .first()
        .map(|run| run.origin_y)
        .unwrap_or(body_paragraph.origin_y);
    body_paragraph.wrap_width = if body_paragraph.wrap_width > 0.0 {
        (body_paragraph.wrap_width - marker_advance).max(body_bbox.right - body_bbox.left)
    } else {
        (body_bbox.right - body_bbox.left).max(1.0)
    };
    body_paragraph.runs = body_runs;

    Some(SessionSplit {
        body_session: ParagraphEditContext {
            anchor_bbox: body_bbox,
            paragraph: body_paragraph,
        },
        marker: Some(ParagraphEditorMarker {
            kind: marker_kind,
            text: marker_text,
            advance: marker_advance,
            runs: marker_runs,
        }),
    })
}

/// Font-aware marker detection for symbolic-font bullets.
fn detect_symbolic_font_marker(session: &ParagraphEditContext) -> Option<(usize, ListMarkerKind)> {
    let runs = &session.paragraph.runs;
    let non_empty_runs: Vec<&LayoutRun> = runs.iter().filter(|r| !r.text.is_empty()).collect();
    if non_empty_runs.len() < 2 {
        return None;
    }

    let first_run = non_empty_runs[0];
    if !looks_like_symbolic_font(&first_run.style.font_name) {
        return None;
    }

    let mut marker_char_count = 0usize;
    for run in runs.iter() {
        if run.text.is_empty() {
            continue;
        }
        if !looks_like_symbolic_font(&run.style.font_name) {
            break;
        }
        marker_char_count += run.text.chars().count();
    }

    if marker_char_count == 0 {
        return None;
    }

    let full_text: String = runs.iter().map(|r| r.text.as_str()).collect();
    let chars: Vec<char> = full_text.chars().collect();
    let mut body_start = marker_char_count;
    while body_start < chars.len() && chars[body_start].is_whitespace() {
        body_start += 1;
    }

    if body_start >= chars.len() {
        return None;
    }

    Some((body_start, ListMarkerKind::Symbol))
}

pub(super) fn synthesize_marker_from_paragraph(
    paragraph: &GlyphPaintParagraph,
    body_session: &ParagraphEditContext,
) -> Option<ParagraphEditorMarker> {
    let body_runs = &body_session.paragraph.runs;
    let body_first = body_runs.iter().find(|run| !run.text.is_empty())?;
    let body_origin_y = body_first.origin_y;
    let body_origin_x = body_first.origin_x;
    let body_font_size = body_first.style.font_size.max(1.0);
    let line_tolerance = (body_font_size * 0.9).max(4.0);

    use std::collections::HashSet;
    let body_run_ids: HashSet<&str> = body_runs.iter().map(|r| r.id.as_str()).collect();

    let candidates: Vec<LayoutRun> = paragraph
        .editor_session
        .paragraph
        .runs
        .iter()
        .filter(|run| !run.text.trim().is_empty())
        .filter(|run| !body_run_ids.contains(run.id.as_str()))
        .filter(|run| (run.origin_y - body_origin_y).abs() <= line_tolerance)
        .filter(|run| run.bbox.right <= body_origin_x + 1.0)
        .filter(|run| {
            let first_char = run.text.trim_start().chars().next();
            first_char
                .map(|c| matches!(c, '•' | '●' | '▪' | '◦' | '·' | '○' | '-' | '▶' | '➤'))
                .unwrap_or(false)
                || looks_like_symbolic_font(&run.style.font_name)
        })
        .cloned()
        .collect();

    if candidates.is_empty() {
        return None;
    }

    // advance 以 body anchor 为基准（与 split_editor_session 的 marker_advance 同语义），
    // 而非 marker→body 间距：保证编辑重排时 marker 锚定在 anchor 左侧不漂移。
    let advance = (body_origin_x - body_session.anchor_bbox.left).max(0.0);
    let text: String = candidates.iter().map(|r| r.text.clone()).collect();
    let kind = derive_list_text_semantics(&text).kind;
    let kind = if kind == ListMarkerKind::None {
        ListMarkerKind::Bullet
    } else {
        kind
    };

    Some(ParagraphEditorMarker {
        kind,
        text,
        advance,
        runs: candidates,
    })
}

/// Resolve a list-marker split for `full_session` using a three-step strategy chain:
///   1. Text semantics (e.g. `"1. "`, `"• "`),
///   2. Symbolic-font run detection (Wingdings/Symbol runs are usually markers),
///   3. Geometric synthesis from sibling runs (left-of-body candidates on the same line).
///
/// Strategy 1 and 2 produce a char-start offset that drives `split_editor_session`;
/// strategy 3 runs after the split to fill in a marker when the first two produced none.
/// If no strategy yields a marker, the session is returned unsplit.
pub(super) fn resolve_marker_split(
    paragraph: &GlyphPaintParagraph,
    full_session: &ParagraphEditContext,
    full_source_text: &str,
    full_text_plan: &EditorSessionTextPlan,
) -> SessionSplit {
    let semantics = derive_list_text_semantics(full_source_text);
    dbg_event(
        "document-plan.marker-detect",
        "start",
        vec![
            dbg_field("paragraphId", full_session.paragraph.id.as_str()),
            dbg_field("hasMarker", semantics.has_marker),
            dbg_field("bodyCharStart", semantics.body_char_start),
            dbg_field("runCount", full_session.paragraph.runs.len()),
            dbg_field("fullTextLen", full_source_text.len()),
        ],
    );

    // Strategies 1 & 2: both yield (body_char_start, marker_kind); strategy 3 is post-split.
    let strategy_result: Option<(usize, ListMarkerKind)> =
        if semantics.has_marker && semantics.body_char_start > 0 {
            Some((semantics.body_char_start, semantics.kind))
        } else {
            detect_symbolic_font_marker(full_session)
        };

    let default_split = || SessionSplit {
        body_session: full_session.clone(),
        marker: None,
    };

    let mut split = match strategy_result {
        Some((body_char_start, marker_kind)) => {
            let raw = full_text_plan.map_reconstructed_to_raw(body_char_start);
            split_editor_session(full_session, raw, marker_kind).unwrap_or_else(default_split)
        }
        None => default_split(),
    };

    // Strategy 3: geometric synthesis fills a missing marker after the split.
    if split.marker.is_none() {
        if let Some(marker) = synthesize_marker_from_paragraph(paragraph, &split.body_session) {
            split.marker = Some(marker);
        }
    }

    dbg_event(
        "document-plan.marker-split",
        "result",
        vec![
            dbg_field("paragraphId", full_session.paragraph.id.as_str()),
            dbg_field("markerPresent", split.marker.is_some()),
            dbg_field(
                "markerText",
                split.marker.as_ref().map(|m| m.text.as_str()).unwrap_or(""),
            ),
            dbg_field("bodyRunCount", split.body_session.paragraph.runs.len()),
        ],
    );

    split
}
