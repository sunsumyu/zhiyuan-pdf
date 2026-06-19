//! 编辑器文档计划 — 数据结构与纯构建函数。

use crate::edit::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use crate::edit::edit_target::{
    collect_edit_targets_from_session, resolve_edit_target_from_session, EditorEditTarget,
};
use crate::edit::source_runs::{target_paint_runs, resolve_preferred_editor_session};
use crate::edit::source_text::session_source_text;
use crate::geometry::source_geometry::compute_bbox_from_runs;
use crate::models::{
    BoundingBox, GlyphPaintParagraph, GlyphPaintRun, LayoutParagraph, LayoutRun,
    ParagraphEditContext, VectorPageModel,
};
use crate::text::glyph_layout::{
    build_editor_session_text_plan, infer_run_advance, is_decorative_text, EditorSessionTextPlan,
};
use crate::text::list_semantics::{derive_list_text_semantics, ListMarkerKind, ListTextSemantic};
use crate::typography::font_resolver::looks_like_symbolic_font;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ParagraphEditorMarker {
    pub kind: ListMarkerKind,
    pub text: String,
    pub advance: f32,
    #[serde(default)]
    pub runs: Vec<LayoutRun>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditContext {
    #[serde(default)]
    pub target_id: String,
    #[serde(default)]
    pub base_paragraph_id: String,
    pub shell_bbox: BoundingBox,
    pub body_session: ParagraphEditContext,
    pub source_body_text: String,
    pub body_text_plan: EditorSessionTextPlan,
    #[serde(default)]
    pub draft_template_run: LayoutRun,
    #[serde(default)]
    pub body_lines: Vec<EditorDocumentLinePlan>,
    #[serde(default)]
    pub body_initial_caret: usize,
    #[serde(default)]
    pub marker: Option<ParagraphEditorMarker>,
    #[serde(default)]
    pub original_runs: Vec<GlyphPaintRun>,
}

impl Default for EditContext {
    fn default() -> Self {
        Self {
            target_id: String::new(),
            base_paragraph_id: String::new(),
            shell_bbox: BoundingBox::default(),
            body_session: ParagraphEditContext {
                anchor_bbox: BoundingBox::default(),
                paragraph: LayoutParagraph::default(),
            },
            source_body_text: String::new(),
            body_text_plan: EditorSessionTextPlan::default(),
            draft_template_run: LayoutRun::default(),
            body_lines: Vec::new(),
            body_initial_caret: 0,
            marker: None,
            original_runs: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EditorDocumentLinePlan {
    #[serde(default)]
    pub template_runs: Vec<LayoutRun>,
    #[serde(default)]
    pub source_runs: Vec<LayoutRun>,
    #[serde(default)]
    pub reconstructed_char_count: usize,
}

impl EditContext {
    pub fn source_body_text(&self) -> &str {
        &self.source_body_text
    }

    pub fn body_char_count(&self) -> usize {
        self.source_body_text.chars().count()
    }
}

// ── Build functions ─────────────────────────────────────────────

fn resolve_shell_bbox(target_session: &ParagraphEditContext, split: &SessionSplit) -> BoundingBox {
    if let Some(marker) = split.marker.as_ref() {
        let mut shell_bbox = split.body_session.anchor_bbox;
        if let Some(marker_bbox) = bbox_from_runs(&marker.runs) {
            shell_bbox.left = shell_bbox.left.min(marker_bbox.left);
            shell_bbox.top = shell_bbox.top.min(marker_bbox.top);
            shell_bbox.right = shell_bbox.right.max(marker_bbox.right);
            shell_bbox.bottom = shell_bbox.bottom.max(marker_bbox.bottom);
        }
        return shell_bbox;
    }

    target_session.anchor_bbox
}

pub fn build_editor_document_plan_from_session(
    session: &ParagraphEditContext,
) -> EditContext {
    let body_text_plan = build_editor_session_text_plan(session);
    let body_lines = build_body_line_plans(session, &body_text_plan);
    let draft_template_run = select_draft_template_run(session, &body_lines);
    EditContext {
        target_id: session.paragraph.id.clone(),
        base_paragraph_id: session.paragraph.id.clone(),
        shell_bbox: session.anchor_bbox,
        body_session: session.clone(),
        source_body_text: session_source_text(session),
        body_text_plan,
        draft_template_run,
        body_lines,
        body_initial_caret: 0,
        marker: None,
        original_runs: Vec::new(),
    }
}

#[derive(Debug, Clone)]
struct SessionSplit {
    body_session: ParagraphEditContext,
    marker: Option<ParagraphEditorMarker>,
}

fn split_run_at_char_index(
    run: &LayoutRun,
    split_index: usize,
) -> (Option<LayoutRun>, Option<LayoutRun>) {
    let chars: Vec<char> = run.text.chars().collect();
    if split_index == 0 {
        return (None, Some(run.clone()));
    }
    if split_index >= chars.len() {
        return (Some(run.clone()), None);
    }

    let x_offset = if split_index < run.char_origins.len() {
        run.char_origins[split_index]
    } else {
        infer_run_advance(run) * split_index as f32
    };

    let mut marker_run = run.clone();
    marker_run.text = chars[..split_index].iter().collect();
    marker_run.char_origins = run.char_origins.iter().take(split_index).copied().collect();
    marker_run.char_widths = run.char_widths.iter().take(split_index).copied().collect();
    marker_run.bbox.right = (run.bbox.left + x_offset).max(marker_run.bbox.left);

    let mut body_run = run.clone();
    body_run.text = chars[split_index..].iter().collect();
    body_run.origin_x += x_offset;
    body_run.bbox.left = (run.bbox.left + x_offset).min(body_run.bbox.right);
    let base_origin = run
        .char_origins
        .get(split_index)
        .copied()
        .unwrap_or(x_offset);
    body_run.char_origins = run
        .char_origins
        .iter()
        .skip(split_index)
        .map(|origin| origin - base_origin)
        .collect();
    body_run.char_widths = run.char_widths.iter().skip(split_index).copied().collect();

    (Some(marker_run), Some(body_run))
}

fn bbox_from_runs(runs: &[LayoutRun]) -> Option<BoundingBox> {
    if let Some(source_bbox) = compute_bbox_from_runs(runs) {
        return Some(source_bbox);
    }
    let first = runs.first()?;
    let mut bbox = first.bbox;
    for run in runs.iter().skip(1) {
        bbox.left = bbox.left.min(run.bbox.left);
        bbox.top = bbox.top.min(run.bbox.top);
        bbox.right = bbox.right.max(run.bbox.right);
        bbox.bottom = bbox.bottom.max(run.bbox.bottom);
    }
    Some(bbox)
}

fn select_draft_template_run(
    session: &ParagraphEditContext,
    body_lines: &[EditorDocumentLinePlan],
) -> LayoutRun {
    let source_candidate = body_lines
        .iter()
        .flat_map(|line| line.source_runs.iter())
        .find(|run| {
            !run.text.trim().is_empty()
                && !is_decorative_text(&run.text)
                && !looks_like_symbolic_font(&run.style.font_name)
        });
    if let Some(run) = source_candidate {
        return run.cleared_style(false, false, false);
    }

    let template_candidate = body_lines
        .iter()
        .flat_map(|line| line.template_runs.iter())
        .find(|run| {
            !run.text.trim().is_empty()
                && !is_decorative_text(&run.text)
                && !looks_like_symbolic_font(&run.style.font_name)
        });
    if let Some(run) = template_candidate {
        return run.cleared_style(false, false, false);
    }

    if let Some(run) = session
        .paragraph
        .runs
        .iter()
        .find(|run| !run.text.trim().is_empty())
    {
        return run.cleared_style(false, false, false);
    }

    let mut run = LayoutRun::default();
    run.id = format!("editor-draft-template-{}", session.paragraph.id);
    run.style.font_size = 12.0;
    run
}

fn split_editor_session(
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

fn same_document_line(reference_origin_y: f32, run: &LayoutRun) -> bool {
    let tolerance = (run.style.font_size * 0.45).max(2.0);
    (reference_origin_y - run.origin_y).abs() <= tolerance
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

fn synthesize_marker_from_paragraph(
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

    let bbox = bbox_from_runs(&candidates)?;
    let advance = (body_origin_x - bbox.left).max(0.0);
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

fn build_body_line_plans(
    session: &ParagraphEditContext,
    _text_plan: &EditorSessionTextPlan,
) -> Vec<EditorDocumentLinePlan> {
    let mut rebuilt_lines: Vec<EditorDocumentLinePlan> = Vec::new();
    let mut raw_line_start = 0usize;
    let mut current_runs: Vec<LayoutRun> = Vec::new();
    let mut current_source_runs: Vec<LayoutRun> = Vec::new();
    let mut current_origin_y: Option<f32> = None;
    let mut raw_consumed_again = 0usize;
    for run in session
        .paragraph
        .runs
        .iter()
        .filter(|run| !run.text.is_empty())
    {
        let glyph_count = run.text.chars().count();
        if let Some(origin_y) = current_origin_y {
            if !same_document_line(origin_y, run) {
                rebuilt_lines.push(EditorDocumentLinePlan {
                    template_runs: std::mem::take(&mut current_runs),
                    source_runs: std::mem::take(&mut current_source_runs),
                    reconstructed_char_count: raw_consumed_again.saturating_sub(raw_line_start),
                });
                raw_line_start = raw_consumed_again;
            }
        }
        current_origin_y = Some(run.origin_y);
        current_runs.push(run.cleared_style(false, true, false));
        current_source_runs.push(run.clone());
        raw_consumed_again += glyph_count;
    }
    if !current_runs.is_empty() {
        rebuilt_lines.push(EditorDocumentLinePlan {
            template_runs: current_runs,
            source_runs: current_source_runs,
            reconstructed_char_count: raw_consumed_again.saturating_sub(raw_line_start),
        });
    }
    rebuilt_lines
}

/// 从段落创建编辑上下文（默认入口）。
pub fn from_paragraph(
    paragraph: &GlyphPaintParagraph,
    vector_model: Option<&VectorPageModel>,
    click_page_point: Option<(f32, f32)>,
) -> Option<EditContext> {
    from_target_id(paragraph, vector_model, &paragraph.id, click_page_point)
}

/// 收集段落所有可编辑区域的上下文。
pub fn collect_all(
    paragraph: &GlyphPaintParagraph,
    vector_model: Option<&VectorPageModel>,
) -> Vec<EditContext> {
    let full_session = resolve_preferred_editor_session(paragraph, vector_model)
        .unwrap_or_else(|| paragraph.editor_session.clone());
    collect_edit_targets_from_session(&paragraph.id, &full_session)
        .into_iter()
        .filter_map(|target| resolve_from_target(paragraph, &full_session, target, None))
        .collect()
}

/// 按 target_id 创建编辑上下文。
pub fn from_target_id(
    paragraph: &GlyphPaintParagraph,
    vector_model: Option<&VectorPageModel>,
    target_id: &str,
    click_page_point: Option<(f32, f32)>,
) -> Option<EditContext> {
    let full_session = resolve_preferred_editor_session(paragraph, vector_model)
        .unwrap_or_else(|| paragraph.editor_session.clone());
    let target =
        resolve_edit_target_from_session(&paragraph.id, target_id, &full_session, click_page_point);

    resolve_from_target(paragraph, &full_session, target, click_page_point)
}

/// Format up to `limit` codepoints of `text` as `U+XXXX(char)` for diagnostics.
fn codepoint_preview(text: &str, limit: usize) -> String {
    text.chars()
        .take(limit)
        .map(|c| format!("U+{:04X}({})", c as u32, c))
        .collect::<Vec<_>>()
        .join(",")
}

/// Resolve a list-marker split for `full_session` using a three-step strategy chain:
///   1. Text semantics (e.g. `"1. "`, `"• "`),
///   2. Symbolic-font run detection (Wingdings/Symbol runs are usually markers),
///   3. Geometric synthesis from sibling runs (left-of-body candidates on the same line).
///
/// Strategy 1 and 2 produce a char-start offset that drives `split_editor_session`;
/// strategy 3 runs after the split to fill in a marker when the first two produced none.
/// If no strategy yields a marker, the session is returned unsplit.
fn resolve_marker_split(
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
            let raw = full_text_plan.to_raw(body_char_start);
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

/// Emit the verbose `open-caret.resolved` trace used when the editor is opened at a click point.
/// Kept separate from `build_plan_for_target_session` so the business logic reads linearly;
/// this is pure observability (cross-cutting concern).
fn trace_open_caret_resolved(
    paragraph: &GlyphPaintParagraph,
    target_id: &str,
    base_paragraph_id: &str,
    full_source_text: &str,
    body_source_text: &str,
    full_session: &ParagraphEditContext,
    semantics: &ListTextSemantic,
    full_caret: usize,
    body_initial_caret: usize,
    shell_bbox: &BoundingBox,
    body_bbox: &BoundingBox,
    click_page_point: Option<(f32, f32)>,
) {
    dbg_event(
        "document-plan.open-caret",
        "resolved",
        vec![
            dbg_field("paragraphId", paragraph.id.as_str()),
            dbg_field("targetId", target_id),
            dbg_field("baseParagraphId", base_paragraph_id),
            dbg_field("fullSourceText", full_source_text),
            dbg_field("fullSourceTextCodepoints", codepoint_preview(full_source_text, 12)),
            dbg_field("bodySourceText", body_source_text),
            dbg_field("bodySourceTextCodepoints", codepoint_preview(body_source_text, 12)),
            dbg_field(
                "runOrder",
                full_session
                    .paragraph
                    .runs
                    .iter()
                    .take(8)
                    .map(|r| {
                        format!(
                            "[x={:.1},y={:.1},'{}']",
                            r.origin_x,
                            r.origin_y,
                            r.text.chars().take(6).collect::<String>()
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(" "),
            ),
            dbg_field("hasMarker", semantics.has_marker),
            dbg_field("bodyCharStart", semantics.body_char_start),
            dbg_field("fullCaret", full_caret),
            dbg_field("bodyCaret", body_initial_caret),
            dbg_field(
                "shellBBox",
                format!(
                    "[{:.2},{:.2},{:.2},{:.2}]",
                    shell_bbox.left, shell_bbox.top, shell_bbox.right, shell_bbox.bottom
                ),
            ),
            dbg_field("shellLeft", shell_bbox.left),
            dbg_field("shellTop", shell_bbox.top),
            dbg_field("shellRight", shell_bbox.right),
            dbg_field("shellBottom", shell_bbox.bottom),
            dbg_field(
                "bodyBBox",
                format!(
                    "[{:.2},{:.2},{:.2},{:.2}]",
                    body_bbox.left, body_bbox.top, body_bbox.right, body_bbox.bottom
                ),
            ),
            dbg_field("bodyLeft", body_bbox.left),
            dbg_field("bodyTop", body_bbox.top),
            dbg_field("bodyRight", body_bbox.right),
            dbg_field("bodyBottom", body_bbox.bottom),
            dbg_field(
                "clickPageX",
                click_page_point
                    .map(|(x, _)| x.to_string())
                    .unwrap_or_else(|| "none".to_string()),
            ),
            dbg_field(
                "clickPageY",
                click_page_point
                    .map(|(_, y)| y.to_string())
                    .unwrap_or_else(|| "none".to_string()),
            ),
        ],
    );
}

fn resolve_from_target(
    paragraph: &GlyphPaintParagraph,
    _full_session: &ParagraphEditContext,
    target: EditorEditTarget,
    click_page_point: Option<(f32, f32)>,
) -> Option<EditContext> {
    let target_id = target.target_id.clone();
    let base_paragraph_id = target.base_paragraph_id.clone();
    let full_session = target.session.clone();
    let full_source_text = session_source_text(&full_session);
    let full_text_plan = build_editor_session_text_plan(&full_session);
    if full_source_text.trim().is_empty() {
        return None;
    }

    // Marker resolution is a three-step strategy chain (semantics -> symbol-font -> geometric
    // synthesis); the chain and its trace events live in `resolve_marker_split`.
    let split = resolve_marker_split(paragraph, &full_session, &full_source_text, &full_text_plan);

    let body_text_plan = build_editor_session_text_plan(&split.body_session);
    let source_body_text = session_source_text(&split.body_session);
    let shell_bbox = resolve_shell_bbox(&full_session, &split);

    let body_lines = build_body_line_plans(&split.body_session, &body_text_plan);
    let draft_template_run = select_draft_template_run(&split.body_session, &body_lines);
    // Caret 解析的唯一权威路径在 UI 层 `editor_controller::open_at_point`
    // 通过 `caret_at_page_point`（与 Move 路径共用 `build_unified_draft_caret_lines`）
    // 计算并覆盖此处的初始值。这里保留为 0，避免出现"core 用旧算法算一遍 + UI 再覆盖"
    // 的双轨制，根除首次点击 caret 偏差。click_page_point 仍用于 segment 选择。
    let body_initial_caret = 0usize;
    let full_caret = body_initial_caret;

    if click_page_point.is_some() {
        let semantics = derive_list_text_semantics(&full_source_text);
        trace_open_caret_resolved(
            paragraph,
            &target_id,
            &base_paragraph_id,
            &full_source_text,
            &source_body_text,
            &full_session,
            &semantics,
            full_caret,
            body_initial_caret,
            &shell_bbox,
            &split.body_session.anchor_bbox,
            click_page_point,
        );
    }

    let original_runs = target_paint_runs(paragraph, &split.body_session, &target);

    Some(EditContext {
        target_id,
        base_paragraph_id,
        shell_bbox,
        body_session: split.body_session,
        source_body_text,
        body_text_plan,
        draft_template_run,
        body_lines,
        body_initial_caret,
        marker: split.marker.map(|mut marker| {
            if marker.kind == ListMarkerKind::None && looks_like_symbolic_font(&marker.text) {
                marker.kind = ListMarkerKind::Symbol;
            }
            marker
        }),
        original_runs,
    })
}

// --- 兼容别名（deprecated）---
// 保留一个周期后删除

/// Deprecated: use [`EditContext`] instead.
#[deprecated(since = "2026.6", note = "Use EditContext instead")]
pub type EditorDocumentPlan = EditContext;

/// Deprecated: use [`from_paragraph`] instead.
#[deprecated(since = "2026.6", note = "Use from_paragraph instead")]
pub fn build_editor_document_plan(
    paragraph: &GlyphPaintParagraph,
    vector_model: Option<&VectorPageModel>,
    click_page_point: Option<(f32, f32)>,
) -> Option<EditContext> {
    from_paragraph(paragraph, vector_model, click_page_point)
}

/// Deprecated: use [`from_target_id`] instead.
#[deprecated(since = "2026.6", note = "Use from_target_id instead")]
pub fn build_editor_document_plan_for_target(
    paragraph: &GlyphPaintParagraph,
    vector_model: Option<&VectorPageModel>,
    target_id: &str,
    click_page_point: Option<(f32, f32)>,
) -> Option<EditContext> {
    from_target_id(paragraph, vector_model, target_id, click_page_point)
}

/// Deprecated: use [`collect_all`] instead.
#[deprecated(since = "2026.6", note = "Use collect_all instead")]
pub fn collect_editor_document_target_plans(
    paragraph: &GlyphPaintParagraph,
    vector_model: Option<&VectorPageModel>,
) -> Vec<EditContext> {
    collect_all(paragraph, vector_model)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        EditorControlStyle, FontSourceKind, PaintMode, ParagraphStyle, ResolvedFontFace,
        ResolvedFontIdentity, RunStyle, SemanticRole, StyledRun, SymbolClass, VectorRenderObject,
        VectorTextObject,
    };
    use crate::text::glyph_layout::build_editor_session_text_plan;

    const CANONICAL_MIXED_TEXT: &str =
        "智能合约: Anchor Framework, Solana Program Library (SPL), ERC-20/721";

    fn test_style() -> RunStyle {
        RunStyle {
            font_name: "Microsoft YaHei".to_string(),
            font_size: 10.0,
            color: "#000000".to_string(),
            is_bold: false,
            is_italic: false,
            is_underline: false,
            char_spacing: 0.0,
            scale_x: 1.0,
        }
    }

    fn test_bbox(left: f32, width: f32) -> BoundingBox {
        BoundingBox {
            left,
            top: 40.0,
            right: left + width,
            bottom: 52.0,
        }
    }

    fn test_layout_run(id: &str, text: &str, left: f32, width: f32) -> LayoutRun {
        LayoutRun {
            id: id.to_string(),
            text: text.to_string(),
            style: test_style(),
            bbox: test_bbox(left, width),
            origin_x: left,
            origin_y: 50.0,
            char_origins: Vec::new(),
            char_widths: Vec::new(),
            object_ids: vec!["obj-1".to_string()],
            object_indices: vec![0],
        }
    }

    fn layout_with_gaps(
        id: &str,
        text: &str,
        left: f32,
        origins: Vec<f32>,
        widths: Vec<f32>,
    ) -> LayoutRun {
        let right = origins
            .iter()
            .zip(widths.iter())
            .map(|(origin, width)| origin + width)
            .fold(left, f32::max);
        let mut run = test_layout_run(id, text, left, (right - left).max(1.0));
        run.char_origins = origins;
        run.char_widths = widths;
        run
    }

    fn session_from_runs(runs: Vec<LayoutRun>) -> ParagraphEditContext {
        let anchor_bbox = runs.iter().fold(
            BoundingBox {
                left: f32::INFINITY,
                top: f32::INFINITY,
                right: f32::NEG_INFINITY,
                bottom: f32::NEG_INFINITY,
            },
            |acc, run| BoundingBox {
                left: acc.left.min(run.bbox.left),
                top: acc.top.min(run.bbox.top),
                right: acc.right.max(run.bbox.right),
                bottom: acc.bottom.max(run.bbox.bottom),
            },
        );

        ParagraphEditContext {
            anchor_bbox,
            paragraph: LayoutParagraph {
                id: "p1".to_string(),
                bbox: anchor_bbox,
                origin_x: anchor_bbox.left,
                origin_y: anchor_bbox.top,
                wrap_width: (anchor_bbox.right - anchor_bbox.left).max(1.0),
                runs,
                ..Default::default()
            },
        }
    }

    fn mixed_runs() -> Vec<LayoutRun> {
        vec![
            test_layout_run("r0", "智能合约: ", 0.0, 50.0),
            test_layout_run("r1", "A", 50.0, 5.0),
            test_layout_run("r2", "nchor", 58.0, 25.0),
            test_layout_run("r3", " ", 83.0, 4.0),
            test_layout_run("r4", "Fram", 87.0, 20.0),
            test_layout_run("r5", "ew", 110.0, 10.0),
            test_layout_run("r6", "ork", 123.0, 15.0),
            test_layout_run("r7", ", ", 138.0, 6.0),
            test_layout_run("r8", "S", 144.0, 5.0),
            test_layout_run("r9", "olana Program Library (", 152.0, 110.0),
            test_layout_run("r10", "S", 262.0, 5.0),
            test_layout_run("r11", "PL)", 270.0, 15.0),
            test_layout_run("r12", ", ER", 285.0, 20.0),
            test_layout_run("r13", "C", 308.0, 5.0),
            test_layout_run("r14", "-20/721", 316.0, 35.0),
        ]
    }

    #[test]
    fn preserves_canonical_source() {
        let session = session_from_runs(mixed_runs());
        let reconstructed = build_editor_session_text_plan(&session).text;

        assert!(reconstructed.contains("A nchor"));
        assert!(reconstructed.contains("Fram ew ork"));
        assert!(reconstructed.contains("S PL"));
        assert!(reconstructed.contains("ER C -20"));

        let document_plan = build_editor_document_plan_from_session(&session);

        assert_eq!(document_plan.source_body_text(), CANONICAL_MIXED_TEXT);
        assert!(!document_plan.source_body_text().contains("A nchor"));
        assert_ne!(document_plan.source_body_text(), reconstructed);
    }

    #[test]
    fn restores_visual_gaps() {
        let session = session_from_runs(vec![
            test_layout_run("r0", "智能合约:", 0.0, 46.0),
            test_layout_run("r1", "A", 51.0, 5.0),
            test_layout_run("r2", "nchor", 59.0, 25.0),
            test_layout_run("r3", "Framework,", 90.0, 58.0),
            test_layout_run("r4", "Solana", 154.0, 36.0),
            test_layout_run("r5", "Program", 196.0, 42.0),
            test_layout_run("r6", "Library", 244.0, 38.0),
            test_layout_run("r7", "(SPL),", 288.0, 32.0),
            test_layout_run("r8", "ERC-20/721", 326.0, 54.0),
        ]);

        let document_plan = build_editor_document_plan_from_session(&session);

        assert_eq!(
            document_plan.source_body_text(),
            "智能合约: Anchor Framework, Solana Program Library (SPL), ERC-20/721"
        );
        assert!(!document_plan.source_body_text().contains("A nchor"));
    }

    #[test]
    fn restores_run_spaces() {
        let text = "智能合约:AnchorFramework,SolanaProgramLibrary(SPL),ERC-20/721";
        let chars = text.chars().collect::<Vec<_>>();
        let mut x = 0.0;
        let mut origins = Vec::new();
        let mut widths = Vec::new();
        for index in 0..chars.len() {
            origins.push(x);
            let width = if chars[index].is_ascii() { 5.0 } else { 10.0 };
            widths.push(width);
            x += width;
            if index + 1 < chars.len()
                && matches!(
                    (chars[index], chars[index + 1]),
                    (':', 'A')
                        | ('r', 'F')
                        | (',', 'S')
                        | ('a', 'P')
                        | ('m', 'L')
                        | ('y', '(')
                        | (',', 'E')
                )
            {
                x += 4.0;
            }
        }
        let session = session_from_runs(vec![layout_with_gaps(
            "single-run",
            text,
            0.0,
            origins,
            widths,
        )]);

        let document_plan = build_editor_document_plan_from_session(&session);

        assert_eq!(
            document_plan.source_body_text(),
            "智能合约: Anchor Framework, Solana Program Library (SPL), ERC-20/721"
        );
        assert!(!document_plan.source_body_text().contains("A nchor"));
        assert!(!document_plan.source_body_text().contains("S PL"));
        assert!(!document_plan.source_body_text().contains("ER C"));
    }

    fn test_resolved_font() -> ResolvedFontFace {
        ResolvedFontFace {
            identity: ResolvedFontIdentity {
                raw_name: "Microsoft YaHei".to_string(),
                canonical_family: "Microsoft YaHei".to_string(),
                style_name: "Regular".to_string(),
                weight: 400,
                is_italic: false,
                symbol_class: SymbolClass::None,
                subset_stripped: false,
            },
            render_family: "Microsoft YaHei".to_string(),
            metrics_family: "Microsoft YaHei".to_string(),
            source: FontSourceKind::SystemMatched,
            confidence: 1.0,
        }
    }

    fn test_paint_run(id: &str, text: &str, left: f32, width: f32) -> GlyphPaintRun {
        GlyphPaintRun {
            id: id.to_string(),
            page_index: 0,
            region_id: "region-1".to_string(),
            paragraph_id: "p1".to_string(),
            text: text.to_string(),
            bbox: test_bbox(left, width),
            origin_x: left,
            origin_y: 50.0,
            char_origins: Vec::new(),
            color: "#000000".to_string(),
            resolved_font: test_resolved_font(),
            font_size: 10.0,
            scale_x: 1.0,
            is_bold: false,
            is_italic: false,
            is_underline: false,
            paint_mode: PaintMode::Fill,
            object_ids: vec!["obj-1".to_string()],
            object_indices: vec![0],
        }
    }

    fn test_styled_run(text: &str, left: f32, width: f32, z_index: usize) -> StyledRun {
        StyledRun {
            text: text.to_string(),
            color: "#000000".to_string(),
            tx: left,
            ty: 50.0,
            width,
            font_size: 10.0,
            font_name: "Microsoft YaHei".to_string(),
            a: 1.0,
            d: 1.0,
            horizontal_scaling: 1.0,
            z_index,
            object_id: Some("obj-1".to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn prefers_vector_source() {
        let polluted_paint_run = test_paint_run("paint-1", "智能合约: A nchor", 0.0, 90.0);
        let paint_session = session_from_runs(vec![test_layout_run(
            "paint-layout-1",
            "智能合约: A nchor",
            0.0,
            90.0,
        )]);
        let paragraph = GlyphPaintParagraph {
            id: "p1".to_string(),
            region_id: "region-1".to_string(),
            bbox: paint_session.anchor_bbox,
            style: ParagraphStyle::default(),
            editor_session: paint_session,
            control_style: EditorControlStyle::default(),
            semantic_role: SemanticRole::None,
            runs: vec![polluted_paint_run],
        };
        let vector_model = VectorPageModel {
            page_index: 0,
            width: 400.0,
            height: 200.0,
            objects: vec![VectorRenderObject::Text(VectorTextObject {
                id: "obj-1".to_string(),
                runs: vec![
                    test_styled_run("智能合约: ", 0.0, 50.0, 0),
                    test_styled_run("A", 50.0, 5.0, 1),
                    test_styled_run("nchor", 55.0, 25.0, 2),
                ],
                z_index: 0,
            })],
        };

        let document_plan =
            build_editor_document_plan_for_target(&paragraph, Some(&vector_model), "p1", None)
                .expect("document plan should use vector source");

        assert_eq!(document_plan.source_body_text(), "智能合约: Anchor");
        assert!(!document_plan.source_body_text().contains("A nchor"));
    }

    #[test]
    fn keeps_overlay_source() {
        let source_session = session_from_runs(vec![test_layout_run(
            "source-layout-1",
            "编程语言: Rust (Solana/Anchor), Solidity (Ethereum)",
            0.0,
            260.0,
        )]);
        let mut patched_display_run = test_paint_run(
            "patched-display-1",
            "编程语言: Rust (Sona/Anchor), Solidity (Ethereum)",
            0.0,
            32.0,
        );
        patched_display_run.object_ids.clear();
        patched_display_run.object_indices.clear();

        let paragraph = GlyphPaintParagraph {
            id: "p1".to_string(),
            region_id: "region-1".to_string(),
            bbox: source_session.anchor_bbox,
            style: ParagraphStyle::default(),
            editor_session: source_session,
            control_style: EditorControlStyle::default(),
            semantic_role: SemanticRole::None,
            runs: vec![patched_display_run],
        };
        let vector_model = VectorPageModel {
            page_index: 0,
            width: 400.0,
            height: 200.0,
            objects: vec![VectorRenderObject::Text(VectorTextObject {
                id: "obj-1".to_string(),
                runs: vec![test_styled_run(
                    "编程语言: Rust (Solana/Anchor), Solidity (Ethereum)",
                    0.0,
                    260.0,
                    0,
                )],
                z_index: 0,
            })],
        };

        let document_plan =
            build_editor_document_plan_for_target(&paragraph, Some(&vector_model), "p1", None)
                .expect("persisted overlay target should recover the original vector source");

        assert_eq!(
            document_plan.source_body_text(),
            "编程语言: Rust (Solana/Anchor), Solidity (Ethereum)"
        );
        assert!(
            document_plan.body_session.anchor_bbox.right >= 259.0,
            "source geometry must come from the original vector text, not the shortened patched display run"
        );
    }

    #[test]
    fn uses_vector_geometry() {
        let paint_session = session_from_runs(vec![test_layout_run(
            "paint-layout-1",
            "编程语言:Rust(Solana/Anchor),Solidity(Ethereum)",
            0.0,
            240.0,
        )]);
        let mut paint_run = test_paint_run(
            "paint-run-1",
            "编程语言:Rust(Solana/Anchor),Solidity(Ethereum)",
            0.0,
            240.0,
        );
        paint_run.object_ids.clear();
        paint_run.object_indices.clear();

        let paragraph = GlyphPaintParagraph {
            id: "p1".to_string(),
            region_id: "region-1".to_string(),
            bbox: paint_session.anchor_bbox,
            style: ParagraphStyle::default(),
            editor_session: paint_session,
            control_style: EditorControlStyle::default(),
            semantic_role: SemanticRole::None,
            runs: vec![paint_run],
        };
        let vector_model = VectorPageModel {
            page_index: 0,
            width: 400.0,
            height: 200.0,
            objects: vec![VectorRenderObject::Text(VectorTextObject {
                id: "unlinked-vector-object".to_string(),
                runs: vec![test_styled_run(
                    "编程语言: Rust (Solana/Anchor), Solidity (Ethereum)",
                    0.0,
                    260.0,
                    0,
                )],
                z_index: 0,
            })],
        };

        let document_plan =
            build_editor_document_plan_for_target(&paragraph, Some(&vector_model), "p1", None)
                .expect("geometry fallback should recover vector source");

        assert_eq!(
            document_plan.source_body_text(),
            "编程语言: Rust (Solana/Anchor), Solidity (Ethereum)"
        );
    }
}
