use crate::infrastructure::pdf::models::PdfModifications;
use crate::infrastructure::pdf::pdf_read_service::PdfReadService;
use crate::infrastructure::pdf_read::backend::PdfReadBackend;
use crate::infrastructure::pdf_read::scanned_backend::ScannedReadBackend;
use crate::infrastructure::pdf::region_materializer::build_region_materialization_plan;
use crate::log_step;
use lopdf::Document;
use std::collections::HashMap;
use std::fs;
use std::sync::Mutex;
use tokio::sync::Mutex as AsyncMutex;
use lazy_static::lazy_static;

use super::cache::{
    invalidate_pdf_layout_cache, invalidate_pdf_light_page_cache, invalidate_pdf_page_cache,
};

lazy_static! {
    static ref WORKING_COPIES: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
    static ref COPY_LOCKS: Mutex<HashMap<String, std::sync::Arc<Mutex<()>>>> =
        Mutex::new(HashMap::new());
    static ref PDF_OPS_LOCK: AsyncMutex<()> = AsyncMutex::new(());
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

pub struct PdfDocumentService;

impl PdfDocumentService {
    pub(crate) fn get_working_path(original_path: &str) -> String {
        PdfReadService::get_working_path(original_path)
    }

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
            let mut image_cache = crate::infrastructure::pdf::cache::PDF_IMAGE_CACHE
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
            let mut image_cache = crate::infrastructure::pdf::cache::PDF_IMAGE_CACHE
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

        {
            use crate::infrastructure::pdf::pdf_write::PdfDocExt;
            let mut by_page: HashMap<
                u32,
                Vec<crate::infrastructure::pdf::models::TextReflowPatch>,
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
    ) -> Result<Option<crate::infrastructure::pdf::models::PdfMaterializationReport>, String> {
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

    pub fn generate_demo_pdf(path: &str) -> Result<String, String> {
        let pdf = b"%PDF-1.7\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>\nendobj\n4 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n5 0 obj\n<< /Length 59 >>\nstream\nBT\n/F1 24 Tf\n100 700 Td\n(Demo) Tj\nET\nendstream\nendobj\nxref\n0 6\n0000000000 65535 f\n0000000010 00000 n\ntrailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n415\n%%EOF\n";
        fs::write(path, pdf).map_err(|_| "IO".to_string())?;
        Ok(path.to_string())
    }
}
