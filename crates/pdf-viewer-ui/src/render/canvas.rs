use crate::editor::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use crate::editor::mode::read_active_editor_state;
use crate::editor::paragraph_overlay::{
    collect_paragraph_render_overlays, ParagraphRenderOverlayOwner,
};
use crate::editor::replacement_region::paragraph_replacement_region;
use crate::render::canvas_overlay::{
    draw_active_editor_shell_overlay_page, draw_persisted_paragraph_overlay_page, path_bbox_summary,
};
use crate::render::effective_page_plan::{
    build_effective_glyph_render_plan, build_effective_vector_render_plan,
    EffectiveGlyphRenderEntry, EffectiveVectorRenderEntry, SuppressedVectorTextRuns,
};
use crate::render::prepared_scene::PreparedPageScene;
use crate::render::progressive::ProgressiveVectorRenderTask;
use crate::common::bbox::bbox_intersects;
use crate::viewport_culling::{
    glyph_run_intersects_viewport, path_object_bbox, resolve_page_viewport_bbox,
};
use js_sys;
use pdf_viewer_core::models::{BoundingBox, PageState, VectorRenderObject};
use pdf_viewer_core::render::renderer::{DrawCommand, PdfRenderer};
use pdf_viewer_core::typography::font_resolver::resolve_font_face;
use std::cell::Cell;
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, HtmlImageElement, ImageBitmap};

#[derive(Clone, Copy)]
pub(crate) enum CoordinateMode {
    PageSpace,
    EditorLocal,
}

pub struct CanvasRenderer {
    pub ctx: CanvasRenderingContext2d,
    pub canvas: HtmlCanvasElement,
    pub dpr: f32,
    pub canvas_height: Cell<f32>,
    pub is_hijacked: bool,
    pub transparent_surface: bool,
}

#[wasm_bindgen]
pub fn render_run_standalone(
    ctx: CanvasRenderingContext2d,
    dpr: f32,
    text: String,
    x: f32,
    baseline_y: f32,
    font_size: f32,
    color: String,
    font_name: String,
    font_weight: String,
    font_style: String,
    is_underline: bool,
    scale_x: f32,
    render_mode: i32,
    char_origins: Option<Vec<f32>>,
) {
    draw_text_run_core(
        &ctx,
        dpr,
        &text,
        x,
        baseline_y,
        font_size,
        &color,
        &font_name,
        &font_weight,
        &font_style,
        is_underline,
        scale_x,
        render_mode,
        char_origins.as_deref(),
        CoordinateMode::PageSpace,
    );
}

pub struct TextMetricsSnapshot {
    pub width: f32,
    pub _ascent: f32,
    pub _descent: f32,
}

fn current_time_ms() -> f64 {
    js_sys::Date::now()
}

fn active_shell_bbox_for_debug() -> Option<BoundingBox> {
    read_active_editor_state()
        .map(|state| paragraph_replacement_region(&state.target).text_clear_bbox)
}

fn debug_bbox_intersects_active_shell(bbox: &BoundingBox) -> bool {
    active_shell_bbox_for_debug()
        .map(|shell| bbox_intersects(bbox, &shell))
        .unwrap_or(false)
}

fn debug_log_canvas_method(
    action: &str,
    object_type: &str,
    object_index: Option<usize>,
    object_id: Option<&str>,
    bbox: Option<BoundingBox>,
    extra: Vec<crate::editor::debug_trace::EditorDebugField>,
) {
    let intersects_shell = bbox
        .as_ref()
        .map(debug_bbox_intersects_active_shell)
        .unwrap_or(false);
    if !intersects_shell {
        return;
    }
    let mut details = vec![
        dbg_field("objectType", object_type),
        dbg_field("intersectsShell", intersects_shell),
    ];
    if let Some(index) = object_index {
        details.push(dbg_field("objectIndex", index));
    }
    if let Some(id) = object_id {
        details.push(dbg_field("objectId", id));
    }
    if let Some(bounds) = bbox {
        details.push(dbg_field(
            "bbox",
            format!(
                "{:.1},{:.1},{:.1},{:.1}",
                bounds.left, bounds.top, bounds.right, bounds.bottom
            ),
        ));
    }
    details.extend(extra);
    dbg_event("canvas.draw", action, details);
}

impl CanvasRenderer {
    // Unused new() purged

    pub fn new_overlay(canvas: HtmlCanvasElement) -> Self {
        let attrs = web_sys::ContextAttributes2d::new();
        attrs.set_alpha(true);

        let ctx = canvas
            .get_context_with_context_options("2d", &attrs.into())
            .unwrap()
            .unwrap()
            .dyn_into::<CanvasRenderingContext2d>()
            .unwrap();

        let dpr = web_sys::window().unwrap().device_pixel_ratio() as f32;

        Self {
            ctx,
            canvas,
            dpr,
            canvas_height: Cell::new(0.0),
            is_hijacked: false,
            transparent_surface: true,
        }
    }

    pub fn new_hijacked(target_id: &str) -> Option<Self> {
        let window = web_sys::window()?;
        let document = window.document()?;
        let canvas = document
            .get_element_by_id(target_id)?
            .dyn_into::<HtmlCanvasElement>()
            .ok()?;

        // 注意：劫持模式下我们不设置 alpha: false，因为我们要保留并操作现有的 Context
        let ctx = canvas
            .get_context("2d")
            .ok()?
            .and_then(|c| c.dyn_into::<CanvasRenderingContext2d>().ok())?;

        let dpr = window.device_pixel_ratio() as f32;

        Some(Self {
            ctx,
            canvas,
            dpr,
            canvas_height: Cell::new(0.0),
            is_hijacked: true,
            transparent_surface: false,
        })
    }

    pub fn new_offscreen(canvas_js: JsValue, dpr: f32) -> Option<Self> {
        let canvas: HtmlCanvasElement = canvas_js.unchecked_into();
        let ctx_val = canvas.get_context("2d").ok()??;
        let ctx: CanvasRenderingContext2d = ctx_val.unchecked_into();

        Some(Self {
            ctx,
            canvas,
            dpr,
            canvas_height: Cell::new(0.0),
            is_hijacked: true, // don't resize DOM
            transparent_surface: false,
        })
    }

    /// 创建一个已知尺寸且已初始化的渲染器（供 standalone 模式使用）
    // Unused new_with_size() purged

    /// 根据当前容器尺寸同步 Canvas 大小
    pub fn sync_size(&self, width: f32, height: f32, zoom: f32) {
        if self.is_hijacked {
            // 在劫持模式下，不应修改外部画布的物理尺寸
            return;
        }
        self.canvas_height.set(height);
        let _ = self.canvas.set_width((width * self.dpr) as u32);
        let _ = self.canvas.set_height((height * self.dpr) as u32);
        let style = self.canvas.style();
        style
            .set_property("width", &format!("{}px", width))
            .unwrap();
        style
            .set_property("height", &format!("{}px", height))
            .unwrap();

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

    // Unused snap_to_pixel() purged

    pub fn draw_text_run(
        &self,
        text: &str,
        x: f32,
        baseline_y: f32,
        font_size: f32,
        color: &str,
        font_name: &str,
        font_weight: &str,
        font_style: &str,
        is_underline: bool,
        scale_x: f32,
        render_mode: i32,
        char_origins: Option<&[f32]>,
    ) {
        draw_text_run_core(
            &self.ctx,
            self.dpr,
            text,
            x,
            baseline_y,
            font_size,
            color,
            font_name,
            font_weight,
            font_style,
            is_underline,
            scale_x,
            render_mode,
            char_origins,
            CoordinateMode::EditorLocal,
        );
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
        // 在劫持模式下，主画布可能已经处于某种变换（如 Zoom）之下
        // 我们直接在当前坐标空间下绘图以保持一致
        self.ctx.set_fill_style_str("#ffffff");
        // 增加少量 buffer 确保擦除干净
        self.ctx.fill_rect(
            x as f64 - 0.5,
            y as f64 - 0.5,
            (w + 1.0) as f64,
            (h + 1.0) as f64,
        );
    }

    // 劫持模式下，我们进入相对于锚点的坐标空间
    // Legacy begin_anchor_render purged.

    /// [Architectural Core] 统一全页面状态化渲染
    fn prepare_page_surface(&self, state: &PageState, _page_width: f32, _page_height: f32) {
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

    fn apply_page_transform(&self, state: &PageState, _page_width: f32, _page_height: f32) {
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

    fn draw_vector_object(
        &self,
        obj: &VectorRenderObject,
        object_index: Option<usize>,
        image_provider: &js_sys::Map,
        suppressed_text_runs: Option<&SuppressedVectorTextRuns>,
    ) {
        match obj {
            VectorRenderObject::Path(path) => {
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
                    let is_suspicious_horizontal_path = path_width >= 120.0
                        && path_height <= (path.stroke_width.max(0.0) * 6.0).max(30.0);
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
                                dbg_field(
                                    "fillColor",
                                    path.fill_color.as_deref().unwrap_or("none"),
                                ),
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
            VectorRenderObject::Image(image) => {
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
            VectorRenderObject::Text(text_obj) => {
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
    }

    pub fn render_vector_slice(
        &self,
        state: &PageState,
        vector_model: &pdf_viewer_core::models::VectorPageModel,
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
                                overlay.marker_text_override.as_deref(),
                            );
                        }
                        ParagraphRenderOverlayOwner::PersistedPageCanvas => {
                            draw_persisted_paragraph_overlay_page(
                                self,
                                &overlay.target,
                                &overlay.draft_text,
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
        image_provider: &js_sys::Map, // 映射 ID -> HtmlImageElement
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
        let overlays = collect_paragraph_render_overlays(plan, state.vector_model.as_ref());
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
                        let replacement_region = paragraph_replacement_region(&overlay.target);
                        let overlay_cull_bbox =
                            replacement_region.viewport_cull_bbox_for_page_width(plan.width);
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
                                        overlay.marker_text_override.as_deref(),
                                    );
                                }
                                ParagraphRenderOverlayOwner::PersistedPageCanvas => {
                                    draw_persisted_paragraph_overlay_page(
                                        self,
                                        &overlay.target,
                                        &overlay.draft_text,
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
                                overlay.marker_text_override.as_deref(),
                            );
                        }
                        ParagraphRenderOverlayOwner::PersistedPageCanvas => {
                            draw_persisted_paragraph_overlay_page(
                                self,
                                &overlay.target,
                                &overlay.draft_text,
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
                            if run.is_bold { "bold" } else { "normal" },
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

impl PdfRenderer for CanvasRenderer {
    fn render(&mut self, commands: &[DrawCommand]) {
        for cmd in commands {
            match cmd {
                DrawCommand::Text {
                    text,
                    x,
                    y,
                    font_size,
                    color,
                    font_name,
                } => {
                    self.ctx.set_fill_style_str(color);
                    self.ctx.set_font(&format!("{}px {}", font_size, font_name));
                    let _ = self.ctx.fill_text(text, *x as f64, *y as f64);
                }
                DrawCommand::Rect {
                    x,
                    y,
                    width,
                    height,
                    color,
                    is_fill,
                } => {
                    let is_suspicious_horizontal_rect = *width >= 120.0 && *height <= 6.0;
                    if is_suspicious_horizontal_rect {
                        dbg_event(
                            "canvas.draw",
                            if *is_fill {
                                "draw-command-fill-rect"
                            } else {
                                "draw-command-stroke-rect"
                            },
                            vec![
                                dbg_field("x1", *x),
                                dbg_field("y1", *y),
                                dbg_field("width", *width),
                                dbg_field("height", *height),
                                dbg_field("color", color),
                            ],
                        );
                    }
                    if *is_fill {
                        self.ctx.set_fill_style_str(color);
                        self.ctx
                            .fill_rect(*x as f64, *y as f64, *width as f64, *height as f64);
                    } else {
                        self.ctx.set_stroke_style_str(color);
                        self.ctx
                            .stroke_rect(*x as f64, *y as f64, *width as f64, *height as f64);
                    }
                }
                DrawCommand::Line {
                    x1,
                    y1,
                    x2,
                    y2,
                    color,
                    width,
                } => {
                    let line_width = (*x2 - *x1).abs();
                    let line_height = (*y2 - *y1).abs();
                    let is_suspicious_horizontal_line = line_width >= 120.0 && line_height <= 6.0;
                    if is_suspicious_horizontal_line {
                        dbg_event(
                            "canvas.draw",
                            "draw-command-line",
                            vec![
                                dbg_field("x1", *x1),
                                dbg_field("y1", *y1),
                                dbg_field("x2", *x2),
                                dbg_field("y2", *y2),
                                dbg_field("strokeWidth", *width),
                                dbg_field("color", color),
                            ],
                        );
                    }
                    self.ctx.begin_path();
                    self.ctx.set_stroke_style_str(color);
                    self.ctx.set_line_width(*width as f64);
                    self.ctx.move_to(*x1 as f64, *y1 as f64);
                    self.ctx.line_to(*x2 as f64, *y2 as f64);
                    self.ctx.stroke();
                }
            }
        }
    }

    fn clear(&mut self) {
        if self.is_hijacked {
            // 在主画布模式下，禁止进行全画布清理，否则会擦掉整页 PDF
            return;
        }
        let w = self.canvas.width() as f64;
        let h = self.canvas.height() as f64;
        let _current_canvas_height = self.canvas_height.get();

        let _ = self.ctx.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);
        self.ctx.set_fill_style_str("#ffffff");
        self.ctx.fill_rect(0.0, 0.0, w, h);
        let _ = self
            .ctx
            .set_transform(self.dpr as f64, 0.0, 0.0, self.dpr as f64, 0.0, 0.0);
    }

    fn name(&self) -> &str {
        "Canvas2D"
    }
}

pub(crate) fn draw_text_run_core(
    ctx: &CanvasRenderingContext2d,
    dpr: f32,
    text: &str,
    x: f32,
    baseline_y: f32,
    font_size: f32,
    color: &str,
    font_name: &str,
    font_weight: &str,
    font_style: &str,
    is_underline: bool,
    scale_x: f32,
    render_mode: i32,
    char_origins: Option<&[f32]>,
    coordinate_mode: CoordinateMode,
) {
    if render_mode == 3 {
        return;
    }
    let snap_to_pixel = |val: f32| -> f64 { (val * dpr).round() as f64 / dpr as f64 };
    let y_scale = match coordinate_mode {
        CoordinateMode::PageSpace => 1.0,
        CoordinateMode::EditorLocal => 1.0,
    };
    let effective_weight = if font_weight == "bold" { "600" } else { "400" };

    ctx.save();
    ctx.set_font(&format!(
        "{} {} {}px {}",
        font_style, effective_weight, font_size, font_name
    ));
    ctx.set_fill_style_str(color);
    ctx.set_stroke_style_str(color);
    ctx.set_text_baseline("alphabetic");
    ctx.set_line_join("round");
    ctx.set_miter_limit(2.0);
    ctx.set_line_width((font_size * 0.03).max(0.4) as f64);

    let snapped_x = snap_to_pixel(x);
    let snapped_baseline_y = snap_to_pixel(baseline_y);
    let _ = ctx.translate(snapped_x, snapped_baseline_y);

    if let Some(origins) = char_origins {
        let _ = ctx.save();
        let _ = ctx.scale(1.0, y_scale);
        for (index, ch) in text.chars().enumerate() {
            let mut glyph_buf = [0_u8; 4];
            let glyph = ch.encode_utf8(&mut glyph_buf);
            let origin_x = origins.get(index).copied().unwrap_or(0.0);
            let origin_x = snap_to_pixel(origin_x);
            if render_mode == 1 || render_mode == 2 {
                let _ = ctx.stroke_text(glyph, origin_x, 0.0);
            }
            if render_mode == 0 || render_mode == 2 {
                let _ = ctx.fill_text(glyph, origin_x, 0.0);
            }
        }
        let _ = ctx.restore();
    } else {
        let _ = ctx.save();
        let _ = ctx.scale(scale_x as f64, y_scale);
        if render_mode == 1 || render_mode == 2 {
            let _ = ctx.stroke_text(text, 0.0, 0.0);
        }
        if render_mode == 0 || render_mode == 2 {
            let _ = ctx.fill_text(text, 0.0, 0.0);
        }
        let _ = ctx.restore();
    }

    if is_underline {
        let measured_width = ctx
            .measure_text(text)
            .ok()
            .map(|metrics| metrics.width() as f32)
            .unwrap_or(0.0);
        let underline_width = if let Some(origins) = char_origins {
            let glyph_count = text.chars().count();
            if glyph_count <= 1 {
                measured_width.max(0.0)
            } else {
                let average_width = measured_width / glyph_count.max(1) as f32;
                origins.last().copied().unwrap_or(0.0) + average_width.max(0.0)
            }
        } else {
            measured_width * scale_x.max(0.01)
        };
        if underline_width > 0.0 {
            let underline_y = snap_to_pixel(font_size * 0.12);
            dbg_event(
                "canvas.draw",
                "underline-stroke",
                vec![
                    dbg_field("color", color),
                    dbg_field("width", underline_width),
                    dbg_field("height", (font_size * 0.055).max(0.8)),
                    dbg_field("strokeWidth", (font_size * 0.055).max(0.8)),
                ],
            );
            ctx.begin_path();
            ctx.set_line_width((font_size * 0.055).max(0.8) as f64);
            ctx.move_to(0.0, underline_y);
            ctx.line_to(snap_to_pixel(underline_width), underline_y);
            ctx.stroke();
        }
    }

    ctx.restore();
}
