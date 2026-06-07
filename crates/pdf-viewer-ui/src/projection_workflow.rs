use pdf_viewer_core::document::page_region_context::build_page_region_context as core_build_page_region_context;
use pdf_viewer_core::geometry::coordinate_transform::{
    ClientPoint, HostPageTransform, HostReferenceRect, PageSize,
};
use pdf_viewer_core::geometry::layout_engine::resolve_editor_projection as core_resolve_editor_projection;
use pdf_viewer_core::models::{BoundingBox, FontHints, NativePageModel, NativeTextModel, RectBox};
use pdf_viewer_core::text::editable_segments::build_editable_segments as core_build_editable_segments;
use pdf_viewer_core::typography::font_resolver::resolve_font_face as core_resolve_font_face;
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;

use crate::dom_projection::{DomPointLike, DomRectLike};

pub fn resolve_font_face(font_name: String, hints: JsValue) -> JsValue {
    let parsed_hints: Option<FontHints> = if hints.is_undefined() || hints.is_null() {
        None
    } else {
        from_value(hints).ok()
    };
    to_value(&core_resolve_font_face(&font_name, parsed_hints.as_ref())).unwrap_or(JsValue::NULL)
}

pub fn build_editable_segments(text_model: JsValue, page_height: f32) -> JsValue {
    let parsed: NativeTextModel = from_value(text_model).unwrap_or_default();
    to_value(&core_build_editable_segments(&parsed, page_height)).unwrap_or(JsValue::NULL)
}

pub fn resolve_editor_projection(
    box_rect_js: JsValue,
    zoom: f32,
    font_size: f32,
    page_height: f32,
) -> JsValue {
    let box_rect: RectBox = from_value(box_rect_js).unwrap_or_default();
    to_value(&core_resolve_editor_projection(
        &box_rect,
        zoom,
        font_size,
        page_height,
    ))
    .unwrap_or(JsValue::NULL)
}

pub fn build_pagination_commands(
    current_page: usize,
    total_pages: usize,
    path: String,
    zoom: f32,
) -> JsValue {
    to_value(
        &pdf_viewer_core::persistence::patch_store::build_pagination_commands(
            current_page,
            total_pages,
            &path,
            zoom,
        ),
    )
    .unwrap_or(JsValue::NULL)
}

pub fn build_page_region_context(page_model: JsValue) -> JsValue {
    let parsed: NativePageModel = from_value(page_model).unwrap_or_default();
    to_value(&core_build_page_region_context(&parsed)).unwrap_or(JsValue::NULL)
}

pub fn project_page_rect(rect: JsValue, zoom: f32) -> JsValue {
    let bbox: BoundingBox = from_value(rect).unwrap_or_default();
    let effective_zoom = if zoom.is_finite() && zoom > 0.0 {
        zoom
    } else {
        1.0
    };
    let projected = BoundingBox {
        left: bbox.left * effective_zoom,
        top: bbox.top * effective_zoom,
        right: bbox.right * effective_zoom,
        bottom: bbox.bottom * effective_zoom,
    };
    to_value(&projected).unwrap_or(JsValue::NULL)
}

pub fn measure_dom_to_page_scale(
    reference_rect: JsValue,
    page_width: f32,
    page_height: f32,
) -> JsValue {
    let rect: DomRectLike = from_value(reference_rect).unwrap_or_default();
    let result = crate::dom_projection::measure_dom_to_page_scale(&rect, page_width, page_height);
    to_value(&result).unwrap_or(JsValue::NULL)
}

pub fn resolve_page_point(
    point: JsValue,
    reference_rect: JsValue,
    page_width: f32,
    page_height: f32,
) -> JsValue {
    let point: DomPointLike = from_value(point).unwrap_or_default();
    let rect: DomRectLike = from_value(reference_rect).unwrap_or_default();
    let transform = HostPageTransform::new(
        HostReferenceRect {
            left: rect.left,
            top: rect.top,
            width: rect.width,
            height: rect.height,
        },
        PageSize {
            width: page_width,
            height: page_height,
        },
    );
    let page_point = transform.client_to_page(ClientPoint {
        x: point.client_x,
        y: point.client_y,
    });
    let result = serde_json::json!({
        "x": page_point.x,
        "y": page_point.y,
    });
    to_value(&result).unwrap_or(JsValue::NULL)
}
