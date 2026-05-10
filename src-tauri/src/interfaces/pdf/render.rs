//! Rendering commands: vector page model, glyph plans, image cache, raster tile.

use crate::infrastructure::pdf::engine::{PdfEditorGeometryService, PdfPageModelService};
use crate::infrastructure::pdf::models::{
    GlyphPaintPlan, NativeTextModel, NativeVectorPageModel, RenderObject,
};
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
pub async fn render_tile(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    page_index: u16,
    zoom: f32,
    width: u32,
    height: u32,
) -> Result<String, String> {
    // 1. Get lopdf paths and text runs from cache
    let (mut objects, runs, _pw, _ph) = {
        let cache = state.pdf_documents.lock().unwrap();
        let doc = cache
            .get(&path)
            .ok_or_else(|| format!("Doc not in cache: {}", path))?;
        crate::infrastructure::pdf::pdf_read::resolve_paths(doc, page_index as u32)
            .unwrap_or_else(|_| (Vec::new(), Vec::new(), 595.0, 842.0))
    };

    // [SLOT V3] Merge text runs into objects for the renderer
    for run in runs {
        let text_id = run
            .object_id
            .clone()
            .unwrap_or_else(|| format!("text_{}_{}", page_index, objects.len()));
        objects.push(RenderObject::Text(NativeTextModel {
            id: text_id,
            text: run.text,
            font_size: run.font_size,
            tx: run.tx,
            ty: run.ty,
            color: run.color,
            stroke_color: run.stroke_color,
            stroke_width: run.stroke_width,
            font_name: run.font_name,
            is_bold: run.is_bold,
            is_italic: run.is_italic,
            font_post_script_name: run.font_post_script_name,
            font_family_hint: run.font_family_hint,
            font_subtype: run.font_subtype,
            embedded_font_key: run.embedded_font_key,
            has_embedded_font_program: run.has_embedded_font_program,
            has_to_unicode_cmap: run.has_to_unicode_cmap,
            scale_x: run.a,
            scale_y: run.d,
            rendering_mode: run.render_mode as i32,
            char_origins: run
                .char_origins
                .into_iter()
                .map(|x| [run.tx + x, run.ty])
                .collect(),
            char_widths: run.char_widths,
            pdf_char_codes: run.pdf_char_codes,
            ..Default::default()
        }));
    }

    // 2. Get or init renderer (handle poisoned mutex from prior panics)
    let needs_init = {
        let opt = state.vello_renderer.lock().unwrap_or_else(|e| {
            eprintln!("[PDF-VELLO] Recovering poisoned mutex");
            e.into_inner()
        });
        opt.is_none()
    };
    if needs_init {
        let new_renderer =
            crate::infrastructure::pdf::vello_renderer::VelloRenderer::new().await?;
        let mut opt = state
            .vello_renderer
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if opt.is_none() {
            *opt = Some(std::sync::Arc::new(std::sync::Mutex::new(new_renderer)));
        }
    }

    let mut vello_renderer_opt = state.vello_renderer.lock().unwrap_or_else(|e| {
        eprintln!("[PDF-VELLO] Recovering poisoned mutex (render phase)");
        e.into_inner()
    });
    let renderer_arc = vello_renderer_opt
        .as_mut()
        .ok_or("Renderer initialization failed")?;

    // 3. Render at zoomed size
    let render_w = ((width as f32 * zoom) as u32).max(1);
    let render_h = ((height as f32 * zoom) as u32).max(1);
    let mut renderer = renderer_arc.lock().map_err(|e| e.to_string())?;
    let png_bytes = renderer.render_objects_to_png(&objects, render_w, render_h, zoom)?;

    // 4. Base64
    use base64::{engine::general_purpose, Engine as _};
    Ok(format!(
        "data:image/png;base64,{}",
        general_purpose::STANDARD.encode(png_bytes)
    ))
}
