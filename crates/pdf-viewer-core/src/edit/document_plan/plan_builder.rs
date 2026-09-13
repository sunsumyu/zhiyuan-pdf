//! 构建入口：build_editor_document_plan* / collect_* 与 open-caret 追踪。

use crate::edit::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use crate::edit::edit_target::{
    collect_edit_targets_from_session, resolve_edit_target_from_session, EditorEditTarget,
};
use crate::edit::source_runs::{original_paint_runs_for_target, resolve_preferred_editor_session};
use crate::edit::source_text::session_source_text;
use crate::models::{BoundingBox, GlyphPaintParagraph, ParagraphEditContext, VectorPageModel};
use crate::text::glyph_layout::build_editor_session_text_plan;
use crate::text::list_semantics::{derive_list_text_semantics, ListMarkerKind, ListTextSemantic};
use crate::typography::font_resolver::looks_like_symbolic_font;

use super::geometry::detect_graphic_markers;
use super::line_plans::{build_body_line_plans, select_draft_template_run};
use super::session_split::{resolve_marker_split, resolve_shell_bbox};
use super::EditorDocumentPlan;

pub fn build_editor_document_plan_from_session(
    session: &ParagraphEditContext,
) -> EditorDocumentPlan {
    let body_text_plan = build_editor_session_text_plan(session);
    let body_lines = build_body_line_plans(session, &body_text_plan);
    let draft_template_run = select_draft_template_run(session, &body_lines);
    EditorDocumentPlan {
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
        graphic_markers: Vec::new(),
        original_runs: Vec::new(),
    }
}

pub fn build_editor_document_plan(
    paragraph: &GlyphPaintParagraph,
    vector_model: Option<&VectorPageModel>,
    click_page_point: Option<(f32, f32)>,
) -> Option<EditorDocumentPlan> {
    build_editor_document_plan_for_target(paragraph, vector_model, &paragraph.id, click_page_point)
}

pub fn collect_editor_document_target_plans(
    paragraph: &GlyphPaintParagraph,
    vector_model: Option<&VectorPageModel>,
) -> Vec<EditorDocumentPlan> {
    let full_session = resolve_preferred_editor_session(paragraph, vector_model)
        .unwrap_or_else(|| paragraph.editor_session.clone());
    collect_edit_targets_from_session(&paragraph.id, &full_session)
        .into_iter()
        .filter_map(|target| {
            build_plan_for_target_session(paragraph, &full_session, target, vector_model, None)
        })
        .collect()
}

pub fn build_editor_document_plan_for_target(
    paragraph: &GlyphPaintParagraph,
    vector_model: Option<&VectorPageModel>,
    target_id: &str,
    click_page_point: Option<(f32, f32)>,
) -> Option<EditorDocumentPlan> {
    let full_session = resolve_preferred_editor_session(paragraph, vector_model)
        .unwrap_or_else(|| paragraph.editor_session.clone());
    let target =
        resolve_edit_target_from_session(&paragraph.id, target_id, &full_session, click_page_point);

    build_plan_for_target_session(
        paragraph,
        &full_session,
        target,
        vector_model,
        click_page_point,
    )
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
/// this is pure observability (cross-cutting concern). The 12-parameter signature is
/// deliberate: a parameter struct would obscure the direct key->value mapping of the
/// emitted trace fields.
#[allow(clippy::too_many_arguments)]
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

fn build_plan_for_target_session(
    paragraph: &GlyphPaintParagraph,
    _full_session: &ParagraphEditContext,
    target: EditorEditTarget,
    vector_model: Option<&VectorPageModel>,
    click_page_point: Option<(f32, f32)>,
) -> Option<EditorDocumentPlan> {
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
    let preliminary_shell_bbox = resolve_shell_bbox(&full_session, &split, &[]);
    let graphic_markers =
        detect_graphic_markers(vector_model, &split.body_session, &preliminary_shell_bbox);
    let shell_bbox = resolve_shell_bbox(&full_session, &split, &graphic_markers);

    let body_lines = build_body_line_plans(&split.body_session, &body_text_plan);
    let draft_template_run = select_draft_template_run(&split.body_session, &body_lines);
    // Caret 解析的唯一权威路径在 UI 层 `editor_controller::open_editor_at_page_point`
    // 通过 `active_caret_index_at_page_point`（与 Move 路径共用 `build_unified_draft_caret_lines`）
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

    let original_runs = original_paint_runs_for_target(paragraph, &split.body_session, &target);

    Some(EditorDocumentPlan {
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
        graphic_markers,
        original_runs,
    })
}
