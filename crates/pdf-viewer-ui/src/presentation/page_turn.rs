use serde::{Deserialize, Serialize};
use std::cell::RefCell;

use crate::viewer::viewer_store::read_viewer_session;

const PREVIEW_PREFETCH_FORWARD_WINDOW_NORMAL: i16 = 3;
const PREVIEW_PREFETCH_FORWARD_WINDOW_FAST_FLIP: i16 = 8;
const PREVIEW_PREFETCH_REVERSE_WINDOW_NORMAL: i16 = 2;
const PREVIEW_PREFETCH_REVERSE_WINDOW_FAST_FLIP: i16 = 1;
const VECTOR_PREFETCH_WINDOW_NORMAL: i16 = 2;
/// fast-flip 模式下暂停 vector 预取，集中资源服务当前页 preview
const VECTOR_PREFETCH_WINDOW_FAST_FLIP: i16 = 0;
/// 两次翻页间隔低于此阈值（ms）时进入 fast-flip 模式
const FAST_FLIP_THRESHOLD_MS: f64 = 100.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PageTurnPhase {
    Idle,
    Turning,
    PreviewVisible,
    VectorVisible,
    DetailVisible,
    RasterVisible,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageTurnSnapshot {
    pub latest_page_turn_id: u32,
    pub latest_page_index: Option<u16>,
    pub previous_page_index: Option<u16>,
    pub visible_page_index: Option<u16>,
    pub visible_surface: String,
    pub direction: i8,
    pub reason: String,
    pub phase: PageTurnPhase,
    /// 最近一次翻页请求时刻（ms，由 TS performance.now() 传入）
    pub last_turn_at_ms: f64,
    /// 是否处于高速翻页模式（两次翻页间隔 < FAST_FLIP_THRESHOLD_MS）
    pub fast_flip_mode: bool,
}

impl Default for PageTurnSnapshot {
    fn default() -> Self {
        Self {
            latest_page_turn_id: 0,
            latest_page_index: None,
            previous_page_index: None,
            visible_page_index: None,
            visible_surface: "none".to_string(),
            direction: 0,
            reason: "idle".to_string(),
            phase: PageTurnPhase::Idle,
            last_turn_at_ms: 0.0,
            fast_flip_mode: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageTurnDecision {
    pub accepted: bool,
    pub page_turn_id: u32,
    pub target_page: u16,
    pub previous_page: u16,
    pub direction: i8,
    pub reason: String,
    pub reject_reason: Option<String>,
    pub snapshot: PageTurnSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageVisibleDecision {
    pub accepted: bool,
    pub page_turn_id: u32,
    pub page_index: u16,
    pub surface: String,
    pub can_prefetch: bool,
    pub reject_reason: Option<String>,
    pub snapshot: PageTurnSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageAssetAdmission {
    pub accepted: bool,
    pub page_index: u16,
    pub role: String,
    pub asset_kind: String,
    pub priority: u8,
    pub reject_reason: Option<String>,
    pub snapshot: PageTurnSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PagePrefetchTarget {
    pub page_index: u16,
    pub priority: u8,
    pub direction: i8,
    pub asset_kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PagePrefetchDecision {
    pub allowed: bool,
    pub anchor_page: u16,
    pub page_turn_id: u32,
    pub targets: Vec<PagePrefetchTarget>,
    pub reject_reason: Option<String>,
    pub snapshot: PageTurnSnapshot,
}

thread_local! {
    static PAGE_TURN_STATE: RefCell<PageTurnSnapshot> = RefCell::new(PageTurnSnapshot::default());
}

pub fn read_snapshot() -> PageTurnSnapshot {
    PAGE_TURN_STATE.with(|state| state.borrow().clone())
}

pub fn reset_state() {
    PAGE_TURN_STATE.with(|state| {
        *state.borrow_mut() = PageTurnSnapshot::default();
    });
}

pub fn request_page_turn(target_page: u16, reason: String, now_ms: f64) -> PageTurnDecision {
    let session = read_viewer_session();
    let current_page = session.current_page;
    let normalized_reason = normalize_reason(reason);

    if session.path.is_none() {
        return reject(target_page, current_page, normalized_reason, "noDocument");
    }

    if session.page_count == 0 {
        return reject(
            target_page,
            current_page,
            normalized_reason,
            "emptyDocument",
        );
    }

    if target_page >= session.page_count {
        return reject(
            target_page,
            current_page,
            normalized_reason,
            "pageOutOfRange",
        );
    }

    if target_page == current_page {
        return reject(target_page, current_page, normalized_reason, "samePage");
    }

    let direction = resolve_direction(current_page, target_page);
    let snapshot = PAGE_TURN_STATE.with(|state| {
        let mut state = state.borrow_mut();
        // fast-flip 检测：两次翻页间隔 < FAST_FLIP_THRESHOLD_MS 时进入高速模式
        let fast_flip_mode = if now_ms.is_finite()
            && state.last_turn_at_ms > 0.0
            && now_ms.is_sign_positive()
        {
            (now_ms - state.last_turn_at_ms) < FAST_FLIP_THRESHOLD_MS
        } else {
            false
        };
        state.latest_page_turn_id = state.latest_page_turn_id.wrapping_add(1).max(1);
        state.previous_page_index = Some(current_page);
        state.latest_page_index = Some(target_page);
        state.direction = direction;
        state.reason = normalized_reason.clone();
        state.phase = PageTurnPhase::Turning;
        state.fast_flip_mode = fast_flip_mode;
        if now_ms.is_finite() && now_ms.is_sign_positive() {
            state.last_turn_at_ms = now_ms;
        }
        state.clone()
    });

    let decision = PageTurnDecision {
        accepted: true,
        page_turn_id: snapshot.latest_page_turn_id,
        target_page,
        previous_page: current_page,
        direction,
        reason: normalized_reason,
        reject_reason: None,
        snapshot,
    };
    emit_decision(crate::events::event_names::PAGE_TURN_INTENT, &decision);
    decision
}

pub fn is_latest_turn(page_turn_id: u32, page_index: u16) -> bool {
    PAGE_TURN_STATE.with(|state| {
        let state = state.borrow();
        state.latest_page_turn_id == page_turn_id && state.latest_page_index == Some(page_index)
    })
}

pub fn mark_page_visible(page_index: u16, surface: String) -> PageVisibleDecision {
    let normalized_surface = normalize_surface(surface);
    let decision = PAGE_TURN_STATE.with(|state| {
        let mut state = state.borrow_mut();
        let accepted =
            state.latest_page_index.is_none() || state.latest_page_index == Some(page_index);

        if accepted {
            state.visible_page_index = Some(page_index);
            state.visible_surface = normalized_surface.clone();
            state.phase = visible_phase(&normalized_surface);
        }

        PageVisibleDecision {
            accepted,
            page_turn_id: state.latest_page_turn_id,
            page_index,
            surface: normalized_surface,
            can_prefetch: accepted && phase_allows_prefetch(state.phase),
            reject_reason: if accepted {
                None
            } else {
                Some("staleVisiblePage".to_string())
            },
            snapshot: state.clone(),
        }
    });

    emit_visible(&decision);
    decision
}

pub fn can_prefetch(page_index: u16) -> bool {
    PAGE_TURN_STATE.with(|state| {
        let state = state.borrow();
        phase_allows_prefetch(state.phase)
            && (state.visible_page_index == Some(page_index) || state.latest_page_index.is_none())
    })
}

pub fn admit_page_asset(page_index: u16, role: String, asset_kind: String) -> PageAssetAdmission {
    let normalized_role = normalize_role(role);
    let normalized_asset_kind = normalize_asset_kind(asset_kind);
    PAGE_TURN_STATE.with(|state| {
        let state = state.borrow();
        let session = read_viewer_session();
        let (accepted, priority, reject_reason) = match normalized_role.as_str() {
            "current" => {
                let is_current_session_page = session.current_page == page_index;
                let is_latest_intent = state.latest_page_index.is_none()
                    || state.latest_page_index == Some(page_index);
                if is_current_session_page && is_latest_intent {
                    (true, current_asset_priority(&normalized_asset_kind), None)
                } else {
                    (false, 0, Some("staleCurrentPage".to_string()))
                }
            }
            "prefetch" => {
                let anchor_page = state.latest_page_index
                    .or(state.visible_page_index)
                    .unwrap_or(session.current_page);
                let prefetch_distance = anchor_page.abs_diff(page_index);
                let fast_flip = state.fast_flip_mode;
                let prefetch_window = prefetch_window_for_asset(&normalized_asset_kind, fast_flip);
                let is_in_prefetch_window = prefetch_window > 0
                    && (1..=prefetch_window).contains(&prefetch_distance);
                if !phase_allows_prefetch(state.phase) {
                    (false, 0, Some("presentationBusy".to_string()))
                } else if state.visible_page_index != Some(anchor_page)
                    && state.latest_page_index != Some(anchor_page)
                {
                    (false, 0, Some("noVisibleAnchor".to_string()))
                } else if !is_in_prefetch_window {
                    (false, 0, Some("notInVisiblePagePrefetchWindow".to_string()))
                } else {
                    (
                        true,
                        prefetch_priority(anchor_page, page_index, state.direction),
                        None,
                    )
                }
            }
            _ => (false, 0, Some("unknownAssetRole".to_string())),
        };

        PageAssetAdmission {
            accepted,
            page_index,
            role: normalized_role,
            asset_kind: normalized_asset_kind,
            priority,
            reject_reason,
            snapshot: state.clone(),
        }
    })
}

pub fn decide_adjacent_prefetch(anchor_page: u16, page_count: u16) -> PagePrefetchDecision {
    PAGE_TURN_STATE.with(|state| {
        let state = state.borrow();
        if !phase_allows_prefetch(state.phase) {
            return reject_prefetch(anchor_page, "presentationBusy", &state);
        }
        if page_count == 0 {
            return reject_prefetch(anchor_page, "emptyDocument", &state);
        }
        if anchor_page >= page_count {
            return reject_prefetch(anchor_page, "pageOutOfRange", &state);
        }
        if state.visible_page_index != Some(anchor_page)
            && state.latest_page_index != Some(anchor_page)
        {
            return reject_prefetch(anchor_page, "stalePrefetchAnchor", &state);
        }

        let fast_flip = state.fast_flip_mode;
        let mut candidates = Vec::with_capacity(12);
        if state.direction >= 0 {
            push_prefetch_runway(&mut candidates, anchor_page, page_count, 1, state.direction, fast_flip);
        } else {
            push_prefetch_runway(
                &mut candidates,
                anchor_page,
                page_count,
                -1,
                state.direction,
                fast_flip,
            );
        }

        let targets = candidates;
        if targets.is_empty() {
            return reject_prefetch(anchor_page, "noAdjacentPage", &state);
        }

        PagePrefetchDecision {
            allowed: true,
            anchor_page,
            page_turn_id: state.latest_page_turn_id,
            targets,
            reject_reason: None,
            snapshot: state.clone(),
        }
    })
}

fn reject(
    target_page: u16,
    current_page: u16,
    reason: String,
    reject_reason: &str,
) -> PageTurnDecision {
    let snapshot = read_snapshot();
    let decision = PageTurnDecision {
        accepted: false,
        page_turn_id: snapshot.latest_page_turn_id,
        target_page,
        previous_page: current_page,
        direction: resolve_direction(current_page, target_page),
        reason,
        reject_reason: Some(reject_reason.to_string()),
        snapshot,
    };
    emit_decision(crate::events::event_names::PAGE_TURN_REJECT, &decision);
    decision
}

fn normalize_reason(reason: String) -> String {
    match reason.as_str() {
        "next" | "prev" | "jump" | "scroll" | "open" | "editCommit" => reason,
        _ => "unknown".to_string(),
    }
}

fn normalize_surface(surface: String) -> String {
    match surface.as_str() {
        "preview" | "vector" | "detail" | "raster" => surface,
        _ => "unknown".to_string(),
    }
}

fn normalize_role(role: String) -> String {
    match role.as_str() {
        "current" | "prefetch" => role,
        _ => "unknown".to_string(),
    }
}

fn normalize_asset_kind(asset_kind: String) -> String {
    match asset_kind.as_str() {
        "preview" | "displayList" | "vectorModel" | "paintPlan" | "baseBitmap" | "detailTile"
        | "editorOverlay" | "imageCache" => asset_kind,
        _ => "unknown".to_string(),
    }
}

fn visible_phase(surface: &str) -> PageTurnPhase {
    match surface {
        "preview" => PageTurnPhase::PreviewVisible,
        "vector" => PageTurnPhase::VectorVisible,
        "detail" => PageTurnPhase::DetailVisible,
        "raster" => PageTurnPhase::RasterVisible,
        _ => PageTurnPhase::Idle,
    }
}

fn phase_allows_prefetch(phase: PageTurnPhase) -> bool {
    matches!(
        phase,
        PageTurnPhase::Idle
            | PageTurnPhase::Turning
            | PageTurnPhase::PreviewVisible
            | PageTurnPhase::VectorVisible
            | PageTurnPhase::DetailVisible
            | PageTurnPhase::RasterVisible
    )
}

fn current_asset_priority(asset_kind: &str) -> u8 {
    match asset_kind {
        "preview" => 100,
        "vectorModel" | "paintPlan" | "baseBitmap" => 90,
        "editorOverlay" => 80,
        "detailTile" => 70,
        "imageCache" => 60,
        _ => 50,
    }
}

fn prefetch_priority(anchor_page: u16, page_index: u16, last_direction: i8) -> u8 {
    let direction = resolve_direction(anchor_page, page_index);
    if last_direction == 0 || direction == last_direction {
        30
    } else {
        10
    }
}

fn prefetch_window_for_asset(asset_kind: &str, fast_flip: bool) -> u16 {
    match asset_kind {
        "preview" => {
            if fast_flip {
                PREVIEW_PREFETCH_FORWARD_WINDOW_FAST_FLIP as u16
            } else {
                PREVIEW_PREFETCH_FORWARD_WINDOW_NORMAL as u16
            }
        }
        _ => {
            if fast_flip {
                VECTOR_PREFETCH_WINDOW_FAST_FLIP as u16
            } else {
                VECTOR_PREFETCH_WINDOW_NORMAL as u16
            }
        }
    }
}

fn push_prefetch_runway(
    targets: &mut Vec<PagePrefetchTarget>,
    anchor_page: u16,
    page_count: u16,
    forward_step: i16,
    last_direction: i8,
    fast_flip: bool,
) {
    // 顺方向 preview runway：fast-flip 下 8 页，normal 下 3 页
    let preview_forward_window = prefetch_window_for_asset("preview", fast_flip);
    for distance in 1..=preview_forward_window {
        push_prefetch_candidate(
            targets,
            anchor_page,
            page_count,
            forward_step * (distance as i16),
            last_direction,
            "preview",
        );
    }
    // vector 预取：fast-flip 下暂停，normal 下 2 页
    let vector_window = if fast_flip {
        VECTOR_PREFETCH_WINDOW_FAST_FLIP
    } else {
        VECTOR_PREFETCH_WINDOW_NORMAL
    };
    for distance in 1..=vector_window {
        push_prefetch_candidate(
            targets,
            anchor_page,
            page_count,
            forward_step * distance,
            last_direction,
            "vectorModel",
        );
    }
    // 逆方向 preview：fast-flip 下只取 1 页，normal 下 2 页
    let reverse_preview_window = if fast_flip {
        PREVIEW_PREFETCH_REVERSE_WINDOW_FAST_FLIP
    } else {
        PREVIEW_PREFETCH_REVERSE_WINDOW_NORMAL
    };
    let reverse_step = -forward_step;
    for distance in 1..=reverse_preview_window {
        push_prefetch_candidate(
            targets,
            anchor_page,
            page_count,
            reverse_step * distance,
            last_direction,
            "preview",
        );
    }
}

fn push_prefetch_candidate(
    targets: &mut Vec<PagePrefetchTarget>,
    anchor_page: u16,
    page_count: u16,
    offset: i16,
    last_direction: i8,
    asset_kind: &str,
) {
    let candidate = anchor_page as i32 + offset as i32;
    if candidate < 0 || candidate >= page_count as i32 {
        return;
    }
    let page_index = candidate as u16;
    targets.push(PagePrefetchTarget {
        page_index,
        priority: prefetch_priority(anchor_page, page_index, last_direction),
        direction: resolve_direction(anchor_page, page_index),
        asset_kind: asset_kind.to_string(),
    });
}

fn reject_prefetch(
    anchor_page: u16,
    reject_reason: &str,
    state: &PageTurnSnapshot,
) -> PagePrefetchDecision {
    PagePrefetchDecision {
        allowed: false,
        anchor_page,
        page_turn_id: state.latest_page_turn_id,
        targets: Vec::new(),
        reject_reason: Some(reject_reason.to_string()),
        snapshot: state.clone(),
    }
}

fn resolve_direction(current_page: u16, target_page: u16) -> i8 {
    if target_page > current_page {
        1
    } else if target_page < current_page {
        -1
    } else {
        0
    }
}

fn emit_decision(event: &str, decision: &PageTurnDecision) {
    #[cfg(target_arch = "wasm32")]
    if let Ok(payload) = serde_wasm_bindgen::to_value(decision) {
        crate::events::emit(event, &payload);
    }
}

fn emit_visible(decision: &PageVisibleDecision) {
    #[cfg(target_arch = "wasm32")]
    if let Ok(payload) = serde_wasm_bindgen::to_value(decision) {
        crate::events::emit(crate::events::event_names::PAGE_TURN_VISIBLE, &payload);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::viewer::viewer_store::{
        reset_viewer_session, set_current_page, set_viewer_document,
    };

    fn reset_with_document(current_page: u16, page_count: u16) {
        reset_state();
        reset_viewer_session();
        set_viewer_document(Some("fixture.pdf".to_string()), page_count, 1.0);
        set_current_page(current_page);
    }

    #[test]
    fn rejects_stale_page() {
        reset_with_document(2, 10);

        let decision = request_page_turn(3, "next".to_string(), 1000.0);

        assert!(decision.accepted);
        assert_eq!(decision.page_turn_id, 1);
        assert_eq!(decision.target_page, 3);
        assert_eq!(decision.previous_page, 2);
        assert_eq!(decision.direction, 1);
        assert!(is_latest_turn(decision.page_turn_id, 3));
        assert!(!is_latest_turn(decision.page_turn_id, 2));

        let stale_visible = mark_page_visible(2, "preview".to_string());
        assert!(!stale_visible.accepted);
        assert_eq!(
            stale_visible.reject_reason.as_deref(),
            Some("staleVisiblePage")
        );

        let visible = mark_page_visible(3, "preview".to_string());
        assert!(visible.accepted);
        assert!(visible.can_prefetch);
        assert_eq!(visible.snapshot.phase, PageTurnPhase::PreviewVisible);
    }

    #[test]
    fn prefers_turn_direction() {
        reset_with_document(4, 20);
        let decision = request_page_turn(5, "next".to_string(), 1000.0);
        assert!(decision.accepted);
        set_current_page(5);
        let visible = mark_page_visible(5, "vector".to_string());
        assert!(visible.accepted);

        let prefetch = decide_adjacent_prefetch(5, 20);

        assert!(prefetch.allowed);
        assert_eq!(prefetch.targets.len(), 12);
        assert_eq!(prefetch.targets[0].page_index, 6);
        assert_eq!(prefetch.targets[0].asset_kind, "preview");
        assert_eq!(prefetch.targets[0].priority, 30);
        assert_eq!(prefetch.targets[7].page_index, 13);
        assert_eq!(prefetch.targets[7].asset_kind, "preview");
        assert_eq!(prefetch.targets[8].page_index, 6);
        assert_eq!(prefetch.targets[8].asset_kind, "vectorModel");
        assert_eq!(prefetch.targets[9].page_index, 7);
        assert_eq!(prefetch.targets[9].asset_kind, "vectorModel");
        assert_eq!(prefetch.targets[10].page_index, 4);
        assert_eq!(prefetch.targets[10].asset_kind, "preview");
    }

    #[test]
    fn rejects_stale_assets() {
        reset_with_document(4, 10);
        let decision = request_page_turn(5, "next".to_string(), 1000.0);
        assert!(decision.accepted);
        set_current_page(5);
        assert!(mark_page_visible(5, "vector".to_string()).accepted);

        let current = admit_page_asset(5, "current".to_string(), "vectorModel".to_string());
        assert!(current.accepted);
        assert_eq!(current.priority, 90);

        let stale_current = admit_page_asset(4, "current".to_string(), "vectorModel".to_string());
        assert!(!stale_current.accepted);
        assert_eq!(
            stale_current.reject_reason.as_deref(),
            Some("staleCurrentPage")
        );

        let nearby_prefetch =
            admit_page_asset(6, "prefetch".to_string(), "vectorModel".to_string());
        assert!(nearby_prefetch.accepted);
        assert_eq!(nearby_prefetch.priority, 30);

        let distant_prefetch =
            admit_page_asset(8, "prefetch".to_string(), "vectorModel".to_string());
        assert!(!distant_prefetch.accepted);
        assert_eq!(
            distant_prefetch.reject_reason.as_deref(),
            Some("notInVisiblePagePrefetchWindow")
        );

        let distant_preview = admit_page_asset(8, "prefetch".to_string(), "preview".to_string());
        assert!(distant_preview.accepted);
    }

    #[test]
    fn rejects_invalid_turn() {
        reset_state();
        reset_viewer_session();
        let no_document = request_page_turn(1, "next".to_string(), 1000.0);
        assert!(!no_document.accepted);
        assert_eq!(no_document.reject_reason.as_deref(), Some("noDocument"));

        reset_with_document(0, 2);
        let out_of_range = request_page_turn(2, "jump".to_string(), 1000.0);
        assert!(!out_of_range.accepted);
        assert_eq!(
            out_of_range.reject_reason.as_deref(),
            Some("pageOutOfRange")
        );
    }

    #[test]
    fn activates_fast_flip() {
        reset_with_document(0, 20);
        // 第一次翻页，时刻 500ms
        let first = request_page_turn(1, "next".to_string(), 500.0);
        assert!(first.accepted);
        assert!(!first.snapshot.fast_flip_mode, "first turn should not be fast-flip");
        set_current_page(1);

        // 第二次翻页，间隔 50ms（< 100ms 阈值）→ fast-flip
        let fast = request_page_turn(2, "next".to_string(), 550.0);
        assert!(fast.accepted);
        assert!(fast.snapshot.fast_flip_mode, "rapid turn should activate fast-flip");
        set_current_page(2);

        // 第三次翻页，间隔 300ms（> 100ms 阈值）→ normal
        let normal = request_page_turn(3, "next".to_string(), 850.0);
        assert!(normal.accepted);
        assert!(!normal.snapshot.fast_flip_mode, "slow turn should deactivate fast-flip");
    }

    #[test]
    fn throttles_fast_flip() {
        reset_with_document(0, 30);
        // 快速两连翻进入 fast-flip
        let _ = request_page_turn(1, "next".to_string(), 500.0);
        set_current_page(1);
        let fast = request_page_turn(2, "next".to_string(), 540.0);
        assert!(fast.snapshot.fast_flip_mode);
        set_current_page(2);
        assert!(mark_page_visible(2, "preview".to_string()).accepted);

        let prefetch = decide_adjacent_prefetch(2, 30);
        assert!(prefetch.allowed);

        // fast-flip 下不应有 vectorModel 目标
        let vector_targets: Vec<_> = prefetch.targets.iter()
            .filter(|t| t.asset_kind == "vectorModel")
            .collect();
        assert!(vector_targets.is_empty(), "fast-flip should pause vector prefetch");

        // preview 顺方向仍有 8 页
        let fwd_preview: Vec<_> = prefetch.targets.iter()
            .filter(|t| t.asset_kind == "preview" && t.direction > 0)
            .collect();
        assert_eq!(fwd_preview.len(), 8, "fast-flip forward preview runway should still be 8");

        // 逆方向 preview 应只有 1 页（非 2 页）
        let rev_preview: Vec<_> = prefetch.targets.iter()
            .filter(|t| t.asset_kind == "preview" && t.direction < 0)
            .collect();
        assert_eq!(rev_preview.len(), 1, "fast-flip reverse preview should be limited to 1");
    }

    #[test]
    fn normal_mode_includes_vector_prefetch() {
        reset_with_document(4, 20);
        // 间隔充分长，保持 normal 模式
        let _ = request_page_turn(5, "next".to_string(), 0.0);
        set_current_page(5);
        let slow = request_page_turn(6, "next".to_string(), 5000.0);
        assert!(!slow.snapshot.fast_flip_mode);
        set_current_page(6);
        assert!(mark_page_visible(6, "vector".to_string()).accepted);

        let prefetch = decide_adjacent_prefetch(6, 20);
        assert!(prefetch.allowed);

        let vector_targets: Vec<_> = prefetch.targets.iter()
            .filter(|t| t.asset_kind == "vectorModel")
            .collect();
        assert!(!vector_targets.is_empty(), "normal mode should include vector prefetch");
    }
}
