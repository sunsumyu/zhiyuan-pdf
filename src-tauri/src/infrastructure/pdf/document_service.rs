use crate::infrastructure::pdf::models::PdfModifications;
use crate::infrastructure::pdf::region_materializer::build_region_materialization_plan;
use crate::infrastructure::pdf_read::backend::PdfReadBackend;
use crate::infrastructure::pdf_read::scanned_backend::ScannedReadBackend;
use lopdf::Document;
use std::collections::HashMap;
use std::fs;
use tokio::sync::Mutex as AsyncMutex;

use super::cache::{invalidate_pdf_layout_cache, invalidate_pdf_page_cache};
use super::pdf_loader::load_pdf_lenient;

lazy_static::lazy_static! {
    static ref PDF_OPS_LOCK: AsyncMutex<()> = AsyncMutex::new(());
}

pub struct PdfDocumentService;

impl PdfDocumentService {
    pub(crate) fn resolve_working_path(original_path: &str) -> String {
        crate::infrastructure::pdf::document_resolver::resolve_working_path(original_path)
    }

    pub fn release_pdf_resources(state: &crate::AppState, path: &str) {
        crate::infrastructure::pdf::document_resolver::release_working_copy(path);

        {
            let mut docs = state.docs.pdf_documents.lock().unwrap();
            docs.remove(path);
        }

        invalidate_pdf_page_cache(state, path);
        invalidate_pdf_layout_cache(state, path);

        {
            let mut tx = state.history.pdf_transactions.lock().unwrap();
            tx.remove(path);
        }
        {
            let mut redo = state.history.pdf_redo_transactions.lock().unwrap();
            redo.remove(path);
        }

        {
            let mut loading = state.docs.loading_docs.lock().unwrap();
            loading.remove(path);
        }

        {
            let mut image_cache = crate::infrastructure::pdf::cache::PDF_IMAGE_CACHE
                .lock()
                .unwrap();
            image_cache.clear();
        }
        {
            let mut reports = state.cache.pdf_materialization_reports.lock().unwrap();
            reports.remove(path);
        }

        crate::infrastructure::pdf::document_resolver::release_working_copy(path);
        crate::log_step!("[PDF][Release] Released PDF resources for {}", path);
    }

    pub fn release_all_pdf_resources(state: &crate::AppState) {
        let paths: Vec<String> = {
            let docs = state.docs.pdf_documents.lock().unwrap();
            docs.keys().cloned().collect()
        };

        for path in paths {
            Self::release_pdf_resources(state, &path);
        }

        {
            let mut docs = state.docs.pdf_documents.lock().unwrap();
            docs.clear();
        }
        {
            let mut page_cache = state.cache.pdf_page_intermediate_cache.lock().unwrap();
            page_cache.clear();
        }
        {
            let mut page_cache = state.cache.pdf_page_cache.lock().unwrap();
            page_cache.clear();
        }
        {
            let mut page_cache = state.cache.pdf_layout_cache.lock().unwrap();
            page_cache.clear();
        }
        {
            let mut tx = state.history.pdf_transactions.lock().unwrap();
            tx.clear();
        }
        {
            let mut redo = state.history.pdf_redo_transactions.lock().unwrap();
            redo.clear();
        }
        {
            let mut loading = state.docs.loading_docs.lock().unwrap();
            loading.clear();
        }
        crate::infrastructure::pdf::document_resolver::release_all_working_copies();
        {
            let mut image_cache = crate::infrastructure::pdf::cache::PDF_IMAGE_CACHE
                .lock()
                .unwrap();
            image_cache.clear();
        }
        {
            let mut reports = state.cache.pdf_materialization_reports.lock().unwrap();
            reports.clear();
        }

        crate::log_step!("[PDF][Release] Released all PDF resources");
    }

    pub async fn open_pdf(
        _app_handle: tauri::AppHandle,
        state: tauri::State<'_, crate::AppState>,
        path: &str,
    ) -> Result<usize, String> {
        // 1. Check cache
        {
            let cache = state.docs.pdf_documents.lock().unwrap();
            if let Some(doc) = cache.get(path) {
                crate::pdf_log!(2, "[PDF][open_pdf] Cache HIT (Arc).");
                let lopdf_count = doc.get_pages().len();
                if lopdf_count > 0 {
                    return Ok(lopdf_count);
                }
                crate::pdf_log!(
                    2,
                    "[PDF][open_pdf] Cache HIT but lopdf returned 0 pages, querying pdf-rs."
                );
            }
        }

        {
            let mut loading = state.docs.loading_docs.lock().unwrap();
            loading.insert(path.to_string(), crate::state::LoadingStatus::Loading);
        }

        let path_for_load = path.to_string();
        let load_start = std::time::Instant::now();
        let doc = tokio::task::spawn_blocking(move || load_pdf_lenient(&path_for_load))
            .await
            .map_err(|e| e.to_string())??;
        let load_elapsed = load_start.elapsed();
        let lopdf_count = doc.get_pages().len();

        {
            let mut cache = state.docs.pdf_documents.lock().unwrap();
            cache.insert(path.to_string(), std::sync::Arc::new(doc));
        }
        {
            let mut loading = state.docs.loading_docs.lock().unwrap();
            loading.remove(path);
        }

        crate::pdf_log!(
            2,
            "[PDF][open_pdf] lopdf load+count took {:?} pages={}",
            load_elapsed,
            lopdf_count
        );

        let count = if lopdf_count > 0 {
            lopdf_count
        } else {
            crate::log_step!("[PDF][open_pdf][FALLBACK] lopdf returned 0 pages, trying pdf-rs (ScannedReadBackend) for path={}", path);
            let path_for_pdfrs = path.to_string();
            let pdfrs_result = tokio::task::spawn_blocking(move || {
                ScannedReadBackend::new().open(&path_for_pdfrs)
            })
            .await
            .map_err(|e| format!("pdf-rs join error: {}", e))?;

            match pdfrs_result {
                Ok(meta) => {
                    crate::log_step!(
                        "[PDF][open_pdf][FALLBACK] pdf-rs SUCCESS: page_count={}",
                        meta.page_count
                    );
                    meta.page_count
                }
                Err(err) => {
                    crate::log_step!("[PDF][open_pdf][FALLBACK] pdf-rs FAILED: {}", err);
                    return Err(format!(
                        "Both lopdf and pdf-rs failed to read pages. lopdf returned 0 pages; pdf-rs error: {}",
                        err
                    ));
                }
            }
        };

        Ok(count)
    }

    pub async fn save_pdf(
        state: tauri::State<'_, crate::AppState>,
        path: &str,
        modifications: PdfModifications,
    ) -> Result<(), String> {
        crate::pdf_log!(2, "[PDF][save_pdf][V206.77] START for {}", path);
        crate::pdf_log!(
            2,
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
        crate::pdf_log!(
            2,
            "[PDF][save_pdf][MATERIALIZED] effective_text_reflows={}",
            effective_text_reflows.len()
        );
        crate::pdf_log!(
            2,
            "[PDF][save_pdf][MATERIALIZE_REPORT] decisions={} materialized={} skipped={}",
            materialization_report.decisions.len(),
            materialization_report.materialized_count,
            materialization_report.skipped_count
        );
        for source in &materialization_report.by_source {
            crate::pdf_log!(
                2,
                "[PDF][save_pdf][MATERIALIZE_REPORT][SOURCE] source={} materialized={} skipped={}",
                source.source,
                source.materialized,
                source.skipped
            );
        }
        for decision in materialization_report
            .decisions
            .iter()
            .filter(|d| d.status == "skipped")
        {
            crate::pdf_log!(
                2,
                "[PDF][save_pdf][MATERIALIZE_REPORT][SKIP] region_id={} source={} reason={}",
                decision.region_id,
                decision.source,
                decision.reason
            );
        }

        let working_path = Self::resolve_working_path(path);
        let doc = {
            let mut cache = state.docs.pdf_documents.lock().unwrap();
            if let Some(d) = cache.get(path) {
                d.clone()
            } else {
                let d = Document::load(&working_path)
                    .map_err(|e| format!("Lopdf Load Error: {}", e))?;
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
            let mut cache = state.docs.pdf_documents.lock().unwrap();
            cache.insert(path.to_string(), std::sync::Arc::new(doc_new));
        }
        invalidate_pdf_page_cache(&state, path);
        invalidate_pdf_layout_cache(&state, path);
        {
            let mut reports = state.cache.pdf_materialization_reports.lock().unwrap();
            reports.insert(path.to_string(), materialization_report);
        }
        Ok(())
    }

    pub async fn rollback_pdf(
        state: tauri::State<'_, crate::AppState>,
        path: &str,
    ) -> Result<(), String> {
        crate::log_step!("[PDF][rollback] Request for {}", path);
        let mut tx_cache = state.history.pdf_transactions.lock().unwrap();
        let mut redo_cache = state.history.pdf_redo_transactions.lock().unwrap();
        let mut doc_cache = state.docs.pdf_documents.lock().unwrap();

        let Some(prev_doc) =
            transfer_snapshot(&mut tx_cache, &mut redo_cache, path, doc_cache.get(path).cloned())
        else {
            return Err("No transaction history to rollback".to_string());
        };
        let mut doc_to_save = (*prev_doc).clone();
        doc_to_save
            .save(path)
            .map_err(|err| format!("Rollback disk save failed: {}", err))?;
        doc_cache.insert(path.to_string(), prev_doc);
        invalidate_pdf_page_cache(&state, path);
        invalidate_pdf_layout_cache(&state, path);
        crate::log_step!(
            "[PDF][rollback] Restored from transaction snapshot and saved to disk. Remaining history: {}",
            tx_cache.get(path).map(|h| h.len()).unwrap_or(0)
        );
        Ok(())
    }

    pub async fn redo_pdf(
        state: tauri::State<'_, crate::AppState>,
        path: &str,
    ) -> Result<(), String> {
        crate::log_step!("[PDF][redo] Request for {}", path);
        let mut tx_cache = state.history.pdf_transactions.lock().unwrap();
        let mut redo_cache = state.history.pdf_redo_transactions.lock().unwrap();
        let mut doc_cache = state.docs.pdf_documents.lock().unwrap();

        let Some(next_doc) =
            transfer_snapshot(&mut redo_cache, &mut tx_cache, path, doc_cache.get(path).cloned())
        else {
            return Err("No redo transaction history".to_string());
        };
        let mut doc_to_save = (*next_doc).clone();
        doc_to_save
            .save(path)
            .map_err(|err| format!("Redo disk save failed: {}", err))?;
        doc_cache.insert(path.to_string(), next_doc);
        invalidate_pdf_page_cache(&state, path);
        invalidate_pdf_layout_cache(&state, path);
        crate::log_step!(
            "[PDF][redo] Restored redo snapshot and saved to disk. Remaining redo: {}",
            redo_cache.get(path).map(|h| h.len()).unwrap_or(0)
        );
        Ok(())
    }

    pub fn generate_demo_pdf(path: &str) -> Result<String, String> {
        let pdf = b"%PDF-1.7\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>\nendobj\n4 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n5 0 obj\n<< /Length 59 >>\nstream\nBT\n/F1 24 Tf\n100 700 Td\n(Demo) Tj\nET\nendstream\nendobj\nxref\n0 6\n0000000000 65535 f\n0000000010 00000 n\ntrailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n415\n%%EOF\n";
        fs::write(path, pdf).map_err(|_| "IO".to_string())?;
        Ok(path.to_string())
    }
}

/// Maximum snapshots kept per path in undo/redo history.
const HISTORY_LIMIT: usize = 20;

/// Pop the newest snapshot for `path` from `from`, archiving `current` into
/// `to` (capped at [`HISTORY_LIMIT`]). Returns the snapshot to restore, if any.
fn transfer_snapshot(
    from: &mut HashMap<String, Vec<std::sync::Arc<Document>>>,
    to: &mut HashMap<String, Vec<std::sync::Arc<Document>>>,
    path: &str,
    current: Option<std::sync::Arc<Document>>,
) -> Option<std::sync::Arc<Document>> {
    let popped = from.get_mut(path)?.pop()?;
    if let Some(current) = current {
        let history = to.entry(path.to_string()).or_insert_with(Vec::new);
        history.push(current);
        if history.len() > HISTORY_LIMIT {
            history.remove(0);
        }
    }
    Some(popped)
}

#[cfg(test)]
mod history_tests {
    use super::*;

    fn blank_doc() -> std::sync::Arc<Document> {
        std::sync::Arc::new(Document::with_version("1.4"))
    }

    fn history_with(path: &str, count: usize) -> HashMap<String, Vec<std::sync::Arc<Document>>> {
        let mut map = HashMap::new();
        map.insert(
            path.to_string(),
            (0..count).map(|_| blank_doc()).collect(),
        );
        map
    }

    #[test]
    fn transfer_pops_newest_and_archives_current_into_target() {
        let path = "a.pdf";
        let mut undo = history_with(path, 2);
        let newest = undo.get(path).unwrap().last().unwrap().clone();
        let mut redo = HashMap::new();
        let current = blank_doc();

        let restored = transfer_snapshot(&mut undo, &mut redo, path, Some(current.clone()));

        assert!(std::sync::Arc::ptr_eq(&restored.unwrap(), &newest));
        assert_eq!(undo.get(path).unwrap().len(), 1);
        let archived = redo.get(path).unwrap();
        assert_eq!(archived.len(), 1);
        assert!(std::sync::Arc::ptr_eq(&archived[0], &current));
    }

    #[test]
    fn transfer_returns_none_for_empty_history() {
        let mut undo = history_with("a.pdf", 0);
        let mut redo = HashMap::new();
        assert!(transfer_snapshot(&mut undo, &mut redo, "a.pdf", Some(blank_doc())).is_none());
        assert!(
            redo.is_empty(),
            "nothing should be archived when no snapshot was popped"
        );
    }

    #[test]
    fn transfer_returns_none_for_unknown_path() {
        let mut undo = history_with("a.pdf", 1);
        let mut redo = HashMap::new();
        assert!(transfer_snapshot(&mut undo, &mut redo, "other.pdf", None).is_none());
        assert_eq!(undo.get("a.pdf").unwrap().len(), 1);
    }

    #[test]
    fn transfer_caps_target_history_at_limit() {
        let path = "a.pdf";
        let mut undo = history_with(path, 1);
        let mut redo = history_with(path, HISTORY_LIMIT);
        let oldest_kept = redo.get(path).unwrap()[1].clone();

        transfer_snapshot(&mut undo, &mut redo, path, Some(blank_doc()));

        let archived = redo.get(path).unwrap();
        assert_eq!(archived.len(), HISTORY_LIMIT);
        assert!(std::sync::Arc::ptr_eq(&archived[0], &oldest_kept));
    }

    #[test]
    fn transfer_without_current_still_pops_snapshot() {
        let path = "a.pdf";
        let mut undo = history_with(path, 1);
        let mut redo = HashMap::new();
        assert!(transfer_snapshot(&mut undo, &mut redo, path, None).is_some());
        assert!(redo.is_empty(), "no current doc means nothing archived");
    }
}
