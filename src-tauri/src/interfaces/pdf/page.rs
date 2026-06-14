//! Page-level read commands: page preview raster.
//!
//! Architecture (post-optimization):
//!   read_preview now uses the already-cached lopdf Document via preview_engine,
//!   eliminating the expensive pdf-rs re-parse that occurred on every page request.
//!   pdf-rs is preserved for document classification at open time and future editing.

use crate::application::pdf::page_asset::{PageAssetAdmissionService, PageAssetKind};
use crate::infrastructure::pdf::preview_engine;
use crate::infrastructure::pdf_read::types::{PagePreview, PdfDocumentKind};
use crate::log_step;
use tauri::command;

#[command]
pub async fn read_preview(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    page_index: u16,
    request_role: Option<String>,
) -> Result<PagePreview, String> {
    let total_start = std::time::Instant::now();
    let cache_key = format!("{}::{}", path, page_index);
    let role = crate::application::pdf::page_asset::PageAssetRole::from_request(request_role);
    PageAssetAdmissionService::admit_before_work(
        &state,
        &path,
        page_index,
        role,
        PageAssetKind::Preview,
    )?;

    // 1. Check preview cache first
    if let Some(preview) = state
        .cache
        .page_preview_cache
        .lock()
        .unwrap()
        .get(&cache_key)
        .cloned()
    {
        log_step!(
            "[PDF-READ][cmd][page] cache_hit=true page={} ready={} total={:?} path={}",
            page_index,
            preview.ready,
            total_start.elapsed(),
            path
        );
        PageAssetAdmissionService::admit_after_work(
            &state,
            &path,
            page_index,
            role,
            PageAssetKind::Preview,
        )?;
        return Ok(preview);
    }

    // 2. Ensure lopdf Document is loaded (reuses cached Arc<Document>)
    crate::interfaces::pdf::ensure_document_loaded(&state, &path).await?;

    // 3. Get the cached lopdf Document
    let doc_arc = {
        let cache = state.docs.pdf_documents.lock().unwrap();
        cache
            .get(&path)
            .cloned()
            .ok_or_else(|| format!("Document not in cache after load: {}", path))?
    };

    let lopdf_pages = doc_arc.get_pages().len();
    if lopdf_pages == 0 {
        crate::log_step!(
            "[PDF-READ][cmd][page] lopdf has 0 pages, falling back to pdf-rs ScannedReadBackend for {}",
            path
        );
        let path_clone = path.clone();
        let preview = tokio::task::spawn_blocking(move || {
            use crate::infrastructure::pdf_read::backend::PdfReadBackend;
            use crate::infrastructure::pdf_read::scanned_backend::ScannedReadBackend;
            ScannedReadBackend::new().read_page_preview(&path_clone, page_index)
        })
        .await
        .map_err(|e| format!("ScannedReadBackend spawn join error: {}", e))??;

        // Cache the result
        state
            .cache
            .page_preview_cache
            .lock()
            .unwrap()
            .insert(cache_key, preview.clone());

        PageAssetAdmissionService::admit_after_work(
            &state,
            &path,
            page_index,
            role,
            PageAssetKind::Preview,
        )?;
        return Ok(preview);
    }

    // 4. Build light page model using lopdf (pure memory operation, no file IO)
    let path_clone = path.clone();
    let light_model = tokio::task::spawn_blocking(move || {
        preview_engine::build_light_page_model(&doc_arc, page_index)
    })
    .await
    .map_err(|e| format!("spawn_blocking join error: {}", e))??;

    // 5. Convert LightPageModel → PagePreview
    let has_image = light_model.preview_image_url.is_some();
    let preview = PagePreview {
        doc_id: path_clone.clone(),
        page_index,
        width: light_model.width,
        height: light_model.height,
        image_url: light_model.preview_image_url,
        kind: match light_model.kind {
            crate::infrastructure::pdf::models::LightPageKind::Scanned => PdfDocumentKind::Scanned,
            _ => PdfDocumentKind::Vector,
        },
        ready: has_image,
    };

    // 6. Cache the result
    state
        .cache
        .page_preview_cache
        .lock()
        .unwrap()
        .insert(cache_key, preview.clone());
    log_step!(
        "[PDF-READ][cmd][page] cache_hit=false page={} ready={} total={:?} width={} height={} path={}",
        page_index,
        preview.ready,
        total_start.elapsed(),
        preview.width,
        preview.height,
        path_clone
    );
    PageAssetAdmissionService::admit_after_work(
        &state,
        &path,
        page_index,
        role,
        PageAssetKind::Preview,
    )?;
    Ok(preview)
}
