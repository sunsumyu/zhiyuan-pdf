use crate::editor::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use crate::editor::draft_layout::build_persisted_overlay_render_plan;
use crate::editor::mode::get_active_editor_state;
use crate::editor::replacement_region::paragraph_replacement_region;
use crate::editor::session::ActiveEditorTarget;
use crate::editor::text_geometry::measure_editor_layout_text_width as measure_editor_layout_text_width_shared;
use crate::render::effective_page_plan::{
    build_effective_glyph_render_plan, build_effective_vector_render_plan,
    EffectiveGlyphRenderEntry, EffectiveVectorRenderEntry, SuppressedVectorTextRuns,
};
use crate::editor::paragraph_overlay::{
    collect_paragraph_render_overlays, ParagraphRenderOverlayOwner, ParagraphRenderOverlay,
};
use crate::render::prepared_scene::PreparedPageScene;
use crate::render::progressive::ProgressiveVectorRenderTask;
use crate::utils::bbox::bbox_intersects;
use crate::utils::debug::truncate_debug_text;
use crate::viewport_culling::{
    glyph_run_intersects_viewport, path_object_bbox, resolve_page_viewport_bbox,
};
use js_sys;
use pdf_viewer_core::font_resolver::resolve_font_face;
use pdf_viewer_core::models::{BoundingBox, PageState, VectorRenderObject};
use pdf_viewer_core::renderer::{DrawCommand, PdfRenderer};
use std::cell::Cell;
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, HtmlImageElement};

#[derive(Clone, Copy)]
enum CoordinateMode {
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
    get_active_editor_state()
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
                if let Some(b) = bbox.as_ref() {
                    web_sys::console::log_1(&JsValue::from_str(&format!(
                        "[CANVAS-DBG] Path id={} bbox=({:.1},{:.1})-({:.1},{:.1}) size={:.1}x{:.1} fill={:?} stroke={:?}",
                        path.id,
                        b.left, b.top, b.right, b.bottom,
                        b.right - b.left, b.bottom - b.top,
                        path.fill_color, path.stroke_color
                    )));
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
                if let Some(img_js) = image_provider
                    .get(&JsValue::from_str(&image.id))
                    .dyn_into::<HtmlImageElement>()
                    .ok()
                {
                    self.ctx.save();
                    let _ = self.ctx.draw_image_with_html_image_element_and_dw_and_dh(
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
        dbg_event("canvas.render", "render_page.start", vec![
            dbg_field("width", plan.width),
            dbg_field("height", plan.height),
            dbg_field("zoom", state.zoom),
            dbg_field("viewport", format!("{:?}", viewport_bbox)),
            dbg_field("has_vector_model", state.vector_model.is_some()),
        ]);
        self.prepare_page_surface(state, plan.width, plan.height);
        let overlays = collect_paragraph_render_overlays(plan, state.vector_model.as_ref());

        if let Some(vector_model) = &state.vector_model {
            let effective_plan = build_effective_vector_render_plan(
                vector_model,
                prepared_scene,
                &viewport_bbox,
                &overlays,
            );
            dbg_event("canvas.render", "effective_plan.ready", vec![
                dbg_field("entry_count", effective_plan.len()),
            ]);

            for entry in effective_plan {
                match entry {
                    EffectiveVectorRenderEntry::Object {
                        object_index,
                        suppressed_text_runs,
                    } => {
                        let Some(obj) = vector_model.objects.get(object_index) else {
                            continue;
                        };
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
                        if bbox_intersects(&overlay_cull_bbox, &viewport_bbox) {
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

fn draw_text_run_core(
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


fn path_bbox_summary(path: &pdf_viewer_core::models::VectorPathObject) -> Option<(f32, f32)> {
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;

    for segment in &path.segments {
        for [x, y] in &segment.points {
            min_x = min_x.min(*x);
            min_y = min_y.min(*y);
            max_x = max_x.max(*x);
            max_y = max_y.max(*y);
        }
    }

    if min_x.is_finite() && min_y.is_finite() && max_x.is_finite() && max_y.is_finite() {
        Some(((max_x - min_x).max(0.0), (max_y - min_y).max(0.0)))
    } else {
        None
    }
}

fn summarize_overlay_render_plan(
    plan: &crate::editor::draft_layout::EditorDraftRenderPlan,
) -> String {
    plan.layout
        .lines
        .iter()
        .take(4)
        .enumerate()
        .map(|(line_index, line)| {
            let runs = line
                .runs
                .iter()
                .take(8)
                .enumerate()
                .map(|(run_index, run)| {
                    let first_origin = run.char_origins.first().copied().unwrap_or(f32::NAN);
                    let last_origin = run.char_origins.last().copied().unwrap_or(f32::NAN);
                    format!(
                        "r{run_index}('{}' x={:.2} origins={} first={:.2} last={:.2})",
                        truncate_debug_text(&run.text, 18),
                        run.origin_x,
                        run.char_origins.len(),
                        first_origin,
                        last_origin,
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "line{line_index}(base={:.2}, off={:.2}, width={:.2}, text='{}', {runs})",
                line.baseline_y,
                line.offset_x,
                line.width,
                truncate_debug_text(&line.text, 40),
            )
        })
        .collect::<Vec<_>>()
        .join(" || ")
}

fn count_overlay_underline_runs(
    plan: &crate::editor::draft_layout::EditorDraftRenderPlan,
) -> usize {
    plan.layout
        .lines
        .iter()
        .flat_map(|line| line.runs.iter())
        .filter(|run| run.style.is_underline)
        .count()
}

fn draw_editor_marker_page(
    renderer: &CanvasRenderer,
    active_target: &ActiveEditorTarget,
    marker_text_override: Option<&str>,
) {
    let synthetic_marker_text = marker_text_override
        .filter(|text| active_target.scene.document_plan.marker.is_none() && !text.is_empty());
    if let Some(marker) = &active_target.scene.document_plan.marker {
        if let Some(override_text) = marker_text_override {
            if !override_text.is_empty() {
                if let Some(run) = marker.runs.first() {
                    draw_text_run_core(
                        &renderer.ctx,
                        renderer.dpr,
                        override_text,
                        run.origin_x,
                        run.origin_y,
                        run.style.font_size,
                        &run.style.color,
                        &run.style.font_name,
                        if run.style.is_bold { "bold" } else { "normal" },
                        if run.style.is_italic {
                            "italic"
                        } else {
                            "normal"
                        },
                        false,
                        run.style.scale_x,
                        0,
                        None,
                        CoordinateMode::PageSpace,
                    );
                }
            }
        } else {
            for run in &marker.runs {
                draw_text_run_core(
                    &renderer.ctx,
                    renderer.dpr,
                    &run.text,
                    run.origin_x,
                    run.origin_y,
                    run.style.font_size,
                    &run.style.color,
                    &run.style.font_name,
                    if run.style.is_bold { "bold" } else { "normal" },
                    if run.style.is_italic {
                        "italic"
                    } else {
                        "normal"
                    },
                    false,
                    run.style.scale_x,
                    0,
                    if run.char_origins.is_empty() {
                        None
                    } else {
                        Some(&run.char_origins)
                    },
                    CoordinateMode::PageSpace,
                );
            }
        }
    } else if let Some(override_text) = synthetic_marker_text {
        let run = active_target
            .scene
            .body_session
            .paragraph
            .runs
            .first()
            .unwrap_or(&active_target.scene.document_plan.draft_template_run);
        draw_text_run_core(
            &renderer.ctx,
            renderer.dpr,
            override_text,
            active_target.scene.body_session.anchor_bbox.left,
            run.origin_y,
            run.style.font_size,
            &run.style.color,
            &run.style.font_name,
            if run.style.is_bold { "bold" } else { "normal" },
            if run.style.is_italic {
                "italic"
            } else {
                "normal"
            },
            false,
            run.style.scale_x,
            0,
            None,
            CoordinateMode::PageSpace,
        );
    }
}

fn draw_active_editor_shell_overlay_page(
    renderer: &CanvasRenderer,
    overlay: &ParagraphRenderOverlay,
    marker_text_override: Option<&str>,
) {
    let active_target = &overlay.target;
    if overlay.replaces_source {
        draw_persisted_paragraph_overlay_page(
            renderer,
            active_target,
            &overlay.draft_text,
            marker_text_override,
            "active-editor-page-canvas",
        );
        return;
    }

    let shell_bbox = active_target.scene.shell_bbox;
    let replacement_region = paragraph_replacement_region(active_target);
    let occlusion_bbox = replacement_region.text_clear_bbox;
    let shell_width = (shell_bbox.right - shell_bbox.left).max(1.0);
    let shell_height = (shell_bbox.bottom - shell_bbox.top).max(1.0);
    let occlusion_width = (occlusion_bbox.right - occlusion_bbox.left).max(1.0);
    let occlusion_height = (occlusion_bbox.bottom - occlusion_bbox.top).max(1.0);
    dbg_event(
        "paint.overlay",
        "active-shell-caret-only",
        vec![
            dbg_field("paragraphId", &active_target.paragraph_id),
            dbg_field(
                "shellBBox",
                format!(
                    "[{:.2},{:.2},{:.2},{:.2}]",
                    shell_bbox.left, shell_bbox.top, shell_bbox.right, shell_bbox.bottom
                ),
            ),
            dbg_field(
                "bodyBBox",
                format!(
                    "[{:.2},{:.2},{:.2},{:.2}]",
                    active_target.scene.body_session.anchor_bbox.left,
                    active_target.scene.body_session.anchor_bbox.top,
                    active_target.scene.body_session.anchor_bbox.right,
                    active_target.scene.body_session.anchor_bbox.bottom
                ),
            ),
            dbg_field(
                "occlusionBBox",
                format!(
                    "[{:.2},{:.2},{:.2},{:.2}]",
                    occlusion_bbox.left,
                    occlusion_bbox.top,
                    occlusion_bbox.right,
                    occlusion_bbox.bottom
                ),
            ),
            dbg_field("width", shell_width),
            dbg_field("height", shell_height),
            dbg_field("occlusionWidth", occlusion_width),
            dbg_field("occlusionHeight", occlusion_height),
            dbg_field("markerTextOverride", marker_text_override.unwrap_or("none")),
            dbg_field("fillsPageCanvas", false),
            dbg_field("redrawsMarker", false),
        ],
    );
    let _ = renderer;
}

fn draw_persisted_paragraph_overlay_page(
    renderer: &CanvasRenderer,
    active_target: &ActiveEditorTarget,
    draft_text: &str,
    marker_text_override: Option<&str>,
    owner_label: &str,
) {
    let shell_bbox = active_target.scene.shell_bbox;
    let shell_width = (shell_bbox.right - shell_bbox.left).max(1.0);
    let shell_height = (shell_bbox.bottom - shell_bbox.top).max(1.0);
    let replacement_region = paragraph_replacement_region(active_target);
    let source_replacement_bbox = replacement_region.text_clear_bbox;
    let replacement_width = (source_replacement_bbox.right - source_replacement_bbox.left).max(1.0);
    let replacement_height =
        (source_replacement_bbox.bottom - source_replacement_bbox.top).max(1.0);
    dbg_event(
        "paint.overlay",
        "method.draw-editor-paragraph.enter",
        vec![
            dbg_field("paragraphId", &active_target.paragraph_id),
            dbg_field("markerTextOverride", marker_text_override.unwrap_or("none")),
            dbg_field("owner", owner_label),
        ],
    );
    renderer.ctx.save();
    renderer.ctx.set_fill_style_str("#ffffff");
    renderer.ctx.fill_rect(
        source_replacement_bbox.left as f64,
        source_replacement_bbox.top as f64,
        replacement_width as f64,
        replacement_height as f64,
    );
    renderer.ctx.restore();
    dbg_event(
        "paint.overlay",
        "method.draw-editor-paragraph.shell-occlusion",
        vec![
            dbg_field("paragraphId", &active_target.paragraph_id),
            dbg_field(
                "shellBBox",
                format!(
                    "[{:.2},{:.2},{:.2},{:.2}]",
                    shell_bbox.left, shell_bbox.top, shell_bbox.right, shell_bbox.bottom
                ),
            ),
            dbg_field("width", shell_width),
            dbg_field("height", shell_height),
            dbg_field(
                "sourceReplacementBBox",
                format!(
                    "[{:.2},{:.2},{:.2},{:.2}]",
                    source_replacement_bbox.left,
                    source_replacement_bbox.top,
                    source_replacement_bbox.right,
                    source_replacement_bbox.bottom
                ),
            ),
            dbg_field("sourceReplacementWidth", replacement_width),
            dbg_field("sourceReplacementHeight", replacement_height),
        ],
    );
    draw_editor_marker_page(renderer, active_target, marker_text_override);

    let document_plan = &active_target.scene.document_plan;
    let session = &document_plan.body_session;
    let render_plan =
        build_persisted_overlay_render_plan(document_plan, draft_text, |text, run| {
            measure_editor_layout_text_width_shared(&renderer.ctx, text, run)
        });

    dbg_event(
        "paint.overlay",
        "render-plan",
        vec![
            dbg_field("paragraphId", &active_target.paragraph_id),
            dbg_field("draftText", draft_text),
            dbg_field("sourceText", document_plan.source_body_text()),
            dbg_field(
                "bodyAnchor",
                format!(
                    "[{:.2},{:.2},{:.2},{:.2}]",
                    session.anchor_bbox.left,
                    session.anchor_bbox.top,
                    session.anchor_bbox.right,
                    session.anchor_bbox.bottom
                ),
            ),
            dbg_field("lineCount", render_plan.layout.lines.len()),
            dbg_field(
                "underlineRunCount",
                count_overlay_underline_runs(&render_plan),
            ),
            dbg_field(
                "lineSummary",
                summarize_overlay_render_plan(&render_plan),
            ),
        ],
    );

    for line in &render_plan.layout.lines {
        let baseline_y = session.anchor_bbox.top + line.baseline_y;
        for run in &line.runs {
            let run_x = session.anchor_bbox.left + line.offset_x + run.origin_x;
            renderer.draw_text_run(
                &run.text,
                run_x,
                baseline_y,
                run.style.font_size,
                &run.style.color,
                &run.style.font_name,
                if run.style.is_bold { "bold" } else { "normal" },
                if run.style.is_italic {
                    "italic"
                } else {
                    "normal"
                },
                false,
                run.style.scale_x,
                0,
                if run.char_origins.is_empty() {
                    None
                } else {
                    Some(&run.char_origins)
                },
            );
        }
    }
}
