//! GeometryApi — unified WASM handle for coordinate-space transforms
//! (Nutrient borrowing #4).
//!
//! Nutrient exposes 6 explicit transform methods on `Instance`:
//!
//!   transformClientToPageSpace / transformPageToClientSpace
//!   transformContentClientToPageSpace / transformContentPageToClientSpace
//!   transformPageToRawSpace / transformRawToPageSpace
//!
//! This project's equivalent logic was scattered across
//! `projection_workflow.rs` (internal helpers, no WASM surface) and
//! hand-rolled math in the TS bridge. `GeometryApi` consolidates them
//! into a single discoverable handle with the same three coordinate
//! spaces:
//!
//!   **Client** — browser DOM pixels (`getBoundingClientRect` coords)
//!   **Page**   — PDF page CSS pixels, Y-Down, zoom-applied
//!   **Raw**    — PDF original units (72 DPI, Y-Up)

use pdf_viewer_core::geometry::coordinate_transform::{
    ClientPoint, HostPageTransform, HostReferenceRect, PageSize, PdfCoordinateSpace,
};
use pdf_viewer_core::geometry::dom_projection::{DomPointLike, DomRectLike};
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;

// ── JS-friendly I/O types ───────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PointResult {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RectResult {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TransformContext {
    /// DOM reference rect from `element.getBoundingClientRect()`.
    pub reference: DomRectLike,
    /// Page dimensions in PDF points (unzoomed).
    pub page_width: f32,
    pub page_height: f32,
}

// ── GeometryApi ─────────────────────────────────────────────────

#[wasm_bindgen]
pub struct GeometryApi;

#[wasm_bindgen]
impl GeometryApi {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        GeometryApi
    }

    // ── Client ↔ Page ───────────────────────────────────────────

    /// Transform a DOM client point to page-space (Y-Down, unzoomed PDF points).
    ///
    /// `ctx` must contain the page container's `getBoundingClientRect()` and
    /// the page dimensions in PDF points.
    #[wasm_bindgen(js_name = "clientToPage")]
    pub fn client_to_page(&self, point_js: JsValue, ctx_js: JsValue) -> JsValue {
        let point: DomPointLike = unwrap_or_null!(from_value(point_js));
        let ctx: TransformContext = unwrap_or_null!(from_value(ctx_js));
        let transform = build_transform(&ctx);
        let page = transform.client_to_page(ClientPoint {
            x: point.client_x,
            y: point.client_y,
        });
        to_value(&PointResult {
            x: page.x,
            y: page.y,
        })
        .unwrap_or(JsValue::NULL)
    }

    /// Transform a page-space point back to DOM client coordinates.
    ///
    /// Inverse of `clientToPage`.
    #[wasm_bindgen(js_name = "pageToClient")]
    pub fn page_to_client(&self, point_js: JsValue, ctx_js: JsValue) -> JsValue {
        let point: PointResult = unwrap_or_null!(from_value(point_js));
        let ctx: TransformContext = unwrap_or_null!(from_value(ctx_js));
        let transform = build_transform(&ctx);
        let scale = transform.scale();
        let client = PointResult {
            x: ctx.reference.left + point.x * scale.x,
            y: ctx.reference.top + point.y * scale.y,
        };
        to_value(&client).unwrap_or(JsValue::NULL)
    }

    // ── Page ↔ Raw (PDF Y-Up 72 DPI) ───────────────────────────

    /// Transform a page-space point (Y-Down) to raw PDF space (Y-Up, 72 DPI).
    #[wasm_bindgen(js_name = "pageToRaw")]
    pub fn page_to_raw(&self, point_js: JsValue, page_height: f32) -> JsValue {
        let point: PointResult = unwrap_or_null!(from_value(point_js));
        let raw = PointResult {
            x: point.x,
            y: PdfCoordinateSpace::denormalize_y(point.y, page_height),
        };
        to_value(&raw).unwrap_or(JsValue::NULL)
    }

    /// Transform a raw PDF point (Y-Up, 72 DPI) to page-space (Y-Down).
    #[wasm_bindgen(js_name = "rawToPage")]
    pub fn raw_to_page(&self, point_js: JsValue, page_height: f32) -> JsValue {
        let point: PointResult = unwrap_or_null!(from_value(point_js));
        let page = PointResult {
            x: point.x,
            y: PdfCoordinateSpace::normalize_y(point.y, page_height),
        };
        to_value(&page).unwrap_or(JsValue::NULL)
    }

    // ── Client → Raw (convenience shortcut) ─────────────────────

    /// Transform a DOM client point directly to raw PDF space.
    ///
    /// Equivalent to `clientToPage` → `pageToRaw`.
    #[wasm_bindgen(js_name = "clientToRaw")]
    pub fn client_to_raw(&self, point_js: JsValue, ctx_js: JsValue) -> JsValue {
        let point: DomPointLike = unwrap_or_null!(from_value(point_js));
        let ctx: TransformContext = unwrap_or_null!(from_value(ctx_js));
        let transform = build_transform(&ctx);
        let page = transform.client_to_page(ClientPoint {
            x: point.client_x,
            y: point.client_y,
        });
        let raw = PointResult {
            x: page.x,
            y: PdfCoordinateSpace::denormalize_y(page.y, ctx.page_height),
        };
        to_value(&raw).unwrap_or(JsValue::NULL)
    }

    // ── Scale / zoom helpers ────────────────────────────────────

    /// Measure the DOM-to-page scale factors for the given reference rect.
    ///
    /// Returns `{ scaleX, scaleY }`.
    #[wasm_bindgen(js_name = "measureScale")]
    pub fn measure_scale(&self, ctx_js: JsValue) -> JsValue {
        let ctx: TransformContext = unwrap_or_null!(from_value(ctx_js));
        let scale = pdf_viewer_core::geometry::dom_projection::measure_dom_to_page_scale(
            &ctx.reference,
            ctx.page_width,
            ctx.page_height,
        );
        to_value(&scale).unwrap_or(JsValue::NULL)
    }

    /// Apply zoom to a page-space rect: multiply all edges by `zoom`.
    ///
    /// Returns `{ left, top, right, bottom }` in zoomed CSS pixels.
    #[wasm_bindgen(js_name = "projectRect")]
    pub fn project_rect(&self, rect_js: JsValue, zoom: f32) -> JsValue {
        let rect: RectResult = unwrap_or_null!(from_value(rect_js));
        let z = if zoom.is_finite() && zoom > 0.0 {
            zoom
        } else {
            1.0
        };
        let projected = RectResult {
            left: rect.left * z,
            top: rect.top * z,
            right: rect.right * z,
            bottom: rect.bottom * z,
        };
        to_value(&projected).unwrap_or(JsValue::NULL)
    }
}

impl Default for GeometryApi {
    fn default() -> Self {
        Self::new()
    }
}

// ── Internal helpers ────────────────────────────────────────────

fn build_transform(ctx: &TransformContext) -> HostPageTransform {
    HostPageTransform::new(
        HostReferenceRect {
            left: ctx.reference.left,
            top: ctx.reference.top,
            width: ctx.reference.width,
            height: ctx.reference.height,
        },
        PageSize {
            width: ctx.page_width,
            height: ctx.page_height,
        },
    )
}

/// Convenience macro: return JsValue::NULL on deserialization failure.
macro_rules! unwrap_or_null {
    ($expr:expr) => {
        match $expr {
            Ok(v) => v,
            Err(_) => return JsValue::NULL,
        }
    };
}
use unwrap_or_null;
