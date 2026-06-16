use lopdf::{Document, Object};
use crate::infrastructure::pdf::pdf_utils::obj_to_f32;

#[derive(Debug, Clone)]
pub struct StoredPdfHighlight {
    pub id: String,
    pub rect: [f32; 4],
    pub color: [f32; 3],
}

#[derive(Debug, Clone)]
pub struct StoredPdfComment {
    pub id: String,
    pub rect: [f32; 4],
    pub color: [f32; 3],
    pub contents: String,
}
pub fn read_page_highlights(
    doc: &Document,
    page_num: u32,
) -> Result<Vec<StoredPdfHighlight>, String> {
    let page_id = *doc
        .get_pages()
        .get(&page_num)
        .ok_or_else(|| format!("Page {} not found", page_num))?;
    let page_height = read_page_height(doc, page_id)?;
    let annots = read_page_annotation_refs(doc, page_id)?;
    let mut highlights = Vec::new();

    for annot_id in annots {
        let Ok(annot_dict) = doc.get_dictionary(annot_id) else {
            continue;
        };
        let subtype = annot_dict
            .get(b"Subtype")
            .ok()
            .and_then(|value| value.as_name().ok())
            .unwrap_or(b"");
        if subtype != b"Highlight" {
            continue;
        }

        let Some(rect) = annot_dict
            .get(b"Rect")
            .ok()
            .and_then(|value| value.as_array().ok())
            .and_then(|items| parse_rect_array(items))
            .map(|rect| pdf_rect_to_top_down_box(rect, page_height))
        else {
            continue;
        };

        let color = annot_dict
            .get(b"C")
            .ok()
            .and_then(|value| value.as_array().ok())
            .and_then(|items| parse_color_array(items))
            .unwrap_or([1.0, 0.92, 0.4]);

        highlights.push(StoredPdfHighlight {
            id: format!("{}-{}", annot_id.0, annot_id.1),
            rect,
            color,
        });
    }

    Ok(highlights)
}
pub fn read_page_comments(doc: &Document, page_num: u32) -> Result<Vec<StoredPdfComment>, String> {
    let page_id = *doc
        .get_pages()
        .get(&page_num)
        .ok_or_else(|| format!("Page {} not found", page_num))?;
    let page_height = read_page_height(doc, page_id)?;
    let annots = read_page_annotation_refs(doc, page_id)?;
    let mut comments = Vec::new();

    for annot_id in annots {
        let Ok(annot_dict) = doc.get_dictionary(annot_id) else {
            continue;
        };
        let subtype = annot_dict
            .get(b"Subtype")
            .ok()
            .and_then(|value| value.as_name().ok())
            .unwrap_or(b"");
        if subtype != b"Text" {
            continue;
        }

        let Some(rect) = annot_dict
            .get(b"Rect")
            .ok()
            .and_then(|value| value.as_array().ok())
            .and_then(|items| parse_rect_array(items))
            .map(|rect| pdf_rect_to_top_down_box(rect, page_height))
        else {
            continue;
        };

        let color = annot_dict
            .get(b"C")
            .ok()
            .and_then(|value| value.as_array().ok())
            .and_then(|items| parse_color_array(items))
            .unwrap_or([0.42, 0.73, 0.98]);
        let contents = annot_dict
            .get(b"Contents")
            .ok()
            .and_then(|value| value.as_str().ok())
            .map(|bytes| String::from_utf8_lossy(bytes).to_string())
            .unwrap_or_default();

        comments.push(StoredPdfComment {
            id: format!("{}-{}", annot_id.0, annot_id.1),
            rect,
            color,
            contents,
        });
    }

    Ok(comments)
}
fn read_page_annotation_refs(
    doc: &Document,
    page_id: lopdf::ObjectId,
) -> Result<Vec<lopdf::ObjectId>, String> {
    let page_dict = doc.get_dictionary(page_id).map_err(|err| err.to_string())?;
    let Some(annots_obj) = page_dict.get(b"Annots").ok() else {
        return Ok(Vec::new());
    };

    match annots_obj {
        Object::Array(items) => Ok(items
            .iter()
            .filter_map(|item| item.as_reference().ok())
            .collect()),
        Object::Reference(array_id) => {
            let items = doc
                .get_object(*array_id)
                .and_then(|value| value.as_array())
                .map_err(|err| err.to_string())?;
            Ok(items
                .iter()
                .filter_map(|item| item.as_reference().ok())
                .collect())
        }
        _ => Ok(Vec::new()),
    }
}
fn read_page_height(doc: &Document, page_id: lopdf::ObjectId) -> Result<f32, String> {
    let page_dict = doc.get_dictionary(page_id).map_err(|err| err.to_string())?;
    let media_box = page_dict
        .get(b"MediaBox")
        .ok()
        .and_then(|value| value.as_array().ok())
        .ok_or_else(|| "Missing MediaBox".to_string())?;
    if media_box.len() < 4 {
        return Err("Invalid MediaBox".to_string());
    }
    let y0 = obj_to_f32(&media_box[1]).map_err(|e| e.to_string())?;
    let y1 = obj_to_f32(&media_box[3]).map_err(|e| e.to_string())?;
    Ok((y1 - y0).abs())
}
fn parse_rect_array(items: &[Object]) -> Option<[f32; 4]> {
    if items.len() < 4 {
        return None;
    }
    Some([
        obj_to_f32(&items[0]).ok()?,
        obj_to_f32(&items[1]).ok()?,
        obj_to_f32(&items[2]).ok()?,
        obj_to_f32(&items[3]).ok()?,
    ])
}
fn parse_color_array(items: &[Object]) -> Option<[f32; 3]> {
    if items.len() < 3 {
        return None;
    }
    Some([
        obj_to_f32(&items[0]).ok()?,
        obj_to_f32(&items[1]).ok()?,
        obj_to_f32(&items[2]).ok()?,
    ])
}
fn pdf_rect_to_top_down_box(rect: [f32; 4], page_height: f32) -> [f32; 4] {
    let left = rect[0].min(rect[2]);
    let right = rect[0].max(rect[2]);
    let bottom = rect[1].min(rect[3]);
    let top = rect[1].max(rect[3]);
    let width = (right - left).abs();
    let height = (top - bottom).abs();

    [left, page_height - top, width, height]
}

