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


