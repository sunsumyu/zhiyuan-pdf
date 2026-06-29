use lopdf::{Dictionary, Document, Object};

pub fn replace_image_xobject(
    _doc: &mut Document,
    _object_id: (u32, u16),
    _new_bytes: &[u8],
) -> Result<(), String> {
    Err("replace_image_xobject not yet implemented".to_string())
}

pub fn delete_page(doc: &mut Document, page_num: u32) -> Result<(), String> {
    doc.delete_pages(&[page_num]);
    Ok(())
}

pub fn rotate_page(doc: &mut Document, page_num: u32, rotation: i32) -> Result<(), String> {
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

pub fn insert_blank_page(_doc: &mut Document, _at_index: u32) -> Result<(), String> {
    Err("insert_blank_page not yet implemented".to_string())
}

pub fn add_highlight(
    doc: &mut Document,
    page_num: u32,
    rect: [f32; 4],
    color: [f32; 3],
) -> Result<(), String> {
    let page_id = *doc
        .get_pages()
        .get(&page_num)
        .ok_or_else(|| format!("Page {} not found", page_num))?;
    let page_height = super::helpers::read_page_height(doc, page_id)?;
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
    super::helpers::append_page_annotation(doc, page_id, annot_id)
}

pub fn add_text_comment(
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
    let page_height = super::helpers::read_page_height(doc, page_id)?;
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
    super::helpers::append_page_annotation(doc, page_id, annot_id)
}

pub fn update_text_comment(
    doc: &mut Document,
    page_num: u32,
    annot_id: (u32, u16),
    contents: &str,
) -> Result<(), String> {
    let page_id = *doc
        .get_pages()
        .get(&page_num)
        .ok_or_else(|| format!("Page {} not found", page_num))?;
    if !super::helpers::read_page_annotation_refs(doc, page_id)?.contains(&annot_id) {
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

pub fn delete_annotation(
    doc: &mut Document,
    page_num: u32,
    annot_id: (u32, u16),
) -> Result<(), String> {
    let page_id = *doc
        .get_pages()
        .get(&page_num)
        .ok_or_else(|| format!("Page {} not found", page_num))?;
    super::helpers::remove_page_annotation(doc, page_id, annot_id)?;
    doc.objects.remove(&annot_id);
    Ok(())
}

pub fn update_metadata(
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
