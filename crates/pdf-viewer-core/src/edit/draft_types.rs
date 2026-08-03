//! Draft layout types — 从 draft_layout.rs 拆分。
//!
//! 包含 caret stop、caret line、render plan 等核心类型定义。

/// 光标停止点 — 表示一个可行的光标位置。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftCaretStop {
    pub index: usize,
    pub left: f32,
}

/// 光标行 — 表示一行内的所有光标停止点。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftCaretLine {
    pub baseline_y: f32,
    pub height: f32,
    pub stops: Vec<DraftCaretStop>,
}

/// 编辑器 draft 布局结果 — 包含排版几何和光标行。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftLayout {
    pub layout: crate::geometry::layout_engine::ParagraphLayout,
    pub caret_lines: Vec<DraftCaretLine>,
}

/// 文本差异结果 — 比较源文本和 draft 文本的公共前后缀。
pub(super) struct TextDiff {
    pub prefix_len: usize,
    pub suffix_len: usize,
    pub source_len: usize,
    pub draft_len: usize,
}

impl TextDiff {
    /// draft 中被插入/编辑片段的起始索引（字符空间）。
    pub fn inserted_start(&self) -> usize {
        self.prefix_len
    }

    /// draft 中被插入/编辑片段的结束索引（字符空间）。
    pub fn inserted_end(&self) -> usize {
        self.draft_len.saturating_sub(self.suffix_len)
    }

    /// 是否存在被编辑的中间片段。
    pub fn has_inserted(&self) -> bool {
        self.inserted_start() < self.inserted_end()
    }
}
