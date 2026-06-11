use crate::infrastructure::pdf::models::{LightPageKind, LightPageModel, NativeVectorPageModel};

use super::cache::light_page_cache_key;
use super::document_service::PdfDocumentService;
use super::page_intermediate_service::PdfPageIntermediateService;

pub struct PdfPageModelService;

impl PdfPageModelService {
    pub(crate) async fn read_pdf_metadata_from_app_state(
        app_state: &crate::AppState,
        path: &str,
    ) -> Result<crate::infrastructure::pdf::models::PdfMetadata, String> {
        crate::log_step!("[PDF][read_pdf_metadata][V206.45] START for {}", path);

        let (page_count, info_dict) = {
            let mut cache = app_state.docs.pdf_documents.lock().unwrap();
            let doc = if let Some(d) = cache.get(path) {
                d.clone()
            } else {
                let wp = PdfDocumentService::resolve_working_path(path);
                let d = crate::infrastructure::pdf::document_service::load_pdf_public(&wp)
                    .map_err(|e| format!("Lopdf Load Error: {}", e))?;
                let d_arc = std::sync::Arc::new(d);
                cache.insert(path.to_string(), d_arc.clone());
                d_arc
            };

            let info = doc
                .trailer
                .get(b"Info")
                .ok()
                .and_then(|obj| obj.as_reference().ok())
                .and_then(|id| doc.get_object(id).ok())
                .and_then(|obj| obj.as_dict().ok())
                .cloned();

            (doc.get_pages().len(), info)
        };

        let get_str = |key: &[u8]| -> Option<String> {
            info_dict
                .as_ref()?
                .get(key)
                .ok()?
                .as_str()
                .ok()
                .map(|b| String::from_utf8_lossy(b).to_string())
        };

        Ok(crate::infrastructure::pdf::models::PdfMetadata {
            title: get_str(b"Title"),
            author: get_str(b"Author"),
            subject: get_str(b"Subject"),
            keywords: get_str(b"Keywords"),
            creator: get_str(b"Creator"),
            producer: get_str(b"Producer"),
            creation_date: get_str(b"CreationDate"),
            mod_date: get_str(b"ModDate"),
            page_count,
        })
    }

    pub async fn read_pdf_metadata(
        state: tauri::State<'_, crate::AppState>,
        path: &str,
    ) -> Result<crate::infrastructure::pdf::models::PdfMetadata, String> {
        Self::read_pdf_metadata_from_app_state(&state, path).await
    }

    pub(crate) async fn resolve_vector_page_model_from_app_state(
        app_state: &crate::AppState,
        path: String,
        page_index: u16,
        _target_zoom: f32,
    ) -> Result<NativeVectorPageModel, String> {
        Self::resolve_model_from_state(
            app_state,
            path,
            page_index,
            _target_zoom,
            None,
        )
        .await
    }

    pub(crate) async fn resolve_model_from_state(
        app_state: &crate::AppState,
        path: String,
        page_index: u16,
        target_zoom: f32,
        document_revision: Option<u64>,
    ) -> Result<NativeVectorPageModel, String> {
        PdfPageIntermediateService::resolve_vector_page_model_from_app_state(
            app_state,
            path,
            page_index,
            target_zoom,
            document_revision,
        )
        .await
    }

    pub async fn resolve_vector_page_model(
        state: tauri::State<'_, crate::AppState>,
        path: String,
        page_index: u16,
        target_zoom: f32,
    ) -> Result<NativeVectorPageModel, String> {
        Self::resolve_vector_page_model_from_app_state(&state, path, page_index, target_zoom).await
    }

    pub async fn resolve_model(
        state: tauri::State<'_, crate::AppState>,
        path: String,
        page_index: u16,
        target_zoom: f32,
        document_revision: Option<u64>,
    ) -> Result<NativeVectorPageModel, String> {
        Self::resolve_model_from_state(
            &state,
            path,
            page_index,
            target_zoom,
            document_revision,
        )
        .await
    }

    pub async fn resolve_light_page_model(
        state: tauri::State<'_, crate::AppState>,
        path: String,
        page_index: u16,
    ) -> Result<LightPageModel, String> {
        crate::log_step!("[PDF][resolve_light_page_model] START page={}", page_index);
        let total_start = std::time::Instant::now();
        let working_path = path.clone();
        crate::log_step!(
            "[PDF][resolve_light_page_model] Using original path for read-only preview: {}",
            working_path
        );
        let cache_key = light_page_cache_key(&path, page_index);

        if let Some(model) = {
            let cache = state.cache.pdf_light_page_cache.lock().unwrap();
            cache.get(&cache_key).cloned()
        } {
            crate::log_step!("[PDF][LightPageCache] HIT for {}", cache_key);
            crate::log_step!(
                "[PDF][get_light_page_model] TOTAL {:?} (cache hit)",
                total_start.elapsed()
            );
            return Ok((*model).clone());
        }

        let lopdf_doc = {
            let cache = state.docs.pdf_documents.lock().unwrap();
            cache.get(&path).cloned()
        };

        let lopdf_doc = if let Some(doc) = lopdf_doc {
            doc
        } else {
            let loading_error = {
                let loading = state.docs.loading_docs.lock().unwrap();
                match loading.get(&path) {
                    Some(crate::state::LoadingStatus::Error(err)) => Some(err.clone()),
                    _ => None,
                }
            };
            if let Some(err) = loading_error {
                return Err(format!("PDF background load failed for {}: {}", path, err));
            }

            crate::log_step!(
                "[PDF][resolve_light_page_model] Document not ready yet for {}. Returning pending model.",
                path
            );
            crate::log_step!(
                "[PDF][resolve_light_page_model] TOTAL {:?} (pending)",
                total_start.elapsed()
            );
            return Ok(LightPageModel {
                page_index,
                width: 595.0,
                height: 842.0,
                kind: LightPageKind::Pending,
                preview_image_url: None,
            });
        };

        let build_start = std::time::Instant::now();
        let model = tokio::task::spawn_blocking(move || {
            crate::infrastructure::pdf::preview_engine::build_light_page_model(
                &lopdf_doc, page_index,
            )
        })
        .await
        .map_err(|e| format!("Light page spawn error: {}", e))??;
        crate::log_step!(
            "[PDF][resolve_light_page_model] build_light_page_model took {:?}",
            build_start.elapsed()
        );

        {
            let mut cache = state.cache.pdf_light_page_cache.lock().unwrap();
            cache.insert(cache_key, std::sync::Arc::new(model.clone()));
        }

        crate::log_step!(
            "[PDF][resolve_light_page_model] TOTAL {:?}",
            total_start.elapsed()
        );
        Ok(model)
    }
}
