//! Application-level orchestration for PDF edit commands.
//!
//! Moved from `interfaces/pdf/ipc_converters.rs` to fix the dependency inversion.
//! This module owns the transaction history, disk persistence, and cache invalidation
//! logic — application concerns that should not live in the IPC boundary.

use crate::infrastructure::pdf::commands::PdfEditCommand;
use crate::log_step;
use pdf_viewer_core::persistence::models::PersistableRegionPatch;

/// Ensure a document is loaded, then return Ok.
pub(crate) async fn ensure_document_loaded(
    app_state: &crate::AppState,
    path: &str,
) -> Result<(), String> {
    crate::infrastructure::pdf::document_resolver::ensure_loaded(app_state, path).await?;
    Ok(())
}

/// Apply a batch of region patches to the named PDF.
pub(crate) async fn execute_region_patches(
    app_state: &crate::AppState,
    path: String,
    page_index: u16,
    patches: Vec<PersistableRegionPatch>,
) -> Result<(), String> {
    use crate::infrastructure::pdf::region_materializer::build_region_materialization_plan;

    for patch in &patches {
        log_step!(
            "[V3-SAVE-CMD][patch] page={} region={} source={} targets={:?} text='{}'",
            patch.page_index,
            patch.region_id,
            patch.source,
            patch.target_indices,
            truncate_for_log(&patch.new_text, 64)
        );
    }

    let materialization_plan = build_region_materialization_plan(&patches, &[]);
    let mut commands: Vec<Box<dyn PdfEditCommand>> = Vec::new();

    if !materialization_plan.effective_text_reflows.is_empty() {
        commands.push(Box::new(
            crate::infrastructure::pdf::commands::BatchTextReflowCommand {
                patches: materialization_plan.effective_text_reflows,
            },
        ));
    }

    execute_commands(app_state, path, page_index, commands).await
}

pub(crate) async fn apply_highlight_annotation(
    app_state: &crate::AppState,
    path: String,
    page_index: u16,
    rect: [f32; 4],
    color: [f32; 3],
) -> Result<(), String> {
    let commands: Vec<Box<dyn PdfEditCommand>> = vec![Box::new(
        crate::infrastructure::pdf::commands::AddHighlightCommand {
            page_num: (page_index + 1) as u32,
            rect,
            color,
        },
    )];

    execute_commands(app_state, path, page_index, commands).await
}

pub(crate) async fn apply_text_comment(
    app_state: &crate::AppState,
    path: String,
    page_index: u16,
    rect: [f32; 4],
    color: [f32; 3],
    contents: String,
) -> Result<(), String> {
    let commands: Vec<Box<dyn PdfEditCommand>> = vec![Box::new(
        crate::infrastructure::pdf::commands::AddCommentCommand {
            page_num: (page_index + 1) as u32,
            rect,
            color,
            contents,
        },
    )];

    execute_commands(app_state, path, page_index, commands).await
}

pub(crate) async fn delete_annotation_internal(
    app_state: &crate::AppState,
    path: String,
    page_index: u16,
    annot_id: (u32, u16),
) -> Result<(), String> {
    let commands: Vec<Box<dyn PdfEditCommand>> = vec![Box::new(
        crate::infrastructure::pdf::commands::DeleteAnnotationCommand {
            page_num: (page_index + 1) as u32,
            annot_id,
        },
    )];

    execute_commands(app_state, path, page_index, commands).await
}

pub(crate) async fn update_text_comment(
    app_state: &crate::AppState,
    path: String,
    page_index: u16,
    annot_id: (u32, u16),
    contents: String,
) -> Result<(), String> {
    let commands: Vec<Box<dyn PdfEditCommand>> = vec![Box::new(
        crate::infrastructure::pdf::commands::UpdateCommentCommand {
            page_num: (page_index + 1) as u32,
            annot_id,
            contents,
        },
    )];

    execute_commands(app_state, path, page_index, commands).await
}

fn truncate_for_log(value: &str, limit: usize) -> String {
    let mut chars = value.chars();
    let mut out = String::new();
    for _ in 0..limit {
        if let Some(ch) = chars.next() {
            out.push(ch);
        } else {
            break;
        }
    }
    if chars.next().is_some() {
        out.push_str("...");
    }
    out
}

/// Apply commands to the in-memory document, persist to disk, refresh caches.
async fn execute_commands(
    app_state: &crate::AppState,
    path: String,
    page_index: u16,
    commands: Vec<Box<dyn PdfEditCommand>>,
) -> Result<(), String> {
    let save_path = path.clone();

    // 1. Manage Transaction History for Undo
    {
        let docs = app_state.docs.pdf_documents.lock().unwrap();
        let mut txs = app_state.history.pdf_transactions.lock().unwrap();
        if let Some(current_doc) = docs.get(&save_path) {
            let history = txs.entry(save_path.clone()).or_insert_with(Vec::new);
            history.push(current_doc.clone());
            if history.len() > 20 {
                history.remove(0);
            }
            log_step!(
                "[PDF-SAVE] Pushed current doc to history. Stack size: {}",
                history.len()
            );
        }
    }
    {
        let mut redo = app_state.history.pdf_redo_transactions.lock().unwrap();
        redo.remove(&save_path);
    }

    // 2. Apply commands to in-memory clone
    let mut new_doc = {
        let docs = app_state.docs.pdf_documents.lock().unwrap();
        let current_doc = docs
            .get(&save_path)
            .ok_or_else(|| "Document not found in cache".to_string())?;
        let doc_clone = (**current_doc).clone();
        crate::infrastructure::pdf::save_engine::apply_pdf_commands(
            doc_clone, page_index, commands,
        )?
    };

    // 3. Save to disk
    new_doc
        .save(&save_path)
        .map_err(|e| format!("Disk Save Failure: {}", e))?;
    log_step!("[PDF-SAVE] Saved to disk: {}", save_path);

    // 4. Update memory cache and invalidate view caches
    {
        let mut docs = app_state.docs.pdf_documents.lock().unwrap();
        docs.insert(save_path.clone(), std::sync::Arc::new(new_doc));
    }

    let light_prefix = format!("light::{}::", save_path);
    let mut light_page_cache = app_state.cache.pdf_light_page_cache.lock().unwrap();
    light_page_cache.retain(|key, _| !key.starts_with(&light_prefix));
    drop(light_page_cache);

    let prefix = format!("{}::", save_path);
    let mut intermediate_cache = app_state.cache.pdf_page_intermediate_cache.lock().unwrap();
    intermediate_cache.retain(|key, _| !key.starts_with(&prefix));
    drop(intermediate_cache);

    let mut page_cache = app_state.cache.pdf_page_cache.lock().unwrap();
    page_cache.retain(|key, _| !key.starts_with(&prefix));
    drop(page_cache);

    let mut layout_cache = app_state.cache.pdf_layout_cache.lock().unwrap();
    layout_cache.retain(|key: &String, _| !key.starts_with(&prefix));
    drop(layout_cache);

    Ok(())
}
