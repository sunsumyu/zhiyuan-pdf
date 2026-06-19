use crate::infrastructure::pdf::models::{GlyphPaintPlan, LayoutInferenceResult};
use std::collections::HashMap;

use super::page_intermediate_service::PdfPageIntermediateService;

pub struct PdfEditorGeometryService;

impl PdfEditorGeometryService {
    pub async fn resolve_layout_inference(
        state: tauri::State<'_, crate::AppState>,
        path: String,
        page_index: u16,
    ) -> Result<LayoutInferenceResult, String> {
        Self::resolve_layout_inference_revisioned(state, path, page_index, None).await
    }

    pub async fn resolve_layout_inference_revisioned(
        state: tauri::State<'_, crate::AppState>,
        path: String,
        page_index: u16,
        document_revision: Option<u64>,
    ) -> Result<LayoutInferenceResult, String> {
        PdfPageIntermediateService::resolve_layout_inference(
            &state,
            path,
            page_index,
            document_revision,
        )
        .await
    }

    pub async fn resolve_glyph_paint_plan(
        state: tauri::State<'_, crate::AppState>,
        path: String,
        page_index: u16,
    ) -> Result<GlyphPaintPlan, String> {
        Self::resolve_plan(state, path, page_index, None).await
    }

    pub async fn resolve_plan(
        state: tauri::State<'_, crate::AppState>,
        path: String,
        page_index: u16,
        document_revision: Option<u64>,
    ) -> Result<GlyphPaintPlan, String> {
        PdfPageIntermediateService::resolve_glyph_paint_plan(
            &state,
            path,
            page_index,
            document_revision,
        )
        .await
    }

    pub fn read_image_cache(_path: &str) -> HashMap<String, String> {
        use base64::{engine::general_purpose, Engine as _};
        let cache = crate::infrastructure::pdf::cache::PDF_IMAGE_CACHE
            .lock()
            .unwrap();
        let mut result = HashMap::new();
        for (id, data) in cache.iter() {
            let mime = if data.len() > 4 && &data[0..3] == b"\xff\xd8\xff" {
                "image/jpeg"
            } else if data.len() > 8 && &data[0..8] == b"\x89PNG\r\n\x1a\n" {
                "image/png"
            } else if data.len() > 2 && &data[0..2] == b"BM" {
                "image/bmp"
            } else {
                "image/png"
            };
            let b64 = general_purpose::STANDARD.encode(&**data);
            result.insert(id.clone(), format!("data:{mime};base64,{b64}"));
        }
        result
    }

    pub fn resolve_editor_caret_index(
        session: pdf_viewer_core::models::ParagraphEditContext,
        click_x_from_anchor_left: f32,
    ) -> Result<usize, String> {
        Ok(
            pdf_viewer_core::text::glyph_layout::resolve_click_caret(
                &session,
                click_x_from_anchor_left,
            ),
        )
    }

    pub fn resolve_field_hit(
        request: pdf_viewer_core::models::FieldHitRequest,
    ) -> Result<pdf_viewer_core::models::FieldHitResolution, String> {
        Ok(pdf_viewer_core::text::glyph_layout::resolve_click_hit(&request))
    }

    pub fn resolve_field_hit_target(
        request: pdf_viewer_core::models::FieldHitBatchRequest,
    ) -> Result<Option<pdf_viewer_core::models::FieldHitMatch>, String> {
        Ok(pdf_viewer_core::text::glyph_layout::resolve_click_target(&request))
    }

    pub fn resolve_field_projection(
        request: pdf_viewer_core::models::FieldProjectionRequest,
    ) -> Result<pdf_viewer_core::models::FieldProjection, String> {
        Ok(pdf_viewer_core::geometry::field_projection::resolve_field_projection(&request))
    }

    pub fn resolve_field_editor_params(
        request: pdf_viewer_core::models::FieldEditorParamsRequest,
    ) -> Result<pdf_viewer_core::models::FieldEditorParams, String> {
        Ok(pdf_viewer_core::render::paint_plan::build_field_editor_params(&request))
    }
}
