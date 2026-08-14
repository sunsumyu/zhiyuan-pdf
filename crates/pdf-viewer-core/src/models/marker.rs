//! 统一视觉 Marker 抽象 - 支持文本和图形两种类型
//!
//! 解决的问题：
//! - 原有 ParagraphEditorMarker 只能表示文本型 marker
//! - 无法处理图形 bullet（如 Image/Path 类型的蓝点图标）
//! - 导致编辑正文时图形 marker 被误抑制

use serde::{Deserialize, Serialize};
use super::geometry::BoundingBox;
use super::layout::LayoutRun;

/// 视觉 Marker 类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VisualMarkerKind {
    #[default]
    None,
    TextBullet,      // 文本 bullet (•, ●, ▪ 等)
    TextNumbering,   // 文本编号 (1., 2., (1) 等)
    GraphicBullet,   // 图形 bullet (Image/Path)
    Custom,          // 自定义 marker
}

/// 图形对象类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GraphicType {
    Image,
    Path,
}

/// Marker 内容：文本或图形引用
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum VisualMarkerContent {
    Text {
        text: String,
        runs: Vec<LayoutRun>,
    },
    Graphic {
        object_index: usize,
        object_type: GraphicType,
        object_id: String,
    },
}

/// 统一视觉 Marker 结构
///
/// 能够表示：
/// - 文本型 marker（原有 ParagraphEditorMarker 的功能）
/// - 图形型 marker（新增：Image/Path 对象引用）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisualMarker {
    /// Marker 类型
    pub kind: VisualMarkerKind,
    
    /// Marker 内容（文本或图形引用）
    pub content: VisualMarkerContent,
    
    /// Marker 的边界框
    pub bbox: BoundingBox,
    
    /// Marker 占用的水平宽度（用于 body 缩进）
    pub advance: f32,
    
    /// VectorPageModel.objects 中的索引列表
    /// 对于文本 marker：从 runs.object_indices 收集
    /// 对于图形 marker：直接存储 object_index
    #[serde(default)]
    pub object_indices: Vec<usize>,
}

impl VisualMarker {
    /// 从文本 marker 创建
    pub fn from_text_marker(
        text: String,
        runs: Vec<LayoutRun>,
        advance: f32,
        kind: VisualMarkerKind,
    ) -> Self {
        let bbox = compute_bbox_from_runs(&runs);
        let object_indices = runs
            .iter()
            .flat_map(|run| run.object_indices.iter().copied())
            .collect();
        
        VisualMarker {
            kind,
            content: VisualMarkerContent::Text { text, runs },
            bbox,
            advance,
            object_indices,
        }
    }
    
    /// 从图形对象创建
    pub fn from_graphic(
        object_index: usize,
        object_type: GraphicType,
        object_id: String,
        bbox: BoundingBox,
    ) -> Self {
        // 计算 advance：从 bbox.left 到 bbox.right + 间隙
        let advance = (bbox.right - bbox.left).max(0.0) + 6.0; // 6px 间隙
        
        VisualMarker {
            kind: VisualMarkerKind::GraphicBullet,
            content: VisualMarkerContent::Graphic {
                object_index,
                object_type,
                object_id,
            },
            bbox,
            advance,
            object_indices: vec![object_index],
        }
    }
    
    /// 是否是图形 marker
    pub fn is_graphic(&self) -> bool {
        matches!(self.content, VisualMarkerContent::Graphic { .. })
    }
    
    /// 是否包含指定的对象索引
    pub fn contains_object_index(&self, index: usize) -> bool {
        self.object_indices.contains(&index)
    }
}

/// 从 runs 计算 bbox
fn compute_bbox_from_runs(runs: &[LayoutRun]) -> BoundingBox {
    if runs.is_empty() {
        return BoundingBox::default();
    }
    
    let mut bbox = runs[0].bbox;
    for run in runs.iter().skip(1) {
        bbox.left = bbox.left.min(run.bbox.left);
        bbox.top = bbox.top.min(run.bbox.top);
        bbox.right = bbox.right.max(run.bbox.right);
        bbox.bottom = bbox.bottom.max(run.bbox.bottom);
    }
    bbox
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{BoundingBox, LayoutRun, RunStyle};
    
    fn test_run(text: &str, left: f32, baseline_y: f32) -> LayoutRun {
        LayoutRun {
            id: "test-run".to_string(),
            text: text.to_string(),
            style: RunStyle {
                font_name: "Arial".to_string(),
                font_size: 12.0,
                color: "#111111".to_string(),
                is_bold: false,
                is_italic: false,
                is_underline: false,
                char_spacing: 0.0,
                scale_x: 1.0,
            },
            bbox: BoundingBox {
                left,
                top: baseline_y - 12.0,
                right: left + 30.0,
                bottom: baseline_y,
            },
            origin_x: left,
            origin_y: baseline_y,
            char_origins: Vec::new(),
            char_widths: Vec::new(),
            object_ids: vec!["obj-1".to_string()],
            object_indices: vec![42],
        }
    }
    
    #[test]
    fn text_marker_collects_object_indices() {
        let marker = VisualMarker::from_text_marker(
            "•".to_string(),
            vec![test_run("•", 36.0, 112.0)],
            12.0,
            VisualMarkerKind::TextBullet,
        );
        
        assert!(marker.object_indices.contains(&42));
        assert!(!marker.is_graphic());
    }
    
    #[test]
    fn graphic_marker_stores_object_index() {
        let marker = VisualMarker::from_graphic(
            7,
            GraphicType::Image,
            "img-bullet".to_string(),
            BoundingBox {
                left: 36.0,
                top: 100.0,
                right: 48.0,
                bottom: 112.0,
            },
        );
        
        assert!(marker.is_graphic());
        assert!(marker.contains_object_index(7));
        assert_eq!(marker.kind, VisualMarkerKind::GraphicBullet);
    }
}