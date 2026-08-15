//! Annotation CRUD operations: highlight, text comment, delete.
//!
//! Extracted from `pdf_write/mod.rs` to give annotation logic its own locality.
//! Each function takes `&mut Document` + page-level params — no trait dispatch here,
//! just the raw operations that `PdfDocExt` delegates to.

use crate::infrastructure::pdf::pdf_utils;
use lopdf::{Dictionary, Document, Object};

pub(super) fn add_highlight_impl(
    doc: &mut Document,
    page_num: u32,
    rect: [f32; 4],
    color: [f32; 3],
) -> Result<(), String> {
    let page_id = *doc
        .get_pages()
        .get(&page_num)
        .ok_or_else(|| format!("Page {} not found", page_num))?;
    let page_height = read_page_height(doc, page_id)?;
    let (left, top, width, height) = (rect[0], rect[1], rect[2].max(1.0), rect[3].max(1.0));
    let (right, p_top, p_bot) = (
        left + width,
        page_height - top,
        page_height - (top + height),
    );

    let annot_id = doc.new_object_id();
    let mut dict = Dictionary::new();
    dict.set("Type", Object::Name(b"Annot".to_vec()));
    dict.set("Subtype", Object::Name(b"Highlight".to_vec()));
    dict.set(
        "Rect",
        Object::Array(vec![
            Object::Real(left),
            Object::Real(p_bot),
            Object::Real(right),
            Object::Real(p_top),
        ]),
    );
    dict.set(
        "QuadPoints",
        Object::Array(vec![
            Object::Real(left),
            Object::Real(p_top),
            Object::Real(right),
            Object::Real(p_top),
            Object::Real(left),
            Object::Real(p_bot),
            Object::Real(right),
            Object::Real(p_bot),
        ]),
    );
    dict.set(
        "C",
        Object::Array(vec![
            Object::Real(color[0]),
            Object::Real(color[1]),
            Object::Real(color[2]),
        ]),
    );
    dict.set("CA", Object::Real(0.35));
    dict.set("F", Object::Integer(4));
    dict.set("P", Object::Reference(page_id));
    doc.objects.insert(annot_id, Object::Dictionary(dict));
    append_page_annotation(doc, page_id, annot_id)
}

pub(super) fn add_text_comment_impl(
    doc: &mut Document,
    page_num: u32,
    rect: [f32; 4],
    color: [f32; 3],
    contents: &str,
) -> Result<(), String> {
    let page_id = *doc
        .get_pages()
        .get(&page_num)
        .ok_or_else(|| format!("Page {} not found", page_num))?;
    let page_height = read_page_height(doc, page_id)?;
    let (left, top, width, height) = (
        rect[0].max(0.0),
        rect[1].max(0.0),
        rect[2].max(14.0),
        rect[3].max(14.0),
    );
    let size = width.min(height).clamp(16.0, 24.0);
    let (n_left, n_top) = (left + width - size, top);
    let (n_right, p_top, p_bot) = (
        n_left + size,
        page_height - n_top,
        page_height - (n_top + size),
    );

    let annot_id = doc.new_object_id();
    let mut dict = Dictionary::new();
    dict.set("Type", Object::Name(b"Annot".to_vec()));
    dict.set("Subtype", Object::Name(b"Text".to_vec()));
    dict.set(
        "Rect",
        Object::Array(vec![
            Object::Real(n_left),
            Object::Real(p_bot),
            Object::Real(n_right),
            Object::Real(p_top),
        ]),
    );
    dict.set("Contents", Object::string_literal(contents));
    dict.set("Name", Object::Name(b"Comment".to_vec()));
    dict.set(
        "C",
        Object::Array(vec![
            Object::Real(color[0]),
            Object::Real(color[1]),
            Object::Real(color[2]),
        ]),
    );
    dict.set("Open", Object::Boolean(false));
    dict.set("F", Object::Integer(4));
    dict.set("P", Object::Reference(page_id));
    doc.objects.insert(annot_id, Object::Dictionary(dict));
    append_page_annotation(doc, page_id, annot_id)
}

pub(super) fn update_text_comment_impl(
    doc: &mut Document,
    page_num: u32,
    annot_id: (u32, u16),
    contents: &str,
) -> Result<(), String> {
    let page_id = *doc
        .get_pages()
        .get(&page_num)
        .ok_or_else(|| format!("Page {} not found", page_num))?;
    if !read_page_annotation_refs(doc, page_id)?.contains(&annot_id) {
        return Err(format!(
            "Annotation {:?} not found on page {}",
            annot_id, page_num
        ));
    }
    let dict = doc
        .get_object_mut(annot_id)
        .and_then(|obj| obj.as_dict_mut())
        .map_err(|e| e.to_string())?;
    if dict
        .get(b"Subtype")
        .ok()
        .and_then(|v| v.as_name().ok())
        .unwrap_or(b"")
        != b"Text"
    {
        return Err(format!("Annotation {:?} is not a text comment", annot_id));
    }
    dict.set("Contents", Object::string_literal(contents));
    Ok(())
}

pub(super) fn delete_annotation_impl(
    doc: &mut Document,
    page_num: u32,
    annot_id: (u32, u16),
) -> Result<(), String> {
    let page_id = *doc
        .get_pages()
        .get(&page_num)
        .ok_or_else(|| format!("Page {} not found", page_num))?;
    remove_page_annotation(doc, page_id, annot_id)?;
    doc.objects.remove(&annot_id);
    Ok(())
}

// ── Internal helpers ─────────────────────────────────────────────

pub(super) fn read_page_height(doc: &Document, id: lopdf::ObjectId) -> Result<f32, String> {
    let page_size = pdf_utils::read_page_size(doc, id);
    Ok(page_size.effective_height())
}

fn append_page_annotation(
    doc: &mut Document,
    page_id: lopdf::ObjectId,
    annot_id: lopdf::ObjectId,
) -> Result<(), String> {
    let annots = {
        let dict = doc
            .get_object(page_id)
            .map_err(|e| e.to_string())?
            .as_dict()
            .map_err(|e| e.to_string())?;
        dict.get(b"Annots").ok().cloned()
    };
    match annots {
        Some(Object::Reference(id)) => {
            doc.get_object_mut(id)
                .and_then(|v| v.as_array_mut())
                .map_err(|e| e.to_string())?
                .push(Object::Reference(annot_id));
        }
        Some(Object::Array(mut arr)) => {
            arr.push(Object::Reference(annot_id));
            doc.get_object_mut(page_id)
                .and_then(|o| o.as_dict_mut())
                .map_err(|e| e.to_string())?
                .set("Annots", Object::Array(arr));
        }
        _ => {
            doc.get_object_mut(page_id)
                .and_then(|o| o.as_dict_mut())
                .map_err(|e| e.to_string())?
                .set("Annots", Object::Array(vec![Object::Reference(annot_id)]));
        }
    }
    Ok(())
}

fn remove_page_annotation(
    doc: &mut Document,
    page_id: lopdf::ObjectId,
    annot_id: lopdf::ObjectId,
) -> Result<(), String> {
    let annots = {
        doc.get_object(page_id)
            .map_err(|e| e.to_string())?
            .as_dict()
            .map_err(|e| e.to_string())?
            .get(b"Annots")
            .ok()
            .cloned()
    };
    match annots {
        Some(Object::Reference(id)) => {
            doc.get_object_mut(id)
                .and_then(|v| v.as_array_mut())
                .map_err(|e| e.to_string())?
                .retain(|i| i.as_reference().ok() != Some(annot_id));
        }
        Some(Object::Array(arr)) => {
            let filtered = arr
                .into_iter()
                .filter(|i| i.as_reference().ok() != Some(annot_id))
                .collect::<Vec<_>>();
            let dict = doc
                .get_object_mut(page_id)
                .and_then(|o| o.as_dict_mut())
                .map_err(|e| e.to_string())?;
            if filtered.is_empty() {
                dict.remove(b"Annots");
            } else {
                dict.set("Annots", Object::Array(filtered));
            }
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn read_page_annotation_refs(
    doc: &Document,
    page_id: lopdf::ObjectId,
) -> Result<Vec<lopdf::ObjectId>, String> {
    let dict = doc
        .get_object(page_id)
        .and_then(|o| o.as_dict())
        .map_err(|e| e.to_string())?;
    match dict.get(b"Annots") {
        Ok(Object::Array(arr)) => Ok(arr.iter().filter_map(|i| i.as_reference().ok()).collect()),
        Ok(Object::Reference(id)) => Ok(doc
            .get_object(*id)
            .and_then(|v| v.as_array())
            .map_err(|e| e.to_string())?
            .iter()
            .filter_map(|i| i.as_reference().ok())
            .collect()),
        _ => Ok(vec![]),
    }
}
