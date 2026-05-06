use crate::editor::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use crate::editor::document_runtime::{chars_to_text, resolve_document_state};
use crate::editor::engine_state::LiveEditorParagraphState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorTextMutation {
    pub text: String,
    pub caret_index: usize,
}

pub fn insert_text(
    state: &LiveEditorParagraphState,
    inserted_text: &str,
) -> EditorTextMutation {
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
