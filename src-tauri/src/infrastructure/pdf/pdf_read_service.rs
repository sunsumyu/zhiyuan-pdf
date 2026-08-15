use crate::infrastructure::pdf::models::{
    GlyphPaintPlan, LayoutInferenceResult, NativeVectorPageModel, PdfMetadata,
};
use crate::infrastructure::pdf::page_intermediate_service::PdfPageIntermediateService;
use std::collections::HashMap;

pub struct PdfReadService;

impl PdfReadService {
    /// 打开PDF文档并加载到内存缓存
    pub async fn open_pdf(
        state: tauri::State<'_, crate::AppState>,
        path: String,
    ) -> Result<(), String> {
        crate::log_step!("[PDF][open_pdf] START path={}", path);

        crate::infrastructure::pdf::document_resolver::ensure_loaded(&state, &path).await?;

        crate::log_step!("[PDF][open_pdf] SUCCESS");
        Ok(())
    }

    /// 从应用状态获取PDF元数据
    pub(crate) async fn read_pdf_metadata_from_app_state(
        app_state: &crate::AppState,
        path: &str,
    ) -> Result<PdfMetadata, String> {
        let docs = app_state.docs.pdf_documents.lock().unwrap();
        let doc = docs
            .get(path)
            .ok_or_else(|| crate::PdfError::DocumentNotFound {
                path: path.to_string(),
            })?;

        crate::infrastructure::pdf::pdf_read::extract_metadata(doc)
            .map_err(|e| format!("Metadata extraction failed: {}", e))
    }

    /// 获取PDF文档元数据
    pub async fn read_pdf_metadata(
        state: &tauri::State<'_, crate::AppState>,
        path: String,
    ) -> Result<PdfMetadata, String> {
        Self::read_pdf_metadata_from_app_state(&state, &path).await
    }

    /// 从应用状态获取矢量页面模型
    pub(crate) async fn resolve_vector_page_model_from_app_state(
        app_state: &crate::AppState,
        path: &str,
        page_index: u16,
    ) -> Result<NativeVectorPageModel, String> {
        PdfPageIntermediateService::resolve_vector_page_model_from_app_state(
            app_state,
            path.to_string(),
            page_index,
            1.0,
            None,
        )
        .await
    }

    /// 获取矢量页面模型
    pub async fn resolve_vector_page_model(
        state: tauri::State<'_, crate::AppState>,
        path: String,
        page_index: u16,
    ) -> Result<NativeVectorPageModel, String> {
        Self::resolve_vector_page_model_from_app_state(&state, &path, page_index).await
    }

    /// 获取布局推断结果
    pub async fn resolve_layout_inference(
        state: tauri::State<'_, crate::AppState>,
        path: String,
        page_index: u16,
    ) -> Result<LayoutInferenceResult, String> {
        PdfPageIntermediateService::resolve_layout_inference_from_app_state(
            &state, path, page_index, None,
        )
        .await
    }

    /// 获取字形绘制计划
    pub async fn resolve_glyph_paint_plan(
        state: tauri::State<'_, crate::AppState>,
        path: String,
        page_index: u16,
    ) -> Result<GlyphPaintPlan, String> {
        PdfPageIntermediateService::resolve_glyph_paint_plan_from_app_state(
            &state, path, page_index, None,
        )
        .await
    }

    /// 读取图像缓存（目前返回空HashMap）
    pub fn read_image_cache(_path: &str) -> HashMap<String, String> {
        HashMap::new()
    }
}
