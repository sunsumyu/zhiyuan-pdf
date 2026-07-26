//! Rendering commands: vector page model, glyph plans, image cache, raster tile.

use crate::application::pdf::page_asset::{
    PageAssetAdmissionService, PageAssetKind, PageAssetRole,
};
use crate::infrastructure::pdf::engine::{PdfEditorGeometryService, PdfPageIntermediateService};
use crate::infrastructure::pdf::models::{GlyphPaintPlan, NativeVectorPageModel};
use tauri::command;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageAssetBundle {
    pub model: NativeVectorPageModel,
    pub paint_plan: GlyphPaintPlan,
}

#[command]
pub async fn read_page_asset_bundle(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    page_index: u16,
    target_zoom: Option<f32>,
    request_role: Option<String>,
    document_revision: Option<u64>,
    image_only: Option<bool>,
    text_only: Option<bool>,
) -> Result<PageAssetBundle, String> {
    let role = PageAssetRole::from_request(request_role);
    let kind = PageAssetKind::PageBundle;
    let span = crate::infrastructure::pdf::log_service::PdfEventSpan::begin(
        1,
        "pageAsset.bundle",
        vec![
            ("role", role.as_str().to_string()),
            ("page", page_index.to_string()),
            ("revision", document_revision.unwrap_or(0).to_string()),
        ],
    );
    PageAssetAdmissionService::admit_before_work(&state, &path, page_index, role, kind)?;
    let _asset_guard = PageAssetAdmissionService::acquire_inflight_lock(
        &state,
        &path,
        page_index,
        document_revision,
        role,
        kind,
    )
    .await;
    PageAssetAdmissionService::admit_after_wait(&state, &path, page_index, role, kind)?;
    PageAssetAdmissionService::apply_test_delay().await;

    let bundle = PdfPageIntermediateService::resolve_page_asset_bundle(
        state.clone(),
        path.clone(),
        page_index,
        target_zoom.unwrap_or(1.0),
        document_revision,
        image_only,
        text_only,
    )
    .await?;
    let model = bundle.model;
    let paint_plan = bundle.paint_plan;

    PageAssetAdmissionService::admit_after_work(&state, &path, page_index, role, kind)?;
    span.finish(
        "accepted",
        vec![
            ("objects", model.objects.len().to_string()),
            ("regions", paint_plan.regions.len().to_string()),
        ],
    );
    Ok(PageAssetBundle { model, paint_plan })
}

#[command]
pub async fn read_vector(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    page_index: u16,
    target_zoom: Option<f32>,
    request_role: Option<String>,
    document_revision: Option<u64>,
    image_only: Option<bool>,
    text_only: Option<bool>,
) -> Result<NativeVectorPageModel, String> {
    let role = PageAssetRole::from_request(request_role);
    let kind = PageAssetKind::VectorModel;
    PageAssetAdmissionService::admit_before_work(&state, &path, page_index, role, kind)?;
    let _asset_guard = PageAssetAdmissionService::acquire_inflight_lock(
        &state,
        &path,
        page_index,
        document_revision,
        role,
        kind,
    )
    .await;
    PageAssetAdmissionService::admit_after_wait(&state, &path, page_index, role, kind)?;
    PageAssetAdmissionService::apply_test_delay().await;

    let mut model = PdfPageIntermediateService::resolve_vector_page_model(
        &state,
        path.clone(),
        page_index,
        target_zoom.unwrap_or(1.0),
        document_revision,
    )
    .await?;

    if image_only.unwrap_or(false) {
        model.objects.retain(|obj| {
            !matches!(
                obj,
                crate::infrastructure::pdf::models::RenderObject::Text(_)
            )
        });
    }
    if text_only.unwrap_or(false) {
        model.objects.retain(|obj| {
            matches!(
                obj,
                crate::infrastructure::pdf::models::RenderObject::Text(_)
            )
        });
    }

    PageAssetAdmissionService::admit_after_work(&state, &path, page_index, role, kind)?;
    Ok(model)
}

#[command]
pub async fn read_glyph_plan(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    page_index: u16,
    request_role: Option<String>,
    document_revision: Option<u64>,
) -> Result<GlyphPaintPlan, String> {
    let role = PageAssetRole::from_request(request_role);
    let kind = PageAssetKind::GlyphPlan;
    PageAssetAdmissionService::admit_before_work(&state, &path, page_index, role, kind)?;
    let _asset_guard = PageAssetAdmissionService::acquire_inflight_lock(
        &state,
        &path,
        page_index,
        document_revision,
        role,
        kind,
    )
    .await;
    PageAssetAdmissionService::admit_after_wait(&state, &path, page_index, role, kind)?;
    PageAssetAdmissionService::apply_test_delay().await;

    let plan = PdfPageIntermediateService::resolve_glyph_paint_plan(
        &state,
        path.clone(),
        page_index,
        document_revision,
    )
    .await?;
    PageAssetAdmissionService::admit_after_work(&state, &path, page_index, role, kind)?;
    Ok(plan)
}

#[command]
pub fn read_images(path: String) -> Result<std::collections::HashMap<String, String>, String> {
    Ok(PdfEditorGeometryService::read_image_cache(&path))
}

#[command]
pub async fn diagnose_page(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    page_index: u16,
) -> Result<serde_json::Value, String> {
    use lopdf::content::Content;

    // Ensure document is loaded
    crate::interfaces::pdf::ipc_converters::ensure_document_loaded(&state, &path).await?;

    let doc_arc = {
        let cache = state.docs.pdf_documents.lock().unwrap();
        cache
            .get(&path)
            .cloned()
            .ok_or("Doc not in cache after load")?
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
    let page_dict_keys: Vec<String> = page_dict
        .iter()
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
                first_ops = content
                    .operations
                    .iter()
                    .take(30)
                    .map(|op| format!("{}({})", op.operator, op.operands.len()))
                    .collect();
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

    let (objects_count, text_runs_count, page_w, page_h, resolve_err) =
        match PdfPageIntermediateService::resolve_page_display_list(
            &state,
            path.clone(),
            page_index,
            None,
        )
        .await
        {
            Ok(display_list) => (
                display_list.objects.len(),
                display_list.text_runs.len(),
                display_list.width,
                display_list.height,
                None,
            ),
            Err(e) => (0, 0, 0.0, 0.0, Some(e)),
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

/// Render a single PDF page to a GPU-accelerated image using vello vector renderer.
/// Returns PNG bytes as base64 string.
#[command]
pub async fn render_page_to_image(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    page_index: u16,
    zoom: f32,
    width: u32,
    height: u32,
    document_revision: Option<u64>,
) -> Result<String, String> {
    use crate::infrastructure::pdf::vello_renderer::VelloRenderer;
    use std::sync::Arc;

    // 1. Ensure document is loaded
    crate::interfaces::pdf::ensure_document_loaded(&state, &path).await?;

    // 2. Resolve vector page model
    let model = PdfPageIntermediateService::resolve_vector_page_model(
        &state,
        path.clone(),
        page_index,
        zoom,
        document_revision,
    )
    .await?;

    // 3. Check GPU texture limit and auto-downscale if needed
    let (render_width, render_height, _render_zoom) = {
        let max_dimension = 16384u32; // Common GPU max texture size
        let _max_width = width.min(max_dimension);
        let _max_height = height.min(max_dimension);
        if width > max_dimension || height > max_dimension {
            let scale = (max_dimension as f32 / width.max(height) as f32).min(1.0);
            let new_width = (width as f32 * scale) as u32;
            let new_height = (height as f32 * scale) as u32;
            let new_zoom = zoom * scale;
            eprintln!("[GPU-DOWNSCALE] {}x{} -> {}x{} (zoom: {} -> {})",
                width, height, new_width, new_height, zoom, new_zoom);
            (new_width, new_height, new_zoom)
        } else {
            (width, height, zoom)
        }
    };

    // 4. Lazy-init vello renderer if needed
    let needs_init = {
        let guard = state.renderer.vello_renderer.lock().unwrap();
        guard.is_none()
    };
    if needs_init {
        let new_renderer = VelloRenderer::new().await.map_err(|e| format!("Vello init failed: {}", e))?;
        let mut guard = state.renderer.vello_renderer.lock().unwrap();
        *guard = Some(Arc::new(std::sync::Mutex::new(new_renderer)));
    }

    // 5. Render to PNG (with timeout and fallback)
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let png_bytes = {
        let renderer_arc = state.renderer.vello_renderer.lock().unwrap().clone().unwrap();
        let mut renderer = renderer_arc.lock().unwrap();
        match renderer.render_objects_to_png(&model.objects, render_width, render_height, model.width, model.height) {
            Ok(bytes) => bytes,
            Err(e) => {
                // GPU rendering failed (e.g., texture too large), fallback to CPU rasterization
                eprintln!("[GPU-FALLBACK] Vello render failed: {}. Falling back to CPU.", e);
                return fallback_cpu_render(&model, width, height, zoom);
            }
        }
    };

    // 6. Encode as base64
    Ok(STANDARD.encode(png_bytes))
}

/// CPU fallback rendering when GPU (vello) fails.
/// Uses lopdf + headless chromium or skia in future; for now returns a simple error PNG placeholder.
fn fallback_cpu_render(
    _model: &NativeVectorPageModel,
    width: u32,
    height: u32,
    _zoom: f32,
) -> Result<String, String> {
    // For now, create a simple white PNG as placeholder
    let img = image::DynamicImage::new_rgba8(width, height);

    let mut png_data = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut png_data), image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;

    use base64::{Engine as _, engine::general_purpose::STANDARD};
    Ok(STANDARD.encode(png_data))
}
