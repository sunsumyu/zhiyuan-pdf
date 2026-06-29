use lopdf::Document;
pub fn extract_vector_page_model(
    doc: &Document,
    page_index: u16,
) -> Result<crate::infrastructure::pdf::models::NativeVectorPageModel, String> {
    crate::infrastructure::pdf::vector_engine::resolve_model(doc, page_index)
}

pub fn extract_layout_inference(
    doc: &Document,
    page_index: u16,
) -> Result<crate::infrastructure::pdf::models::LayoutInferenceResult, String> {
    crate::infrastructure::pdf::vector_engine::resolve_layout_inference(doc, page_index)
}

pub fn extract_glyph_paint_plan(
    doc: &Document,
    page_index: u16,
) -> Result<crate::infrastructure::pdf::models::GlyphPaintPlan, String> {
    crate::log_step!(
        "[PDF][extract_glyph_paint_plan] Extracting glyph paint plan for page {}",
        page_index
    );

    let pages = doc.get_pages();
    let page_id = pages
        .get(&(page_index as u32 + 1))
        .copied()
        .ok_or_else(|| format!("Page {} not found", page_index))?;
    let page_dict = doc.get_dictionary(page_id).map_err(|e| e.to_string())?;

    let (width, height) = if let Ok(box_obj) = page_dict.get(b"MediaBox") {
        if let Ok(arr) = box_obj.as_array() {
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
                (w, h - y0)
            } else {
                (595.0, 842.0)
            }
        } else {
            (595.0, 842.0)
        }
    } else {
        (595.0, 842.0)
    };

    let glyph_paint_plan = crate::infrastructure::pdf::models::GlyphPaintPlan {
        page_index,
        width,
        height,
        ..Default::default()
    };

    let content_data = doc
        .get_page_content(page_id)
        .map_err(|e| format!("Failed to get page content: {}", e))?;
    let content_str = String::from_utf8_lossy(&content_data);

    if content_str.contains("BT") && content_str.contains("ET") {
        crate::log_step!(
            "[PDF][extract_glyph_paint_plan] Found text objects in page {}",
            page_index
        );
    }

    crate::log_step!(
        "[PDF][extract_glyph_paint_plan] Successfully extracted glyph paint plan for page {}",
        page_index
    );
    Ok(glyph_paint_plan)
}
