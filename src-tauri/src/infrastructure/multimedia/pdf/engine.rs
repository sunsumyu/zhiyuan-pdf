use crate::infrastructure::multimedia::pdf::models::{
    GlyphPaintPlan, LayoutInferenceResult, LightPageKind, LightPageModel,
    PdfMaterializationReport, PdfModifications, VectorPageModel,
};
use crate::infrastructure::multimedia::pdf::pdf_read_service::PdfReadService;
use crate::infrastructure::multimedia::pdf::pdf_write_service::PdfWriteService;
use crate::infrastructure::multimedia::pdf::pdf_geometry_service::PdfGeometryService;
use crate::infrastructure::multimedia::pdf_read::backend::PdfReadBackend;
use crate::infrastructure::multimedia::pdf_read::scanned_backend::ScannedReadBackend;
use lazy_static::lazy_static;
use lopdf::Document;
use std::collections::HashMap;
use std::fs;
use std::sync::Mutex;
use tokio::sync::Mutex as AsyncMutex;
use crate::infrastructure::multimedia::pdf::region_materializer::build_region_materialization_plan;
use crate::log_step;

lazy_static! {
    static ref WORKING_COPIES: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
    static ref COPY_LOCKS: Mutex<HashMap<String, std::sync::Arc<Mutex<()>>>> =
        Mutex::new(HashMap::new());
    static ref PDF_OPS_LOCK: AsyncMutex<()> = AsyncMutex::new(());
}
pub struct PdfDocumentService;
impl PdfDocumentService {
pub(crate)
fn get_working_path(original_path: &str) -> String {
    PdfReadService::get_working_path(original_path)
}
}
fn page_cache_key(path: &str, page_index: u16) -> String {
    format!("{}::{}", path, page_index)
}
fn light_page_cache_key(path: &str, page_index: u16) -> String {
    format!("light::{}::{}", path, page_index)
}
fn invalidate_pdf_light_page_cache(state: &crate::AppState, path: &str) {
    let prefix = format!("light::{}::", path);
    let mut cache = state.pdf_light_page_cache.lock().unwrap();
    cache.retain(|key, _| !key.starts_with(&prefix));
    log_step!(
        "[PDF][LightPageCache] Invalidated cached light page models for {}",
        path
    );
}
fn invalidate_pdf_page_cache(state: &crate::AppState, path: &str) {
    let prefix = format!("{}::", path);
    let mut cache = state.pdf_page_cache.lock().unwrap();
    cache.retain(|key, _| !key.starts_with(&prefix));
    log_step!(
        "[PDF][PageCache] Invalidated cached page models for {}",
        path
    );
}
fn invalidate_pdf_layout_cache(state: &crate::AppState, path: &str) {
    let prefix = format!("{}::", path);
    let mut cache = state.pdf_layout_cache.lock().unwrap();
    cache.retain(|key, _| !key.starts_with(&prefix));
    log_step!(
        "[PDF][LayoutCache] Invalidated cached models for {}",
        path
    );
}
fn release_working_copy(path: &str) {
    let working_path = {
        let mut copies = WORKING_COPIES.lock().unwrap();
        copies.remove(path)
    };

    {
        let mut locks = COPY_LOCKS.lock().unwrap();
        locks.remove(path);
    }

    if let Some(working_path) = working_path {
        let _ = fs::remove_file(&working_path);
        log_step!("[PDF][Release] Removed working copy for {}", path);
    }
}
impl PdfDocumentService {
pub fn release_pdf_resources(state: &crate::AppState, path: &str) {
    {
        let mut docs = state.pdf_documents.lock().unwrap();
        docs.remove(path);
    }

    invalidate_pdf_light_page_cache(state, path);
    invalidate_pdf_page_cache(state, path);
    invalidate_pdf_layout_cache(state, path);

    {
        let mut tx = state.pdf_transactions.lock().unwrap();
        tx.remove(path);
    }
    {
        let mut redo = state.pdf_redo_transactions.lock().unwrap();
        redo.remove(path);
    }

    {
        let mut loading = state.loading_docs.lock().unwrap();
        loading.remove(path);
    }

    {
        let mut image_cache = crate::infrastructure::multimedia::pdf::models::PDF_IMAGE_CACHE
            .lock()
            .unwrap();
        image_cache.clear();
    }
    {
        let mut reports = state.pdf_materialization_reports.lock().unwrap();
        reports.remove(path);
    }

    release_working_copy(path);
    log_step!("[PDF][Release] Released PDF resources for {}", path);
}
pub fn release_all_pdf_resources(state: &crate::AppState) {
    let paths: Vec<String> = {
        let docs = state.pdf_documents.lock().unwrap();
        docs.keys().cloned().collect()
    };

    for path in paths {
        Self::release_pdf_resources(state, &path);
    }

    {
        let mut docs = state.pdf_documents.lock().unwrap();
        docs.clear();
    }
    {
        let mut page_cache = state.pdf_light_page_cache.lock().unwrap();
        page_cache.clear();
    }
    {
        let mut page_cache = state.pdf_page_cache.lock().unwrap();
        page_cache.clear();
    }
    {
        let mut page_cache = state.pdf_layout_cache.lock().unwrap();
        page_cache.clear();
    }
    {
        let mut tx = state.pdf_transactions.lock().unwrap();
        tx.clear();
    }
    {
        let mut redo = state.pdf_redo_transactions.lock().unwrap();
        redo.clear();
    }
    {
        let mut loading = state.loading_docs.lock().unwrap();
        loading.clear();
    }
    {
        let mut copies = WORKING_COPIES.lock().unwrap();
        copies.clear();
    }
    {
        let mut locks = COPY_LOCKS.lock().unwrap();
        locks.clear();
    }
    {
        let mut image_cache = crate::infrastructure::multimedia::pdf::models::PDF_IMAGE_CACHE
            .lock()
            .unwrap();
        image_cache.clear();
    }
    {
        let mut reports = state.pdf_materialization_reports.lock().unwrap();
        reports.clear();
    }

    log_step!("[PDF][Release] Released all PDF resources");
}
}
impl PdfDocumentService {
pub async fn open_pdf(
    _app_handle: tauri::AppHandle,
    state: tauri::State<'_, crate::AppState>,
    path: &str,
) -> Result<usize, String> {
    crate::prof_span!("open_pdf_fast");
    log_step!("[PDF][open_pdf][de-pdfium-trace] START for {}", path);
    log_step!("[PDF][open_pdf][de-pdfium-step1] OPEN for {}", path);
    let total_start = std::time::Instant::now();

    // 1. Check cache
    {
        let cache = state.pdf_documents.lock().unwrap();
        if let Some(doc) = cache.get(path) {
            log_step!("[PDF][open_pdf] Cache HIT (Arc).");
            let lopdf_count = doc.get_pages().len();
            if lopdf_count > 0 {
                return Ok(lopdf_count);
            }
            log_step!("[PDF][open_pdf] Cache HIT but lopdf returned 0 pages, querying pdf-rs.");
        }
    }

    {
        let mut loading = state.loading_docs.lock().unwrap();
        loading.insert(path.to_string(), crate::state::LoadingStatus::Loading);
    }

    let path_for_load = path.to_string();
    let load_start = std::time::Instant::now();
    let doc = tokio::task::spawn_blocking(move || {
        Document::load(&path_for_load).map_err(|e| format!("Lopdf Load Error: {}", e))
    })
    .await
    .map_err(|e| e.to_string())??;
    let load_elapsed = load_start.elapsed();
    let lopdf_count = doc.get_pages().len();

    {
        let mut cache = state.pdf_documents.lock().unwrap();
        cache.insert(path.to_string(), std::sync::Arc::new(doc));
    }
    {
        let mut loading = state.loading_docs.lock().unwrap();
        loading.remove(path);
    }

    log_step!("[PDF][open_pdf] lopdf load+count took {:?} pages={}", load_elapsed, lopdf_count);

    // lopdf's get_pages() can return 0 for some valid PDFs (non-standard page trees).
    // Fall back to the pdf-rs crate (ScannedReadBackend) which uses num_pages() instead.
    let count = if lopdf_count > 0 {
        lopdf_count
    } else {
        log_step!("[PDF][open_pdf][FALLBACK] lopdf returned 0 pages, trying pdf-rs (ScannedReadBackend) for path={}", path);
        let path_for_pdfrs = path.to_string();
        let pdfrs_result = tokio::task::spawn_blocking(move || {
            ScannedReadBackend::new().open(&path_for_pdfrs)
        })
        .await
        .map_err(|e| format!("pdf-rs join error: {}", e))?;

        match pdfrs_result {
            Ok(meta) => {
                log_step!("[PDF][open_pdf][FALLBACK] pdf-rs SUCCESS: page_count={}", meta.page_count);
                meta.page_count
            }
            Err(err) => {
                log_step!("[PDF][open_pdf][FALLBACK] pdf-rs FAILED: {}", err);
                return Err(format!(
                    "Both lopdf and pdf-rs failed to read pages. lopdf returned 0 pages; pdf-rs error: {}",
                    err
                ));
            }
        }
    };

    log_step!("[PDF][open_pdf] Returning Page Count: {}", count);
    log_step!("[PDF][open_pdf] TOTAL {:?}", total_start.elapsed());
    Ok(count)
}
pub async fn save_pdf(
    state: tauri::State<'_, crate::AppState>,
    path: &str,
    modifications: PdfModifications,
) -> Result<(), String> {
    log_step!("[PDF][save_pdf][V206.77] START for {}", path);
    log_step!(
        "[PDF][save_pdf][REGION_PATCHES] region_patches={} text_reflows={}",
        modifications.region_patches.len(),
        modifications.text_reflows.len()
    );
    let materialization_plan = build_region_materialization_plan(
        &modifications.region_patches,
        &modifications.text_reflows,
    );
    let materialization_report = materialization_plan.to_report(
        path,
        modifications.region_patches.len(),
        modifications.text_reflows.len(),
    );
    let effective_text_reflows = materialization_plan.effective_text_reflows;
    log_step!(
        "[PDF][save_pdf][MATERIALIZED] effective_text_reflows={}",
        effective_text_reflows.len()
    );
    let materialized_count = materialization_plan
        .decisions
        .iter()
        .filter(|d| d.status == "materialized")
        .count();
    let skipped_count = materialization_plan
        .decisions
        .iter()
        .filter(|d| d.status == "skipped")
        .count();
    log_step!(
        "[PDF][save_pdf][MATERIALIZE_REPORT] decisions={} materialized={} skipped={}",
        materialization_plan.decisions.len(),
        materialized_count,
        skipped_count
    );
    let mut by_source: HashMap<String, (usize, usize)> = HashMap::new();
    for decision in &materialization_plan.decisions {
        let entry = by_source.entry(decision.source.clone()).or_insert((0, 0));
        if decision.status == "materialized" {
            entry.0 += 1;
        } else {
            entry.1 += 1;
        }
    }
    for (source, (ok_count, skip_count)) in by_source {
        log_step!(
            "[PDF][save_pdf][MATERIALIZE_REPORT][SOURCE] source={} materialized={} skipped={}",
            source,
            ok_count,
            skip_count
        );
    }
    for decision in materialization_plan
        .decisions
        .iter()
        .filter(|d| d.status == "skipped")
    {
        log_step!(
            "[PDF][save_pdf][MATERIALIZE_REPORT][SKIP] region_id={} source={} reason={}",
            decision.region_id,
            decision.source,
            decision.reason
        );
    }

    let working_path = Self::get_working_path(path);
    let doc = {
        let mut cache = state.pdf_documents.lock().unwrap();
        if let Some(d) = cache.get(path) {
            d.clone()
        } else {
            let d =
                Document::load(&working_path).map_err(|e| format!("Lopdf Load Error: {}", e))?;
            let d_arc = std::sync::Arc::new(d);
            cache.insert(path.to_string(), d_arc.clone());
            d_arc
        }
    };

    let mut doc_new = (*doc).clone();

    // V206.78: Apply Atomic Text Reflows in Batches (V19 Optimization)
    {
use crate::infrastructure::multimedia::pdf::pdf_write::PdfDocExt;
        let mut by_page: HashMap<
            u32,
            Vec<crate::infrastructure::multimedia::pdf::models::TextReflowPatch>,
        > = HashMap::new();
        for reflow in effective_text_reflows {
            by_page
                .entry(reflow.page_index as u32 + 1)
                .or_insert_with(Vec::new)
                .push(reflow);
        }

        for (page_num, patches) in by_page {
            if let Err(e) = doc_new.apply_batch_reflow_to_doc(page_num, &patches) {
                return Err(format!(
                    "Apply batch reflow error on page {}: {}",
                    page_num, e
                ));
            }
        }
    }

    doc_new
        .save(path)
        .map_err(|e| format!("Lopdf Save Error: {}", e))?;

    // Update memory cache
    {
        let mut cache = state.pdf_documents.lock().unwrap();
        cache.insert(path.to_string(), std::sync::Arc::new(doc_new));
    }
    invalidate_pdf_light_page_cache(&state, path);
    invalidate_pdf_page_cache(&state, path);
    invalidate_pdf_layout_cache(&state, path);
    {
        let mut reports = state.pdf_materialization_reports.lock().unwrap();
        reports.insert(path.to_string(), materialization_report);
    }
    Ok(())
}
pub fn read_last_pdf_materialization_report(
    state: tauri::State<'_, crate::AppState>,
    path: &str,
) -> Result<Option<PdfMaterializationReport>, String> {
    let reports = state.pdf_materialization_reports.lock().unwrap();
    Ok(reports.get(path).cloned())
}
pub async fn rollback_pdf(
    state: tauri::State<'_, crate::AppState>,
    path: &str,
) -> Result<(), String> {
    log_step!("[PDF][rollback] Request for {}", path);
    let mut tx_cache = state.pdf_transactions.lock().unwrap();
    let mut redo_cache = state.pdf_redo_transactions.lock().unwrap();
    let mut doc_cache = state.pdf_documents.lock().unwrap();

    if let Some(history) = tx_cache.get_mut(path) {
        if let Some(prev_doc) = history.pop() {
            if let Some(current_doc) = doc_cache.get(path) {
                let redo_history = redo_cache.entry(path.to_string()).or_insert_with(Vec::new);
                redo_history.push(current_doc.clone());
                if redo_history.len() > 20 {
                    redo_history.remove(0);
                }
            }
            let mut doc_to_save = (*prev_doc).clone();
            doc_to_save
                .save(path)
                .map_err(|err| format!("Rollback disk save failed: {}", err))?;
            doc_cache.insert(path.to_string(), prev_doc);
            invalidate_pdf_light_page_cache(&state, path);
            invalidate_pdf_page_cache(&state, path);
            invalidate_pdf_layout_cache(&state, path);
            log_step!(
                "[PDF][rollback] Restored from transaction snapshot and saved to disk. Remaining history: {}",
                history.len()
            );
            return Ok(());
        }
    }
    Err("No transaction history to rollback".to_string())
}
pub async fn redo_pdf(state: tauri::State<'_, crate::AppState>, path: &str) -> Result<(), String> {
    log_step!("[PDF][redo] Request for {}", path);
    let mut tx_cache = state.pdf_transactions.lock().unwrap();
    let mut redo_cache = state.pdf_redo_transactions.lock().unwrap();
    let mut doc_cache = state.pdf_documents.lock().unwrap();

    if let Some(redo_history) = redo_cache.get_mut(path) {
        if let Some(next_doc) = redo_history.pop() {
            if let Some(current_doc) = doc_cache.get(path) {
                let history = tx_cache.entry(path.to_string()).or_insert_with(Vec::new);
                history.push(current_doc.clone());
                if history.len() > 20 {
                    history.remove(0);
                }
            }
            let mut doc_to_save = (*next_doc).clone();
            doc_to_save
                .save(path)
                .map_err(|err| format!("Redo disk save failed: {}", err))?;
            doc_cache.insert(path.to_string(), next_doc);
            invalidate_pdf_light_page_cache(&state, path);
            invalidate_pdf_page_cache(&state, path);
            invalidate_pdf_layout_cache(&state, path);
            log_step!(
                "[PDF][redo] Restored redo snapshot and saved to disk. Remaining redo: {}",
                redo_history.len()
            );
            return Ok(());
        }
    }
    Err("No redo transaction history".to_string())
}
}
pub struct PdfPageModelService;
impl PdfPageModelService {
pub(crate) async fn get_pdf_metadata_from_app_state(
    app_state: &crate::AppState,
    path: &str,
) -> Result<crate::infrastructure::multimedia::pdf::models::PdfMetadata, String> {
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
        crate::infrastructure::multimedia::pdf::models::PdfMetadata {
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
) -> Result<crate::infrastructure::multimedia::pdf::models::PdfMetadata, String> {
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

    // V151: DISABLED PDFium background for Pure Vector (Lopdf) Mode
    // let bg_path = render_pdf_page(working_path.clone(), page_index, true, target_zoom).await.ok();

    let background_image = None;
    /* match bg_path {
        Some(p) => match std::fs::read(&p) {
            Ok(bytes) => {
                log_step!("[PDF][get_vector_page_model] BG image read SUCCESS. Size: {} bytes", bytes.len());
use base64::{engine::general_purpose, Engine as _};
                Some(format!("data:image/png;base64,{}", general_purpose::STANDARD_NO_PAD.encode(bytes)))
            },
            Err(e) => {
                log_step!("[PDF][get_vector_page_model] BG image read FAILED from {}: {}", p, e);
                None
            }
        },
        None => None
    }; */

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
        crate::infrastructure::multimedia::pdf::vector_engine::get_vector_page_model_with_doc(
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
}
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
        let cache = state.pdf_layout_cache.lock().unwrap();
        cache.get(&cache_key).cloned()
    } {
        log_step!("[PDF-Cache] HIT for {}", cache_key);
        return Ok((*result).clone());
    }

    let lopdf_doc = {
        let cache = state.pdf_documents.lock().unwrap();
        cache.get(&path).cloned()
    };

    let lopdf_doc = if let Some(doc) = lopdf_doc {
        doc
    } else {
        return Err(format!("Document not found in cache for path={}", path));
    };

    let result: LayoutInferenceResult = tokio::task::spawn_blocking(move || {
        crate::infrastructure::multimedia::pdf::vector_engine::get_layout_inference(
            &lopdf_doc, page_index,
        )
    })
    .await
    .map_err(|e| format!("Spawn Error V3: {}", e))??;

    {
        let mut cache = state.pdf_layout_cache.lock().unwrap();
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
    let plan = pdf_viewer_core::paint_plan::build_glyph_paint_plan(&layout);
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
    let cache = crate::infrastructure::multimedia::pdf::models::PDF_IMAGE_CACHE
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
    session: pdf_viewer_core::models::EditorSession,
    click_x_from_anchor_left: f32,
) -> Result<usize, String> {
    Ok(
        pdf_viewer_core::glyph_layout::resolve_caret_index_for_click(
            &session,
            click_x_from_anchor_left,
        ),
    )
}
pub fn resolve_field_hit(
    request: pdf_viewer_core::models::FieldHitRequest,
) -> Result<pdf_viewer_core::models::FieldHitResolution, String> {
    Ok(pdf_viewer_core::glyph_layout::resolve_field_hit_for_click(
        &request,
    ))
}
pub fn resolve_field_hit_target(
    request: pdf_viewer_core::models::FieldHitBatchRequest,
) -> Result<Option<pdf_viewer_core::models::FieldHitMatch>, String> {
    Ok(pdf_viewer_core::glyph_layout::resolve_field_hit_target_for_click(&request))
}
pub fn resolve_field_projection(
    request: pdf_viewer_core::models::FieldProjectionRequest,
) -> Result<pdf_viewer_core::models::FieldProjection, String> {
    Ok(pdf_viewer_core::field_projection::resolve_field_projection(
        &request,
    ))
}
pub fn resolve_field_editor_params(
    request: pdf_viewer_core::models::FieldEditorParamsRequest,
) -> Result<pdf_viewer_core::models::FieldEditorParams, String> {
    Ok(pdf_viewer_core::paint_plan::build_field_editor_params(
        &request,
    ))
}
}
impl PdfPageModelService {
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
        crate::infrastructure::multimedia::pdf::preview_engine::build_light_page_model(
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
impl PdfDocumentService {
pub fn generate_demo_pdf(path: &str) -> Result<String, String> {
    let pdf = b"%PDF-1.7\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>\nendobj\n4 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n5 0 obj\n<< /Length 59 >>\nstream\nBT\n/F1 24 Tf\n100 700 Td\n(Demo) Tj\nET\nendstream\nendobj\nxref\n0 6\n0000000000 65535 f\n0000000010 00000 n\ntrailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n415\n%%EOF\n";
    fs::write(path, pdf).map_err(|_| "IO".to_string())?;
    Ok(path.to_string())
}
}

