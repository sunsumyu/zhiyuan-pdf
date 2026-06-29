use crate::infrastructure::pdf::models::{RenderObject, StyledRun};
use crate::infrastructure::pdf::pdf_font::ResourceCache;
use crate::infrastructure::pdf::pdf_read::content_parser::parse_content_stream;
use crate::infrastructure::pdf::pdf_read::graphics_state::GraphicsState;
use crate::infrastructure::pdf::pdf_read::resource_reader::read_resources;
use lopdf::{content::Content, Document};
use std::sync::Arc;
lazy_static::lazy_static! {
    static ref PAGE_LOCKS: std::sync::Mutex<std::collections::HashMap<String, std::sync::Arc<std::sync::Mutex<()>>>> =
        std::sync::Mutex::new(std::collections::HashMap::new());
}

pub fn resolve_paths(
    doc: &Document,
    page_index: u32,
) -> Result<(Vec<RenderObject>, Vec<StyledRun>, f32, f32), String> {
    let doc_id = doc as *const Document as usize;
    let cache_key = format!("{}_{}", doc_id, page_index);

    // 1. Fast check without lock (for already-cached pages)
    if let Some(cached) = {
        let cache = crate::infrastructure::pdf::cache::PDF_RESOLVE_PATHS_CACHE
            .lock()
            .unwrap();
        cache.get(&cache_key).cloned()
    } {
        crate::log_step!(
            "[PDF-Vector][Cache] HIT for resolve_paths: key={}",
            cache_key
        );
        return Ok((*cached).clone());
    }

    // 2. Lock for this specific page to serialize concurrent duplicate requests
    let page_lock = {
        let mut locks = PAGE_LOCKS.lock().unwrap();
        locks
            .entry(cache_key.clone())
            .or_insert_with(|| std::sync::Arc::new(std::sync::Mutex::new(())))
            .clone()
    };

    let _guard = page_lock.lock().unwrap();

    // 3. Double-check cache inside the lock (in case another thread resolved it while we were waiting)
    if let Some(cached) = {
        let cache = crate::infrastructure::pdf::cache::PDF_RESOLVE_PATHS_CACHE
            .lock()
            .unwrap();
        cache.get(&cache_key).cloned()
    } {
        crate::log_step!(
            "[PDF-Vector][Cache] HIT (after lock wait) for resolve_paths: key={}",
            cache_key
        );
        return Ok((*cached).clone());
    }

    crate::log_audit!("[PDF-AUDIT] resolve_paths START page={}", page_index);
    let page_id = *doc
        .get_pages()
        .get(&(page_index + 1))
        .ok_or("Page not found")?;
    let page_dict = doc.get_dictionary(page_id).map_err(|e| e.to_string())?;

    let (mut width, mut height) = if let Ok(box_obj) = page_dict.get(b"MediaBox") {
        let arr = box_obj.as_array().map_err(|e| e.to_string())?;
        if arr.len() >= 4 {
            let w = arr[2]
                .as_float()
                .or_else(|_| arr[2].as_i64().map(|v| v as f32))
                .unwrap_or(595.0);
            let h = arr[3]
                .as_float()
                .or_else(|_| arr[3].as_i64().map(|v| v as f32))
                .unwrap_or(842.0);
            let y0 = arr[1]
                .as_float()
                .or_else(|_| arr[1].as_i64().map(|v| v as f32))
                .unwrap_or(0.0);
            ((w).abs(), (h - y0).abs())
        } else {
            (595.0, 842.0)
        }
    } else {
        (595.0, 842.0)
    };

    // Support inherited /Rotate attribute in page dictionary tree
    let mut rotation = 0i64;
    let mut current_id = page_id;
    while let Ok(dict) = doc.get_dictionary(current_id) {
        if let Ok(rotate_obj) = dict.get(b"Rotate") {
            if let Ok(r) = rotate_obj.as_i64() {
                rotation = r;
                break;
            }
        }
        if let Ok(parent_id) = dict.get(b"Parent").and_then(|o| o.as_reference()) {
            current_id = parent_id;
        } else {
            break;
        }
    }

    // Normalize rotation to 0, 90, 180, 270
    let normalized_rotation = ((rotation % 360) + 360) % 360;
    if normalized_rotation == 90 || normalized_rotation == 270 {
        std::mem::swap(&mut width, &mut height);
        crate::log_step!(
            "[PDF-Vector] swapped page size due to {} deg rotation. final={}x{}",
            normalized_rotation,
            width,
            height
        );
    }

    let flat_resources = read_resources(doc, page_id);
    let mut res_cache = ResourceCache::new();

    let content_data = doc.get_page_content(page_id).map_err(|e| e.to_string())?;
    crate::pdf_log!(
        3,
        "[PDF-DIAG] resolve_paths page={} content_bytes={}",
        page_index,
        content_data.len()
    );

    let content = Content::decode(&content_data).map_err(|e| e.to_string())?;
    crate::pdf_log!(
        3,
        "[PDF-DIAG] resolve_paths page={} ops_count={}",
        page_index,
        content.operations.len()
    );

    // Log first 20 operators for diagnostics
    let ops_preview: Vec<String> = content
        .operations
        .iter()
        .take(20)
        .map(|op| format!("{}({})", op.operator, op.operands.len()))
        .collect();
    crate::pdf_log!(
        3,
        "[PDF-DIAG] resolve_paths page={} first_ops={:?}",
        page_index,
        ops_preview
    );

    // Log XObject resource keys
    if let Some(xobjects) = flat_resources.get(b"XObject" as &[u8]) {
        let xobj_keys: Vec<String> = xobjects
            .keys()
            .map(|k| String::from_utf8_lossy(k).to_string())
            .collect();
        crate::pdf_log!(
            3,
            "[PDF-DIAG] resolve_paths page={} xobject_keys={:?}",
            page_index,
            xobj_keys
        );
    } else {
        crate::pdf_log!(
            3,
            "[PDF-DIAG] resolve_paths page={} NO XObject resources",
            page_index
        );
    }

    let mut objects = Vec::new();
    let mut text_runs = Vec::new();
    let mut obj_counter = 0;

    parse_content_stream(
        doc,
        &content,
        &flat_resources,
        &mut res_cache,
        GraphicsState::new(),
        &mut objects,
        &mut text_runs,
        &mut obj_counter,
    )?;

    crate::pdf_log!(
        2,
        "[PDF-DIAG] resolve_paths page={} RESULT objects={} text_runs={} w={} h={}",
        page_index,
        objects.len(),
        text_runs.len(),
        width,
        height
    );

    let res = (objects, text_runs, width, height);
    {
        let mut cache = crate::infrastructure::pdf::cache::PDF_RESOLVE_PATHS_CACHE
            .lock()
            .unwrap();
        cache.insert(cache_key, Arc::new(res.clone()));
    }
    Ok(res)
}
