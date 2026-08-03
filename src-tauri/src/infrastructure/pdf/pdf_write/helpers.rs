use crate::infrastructure::pdf::pdf_utils;
use lopdf::{Document, Object};

pub(crate) fn read_page_height(doc: &Document, id: lopdf::ObjectId) -> Result<f32, String> {
    let page_size = pdf_utils::read_page_size(doc, id);
    Ok(page_size.effective_height())
}

pub(crate) fn append_page_annotation(
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

pub(crate) fn remove_page_annotation(
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

pub(crate) fn read_page_annotation_refs(
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

pub(crate) fn resolve_line_color(
    line: &pdf_viewer_core::geometry::layout_engine::VisualLine,
) -> String {
    line.runs
        .iter()
        .find(|r| !r.text.is_empty())
        .map(|r| r.style.color.clone())
        .filter(|c| !c.trim().is_empty())
        .unwrap_or_else(|| "#000000".to_string())
}
pub(crate) fn resolve_line_underline(
    line: &pdf_viewer_core::geometry::layout_engine::VisualLine,
) -> bool {
    line.runs.iter().any(|r| r.style.is_underline)
}
