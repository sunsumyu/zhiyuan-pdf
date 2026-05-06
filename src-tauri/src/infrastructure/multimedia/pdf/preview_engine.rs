use lopdf::{Document, Object, ObjectId};
use std::sync::Arc;
use crate::infrastructure::multimedia::pdf::models::{
    LightPageKind, LightPageModel, PDF_IMAGE_CACHE,
};
use crate::log_step;
fn page_dimensions(doc: &Document, page_id: ObjectId) -> Result<(f32, f32), String> {
    let page_dict = doc.get_dictionary(page_id).map_err(|e| e.to_string())?;
    let media_box = page_dict
        .get(b"MediaBox")
        .and_then(|o| o.as_array())
        .map_err(|e| e.to_string())?;
    if media_box.len() < 4 {
        return Ok((595.0, 842.0));
    }

    let x0 = media_box[0]
        .as_float()
        .ok()
        .or_else(|| media_box[0].as_i64().ok().map(|v| v as f32))
        .unwrap_or(0.0);
    let y0 = media_box[1]
        .as_float()
        .ok()
        .or_else(|| media_box[1].as_i64().ok().map(|v| v as f32))
        .unwrap_or(0.0);
    let x1 = media_box[2]
        .as_float()
        .ok()
        .or_else(|| media_box[2].as_i64().ok().map(|v| v as f32))
        .unwrap_or(595.0);
    let y1 = media_box[3]
        .as_float()
        .ok()
        .or_else(|| media_box[3].as_i64().ok().map(|v| v as f32))
        .unwrap_or(842.0);
    Ok(((x1 - x0).abs(), (y1 - y0).abs()))
}
fn collect_page_xobjects(doc: &Document, page_id: ObjectId) -> Vec<ObjectId> {
    let mut result = Vec::new();
    let mut current_id = page_id;
    let mut visited = std::collections::HashSet::new();

    while let Ok(dict) = doc.get_dictionary(current_id) {
        if let Ok(resources_obj) = dict.get(b"Resources") {
            if let Ok(resources_dict) = resources_obj.as_dict().or_else(|_| {
                resources_obj
                    .as_reference()
                    .and_then(|id| doc.get_dictionary(id))
            }) {
                if let Ok(xobject_obj) = resources_dict.get(b"XObject") {
                    if let Ok(xobject_dict) = xobject_obj.as_dict().or_else(|_| {
                        xobject_obj
                            .as_reference()
                            .and_then(|id| doc.get_dictionary(id))
                    }) {
                        for (_, obj) in xobject_dict.iter() {
                            if let Ok(id) = obj.as_reference() {
                                result.push(id);
                            }
                        }
                    }
                }
            }
        }

        if let Ok(parent_id) = dict.get(b"Parent").and_then(|o| o.as_reference()) {
            if !visited.insert(parent_id) {
                break;
            }
            current_id = parent_id;
        } else {
            break;
        }
    }

    result
}
fn cache_image_asset(xobj_stream: &lopdf::Stream, width: i64, height: i64) -> Option<String> {
    let mut filter_name: &[u8] = b"";
    if let Ok(filter_obj) = xobj_stream.dict.get(b"Filter") {
        if let Ok(name) = filter_obj.as_name() {
            filter_name = name;
        } else if let Ok(arr) = filter_obj.as_array() {
            if let Some(first) = arr.first().and_then(|o: &Object| o.as_name().ok()) {
                filter_name = first;
            }
        }
    }

    if filter_name != b"DCTDecode" {
        log_step!(
            "[PDF][preview] skip non-jpeg preview asset {}x{} filter={}",
            width,
            height,
            String::from_utf8_lossy(filter_name)
        );
        return None;
    }

    let img_data = Arc::<[u8]>::from(xobj_stream.content.clone());
    if img_data.is_empty() {
        return None;
    }

    let asset_id = ::uuid::Uuid::new_v4().to_string();
    let mut cache = PDF_IMAGE_CACHE.lock().unwrap();
    let byte_len = img_data.len();
    cache.insert(asset_id.clone(), img_data);
    log_step!(
        "[PDF][preview] cached original jpeg {}x{} bytes={}",
        width,
        height,
        byte_len
    );
    Some(format!("http://pdfasset.localhost/{}", asset_id))
}
pub fn build_light_page_model(doc: &Document, page_index: u16) -> Result<LightPageModel, String> {
    let total_start = std::time::Instant::now();
    let page_id = *doc
        .get_pages()
        .get(&(page_index as u32 + 1))
        .ok_or_else(|| format!("Page not found: {}", page_index))?;

    let (width, height) = page_dimensions(doc, page_id)?;
    let xobjects = collect_page_xobjects(doc, page_id);
    log_step!(
        "[PDF][preview] page={} size={}x{} xobjects={}",
        page_index,
        width,
        height,
        xobjects.len()
    );
    let mut best_preview: Option<(i64, String)> = None;

    for xobj_id in xobjects {
        let stream = match doc.get_object(xobj_id).and_then(|o| o.as_stream()) {
            Ok(stream) => stream,
            Err(_) => continue,
        };

        let subtype = stream
            .dict
            .get(b"Subtype")
            .ok()
            .and_then(|o| o.as_name().ok())
            .unwrap_or(b"");
        if subtype != b"Image" {
            continue;
        }

        let img_width = stream
            .dict
            .get(b"Width")
            .and_then(|o| o.as_i64())
            .unwrap_or(0);
        let img_height = stream
            .dict
            .get(b"Height")
            .and_then(|o| o.as_i64())
            .unwrap_or(0);
        if img_width <= 0 || img_height <= 0 {
            continue;
        }

        let area = img_width.saturating_mul(img_height);
        log_step!(
            "[PDF][preview] candidate image {}x{} area={}",
            img_width,
            img_height,
            area
        );
        if let Some(url) = cache_image_asset(stream, img_width, img_height) {
            match &best_preview {
                Some((best_area, _)) if *best_area >= area => {}
                _ => best_preview = Some((area, url)),
            }
        }
    }

    let preview_image_url = best_preview.map(|(_, url)| url);
    let kind = if preview_image_url.is_some() {
        LightPageKind::Scanned
    } else {
        LightPageKind::Text
    };

    Ok(LightPageModel {
        page_index,
        width,
        height,
        kind,
        preview_image_url,
    })
    .inspect(|_| {
        log_step!(
            "[PDF][preview] build_light_page_model TOTAL {:?}",
            total_start.elapsed()
        );
    })
}
