use std::cell::RefCell;

use pdf_viewer_core::models::{GlyphPaintPlan, PageState, VectorPageModel};

use crate::render::prepared_scene::PreparedPageScene;
use crate::render::progressive::ProgressiveVectorRenderTask;

thread_local! {
    pub static PAGE_STATE: RefCell<PageState> = RefCell::new(PageState::default());
    pub static PREPARED_SCENE: RefCell<Option<PreparedPageScene>> = const { RefCell::new(None) };
    pub static PROGRESSIVE_RENDER_TASK: RefCell<Option<ProgressiveVectorRenderTask>> = const { RefCell::new(None) };
}

pub type HostPageState = RefCell<PageState>;

pub fn with_page_state<R>(f: impl FnOnce(&PageState) -> R) -> R {
    PAGE_STATE.with(|state| f(&state.borrow()))
}

pub fn with_page_and_scene<R>(f: impl FnOnce(&PageState, &Option<PreparedPageScene>) -> R) -> R {
    PAGE_STATE.with(|state| PREPARED_SCENE.with(|scene| f(&state.borrow(), &scene.borrow())))
}

pub fn with_progressive_task_mut<R>(
    f: impl FnOnce(&mut Option<ProgressiveVectorRenderTask>) -> R,
) -> R {
    PROGRESSIVE_RENDER_TASK.with(|task| f(&mut task.borrow_mut()))
}

pub fn set_progressive_task(task: Option<ProgressiveVectorRenderTask>) {
    PROGRESSIVE_RENDER_TASK.with(|t| {
        *t.borrow_mut() = task;
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
    PAGE_STATE.with(|state| {
        let mut state = state.borrow_mut();
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
    });
    PREPARED_SCENE.with(|state| {
        *state.borrow_mut() = prepared_scene;
    });
    PROGRESSIVE_RENDER_TASK.with(|task| {
        *task.borrow_mut() = None;
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
    PAGE_STATE.with(|state| {
        let mut state = state.borrow_mut();
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
    PROGRESSIVE_RENDER_TASK.with(|task| {
        *task.borrow_mut() = None;
    });
}
