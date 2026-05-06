use crate::log_step;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::models::EmbeddedGlyphMap;

lazy_static! {
    pub static ref PDF_IMAGE_CACHE: Arc<Mutex<HashMap<String, Arc<[u8]>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    pub static ref PDF_FONT_PROGRAM_CACHE: Arc<Mutex<HashMap<String, Arc<Vec<u8>>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    pub static ref PDF_FONT_GLYPH_MAP_CACHE: Arc<Mutex<HashMap<String, Arc<EmbeddedGlyphMap>>>> =
        Arc::new(Mutex::new(HashMap::new()));
}

pub(crate) fn page_cache_key(path: &str, page_index: u16) -> String {
    format!("{}::{}", path, page_index)
}

pub(crate) fn light_page_cache_key(path: &str, page_index: u16) -> String {
    format!("light::{}::{}", path, page_index)
}

pub(crate) fn invalidate_pdf_light_page_cache(state: &crate::AppState, path: &str) {
    let prefix = format!("light::{}::", path);
    let mut cache = state.pdf_light_page_cache.lock().unwrap();
    cache.retain(|key, _| !key.starts_with(&prefix));
    log_step!(
        "[PDF][LightPageCache] Invalidated cached light page models for {}",
        path
    );
}

pub(crate) fn invalidate_pdf_page_cache(state: &crate::AppState, path: &str) {
    let prefix = format!("{}::", path);
    let mut cache = state.pdf_page_cache.lock().unwrap();
    cache.retain(|key, _| !key.starts_with(&prefix));
    log_step!(
        "[PDF][PageCache] Invalidated cached page models for {}",
        path
    );
}

pub(crate) fn invalidate_pdf_layout_cache(state: &crate::AppState, path: &str) {
    let prefix = format!("{}::", path);
    let mut cache = state.pdf_layout_cache.lock().unwrap();
    cache.retain(|key, _| !key.starts_with(&prefix));
    log_step!(
        "[PDF][LayoutCache] Invalidated cached models for {}",
        path
    );
}
