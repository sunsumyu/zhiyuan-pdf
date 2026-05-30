//! Rendering commands: vector page model, glyph plans, image cache, raster tile.

use crate::infrastructure::pdf::engine::{PdfEditorGeometryService, PdfPageModelService};
use crate::infrastructure::pdf::models::{GlyphPaintPlan, NativeVectorPageModel};
use tauri::command;

#[command]
pub async fn read_vector(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    page_index: u16,
    target_zoom: Option<f32>,
) -> Result<NativeVectorPageModel, String> {
    PdfPageModelService::get_vector_page_model(state, path, page_index, target_zoom.unwrap_or(1.0)).await
}

#[command]
pub async fn read_glyph_plan(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    page_index: u16,
) -> Result<GlyphPaintPlan, String> {
    PdfEditorGeometryService::get_glyph_paint_plan(state, path, page_index).await
}

#[command]
pub fn read_images(
    path: String,
) -> Result<std::collections::HashMap<String, String>, String> {
    Ok(PdfEditorGeometryService::get_image_cache(&path))
}

#[command]
pub async fn diagnose_page(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    page_index: u16,
) -> Result<serde_json::Value, String> {
    use lopdf::content::Content;

    // Ensure document is loaded
    crate::interfaces::pdf::helpers::ensure_document_loaded(&state, &path).await?;

    let doc_arc = {
        let cache = state.docs.pdf_documents.lock().unwrap();
        cache.get(&path).cloned().ok_or("Doc not in cache after load")?
    };

    let pages = doc_arc.get_pages();
    let total_pages = pages.len();
    let page_id = match pages.get(&((page_index as u32) + 1)) {
        Some(id) => *id,
        None => {
            return Ok(serde_json::json!({
                "error": "Page not found",
                "pageIndex": page_index,
                "totalPages": total_pages,
            }));
        }
    };

    let page_dict = doc_arc.get_dictionary(page_id).map_err(|e| e.to_string())?;
    let page_dict_keys: Vec<String> = page_dict.iter()
        .map(|(k, _)| String::from_utf8_lossy(k).to_string())
        .collect();

    let contents_kind = match page_dict.get(b"Contents") {
        Ok(obj) => match obj {
            lopdf::Object::Reference(_) => "Reference".to_string(),
            lopdf::Object::Array(_) => "Array".to_string(),
            lopdf::Object::Stream(_) => "Stream".to_string(),
            other => format!("Other:{:?}", std::mem::discriminant(other)),
        },
        Err(_) => "Missing".to_string(),
    };

    let content_data = doc_arc.get_page_content(page_id);
    let (content_bytes, content_err) = match &content_data {
        Ok(b) => (b.len(), None),
        Err(e) => (0, Some(e.to_string())),
    };

    let mut ops_count: usize = 0;
    let mut first_ops: Vec<String> = Vec::new();
    let mut decode_err: Option<String> = None;
    if let Ok(bytes) = &content_data {
        match Content::decode(bytes) {
            Ok(content) => {
                ops_count = content.operations.len();
                first_ops = content.operations.iter().take(30).map(|op| {
                    format!("{}({})", op.operator, op.operands.len())
                }).collect();
            }
            Err(e) => decode_err = Some(e.to_string()),
        }
    }

    // Resources analysis
    let resources_kind = match page_dict.get(b"Resources") {
        Ok(obj) => match obj {
            lopdf::Object::Reference(_) => "Reference".to_string(),
            lopdf::Object::Dictionary(_) => "Dictionary".to_string(),
            other => format!("Other:{:?}", std::mem::discriminant(other)),
        },
        Err(_) => "Missing(inherited?)".to_string(),
    };

    // Run resolve_paths and capture results
    let (objects_count, text_runs_count, page_w, page_h, resolve_err) = {
        match crate::infrastructure::pdf::pdf_read::resolve_paths(&doc_arc, page_index as u32) {
            Ok((objs, runs, w, h)) => (objs.len(), runs.len(), w, h, None),
            Err(e) => (0, 0, 0.0, 0.0, Some(e)),
        }
    };

    Ok(serde_json::json!({
        "pageIndex": page_index,
        "totalPages": total_pages,
        "pageDictKeys": page_dict_keys,
        "contentsKind": contents_kind,
        "contentBytes": content_bytes,
        "contentErr": content_err,
        "opsCount": ops_count,
        "decodeErr": decode_err,
        "firstOps": first_ops,
        "resourcesKind": resources_kind,
        "resolveObjects": objects_count,
        "resolveTextRuns": text_runs_count,
        "pageWidth": page_w,
        "pageHeight": page_h,
        "resolveErr": resolve_err,
    }))
}

