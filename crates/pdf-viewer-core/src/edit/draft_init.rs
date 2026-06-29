//! Draft init — 初始与模板化子模块。

use super::draft_style::{resolve_draft_template_run, source_baseline_y};
use super::draft_types::{DraftCaretLine, DraftCaretStop, EditorDraftRenderPlan};
use crate::edit::document_plan::EditContext;
use crate::geometry::layout_engine::{ParagraphLayout, VisualLine};

pub(super) fn build_empty_render_plan(document_plan: &EditContext) -> EditorDraftRenderPlan {
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
