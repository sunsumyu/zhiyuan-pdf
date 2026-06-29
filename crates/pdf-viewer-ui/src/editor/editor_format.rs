use pdf_viewer_core::models::LayoutAlignment;
use pdf_viewer_core::text::list_semantics::ListMarkerKind;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ActiveEditorFormatState {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub color: String,
    pub font_family: String,
    pub font_size: f32,
    pub char_spacing: f32,
    pub line_height: f32,
    pub paragraph_mode: String,
    pub alignment: String,
    pub list_kind: String,
    pub changed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum EditorFormatAction {
    ToggleBold,
    ToggleItalic,
    ToggleUnderline,
    IncreaseFontSize,
    DecreaseFontSize,
    SetParagraphMode { mode: String },
    SetColor { color: String },
    SetFontFamily { font_family: String },
    SetFontSize { font_size: f32 },
    SetCharSpacing { char_spacing: f32 },
    SetLineHeight { line_height: f32 },
    SetAlignment { alignment: String },
    SetListKind { list_kind: String },
}

fn build_format_state(
    live_state: &crate::editor::engine_state::LiveEditorParagraphState,
    changed: bool,
) -> ActiveEditorFormatState {
    ActiveEditorFormatState {
        bold: live_state.is_bold_active(),
        italic: live_state.is_italic_active(),
        underline: live_state.is_underline_active(),
        color: live_state.active_color(),
        font_family: live_state.active_font_family(),
        font_size: live_state.active_font_size(),
        char_spacing: live_state.active_char_spacing(),
        line_height: live_state.active_line_height(),
        paragraph_mode: live_state.active_paragraph_mode_label(),
        alignment: live_state.active_alignment_label(),
        list_kind: live_state.active_list_kind_label(),
        changed,
    }
}

fn resolve_font_size_step(current: f32, increase: bool) -> f32 {
    const STEPS: [f32; 14] = [
        8.0, 9.0, 10.0, 10.5, 11.0, 12.0, 14.0, 16.0, 18.0, 20.0, 24.0, 28.0, 32.0, 36.0,
    ];
    let normalized = current.clamp(1.0, 288.0);
    if increase {
        STEPS
            .into_iter()
            .find(|step| *step > normalized + 0.01)
            .unwrap_or_else(|| (normalized + 1.0).min(288.0))
    } else {
        STEPS
            .into_iter()
            .rev()
            .find(|step| *step < normalized - 0.01)
            .unwrap_or_else(|| (normalized - 1.0).max(1.0))
    }
}

fn parse_alignment(value: &str) -> Option<LayoutAlignment> {
    match value.trim().to_ascii_lowercase().as_str() {
        "left" => Some(LayoutAlignment::Left),
        "center" => Some(LayoutAlignment::Center),
        "right" => Some(LayoutAlignment::Right),
        "justify" => Some(LayoutAlignment::Justify),
        _ => None,
    }
}

fn parse_list_kind(value: &str) -> Option<ListMarkerKind> {
    match value.trim().to_ascii_lowercase().as_str() {
        "none" => Some(ListMarkerKind::None),
        "bullet" => Some(ListMarkerKind::Bullet),
        "numbering" => Some(ListMarkerKind::Numbering),
        "symbol" => Some(ListMarkerKind::Symbol),
        "custom" => Some(ListMarkerKind::Custom),
        _ => None,
    }
}

use crate::editor::session::{with_editor_mode, with_editor_mode_mut};

pub fn toggle_bold() -> ActiveEditorFormatState {
    with_editor_mode_mut(|mode| {
        let Some(live_state) = mode.live_state.as_mut() else {
            return ActiveEditorFormatState::default();
        };
        let changed = live_state.toggle_bold_all();
        build_format_state(live_state, changed)
    })
}

pub fn toggle_italic() -> ActiveEditorFormatState {
    with_editor_mode_mut(|mode| {
        let Some(live_state) = mode.live_state.as_mut() else {
            return ActiveEditorFormatState::default();
        };
        let changed = live_state.toggle_italic_all();
        build_format_state(live_state, changed)
    })
}

pub fn toggle_underline() -> ActiveEditorFormatState {
    with_editor_mode_mut(|mode| {
        let Some(live_state) = mode.live_state.as_mut() else {
            return ActiveEditorFormatState::default();
        };
        let changed = live_state.toggle_underline_all();
        build_format_state(live_state, changed)
    })
}

pub fn set_color(color: &str) -> ActiveEditorFormatState {
    with_editor_mode_mut(|mode| {
        let Some(live_state) = mode.live_state.as_mut() else {
            return ActiveEditorFormatState::default();
        };
        let changed = live_state.set_color_all(color);
        build_format_state(live_state, changed)
    })
}

pub fn set_font_family(font_family: &str) -> ActiveEditorFormatState {
    with_editor_mode_mut(|mode| {
        let Some(live_state) = mode.live_state.as_mut() else {
            return ActiveEditorFormatState::default();
        };
        let changed = live_state.set_font_family_all(font_family);
        build_format_state(live_state, changed)
    })
}

pub fn set_font_size(font_size: f32) -> ActiveEditorFormatState {
    with_editor_mode_mut(|mode| {
        let Some(live_state) = mode.live_state.as_mut() else {
            return ActiveEditorFormatState::default();
        };
        let changed = live_state.set_font_size_all(font_size);
        build_format_state(live_state, changed)
    })
}

pub fn step_font_size(increase: bool) -> ActiveEditorFormatState {
    with_editor_mode_mut(|mode| {
        let Some(live_state) = mode.live_state.as_mut() else {
            return ActiveEditorFormatState::default();
        };
        let next_size = resolve_font_size_step(live_state.active_font_size(), increase);
        let changed = live_state.set_font_size_all(next_size);
        build_format_state(live_state, changed)
    })
}

pub fn set_char_spacing(char_spacing: f32) -> ActiveEditorFormatState {
    with_editor_mode_mut(|mode| {
        let Some(live_state) = mode.live_state.as_mut() else {
            return ActiveEditorFormatState::default();
        };
        let changed = live_state.set_char_spacing_all(char_spacing);
        build_format_state(live_state, changed)
    })
}

pub fn set_line_height(line_height: f32) -> ActiveEditorFormatState {
    with_editor_mode_mut(|mode| {
        let Some(live_state) = mode.live_state.as_mut() else {
            return ActiveEditorFormatState::default();
        };
        let changed = live_state.set_line_height(line_height);
        build_format_state(live_state, changed)
    })
}

pub fn set_paragraph_mode(mode: &str) -> ActiveEditorFormatState {
    with_editor_mode_mut(|editor_mode| {
        let Some(live_state) = editor_mode.live_state.as_mut() else {
            return ActiveEditorFormatState::default();
        };
        let changed = live_state.set_paragraph_mode(mode);
        build_format_state(live_state, changed)
    })
}

pub fn set_alignment(alignment: &str) -> ActiveEditorFormatState {
    with_editor_mode_mut(|mode| {
        let Some(live_state) = mode.live_state.as_mut() else {
            return ActiveEditorFormatState::default();
        };
        let Some(next_alignment) = parse_alignment(alignment) else {
            return build_format_state(live_state, false);
        };
        let changed = live_state.set_alignment(next_alignment);
        build_format_state(live_state, changed)
    })
}

pub fn set_list_kind(list_kind: &str) -> ActiveEditorFormatState {
    with_editor_mode_mut(|mode| {
        let Some(live_state) = mode.live_state.as_mut() else {
            return ActiveEditorFormatState::default();
        };
        let Some(next_list_kind) = parse_list_kind(list_kind) else {
            return build_format_state(live_state, false);
        };
        let changed = live_state.set_list_kind(next_list_kind);
        build_format_state(live_state, changed)
    })
}

pub fn format_state() -> ActiveEditorFormatState {
    with_editor_mode(|mode| {
        let Some(live_state) = mode.live_state.as_ref() else {
            return ActiveEditorFormatState::default();
        };
        build_format_state(live_state, false)
    })
}

pub fn apply_format(action: EditorFormatAction) -> ActiveEditorFormatState {
    match action {
        EditorFormatAction::ToggleBold => toggle_bold(),
        EditorFormatAction::ToggleItalic => toggle_italic(),
        EditorFormatAction::ToggleUnderline => toggle_underline(),
        EditorFormatAction::IncreaseFontSize => step_font_size(true),
        EditorFormatAction::DecreaseFontSize => step_font_size(false),
        EditorFormatAction::SetParagraphMode { mode } => set_paragraph_mode(&mode),
        EditorFormatAction::SetColor { color } => set_color(&color),
        EditorFormatAction::SetFontFamily { font_family } => set_font_family(&font_family),
        EditorFormatAction::SetFontSize { font_size } => set_font_size(font_size),
        EditorFormatAction::SetCharSpacing { char_spacing } => set_char_spacing(char_spacing),
        EditorFormatAction::SetLineHeight { line_height } => set_line_height(line_height),
        EditorFormatAction::SetAlignment { alignment } => set_alignment(&alignment),
        EditorFormatAction::SetListKind { list_kind } => set_list_kind(&list_kind),
    }
}
