use wasm_bindgen::{JsCast, JsValue};

use crate::bridge::on_debug;
use crate::render::canvas::CanvasRenderer;
use crate::page::page_store::{with_page_state, with_page_and_scene, with_progressive_task_mut, set_progressive_task};
use crate::editor::paragraph_overlay::collect_paragraph_render_overlays;
use crate::render::progressive::{
    ProgressiveRenderStart, ProgressiveRenderStep, ProgressiveVectorRenderTask,
};
use crate::render::workflow::{
    progressive_start_result, progressive_step_result, ProgressiveRenderStartResult,
    ProgressiveRenderStepResult,
};
use crate::viewport_culling::resolve_page_viewport_bbox;

pub fn start_progressive_render() -> ProgressiveRenderStartResult {
    let start = with_page_and_scene(|state, prepared_scene| {
        let Some(vector_model) = state.vector_model.as_ref() else {
            return ProgressiveRenderStart::default();
        };
        let viewport_bbox =
            resolve_page_viewport_bbox(state, vector_model.width, vector_model.height);
        let Some(paint_plan) = state.paint_plan.as_ref() else {
            return ProgressiveRenderStart::default();
        };
        let overlays = collect_paragraph_render_overlays(paint_plan, Some(vector_model));
        crate::chain_trace!("progressive.start", "overlayCount" => overlays.len());
        let task = ProgressiveVectorRenderTask::build(
            vector_model,
            prepared_scene.as_ref(),
            viewport_bbox,
            &overlays,
        );
        if let Some(ref t) = task {
            let overlay_count = t.entries.iter()
                .filter(|e| matches!(e, crate::render::effective_page_plan::EffectiveVectorRenderEntry::ParagraphOverlay(_)))
                .count();
            crate::chain_trace!(
                "progressive.task",
                "totalItems" => t.total_items(),
                "overlayEntries" => overlay_count,
            );
        }
        set_progressive_task(task.clone());
        task.map(|task| ProgressiveRenderStart {
            started: true,
            total_items: task.total_items(),
        })
        .unwrap_or_default()
    });
    progressive_start_result(start)
}

pub fn step_progressive_render(
    canvas_id: String,
    image_cache: JsValue,
    budget_ms: f64,
    max_items: u32,
) -> ProgressiveRenderStepResult {
    let image_provider: js_sys::Map = image_cache.unchecked_into();
    let step = with_progressive_task_mut(|task_state| {
        let Some(task) = task_state.as_mut() else {
            return ProgressiveRenderStep {
                active: false,
                completed: true,
                processed_items: 0,
                remaining_items: 0,
            };
        };

        let clear_first = task.next_index == 0;
        let processed_before = task.next_index;
        let processed_items = with_page_state(|state| {
            let Some(vector_model) = state.vector_model.as_ref() else {
                return 0;
            };
            let Some(renderer) = CanvasRenderer::new_hijacked(&canvas_id) else {
                return 0;
            };
            renderer.render_vector_slice(
                state,
                vector_model,
                task,
                &image_provider,
                max_items.max(1) as usize,
                Some(budget_ms),
                clear_first,
            )
        });
        let processed_items = processed_items.max(task.next_index.saturating_sub(processed_before));
        let remaining_items = task.total_items().saturating_sub(task.next_index);
        let completed = task.is_complete();
        let step = ProgressiveRenderStep {
            active: true,
            completed,
            processed_items,
            remaining_items,
        };
        if completed {
            *task_state = None;
        }
        step
    });
    progressive_step_result(step)
}

pub fn cancel_progressive_render() {
    set_progressive_task(None);
}

pub fn render_page(canvas_id: String, image_cache: JsValue) {
    let image_provider: js_sys::Map = image_cache.unchecked_into();
    cancel_progressive_render();
    with_page_and_scene(|state, prepared_scene| {
        if let Some(renderer) = CanvasRenderer::new_hijacked(&canvas_id) {
            renderer.render_page(state, &image_provider, prepared_scene.as_ref());
            on_debug(
                "RENDER_PAGE".into(),
                format!("Executing Physics Render on {}", canvas_id),
            );
        } else {
            on_debug(
                "RENDER_PAGE".into(),
                format!("Canvas not found: {}", canvas_id),
            );
        }
    });
}
