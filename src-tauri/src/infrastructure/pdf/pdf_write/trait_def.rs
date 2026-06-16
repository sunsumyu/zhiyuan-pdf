use crate::infrastructure::pdf::models::TextReflowPatch;

pub trait PdfDocExt {
    fn apply_text_patch(
        &mut self,
        page_num: u32,
        old_text: &str,
        new_text: &str,
        target_index: Option<usize>,
        offset_x: Option<f32>,
    ) -> Result<(), String>;
    fn apply_atomic_reflow_to_doc(
        &mut self,
        page_num: u32,
        target_indices: &[usize],
        new_text: &str,
        new_runs: Option<Vec<pdf_viewer_core::models::LayoutRun>>,
        displacement_y: Option<f32>,
        wrap_width: Option<f32>,
        align: Option<pdf_viewer_core::models::LayoutAlignment>,
        line_height: Option<f32>,
        char_spacing: f32,
        horizontal_scaling: f32,
    ) -> Result<(), String>;
    fn apply_batch_reflow_to_doc(
        &mut self,
        page_num: u32,
        patches: &[TextReflowPatch],
    ) -> Result<(), String>;
    fn replace_image_xobject(
        &mut self,
        object_id: (u32, u16),
        new_bytes: &[u8],
    ) -> Result<(), String>;
    fn delete_page(&mut self, page_num: u32) -> Result<(), String>;
    fn rotate_page(&mut self, page_num: u32, rotation: i32) -> Result<(), String>;
    fn insert_blank_page(&mut self, at_index: u32) -> Result<(), String>;
    fn add_highlight(
        &mut self,
        page_num: u32,
        rect: [f32; 4],
        color: [f32; 3],
    ) -> Result<(), String>;
    fn add_text_comment(
        &mut self,
        page_num: u32,
        rect: [f32; 4],
        color: [f32; 3],
        contents: &str,
    ) -> Result<(), String>;
    fn update_text_comment(
        &mut self,
        page_num: u32,
        annot_id: (u32, u16),
        contents: &str,
    ) -> Result<(), String>;
    fn delete_annotation(&mut self, page_num: u32, annot_id: (u32, u16)) -> Result<(), String>;
    fn update_metadata(
        &mut self,
        title: &str,
        author: &str,
        subject: &str,
        keywords: &str,
    ) -> Result<(), String>;
}
