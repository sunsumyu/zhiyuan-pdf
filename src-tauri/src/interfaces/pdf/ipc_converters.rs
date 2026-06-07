//! Shared helpers for the `pdf` interface module.
//!
//! These were originally inline in `interfaces/pdf.rs`; extracted here so each
//! domain command file can stay focused on its own concern (SRP).
//!
//! Public surface preserved: callers still reference these as
//! `crate::interfaces::pdf::ensure_document_loaded`, etc., via the re-export
//! in `interfaces/pdf/mod.rs`.

use crate::infrastructure::pdf::commands::PdfEditCommand;
use crate::infrastructure::pdf::engine::PdfDocumentService;
use crate::log_step;
use pdf_viewer_core::persistence::models::PersistableRegionPatch;

pub(crate) async fn ensure_document_loaded(
    app_state: &crate::AppState,
    path: &str,
) -> Result<(), String> {
    {
        let cache = app_state.docs.pdf_documents.lock().unwrap();
        if cache.contains_key(path) {
            return Ok(());
        }
    }

    let working_path = PdfDocumentService::resolve_working_path(path);
    let path_for_load = path.to_string();
    let loaded_doc = tokio::task::spawn_blocking(move || {
        crate::infrastructure::pdf::document_service::load_pdf_public(&working_path)
            .map(std::sync::Arc::new)
            .map_err(|e| format!("Lopdf Load Error for {}: {}", path_for_load, e))
    })
    .await
    .map_err(|e| e.to_string())??;

    let mut cache = app_state.docs.pdf_documents.lock().unwrap();
    cache.insert(path.to_string(), loaded_doc);
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

    execute_pdf_commands_with_app_state(app_state, path, page_index, commands).await
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

    execute_pdf_commands_with_app_state(app_state, path, page_index, commands).await
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

    execute_pdf_commands_with_app_state(app_state, path, page_index, commands).await
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

    execute_pdf_commands_with_app_state(app_state, path, page_index, commands).await
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

    execute_pdf_commands_with_app_state(app_state, path, page_index, commands).await
}

pub(crate) fn truncate_for_log(value: &str, limit: usize) -> String {
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
pub(crate) async fn execute_pdf_commands_with_app_state(
    app_state: &crate::AppState,
    path: String,
    page_index: u16,
    commands: Vec<Box<dyn PdfEditCommand>>,
) -> Result<(), String> {
    println!(
        ">>>>> [CORE] execute_pdf_commands | path={} | cmd_count={}",
        path,
        commands.len()
    );
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
