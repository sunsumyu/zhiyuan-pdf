//! Vector object drawing: path, image, text objects, and progressive slice dispatch.

use super::debug::{debug_bbox_intersects_active_shell, debug_log_canvas_method};
use super::CanvasRenderer;
use super::CoordinateMode;
use crate::editor::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use crate::render::canvas_overlay::path_bbox_summary;
use crate::render::effective_page_plan::SuppressedVectorTextRuns;
use crate::viewport_culling::path_object_bbox;
use pdf_viewer_core::models::{
    BoundingBox, VectorImageObject, VectorPathObject, VectorRenderObject, VectorTextObject,
};
use pdf_viewer_core::typography::font_resolver::resolve_font_face;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

impl CanvasRenderer {
    pub(crate) fn draw_vector_object(
        &self,
        obj: &VectorRenderObject,
        object_index: Option<usize>,
        image_provider: &js_sys::Map,
        suppressed_text_runs: Option<&SuppressedVectorTextRuns>,
    ) {
        match obj {
            VectorRenderObject::Path(path) => {
                self.draw_path_object(path, object_index);
            }
            VectorRenderObject::Image(image) => {
                self.draw_image_object(image, object_index, image_provider);
            }
            VectorRenderObject::Text(text_obj) => {
                self.draw_text_object(text_obj, object_index, suppressed_text_runs);
            }
        }
    }

    fn draw_path_object(&self, path: &VectorPathObject, object_index: Option<usize>) {
        let bbox = path_object_bbox(path);
        debug_log_canvas_method(
            "method.draw-vector-object.path",
            "path",
            object_index,
            Some(path.id.as_str()),
            bbox,
            vec![
                dbg_field(
                    "strokeColor",
                    path.stroke_color.as_deref().unwrap_or("none"),
                ),
                dbg_field("fillColor", path.fill_color.as_deref().unwrap_or("none")),
                dbg_field("strokeWidth", path.stroke_width),
            ],
        );
        if let Some((path_width, path_height)) = path_bbox_summary(path) {
            let is_suspicious_horizontal_path =
                path_width >= 120.0 && path_height <= (path.stroke_width.max(0.0) * 6.0).max(30.0);
            if is_suspicious_horizontal_path
                && bbox
                    .as_ref()
                    .map(debug_bbox_intersects_active_shell)
                    .unwrap_or(false)
            {
                dbg_event(
                    "canvas.draw",
                    "vector-path",
                    vec![
                        dbg_field("objectId", path.id.as_str()),
                        dbg_field(
                            "strokeColor",
                            path.stroke_color.as_deref().unwrap_or("none"),
                        ),
                        dbg_field("fillColor", path.fill_color.as_deref().unwrap_or("none")),
                        dbg_field("strokeWidth", path.stroke_width),
                        dbg_field("pathWidth", path_width),
                        dbg_field("pathHeight", path_height),
                    ],
                );
            }
        }
        self.ctx.save();
        self.ctx.set_line_width(path.stroke_width.max(0.4) as f64);
        self.ctx.begin_path();
        for seg in &path.segments {
            match seg.command.as_str() {
                "move" => {
                    if let Some([x, y]) = seg.points.first().copied() {
                        self.ctx.move_to(x as f64, y as f64);
                    }
                }
                "line" => {
                    if let Some([x, y]) = seg.points.first().copied() {
                        self.ctx.line_to(x as f64, y as f64);
                    }
                }
                "close" => self.ctx.close_path(),
                _ => {}
            }
        }
        if path.fill {
            if let Some(color) = &path.fill_color {
                self.ctx.set_fill_style_str(color);
                self.ctx.fill();
            }
        }
        if path.stroke {
            if let Some(color) = &path.stroke_color {
                self.ctx.set_stroke_style_str(color);
                self.ctx.stroke();
            }
        }
        self.ctx.restore();
    }

    fn draw_image_object(
        &self,
        image: &VectorImageObject,
        object_index: Option<usize>,
        image_provider: &js_sys::Map,
    ) {
        let bbox = Some(BoundingBox {
            left: image.x,
            top: image.y,
            right: image.x + image.width.max(0.0),
            bottom: image.y + image.height.max(0.0),
        });
        debug_log_canvas_method(
            "method.draw-vector-object.image",
            "image",
            object_index,
            Some(image.id.as_str()),
            bbox,
            vec![
                dbg_field("width", image.width),
                dbg_field("height", image.height),
            ],
        );
        let img_val = image_provider.get(&JsValue::from_str(&image.id));
        if let Ok(img_js) = img_val.clone().dyn_into::<web_sys::HtmlImageElement>() {
            self.ctx.save();
            let _ = self.ctx.draw_image_with_html_image_element_and_dw_and_dh(
                &img_js,
                image.x as f64,
                image.y as f64,
                image.width as f64,
                image.height as f64,
            );
            self.ctx.restore();
        } else if let Ok(img_js) = img_val.dyn_into::<web_sys::ImageBitmap>() {
            self.ctx.save();
            let _ = self.ctx.draw_image_with_image_bitmap_and_dw_and_dh(
                &img_js,
                image.x as f64,
                image.y as f64,
                image.width as f64,
                image.height as f64,
            );
            self.ctx.restore();
        }
    }

    fn draw_text_object(
        &self,
        text_obj: &VectorTextObject,
        object_index: Option<usize>,
        suppressed_text_runs: Option<&SuppressedVectorTextRuns>,
    ) {
        let text_bbox = text_obj
            .runs
            .iter()
            .fold(None, |acc: Option<BoundingBox>, run| {
                let run_bbox = BoundingBox {
                    left: run.tx,
                    top: run.ty - run.font_size.max(0.0),
                    right: run.tx + run.width.max(0.0),
                    bottom: run.ty,
                };
                Some(match acc {
                    Some(current) => BoundingBox {
                        left: current.left.min(run_bbox.left),
                        top: current.top.min(run_bbox.top),
                        right: current.right.max(run_bbox.right),
                        bottom: current.bottom.max(run_bbox.bottom),
                    },
                    None => run_bbox,
                })
            });
        debug_log_canvas_method(
            "method.draw-vector-object.text",
            "text",
            object_index,
            Some(text_obj.id.as_str()),
            text_bbox,
            vec![dbg_field("runCount", text_obj.runs.len())],
        );
        for (run_index, run) in text_obj.runs.iter().enumerate() {
            if run.render_mode == 3 {
                continue;
            }
            let should_skip_run = suppressed_text_runs
                .map(|suppressed| suppressed.suppresses_run(run_index, run))
                .unwrap_or(false);
            if should_skip_run {
                continue;
            }
            let resolved_font = resolve_font_face(&run.font_name, run.font_hints.as_ref());
            super::draw::draw_text_run_core(
                &self.ctx,
                self.dpr,
                &run.text,
                run.tx,
                run.ty,
                run.font_size,
                &run.color,
                &resolved_font.render_family,
                if run.is_bold { "bold" } else { "normal" },
                if run.is_italic { "italic" } else { "normal" },
                run.is_underline,
                run.a.max(0.01),
                run.render_mode as i32,
                Some(&run.char_origins),
                CoordinateMode::PageSpace,
            );
        }
    }
}
