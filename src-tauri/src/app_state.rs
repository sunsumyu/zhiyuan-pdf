//! Top-level application state, grouped by domain (architecture-review §2.3 / Phase 1).
//!
//! The previous flat `AppState` (11 `Mutex<HashMap>` fields) was a typical
//! "god struct" — every command handler had access to every cache, violating
//! Law of Demeter. The state is now grouped into 4 sub-stores so that each
//! handler can be reasoned about (and later refactored to take only the
//! sub-store it needs):
//!
//!   docs     — owned PDF documents + load tracking + parsed metadata
//!   cache    — derived/computed view caches (cheap to evict)
//!   history  — undo/redo transaction stacks
//!   renderer — vello GPU renderer
//!
//! Field names within each sub-store keep their original `pdf_xxx` /
//! `read_xxx` prefixes so that grep-friendly identifiers stay intact;
//! the grouping is applied at the access path level only.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::infrastructure::pdf::models::{
    LightPageModel, NativeVectorPageModel, PdfMaterializationReport,
};
use crate::infrastructure::pdf::vello_renderer::VelloRenderer;
use crate::infrastructure::pdf_read::types::{PagePreview, ReadDocumentMeta};
use crate::state::LoadingStatus;

/// Owned PDF documents and their load lifecycle.
pub struct DocumentStore {
    pub pdf_documents: Mutex<HashMap<String, Arc<lopdf::Document>>>,
    pub loading_docs: Mutex<HashMap<String, LoadingStatus>>,
    pub read_document_meta_cache: Mutex<HashMap<String, ReadDocumentMeta>>,
}

impl DocumentStore {
    fn new() -> Self {
        Self {
            pdf_documents: Mutex::new(HashMap::new()),
            loading_docs: Mutex::new(HashMap::new()),
            read_document_meta_cache: Mutex::new(HashMap::new()),
        }
    }
}

/// Derived view caches — invalidated on document mutation.
pub struct CacheStore {
    pub pdf_light_page_cache: Mutex<HashMap<String, Arc<LightPageModel>>>,
    pub pdf_page_cache: Mutex<HashMap<String, Arc<NativeVectorPageModel>>>,
    pub pdf_layout_cache: Mutex<HashMap<String, Arc<pdf_viewer_core::models::LayoutInferenceResult>>>,
    pub page_preview_cache: Mutex<HashMap<String, PagePreview>>,
    pub pdf_materialization_reports: Mutex<HashMap<String, PdfMaterializationReport>>,
}

impl CacheStore {
    fn new() -> Self {
        Self {
            pdf_light_page_cache: Mutex::new(HashMap::new()),
            pdf_page_cache: Mutex::new(HashMap::new()),
            pdf_layout_cache: Mutex::new(HashMap::new()),
            page_preview_cache: Mutex::new(HashMap::new()),
            pdf_materialization_reports: Mutex::new(HashMap::new()),
        }
    }
}

/// Undo/redo transaction history per document.
pub struct HistoryStore {
    pub pdf_transactions: Mutex<HashMap<String, Vec<Arc<lopdf::Document>>>>,
    pub pdf_redo_transactions: Mutex<HashMap<String, Vec<Arc<lopdf::Document>>>>,
}

impl HistoryStore {
    fn new() -> Self {
        Self {
            pdf_transactions: Mutex::new(HashMap::new()),
            pdf_redo_transactions: Mutex::new(HashMap::new()),
        }
    }
}

/// GPU renderer (vello) — single shared instance, lazily initialised.
pub struct RendererState {
    pub vello_renderer: Mutex<Option<Arc<Mutex<VelloRenderer>>>>,
}

impl RendererState {
    fn new() -> Self {
        Self {
            vello_renderer: Mutex::new(None),
        }
    }
}

/// Root application state — Tauri's `State<'_, AppState>` injection point.
pub struct AppState {
    pub docs: DocumentStore,
    pub cache: CacheStore,
    pub history: HistoryStore,
    pub renderer: RendererState,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            docs: DocumentStore::new(),
            cache: CacheStore::new(),
            history: HistoryStore::new(),
            renderer: RendererState::new(),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
