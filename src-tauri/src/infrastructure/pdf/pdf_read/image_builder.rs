use std::sync::Arc;
/// Apply PNG-style predictor (PDF Predictor values 10-15) to unfilter the data.
/// Each row begins with a filter type byte (0=None, 1=Sub, 2=Up, 3=Average, 4=Paeth).
pub(crate) fn apply_png_predictor(raw: &[u8], bytes_per_row: usize, bpp: usize) -> Option<Vec<u8>> {
    let row_with_filter = bytes_per_row + 1;
    if raw.is_empty() || raw.len() % row_with_filter != 0 {
        // Try to be lenient - PDFs sometimes have extra/missing bytes
        if raw.len() < row_with_filter {
            return None;
        }
    }
    let bpp = bpp.max(1);
    let rows = raw.len() / row_with_filter;
    let mut out = vec![0u8; rows * bytes_per_row];
    let mut prev_row = vec![0u8; bytes_per_row];
    for r in 0..rows {
        let row_start = r * row_with_filter;
        if row_start + row_with_filter > raw.len() {
            break;
        }
        let filter = raw[row_start];
        let row_data = &raw[row_start + 1..row_start + row_with_filter];
        let out_row_start = r * bytes_per_row;
        let cur_row = &mut out[out_row_start..out_row_start + bytes_per_row];
        for c in 0..bytes_per_row {
            let left = if c < bpp { 0 } else { cur_row[c - bpp] };
            let up = prev_row[c];
            let up_left = if c < bpp { 0 } else { prev_row[c - bpp] };
            let val = match filter {
                0 => row_data[c],                                                     // None
                1 => row_data[c].wrapping_add(left),                                  // Sub
                2 => row_data[c].wrapping_add(up),                                    // Up
                3 => row_data[c].wrapping_add(((left as u16 + up as u16) / 2) as u8), // Average
                4 => {
                    // Paeth
                    let a = left as i32;
                    let b = up as i32;
                    let c2 = up_left as i32;
                    let p = a + b - c2;
                    let pa = (p - a).abs();
                    let pb = (p - b).abs();
                    let pc = (p - c2).abs();
                    let predictor = if pa <= pb && pa <= pc {
                        a
                    } else if pb <= pc {
                        b
                    } else {
                        c2
                    };
                    row_data[c].wrapping_add(predictor as u8)
                }
                _ => row_data[c],
            };
            cur_row[c] = val;
        }
        prev_row.copy_from_slice(cur_row);
    }
    Some(out)
}

/// Read DecodeParms dictionary and extract Predictor / Columns / Colors / BitsPerComponent.
/// Accepts `doc` to resolve indirect references in the DecodeParms entry.
pub(crate) fn read_decode_params(doc: &lopdf::Document, stream: &lopdf::Stream) -> (i64, i64, i64, i64) {
    let mut predictor = 1i64;
    let mut columns = 1i64;
    let mut colors = 1i64;
    let mut bpc = 8i64;
    if let Ok(obj) = stream.dict.get(b"DecodeParms") {
        // Resolve if it's an indirect reference
        let resolved = match obj {
            lopdf::Object::Reference(r) => doc.get_object(*r).ok(),
            other => Some(other),
        };
        let dict_opt = resolved.and_then(|o| {
            o.as_dict().ok().cloned().or_else(|| {
                o.as_array().ok()?.first().and_then(|item| match item {
                    lopdf::Object::Reference(r) => doc.get_object(*r).ok()?.as_dict().ok().cloned(),
                    other => other.as_dict().ok().cloned(),
                })
            })
        });
        if let Some(dict) = dict_opt {
            if let Ok(v) = dict.get(b"Predictor").and_then(|o| o.as_i64()) {
                predictor = v;
            }
            if let Ok(v) = dict.get(b"Columns").and_then(|o| o.as_i64()) {
                columns = v;
            }
            if let Ok(v) = dict.get(b"Colors").and_then(|o| o.as_i64()) {
                colors = v;
            }
            if let Ok(v) = dict.get(b"BitsPerComponent").and_then(|o| o.as_i64()) {
                bpc = v;
            }
        }
    }
    (predictor, columns, colors, bpc)
}

/// Manually decompress FlateDecode data using flate2 as fallback when lopdf fails.
pub(crate) fn manual_flate_decompress(compressed: &[u8]) -> Option<Vec<u8>> {
    use std::io::Read;
    // Try zlib (with header) first, then raw deflate
    if let Ok(decoder) = flate2::read::ZlibDecoder::new(compressed)
        .bytes()
        .collect::<Result<Vec<u8>, _>>()
    {
        if !decoder.is_empty() {
            return Some(decoder);
        }
    }
    // Fallback: raw deflate
    let mut decoder = flate2::read::DeflateDecoder::new(compressed);
    let mut out = Vec::new();
    if decoder.read_to_end(&mut out).is_ok() && !out.is_empty() {
        return Some(out);
    }
    None
}

/// Extract a non-JPEG image XObject's raw samples, convert to JPEG bytes.
/// Handles DeviceRGB, DeviceGray, CMYK, and FlateDecode with PNG/TIFF predictors.
pub(crate) fn build_image_as_jpeg(
    doc: &lopdf::Document,
    stream: &lopdf::Stream,
    w: u32,
    h: u32,
) -> Option<Arc<[u8]>> {
    // Try lopdf's built-in decompression first; fall back to manual flate2
    let raw_decompressed = match stream.decompressed_content() {
        Ok(d) => {
            crate::log_step!("[PDF-IMG] lopdf decompress OK len={}", d.len());
            d
        }
        Err(e) => {
            crate::log_step!(
                "[PDF-IMG] lopdf decompress failed: {} — trying manual flate2 on {} raw bytes",
                e,
                stream.content.len()
            );
            match manual_flate_decompress(&stream.content) {
                Some(d) => {
                    crate::log_step!("[PDF-IMG] manual flate2 OK len={}", d.len());
                    d
                }
                None => {
                    crate::log_step!("[PDF-IMG] manual flate2 also failed");
                    return None;
                }
            }
        }
    };
    if raw_decompressed.is_empty() {
        crate::log_step!("[PDF-IMG] decompressed empty");
        return None;
    }

    let cs_name = stream.dict.get(b"ColorSpace").ok().and_then(|o| {
        o.as_name().ok().map(|n| n.to_vec()).or_else(|| {
            o.as_array()
                .ok()?
                .first()?
                .as_name()
                .ok()
                .map(|n| n.to_vec())
        })
    });
    let bpc = stream
        .dict
        .get(b"BitsPerComponent")
        .and_then(|o| o.as_i64())
        .unwrap_or(8) as u32;
    let cs = cs_name.as_deref().unwrap_or(b"DeviceRGB");

    let (predictor, dp_columns, dp_colors, dp_bpc) = read_decode_params(doc, stream);
    crate::log_step!(
        "[PDF-IMG] decoding {}x{} cs={} bpc={} decompressed={} predictor={} dp_cols={} dp_colors={} dp_bpc={}",
        w, h, String::from_utf8_lossy(cs), bpc, raw_decompressed.len(),
        predictor, dp_columns, dp_colors, dp_bpc
    );

    // Apply predictor if needed
    let raw: Vec<u8> = if predictor >= 10 {
        let columns = if dp_columns > 0 {
            dp_columns as usize
        } else {
            w as usize
        };
        let colors = if dp_colors > 0 {
            dp_colors as usize
        } else {
            match cs {
                b"DeviceRGB" => 3,
                b"DeviceCMYK" => 4,
                _ => 1,
            }
        };
        let pbpc = if dp_bpc > 0 {
            dp_bpc as usize
        } else {
            bpc as usize
        };
        let bytes_per_row = (columns * colors * pbpc + 7) / 8;
        let bpp = ((colors * pbpc) + 7) / 8;
        match apply_png_predictor(&raw_decompressed, bytes_per_row, bpp) {
            Some(unfiltered) => {
                crate::log_step!(
                    "[PDF-IMG] PNG predictor applied: bytes_per_row={} bpp={} unfiltered_len={}",
                    bytes_per_row,
                    bpp,
                    unfiltered.len()
                );
                unfiltered
            }
            None => {
                crate::log_step!("[PDF-IMG] PNG predictor failed, using raw");
                raw_decompressed
            }
        }
    } else {
        raw_decompressed
    };

    let expected_len = match cs {
        b"DeviceRGB" => (w * h * 3 * bpc / 8) as usize,
        b"DeviceGray" => (w * h * bpc / 8) as usize,
        b"DeviceCMYK" => (w * h * 4 * bpc / 8) as usize,
        _ => (w * h * 3 * bpc / 8) as usize,
    };

    if raw.len() < expected_len {
        crate::log_step!(
            "[PDF-IMG] raw data too short after predictor: have={} expected={} cs={} bpc={} {}x{}",
            raw.len(),
            expected_len,
            String::from_utf8_lossy(cs),
            bpc,
            w,
            h
        );
        return None;
    }

    // Build RGBA buffer
    let pixel_count = (w * h) as usize;
    let mut rgba = vec![255u8; pixel_count * 4];
    match cs {
        b"DeviceRGB" if bpc == 8 => {
            for i in 0..pixel_count {
                rgba[i * 4] = raw[i * 3];
                rgba[i * 4 + 1] = raw[i * 3 + 1];
                rgba[i * 4 + 2] = raw[i * 3 + 2];
            }
        }
        b"DeviceGray" if bpc == 8 => {
            for i in 0..pixel_count {
                let g = raw[i];
                rgba[i * 4] = g;
                rgba[i * 4 + 1] = g;
                rgba[i * 4 + 2] = g;
            }
        }
        b"DeviceCMYK" if bpc == 8 => {
            for i in 0..pixel_count {
                let c = raw[i * 4] as f32 / 255.0;
                let m = raw[i * 4 + 1] as f32 / 255.0;
                let y = raw[i * 4 + 2] as f32 / 255.0;
                let k = raw[i * 4 + 3] as f32 / 255.0;
                rgba[i * 4] = (255.0 * (1.0 - c) * (1.0 - k)) as u8;
                rgba[i * 4 + 1] = (255.0 * (1.0 - m) * (1.0 - k)) as u8;
                rgba[i * 4 + 2] = (255.0 * (1.0 - y) * (1.0 - k)) as u8;
            }
        }
        b"DeviceGray" if bpc == 1 => {
            // 1-bit monochrome (common in B&W scans)
            for i in 0..pixel_count {
                let byte_idx = i / 8;
                let bit_idx = 7 - (i % 8);
                let g = if byte_idx < raw.len() && (raw[byte_idx] >> bit_idx) & 1 == 1 {
                    255u8
                } else {
                    0u8
                };
                rgba[i * 4] = g;
                rgba[i * 4 + 1] = g;
                rgba[i * 4 + 2] = g;
            }
        }
        _ => {
            // Best-effort: treat as RGB if enough data, else grayscale
            if raw.len() >= pixel_count * 3 {
                for i in 0..pixel_count {
                    rgba[i * 4] = raw[i * 3];
                    rgba[i * 4 + 1] = raw[i * 3 + 1];
                    rgba[i * 4 + 2] = raw[i * 3 + 2];
                }
            } else {
                for i in 0..pixel_count.min(raw.len()) {
                    rgba[i * 4] = raw[i];
                    rgba[i * 4 + 1] = raw[i];
                    rgba[i * 4 + 2] = raw[i];
                }
            }
        }
    }

    // Convert RGBA8 to RGB8 for JPEG encoding (JPEG has no Alpha channel)
    let mut rgb = Vec::with_capacity(pixel_count * 3);
    for i in 0..pixel_count {
        rgb.push(rgba[i * 4]);
        rgb.push(rgba[i * 4 + 1]);
        rgb.push(rgba[i * 4 + 2]);
    }

    // Encode to JPEG
    let mut jpeg_buf = Vec::new();
    {
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg_buf, 80);
        use image::ImageEncoder;
        if encoder
            .write_image(&rgb, w, h, image::ColorType::Rgb8)
            .is_err()
        {
            return None;
        }
    }
    Some(Arc::from(jpeg_buf.as_slice()))
}

