//! CanvasRenderer lifecycle: construction, size sync, surface transforms.

use super::draw::TextMetricsSnapshot;
use super::CanvasRenderer;
use wasm_bindgen::JsCast;
use web_sys::{ContextAttributes2d, HtmlCanvasElement};

impl CanvasRenderer {
    /// Create an overlay canvas with alpha (for editor overlays over the page).
    pub fn new_overlay(canvas: HtmlCanvasElement) -> Option<Self> {
        let attrs = ContextAttributes2d::new();
        attrs.set_alpha(true);

        let ctx = canvas
            .get_context_with_context_options("2d", &attrs.into())
            .ok()??;

        let ctx = ctx.dyn_into::<web_sys::CanvasRenderingContext2d>().ok()?;

        let dpr = web_sys::window()?.device_pixel_ratio() as f32;

        Some(Self {
            ctx,
            canvas,
            dpr,
            canvas_height: std::cell::Cell::new(0.0),
            is_hijacked: false,
            transparent_surface: true,
        })
    }

    /// Hijack an existing page canvas for direct drawing.
    pub fn new_hijacked(target_id: &str) -> Option<Self> {
        let window = web_sys::window()?;
        let document = window.document()?;
        let canvas = document
            .get_element_by_id(target_id)?
            .dyn_into::<HtmlCanvasElement>()
            .ok()?;

        let ctx = canvas
            .get_context("2d")
            .ok()?
            .and_then(|c| c.dyn_into::<web_sys::CanvasRenderingContext2d>().ok())?;

        let dpr = window.device_pixel_ratio() as f32;

        Some(Self {
            ctx,
            canvas,
            dpr,
            canvas_height: std::cell::Cell::new(0.0),
            is_hijacked: true,
            transparent_surface: false,
        })
    }

    /// Create an offscreen renderer from a canvas JS handle.
    pub fn new_offscreen(canvas_js: wasm_bindgen::JsValue, dpr: f32) -> Option<Self> {
        let canvas: HtmlCanvasElement = canvas_js.unchecked_into();
        let ctx_val = canvas.get_context("2d").ok()??;
        let ctx = ctx_val.unchecked_into();

        Some(Self {
            ctx,
            canvas,
            dpr,
            canvas_height: std::cell::Cell::new(0.0),
            is_hijacked: true,
            transparent_surface: false,
        })
    }

    /// 根据当前容器尺寸同步 Canvas 大小
    pub fn sync_size(&self, width: f32, height: f32, zoom: f32) {
        if self.is_hijacked {
            return;
        }
        self.canvas_height.set(height);
        self.canvas.set_width((width * self.dpr) as u32);
        self.canvas.set_height((height * self.dpr) as u32);
        let style = self.canvas.style();
        let _ = style.set_property("width", &format!("{}px", width));
        let _ = style.set_property("height", &format!("{}px", height));

        // 编辑器 canvas 使用本地坐标系：左上角为原点，Y 轴向下。
        // [Architectural Correction] 内部绘图单位应为 PDF Points，因此需要同时乘以 zoom 和 dpr
        let combined_scale = (self.dpr * zoom) as f64;
        let _ = self
            .ctx
            .set_transform(combined_scale, 0.0, 0.0, combined_scale, 0.0, 0.0);
    }

    pub fn measure_text_metrics(
        &self,
        text: &str,
        font_size: f32,
        font_name: &str,
        font_weight: &str,
        font_style: &str,
    ) -> TextMetricsSnapshot {
        self.ctx.set_font(&format!(
            "{} {} {}px {}",
            font_style, font_weight, font_size, font_name
        ));
        let measure_target = if text.is_empty() { "Hg" } else { text };
        match self.ctx.measure_text(measure_target) {
            Ok(metrics) => TextMetricsSnapshot {
                width: if text.is_empty() {
                    0.0
                } else {
                    metrics.width() as f32
                },
                _ascent: metrics.actual_bounding_box_ascent() as f32,
                _descent: metrics.actual_bounding_box_descent() as f32,
            },
            Err(_) => TextMetricsSnapshot {
                width: 0.0,
                _ascent: font_size * 0.8,
                _descent: font_size * 0.2,
            },
        }
    }

    pub fn clear_dirty_rect(&self, x: f32, y: f32, w: f32, h: f32) {
        if self.transparent_surface {
            self.ctx.clear_rect(
                x as f64 - 0.5,
                y as f64 - 0.5,
                (w + 1.0) as f64,
                (h + 1.0) as f64,
            );
            return;
        }
        self.ctx.set_fill_style_str("#ffffff");
        self.ctx.fill_rect(
            x as f64 - 0.5,
            y as f64 - 0.5,
            (w + 1.0) as f64,
            (h + 1.0) as f64,
        );
    }

    /// [Architectural Core] 统一全页面状态化渲染
    pub(crate) fn prepare_page_surface(
        &self,
        state: &pdf_viewer_core::models::PageState,
        _page_width: f32,
        _page_height: f32,
    ) {
        let page_scale = (state.zoom * state.dpr) as f64;
        let viewport_left_px = (state.viewport_left * state.dpr).max(0.0) as f64;
        let viewport_top_px = (state.viewport_top * state.dpr).max(0.0) as f64;

        let _ = self.ctx.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);
        self.ctx.set_fill_style_str("#ffffff");
        self.ctx.fill_rect(
            0.0,
            0.0,
            self.canvas.width() as f64,
            self.canvas.height() as f64,
        );
        let _ = self.ctx.set_transform(
            page_scale,
            0.0,
            0.0,
            page_scale,
            -viewport_left_px,
            -viewport_top_px,
        );
    }

    pub(crate) fn apply_page_transform(
        &self,
        state: &pdf_viewer_core::models::PageState,
        _page_width: f32,
        _page_height: f32,
    ) {
        let page_scale = (state.zoom * state.dpr) as f64;
        let viewport_left_px = (state.viewport_left * state.dpr).max(0.0) as f64;
        let viewport_top_px = (state.viewport_top * state.dpr).max(0.0) as f64;
        let _ = self.ctx.set_transform(
            page_scale,
            0.0,
            0.0,
            page_scale,
            -viewport_left_px,
            -viewport_top_px,
        );
    }
}
