//! 物理字形空间微观排版引擎 (Micro-Typography & Glyph Layout Engine)
//!
//! 拆分（closeout #06，范式同 canvas/）：类型定义住 mod.rs，逻辑按职责分布：
//! - `text_predicates`: CJK/标点判定 + 合成间隙启发式谓词
//! - `glyph_geometry`:    glyph 左右物理边界、视觉宽度、同行判定、典型 advance、间隙插入判定
//! - `caret`:             run advance 推测、光标定位、点击命中、装饰前缀提取
//! - `layout_engine`:     行级上下文 delta + build_editor_session_text_plan（微排主循环）
//! - `tests`:             原内联测试模块（原样搬移）

#[cfg(test)]
mod tests;

mod caret;
mod glyph_geometry;
mod layout_engine;
mod text_predicates;

pub use caret::{
    compute_run_aware_caret_left, extract_decorative_prefix, infer_run_advance,
    resolve_caret_index_for_click, resolve_field_hit_for_click, resolve_field_hit_target_for_click,
};
pub use layout_engine::build_editor_session_text_plan;
pub use text_predicates::{is_decorative_glyph, is_decorative_text};

use crate::models::LayoutRun;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct DecorativePrefixLayout {
    pub text: String,
    pub char_len: usize,
    pub width: f32,
    pub runs: Vec<LayoutRun>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EditorSessionTextPlan {
    pub text: String,
    pub slots: Vec<EditorGlyphSlot>,
    raw_to_reconstructed: Vec<usize>,
    reconstructed_to_raw: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum EditorGlyphSlotKind {
    #[default]
    Glyph,
    Gap,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EditorGlyphSlot {
    pub kind: EditorGlyphSlotKind,
    pub ch: char,
    pub raw_char_index: Option<usize>,
    pub left: f32,
    pub right: f32,
}

impl EditorSessionTextPlan {
    pub fn map_raw_to_reconstructed(&self, raw_index: usize) -> usize {
        self.raw_to_reconstructed
            .get(raw_index)
            .copied()
            .or_else(|| self.raw_to_reconstructed.last().copied())
            .unwrap_or(0)
    }

    pub fn reconstructed_char_count(&self) -> usize {
        self.text.chars().count()
    }

    pub fn map_reconstructed_to_raw(&self, reconstructed_index: usize) -> usize {
        self.reconstructed_to_raw
            .get(reconstructed_index)
            .copied()
            .or_else(|| self.reconstructed_to_raw.last().copied())
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod test_reexports {
    pub(crate) use super::glyph_geometry::*;
    pub(crate) use super::layout_engine::{has_suspicious_run_geometry, needs_gap};
    pub(crate) use super::text_predicates::*;
}
