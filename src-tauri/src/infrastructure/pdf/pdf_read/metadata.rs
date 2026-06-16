use lopdf::Document;
pub fn extract_metadata(
    doc: &Document,
) -> Result<crate::infrastructure::pdf::models::PdfMetadata, String> {
    let mut meta = crate::infrastructure::pdf::models::PdfMetadata::default();

    if let Ok(info_dict) = doc
        .trailer
        .get(b"Info")
        .and_then(|o| o.as_reference())
        .and_then(|id| doc.get_object(id))
        .and_then(|o| o.as_dict())
    {
        if let Ok(t) = info_dict.get(b"Title").and_then(|o| o.as_string()) {
            meta.title = Some(t.to_string());
        }
        if let Ok(a) = info_dict.get(b"Author").and_then(|o| o.as_string()) {
            meta.author = Some(a.to_string());
        }
        if let Ok(s) = info_dict.get(b"Subject").and_then(|o| o.as_string()) {
            meta.subject = Some(s.to_string());
        }
    }

    meta.page_count = doc.get_pages().len();
    Ok(meta)
}

pub fn read_page_count(doc: &Document) -> Result<u32, String> {
    Ok(doc.get_pages().len() as u32)
}

pub fn extract_page_bbox(doc: &Document, page_index: u16) -> Result<[f32; 4], String> {
    let pages = doc.get_pages();
    let page_id = pages
        .get(&(page_index as u32 + 1))
        .copied()
        .ok_or_else(|| format!("Page {} not found", page_index))?;
    let page_dict = doc.get_dictionary(page_id).map_err(|e| e.to_string())?;
    let media = page_dict
        .get(b"MediaBox")
        .map_err(|_| "No MediaBox".to_string())?;
    let arr = media.as_array().map_err(|e| e.to_string())?;
    let mut result = [0.0f32, 0.0, 595.0, 842.0];
    for (i, obj) in arr.iter().enumerate().take(4) {
        result[i] = obj
            .as_float()
            .or_else(|_| obj.as_i64().map(|v| v as f32))
            .unwrap_or(result[i]);
    }
    Ok(result)
}


