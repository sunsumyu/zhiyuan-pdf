use crate::infrastructure::pdf::models::{
    LightPageKind, LightPageModel, VectorPageModel,
};
use crate::infrastructure::pdf::document_service::PdfDocumentService;
use crate::log_step;
use lopdf::Document;
use std::collections::HashMap;

use super::cache::{light_page_cache_key, page_cache_key};

pub struct PdfPageModelService;

impl PdfPageModelService {
    pub(crate) async fn get_pdf_metadata_from_app_state(
        app_state: &crate::AppState,
        path: &str,
    ) -> Result<crate::infrastructure::pdf::models::PdfMetadata, String> {
        log_step!("[PDF][get_pdf_metadata][V206.45] START for {}", path);

        let (page_count, info_dict) = {
            let mut cache = app_state.pdf_documents.lock().unwrap();
            let doc = if let Some(d) = cache.get(path) {
                d.clone()
            } else {
                let wp = PdfDocumentService::get_working_path(path);
                let d = Document::load(&wp).map_err(|e| format!("Lopdf Load Error: {}", e))?;
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

        Ok(
            crate::infrastructure::pdf::models::PdfMetadata {
                title: get_str(b"Title"),
                author: get_str(b"Author"),
                subject: get_str(b"Subject"),
                keywords: get_str(b"Keywords"),
                creator: get_str(b"Creator"),
                producer: get_str(b"Producer"),
                creation_date: get_str(b"CreationDate"),
                mod_date: get_str(b"ModDate"),
                page_count,
            },
        )
    }

    pub async fn get_pdf_metadata(
        state: tauri::State<'_, crate::AppState>,
        path: &str,
    ) -> Result<crate::infrastructure::pdf::models::PdfMetadata, String> {
        Self::get_pdf_metadata_from_app_state(&state, path).await
    }

    pub(crate) async fn get_vector_page_model_from_app_state(
        app_state: &crate::AppState,
        path: String,
        page_index: u16,
        _target_zoom: f32,
    ) -> Result<VectorPageModel, String> {
        log_step!("[PDF][get_vector_page_model] START page={}", page_index);
        let working_path = path.clone();
        let cache_key = page_cache_key(&path, page_index);

        if let Some(model) = {
            let cache = app_state.pdf_page_cache.lock().unwrap();
            cache.get(&cache_key).cloned()
        } {
            log_step!("[PDF][PageCache] HIT for {}", cache_key);
            return Ok((*model).clone());
        }

        let background_image = None;

        let lopdf_doc = {
            let cache = app_state.pdf_documents.lock().unwrap();
            cache.get(&path).cloned()
        };

        let lopdf_doc = if let Some(doc) = lopdf_doc {
            doc
        } else {
            let loading_error = {
                let loading = app_state.loading_docs.lock().unwrap();
                match loading.get(&path) {
                    Some(crate::state::LoadingStatus::Error(err)) => Some(err.clone()),
                    _ => None,
                }
            };

            if let Some(err) = loading_error {
                return Err(format!("PDF background load failed for {}: {}", path, err));
            }

            log_step!(
                "[PDF][get_vector_page_model] Cache MISS for {}. Falling back to synchronous lopdf load.",
                path
            );

            let path_for_load = path.clone();
            let working_path_for_load = working_path.clone();
            let loaded_doc = tokio::task::spawn_blocking(move || {
                Document::load(&working_path_for_load)
                    .map(std::sync::Arc::new)
                    .map_err(|e| format!("Lopdf Load Error (fallback) for {}: {}", path_for_load, e))
            })
            .await
            .map_err(|e| format!("Fallback lopdf load join error: {}", e))??;

            {
                let mut cache = app_state.pdf_documents.lock().unwrap();
                cache.insert(path.clone(), loaded_doc.clone());
            }

            {
                let mut loading = app_state.loading_docs.lock().unwrap();
                loading.remove(&path);
            }

            loaded_doc
        };

        let mut model = tokio::task::spawn_blocking(move || {
            crate::infrastructure::pdf::vector_engine::get_vector_page_model_with_doc(
                &lopdf_doc, page_index,
            )
        })
        .await
        .map_err(|e| format!(" Spawn Error: {}", e))??;
        model.background_image = background_image;
        {
            let mut cache = app_state.pdf_page_cache.lock().unwrap();
            cache.insert(cache_key, std::sync::Arc::new(model.clone()));
        }
        Ok(model)
    }

    pub async fn get_vector_page_model(
        state: tauri::State<'_, crate::AppState>,
        path: String,
        page_index: u16,
        target_zoom: f32,
    ) -> Result<VectorPageModel, String> {
        Self::get_vector_page_model_from_app_state(&state, path, page_index, target_zoom).await
    }

    pub async fn get_light_page_model(
        state: tauri::State<'_, crate::AppState>,
        path: String,
        page_index: u16,
    ) -> Result<LightPageModel, String> {
        log_step!("[PDF][get_light_page_model] START page={}", page_index);
        let total_start = std::time::Instant::now();
        let working_path = path.clone();
        log_step!(
            "[PDF][get_light_page_model] Using original path for read-only preview: {}",
            working_path
        );
        let cache_key = light_page_cache_key(&path, page_index);

        if let Some(model) = {
            let cache = state.pdf_light_page_cache.lock().unwrap();
            cache.get(&cache_key).cloned()
        } {
            log_step!("[PDF][LightPageCache] HIT for {}", cache_key);
            log_step!(
                "[PDF][get_light_page_model] TOTAL {:?} (cache hit)",
                total_start.elapsed()
            );
            return Ok((*model).clone());
        }

        let lopdf_doc = {
            let cache = state.pdf_documents.lock().unwrap();
            cache.get(&path).cloned()
        };

        let lopdf_doc = if let Some(doc) = lopdf_doc {
            doc
        } else {
            let loading_error = {
                let loading = state.loading_docs.lock().unwrap();
                match loading.get(&path) {
                    Some(crate::state::LoadingStatus::Error(err)) => Some(err.clone()),
                    _ => None,
                }
            };
            if let Some(err) = loading_error {
                return Err(format!("PDF background load failed for {}: {}", path, err));
            }

            log_step!(
                "[PDF][get_light_page_model] Document not ready yet for {}. Returning pending model.",
                path
            );
            log_step!(
                "[PDF][get_light_page_model] TOTAL {:?} (pending)",
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
        log_step!(
            "[PDF][get_light_page_model] build_light_page_model took {:?}",
            build_start.elapsed()
        );

        {
            let mut cache = state.pdf_light_page_cache.lock().unwrap();
            cache.insert(cache_key, std::sync::Arc::new(model.clone()));
        }

        log_step!(
            "[PDF][get_light_page_model] TOTAL {:?}",
            total_start.elapsed()
        );
        Ok(model)
    }
}
