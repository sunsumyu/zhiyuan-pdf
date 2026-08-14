use crate::infrastructure::pdf::cache::PDF_IMAGE_CACHE;
use crate::infrastructure::pdf::models::{LightPageKind, LightPageModel};
use lopdf::{Document, ObjectId};
use crate::infrastructure::pdf::pdf_utils;
use std::sync::Arc;
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

    // Fallback: If no XObjects were found in this page's hierarchy, but the content stream
    // references XObjects via "Do", look up those referenced names across all page resource dictionaries.
    if result.is_empty() {
        if let Ok(content_data) = doc.get_page_content(page_id) {
            if let Ok(content) = lopdf::content::Content::decode(&content_data) {
                for op in content.operations.iter() {
                    if op.operator.as_str() == "Do" {
                        if let Some(name) = op.operands.get(0).and_then(|o| o.as_name().ok()) {
                            // Find the name in any page's resource dictionary
                            for (_, other_page_id) in doc.get_pages() {
                                if let Ok(other_dict) = doc.get_dictionary(other_page_id) {
                                    if let Ok(res_obj) = other_dict.get(b"Resources") {
                                        if let Ok(res_dict) = res_obj.as_dict().or_else(|_| {
                                            res_obj.as_reference().and_then(|id| doc.get_dictionary(id))
                                        }) {
                                            if let Ok(xobject_obj) = res_dict.get(b"XObject") {
                                                if let Ok(xobject_dict) = xobject_obj.as_dict().or_else(|_| {
                                                    xobject_obj.as_reference().and_then(|id| doc.get_dictionary(id))
                                                }) {
                                                    if let Ok(val) = xobject_dict.get(name) {
                                                        if let Ok(id) = val.as_reference() {
                                                            if !result.contains(&id) {
                                                                result.push(id);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    result
}

fn page_has_font_resources(doc: &Document, page_id: ObjectId) -> bool {
    let mut current_id = page_id;
    let mut visited = std::collections::HashSet::new();

    while let Ok(dict) = doc.get_dictionary(current_id) {
        if let Ok(resources_obj) = dict.get(b"Resources") {
            if let Ok(resources_dict) = resources_obj.as_dict().or_else(|_| {
                resources_obj
                    .as_reference()
                    .and_then(|id| doc.get_dictionary(id))
            }) {
                if resources_dict.get(b"Font").is_ok() {
                    return true;
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

    false
}

fn page_has_text_operators(doc: &Document, page_id: ObjectId) -> bool {
    let Ok(content_data) = doc.get_page_content(page_id) else {
        return false;
    };
    let Ok(content) = lopdf::content::Content::decode(&content_data) else {
        return false;
    };
    content.operations.iter().any(|op| {
        matches!(
            op.operator.as_str(),
            "Tj" | "TJ" | "'" | "\"" | "BT" | "Tf" | "Td" | "TD" | "Tm"
        )
    })
}

fn cache_image_asset(doc: &Document, xobj_stream: &lopdf::Stream, width: i64, height: i64) -> Option<String> {
    // Determine the effective image filter, handling both single name and array forms.
    // Common patterns:
    //   Filter: /DCTDecode               → raw JPEG in stream.content
    //   Filter: [/FlateDecode /DCTDecode] → stream.content is Flate-compressed JPEG
    //   Filter: [/DCTDecode]             → raw JPEG
    let filter_obj = xobj_stream.dict.get(b"Filter").ok();

    let filters: Vec<&[u8]> = if let Some(obj) = filter_obj {
        if let Ok(name) = obj.as_name() {
            vec![name]
        } else if let Ok(arr) = obj.as_array() {
            arr.iter().filter_map(|o| o.as_name().ok()).collect()
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    // Must contain DCTDecode somewhere to be a JPEG
    let has_dct = filters.iter().any(|f| *f == b"DCTDecode");

    let img_data: Arc<[u8]> = if has_dct {
        // Get the raw stream content
        let raw_content = &xobj_stream.content;
        if raw_content.is_empty() {
            return None;
        }

        // If FlateDecode precedes DCTDecode, we need to decompress first
        let has_flate_before_dct = filters.len() > 1
            && filters.iter().position(|f| *f == b"FlateDecode")
                < filters.iter().position(|f| *f == b"DCTDecode");

        if has_flate_before_dct {
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
                            crate::log_step!(
                                "[PDF][preview] flate decompress failed for {}x{}: {:?}",
                                width,
                                height,
                                e
                            );
                            return None;
                        }
                    }
                }
            }
        } else {
            Arc::from(raw_content.clone())
        }
    } else {
        // Non-JPEG: use build_image_as_jpeg to decode and encode to JPEG
        crate::log_step!(
            "[PDF][preview] non-jpeg preview asset {}x{} filters={:?}, converting to jpeg via build_image_as_jpeg",
            width,
            height,
            filters
                .iter()
                .map(|f| String::from_utf8_lossy(f).to_string())
                .collect::<Vec<_>>()
        );
        crate::infrastructure::pdf::pdf_read::build_image_as_jpeg(
            doc,
            xobj_stream,
            width as u32,
            height as u32,
        )?
    };

    if img_data.is_empty() {
        return None;
    }

    let asset_id = ::uuid::Uuid::new_v4().to_string();
    let mut cache = PDF_IMAGE_CACHE.lock().unwrap();
    let byte_len = img_data.len();
    cache.insert(asset_id.clone(), img_data);
    crate::log_step!(
        "[PDF][preview] cached original/converted jpeg {}x{} bytes={}",
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

    let page_size = pdf_utils::read_page_size(doc, page_id);
    let rotation = pdf_utils::read_page_rotation(doc, page_id);
    let (width, height) = pdf_utils::apply_rotation(page_size.width, page_size.effective_height(), rotation);
    let has_text_content = page_has_text_operators(doc, page_id);
    let has_font_resources = page_has_font_resources(doc, page_id);
    let has_text = has_text_content || has_font_resources;

    let xobjects = collect_page_xobjects(doc, page_id);
    crate::log_step!(
        "[PDF][preview] page={} size={}x{} xobjects={} has_text={}",
        page_index,
        width,
        height,
        xobjects.len(),
        has_text
    );
    let mut best_preview: Option<(i64, String, i64, i64)> = None;

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
        crate::log_step!(
            "[PDF][preview] candidate image {}x{} area={}",
            img_width,
            img_height,
            area
        );
        if let Some(url) = cache_image_asset(doc, stream, img_width, img_height) {
            match &best_preview {
                Some((best_area, _, _, _)) if *best_area >= area => {}
                _ => best_preview = Some((area, url, img_width, img_height)),
            }
        }
    }

    let mut preview_image_url = None;
    let mut kind = if has_text {
        LightPageKind::Text
    } else {
        LightPageKind::Scanned
    };

    if let Some((_, url, img_width, img_height)) = best_preview {
        if !has_text {
            preview_image_url = Some(url);
            kind = LightPageKind::Scanned;
        } else {
            // For vector-like or OCR'd scanned documents, we only accept it as a preview
            // if the image is large enough to cover the page (background scan image)
            let w_ratio = img_width as f32 / width;
            let h_ratio = img_height as f32 / height;
            let w_ratio_swapped = img_width as f32 / height;
            let h_ratio_swapped = img_height as f32 / width;

            let is_full_page = (w_ratio >= 0.85 && h_ratio >= 0.85)
                || (w_ratio_swapped >= 0.85 && h_ratio_swapped >= 0.85);

            if is_full_page {
                preview_image_url = Some(url);
                kind = LightPageKind::Text;
            }
        }
    }

    Ok(LightPageModel {
        page_index,
        width,
        height,
        kind,
        preview_image_url,
    })
    .inspect(|_| {
        crate::log_step!(
            "[PDF][preview] build_light_page_model TOTAL {:?}",
            total_start.elapsed()
        );
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::Document;

    #[test]
    fn test_diagnose_scanned_pdf() {
        let path = r"C:\Users\AREN\Documents\刘---20250514 - 副本 (3) - 副本.pdf";
        let doc = Document::load(path).unwrap();
        for page_index in 0..4 {
            let page_id = doc.page_iter().nth(page_index).unwrap();
            let xobjects = collect_page_xobjects(&doc, page_id);
            println!("Page {} ID: {:?} xobjects collected: {:?}", page_index, page_id, xobjects);
            if page_index == 0 {
                assert!(!xobjects.is_empty(), "Page 0 must collect at least one XObject image");
                
                // Assert that our preview extraction does NOT resolve a preview image URL for a vector page
                let model = build_light_page_model(&doc, page_index as u16).unwrap();
                assert!(model.preview_image_url.is_none(), "Page 0 is a vector page and must NOT resolve a template graphic as a page preview image");
                println!("Page 0 resolved preview image URL: {:?}", model.preview_image_url);
            }
        }
    }

    #[test]
    fn test_user_scanned_pdf() {
        let path = r"H:\迅雷下载\book\SSM企业级框架实战.pdf";
        if !std::path::Path::new(path).exists() {
            println!("User PDF path does not exist on this machine, skipping");
            let fallback_path = r"H:\迅雷下载\book\Java EE框架整合开发入门到实战：Spring+Spring MVC+MyBatis（微课版）.pdf";
            if !std::path::Path::new(fallback_path).exists() {
                return;
            }
        }
        let doc = Document::load(path).unwrap_or_else(|e| {
            println!("Failed to load SSM PDF directly via lopdf: {}", e);
            let raw_bytes = std::fs::read(path).expect("Failed to read SSM PDF bytes");
            Document::load_mem(&raw_bytes).expect("Failed to load SSM PDF from memory")
        });
        println!("User PDF page count: {}", doc.get_pages().len());
        for page_index in 0..5 {
            let page_id = match doc.get_pages().get(&(page_index as u32 + 1)) {
                Some(id) => *id,
                None => break,
            };
            println!("--- Page {} (ID: {:?}) ---", page_index, page_id);
            let has_text_content = page_has_text_operators(&doc, page_id);
            let has_font_resources = page_has_font_resources(&doc, page_id);
            println!("has_text_content: {}, has_font_resources: {}", has_text_content, has_font_resources);
            
            let xobjects = collect_page_xobjects(&doc, page_id);
            println!("xobjects count: {}", xobjects.len());
            for (idx, xid) in xobjects.iter().enumerate() {
                if let Ok(stream) = doc.get_object(*xid).and_then(|o| o.as_stream()) {
                    let subtype = stream.dict.get(b"Subtype").ok().and_then(|o| o.as_name().ok()).unwrap_or(b"");
                    let width = stream.dict.get(b"Width").and_then(|o| o.as_i64()).unwrap_or(0);
                    let height = stream.dict.get(b"Height").and_then(|o| o.as_i64()).unwrap_or(0);
                    let filter = stream.dict.get(b"Filter").ok().map(|o| format!("{:?}", o)).unwrap_or_else(|| "None".to_string());
                    println!("  [{}] ID: {:?} Subtype: {} Size: {}x{} Filter: {}", 
                        idx, xid, String::from_utf8_lossy(subtype), width, height, filter
                    );
                }
            }
            
            let model = build_light_page_model(&doc, page_index as u16).unwrap();
            println!("Resolved model: kind={:?}, width={}, height={}, preview_image_url={:?}", 
                model.kind, model.width, model.height, model.preview_image_url
            );

            let vector_model = crate::infrastructure::pdf::vector_engine::resolve_model(&doc, page_index as u16).unwrap();
            for obj in &vector_model.objects {
                if let crate::infrastructure::pdf::models::RenderObject::Image(img) = obj {
                    println!("  Vector image object: id={}, data_url={}, size={}x{}", img.id, img.data_url, img.width, img.height);
                    assert!(img.data_url.starts_with("http://pdfasset.localhost/"));
                }
            }
        }
        panic!("Show stdout output"); // intentionally panic to see print statements
    }
}
