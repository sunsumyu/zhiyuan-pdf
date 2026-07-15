use super::{
    current_time_ms, debug_log_canvas_method, draw_text_run_core, CanvasRenderer, CoordinateMode,
};
use crate::common::bbox::bbox_intersects;
use crate::editor::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use crate::editor::paragraph_overlay::{collect_overlays, ParagraphRenderOverlayOwner};
use crate::editor::replacement_region::build_region;
use crate::render::canvas_overlay::{
    draw_active_editor_shell_overlay_page, draw_persisted_paragraph_overlay_page, path_bbox_summary,
};
use crate::render::effective_page_plan::{
    build_effective_glyph_render_plan, build_effective_vector_render_plan,
    EffectiveGlyphRenderEntry, EffectiveVectorRenderEntry, SuppressedVectorTextRuns,
};
use crate::render::prepared_scene::PreparedPageScene;
use crate::render::progressive::ProgressiveVectorRenderTask;
use crate::viewport_culling::{
    glyph_run_intersects_viewport, path_bbox, resolve_page_viewport_bbox,
};
use pdf_viewer_core::models::{
    BoundingBox, PageState, VectorImageObject, VectorPageModel, VectorPathObject,
    VectorRenderObject, VectorTextObject,
};
use pdf_viewer_core::typography::font_resolver::resolve_font_face;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlImageElement, ImageBitmap};

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

    pub(super) fn draw_path_object(&self, path: &VectorPathObject, object_index: Option<usize>) {
        let bbox = path_bbox(path);
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
                    .map(crate::render::canvas::debug_bbox_intersects_active_shell)
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

    pub(super) fn draw_image_object(
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
        if let Some(img_js) = img_val.clone().dyn_into::<HtmlImageElement>().ok() {
            self.ctx.save();
            let _ = self.ctx.draw_image_with_html_image_element_and_dw_and_dh(
                &img_js,
                image.x as f64,
                image.y as f64,
                image.width as f64,
                image.height as f64,
            );
            self.ctx.restore();
        } else if let Some(img_js) = img_val.dyn_into::<ImageBitmap>().ok() {
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

    pub(super) fn draw_text_object(
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
            draw_text_run_core(
                &self.ctx,
                self.dpr,
                &run.text,
                run.tx,
                run.ty,
                run.font_size,
                &run.color,
                &resolved_font.render_family,
                resolved_font.identity.weight as u16,
                if run.is_italic { "italic" } else { "normal" },
                run.is_underline,
                run.a.max(0.01),
                run.render_mode as i32,
                Some(&run.char_origins),
                CoordinateMode::PageSpace,
            );
        }
    }

    pub fn render_vector_slice(
        &self,
        state: &PageState,
        vector_model: &VectorPageModel,
        task: &mut ProgressiveVectorRenderTask,
        image_provider: &js_sys::Map,
        max_items: usize,
        budget_ms: Option<f64>,
        clear_first: bool,
    ) -> usize {
        if clear_first {
            self.prepare_page_surface(state, vector_model.width, vector_model.height);
        } else {
            self.apply_page_transform(state, vector_model.width, vector_model.height);
        }

        let max_items = max_items.max(1);
        let budget_ms = budget_ms.filter(|budget| budget.is_finite() && *budget > 0.0);
        let slice_start_time = budget_ms.map(|_| current_time_ms());
        let mut processed_items = 0;

        while task.next_index < task.entries.len() && processed_items < max_items {
            if let (Some(start_time), Some(budget_ms)) = (slice_start_time, budget_ms) {
                if processed_items > 0 {
                    let now = current_time_ms();
                    if now - start_time >= budget_ms {
                        break;
                    }
                }
            }

            let visible_index = task.next_index;
            let Some(entry) = task.entries.get(visible_index) else {
                task.next_index += 1;
                continue;
            };
            match entry {
                EffectiveVectorRenderEntry::Object {
                    object_index,
                    suppressed_text_runs,
                } => {
                    let Some(obj) = vector_model.objects.get(*object_index) else {
                        task.next_index += 1;
                        continue;
                    };
                    self.draw_vector_object(
                        obj,
                        Some(*object_index),
                        image_provider,
                        Some(suppressed_text_runs),
                    );
                }
                EffectiveVectorRenderEntry::ParagraphOverlay(overlay) => {
                    dbg_event(
                        "paint.overlay",
                        "method.render-vector-slice.overlay-entry",
                        vec![
                            dbg_field("paragraphId", overlay.target.paragraph_id.as_str()),
                            dbg_field("entryKind", "paragraphOverlay"),
                            dbg_field(
                                "owner",
                                match overlay.owner {
                                    ParagraphRenderOverlayOwner::ActiveEditorShell => {
                                        "active-editor-shell"
                                    }
                                    ParagraphRenderOverlayOwner::PersistedPageCanvas => {
                                        "persisted-page-canvas"
                                    }
                                },
                            ),
                        ],
                    );
                    match overlay.owner {
                        ParagraphRenderOverlayOwner::ActiveEditorShell => {
                            draw_active_editor_shell_overlay_page(
                                self,
                                &overlay,
                                None,
                                image_provider,
                                overlay.marker_text_override.as_deref(),
                            );
                        }
                        ParagraphRenderOverlayOwner::PersistedPageCanvas => {
                            draw_persisted_paragraph_overlay_page(
                                self,
                                &overlay.target,
                                &overlay.draft_text,
                                None,
                                image_provider,
                                overlay.marker_text_override.as_deref(),
                                "persisted-page-canvas",
                            );
                        }
                    }
                }
            }
            task.next_index += 1;
            processed_items += 1;
        }

        processed_items
    }

    pub fn render_page(
        &self,
        state: &PageState,
        image_provider: &js_sys::Map,
        prepared_scene: Option<&PreparedPageScene>,
    ) {
        let plan = match &state.paint_plan {
            Some(p) => p,
            None => return,
        };

        let viewport_bbox = resolve_page_viewport_bbox(state, plan.width, plan.height);
        dbg_event(
            "canvas.render",
            "render_page.start",
            vec![
                dbg_field("width", plan.width),
                dbg_field("height", plan.height),
                dbg_field("zoom", state.zoom),
                dbg_field("viewport", format!("{:?}", viewport_bbox)),
                dbg_field("has_vector_model", state.vector_model.is_some()),
            ],
        );
        self.prepare_page_surface(state, plan.width, plan.height);
        let overlays = collect_overlays(plan, state.vector_model.as_ref());
        dbg_event(
            "canvas.render",
            "overlay-summary",
            vec![dbg_field("overlayCount", overlays.len())],
        );
        for (ov_idx, ov) in overlays.iter().enumerate() {
            dbg_event(
                "canvas.render",
                "overlay-detail",
                vec![
                    dbg_field("index", ov_idx),
                    dbg_field("paragraphId", ov.target.paragraph_id.as_str()),
                    dbg_field("owner", format!("{:?}", ov.owner)),
                    dbg_field("replacesSource", ov.replaces_source),
                    dbg_field(
                        "sourceObjectIndices",
                        format!("{:?}", ov.source_object_indices),
                    ),
                    dbg_field("sourceTextLen", ov.source_text.chars().count()),
                    dbg_field("draftTextLen", ov.draft_text.chars().count()),
                ],
            );
        }

        if let Some(vector_model) = &state.vector_model {
            let effective_plan = build_effective_vector_render_plan(
                vector_model,
                prepared_scene,
                &viewport_bbox,
                &overlays,
            );

            let mut draw_text_count = 0;
            let mut draw_path_count = 0;
            let mut draw_image_count = 0;

            for entry in effective_plan {
                match entry {
                    EffectiveVectorRenderEntry::Object {
                        object_index,
                        suppressed_text_runs,
                    } => {
                        let Some(obj) = vector_model.objects.get(object_index) else {
                            continue;
                        };
                        match obj {
                            VectorRenderObject::Text(_) => draw_text_count += 1,
                            VectorRenderObject::Path(_) => draw_path_count += 1,
                            VectorRenderObject::Image(_) => draw_image_count += 1,
                        }
                        self.draw_vector_object(
                            obj,
                            Some(object_index),
                            image_provider,
                            Some(&suppressed_text_runs),
                        );
                    }
                    EffectiveVectorRenderEntry::ParagraphOverlay(overlay) => {
                        let replacement_region = build_region(&overlay.target);
                        let overlay_cull_bbox = replacement_region.viewport_cull_bbox(plan.width);
                        let intersects = bbox_intersects(&overlay_cull_bbox, &viewport_bbox);
                        if intersects {
                            dbg_event(
                                "paint.overlay",
                                "method.render-page.overlay-entry",
                                vec![
                                    dbg_field("paragraphId", overlay.target.paragraph_id.as_str()),
                                    dbg_field("entryKind", "paragraphOverlay"),
                                    dbg_field(
                                        "owner",
                                        match overlay.owner {
                                            ParagraphRenderOverlayOwner::ActiveEditorShell => {
                                                "active-editor-shell"
                                            }
                                            ParagraphRenderOverlayOwner::PersistedPageCanvas => {
                                                "persisted-page-canvas"
                                            }
                                        },
                                    ),
                                ],
                            );
                            match overlay.owner {
                                ParagraphRenderOverlayOwner::ActiveEditorShell => {
                                    draw_active_editor_shell_overlay_page(
                                        self,
                                        &overlay,
                                        Some(vector_model),
                                        image_provider,
                                        overlay.marker_text_override.as_deref(),
                                    );
                                }
                                ParagraphRenderOverlayOwner::PersistedPageCanvas => {
                                    draw_persisted_paragraph_overlay_page(
                                        self,
                                        &overlay.target,
                                        &overlay.draft_text,
                                        Some(vector_model),
                                        image_provider,
                                        overlay.marker_text_override.as_deref(),
                                        "persisted-page-canvas",
                                    );
                                }
                            }
                        }
                    }
                }
            }
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
                "[CANVAS-DBG] render_page finished. Drew: paths={}, images={}, texts={}",
                draw_path_count, draw_image_count, draw_text_count
            )));
            return;
        }

        let effective_plan = build_effective_glyph_render_plan(plan, &viewport_bbox, &overlays);
        for entry in effective_plan {
            match entry {
                EffectiveGlyphRenderEntry::ParagraphOverlay(overlay) => {
                    dbg_event(
                        "paint.overlay",
                        "method.render-page.glyph-overlay-entry",
                        vec![
                            dbg_field("paragraphId", overlay.target.paragraph_id.as_str()),
                            dbg_field("entryKind", "glyphParagraphOverlay"),
                            dbg_field(
                                "owner",
                                match overlay.owner {
                                    ParagraphRenderOverlayOwner::ActiveEditorShell => {
                                        "active-editor-shell"
                                    }
                                    ParagraphRenderOverlayOwner::PersistedPageCanvas => {
                                        "persisted-page-canvas"
                                    }
                                },
                            ),
                        ],
                    );
                    match overlay.owner {
                        ParagraphRenderOverlayOwner::ActiveEditorShell => {
                            draw_active_editor_shell_overlay_page(
                                self,
                                &overlay,
                                None,
                                image_provider,
                                overlay.marker_text_override.as_deref(),
                            );
                        }
                        ParagraphRenderOverlayOwner::PersistedPageCanvas => {
                            draw_persisted_paragraph_overlay_page(
                                self,
                                &overlay.target,
                                &overlay.draft_text,
                                None,
                                image_provider,
                                overlay.marker_text_override.as_deref(),
                                "persisted-page-canvas",
                            );
                        }
                    }
                }
                EffectiveGlyphRenderEntry::Paragraph(reference) => {
                    let Some(region) = plan.regions.get(reference.region_index) else {
                        continue;
                    };
                    let Some(paragraph) = region.paragraphs.get(reference.paragraph_index) else {
                        continue;
                    };
                    for (run_index, run) in paragraph.runs.iter().enumerate() {
                        if reference.suppressed_run_indices.contains(&run_index)
                            || run.object_ids.iter().any(|object_id| {
                                reference.suppressed_run_object_ids.contains(object_id)
                            })
                        {
                            continue;
                        }
                        if !glyph_run_intersects_viewport(run, &viewport_bbox) {
                            continue;
                        }
                        draw_text_run_core(
                            &self.ctx,
                            self.dpr,
                            &run.text,
                            run.origin_x,
                            run.origin_y,
                            run.font_size,
                            &run.color,
                            &run.resolved_font.render_family,
                            run.resolved_font.identity.weight as u16,
                            if run.is_italic { "italic" } else { "normal" },
                            run.is_underline,
                            run.scale_x,
                            match run.paint_mode {
                                pdf_viewer_core::models::PaintMode::Stroke => 1,
                                pdf_viewer_core::models::PaintMode::FillStroke => 2,
                                _ => 0,
                            },
                            Some(&run.char_origins),
                            CoordinateMode::PageSpace,
                        );
                    }
                }
            }
        }
    }
}
