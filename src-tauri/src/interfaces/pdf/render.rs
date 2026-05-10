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

