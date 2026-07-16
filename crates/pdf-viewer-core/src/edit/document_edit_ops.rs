use crate::edit::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use crate::edit::document_runtime::{chars_to_text, resolve_document_state};
use crate::edit::engine_state::LiveEditorParagraphState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorTextMutation {
    pub text: String,
    pub caret_index: usize,
}

pub fn insert_text(state: &LiveEditorParagraphState, inserted_text: &str) -> EditorTextMutation {
    let resolved = resolve_document_state(state);
    let current_caret = resolved.caret_index;
    let mut next_chars = resolved.mutation_chars;
    next_chars.splice(current_caret..current_caret, inserted_text.chars());
    let next_text = chars_to_text(next_chars);
    dbg_event(
        "mutation.insert",
        "applied",
        vec![
            dbg_field("paragraphId", state.paragraph_id()),
            dbg_field("beforeText", resolved.current_text),
            dbg_field("insertedText", inserted_text),
            dbg_field("caretBefore", current_caret),
            dbg_field("caretAfter", current_caret + inserted_text.chars().count()),
            dbg_field("afterText", &next_text),
            dbg_field("isPristine", resolved.is_pristine),
            dbg_field("isSlotBacked", resolved.is_slot_backed),
        ],
    );
    EditorTextMutation {
        text: next_text,
        caret_index: current_caret + inserted_text.chars().count(),
    }
}

pub fn delete_backward(state: &LiveEditorParagraphState) -> EditorTextMutation {
    let resolved = resolve_document_state(state);
    let current_caret = resolved.caret_index;
    if current_caret == 0 {
        dbg_event(
            "mutation.backspace",
            "noop-at-start",
            vec![
                dbg_field("paragraphId", state.paragraph_id()),
                dbg_field("beforeText", &resolved.current_text),
                dbg_field("caretBefore", current_caret),
            ],
        );
        return EditorTextMutation {
            text: resolved.current_text,
            caret_index: 0,
        };
    }

    let mut next_chars = resolved.mutation_chars;
    let removed = next_chars
        .get(current_caret - 1)
        .copied()
        .map(|ch| ch.to_string())
        .unwrap_or_default();
    next_chars.remove(current_caret - 1);
    let next_text = chars_to_text(next_chars);
    dbg_event(
        "mutation.backspace",
        "applied",
        vec![
            dbg_field("paragraphId", state.paragraph_id()),
            dbg_field("beforeText", resolved.current_text),
            dbg_field("removedText", removed),
            dbg_field("removeIndex", current_caret - 1),
            dbg_field("caretBefore", current_caret),
            dbg_field("caretAfter", current_caret - 1),
            dbg_field("afterText", &next_text),
            dbg_field("isPristine", resolved.is_pristine),
            dbg_field("isSlotBacked", resolved.is_slot_backed),
        ],
    );
    EditorTextMutation {
        text: next_text,
        caret_index: current_caret - 1,
    }
}

pub fn delete_forward(state: &LiveEditorParagraphState) -> EditorTextMutation {
    let resolved = resolve_document_state(state);
    let current_caret = resolved.caret_index;
    let char_count = resolved.char_count();
    if current_caret >= char_count {
        dbg_event(
            "mutation.delete",
            "noop-at-end",
            vec![
                dbg_field("paragraphId", state.paragraph_id()),
                dbg_field("beforeText", &resolved.current_text),
                dbg_field("caretBefore", current_caret),
                dbg_field("charCount", char_count),
            ],
        );
        return EditorTextMutation {
            text: resolved.current_text,
            caret_index: current_caret,
        };
    }

    let mut next_chars = resolved.mutation_chars;
    let removed = next_chars
        .get(current_caret)
        .copied()
        .map(|ch| ch.to_string())
        .unwrap_or_default();
    next_chars.remove(current_caret);
    let next_text = chars_to_text(next_chars);
    dbg_event(
        "mutation.delete",
        "applied",
        vec![
            dbg_field("paragraphId", state.paragraph_id()),
            dbg_field("beforeText", resolved.current_text),
            dbg_field("removedText", removed),
            dbg_field("removeIndex", current_caret),
            dbg_field("caretBefore", current_caret),
            dbg_field("caretAfter", current_caret),
            dbg_field("afterText", &next_text),
            dbg_field("isPristine", resolved.is_pristine),
            dbg_field("isSlotBacked", resolved.is_slot_backed),
        ],
    );
    EditorTextMutation {
        text: next_text,
        caret_index: current_caret,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edit::active_target::ActiveEditorTarget;
    use crate::edit::document_plan::EditContext;
    use crate::edit::paragraph_scene::from_context;
    use crate::models::{BoundingBox, LayoutParagraph, LayoutRun, ParagraphEditContext, RunStyle};
    use crate::text::glyph_layout::build_editor_session_text_plan;

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
            font_weight_numeric: 400,
        }
    }

    fn test_run(id: &str, text: &str, left: f32, width: f32) -> LayoutRun {
        LayoutRun {
            id: id.to_string(),
            text: text.to_string(),
            style: test_style(),
            bbox: BoundingBox {
                left,
                top: 40.0,
                right: left + width,
                bottom: 52.0,
            },
            origin_x: left,
            origin_y: 50.0,
            char_origins: Vec::new(),
            char_widths: Vec::new(),
            object_ids: Vec::new(),
            object_indices: Vec::new(),
        }
    }

    fn state_with_text(text: &str, caret_index: usize) -> LiveEditorParagraphState {
        state_with_source_and_raw_runs(text, vec![test_run("r1", text, 10.0, 100.0)], caret_index)
    }

    fn state_with_source_and_raw_runs(
        source_text: &str,
        runs: Vec<LayoutRun>,
        caret_index: usize,
    ) -> LiveEditorParagraphState {
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
        let body_session = ParagraphEditContext {
            anchor_bbox,
            paragraph: LayoutParagraph {
                id: "p-mutation".to_string(),
                bbox: anchor_bbox,
                origin_x: anchor_bbox.left,
                origin_y: anchor_bbox.top,
                wrap_width: (anchor_bbox.right - anchor_bbox.left).max(1.0),
                runs,
                ..Default::default()
            },
        };
        let document_plan = EditContext {
            target_id: "p-mutation".to_string(),
            base_paragraph_id: "p-mutation".to_string(),
            shell_bbox: anchor_bbox,
            source_body_text: source_text.to_string(),
            body_text_plan: build_editor_session_text_plan(&body_session),
            body_session,
            body_initial_caret: caret_index,
            ..Default::default()
        };
        let scene = from_context(document_plan).expect("scene should build");
        let target = ActiveEditorTarget {
            paragraph_id: "p-mutation".to_string(),
            region_id: "region-1".to_string(),
            page_index: 0,
            text: source_text.to_string(),
            bbox_left: anchor_bbox.left,
            bbox_top: anchor_bbox.top,
            bbox_right: anchor_bbox.right,
            bbox_bottom: anchor_bbox.bottom,
            font_family: "Microsoft YaHei".to_string(),
            font_size: 10.0,
            font_weight: "400".to_string(),
            font_style: "normal".to_string(),
            color: "#000000".to_string(),
            text_decoration: String::new(),
            initial_caret_index: caret_index,
            editor_session: scene.body_session().clone(),
            scene,
        };
        LiveEditorParagraphState::new(target)
    }

    #[test]
    fn delete_commands_use_char_indices() {
        let state = state_with_text("abcd", 2);

        let backspace = delete_backward(&state);
        assert_eq!(backspace.text, "acd");
        assert_eq!(backspace.caret_index, 1);

        let delete = delete_forward(&state);
        assert_eq!(delete.text, "abd");
        assert_eq!(delete.caret_index, 2);
    }

    #[test]
    fn mutations_count_unicode_scalars_not_bytes() {
        let state = state_with_text("你a好", 1);

        let inserted = insert_text(&state, "🦀");
        assert_eq!(inserted.text, "你🦀a好");
        assert_eq!(inserted.caret_index, 2);

        let backspace = delete_backward(&state);
        assert_eq!(backspace.text, "a好");
        assert_eq!(backspace.caret_index, 0);

        let delete = delete_forward(&state);
        assert_eq!(delete.text, "你好");
        assert_eq!(delete.caret_index, 1);
    }

    #[test]
    fn delete_boundaries_are_noops() {
        let at_start = state_with_text("abc", 0);
        let backspace = delete_backward(&at_start);
        assert_eq!(backspace.text, "abc");
        assert_eq!(backspace.caret_index, 0);

        let at_end = state_with_text("abc", 3);
        let delete = delete_forward(&at_end);
        assert_eq!(delete.text, "abc");
        assert_eq!(delete.caret_index, 3);
    }

    #[test]
    fn mutation_uses_semantic_text_not_raw_slot_text() {
        let state = state_with_source_and_raw_runs(
            "编程语言: Rust",
            vec![
                test_run("r1", "编程语言:", 10.0, 50.0),
                test_run("r2", "Rust", 80.0, 30.0),
            ],
            "编程语言: ".chars().count(),
        );

        let backspace = delete_backward(&state);
        assert_eq!(backspace.text, "编程语言:Rust");
        assert_eq!(backspace.caret_index, "编程语言:".chars().count());

        let delete = delete_forward(&state);
        assert_eq!(delete.text, "编程语言: ust");
        assert_eq!(delete.caret_index, "编程语言: ".chars().count());
    }
}
