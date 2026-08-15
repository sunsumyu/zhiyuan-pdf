//! PDF document editing operations via `PdfDocExt` trait.
//!
//! The trait definition and thin `impl` live here. Heavy logic is delegated to:
//! - `reflow` — text replacement and layout-aware reflow
//! - `annotations` — highlight, comment, delete
//! - `pages` — delete, rotate, insert, metadata, image replace
//! - `emitters` — PDF operator emission for deferred text lines

mod annotations;
mod emitters;
mod pages;
mod reflow;

pub(crate) use emitters::*;
pub(crate) use reflow::{
    PersistedTextLinePlan, PdfTextState, ReflowCluster,
    patch_content_recursive, patch_atomic_reflow_recursive,
};

use crate::infrastructure::pdf::models::*;
use lopdf::{content::Content, Dictionary, Document, Object, Stream};
use crate::infrastructure::pdf::pdf_read::read_resources;
use crate::infrastructure::pdf::font::ResourceCache;
use crate::infrastructure::pdf::pdf_utils;
use std::collections::HashMap;

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

impl PdfDocExt for Document {
    fn apply_text_patch(
        &mut self,
        page_num: u32,
        old_text: &str,
        new_text: &str,
        target_index: Option<usize>,
        offset_x: Option<f32>,
    ) -> Result<(), String> {
        let page_id = *self
            .get_pages()
            .get(&page_num)
            .ok_or_else(|| format!("Page {} not found", page_num))?;
        let resources = read_resources(self, page_id);
        let mut cache = ResourceCache::new();
        let content_data = self.get_page_content(page_id).map_err(|e| e.to_string())?;
        let mut content = Content::decode(&content_data).map_err(|e| e.to_string())?;

        let mut obj_counter = 0;
        if patch_content_recursive(
            self,
            &mut content,
            &resources,
            &mut cache,
            old_text,
            new_text,
            target_index,
            offset_x,
            &mut obj_counter,
        )? {
            let new_content = content.encode().map_err(|e| e.to_string())?;
            let stream_id = self.new_object_id();
            self.objects.insert(
                stream_id,
                Object::Stream(Stream::new(Dictionary::new(), new_content)),
            );
            let page_dict = self
                .get_object_mut(page_id)
                .and_then(|obj| obj.as_dict_mut())
                .map_err(|e| e.to_string())?;
            page_dict.set("Contents", Object::Reference(stream_id));
            Ok(())
        } else {
            Err("Text not found for patching".into())
        }
    }

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
    ) -> Result<(), String> {
        let single_patch = TextReflowPatch {
            page_index: page_num as u16,
            target_indices: target_indices.to_vec(),
            new_text: new_text.to_string(),
            new_runs,
            alignment: align,
            line_height,
            displacement_y,
            wrap_width,
            char_spacing,
            horizontal_scaling,
        };
        self.apply_batch_reflow_to_doc(page_num, &[single_patch])
    }

    fn apply_batch_reflow_to_doc(
        &mut self,
        page_num: u32,
        patches: &[TextReflowPatch],
    ) -> Result<(), String> {
        let page_id = *self
            .get_pages()
            .get(&page_num)
            .ok_or_else(|| format!("Page {} not found", page_num))?;
        let flat_resources = read_resources(self, page_id);
        let mut res_cache = ResourceCache::new();

        let _page_dict = self.get_dictionary(page_id).map_err(|e| e.to_string())?;
        let page_size = pdf_utils::read_page_size(self, page_id);
        let page_height = page_size.effective_height();

        let content_data = self
            .get_page_content(page_id)
            .map_err(|e| format!("Failed to get page content: {}", e))?;
        let mut content = Content::decode(&content_data)
            .map_err(|e| format!("Failed to decode content: {}", e))?;

        let user_unit = self
            .get_dictionary(page_id)
            .ok()
            .and_then(|dict| dict.get(b"UserUnit").ok())
            .and_then(|o| {
                o.as_float()
                    .ok()
                    .or_else(|| o.as_i64().ok().map(|i| i as f32))
            })
            .unwrap_or(1.0);

        content
            .operations
            .insert(0, lopdf::content::Operation::new("q", vec![]));

        let clusters = ReflowCluster::build(patches);
        let mut cluster_map = HashMap::new();
        for cluster in &clusters {
            cluster_map.insert(cluster.min_idx, cluster.clone());
        }

        let mut state = PdfTextState::new();
        let mut deferred_lines: Vec<PersistedTextLinePlan> = Vec::new();
        let mut obj_counter = 0;

        let changed = patch_atomic_reflow_recursive(
            self,
            page_id,
            &mut content,
            &flat_resources,
            &mut res_cache,
            &cluster_map,
            page_height,
            &mut obj_counter,
            &mut state,
            &mut deferred_lines,
        )?;

        if changed || !deferred_lines.is_empty() {
            content
                .operations
                .push(lopdf::content::Operation::new("Q", vec![]));
        }

        if !deferred_lines.is_empty() {
            content
                .operations
                .extend(emit_deferred_text_block(&deferred_lines, page_height, user_unit));
        }

        if changed || !deferred_lines.is_empty() {
            let new_content = content
                .encode()
                .map_err(|e| format!("Failed to encode: {}", e))?;
            let new_id = self.new_object_id();
            self.objects.insert(
                new_id,
                Object::Stream(Stream::new(Dictionary::new(), new_content)),
            );
            let p_dict = self
                .get_object_mut(page_id)
                .and_then(|o| o.as_dict_mut())
                .map_err(|e| e.to_string())?;
            p_dict.set("Contents", Object::Reference(new_id));
        }
        Ok(())
    }

    fn replace_image_xobject(
        &mut self,
        object_id: (u32, u16),
        new_bytes: &[u8],
    ) -> Result<(), String> {
        pages::replace_image_xobject_impl(self, object_id, new_bytes)
    }

    fn delete_page(&mut self, page_num: u32) -> Result<(), String> {
        pages::delete_page_impl(self, page_num)
    }

    fn rotate_page(&mut self, page_num: u32, rotation: i32) -> Result<(), String> {
        pages::rotate_page_impl(self, page_num, rotation)
    }

    fn insert_blank_page(&mut self, at_index: u32) -> Result<(), String> {
        pages::insert_blank_page_impl(self, at_index)
    }

    fn add_highlight(
        &mut self,
        page_num: u32,
        rect: [f32; 4],
        color: [f32; 3],
    ) -> Result<(), String> {
        annotations::add_highlight_impl(self, page_num, rect, color)
    }

    fn add_text_comment(
        &mut self,
        page_num: u32,
        rect: [f32; 4],
        color: [f32; 3],
        contents: &str,
    ) -> Result<(), String> {
        annotations::add_text_comment_impl(self, page_num, rect, color, contents)
    }

    fn update_text_comment(
        &mut self,
        page_num: u32,
        annot_id: (u32, u16),
        contents: &str,
    ) -> Result<(), String> {
        annotations::update_text_comment_impl(self, page_num, annot_id, contents)
    }

    fn delete_annotation(&mut self, page_num: u32, annot_id: (u32, u16)) -> Result<(), String> {
        annotations::delete_annotation_impl(self, page_num, annot_id)
    }

    fn update_metadata(
        &mut self,
        title: &str,
        author: &str,
        subject: &str,
        keywords: &str,
    ) -> Result<(), String> {
        pages::update_metadata_impl(self, title, author, subject, keywords)
    }
}
