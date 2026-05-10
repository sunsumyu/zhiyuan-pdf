//! Page-level read commands: page preview raster.

use crate::infrastructure::pdf_read::facade::PdfReadFacade;
use crate::infrastructure::pdf_read::types::PagePreview;
use crate::{log_step, pdf_log};
use tauri::command;

#[command]
pub async fn read_preview(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    page_index: u16,
) -> Result<PagePreview, String> {
    let total_start = std::time::Instant::now();
    let cache_key = format!("{}::{}", path, page_index);
    if let Some(preview) = state
        .cache.page_preview_cache
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
        return Ok(preview);
    }

    let path_for_task = path.clone();
    pdf_log!(
        2,
        "[PDF-READ][cmd][page][detail] spawn page={} path={}",
        page_index,
        path
    );
    let preview = tokio::task::spawn_blocking(move || {
        let facade = PdfReadFacade::new();
        facade.get_page_preview(&path_for_task, page_index)
    })
    .await
    .map_err(|e: tokio::task::JoinError| e.to_string())??;

    state
        .cache.page_preview_cache
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
        path
    );
    Ok(preview)
}

