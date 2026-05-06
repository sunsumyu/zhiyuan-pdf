use crate::infrastructure::pdf::models::{
    GlyphPaintPlan, LayoutInferenceResult, LightPageModel, VectorPageModel, PdfMetadata,
};
use crate::infrastructure::pdf::domain::{
    DocumentId, PageNumber, BoundingBox, Document as DomainDocument, Page, DomainResult, DomainError,
};
use crate::log_step;
use lopdf::Document as LopdfDocument;
use std::collections::HashMap;
use std::sync::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    static ref WORKING_COPIES: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
    static ref COPY_LOCKS: Mutex<HashMap<String, std::sync::Arc<Mutex<()>>>> =
        Mutex::new(HashMap::new());
}

pub struct PdfReadService;

impl PdfReadService {
    /// 获取PDF文档的工作路径（用于编辑）
    pub(crate) fn get_working_path(original_path: &str) -> String {
        let total_start = std::time::Instant::now();
        let (working_path, lock) = {
            let mut copies = WORKING_COPIES.lock().unwrap();
            let mut locks = COPY_LOCKS.lock().unwrap();

            let digest = md5::compute(original_path);
            let hashed_name = format!("{:x}.pdf", digest);
            let wp = std::env::temp_dir()
                .join(format!("working_{}", hashed_name))
                .to_string_lossy()
                .to_string();

            copies
                .entry(original_path.to_string())
                .or_insert(wp.clone());
            let l = locks
                .entry(original_path.to_string())
                .or_insert_with(|| std::sync::Arc::new(Mutex::new(())))
                .clone();
            (wp, l)
        };

        // V206.77: Atomic copy protection
        let _guard = lock.lock().unwrap();
        if !std::path::Path::new(&working_path).exists() {
            let copy_start = std::time::Instant::now();
            log_step!(
                "[WORKING-PATH] Copying {} -> {}",
                original_path,
                working_path
            );
            if let Err(e) = std::fs::copy(original_path, &working_path) {
                log_step!("[WORKING-PATH] Copy failed: {}", e);
            }
            log_step!(
                "[WORKING-PATH] Copy took {:?}",
                copy_start.elapsed()
            );
        }
        log_step!(
            "[WORKING-PATH] Total took {:?}",
            total_start.elapsed()
        );
        working_path
    }

    /// 打开PDF文档并加载到内存缓存
    pub async fn open_pdf(
        state: tauri::State<'_, crate::AppState>,
        path: String,
    ) -> Result<(), String> {
        log_step!("[PDF][open_pdf] START path={}", path);
        
        // 检查是否已在缓存中
        {
            let cache = state.pdf_documents.lock().unwrap();
            if cache.contains_key(&path) {
                log_step!("[PDF][open_pdf] Already cached: {}", path);
                return Ok(());
            }
        }

        // 设置加载状态
        {
            let mut loading = state.loading_docs.lock().unwrap();
            loading.insert(path.clone(), crate::state::LoadingStatus::Loading);
        }

        let path_for_load = path.clone();
        let loaded_doc = tokio::task::spawn_blocking(move || {
            let working_path = Self::get_working_path(&path_for_load);
            LopdfDocument::load(&working_path)
                .map(std::sync::Arc::new)
                .map_err(|e| format!("Lopdf Load Error for {}: {}", path_for_load, e))
        })
        .await
        .map_err(|e| e.to_string())??;

        // 更新缓存
        {
            let mut cache = state.pdf_documents.lock().unwrap();
            cache.insert(path.clone(), loaded_doc);
        }

        // 更新加载状态
        {
            let mut loading = state.loading_docs.lock().unwrap();
            loading.insert(path, crate::state::LoadingStatus::Ready);
        }

        log_step!("[PDF][open_pdf] SUCCESS");
        Ok(())
    }

    /// 从应用状态获取PDF元数据
    pub(crate) async fn get_pdf_metadata_from_app_state(
        app_state: &crate::AppState,
        path: &str,
    ) -> Result<PdfMetadata, String> {
        let docs = app_state.pdf_documents.lock().unwrap();
        let doc = docs
            .get(path)
            .ok_or_else(|| format!("Document not found in cache: {}", path))?;
        
        crate::infrastructure::pdf::pdf_read::extract_metadata(doc)
            .map_err(|e| format!("Metadata extraction failed: {}", e))
    }

    /// 获取PDF文档元数据
    pub async fn get_pdf_metadata(
        state: &tauri::State<'_, crate::AppState>,
        path: String,
    ) -> Result<PdfMetadata, String> {
        Self::get_pdf_metadata_from_app_state(&state, &path).await
    }

    /// 从应用状态获取矢量页面模型
    pub(crate) async fn get_vector_page_model_from_app_state(
        app_state: &crate::AppState,
        path: &str,
        page_index: u16,
    ) -> Result<VectorPageModel, String> {
        let docs = app_state.pdf_documents.lock().unwrap();
        let doc = docs
            .get(path)
            .ok_or_else(|| format!("Document not found in cache: {}", path))?;
        
        crate::infrastructure::pdf::pdf_read::extract_vector_page_model(
            doc, page_index,
        )
        .map_err(|e| format!("Vector page model extraction failed: {}", e))
    }

    /// 获取矢量页面模型
    pub async fn get_vector_page_model(
        state: tauri::State<'_, crate::AppState>,
        path: String,
        page_index: u16,
    ) -> Result<VectorPageModel, String> {
        Self::get_vector_page_model_from_app_state(&state, &path, page_index).await
    }

    /// 获取布局推断结果
    pub async fn get_layout_inference(
        state: tauri::State<'_, crate::AppState>,
        path: String,
        page_index: u16,
    ) -> Result<LayoutInferenceResult, String> {
        let docs = state.pdf_documents.lock().unwrap();
        let doc = docs
            .get(&path)
            .ok_or_else(|| format!("Document not found in cache: {}", path))?;
        
        crate::infrastructure::pdf::pdf_read::extract_layout_inference(
            doc, page_index,
        )
        .map_err(|e| format!("Layout inference failed: {}", e))
    }

    /// 获取字形绘制计划
    pub async fn get_glyph_paint_plan(
        state: tauri::State<'_, crate::AppState>,
        path: String,
        page_index: u16,
    ) -> Result<GlyphPaintPlan, String> {
        let docs = state.pdf_documents.lock().unwrap();
        let doc = docs
            .get(&path)
            .ok_or_else(|| format!("Document not found in cache: {}", path))?;
        
        crate::infrastructure::pdf::pdf_read::extract_glyph_paint_plan(
            doc, page_index,
        )
        .map_err(|e| format!("Glyph paint plan extraction failed: {}", e))
    }

    /// 读取图像缓存（目前返回空HashMap）
    pub fn read_image_cache(_path: &str) -> HashMap<String, String> {
        HashMap::new()
    }

    // === 新增：领域模型支持方法 ===

    /// 创建文档领域模型
    pub async fn create_document(
        state: tauri::State<'_, crate::AppState>,
        document_id: DocumentId,
    ) -> DomainResult<DomainDocument> {
        log_step!("[READ_SERVICE] Creating document domain model: {}", document_id);
        
        // 获取文档元数据
        let metadata = Self::get_pdf_metadata(&state, document_id.as_str().to_string())
            .await
            .map_err(|e| DomainError::PdfProcessingError(e))?;
        
        // 获取页面数量
        let docs = state.pdf_documents.lock().unwrap();
        let doc = docs
            .get(document_id.as_str())
            .ok_or_else(|| DomainError::DocumentNotFound(document_id.as_str().to_string()))?;
        
        let page_count = crate::infrastructure::pdf::pdf_read::get_page_count(doc)
            .map_err(|e| DomainError::PdfProcessingError(e.to_string()))?;
        
        // 创建文档元数据
        let domain_metadata = crate::infrastructure::pdf::domain::DocumentMetadata::new()
            .with_title(metadata.title.unwrap_or_default())
            .with_author(metadata.author.unwrap_or_default());
        
        // 创建文档
        let mut document = DomainDocument::new(document_id)?;
        document.update_metadata(domain_metadata)?;
        
        // 添加页面
        for page_idx in 1..=page_count {
            let page_number = PageNumber::new(page_idx as u16)?;
            
            // 获取页面边界框
            let page_bbox = crate::infrastructure::pdf::pdf_read::extract_page_bbox(doc, page_idx as u16)
                .map_err(|e| DomainError::PdfProcessingError(e.to_string()))?;
            
            let bbox = BoundingBox::from_array(page_bbox)?;
            let page = Page::new(page_number, bbox)?;
            
            document.add_page(page)?;
        }
        
        Ok(document)
    }

    /// 获取文档领域模型（从缓存）
    pub async fn get_document(
        state: tauri::State<'_, crate::AppState>,
        document_id: DocumentId,
    ) -> DomainResult<DomainDocument> {
        // TODO: 添加领域模型缓存到 AppState 后启用缓存逻辑
        // 目前直接创建新文档模型
        Self::create_document(state, document_id).await
    }

    /// 获取页面领域模型
    pub async fn get_page(
        state: tauri::State<'_, crate::AppState>,
        document_id: DocumentId,
        page_number: PageNumber,
    ) -> DomainResult<Page> {
        let document = Self::get_document(state, document_id).await?;
        document.get_page(page_number).cloned()
    }

    /// 刷新文档缓存
    pub async fn refresh_document_cache(
        state: tauri::State<'_, crate::AppState>,
        document_id: DocumentId,
    ) -> DomainResult<()> {
        // TODO: 添加领域模型缓存后清除缓存
        Self::create_document(state, document_id).await?;
        Ok(())
    }

    /// 验证文档完整性
    pub async fn validate_document(
        state: tauri::State<'_, crate::AppState>,
        document_id: DocumentId,
    ) -> DomainResult<()> {
        let document = Self::get_document(state, document_id).await?;
        document.validate()
    }
}
