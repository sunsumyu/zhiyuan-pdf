use std::sync::atomic::{AtomicU64, Ordering};

static PAGE_ASSET_TEST_DELAY_MS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PageAssetRole {
    Current,
    Prefetch,
}

impl PageAssetRole {
    pub(crate) fn from_request(value: Option<String>) -> Self {
        match value.as_deref() {
            Some("prefetch") => Self::Prefetch,
            _ => Self::Current,
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Current => "current",
            Self::Prefetch => "prefetch",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;

    #[tokio::test]
    async fn same_asset_key_waits_for_existing_inflight_work() {
        crate::infrastructure::pdf::log_service::clear_pdf_event_log();
        let state = Arc::new(crate::AppState::new());
        let first = PageAssetAdmissionService::acquire_inflight_lock(
            &state,
            "doc-a.pdf",
            2,
            Some(7),
            PageAssetRole::Current,
            PageAssetKind::PageBundle,
        )
        .await;

        let waiting_state = Arc::clone(&state);
        let (acquired_tx, mut acquired_rx) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            let _second = PageAssetAdmissionService::acquire_inflight_lock(
                &waiting_state,
                "doc-a.pdf",
                2,
                Some(7),
                PageAssetRole::Current,
                PageAssetKind::PageBundle,
            )
            .await;
            let _ = acquired_tx.send(());
        });

        assert!(
            tokio::time::timeout(Duration::from_millis(30), &mut acquired_rx)
                .await
                .is_err(),
            "same document/page/revision/kind should wait for the existing in-flight lock",
        );
        let waiting_events = crate::infrastructure::pdf::log_service::read_pdf_event_log();
        assert!(
            waiting_events
                .iter()
                .any(|line| line.contains("pageAsset.dedupeWait.begin")
                    && line.contains("kind=pageBundle")
                    && line.contains("page=2")
                    && line.contains("revision=7")),
            "waiting duplicate asset request should emit dedupe wait begin",
        );

        drop(first);
        tokio::time::timeout(Duration::from_secs(1), acquired_rx)
            .await
            .expect("waiting asset request should acquire after first lock drops")
            .expect("waiting task should report acquisition");
        let completed_events = crate::infrastructure::pdf::log_service::read_pdf_event_log();
        assert!(
            completed_events
                .iter()
                .any(|line| line.contains("pageAsset.dedupeWait.end")
                    && line.contains("kind=pageBundle")
                    && line.contains("page=2")
                    && line.contains("revision=7")
                    && line.contains("elapsedMs=")),
            "released duplicate asset request should emit dedupe wait end",
        );
    }

    #[tokio::test]
    async fn different_document_revision_uses_distinct_inflight_lock() {
        let state = crate::AppState::new();
        let _first = PageAssetAdmissionService::acquire_inflight_lock(
            &state,
            "doc-a.pdf",
            2,
            Some(7),
            PageAssetRole::Current,
            PageAssetKind::PageBundle,
        )
        .await;

        let second = tokio::time::timeout(
            Duration::from_millis(100),
            PageAssetAdmissionService::acquire_inflight_lock(
                &state,
                "doc-a.pdf",
                2,
                Some(8),
                PageAssetRole::Current,
                PageAssetKind::PageBundle,
            ),
        )
        .await;

        assert!(
            second.is_ok(),
            "different document revisions should not share an in-flight lock",
        );
    }

    #[tokio::test]
    async fn invalidating_page_cache_removes_document_asset_locks() {
        let state = crate::AppState::new();
        {
            let mut cache = state.cache.pdf_page_intermediate_cache.lock().unwrap();
            cache.insert(
                "doc-a.pdf::rev7::2".to_string(),
                Arc::new(crate::infrastructure::pdf::models::PageDisplayList {
                    page_index: 2,
                    width: 100.0,
                    height: 100.0,
                    objects: Vec::new(),
                    text_runs: Vec::new(),
                }),
            );
            cache.insert(
                "doc-b.pdf::rev7::2".to_string(),
                Arc::new(crate::infrastructure::pdf::models::PageDisplayList {
                    page_index: 2,
                    width: 100.0,
                    height: 100.0,
                    objects: Vec::new(),
                    text_runs: Vec::new(),
                }),
            );
        }
        let _doc_lock = PageAssetAdmissionService::acquire_inflight_lock(
            &state,
            "doc-a.pdf",
            2,
            Some(7),
            PageAssetRole::Current,
            PageAssetKind::PageBundle,
        )
        .await;
        let _other_doc_lock = PageAssetAdmissionService::acquire_inflight_lock(
            &state,
            "doc-b.pdf",
            2,
            Some(7),
            PageAssetRole::Current,
            PageAssetKind::PageBundle,
        )
        .await;

        crate::infrastructure::pdf::cache::invalidate_pdf_page_cache(&state, "doc-a.pdf");

        let locks = state.cache.pdf_page_asset_locks.lock().unwrap();
        assert!(
            locks.keys().all(|key| !key.starts_with("doc-a.pdf::")),
            "invalidated document asset locks should be removed",
        );
        assert!(
            locks.keys().any(|key| key.starts_with("doc-b.pdf::")),
            "unrelated document asset locks should be retained",
        );
        drop(locks);

        let cache = state.cache.pdf_page_intermediate_cache.lock().unwrap();
        assert!(
            cache.keys().all(|key| !key.starts_with("doc-a.pdf::")),
            "invalidated document intermediate cache entries should be removed",
        );
        assert!(
            cache.keys().any(|key| key.starts_with("doc-b.pdf::")),
            "unrelated document intermediate cache entries should be retained",
        );
    }

    #[test]
    fn preview_prefetch_uses_wider_runway_than_vector_assets() {
        let state = crate::AppState::new();
        PageAssetAdmissionService::mark_current_page(
            &state,
            "doc-a.pdf",
            10,
            PageAssetKind::Preview,
            "test",
        );

        let preview_in_runway = PageAssetAdmissionService::admit_before_work(
            &state,
            "doc-a.pdf",
            18,
            PageAssetRole::Prefetch,
            PageAssetKind::Preview,
        );
        assert!(preview_in_runway.is_ok());

        let preview_outside_runway = PageAssetAdmissionService::admit_before_work(
            &state,
            "doc-a.pdf",
            19,
            PageAssetRole::Prefetch,
            PageAssetKind::Preview,
        );
        assert!(preview_outside_runway.is_err());

        let vector_outside_near_window = PageAssetAdmissionService::admit_before_work(
            &state,
            "doc-a.pdf",
            13,
            PageAssetRole::Prefetch,
            PageAssetKind::VectorModel,
        );
        assert!(vector_outside_near_window.is_err());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PageAssetKind {
    Preview,
    PageBundle,
    VectorModel,
    GlyphPlan,
}

impl PageAssetKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Preview => "preview",
            Self::PageBundle => "pageBundle",
            Self::VectorModel => "vectorModel",
            Self::GlyphPlan => "glyphPlan",
        }
    }
}

pub(crate) struct PageAssetAdmissionService;

impl PageAssetAdmissionService {
    pub(crate) fn set_test_delay_ms(delay_ms: u64) {
        #[cfg(debug_assertions)]
        PAGE_ASSET_TEST_DELAY_MS.store(delay_ms.min(5_000), Ordering::SeqCst);
        #[cfg(not(debug_assertions))]
        let _ = delay_ms;
    }

    pub(crate) async fn apply_test_delay() {
        #[cfg(debug_assertions)]
        {
            let delay_ms = PAGE_ASSET_TEST_DELAY_MS.load(Ordering::SeqCst);
            if delay_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            }
        }
    }

    fn emit_event(level: u8, event: &str, fields: Vec<(&str, String)>) {
        crate::infrastructure::pdf::log_service::log_pdf_event(level, event, &fields);
    }

    fn lock_for(
        state: &crate::AppState,
        path: &str,
        page_index: u16,
        document_revision: Option<u64>,
        kind: PageAssetKind,
    ) -> std::sync::Arc<tokio::sync::Mutex<()>> {
        let key = format!(
            "{}::rev{}::{}::{}",
            path,
            document_revision.unwrap_or(0),
            page_index,
            kind.as_str()
        );
        let mut locks = state.cache.pdf_page_asset_locks.lock().unwrap();
        locks
            .entry(key)
            .or_insert_with(|| std::sync::Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    }

    pub(crate) async fn acquire_inflight_lock(
        state: &crate::AppState,
        path: &str,
        page_index: u16,
        document_revision: Option<u64>,
        role: PageAssetRole,
        kind: PageAssetKind,
    ) -> tokio::sync::OwnedMutexGuard<()> {
        let lock = Self::lock_for(state, path, page_index, document_revision, kind);
        match lock.clone().try_lock_owned() {
            Ok(guard) => guard,
            Err(_) => {
                let wait_start = std::time::Instant::now();
                Self::emit_event(
                    1,
                    "pageAsset.dedupeWait.begin",
                    vec![
                        ("role", role.as_str().to_string()),
                        ("kind", kind.as_str().to_string()),
                        ("page", page_index.to_string()),
                        ("revision", document_revision.unwrap_or(0).to_string()),
                    ],
                );
                let guard = lock.lock_owned().await;
                Self::emit_event(
                    1,
                    "pageAsset.dedupeWait.end",
                    vec![
                        ("role", role.as_str().to_string()),
                        ("kind", kind.as_str().to_string()),
                        ("page", page_index.to_string()),
                        ("revision", document_revision.unwrap_or(0).to_string()),
                        ("elapsedMs", wait_start.elapsed().as_millis().to_string()),
                    ],
                );
                guard
            }
        }
    }

    pub(crate) fn admit_before_work(
        state: &crate::AppState,
        path: &str,
        page_index: u16,
        role: PageAssetRole,
        kind: PageAssetKind,
    ) -> Result<(), String> {
        match role {
            PageAssetRole::Current => {
                Self::mark_current_page(state, path, page_index, kind, "beforeWork");
                Ok(())
            }
            PageAssetRole::Prefetch => {
                Self::admit_prefetch(state, path, page_index, kind, "beforeWork")
            }
        }
    }

    pub(crate) fn mark_current_page(
        state: &crate::AppState,
        path: &str,
        page_index: u16,
        kind: PageAssetKind,
        phase: &str,
    ) {
        let previous = {
            let mut active = state.active_pages.lock().unwrap();
            active.insert(path.to_string(), page_index)
        };
        Self::emit_event(
            2,
            "pageAsset.admit",
            vec![
                ("role", PageAssetRole::Current.as_str().to_string()),
                ("kind", kind.as_str().to_string()),
                ("page", page_index.to_string()),
                (
                    "previous",
                    previous
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "none".to_string()),
                ),
                ("phase", phase.to_string()),
                ("result", "accepted".to_string()),
            ],
        );
    }

    pub(crate) fn admit_after_wait(
        state: &crate::AppState,
        path: &str,
        page_index: u16,
        role: PageAssetRole,
        kind: PageAssetKind,
    ) -> Result<(), String> {
        Self::admit_after_work(state, path, page_index, role, kind)
    }

    pub(crate) fn admit_after_work(
        state: &crate::AppState,
        path: &str,
        page_index: u16,
        role: PageAssetRole,
        kind: PageAssetKind,
    ) -> Result<(), String> {
        match role {
            PageAssetRole::Current => {
                let active = state.active_pages.lock().unwrap();
                let latest = active.get(path).copied();
                if latest == Some(page_index) {
                    return Ok(());
                }
                Self::reject(page_index, role, kind, latest, "currentSupersededAfterWork")
            }
            PageAssetRole::Prefetch => {
                Self::admit_prefetch(state, path, page_index, kind, "afterWork")
            }
        }
    }

    fn admit_prefetch(
        state: &crate::AppState,
        path: &str,
        page_index: u16,
        kind: PageAssetKind,
        phase: &str,
    ) -> Result<(), String> {
        let active = state.active_pages.lock().unwrap();
        let latest = active.get(path).copied();
        let Some(latest_page) = latest else {
            return Self::reject(
                page_index,
                PageAssetRole::Prefetch,
                kind,
                latest,
                "noActivePage",
            );
        };

        let prefetch_window = match kind {
            PageAssetKind::Preview => 8,
            _ => 2,
        };

        if latest_page.abs_diff(page_index) <= prefetch_window {
            Self::emit_event(
                2,
                "pageAsset.admit",
                vec![
                    ("role", PageAssetRole::Prefetch.as_str().to_string()),
                    ("kind", kind.as_str().to_string()),
                    ("page", page_index.to_string()),
                    ("anchor", latest_page.to_string()),
                    ("phase", phase.to_string()),
                    ("result", "accepted".to_string()),
                ],
            );
            return Ok(());
        }

        Self::reject(
            page_index,
            PageAssetRole::Prefetch,
            kind,
            latest,
            "prefetchOutsideActivePageWindow",
        )
    }

    fn reject(
        page_index: u16,
        role: PageAssetRole,
        kind: PageAssetKind,
        latest: Option<u16>,
        reason: &str,
    ) -> Result<(), String> {
        let event = if role == PageAssetRole::Prefetch {
            "pageAsset.prefetchRejected"
        } else {
            "pageAsset.currentSuperseded"
        };
        Self::emit_event(
            1,
            event,
            vec![
                ("role", role.as_str().to_string()),
                ("kind", kind.as_str().to_string()),
                ("page", page_index.to_string()),
                (
                    "latest",
                    latest
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "none".to_string()),
                ),
                ("reason", reason.to_string()),
                ("result", "rejected".to_string()),
            ],
        );
        Err(format!("stale page asset request: {}", reason))
    }
}
