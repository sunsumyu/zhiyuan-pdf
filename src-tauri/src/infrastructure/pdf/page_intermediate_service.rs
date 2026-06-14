use crate::infrastructure::pdf::models::{
    GlyphPaintPlan, LayoutInferenceResult, NativeVectorPageModel, PageDisplayList,
};
use std::sync::Arc;

use super::cache::page_revision_cache_key;

pub(crate) struct PageIntermediateBundle {
    pub model: NativeVectorPageModel,
    pub paint_plan: GlyphPaintPlan,
}

pub struct PdfPageIntermediateService;

impl PdfPageIntermediateService {
    pub(crate) async fn resolve_page_display_list_from_app_state(
        app_state: &crate::AppState,
        path: String,
        page_index: u16,
        document_revision: Option<u64>,
    ) -> Result<Arc<PageDisplayList>, String> {
        let cache_key = page_revision_cache_key(&path, page_index, document_revision);
        if let Some(display_list) = {
            let cache = app_state.cache.pdf_page_intermediate_cache.lock().unwrap();
            cache.get(&cache_key).cloned()
        } {
            crate::infrastructure::pdf::log_service::log_pdf_event(
                2,
                "pageIntermediate.displayListCache",
                &[
                    ("page", page_index.to_string()),
                    ("revision", document_revision.unwrap_or(0).to_string()),
                    ("result", "hit".to_string()),
                    ("objects", display_list.objects.len().to_string()),
                    ("textRuns", display_list.text_runs.len().to_string()),
                ],
            );
            return Ok(display_list);
        }

        let lopdf_doc = {
            let cache = app_state.docs.pdf_documents.lock().unwrap();
            cache.get(&path).cloned()
        };

        let lopdf_doc = if let Some(doc) = lopdf_doc {
            doc
        } else {
            let loading_error = {
                let loading = app_state.docs.loading_docs.lock().unwrap();
                match loading.get(&path) {
                    Some(crate::state::LoadingStatus::Error(err)) => Some(err.clone()),
                    _ => None,
                }
            };

            if let Some(err) = loading_error {
                return Err(format!("PDF background load failed for {}: {}", path, err));
            }

            let path_for_load = path.clone();
            let working_path = crate::infrastructure::pdf::document_service::PdfDocumentService::resolve_working_path(&path);
            let loaded_doc = tokio::task::spawn_blocking(move || {
                crate::infrastructure::pdf::document_service::load_pdf_public(&working_path)
                    .map(Arc::new)
                    .map_err(|e| {
                        format!(
                            "Lopdf Load Error (intermediate) for {}: {}",
                            path_for_load, e
                        )
                    })
            })
            .await
            .map_err(|e| format!("Intermediate lopdf load join error: {}", e))??;

            {
                let mut cache = app_state.docs.pdf_documents.lock().unwrap();
                cache.insert(path.clone(), loaded_doc.clone());
            }
            {
                let mut loading = app_state.docs.loading_docs.lock().unwrap();
                loading.remove(&path);
            }

            loaded_doc
        };

        let display_list = tokio::task::spawn_blocking(move || {
            crate::infrastructure::pdf::vector_engine::resolve_display_list(
                &lopdf_doc, page_index,
            )
        })
        .await
        .map_err(|e| format!("Intermediate display list spawn error: {}", e))??;

        let display_list = Arc::new(display_list);
        {
            let mut cache = app_state.cache.pdf_page_intermediate_cache.lock().unwrap();
            cache.insert(cache_key, display_list.clone());
        }
        crate::infrastructure::pdf::log_service::log_pdf_event(
            2,
            "pageIntermediate.displayListCache",
            &[
                ("page", page_index.to_string()),
                ("revision", document_revision.unwrap_or(0).to_string()),
                ("result", "miss".to_string()),
                ("objects", display_list.objects.len().to_string()),
                ("textRuns", display_list.text_runs.len().to_string()),
            ],
        );
        Ok(display_list)
    }

    pub(crate) async fn resolve_vector_page_model(
        state: tauri::State<'_, crate::AppState>,
        path: String,
        page_index: u16,
        _target_zoom: f32,
        document_revision: Option<u64>,
    ) -> Result<NativeVectorPageModel, String> {
        Self::resolve_vector_page_model_from_app_state(
            &state,
            path,
            page_index,
            _target_zoom,
            document_revision,
        )
        .await
    }

    pub(crate) async fn resolve_vector_page_model_from_app_state(
        app_state: &crate::AppState,
        path: String,
        page_index: u16,
        _target_zoom: f32,
        document_revision: Option<u64>,
    ) -> Result<NativeVectorPageModel, String> {
        let cache_key = page_revision_cache_key(&path, page_index, document_revision);
        if let Some(model) = {
            let cache = app_state.cache.pdf_page_cache.lock().unwrap();
            cache.get(&cache_key).cloned()
        } {
            crate::pdf_log!(2, "[PDF][PageCache] HIT for {}", cache_key);
            return Ok((*model).clone());
        }

        let display_list = Self::resolve_page_display_list_from_app_state(
            app_state,
            path,
            page_index,
            document_revision,
        )
        .await?;
        let display_list = (*display_list).clone();
        let model = tokio::task::spawn_blocking(move || {
            crate::infrastructure::pdf::vector_engine::build_vector_page_model_from_display_list(
                &display_list,
            )
        })
        .await
        .map_err(|e| format!("Intermediate vector model spawn error: {}", e))??;

        {
            let mut cache = app_state.cache.pdf_page_cache.lock().unwrap();
            cache.insert(cache_key, Arc::new(model.clone()));
        }
        Ok(model)
    }

    pub(crate) async fn resolve_layout_inference_from_app_state(
        app_state: &crate::AppState,
        path: String,
        page_index: u16,
        document_revision: Option<u64>,
    ) -> Result<LayoutInferenceResult, String> {
        let cache_key = page_revision_cache_key(&path, page_index, document_revision);
        if let Some(result) = {
            let cache = app_state.cache.pdf_layout_cache.lock().unwrap();
            cache.get(&cache_key).cloned()
        } {
            crate::pdf_log!(2, "[PDF-Cache] HIT for {}", cache_key);
            return Ok((*result).clone());
        }

        let display_list = Self::resolve_page_display_list_from_app_state(
            app_state,
            path,
            page_index,
            document_revision,
        )
        .await?;
        let display_list = (*display_list).clone();
        let result = tokio::task::spawn_blocking(move || {
            crate::infrastructure::pdf::vector_engine::resolve_layout_inference_from_display_list(
                &display_list,
            )
        })
        .await
        .map_err(|e| format!("Intermediate layout inference spawn error: {}", e))??;

        {
            let mut cache = app_state.cache.pdf_layout_cache.lock().unwrap();
            cache.insert(cache_key, Arc::new(result.clone()));
        }
        Ok(result)
    }

    pub(crate) async fn resolve_glyph_paint_plan(
        state: tauri::State<'_, crate::AppState>,
        path: String,
        page_index: u16,
        document_revision: Option<u64>,
    ) -> Result<GlyphPaintPlan, String> {
        Self::resolve_glyph_paint_plan_from_app_state(&state, path, page_index, document_revision)
            .await
    }

    pub(crate) async fn resolve_glyph_paint_plan_from_app_state(
        app_state: &crate::AppState,
        path: String,
        page_index: u16,
        document_revision: Option<u64>,
    ) -> Result<GlyphPaintPlan, String> {
        let layout = Self::resolve_layout_inference_from_app_state(
            app_state,
            path,
            page_index,
            document_revision,
        )
        .await?;
        let plan = pdf_viewer_core::render::paint_plan::build_glyph_paint_plan(&layout);
        let region_count = plan.regions.len();
        let paragraph_count: usize = plan.regions.iter().map(|r| r.paragraphs.len()).sum();
        crate::pdf_log!(
            2,
            "[PDF][resolve_glyph_paint_plan] page={} regions={} paragraphs={} width={} height={}",
            page_index,
            region_count,
            paragraph_count,
            plan.width,
            plan.height
        );
        Ok(plan)
    }

    pub(crate) async fn resolve_page_asset_bundle(
        state: tauri::State<'_, crate::AppState>,
        path: String,
        page_index: u16,
        target_zoom: f32,
        document_revision: Option<u64>,
        image_only: Option<bool>,
        text_only: Option<bool>,
    ) -> Result<PageIntermediateBundle, String> {
        let mut model = Self::resolve_vector_page_model(
            state.clone(),
            path.clone(),
            page_index,
            target_zoom,
            document_revision,
        )
        .await?;

        if image_only.unwrap_or(false) {
            model.objects.retain(|obj| !matches!(obj, crate::infrastructure::pdf::models::RenderObject::Text(_)));
        }
        if text_only.unwrap_or(false) {
            model.objects.retain(|obj| matches!(obj, crate::infrastructure::pdf::models::RenderObject::Text(_)));
        }

        let paint_plan = if image_only.unwrap_or(false) {
            GlyphPaintPlan {
                width: model.width,
                height: model.height,
                page_index,
                ..Default::default()
            }
        } else {
            Self::resolve_glyph_paint_plan(state, path, page_index, document_revision).await?
        };

        Ok(PageIntermediateBundle { model, paint_plan })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::pdf::cache::page_revision_cache_key;
    use crate::infrastructure::pdf::models::StyledRun;

    fn styled_run(text: &str, tx: f32, ty: f32, z_index: usize) -> StyledRun {
        StyledRun {
            text: text.to_string(),
            color: "#111111".to_string(),
            tx,
            ty,
            width: text.len() as f32 * 6.0,
            font_size: 10.0,
            font_name: "Helvetica".to_string(),
            a: 1.0,
            d: 1.0,
            z_index,
            char_widths: vec![6.0; text.len()],
            ..Default::default()
        }
    }

    fn display_list(page_index: u16) -> PageDisplayList {
        let text_runs = vec![
            styled_run("Total:", 20.0, 80.0, 0),
            styled_run("42", 60.0, 80.0, 1),
        ];
        PageDisplayList {
            page_index,
            width: 200.0,
            height: 100.0,
            objects: Vec::new(),
            text_runs,
        }
    }

    #[tokio::test]
    async fn uses_seeded_display_list() {
        let _log_guard = crate::infrastructure::pdf::log_service::PDF_EVENT_LOG_MUTEX.lock().unwrap();
        crate::infrastructure::pdf::log_service::clear_pdf_event_log();
        let state = crate::AppState::new();
        let path = "cached-doc.pdf".to_string();
        let page_index = 0;
        let revision = Some(11);
        let cache_key = page_revision_cache_key(&path, page_index, revision);
        let seeded_display_list = Arc::new(display_list(page_index));
        {
            let mut cache = state.cache.pdf_page_intermediate_cache.lock().unwrap();
            cache.insert(cache_key.clone(), seeded_display_list.clone());
        }

        let model = PdfPageIntermediateService::resolve_vector_page_model_from_app_state(
            &state,
            path.clone(),
            page_index,
            1.0,
            revision,
        )
        .await
        .expect("seeded PageDisplayList should derive a vector model without loading a PDF");

        assert_eq!(model.page_index, page_index);
        assert!(
            model
                .objects
                .iter()
                .any(|object| matches!(object, crate::infrastructure::pdf::models::RenderObject::Text(text) if text.text == "Total:42")),
            "vector model should be derived from display-list text runs",
        );
        assert!(
            state
                .cache
                .pdf_page_cache
                .lock()
                .unwrap()
                .contains_key(&cache_key),
            "vector derivation should backfill the legacy page model cache",
        );

        let layout = PdfPageIntermediateService::resolve_layout_inference_from_app_state(
            &state,
            path.clone(),
            page_index,
            revision,
        )
        .await
        .expect("seeded PageDisplayList should derive layout inference");

        assert_eq!(layout.page_index, page_index);
        assert_eq!(layout.width, 200.0);
        assert!(
            state
                .cache
                .pdf_layout_cache
                .lock()
                .unwrap()
                .contains_key(&cache_key),
            "layout derivation should backfill the legacy layout cache",
        );
        assert!(
            Arc::ptr_eq(
                &seeded_display_list,
                state
                    .cache
                    .pdf_page_intermediate_cache
                    .lock()
                    .unwrap()
                    .get(&cache_key)
                    .expect("display-list cache entry should remain present"),
            ),
            "derived artifacts should reuse the seeded display-list cache entry",
        );
    }

    #[tokio::test]
    async fn shares_derived_page_model() {
        let state = crate::AppState::new();
        let path = "region-doc.pdf".to_string();
        let page_index = 0;
        let cache_key = page_revision_cache_key(&path, page_index, None);
        {
            let mut cache = state.cache.pdf_page_intermediate_cache.lock().unwrap();
            cache.insert(
                cache_key.clone(),
                Arc::new(PageDisplayList {
                    page_index,
                    width: 200.0,
                    height: 100.0,
                    objects: Vec::new(),
                    text_runs: vec![
                        styled_run("Invoice ", 20.0, 80.0, 0),
                        styled_run("ready", 70.0, 80.0, 1),
                    ],
                }),
            );
        }

        let targets = crate::application::pdf::page_annotation::list_page_annotation_targets(
            &state, &path, page_index,
        )
        .await
        .expect("annotation targets should derive from a seeded display-list cache");

        assert!(
            targets
                .targets
                .iter()
                .any(|target| target.label.contains("Invoice ready")),
            "annotation target labels should be built from the display-list-derived page model",
        );

        let page_model = PdfPageIntermediateService::resolve_vector_page_model_from_app_state(
            &state,
            path.clone(),
            page_index,
            1.0,
            None,
        )
        .await
        .expect("search should reuse the display-list-derived page model");
        let search_result = crate::application::pdf::page_search::search_page_regions(
            &page_model,
            &crate::application::pdf::page_search::PdfPageSearchRequest {
                query: "invoice".to_string(),
                case_sensitive: false,
            },
        );

        assert_eq!(search_result.total_matches, 1);
        assert!(
            state
                .cache
                .pdf_page_cache
                .lock()
                .unwrap()
                .contains_key(&cache_key),
            "annotation/search derivation should backfill the legacy page cache",
        );
    }
}
