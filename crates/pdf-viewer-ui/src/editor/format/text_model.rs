use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EditorTextModel {
    pub source_text: String,
    pub current_text: String,
}

impl EditorTextModel {
    pub fn new(source_text: String) -> Self {
        Self {
            current_text: source_text.clone(),
            source_text,
        }
    }

    pub fn source_text(&self) -> &str {
        &self.source_text
    }

    pub fn current_text(&self) -> &str {
        &self.current_text
    }

    pub fn current_char_count(&self) -> usize {
        self.current_text.chars().count()
    }

    pub fn is_pristine(&self) -> bool {
        self.current_text == self.source_text
    }

    pub fn set_current_text(&mut self, next_text: String) -> bool {
        let changed = self.current_text != next_text;
        if changed {
            self.current_text = next_text;
        }
        changed
    }
}
