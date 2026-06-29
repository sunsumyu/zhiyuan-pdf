use pdf_viewer_core::models::{GlyphPaintPlan, PageState, VectorPageModel};

use crate::render::prepared_scene::PreparedPageScene;
use crate::render::progressive::ProgressiveVectorRenderTask;

use crate::app_context;

pub struct PageSceneSnapshot {
    pub page: PageState,
    pub prepared_scene: Option<PreparedPageScene>,
}

pub fn with_page_state<R>(f: impl FnOnce(&PageState) -> R) -> R {
    app_context::with_page(f)
}

pub fn with_page_and_scene<R>(f: impl FnOnce(&PageState, &Option<PreparedPageScene>) -> R) -> R {
    app_context::with_page_and_scene(f)
}

pub fn with_page_scene_snapshot() -> PageSceneSnapshot {
    app_context::with_page_and_scene(|page, prepared_scene| PageSceneSnapshot {
        page: page.clone(),
        prepared_scene: prepared_scene.clone(),
    })
}

pub fn with_progressive_task_mut<R>(
    f: impl FnOnce(&mut Option<ProgressiveVectorRenderTask>) -> R,
) -> R {
    app_context::with_progressive_task_mut(f)
}

pub fn set_progressive_task(task: Option<ProgressiveVectorRenderTask>) {
    app_context::with_progressive_task_mut(|progressive_task| {
        *progressive_task = task;
    });
}

pub fn init_page_context(
    vector_model: VectorPageModel,
    paint_plan: GlyphPaintPlan,
    zoom: f32,
    dpr: f32,
    viewport_left: Option<f32>,
    viewport_top: Option<f32>,
    viewport_width: Option<f32>,
    viewport_height: Option<f32>,
) -> Option<(f32, f32)> {
    let prepared_scene = PreparedPageScene::build(Some(&vector_model), Some(&paint_plan));
    let page_width = vector_model.width;
    let page_height = vector_model.height;
    app_context::with_page_runtime_mut(|state, prepared_scene_slot, progressive_task| {
        state.zoom = zoom;
        state.dpr = dpr;
        state.viewport_left = viewport_left.unwrap_or_default().max(0.0);
        state.viewport_top = viewport_top.unwrap_or_default().max(0.0);
        state.viewport_width = viewport_width.unwrap_or(vector_model.width * zoom).max(1.0);
        state.viewport_height = viewport_height
            .unwrap_or(vector_model.height * zoom)
            .max(1.0);
        state.paint_plan = Some(paint_plan);
        state.vector_model = Some(vector_model);

        *prepared_scene_slot = prepared_scene;
        *progressive_task = None;
    });
    Some((page_width, page_height))
}

pub fn update_page_viewport(
    zoom: f32,
    dpr: f32,
    viewport_left: Option<f32>,
    viewport_top: Option<f32>,
    viewport_width: Option<f32>,
    viewport_height: Option<f32>,
) -> (f32, f32) {
    app_context::with_page_mut(|state| {
        let page_width = state
            .vector_model
            .as_ref()
            .map(|model| model.width)
            .or_else(|| state.paint_plan.as_ref().map(|plan| plan.width))
            .unwrap_or(595.0);
        let page_height = state
            .vector_model
            .as_ref()
            .map(|model| model.height)
            .or_else(|| state.paint_plan.as_ref().map(|plan| plan.height))
            .unwrap_or(842.0);
        state.zoom = zoom;
        state.dpr = dpr;
        state.viewport_left = viewport_left.unwrap_or_default().max(0.0);
        state.viewport_top = viewport_top.unwrap_or_default().max(0.0);
        state.viewport_width = viewport_width.unwrap_or(page_width * zoom).max(1.0);
        state.viewport_height = viewport_height.unwrap_or(page_height * zoom).max(1.0);
        (page_width, page_height)
    })
}

pub fn reset_progressive_render_task() {
    app_context::with_progressive_task_mut(|progressive_task| {
        *progressive_task = None;
    });
}
