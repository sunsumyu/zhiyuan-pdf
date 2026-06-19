//! Draft layout — 编辑器 draft 渲染计划的核心入口。
//!
//! 模块拆分：
//! - `draft_types`: 核心类型定义（DraftCaretStop, DraftCaretLine, EditorDraftRenderPlan, TextDiff）
//! - `draft_text_diff`: 文本差异计算和索引映射
//! - `draft_style`: 样式构建、源布局、run 切片

use crate::common::debug::truncate_debug_text;
use crate::edit::debug_trace::{editor_debug_field as dbg_field, record_editor_debug_event as dbg_event};
use crate::edit::document_plan::{EditContext, EditorDocumentPlan};
use crate::geometry::layout_engine::{layout_paragraph, ParagraphLayout, VisualLine};
use crate::models::{LayoutParagraph, LayoutRun};

#[path = "draft_style.rs"]
mod draft_style;
#[path = "draft_text_diff.rs"]
mod draft_text_diff;
#[path = "draft_types.rs"]
mod draft_types;

pub use draft_types::{DraftCaretLine, DraftCaretStop, EditorDraftRenderPlan};

use draft_style::{
    build_draft_paragraph_with_policy, build_source_layout, paragraph_preserve_underline,
    resolve_draft_template_run, resolve_template, shell_width, source_baseline_y,
};
use draft_text_diff::{body_runs_match_source_text, remap_caret_indices_to_draft_space};

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

fn rebuild_layout_pipeline<F>(
    paragraph: LayoutParagraph,
    document_plan: &EditorDocumentPlan,
    draft_text: &str,
    measure_width: &F,
) -> EditorDraftRenderPlan
where
    F: Fn(&str, &LayoutRun) -> f32,
{
    let mut layout = layout_paragraph(&paragraph, paragraph.wrap_width, measure_width);
    align_layout_baseline(&mut layout, source_baseline_y(document_plan));
    let mut caret_lines = build_editor_draft_caret_plan_from_layout(&layout, measure_width);
    remap_caret_indices_to_draft_space(&mut caret_lines, document_plan, draft_text);
    EditorDraftRenderPlan { layout, caret_lines }
}

fn trace_render_plan(
    action: &str,
    paragraph_id: &str,
    draft_text: &str,
    body_text: &str,
    plan: &EditorDraftRenderPlan,
) {
    dbg_event(
        "render-plan",
        action,
        vec![
            dbg_field("paragraphId", paragraph_id),
            dbg_field("draftText", draft_text),
            dbg_field("bodyText", body_text),
            dbg_field("lineSummary", summarize_render_plan_lines(plan)),
            dbg_field("visualLineCount", plan.layout.lines.len()),
            dbg_field("caretLineCount", plan.caret_lines.len()),
            dbg_field(
                "caretStopCount",
                plan.caret_lines
                    .iter()
                    .map(|l| l.stops.len())
                    .sum::<usize>(),
            ),
        ],
    );
}

/// 构建 draft 渲染计划 — 编辑器 active editing 模式的核心入口。
pub fn build_draft_render_plan<F>(
    document_plan: &EditorDocumentPlan,
    draft_text: &str,
    measure_width: F,
) -> EditorDraftRenderPlan
where
    F: Fn(&str, &LayoutRun) -> f32,
{
    if draft_text == document_plan.source_body_text() && body_runs_match_source_text(document_plan)
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

/// 构建 persisted overlay 渲染计划 — 用于提交/持久化编辑后的渲染。
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
    let plan = rebuild_layout_pipeline(paragraph, document_plan, draft_text, &measure_width);
    trace_render_plan(
        "persisted-overlay-uniform-layout",
        &document_plan.body_session.paragraph.id,
        draft_text,
        &document_plan.source_body_text(),
        &plan,
    );

    plan
}

#[cfg(test)]
mod tests {
    use super::{
        build_draft_render_plan, build_persisted_overlay_render_plan, build_source_layout,
    };
    use crate::edit::document_plan::EditContext;
    use crate::models::{BoundingBox, LayoutParagraph, LayoutRun, ParagraphEditContext, RunStyle};
    use crate::text::glyph_layout::build_editor_session_text_plan;

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
    fn sanitizes_underlines() {
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
    fn renders_compact_runs() {
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
    fn preserves_active_geometry() {
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
    fn preserves_overlay_geometry() {
        // 同上：persisted/commit overlay 与 active editing 共享同一布局逻辑，
        // 必须保留未修改前后缀的 PDF 度量，避免提交后字体/字距漂移。
        let document_plan = changed_text_document_plan();
        let draft_text = "智能合约: Anchor Framwork, Solana Program Library (SPL), ERC-20/721";

        let plan = build_persisted_overlay_render_plan(&document_plan, draft_text, |text, run| {
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
    fn preserves_origins() {
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
        let expected_compact = "智能合约:AnchorFramwork,SolanaProgramLibrary(SPL),ERC-20/721";
        assert_eq!(rendered_text(&plan), expected_compact);
        assert!(
            plan_has_source_char_origins(&plan),
            "compact-PDF (synthetic-space) scenario must still preserve PDF char_origins \
             for unchanged prefix/suffix via source→runs index mapping"
        );
    }

    #[test]
    fn keeps_split_word_geometry() {
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
    fn maps_synthetic_spaces() {
        // source_text has synthesized spaces; runs_text is compact PDF text.
        let source = "编程语言: Rust"; // 10 chars (space after colon)
        let runs = "编程语言:Rust"; // 9 chars (no space)
        let (_, inv) = super::draft_text_diff::build_index_map(source, runs);
        // inv[0]=0(编), inv[1]=1(程), inv[2]=2(语), inv[3]=3(言),
        // inv[4]=4(:), inv[5]=6(R, skips space@5), inv[6]=7(u), inv[7]=8(s), inv[8]=9(t)
        // inv[9]=10 (end sentinel = source.chars().count())
        assert_eq!(inv, vec![0, 1, 2, 3, 4, 6, 7, 8, 9, 10]);
    }

    /// Regression: 删除 draft 中的字符后，runs 仍包含被删字符。
    /// 此前实现会让 source_cursor 越过 source_len 后无界递增，
    /// 导致 inverse 表里出现 > source_len 的非法值（caret 跳到末尾之外）。
    #[test]
    fn clamps_missing_source_chars() {
        // 模拟：draft 已被删除最后两个字符 ('s', 't')，但 runs 仍是完整 "Rust"。
        let source = "Ru"; // 2 chars
        let runs = "Rust"; // 4 chars
        let (_, inv) = super::draft_text_diff::build_index_map(source, runs);
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