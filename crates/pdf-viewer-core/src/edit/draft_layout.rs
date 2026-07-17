//! Draft layout — 编辑器 draft 渲染计划的核心入口。
//!
//! 模块拆分：
//! - `draft_types`: 核心类型定义（DraftCaretStop, DraftCaretLine, DraftLayout, TextDiff）
//! - `draft_text_diff`: 文本差异计算和索引映射
//! - `draft_style`: 样式构建、源布局、run 切片
//! - `draft_init`: 空段落初始化
//! - `draft_geometry`: 几何与光标转换
//! - `draft_reflow`: 核心重排计算

#[path = "draft_geometry.rs"]
mod draft_geometry;
#[path = "draft_init.rs"]
mod draft_init;
#[path = "draft_reflow.rs"]
mod draft_reflow;
#[path = "draft_style.rs"]
mod draft_style;
#[path = "draft_text_diff.rs"]
mod draft_text_diff;
#[path = "draft_types.rs"]
mod draft_types;

pub use draft_reflow::{build_edit_layout, build_save_layout};
pub use draft_types::{DraftCaretLine, DraftCaretStop, DraftLayout};

#[cfg(test)]
mod tests {
    use super::draft_style::build_source_layout;
    use super::{build_edit_layout, build_save_layout};
    use crate::edit::document_plan::EditContext;
    use crate::models::{
        BoundingBox, LayoutAlignment, LayoutParagraph, LayoutRun, ParagraphEditContext, RunStyle,
    };
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
            font_weight_numeric: 400,
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

    fn marker_test_run(id: &str, text: &str, left: f32, width: f32) -> LayoutRun {
        let mut run = test_run(id, text, left, left + width, false);
        run.char_origins = vec![0.0; text.chars().count()];
        run.char_widths = vec![width; text.chars().count()];
        run
    }

    fn changed_text_document_plan() -> EditContext {
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
        EditContext {
            source_body_text: source_text,
            body_text_plan: build_editor_session_text_plan(&body_session),
            body_session,
            ..Default::default()
        }
    }

    fn rendered_text(plan: &super::DraftLayout) -> String {
        plan.layout
            .lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<String>()
    }

    fn plan_has_source_char_origins(plan: &super::DraftLayout) -> bool {
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
        let document_plan = EditContext {
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
        let document_plan = EditContext {
            source_body_text: "编程语言: Rust".to_string(),
            body_text_plan: build_editor_session_text_plan(&body_session),
            body_session,
            ..Default::default()
        };

        let plan = build_save_layout(
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

        let plan = build_edit_layout(&document_plan, draft_text, |text, run| {
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

        let plan = build_save_layout(&document_plan, draft_text, |text, run| {
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
        // source_text 注入合成空格 → "智能合约: Anchor Framework, ..."。
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
        let document_plan = EditContext {
            source_body_text: visual_source_text.clone(),
            body_text_plan: build_editor_session_text_plan(&body_session),
            body_session,
            ..Default::default()
        };
        // 用户在 visual 文本基础上把 "Framework" 改成 "Framwork"（删一个 e）。
        let draft_text = "智能合约: Anchor Framwork, Solana Program Library (SPL), ERC-20/721";

        let plan = build_save_layout(&document_plan, draft_text, |text, run| {
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
        let document_plan = EditContext {
            source_body_text: "Anchor".to_string(),
            body_text_plan: build_editor_session_text_plan(&body_session),
            body_session,
            ..Default::default()
        };

        let plan = build_edit_layout(&document_plan, "Anchor", |text, _run| {
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

    #[test]
    fn unified_layout_with_marker() {
        // 验证 marker + body 统一布局：marker 作为第一个 run 插入
        use crate::edit::document_plan::ParagraphEditorMarker;
        use crate::text::list_semantics::ListMarkerKind;

        let body_text = "Anchor Framework, Solana Program Library (SPL)".to_string();
        let runs = vec![test_run_with_origins("r1", &body_text, 50.0, false)];
        let body_session = ParagraphEditContext {
            anchor_bbox: BoundingBox {
                left: 40.0,
                top: 40.0,
                right: 250.0,
                bottom: 52.0,
            },
            paragraph: LayoutParagraph {
                id: "p-list-item".to_string(),
                runs,
                wrap_width: 210.0,
                ..Default::default()
            },
        };

        let marker = ParagraphEditorMarker {
            kind: ListMarkerKind::Bullet,
            text: "•".to_string(),
            advance: 10.0, // body 相对 anchor 左边界偏移 10px
            runs: vec![test_run("marker-1", "•", 40.0, 50.0, false)],
            is_cross_paragraph: false,
        };

        let document_plan = EditContext {
            source_body_text: body_text.clone(),
            body_text_plan: build_editor_session_text_plan(&body_session),
            body_session,
            marker: Some(marker),
            ..Default::default()
        };

        // 删除部分文字后
        let draft_text = "Anchor Framework";
        let plan = build_save_layout(&document_plan, draft_text, |text, run| {
            text.chars().count() as f32 * run.style.font_size.max(1.0) * 0.5
        });

        // 验证：marker 文本应该在渲染结果的开头
        let rendered = rendered_text(&plan);
        assert!(
            rendered.starts_with("•"),
            "rendered text should start with marker, got: {}",
            rendered
        );

        // 验证：只有一行
        assert_eq!(plan.layout.lines.len(), 1, "single line expected");

        // 验证：第一个 run 是 marker，且 marker 保持在 body 左侧。
        let first_line = &plan.layout.lines[0];
        let first_run = first_line.runs.first();
        assert!(first_run.is_some(), "should have first run");
        assert_eq!(first_run.unwrap().text, "•", "first run should be marker");
        assert!(
            first_line.runs.len() >= 2,
            "marker line should also contain at least one body run"
        );
        assert!(
            first_line.runs[0].origin_x < first_line.runs[1].origin_x,
            "marker must render to the left of body text, marker_x={}, body_x={}",
            first_line.runs[0].origin_x,
            first_line.runs[1].origin_x
        );
    }

    #[test]
    fn deleting_text_keeps_semantic_marker_body_at_single_advance() {
        use crate::edit::document_plan::ParagraphEditorMarker;
        use crate::text::list_semantics::ListMarkerKind;

        let body_text = "Shoes are boring. Wear sneakers. 用王威表达你的态度。".to_string();
        let runs = vec![test_run_with_origins("body", &body_text, 60.0, false)];
        let body_session = ParagraphEditContext {
            anchor_bbox: BoundingBox {
                left: 40.0,
                top: 40.0,
                right: 360.0,
                bottom: 52.0,
            },
            paragraph: LayoutParagraph {
                id: "p-semantic-delete-marker".to_string(),
                runs,
                wrap_width: 320.0,
                ..Default::default()
            },
        };
        let marker = ParagraphEditorMarker {
            kind: ListMarkerKind::Bullet,
            text: "•".to_string(),
            advance: 20.0,
            runs: vec![marker_test_run("marker", "•", 40.0, 8.0)],
            is_cross_paragraph: false,
        };
        let document_plan = EditContext {
            source_body_text: body_text,
            body_text_plan: build_editor_session_text_plan(&body_session),
            body_session,
            marker: Some(marker),
            ..Default::default()
        };

        let draft_text = "Shoes are boring. Wear sneakers. 用王表达你的态度。";
        let plan = build_save_layout(&document_plan, draft_text, |text, _run| {
            text.chars().count() as f32 * 5.0
        });

        let first_line = plan.layout.lines.first().expect("expected first line");
        assert!(first_line.runs.len() >= 2, "expected marker and body runs");
        assert_eq!(first_line.runs[0].text, "•");
        assert_eq!(first_line.runs[0].origin_x, 0.0);
        assert_eq!(
            first_line.runs[1].origin_x, 20.0,
            "deleting text must not double-apply marker.advance or marker width to body x"
        );
    }

    #[test]
    fn deleting_text_keeps_geometric_marker_left_of_body_anchor() {
        use crate::edit::document_plan::ParagraphEditorMarker;
        use crate::text::list_semantics::ListMarkerKind;

        let body_text = "你的最爱，由你定制。释放奇思妙想".to_string();
        let runs = vec![test_run_with_origins("body", &body_text, 60.0, false)];
        let body_session = ParagraphEditContext {
            anchor_bbox: BoundingBox {
                left: 60.0,
                top: 40.0,
                right: 260.0,
                bottom: 52.0,
            },
            paragraph: LayoutParagraph {
                id: "p-geometric-delete-marker".to_string(),
                runs,
                wrap_width: 200.0,
                ..Default::default()
            },
        };
        let marker = ParagraphEditorMarker {
            kind: ListMarkerKind::Bullet,
            text: "•".to_string(),
            advance: 0.0,
            runs: vec![marker_test_run("marker", "•", 42.0, 8.0)],
            is_cross_paragraph: false,
        };
        let document_plan = EditContext {
            source_body_text: body_text,
            body_text_plan: build_editor_session_text_plan(&body_session),
            body_session,
            marker: Some(marker),
            ..Default::default()
        };

        let draft_text = "你的最爱，由你定制。释放奇思想";
        let plan = build_save_layout(&document_plan, draft_text, |text, _run| {
            text.chars().count() as f32 * 5.0
        });

        let first_line = plan.layout.lines.first().expect("expected first line");
        assert!(first_line.runs.len() >= 2, "expected marker and body runs");
        assert_eq!(first_line.runs[0].text, "•");
        assert_eq!(
            first_line.runs[0].origin_x, -18.0,
            "geometric marker must stay at its PDF position relative to the body anchor"
        );
        assert_eq!(
            first_line.runs[1].origin_x, 0.0,
            "geometric marker gap must not be reused as body indent after deleting text"
        );
    }

    #[test]
    fn persisted_overlay_prefers_marker_source_width() {
        use crate::edit::document_plan::ParagraphEditorMarker;
        use crate::text::list_semantics::ListMarkerKind;

        let body_text = "Body".to_string();
        let runs = vec![test_run_with_origins("r1", &body_text, 50.0, false)];
        let body_session = ParagraphEditContext {
            anchor_bbox: BoundingBox {
                left: 50.0,
                top: 40.0,
                right: 90.0,
                bottom: 52.0,
            },
            paragraph: LayoutParagraph {
                id: "p-source-width-marker".to_string(),
                runs,
                wrap_width: 120.0,
                ..Default::default()
            },
        };

        let marker = ParagraphEditorMarker {
            kind: ListMarkerKind::Bullet,
            text: "•".to_string(),
            advance: 0.0,
            runs: vec![marker_test_run("marker-1", "•", 42.0, 8.0)],
            is_cross_paragraph: false,
        };

        let document_plan = EditContext {
            source_body_text: body_text.clone(),
            body_text_plan: build_editor_session_text_plan(&body_session),
            body_session,
            marker: Some(marker),
            ..Default::default()
        };

        let plan = build_save_layout(&document_plan, &body_text, |text, _run| {
            if text == "•" {
                40.0
            } else {
                text.chars().count() as f32 * 5.0
            }
        });

        let first_line = plan.layout.lines.first().expect("expected first line");
        assert!(first_line.runs.len() >= 2, "expected marker and body runs");
        let marker_run = &first_line.runs[0];
        let body_run = &first_line.runs[1];

        assert_eq!(marker_run.text, "•");
        assert_eq!(marker_run.origin_x, -8.0);
        assert_eq!(body_run.origin_x, 0.0);
    }

    #[test]
    fn persisted_overlay_uses_marker_bbox_when_char_width_missing() {
        use crate::edit::document_plan::ParagraphEditorMarker;
        use crate::text::list_semantics::ListMarkerKind;

        let body_text = "Body".to_string();
        let runs = vec![test_run_with_origins("r1", &body_text, 50.0, false)];
        let body_session = ParagraphEditContext {
            anchor_bbox: BoundingBox {
                left: 50.0,
                top: 40.0,
                right: 90.0,
                bottom: 52.0,
            },
            paragraph: LayoutParagraph {
                id: "p-bbox-width-marker".to_string(),
                runs,
                wrap_width: 120.0,
                ..Default::default()
            },
        };

        let marker = ParagraphEditorMarker {
            kind: ListMarkerKind::Bullet,
            text: "•".to_string(),
            advance: 0.0,
            runs: vec![test_run("marker-1", "•", 42.0, 50.0, false)],
            is_cross_paragraph: false,
        };

        let document_plan = EditContext {
            source_body_text: body_text.clone(),
            body_text_plan: build_editor_session_text_plan(&body_session),
            body_session,
            marker: Some(marker),
            ..Default::default()
        };

        let plan = build_save_layout(&document_plan, &body_text, |text, _run| {
            if text == "•" {
                40.0
            } else {
                text.chars().count() as f32 * 5.0
            }
        });

        let first_line = plan.layout.lines.first().expect("expected first line");
        assert!(first_line.runs.len() >= 2, "expected marker and body runs");
        let marker_run = &first_line.runs[0];
        let body_run = &first_line.runs[1];

        assert_eq!(marker_run.text, "•");
        assert_eq!(marker_run.origin_x, -8.0);
        assert_eq!(body_run.origin_x, 0.0);
    }

    #[test]
    fn persisted_overlay_shifts_caret_stops_by_multichar_marker() {
        use crate::edit::document_plan::ParagraphEditorMarker;
        use crate::text::list_semantics::ListMarkerKind;

        let body_text = "Body".to_string();
        let runs = vec![test_run_with_origins("r1", &body_text, 50.0, false)];
        let body_session = ParagraphEditContext {
            anchor_bbox: BoundingBox {
                left: 50.0,
                top: 40.0,
                right: 90.0,
                bottom: 52.0,
            },
            paragraph: LayoutParagraph {
                id: "p-numbered-list-item".to_string(),
                runs,
                wrap_width: 120.0,
                ..Default::default()
            },
        };

        let marker = ParagraphEditorMarker {
            kind: ListMarkerKind::Numbering,
            text: "10. ".to_string(),
            advance: 20.0,
            runs: vec![test_run("marker-1", "10. ", 30.0, 48.0, false)],
            is_cross_paragraph: false,
        };

        let document_plan = EditContext {
            source_body_text: body_text.clone(),
            body_text_plan: build_editor_session_text_plan(&body_session),
            body_session,
            marker: Some(marker),
            ..Default::default()
        };

        let plan = build_save_layout(&document_plan, &body_text, |text, _run| {
            text.chars().count() as f32 * 5.0
        });

        assert!(rendered_text(&plan).starts_with("10. "));
        let first_line = plan.caret_lines.first().expect("expected caret line");
        let indices = first_line
            .stops
            .iter()
            .map(|stop| stop.index)
            .collect::<Vec<_>>();
        assert!(
            indices.first().copied().unwrap_or_default() >= "10. ".chars().count(),
            "caret stops should be shifted by marker text length, got {:?}",
            indices
        );
    }

    #[test]
    fn keeps_marker_left_of_body_when_right_aligned() {
        use crate::edit::document_plan::ParagraphEditorMarker;
        use crate::text::list_semantics::ListMarkerKind;

        let body_text = "分布式：Seata分布式事务、Redis持久化".to_string();
        let runs = vec![test_run_with_origins("r1", &body_text, 50.0, false)];
        let body_session = ParagraphEditContext {
            anchor_bbox: BoundingBox {
                left: 50.0,
                top: 40.0,
                right: 260.0,
                bottom: 52.0,
            },
            paragraph: LayoutParagraph {
                id: "p-right-list-item".to_string(),
                style: crate::models::ParagraphStyle {
                    align: LayoutAlignment::Right,
                    ..Default::default()
                },
                runs,
                wrap_width: 210.0,
                ..Default::default()
            },
        };

        let marker = ParagraphEditorMarker {
            kind: ListMarkerKind::Bullet,
            text: "•".to_string(),
            advance: 10.0,
            runs: vec![test_run("marker-1", "•", 40.0, 50.0, false)],
            is_cross_paragraph: false,
        };

        let document_plan = EditContext {
            source_body_text: body_text.clone(),
            body_text_plan: build_editor_session_text_plan(&body_session),
            body_session,
            marker: Some(marker),
            ..Default::default()
        };

        let plan = build_save_layout(&document_plan, &body_text, |text, run| {
            text.chars().count() as f32 * run.style.font_size.max(1.0) * 0.5
        });

        let first_line = plan.layout.lines.first().expect("expected first line");
        assert!(first_line.runs.len() >= 2, "expected marker and body runs");
        let marker_run = &first_line.runs[0];
        let body_run = &first_line.runs[1];

        assert_eq!(marker_run.text, "•");
        assert!(
            marker_run.origin_x < body_run.origin_x,
            "right alignment must not move marker after body, marker_x={}, body_x={}",
            marker_run.origin_x,
            body_run.origin_x
        );
        assert!(
            marker_run.origin_x < 0.0,
            "marker should preserve its PDF position to the left of the body anchor, got {}",
            marker_run.origin_x
        );
    }
}
