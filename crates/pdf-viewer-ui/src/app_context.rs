//! AppContext — WASM UI 层的领域级状态仓库。
//!
//! **架构原则：状态按业务域拆分 RefCell。**
//! 不再使用单一 `RefCell<AppContext>` 包裹所有状态，避免 render/editor/viewer
//! 等不同业务域在同步调用链中相互重入时触发 `RefCell already borrowed`。

use std::cell::RefCell;

// 1. Zoom
use pdf_viewer_core::render::zoom_state::HostZoomState;
// 2. Viewer
use pdf_viewer_core::render::viewer_session::HostViewerSession;
// 3. Review
use pdf_viewer_core::render::comment_review_state::HostCommentReviewSession;
// 4. Render
use pdf_viewer_core::render::scheduler::HostRenderState;
use crate::render::platform_bridge::HostRenderLoopState;
// 5. Page
use pdf_viewer_core::models::PageState;
use crate::render::prepared_scene::PreparedPageScene;
use crate::render::progressive::ProgressiveVectorRenderTask;
// 6. Presentation / PageTurn
use crate::presentation::page_turn::PageTurnSnapshot;
use crate::render::tile_cache::{HostFrameCacheState, HostPresentState};
use crate::viewport_refresh::HostViewportRefreshState;
// 7. Find
use crate::find::find_store::FindControllerInner;
use pdf_viewer_core::render::find_state::HostFindSession;
// 8. Editor
use crate::editor::session::session::EditorModeState;
use crate::editor::platform_bridge::EditorHostRuntimeState;
use crate::editor::editor_types::SessionState;

pub struct EditorSessionStore {
    pub session_state: SessionState,
    pub active_block_id: Option<String>,
}

impl Default for EditorSessionStore {
    fn default() -> Self {
        Self {
            session_state: SessionState::Viewing,
            active_block_id: None,
        }
    }
}

/// 领域级状态仓库。
///
/// 每个业务域拥有独立 `RefCell`，因此 `render` 写入期间读取 `viewer`、`page`、
/// `zoom` 等不会被同一个全局 borrow root 阻塞。仍需遵守：持有某一领域的
/// mutable borrow 时，不要再次进入同一领域的 accessor。
pub struct AppStores {
    pub zoom: RefCell<HostZoomState>,
    pub viewer: RefCell<HostViewerSession>,
    pub review: RefCell<HostCommentReviewSession>,
    pub render: RefCell<HostRenderState<serde_json::Value>>,
    pub render_loop: RefCell<HostRenderLoopState>,
    pub page: RefCell<PageState>,
    pub prepared_scene: RefCell<Option<PreparedPageScene>>,
    pub progressive_task: RefCell<Option<ProgressiveVectorRenderTask>>,
    pub page_turn: RefCell<PageTurnSnapshot>,
    pub present: RefCell<HostPresentState>,
    pub frame_cache: RefCell<HostFrameCacheState>,
    pub viewport_refresh: RefCell<HostViewportRefreshState>,
    pub find_controller: RefCell<FindControllerInner>,
    pub find_session: RefCell<HostFindSession>,
    pub editor_mode: RefCell<EditorModeState>,
    pub editor_host: RefCell<EditorHostRuntimeState>,
    pub editor_session: RefCell<EditorSessionStore>,
    #[cfg(target_arch = "wasm32")]
    pub state_change_cb: RefCell<Option<js_sys::Function>>,
    #[cfg(target_arch = "wasm32")]
    pub change_cb: RefCell<Option<js_sys::Function>>,
}

impl Default for AppStores {
    fn default() -> Self {
        Self {
            zoom: RefCell::new(HostZoomState::default()),
            viewer: RefCell::new(HostViewerSession::default()),
            review: RefCell::new(HostCommentReviewSession::default()),
            render: RefCell::new(HostRenderState::default()),
            render_loop: RefCell::new(HostRenderLoopState::default()),
            page: RefCell::new(PageState::default()),
            prepared_scene: RefCell::new(None),
            progressive_task: RefCell::new(None),
            page_turn: RefCell::new(PageTurnSnapshot::default()),
            present: RefCell::new(HostPresentState::default()),
            frame_cache: RefCell::new(HostFrameCacheState::default()),
            viewport_refresh: RefCell::new(HostViewportRefreshState::default()),
            find_controller: RefCell::new(FindControllerInner::default()),
            find_session: RefCell::new(HostFindSession::default()),
            editor_mode: RefCell::new(EditorModeState::default()),
            editor_host: RefCell::new(EditorHostRuntimeState::default()),
            editor_session: RefCell::new(EditorSessionStore::default()),
            #[cfg(target_arch = "wasm32")]
            state_change_cb: RefCell::new(None),
            #[cfg(target_arch = "wasm32")]
            change_cb: RefCell::new(None),
        }
    }
}

thread_local! {
    pub static APP_STORES: AppStores = AppStores::default();
}

pub fn with_zoom<R>(f: impl FnOnce(&HostZoomState) -> R) -> R {
    APP_STORES.with(|stores| f(&stores.zoom.borrow()))
}

pub fn with_zoom_mut<R>(f: impl FnOnce(&mut HostZoomState) -> R) -> R {
    APP_STORES.with(|stores| f(&mut *stores.zoom.borrow_mut()))
}

pub fn with_viewer<R>(f: impl FnOnce(&HostViewerSession) -> R) -> R {
    APP_STORES.with(|stores| f(&stores.viewer.borrow()))
}

pub fn with_viewer_mut<R>(f: impl FnOnce(&mut HostViewerSession) -> R) -> R {
    APP_STORES.with(|stores| f(&mut *stores.viewer.borrow_mut()))
}

pub fn with_review<R>(f: impl FnOnce(&HostCommentReviewSession) -> R) -> R {
    APP_STORES.with(|stores| f(&stores.review.borrow()))
}

pub fn with_review_mut<R>(f: impl FnOnce(&mut HostCommentReviewSession) -> R) -> R {
    APP_STORES.with(|stores| f(&mut *stores.review.borrow_mut()))
}

pub fn with_render<R>(f: impl FnOnce(&HostRenderState<serde_json::Value>) -> R) -> R {
    APP_STORES.with(|stores| f(&stores.render.borrow()))
}

pub fn with_render_mut<R>(f: impl FnOnce(&mut HostRenderState<serde_json::Value>) -> R) -> R {
    APP_STORES.with(|stores| f(&mut *stores.render.borrow_mut()))
}

pub fn with_render_loop_mut<R>(f: impl FnOnce(&mut HostRenderLoopState) -> R) -> R {
    APP_STORES.with(|stores| f(&mut *stores.render_loop.borrow_mut()))
}

pub fn with_page<R>(f: impl FnOnce(&PageState) -> R) -> R {
    APP_STORES.with(|stores| f(&stores.page.borrow()))
}

pub fn with_page_mut<R>(f: impl FnOnce(&mut PageState) -> R) -> R {
    APP_STORES.with(|stores| f(&mut *stores.page.borrow_mut()))
}

pub fn with_page_and_scene<R>(
    f: impl FnOnce(&PageState, &Option<PreparedPageScene>) -> R,
) -> R {
    APP_STORES.with(|stores| {
        let page = stores.page.borrow();
        let scene = stores.prepared_scene.borrow();
        f(&*page, &*scene)
    })
}

pub fn with_progressive_task_mut<R>(
    f: impl FnOnce(&mut Option<ProgressiveVectorRenderTask>) -> R,
) -> R {
    APP_STORES.with(|stores| f(&mut *stores.progressive_task.borrow_mut()))
}

pub fn with_page_runtime_mut<R>(
    f: impl FnOnce(
        &mut PageState,
        &mut Option<PreparedPageScene>,
        &mut Option<ProgressiveVectorRenderTask>,
    ) -> R,
) -> R {
    APP_STORES.with(|stores| {
        let mut page = stores.page.borrow_mut();
        let mut scene = stores.prepared_scene.borrow_mut();
        let mut task = stores.progressive_task.borrow_mut();
        f(&mut *page, &mut *scene, &mut *task)
    })
}

pub fn with_page_turn<R>(f: impl FnOnce(&PageTurnSnapshot) -> R) -> R {
    APP_STORES.with(|stores| f(&stores.page_turn.borrow()))
}

pub fn with_page_turn_mut<R>(f: impl FnOnce(&mut PageTurnSnapshot) -> R) -> R {
    APP_STORES.with(|stores| f(&mut *stores.page_turn.borrow_mut()))
}

pub fn with_present<R>(f: impl FnOnce(&HostPresentState) -> R) -> R {
    APP_STORES.with(|stores| f(&stores.present.borrow()))
}

pub fn with_present_mut<R>(f: impl FnOnce(&mut HostPresentState) -> R) -> R {
    APP_STORES.with(|stores| f(&mut *stores.present.borrow_mut()))
}

pub fn with_frame_cache_mut<R>(f: impl FnOnce(&mut HostFrameCacheState) -> R) -> R {
    APP_STORES.with(|stores| f(&mut *stores.frame_cache.borrow_mut()))
}

pub fn with_viewport_refresh<R>(f: impl FnOnce(&HostViewportRefreshState) -> R) -> R {
    APP_STORES.with(|stores| f(&stores.viewport_refresh.borrow()))
}

pub fn with_present_runtime_mut<R>(
    f: impl FnOnce(
        &mut HostPresentState,
        &mut HostFrameCacheState,
        &mut HostViewportRefreshState,
    ) -> R,
) -> R {
    APP_STORES.with(|stores| {
        let mut present = stores.present.borrow_mut();
        let mut frame_cache = stores.frame_cache.borrow_mut();
        let mut viewport_refresh = stores.viewport_refresh.borrow_mut();
        f(&mut *present, &mut *frame_cache, &mut *viewport_refresh)
    })
}

pub fn with_present_and_viewport_refresh_mut<R>(
    f: impl FnOnce(&mut HostPresentState, &mut HostViewportRefreshState) -> R,
) -> R {
    APP_STORES.with(|stores| {
        let mut present = stores.present.borrow_mut();
        let mut viewport_refresh = stores.viewport_refresh.borrow_mut();
        f(&mut *present, &mut *viewport_refresh)
    })
}

pub fn with_find_controller<R>(f: impl FnOnce(&FindControllerInner) -> R) -> R {
    APP_STORES.with(|stores| f(&stores.find_controller.borrow()))
}

pub fn with_find_controller_mut<R>(f: impl FnOnce(&mut FindControllerInner) -> R) -> R {
    APP_STORES.with(|stores| f(&mut *stores.find_controller.borrow_mut()))
}

pub fn with_find_session<R>(f: impl FnOnce(&HostFindSession) -> R) -> R {
    APP_STORES.with(|stores| f(&stores.find_session.borrow()))
}

pub fn with_find_session_mut<R>(f: impl FnOnce(&mut HostFindSession) -> R) -> R {
    APP_STORES.with(|stores| f(&mut *stores.find_session.borrow_mut()))
}

pub fn with_editor_mode<R>(f: impl FnOnce(&EditorModeState) -> R) -> R {
    APP_STORES.with(|stores| f(&stores.editor_mode.borrow()))
}

pub fn with_editor_mode_mut<R>(f: impl FnOnce(&mut EditorModeState) -> R) -> R {
    APP_STORES.with(|stores| f(&mut *stores.editor_mode.borrow_mut()))
}

pub fn with_editor_host<R>(f: impl FnOnce(&EditorHostRuntimeState) -> R) -> R {
    APP_STORES.with(|stores| f(&stores.editor_host.borrow()))
}

pub fn with_editor_host_mut<R>(f: impl FnOnce(&mut EditorHostRuntimeState) -> R) -> R {
    APP_STORES.with(|stores| f(&mut *stores.editor_host.borrow_mut()))
}

pub fn with_editor_session<R>(f: impl FnOnce(&EditorSessionStore) -> R) -> R {
    APP_STORES.with(|stores| f(&stores.editor_session.borrow()))
}

pub fn with_editor_session_mut<R>(f: impl FnOnce(&mut EditorSessionStore) -> R) -> R {
    APP_STORES.with(|stores| f(&mut *stores.editor_session.borrow_mut()))
}

// ── editor callbacks (wasm32 only) ──────────────────────────────

#[cfg(target_arch = "wasm32")]
pub fn with_state_change_cb<R>(f: impl FnOnce(&Option<js_sys::Function>) -> R) -> R {
    APP_STORES.with(|stores| f(&stores.state_change_cb.borrow()))
}

#[cfg(target_arch = "wasm32")]
pub fn with_state_change_cb_mut<R>(f: impl FnOnce(&mut Option<js_sys::Function>) -> R) -> R {
    APP_STORES.with(|stores| f(&mut *stores.state_change_cb.borrow_mut()))
}

#[cfg(target_arch = "wasm32")]
pub fn with_change_cb<R>(f: impl FnOnce(&Option<js_sys::Function>) -> R) -> R {
    APP_STORES.with(|stores| f(&stores.change_cb.borrow()))
}

#[cfg(target_arch = "wasm32")]
pub fn with_change_cb_mut<R>(f: impl FnOnce(&mut Option<js_sys::Function>) -> R) -> R {
    APP_STORES.with(|stores| f(&mut *stores.change_cb.borrow_mut()))
}
