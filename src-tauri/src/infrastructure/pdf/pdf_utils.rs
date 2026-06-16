//! Shared PDF parsing utilities — eliminates duplication of lopdf Object
//! number extraction, MediaBox page-size parsing, and inherited /Rotate
//! resolution across pdf_read, pdf_write, preview_engine, annotation_store.

use lopdf::{Document, Object, ObjectId};

/// Default page size (A4 in PDF points) used when MediaBox is missing or malformed.
pub const DEFAULT_PAGE_WIDTH: f32 = 595.0;
pub const DEFAULT_PAGE_HEIGHT: f32 = 842.0;

/// Convert a lopdf `Object` to `f32`, accepting both `Real` and `Integer` operands.
/// Returns the lopdf error if neither variant matches.
pub fn obj_to_f32(obj: &Object) -> Result<f32, lopdf::Error> {
    obj.as_float().or_else(|_| obj.as_i64().map(|n| n as f32))
}

/// Like `obj_to_f32` but returns `default` on failure instead of an error.
/// Use this when a missing/malformed value should silently fall back.
pub fn obj_to_f32_or(obj: &Object, default: f32) -> f32 {
    obj_to_f32(obj).unwrap_or(default)
}

/// Resolved page geometry from a MediaBox array.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PageSize {
    pub width: f32,
    pub height: f32,
    /// The y-origin (MediaBox[1]); height is measured from this baseline.
    pub y_origin: f32,
}

impl PageSize {
    /// Returns the effective drawing height (height - y_origin).
    pub fn effective_height(&self) -> f32 {
        (self.height - self.y_origin).abs()
    }
}

/// Read the MediaBox from a page dictionary and return the resolved `PageSize`.
/// Falls back to A4 defaults (595 x 842) if the MediaBox is absent or malformed.
pub fn read_page_size(doc: &Document, page_id: ObjectId) -> PageSize {
    let Ok(page_dict) = doc.get_dictionary(page_id) else {
        return PageSize::default_a4();
    };
    let Ok(box_obj) = page_dict.get(b"MediaBox") else {
        return PageSize::default_a4();
    };
    let Ok(arr) = box_obj.as_array() else {
        return PageSize::default_a4();
    };
    if arr.len() < 4 {
        return PageSize::default_a4();
    }
    // MediaBox = [x0 y0 x1 y1]; width = |x1 - x0|, height = y1.
    // Most PDFs use [0 0 w h], so width=arr[2], height=arr[3], y_origin=arr[1].
    let y0 = obj_to_f32_or(&arr[1], 0.0);
    let x1 = obj_to_f32_or(&arr[2], DEFAULT_PAGE_WIDTH);
    let y1 = obj_to_f32_or(&arr[3], DEFAULT_PAGE_HEIGHT);
    PageSize {
        width: x1.abs(),
        height: y1,
        y_origin: y0,
    }
}

impl PageSize {
    pub fn default_a4() -> Self {
        PageSize {
            width: DEFAULT_PAGE_WIDTH,
            height: DEFAULT_PAGE_HEIGHT,
            y_origin: 0.0,
        }
    }
}

/// Read the inherited /Rotate attribute by walking up the page tree to /Parent.
/// Returns the rotation normalized to 0, 90, 180, or 270.
pub fn read_page_rotation(doc: &Document, page_id: ObjectId) -> i64 {
    let mut rotation = 0i64;
    let mut current_id = page_id;
    while let Ok(dict) = doc.get_dictionary(current_id) {
        if let Ok(rotate_obj) = dict.get(b"Rotate") {
            if let Ok(r) = rotate_obj.as_i64() {
                rotation = r;
                break;
            }
        }
        match dict.get(b"Parent").and_then(|o| o.as_reference()) {
            Ok(parent_id) => current_id = parent_id,
            Err(_) => break,
        }
    }
    ((rotation % 360) + 360) % 360
}

/// Apply page rotation to a (width, height) pair, swapping dimensions for 90/270.
pub fn apply_rotation(width: f32, height: f32, rotation: i64) -> (f32, f32) {
    if rotation == 90 || rotation == 270 {
        (height, width)
    } else {
        (width, height)
    }
}
