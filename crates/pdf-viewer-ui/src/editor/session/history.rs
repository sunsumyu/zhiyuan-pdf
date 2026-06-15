use crate::editor::engine_state::LiveEditorParagraphState;
use js_sys::Date;

#[derive(Debug, Clone)]
pub struct HistorySnapshot {
    pub state: LiveEditorParagraphState,
    pub timestamp_ms: f64,
}

#[derive(Debug, Clone, Default)]
pub struct LocalEditHistory {
    undo_stack: Vec<HistorySnapshot>,
    redo_stack: Vec<HistorySnapshot>,
}

impl LocalEditHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    pub fn push_snapshot(&mut self, state: &LiveEditorParagraphState) {
        let now = Date::now();
        let should_push = if let Some(last) = self.undo_stack.last() {
            if (now - last.timestamp_ms).abs() > 1000.0 {
                true
            } else {
                // If text is unchanged or only minor character length change, don't push a new snapshot
                let text_diff = (state.current_text().len() as isize - last.state.current_text().len() as isize).abs();
                text_diff > 2 || state.has_style_changes() != last.state.has_style_changes()
            }
        } else {
            true
        };

        if should_push {
            self.undo_stack.push(HistorySnapshot {
                state: state.clone(),
                timestamp_ms: now,
            });
            if self.undo_stack.len() > 100 {
                self.undo_stack.remove(0);
            }
        }
        self.redo_stack.clear();
    }

    pub fn undo(&mut self, current_state: &LiveEditorParagraphState) -> Option<LiveEditorParagraphState> {
        let prev = self.undo_stack.pop()?;
        self.redo_stack.push(HistorySnapshot {
            state: current_state.clone(),
            timestamp_ms: Date::now(),
        });
        Some(prev.state)
    }

    pub fn redo(&mut self, current_state: &LiveEditorParagraphState) -> Option<LiveEditorParagraphState> {
        let next = self.redo_stack.pop()?;
        self.undo_stack.push(HistorySnapshot {
            state: current_state.clone(),
            timestamp_ms: Date::now(),
        });
        Some(next.state)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
}
