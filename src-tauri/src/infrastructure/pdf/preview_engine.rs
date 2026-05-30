use lopdf::{Document, ObjectId};
use std::sync::Arc;
use crate::infrastructure::pdf::models::{LightPageKind, LightPageModel};
use crate::infrastructure::pdf::cache::PDF_IMAGE_CACHE;
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

    let mut w = (x1 - x0).abs();
    let mut h = (y1 - y0).abs();

    // Support inherited /Rotate attribute in page dictionary tree
    let mut rotation = 0i64;
    let mut current_id = page_id;
    while let Ok(dict) = doc.get_dictionary(current_id) {
        if let Ok(rotate_obj) = dict.get(b"Rotate") {
            if let Ok(r) = rotate_obj.as_i64() {
                rotation = r;
                break;
            }
        }
        if let Ok(parent_id) = dict.get(b"Parent").and_then(|o| o.as_reference()) {
            current_id = parent_id;
        } else {
            break;
        }
    }

    // Normalize rotation to 0, 90, 180, 270
    let normalized_rotation = ((rotation % 360) + 360) % 360;
    if normalized_rotation == 90 || normalized_rotation == 270 {
        std::mem::swap(&mut w, &mut h);
        log_step!(
            "[PDF][preview] swapped page size due to {} deg rotation. final={}x{}",
            normalized_rotation, w, h
        );
    }

    Ok((w, h))
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
    // Determine the effective image filter, handling both single name and array forms.
    // Common patterns:
    //   Filter: /DCTDecode               → raw JPEG in stream.content
    //   Filter: [/FlateDecode /DCTDecode] → stream.content is Flate-compressed JPEG
    //   Filter: [/DCTDecode]             → raw JPEG
    let filter_obj = xobj_stream.dict.get(b"Filter").ok()?;

    let filters: Vec<&[u8]> = if let Ok(name) = filter_obj.as_name() {
        vec![name]
    } else if let Ok(arr) = filter_obj.as_array() {
        arr.iter().filter_map(|o| o.as_name().ok()).collect()
    } else {
        return None;
    };

    // Must contain DCTDecode somewhere to be a JPEG
    let has_dct = filters.iter().any(|f| *f == b"DCTDecode");
    if !has_dct {
        log_step!(
            "[PDF][preview] skip non-jpeg preview asset {}x{} filters={:?}",
            width,
            height,
            filters.iter().map(|f| String::from_utf8_lossy(f).to_string()).collect::<Vec<_>>()
        );
        return None;
    }

    // Get the raw stream content
    let raw_content = &xobj_stream.content;
    if raw_content.is_empty() {
        return None;
    }

    // If FlateDecode precedes DCTDecode, we need to decompress first
    let has_flate_before_dct = filters.len() > 1
        && filters.iter().position(|f| *f == b"FlateDecode")
            < filters.iter().position(|f| *f == b"DCTDecode");

    let img_data: Arc<[u8]> = if has_flate_before_dct {
        // Decompress Flate to get the raw JPEG bytes
        use flate2::read::ZlibDecoder;
        use std::io::Read;
        let mut decoder = ZlibDecoder::new(raw_content.as_slice());
        let mut decompressed = Vec::new();
        match decoder.read_to_end(&mut decompressed) {
            Ok(_) => Arc::from(decompressed),
            Err(_) => {
                // Try raw deflate without zlib header
                use flate2::read::DeflateDecoder;
                let mut decoder2 = DeflateDecoder::new(raw_content.as_slice());
                let mut decompressed2 = Vec::new();
                match decoder2.read_to_end(&mut decompressed2) {
                    Ok(_) => Arc::from(decompressed2),
                    Err(e) => {
                        log_step!(
                            "[PDF][preview] flate decompress failed for {}x{}: {:?}",
                            width, height, e
                        );
                        return None;
                    }
                }
            }
        }
    } else {
        Arc::from(raw_content.clone())
    };

    if img_data.is_empty() {
        return None;
    }

    let asset_id = ::uuid::Uuid::new_v4().to_string();
    let mut cache = PDF_IMAGE_CACHE.lock().unwrap();
    let byte_len = img_data.len();
    cache.insert(asset_id.clone(), img_data);
    log_step!(
        "[PDF][preview] cached original jpeg {}x{} bytes={} flate_decompressed={}",
        width,
        height,
        byte_len,
        has_flate_before_dct
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
