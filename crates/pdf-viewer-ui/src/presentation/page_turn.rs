use serde::{Deserialize, Serialize};
use std::cell::RefCell;

use crate::viewer::viewer_store::read_viewer_session;

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

pub fn read_page_turn_snapshot() -> PageTurnSnapshot {
    PAGE_TURN_STATE.with(|state| state.borrow().clone())
}

pub fn reset_page_turn_state() {
    PAGE_TURN_STATE.with(|state| {
        *state.borrow_mut() = PageTurnSnapshot::default();
    });
}

pub fn request_page_turn(target_page: u16, reason: String) -> PageTurnDecision {
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
        state.latest_page_turn_id = state.latest_page_turn_id.wrapping_add(1).max(1);
        state.previous_page_index = Some(current_page);
        state.latest_page_index = Some(target_page);
        state.direction = direction;
        state.reason = normalized_reason.clone();
        state.phase = PageTurnPhase::Turning;
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

pub fn is_latest_page_turn(page_turn_id: u32, page_index: u16) -> bool {
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
                let anchor_page = state.visible_page_index.unwrap_or(session.current_page);
                let prefetch_distance = anchor_page.abs_diff(page_index);
                let is_in_prefetch_window = (1..=2).contains(&prefetch_distance);
                if !phase_allows_prefetch(state.phase) {
                    (false, 0, Some("presentationBusy".to_string()))
                } else if state.visible_page_index != Some(anchor_page) {
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
        if state.visible_page_index != Some(anchor_page) {
            return reject_prefetch(anchor_page, "stalePrefetchAnchor", &state);
        }

        let mut candidates = Vec::with_capacity(4);
        if state.direction >= 0 {
            push_prefetch_candidate(&mut candidates, anchor_page, page_count, 1, state.direction);
            push_prefetch_candidate(&mut candidates, anchor_page, page_count, 2, state.direction);
            push_prefetch_candidate(
                &mut candidates,
                anchor_page,
                page_count,
                -1,
                state.direction,
            );
            push_prefetch_candidate(
                &mut candidates,
                anchor_page,
                page_count,
                -2,
                state.direction,
            );
        } else {
            push_prefetch_candidate(
                &mut candidates,
                anchor_page,
                page_count,
                -1,
                state.direction,
            );
            push_prefetch_candidate(
                &mut candidates,
                anchor_page,
                page_count,
                -2,
                state.direction,
            );
            push_prefetch_candidate(&mut candidates, anchor_page, page_count, 1, state.direction);
            push_prefetch_candidate(&mut candidates, anchor_page, page_count, 2, state.direction);
        }

        let targets = candidates.into_iter().take(2).collect::<Vec<_>>();
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
    let snapshot = read_page_turn_snapshot();
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

fn push_prefetch_candidate(
    targets: &mut Vec<PagePrefetchTarget>,
    anchor_page: u16,
    page_count: u16,
    offset: i16,
    last_direction: i8,
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
        asset_kind: "vectorModel".to_string(),
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
        reset_page_turn_state();
        reset_viewer_session();
        set_viewer_document(Some("fixture.pdf".to_string()), page_count, 1.0);
        set_current_page(current_page);
    }

    #[test]
    fn page_turn_tracks_latest_intent_and_rejects_stale_visible_page() {
        reset_with_document(2, 10);

        let decision = request_page_turn(3, "next".to_string());

        assert!(decision.accepted);
        assert_eq!(decision.page_turn_id, 1);
        assert_eq!(decision.target_page, 3);
        assert_eq!(decision.previous_page, 2);
        assert_eq!(decision.direction, 1);
        assert!(is_latest_page_turn(decision.page_turn_id, 3));
        assert!(!is_latest_page_turn(decision.page_turn_id, 2));

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
    fn prefetch_decision_prefers_turn_direction_and_limits_to_two_targets() {
        reset_with_document(4, 10);
        let decision = request_page_turn(5, "next".to_string());
        assert!(decision.accepted);
        set_current_page(5);
        let visible = mark_page_visible(5, "vector".to_string());
        assert!(visible.accepted);

        let prefetch = decide_adjacent_prefetch(5, 10);

        assert!(prefetch.allowed);
        assert_eq!(prefetch.targets.len(), 2);
        assert_eq!(prefetch.targets[0].page_index, 6);
        assert_eq!(prefetch.targets[0].priority, 30);
        assert_eq!(prefetch.targets[1].page_index, 7);
        assert_eq!(prefetch.targets[1].priority, 30);
    }

    #[test]
    fn asset_admission_rejects_stale_current_and_out_of_window_prefetch() {
        reset_with_document(4, 10);
        let decision = request_page_turn(5, "next".to_string());
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
    }

    #[test]
    fn page_turn_rejects_without_document_or_out_of_range_target() {
        reset_page_turn_state();
        reset_viewer_session();
        let no_document = request_page_turn(1, "next".to_string());
        assert!(!no_document.accepted);
        assert_eq!(no_document.reject_reason.as_deref(), Some("noDocument"));

        reset_with_document(0, 2);
        let out_of_range = request_page_turn(2, "jump".to_string());
        assert!(!out_of_range.accepted);
        assert_eq!(
            out_of_range.reject_reason.as_deref(),
            Some("pageOutOfRange")
        );
    }
}
