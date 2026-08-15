//! Unified document loading and working copy management.
//!
//! Replaces three duplicate "check cache → load → cache" paths scattered across
//! `ipc_converters.rs`, `pdf_read_service.rs`, and `page_intermediate_service.rs`.
//! Also consolidates the two duplicate `WORKING_COPIES` + `COPY_LOCKS` global
//! state instances from `document_service.rs` and `pdf_read_service.rs`.

use lazy_static::lazy_static;
use lopdf::Document as LopdfDocument;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

lazy_static! {
    static ref WORKING_COPIES: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
    static ref COPY_LOCKS: Mutex<HashMap<String, Arc<Mutex<()>>>> =
        Mutex::new(HashMap::new());
}

/// Resolve the working copy path for a given original path.
/// Creates the working copy if it doesn't exist.
pub(crate) fn resolve_working_path(original_path: &str) -> String {
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
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone();
        (wp, l)
    };

    // Atomic copy protection
    let _guard = lock.lock().unwrap();
    if !std::path::Path::new(&working_path).exists() {
        let copy_start = std::time::Instant::now();
        crate::log_step!(
            "[WORKING-PATH] Copying {} -> {}",
            original_path,
            working_path
        );
        if let Err(e) = std::fs::copy(original_path, &working_path) {
            crate::log_step!("[WORKING-PATH] Copy failed: {}", e);
        }
        crate::log_step!("[WORKING-PATH] Copy took {:?}", copy_start.elapsed());
    }
    crate::log_step!("[WORKING-PATH] Total took {:?}", total_start.elapsed());
    working_path
}

/// Release the working copy for a path, deleting the temp file.
pub(crate) fn release_working_copy(path: &str) {
    let working_path = {
        let mut copies = WORKING_COPIES.lock().unwrap();
        copies.remove(path)
    };

    {
        let mut locks = COPY_LOCKS.lock().unwrap();
        locks.remove(path);
    }

    if let Some(working_path) = working_path {
        let _ = std::fs::remove_file(&working_path);
        crate::log_step!("[PDF][Release] Removed working copy for {}", path);
    }
}

/// Release all working copies, deleting temp files.
pub(crate) fn release_all_working_copies() {
    let copies: HashMap<String, String> = {
        let mut copies = WORKING_COPIES.lock().unwrap();
        std::mem::take(&mut *copies)
    };

    {
        let mut locks = COPY_LOCKS.lock().unwrap();
        locks.clear();
    }

    for (path, working_path) in copies {
        let _ = std::fs::remove_file(&working_path);
        crate::log_step!("[PDF][Release] Removed working copy for {}", path);
    }
}

/// Ensure a document is loaded into the app state cache.
/// If already cached, returns immediately. Otherwise loads via the lenient
/// loader and caches the result. Manages loading status.
pub(crate) async fn ensure_loaded(
    app_state: &crate::AppState,
    path: &str,
) -> Result<Arc<LopdfDocument>, String> {
    // Fast path: already cached
    {
        let cache = app_state.docs.pdf_documents.lock().unwrap();
        if let Some(doc) = cache.get(path) {
            return Ok(doc.clone());
        }
    }

    // Check for background load error
    {
        let loading = app_state.docs.loading_docs.lock().unwrap();
        if let Some(crate::state::LoadingStatus::Error(err)) = loading.get(path) {
            return Err(format!("PDF background load failed for {}: {}", path, err));
        }
    }

    // Set loading status
    {
        let mut loading = app_state.docs.loading_docs.lock().unwrap();
        loading.insert(path.to_string(), crate::state::LoadingStatus::Loading);
    }

    // Load on blocking thread
    let path_for_load = path.to_string();
    let working_path = resolve_working_path(path);
    let loaded_doc = tokio::task::spawn_blocking(move || {
        crate::infrastructure::pdf::pdf_loader::load_pdf_public(&working_path)
            .map(Arc::new)
            .map_err(|e| format!("Lopdf Load Error for {}: {}", path_for_load, e))
    })
    .await
    .map_err(|e| e.to_string())??;

    // Cache the result
    {
        let mut cache = app_state.docs.pdf_documents.lock().unwrap();
        cache.insert(path.to_string(), loaded_doc.clone());
    }

    // Clear loading status
    {
        let mut loading = app_state.docs.loading_docs.lock().unwrap();
        loading.insert(path.to_string(), crate::state::LoadingStatus::Ready);
    }

    Ok(loaded_doc)
}
