use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::infrastructure::pdf::models::{RenderObject, StyledRun};

lazy_static! {
    pub static ref PDF_IMAGE_CACHE: Arc<Mutex<HashMap<String, Arc<[u8]>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    pub static ref PDF_FONT_PROGRAM_CACHE: Arc<Mutex<HashMap<String, Arc<Vec<u8>>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    pub static ref PDF_RESOLVE_PATHS_CACHE: Arc<Mutex<HashMap<String, Arc<(Vec<RenderObject>, Vec<StyledRun>, f32, f32)>>>> =
        Arc::new(Mutex::new(HashMap::new()));
}

pub(crate) fn page_cache_key(path: &str, page_index: u16) -> String {
    format!("{}::{}", path, page_index)
}

pub(crate) fn page_revision_cache_key(
    path: &str,
    page_index: u16,
    document_revision: Option<u64>,
) -> String {
    match document_revision {
        Some(revision) => format!("{}::rev{}::{}", path, revision, page_index),
        None => page_cache_key(path, page_index),
    }
}

pub(crate) fn light_page_cache_key(path: &str, page_index: u16) -> String {
    format!("light::{}::{}", path, page_index)
}

pub(crate) fn invalidate_pdf_light_page_cache(state: &crate::AppState, path: &str) {
    let prefix = format!("light::{}::", path);
    let mut cache = state.cache.pdf_light_page_cache.lock().unwrap();
    cache.retain(|key, _| !key.starts_with(&prefix));
    crate::log_step!(
        "[PDF][LightPageCache] Invalidated cached light page models for {}",
        path
    );
}

pub(crate) fn invalidate_pdf_page_cache(state: &crate::AppState, path: &str) {
    let prefix = format!("{}::", path);
    {
        let mut cache = state.cache.pdf_page_intermediate_cache.lock().unwrap();
        cache.retain(|key, _| !key.starts_with(&prefix));
    }
    {
        let mut cache = state.cache.pdf_page_cache.lock().unwrap();
        cache.retain(|key, _| !key.starts_with(&prefix));
    }
    {
        let mut locks = state.cache.pdf_page_asset_locks.lock().unwrap();
        locks.retain(|key, _| !key.starts_with(&prefix));
    }

    // Clean up memory-address-based resolve_paths cache entries for this document
    if let Some(doc) = {
        let docs = state.docs.pdf_documents.lock().unwrap();
        docs.get(path).map(|d| d.clone())
    } {
        let doc_id = doc.as_ref() as *const lopdf::Document as usize;
        let mut cache = PDF_RESOLVE_PATHS_CACHE.lock().unwrap();
        cache.retain(|key, _| !key.starts_with(&format!("{}_", doc_id)));
    }

    crate::log_step!(
        "[PDF][PageCache] Invalidated cached page models and resolve_paths cache for {}",
        path
    );
}

pub(crate) fn invalidate_pdf_layout_cache(state: &crate::AppState, path: &str) {
    let prefix = format!("{}::", path);
    let mut cache = state.cache.pdf_layout_cache.lock().unwrap();
    cache.retain(|key, _| !key.starts_with(&prefix));
    crate::log_step!("[PDF][LayoutCache] Invalidated cached models for {}", path);
}
