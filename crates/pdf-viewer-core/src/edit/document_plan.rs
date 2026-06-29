//! 编辑器文档计划 — 数据结构与纯构建函数。

pub mod marker;
#[cfg(test)]
mod tests;

pub use marker::{
    bbox_from_runs, resolve_marker_split, split_editor_session, split_run, ParagraphEditorMarker,
    SessionSplit,
};

use crate::edit::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use crate::edit::edit_target::{
    collect_edit_targets_from_session, resolve_edit_target_from_session, EditorEditTarget,
};
use crate::edit::source_runs::{resolve_preferred_editor_session, target_paint_runs};
use crate::edit::source_text::session_source_text;
use crate::models::{
    BoundingBox, GlyphPaintParagraph, GlyphPaintRun, LayoutParagraph, LayoutRun,
    ParagraphEditContext, VectorPageModel,
};
use crate::text::glyph_layout::{
    build_editor_session_text_plan, is_decorative_text, EditorSessionTextPlan,
};
use crate::text::list_semantics::{derive_list_text_semantics, ListMarkerKind, ListTextSemantic};
use crate::typography::font_resolver::looks_like_symbolic_font;
use serde::{Deserialize, Serialize};

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

pub fn build_editor_document_plan_from_session(session: &ParagraphEditContext) -> EditContext {
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

fn same_document_line(reference_origin_y: f32, run: &LayoutRun) -> bool {
    let tolerance = (run.style.font_size * 0.45).max(2.0);
    (reference_origin_y - run.origin_y).abs() <= tolerance
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
            dbg_field(
                "fullSourceTextCodepoints",
                codepoint_preview(full_source_text, 12),
            ),
            dbg_field("bodySourceText", body_source_text),
            dbg_field(
                "bodySourceTextCodepoints",
                codepoint_preview(body_source_text, 12),
            ),
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
