use crate::edit::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use crate::edit::engine_state::LiveEditorParagraphState;

#[derive(Debug, Clone)]
pub struct EditorResolvedDocumentState {
    pub source_text: String,
    pub current_text: String,
    pub current_chars: Vec<char>,
    pub mutation_chars: Vec<char>,
    pub caret_index: usize,
    pub is_pristine: bool,
    pub is_slot_backed: bool,
}

impl EditorResolvedDocumentState {
    pub fn char_count(&self) -> usize {
        self.mutation_chars.len()
    }
}

pub fn chars_to_text(chars: Vec<char>) -> String {
    chars.into_iter().collect()
}

pub fn resolve_document_state(
    state: &LiveEditorParagraphState,
) -> EditorResolvedDocumentState {
    let source_text = state.source_text().to_string();
    let current_text = state.current_text().to_string();
    let current_chars: Vec<char> = current_text.chars().collect();
    let is_pristine = current_text == source_text;
    let slot_chars: Vec<char> = state
        .target
        .scene
        .document_plan
        .body_text_plan
        .slots
        .iter()
        .map(|slot| slot.ch)
        .collect();
    let slot_text = slot_chars.iter().collect::<String>();
    let slot_char_count = slot_chars.len();
    let is_slot_backed = is_pristine && !slot_chars.is_empty() && slot_text == source_text;
    // Editing commands must always mutate the semantic text model. Slot/gap geometry is
    // only for layout and hit-testing; treating it as mutable text creates a second index
    // space where delete/backspace can target a different character than the visible caret.
    let mutation_chars = current_chars.clone();
    let caret_index = state.normalized_caret_index().min(mutation_chars.len());
    dbg_event(
        "document-state",
        "resolved",
        vec![
            dbg_field("paragraphId", state.paragraph_id()),
            dbg_field("sourceText", &source_text),
            dbg_field("currentText", &current_text),
            dbg_field("caretIndex", caret_index),
            dbg_field("currentCharCount", current_chars.len()),
            dbg_field("mutationCharCount", mutation_chars.len()),
            dbg_field("slotCharCount", slot_char_count),
            dbg_field("isPristine", is_pristine),
            dbg_field("isSlotBacked", is_slot_backed),
        ],
    );

    EditorResolvedDocumentState {
        source_text,
        current_text,
        current_chars,
        mutation_chars,
        caret_index,
        is_pristine,
        is_slot_backed,
    }
}
