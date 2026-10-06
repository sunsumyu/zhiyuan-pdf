//! Full-page render orchestration: render_page and render_vector_slice.
//!
//! `render_page` draws a complete page (vector model or glyph plan), while
//! `render_vector_slice` time-slices progressive rendering of vector objects.

use super::CanvasRenderer;
use crate::editor::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use crate::editor::paragraph_overlay::{
    collect_paragraph_render_overlays, ParagraphRenderOverlayOwner,
};
use crate::render::canvas_overlay::{
    draw_active_editor_shell_overlay_page, draw_persisted_paragraph_overlay_page,
};
use crate::render::effective_page_plan::{
    build_effective_glyph_render_plan, build_effective_vector_render_plan,
    EffectiveGlyphRenderEntry, EffectiveVectorRenderEntry,
};
use crate::render::prepared_scene::PreparedPageScene;
use crate::render::progressive::ProgressiveVectorRenderTask;
use crate::viewport_culling::{glyph_run_intersects_viewport, resolve_page_viewport_bbox};
use pdf_viewer_core::models::PageState;

impl CanvasRenderer {
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
        let slice_start_time = budget_ms.map(|_| js_sys::Date::now());
        let mut processed_items = 0;

        while task.next_index < task.entries.len() && processed_items < max_items {
            if let (Some(start_time), Some(budget_ms)) = (slice_start_time, budget_ms) {
                if processed_items > 0 {
                    let now = js_sys::Date::now();
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
                                overlay,
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
                        let replacement_region =
                            crate::editor::replacement_region::paragraph_replacement_region(
                                &overlay.target,
                            );
                        let overlay_cull_bbox =
                            replacement_region.viewport_cull_bbox_for_page_width(plan.width);
                        let intersects = crate::common::bbox::bbox_intersects(
                            &overlay_cull_bbox,
                            &viewport_bbox,
                        );
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
                                state.vector_model.as_ref(),
                                image_provider,
                                overlay.marker_text_override.as_deref(),
                            );
                        }
                        ParagraphRenderOverlayOwner::PersistedPageCanvas => {
                            draw_persisted_paragraph_overlay_page(
                                self,
                                &overlay.target,
                                &overlay.draft_text,
                                state.vector_model.as_ref(),
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
                        super::draw::draw_text_run_core(
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
                            super::CoordinateMode::PageSpace,
                        );
                    }
                }
            }
        }
    }
}
