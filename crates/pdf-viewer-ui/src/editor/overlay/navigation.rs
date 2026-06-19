use crate::editor::editor_controller::set_editor_caret;
use crate::editor::mode::read_state;
use crate::editor::text_geometry::move_caret_by_key;

pub fn execute_navigation(key: &str) -> Option<usize> {
    let active_state = read_state()?;
    let draft_text = active_state.current_text().to_string();
    let caret_index = active_state.normalized_caret_index();
    let next_caret = move_caret_by_key(&active_state.target, &draft_text, caret_index, key)?;
    let _ = set_editor_caret(next_caret);
    Some(next_caret)
}
