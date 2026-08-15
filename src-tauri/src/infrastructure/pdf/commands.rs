use crate::infrastructure::pdf::models::{PersistableRegionPatch, TextPatch, TextReflowPatch};
use crate::infrastructure::pdf::pdf_utils::truncate_for_log;
use crate::infrastructure::pdf::pdf_write::PdfDocExt;
use lopdf::Document;
pub use pdf_viewer_core::models::{
    AddHighlightCommand, DeletePageCommand, InsertPageCommand, RotatePageCommand,
    UpdateMetadataCommand,
};

/// PDF 编辑命令接口
pub trait PdfEditCommand: Send + Sync {
    /// 在已加载的 lopdf 文档上执行修改
    fn execute(&self, doc: &mut Document, page_num: u32) -> Result<(), String>;
}

/// 文本替换命令
/// 实现说明：利用 lopdf 的 apply_text_patch 功能，通过内容匹配进行“模糊寻址”替换。
pub struct ReplaceTextCommand {
    pub patch: TextPatch,
}
impl PdfEditCommand for ReplaceTextCommand {
    fn execute(&self, doc: &mut Document, page_num: u32) -> Result<(), String> {
        doc.apply_text_patch(
            page_num,
            &self.patch.old_text,
            &self.patch.new_text,
            self.patch.target_index,
            self.patch.offset_x,
        )
        .map_err(|e| format!("ReplaceText Error: {}", e))
    }
}

/// 区域补丁命令 (V3 Sovereign)
/// 利用 lopdf 物理精确定位并替换文字流。
pub struct PersistableRegionPatchCommand {
    pub patch: PersistableRegionPatch,
}
impl PdfEditCommand for PersistableRegionPatchCommand {
    fn execute(&self, doc: &mut Document, page_num: u32) -> Result<(), String> {
        crate::log_step!(
            "[PDF-ATOMIC][cmd] page={} region={} targets={:?} text='{}'",
            page_num,
            self.patch.region_id,
            self.patch.target_indices,
            truncate_for_log(&self.patch.new_text, 64)
        );
        // [V3 Persistence] Use atomic reflow to handle potential line breaking and multi-object replacement
        doc.apply_atomic_reflow_to_doc(
            page_num,
            &self.patch.target_indices,
            &self.patch.new_text,
            self.patch.new_runs.clone(),
            self.patch.displacement_y,
            self.patch.wrap_width,
            self.patch.align,
            self.patch.line_height,
            self.patch.char_spacing,
            self.patch.horizontal_scaling,
        )
        .map_err(|e| format!("PersistableRegionPatch Error (Atomic): {}", e))
    }
}

/// 文本重排命令 (Materialized Reflow)
/// 统一由 materializer 产出的结构化文本重排计划驱动，避免把整段文字硬塞回首个对象。
pub struct TextReflowCommand {
    pub patch: TextReflowPatch,
}
impl PdfEditCommand for TextReflowCommand {
    fn execute(&self, doc: &mut Document, page_num: u32) -> Result<(), String> {
        crate::log_step!(
            "[PDF-ATOMIC][reflow-cmd] page={} targets={:?} text='{}'",
            page_num,
            self.patch.target_indices,
            truncate_for_log(&self.patch.new_text, 64)
        );
        doc.apply_atomic_reflow_to_doc(
            page_num,
            &self.patch.target_indices,
            &self.patch.new_text,
            self.patch.new_runs.clone(),
            self.patch.displacement_y,
            self.patch.wrap_width,
            self.patch.alignment,
            self.patch.line_height,
            self.patch.char_spacing,
            self.patch.horizontal_scaling,
        )
        .map_err(|e| format!("TextReflow Error (Atomic): {}", e))
    }
}

/// 批量文本重排命令 (V19 Batch Architecture)
/// 强制在单次内容流遍历中处理所有补丁，确保对象索引稳定性。
pub struct BatchTextReflowCommand {
    pub patches: Vec<TextReflowPatch>,
}
impl PdfEditCommand for BatchTextReflowCommand {
    fn execute(&self, doc: &mut Document, page_num: u32) -> Result<(), String> {
        crate::log_step!(
            "[PDF-ATOMIC][batch-cmd] page={} patch_count={}",
            page_num,
            self.patches.len()
        );
        doc.apply_batch_reflow_to_doc(page_num, &self.patches)
            .map_err(|e| format!("BatchTextReflow Error: {}", e))
    }
}

/// 图像物理替换命令 (V3 Sovereign)
pub struct ReplaceImageCommand {
    pub object_id: (u32, u16),
    pub new_image_bytes: Vec<u8>,
}
impl PdfEditCommand for ReplaceImageCommand {
    fn execute(&self, doc: &mut Document, _page_num: u32) -> Result<(), String> {
        doc.replace_image_xobject(self.object_id, &self.new_image_bytes)
            .map_err(|e| format!("ReplaceImage Error: {}", e))
    }
}

impl PdfEditCommand for DeletePageCommand {
    fn execute(&self, doc: &mut Document, _page_num: u32) -> Result<(), String> {
        doc.delete_page(self.page_num)
            .map_err(|e| format!("DeletePage Error: {}", e))
    }
}

impl PdfEditCommand for RotatePageCommand {
    fn execute(&self, doc: &mut Document, _page_num: u32) -> Result<(), String> {
        doc.rotate_page(self.page_num, self.delta)
            .map_err(|e| format!("RotatePage Error: {}", e))
    }
}

impl PdfEditCommand for InsertPageCommand {
    fn execute(&self, doc: &mut Document, _page_num: u32) -> Result<(), String> {
        doc.insert_blank_page(self.at_index)
            .map_err(|e| format!("InsertPage Error: {}", e))
    }
}

impl PdfEditCommand for AddHighlightCommand {
    fn execute(&self, doc: &mut Document, _page_num: u32) -> Result<(), String> {
        doc.add_highlight(self.page_num, self.rect, self.color)
            .map_err(|e| format!("AddHighlight Error: {}", e))
    }
}

/// 文本批注命令 (PDF Text Annotation)
pub struct AddCommentCommand {
    pub page_num: u32,
    pub rect: [f32; 4],
    pub color: [f32; 3],
    pub contents: String,
}
impl PdfEditCommand for AddCommentCommand {
    fn execute(&self, doc: &mut Document, _page_num: u32) -> Result<(), String> {
        doc.add_text_comment(self.page_num, self.rect, self.color, &self.contents)
            .map_err(|e| format!("AddComment Error: {}", e))
    }
}

/// 文本批注更新命令 (PDF Text Annotation)
pub struct UpdateCommentCommand {
    pub page_num: u32,
    pub annot_id: (u32, u16),
    pub contents: String,
}
impl PdfEditCommand for UpdateCommentCommand {
    fn execute(&self, doc: &mut Document, _page_num: u32) -> Result<(), String> {
        doc.update_text_comment(self.page_num, self.annot_id, &self.contents)
            .map_err(|e| format!("UpdateComment Error: {}", e))
    }
}

/// 注解删除命令 (PDF Annotation)
pub struct DeleteAnnotationCommand {
    pub page_num: u32,
    pub annot_id: (u32, u16),
}
impl PdfEditCommand for DeleteAnnotationCommand {
    fn execute(&self, doc: &mut Document, _page_num: u32) -> Result<(), String> {
        doc.delete_annotation(self.page_num, self.annot_id)
            .map_err(|e| format!("DeleteAnnotation Error: {}", e))
    }
}

impl PdfEditCommand for UpdateMetadataCommand {
    fn execute(&self, doc: &mut Document, _page_num: u32) -> Result<(), String> {
        doc.update_metadata(&self.title, &self.author, &self.subject, &self.keywords)
            .map_err(|e| format!("UpdateMetadata Error: {}", e))
    }
}
