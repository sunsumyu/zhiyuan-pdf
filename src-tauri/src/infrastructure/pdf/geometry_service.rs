use crate::infrastructure::pdf::models::{GlyphPaintPlan, LayoutInferenceResult};
use crate::log_step;
use std::collections::HashMap;

use super::cache::page_cache_key;

pub struct PdfEditorGeometryService;

impl PdfEditorGeometryService {
    pub async fn get_layout_inference(
        state: tauri::State<'_, crate::AppState>,
        path: String,
        page_index: u16,
    ) -> Result<LayoutInferenceResult, String> {
        log_step!("[PDF-V3] Requesting Inference for page={}", page_index);
        let cache_key = page_cache_key(&path, page_index);

        if let Some(result) = {
            let cache = state.cache.pdf_layout_cache.lock().unwrap();
            cache.get(&cache_key).cloned()
        } {
            log_step!("[PDF-Cache] HIT for {}", cache_key);
            return Ok((*result).clone());
        }

        let lopdf_doc = {
            let cache = state.docs.pdf_documents.lock().unwrap();
            cache.get(&path).cloned()
        };

        let lopdf_doc = if let Some(doc) = lopdf_doc {
            doc
        } else {
            return Err(format!("Document not found in cache for path={}", path));
        };

        let result: LayoutInferenceResult = tokio::task::spawn_blocking(move || {
            crate::infrastructure::pdf::vector_engine::get_layout_inference(
                &lopdf_doc, page_index,
            )
        })
        .await
        .map_err(|e| format!("Spawn Error V3: {}", e))??;

        {
            let mut cache = state.cache.pdf_layout_cache.lock().unwrap();
            cache.insert(cache_key, std::sync::Arc::new(result.clone()));
        }

        Ok(result)
    }

    pub async fn get_glyph_paint_plan(
        state: tauri::State<'_, crate::AppState>,
        path: String,
        page_index: u16,
    ) -> Result<GlyphPaintPlan, String> {
        let layout = Self::get_layout_inference(state, path, page_index).await?;
        let plan = pdf_viewer_core::render::paint_plan::build_glyph_paint_plan(&layout);
        let region_count = plan.regions.len();
        let paragraph_count: usize = plan.regions.iter().map(|r| r.paragraphs.len()).sum();
        crate::log_step!(
            "[PDF][get_glyph_paint_plan] page={} regions={} paragraphs={} width={} height={}",
            page_index, region_count, paragraph_count, plan.width, plan.height
        );
        Ok(plan)
    }

    pub fn get_image_cache(_path: &str) -> HashMap<String, String> {
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
            pdf_viewer_core::text::glyph_layout::resolve_caret_index_for_click(
                &session,
                click_x_from_anchor_left,
            ),
        )
    }

    pub fn resolve_field_hit(
        request: pdf_viewer_core::models::FieldHitRequest,
    ) -> Result<pdf_viewer_core::models::FieldHitResolution, String> {
        Ok(pdf_viewer_core::text::glyph_layout::resolve_field_hit_for_click(
            &request,
        ))
    }

    pub fn resolve_field_hit_target(
        request: pdf_viewer_core::models::FieldHitBatchRequest,
    ) -> Result<Option<pdf_viewer_core::models::FieldHitMatch>, String> {
        Ok(pdf_viewer_core::text::glyph_layout::resolve_field_hit_target_for_click(&request))
    }

    pub fn resolve_field_projection(
        request: pdf_viewer_core::models::FieldProjectionRequest,
    ) -> Result<pdf_viewer_core::models::FieldProjection, String> {
        Ok(pdf_viewer_core::geometry::field_projection::resolve_field_projection(
            &request,
        ))
    }

    pub fn resolve_field_editor_params(
        request: pdf_viewer_core::models::FieldEditorParamsRequest,
    ) -> Result<pdf_viewer_core::models::FieldEditorParams, String> {
        Ok(pdf_viewer_core::render::paint_plan::build_field_editor_params(
            &request,
        ))
    }
}
