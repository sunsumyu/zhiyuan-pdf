//! Page operations: delete, rotate, insert, metadata update, image replace.
//!
//! Extracted from `pdf_write/mod.rs` to isolate page-level mutations.

use lopdf::{Document, Object};

pub(super) fn delete_page_impl(doc: &mut Document, page_num: u32) -> Result<(), String> {
    doc.delete_pages(&[page_num]);
    Ok(())
}

pub(super) fn rotate_page_impl(
    doc: &mut Document,
    page_num: u32,
    rotation: i32,
) -> Result<(), String> {
    let page_id = *doc
        .get_pages()
        .get(&page_num)
        .ok_or_else(|| format!("Page {} not found", page_num))?;
    let page_dict = doc
        .get_object_mut(page_id)
        .and_then(|obj| obj.as_dict_mut())
        .map_err(|e| format!("Get page dict error: {}", e))?;
    page_dict.set("Rotate", rotation);
    Ok(())
}

pub(super) fn insert_blank_page_impl(
    _doc: &mut Document,
    _at_index: u32,
) -> Result<(), String> {
    Err("insert_blank_page not yet implemented".to_string())
}

pub(super) fn replace_image_xobject_impl(
    _doc: &mut Document,
    _object_id: (u32, u16),
    _new_bytes: &[u8],
) -> Result<(), String> {
    Err("replace_image_xobject not yet implemented".to_string())
}

pub(super) fn update_metadata_impl(
    doc: &mut Document,
    title: &str,
    author: &str,
    subject: &str,
    keywords: &str,
) -> Result<(), String> {
    let info_id = doc
        .trailer
        .get(b"Info")
        .ok()
        .and_then(|obj| obj.as_reference().ok())
        .ok_or("No Info dict")?;
    let dict = doc
        .get_object_mut(info_id)
        .and_then(|obj| obj.as_dict_mut())
        .map_err(|e| e.to_string())?;
    dict.set("Title", Object::string_literal(title));
    dict.set("Author", Object::string_literal(author));
    dict.set("Subject", Object::string_literal(subject));
    dict.set("Keywords", Object::string_literal(keywords));
    Ok(())
}
