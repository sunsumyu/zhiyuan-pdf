use crate::edit::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use crate::text::list_semantics::{derive_list_text_semantics, ListMarkerKind};
use crate::models::LayoutAlignment;
use serde::{Deserialize, Serialize};

use crate::edit::active_target::ActiveEditorTarget;
use crate::text::text_model::EditorTextModel;
use crate::text::style_mapper::StyleMapper;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LiveEditorParagraphState {
    pub target: ActiveEditorTarget,
    pub text_model: EditorTextModel,
    #[serde(default)]
    pub style_mapper: StyleMapper,
    #[serde(default)]
    pub list_kind: ListMarkerKind,
    #[serde(default)]
    pub source_alignment: LayoutAlignment,
    #[serde(default)]
    pub source_line_height: f32,
    pub caret_index: usize,
    #[serde(default)]
    pub scene_revision: u64,
    #[serde(default)]
    pub session_dirty: bool,
}

fn alignment_label(align: LayoutAlignment) -> &'static str {
    match align {
        LayoutAlignment::Left => "left",
        LayoutAlignment::Center => "center",
        LayoutAlignment::Right => "right",
        LayoutAlignment::Justify => "justify",
    }
}

fn list_kind_label(kind: ListMarkerKind) -> &'static str {
    match kind {
        ListMarkerKind::None => "none",
        ListMarkerKind::Bullet => "bullet",
        ListMarkerKind::Numbering => "numbering",
        ListMarkerKind::Symbol => "symbol",
        ListMarkerKind::Custom => "custom",
    }
}

fn derive_next_marker_text(
    next_kind: ListMarkerKind,
    source_kind: ListMarkerKind,
    source_marker_text: Option<&str>,
) -> String {
    let source_marker = source_marker_text.unwrap_or("").to_string();
    match next_kind {
        ListMarkerKind::None => String::new(),
        ListMarkerKind::Bullet => {
            if matches!(
                source_kind,
                ListMarkerKind::Bullet | ListMarkerKind::Symbol | ListMarkerKind::Custom
            ) && !source_marker.is_empty()
            {
                source_marker
            } else {
                "•".to_string()
            }
        }
        ListMarkerKind::Numbering => {
            if source_kind == ListMarkerKind::Numbering && !source_marker.is_empty() {
                source_marker
            } else {
                "1.".to_string()
            }
        }
        ListMarkerKind::Symbol | ListMarkerKind::Custom => {
            if !source_marker.is_empty() {
                source_marker
            } else {
                "•".to_string()
            }
        }
    }
}

impl LiveEditorParagraphState {
    pub fn new(target: ActiveEditorTarget) -> Self {
        let source_text = target.source_body_text().to_string();
        let style_mapper = StyleMapper::new_from_paragraph_for_text(
            &target.scene.body_session.paragraph,
            &source_text,
        );
        let list_kind = target
            .scene
            .marker
            .as_ref()
            .map(|marker| marker.kind)
            .unwrap_or(ListMarkerKind::None);
        Self {
            text_model: EditorTextModel::new(source_text),
            style_mapper,
            list_kind,
            source_alignment: target.scene.body_session.paragraph.style.align,
            source_line_height: target
                .scene
                .body_session
                .paragraph
                .style
                .line_height
                .max(1.0),
            caret_index: target.initial_body_caret_index(),
            target,
            scene_revision: 0,
            session_dirty: false,
        }
    }

    pub fn paragraph_id(&self) -> &str {
        &self.target.paragraph_id
    }

    pub fn text_char_count(&self) -> usize {
        self.text_model.current_char_count()
    }

    pub fn normalize_caret(&mut self) {
        self.caret_index = self.caret_index.min(self.text_char_count());
    }

    pub fn set_caret_index(&mut self, caret_index: usize) -> bool {
        let normalized = caret_index.min(self.text_char_count());
        let changed = self.caret_index != normalized;
        if changed {
            self.caret_index = normalized;
        }
        changed
    }

    pub fn set_draft_text(&mut self, new_text: String) -> bool {
        let changed = self.text_model.set_current_text(new_text);
        if changed {
            self.style_mapper
                .update_with_text(self.text_model.current_text());
            self.normalize_caret();
            self.sync_target_control_style();
            self.scene_revision = self.scene_revision.saturating_add(1);
            self.session_dirty = true;
        }
        changed
    }

    pub fn current_text(&self) -> &str {
        self.text_model.current_text()
    }

    pub fn draft_text(&self) -> &str {
        self.text_model.current_text()
    }

    pub fn source_text(&self) -> &str {
        self.text_model.source_text()
    }

    pub fn normalized_caret_index(&self) -> usize {
        self.caret_index.min(self.text_char_count())
    }

    pub fn toggle_bold_all(&mut self) -> bool {
        let next = !self.style_mapper.is_bold_all();
        self.style_mapper.set_bold_all(next);
        self.sync_target_control_style();
        self.scene_revision = self.scene_revision.saturating_add(1);
        self.session_dirty = true;
        next
    }

    pub fn toggle_italic_all(&mut self) -> bool {
        let next = !self.style_mapper.is_italic_all();
        self.style_mapper.set_italic_all(next);
        self.sync_target_control_style();
        self.scene_revision = self.scene_revision.saturating_add(1);
        self.session_dirty = true;
        next
    }

    pub fn toggle_underline_all(&mut self) -> bool {
        let next = !self.style_mapper.is_underline_all();
        self.style_mapper.set_underline_all(next);
        self.sync_target_control_style();
        self.scene_revision = self.scene_revision.saturating_add(1);
        self.session_dirty = true;
        next
    }

    pub fn is_bold_active(&self) -> bool {
        self.style_mapper.is_bold_any()
    }

    pub fn is_italic_active(&self) -> bool {
        self.style_mapper.is_italic_any()
    }

    pub fn is_underline_active(&self) -> bool {
        self.style_mapper.is_underline_any()
    }

    pub fn active_color(&self) -> String {
        self.style_mapper.dominant_style().color
    }

    pub fn active_font_family(&self) -> String {
        self.style_mapper.dominant_style().font_name
    }

    pub fn active_font_size(&self) -> f32 {
        self.style_mapper.dominant_style().font_size.max(1.0)
    }

    pub fn active_char_spacing(&self) -> f32 {
        self.style_mapper.dominant_style().char_spacing
    }

    pub fn active_line_height(&self) -> f32 {
        self.target
            .scene
            .body_session
            .paragraph
            .style
            .line_height
            .max(1.0)
    }

    pub fn source_line_height(&self) -> f32 {
        self.source_line_height.max(1.0)
    }

    pub fn active_paragraph_mode_label(&self) -> String {
        let line_height = self.active_line_height();
        if line_height <= 1.05 {
            "compact".to_string()
        } else if line_height >= 1.5 {
            "relaxed".to_string()
        } else if (line_height - 1.2).abs() <= 0.11 {
            "normal".to_string()
        } else {
            "custom".to_string()
        }
    }

    pub fn active_alignment(&self) -> LayoutAlignment {
        self.target.scene.body_session.paragraph.style.align
    }

    pub fn active_alignment_label(&self) -> String {
        alignment_label(self.active_alignment()).to_string()
    }

    pub fn source_alignment(&self) -> LayoutAlignment {
        self.source_alignment
    }

    pub fn source_list_kind(&self) -> ListMarkerKind {
        self.target
            .scene
            .marker
            .as_ref()
            .map(|marker| marker.kind)
            .unwrap_or(ListMarkerKind::None)
    }

    pub fn active_list_kind(&self) -> ListMarkerKind {
        self.list_kind
    }

    pub fn active_list_kind_label(&self) -> String {
        list_kind_label(self.active_list_kind()).to_string()
    }

    pub fn has_style_changes(&self) -> bool {
        self.style_mapper
            .has_style_changes_against_paragraph(&self.target.scene.body_session.paragraph)
    }

    pub fn requires_source_replacement(&self) -> bool {
        self.current_text() != self.source_text()
            || self.has_style_changes()
            || self.active_alignment() != self.source_alignment()
            || (self.active_line_height() - self.source_line_height()).abs() >= 0.01
            || self.active_list_kind() != self.source_list_kind()
    }

    pub fn has_session_changes(&self) -> bool {
        self.session_dirty
    }

    pub fn mark_session_clean(&mut self) {
        self.session_dirty = false;
    }

    pub fn draft_runs(&self) -> Vec<crate::models::LayoutRun> {
        self.style_mapper.to_layout_runs()
    }

    pub fn sync_target_control_style(&mut self) {
        let style = self.style_mapper.dominant_style();
        self.target.font_family = style.font_name.clone();
        self.target.font_size = style.font_size.max(1.0);
        self.target.font_weight = if style.is_bold {
            "bold".to_string()
        } else {
            "normal".to_string()
        };
        self.target.font_style = if style.is_italic {
            "italic".to_string()
        } else {
            "normal".to_string()
        };
        self.target.color = style.color;
        self.target.text_decoration = if style.is_underline {
            "underline".to_string()
        } else {
            "none".to_string()
        };
        dbg_event(
            "live-style",
            "sync-target-control-style",
            vec![
                dbg_field("paragraphId", self.paragraph_id()),
                dbg_field("liveColor", self.target.color.as_str()),
                dbg_field("liveTextDecoration", self.target.text_decoration.as_str()),
                dbg_field("liveUnderline", style.is_underline),
                dbg_field("fontWeight", self.target.font_weight.as_str()),
                dbg_field("fontStyle", self.target.font_style.as_str()),
                dbg_field("listKind", self.active_list_kind_label()),
            ],
        );
    }

    pub fn set_alignment(&mut self, align: LayoutAlignment) -> bool {
        if self.active_alignment() == align {
            return false;
        }
        self.target.scene.body_session.paragraph.style.align = align;
        self.target.editor_session.paragraph.style.align = align;
        self.scene_revision = self.scene_revision.saturating_add(1);
        self.session_dirty = true;
        true
    }

    pub fn set_list_kind(&mut self, list_kind: ListMarkerKind) -> bool {
        let next_kind = if self.list_kind == list_kind && list_kind != ListMarkerKind::None {
            ListMarkerKind::None
        } else {
            list_kind
        };
        if self.list_kind == next_kind {
            return false;
        }
        self.list_kind = next_kind;
        self.scene_revision = self.scene_revision.saturating_add(1);
        self.session_dirty = true;
        true
    }

    pub fn restore_list_kind_from_marker_text(&mut self, marker_text: &str) {
        let restored_kind = derive_list_text_semantics(marker_text).kind;
        self.list_kind = restored_kind;
    }

    pub fn resolved_marker_text_for_patch(&self) -> Option<String> {
        let source_marker_text = self
            .target
            .scene
            .marker
            .as_ref()
            .map(|marker| marker.text.as_str());
        let next = derive_next_marker_text(
            self.active_list_kind(),
            self.source_list_kind(),
            source_marker_text,
        );
        if next.is_empty() && self.source_list_kind() == ListMarkerKind::None {
            None
        } else {
            Some(next)
        }
    }

    pub fn source_marker_text_for_patch(&self) -> Option<&str> {
        self.target
            .scene
            .marker
            .as_ref()
            .map(|marker| marker.text.as_str())
    }

    pub fn set_color_all(&mut self, color: &str) -> bool {
        let normalized = color.trim();
        if normalized.is_empty() {
            return false;
        }
        if self.active_color().eq_ignore_ascii_case(normalized) {
            return false;
        }
        self.style_mapper.set_color_all(normalized);
        self.sync_target_control_style();
        self.scene_revision = self.scene_revision.saturating_add(1);
        self.session_dirty = true;
        true
    }

    pub fn set_font_family_all(&mut self, font_family: &str) -> bool {
        let normalized = font_family.trim();
        if normalized.is_empty() {
            return false;
        }
        if self.active_font_family().eq_ignore_ascii_case(normalized) {
            return false;
        }
        self.style_mapper.set_font_name_all(normalized);
        self.sync_target_control_style();
        self.scene_revision = self.scene_revision.saturating_add(1);
        self.session_dirty = true;
        true
    }

    pub fn set_font_size_all(&mut self, font_size: f32) -> bool {
        if !font_size.is_finite() {
            return false;
        }
        let normalized = font_size.clamp(1.0, 288.0);
        if (self.active_font_size() - normalized).abs() < 0.01 {
            return false;
        }
        self.style_mapper.set_font_size_all(normalized);
        self.sync_target_control_style();
        self.scene_revision = self.scene_revision.saturating_add(1);
        self.session_dirty = true;
        true
    }

    pub fn set_char_spacing_all(&mut self, char_spacing: f32) -> bool {
        if !char_spacing.is_finite() {
            return false;
        }
        let normalized = char_spacing.clamp(-5.0, 20.0);
        if (self.active_char_spacing() - normalized).abs() < 0.01 {
            return false;
        }
        self.style_mapper.set_char_spacing_all(normalized);
        self.sync_target_control_style();
        self.scene_revision = self.scene_revision.saturating_add(1);
        self.session_dirty = true;
        true
    }

    pub fn set_line_height(&mut self, line_height: f32) -> bool {
        if !line_height.is_finite() {
            return false;
        }
        let normalized = line_height.clamp(0.8, 4.0);
        if (self.active_line_height() - normalized).abs() < 0.01 {
            return false;
        }
        self.target.scene.body_session.paragraph.style.line_height = normalized;
        self.target.editor_session.paragraph.style.line_height = normalized;
        self.scene_revision = self.scene_revision.saturating_add(1);
        self.session_dirty = true;
        true
    }

    pub fn set_paragraph_mode(&mut self, mode: &str) -> bool {
        let normalized = mode.trim().to_ascii_lowercase();
        let target_line_height = match normalized.as_str() {
            "compact" => 1.0,
            "normal" => 1.2,
            "relaxed" => 1.6,
            _ => return false,
        };
        self.set_line_height(target_line_height)
    }
}
