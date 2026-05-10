use serde::{Deserialize, Serialize};
use std::cell::RefCell;

use crate::find::host_find_store::{
    clear_find_session, get_find_session, move_find_match, set_find_session, HostFindScope,
};

// ─── Types ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SearchMatch {
    pub id: String,
    pub kind: String,
    pub page_index: u16,
    pub page_width: f32,
    pub page_height: f32,
    pub line_index: usize,
    pub source_text: String,
    pub preview_text: String,
    pub matched_text: String,
    pub object_indices: Vec<usize>,
    pub box_rect: SearchBox,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SearchBox {
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub query: String,
    pub total_matches: usize,
    pub matches: Vec<SearchMatch>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum FindScope {
    #[default]
    Page,
    Document,
}

// ─── FindSessionState (Batch 2 sec 4) ────────────────────────────
//
// Explicit enum for the Find toolbar state machine, complementing the
// `SessionState` enum on `EditorSession`. Unlike EditorSession — which
// *stores* its state in a dedicated `Cell<SessionState>` because its
// transitions guard re-entrant saves — Find's state is fully
// **derivable** from already-stored data (`is_open` + `query` +
// `matches`). Storing a redundant copy would invite drift, so we
// compute the enum on demand via `derive()`.
//
// Semantics
//
//   Closed    toolbar not open (is_open == false)
//   Open      toolbar open, no active search (empty query)
//   Searching toolbar open, has query, zero matches (in-flight or no hits)
//   Active    toolbar open, has query, >= 1 match
//
// The UI can use the state to disable / gray out buttons, show empty
// state copy, etc. TS reads it via `FindSession::getState()`.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FindSessionState {
    Closed,
    Open,
    Searching,
    Active,
}

impl FindSessionState {
    pub fn as_str(&self) -> &'static str {
        match self {
            FindSessionState::Closed => "Closed",
            FindSessionState::Open => "Open",
            FindSessionState::Searching => "Searching",
            FindSessionState::Active => "Active",
        }
    }

    fn derive(is_open: bool, query: &str, total_matches: usize) -> Self {
        if !is_open {
            return FindSessionState::Closed;
        }
        if query.is_empty() {
            return FindSessionState::Open;
        }
        if total_matches == 0 {
            return FindSessionState::Searching;
        }
        FindSessionState::Active
    }
}

/// Snapshot of the current find session state, suitable for TS consumption.
pub fn get_find_state() -> FindSessionState {
    CONTROLLER.with(|c| {
        let ctrl = c.borrow();
        FindSessionState::derive(
            ctrl.is_open,
            &ctrl.last_result.query,
            ctrl.last_result.total_matches,
        )
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FindControllerState {
    pub is_open: bool,
    pub query: String,
    pub scope: FindScope,
    pub active_index: usize,
    pub total_matches: usize,
    pub matches: Vec<SearchMatch>,
    pub current_page: u16,
}

/// Result returned to TS after any state-mutating operation.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FindStateUpdate {
    pub state: FindControllerState,
    /// Matches on the current page with their global indices (for overlay rendering).
    pub current_page_matches: Vec<CurrentPageMatch>,
    /// If set, TS should navigate to this page.
    pub navigate_to_page: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CurrentPageMatch {
    pub global_index: usize,
    pub is_active: bool,
    pub is_editable: bool,
    pub box_rect: SearchBox,
    pub page_width: f32,
    pub page_height: f32,
    pub preview_text: String,
    pub id: String,
    pub kind: String,
    pub source_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReplaceRequest {
    pub page_index: u16,
    pub region_id: String,
    pub kind: String,
    pub original_text: String,
    pub query: String,
    pub replacement: String,
    pub replace_all_occurrences: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FindToolbarState {
    pub is_open: bool,
    pub count_text: String,
    pub has_matches: bool,
    pub can_replace_current: bool,
    pub can_replace_all: bool,
}

// ─── Controller State ─────────────────────────────────────────────────────────

thread_local! {
    static CONTROLLER: RefCell<FindControllerInner> = RefCell::new(FindControllerInner::default());
}

#[derive(Debug, Default)]
struct FindControllerInner {
    is_open: bool,
    last_result: SearchResult,
    current_page: u16,
    page_count: u16,
    path: String,
}

// ─── Public API ──────────────────────────────────────────────────────────────

pub fn open_find(current_page: u16, page_count: u16, path: String) -> FindStateUpdate {
    CONTROLLER.with(|c| {
        let mut ctrl = c.borrow_mut();
        ctrl.is_open = true;
        ctrl.current_page = current_page;
        ctrl.page_count = page_count;
        ctrl.path = path;
        build_state_update(&ctrl)
    })
}

pub fn close_find() -> FindStateUpdate {
    CONTROLLER.with(|c| {
        let mut ctrl = c.borrow_mut();
        ctrl.is_open = false;
        ctrl.last_result = SearchResult::default();
        clear_find_session();
        build_state_update(&ctrl)
    })
}

pub fn toggle_find(current_page: u16, page_count: u16, path: String) -> FindStateUpdate {
    let is_open = CONTROLLER.with(|c| c.borrow().is_open);
    if is_open {
        close_find()
    } else {
        open_find(current_page, page_count, path)
    }
}

pub fn set_search_result(result: SearchResult, scope: FindScope, current_page: u16) -> FindStateUpdate {
    CONTROLLER.with(|c| {
        let mut ctrl = c.borrow_mut();
        ctrl.current_page = current_page;
        ctrl.last_result = result;

        // Update session tracking
        let match_pages: Vec<u16> = ctrl.last_result.matches.iter().map(|m| m.page_index).collect();
        let host_scope = match scope {
            FindScope::Page => HostFindScope::Page,
            FindScope::Document => HostFindScope::Document,
        };
        set_find_session(
            ctrl.last_result.query.clone(),
            host_scope,
            match_pages,
            Some(current_page),
        );

        build_state_update(&ctrl)
    })
}

pub fn clear_search() -> FindStateUpdate {
    CONTROLLER.with(|c| {
        let mut ctrl = c.borrow_mut();
        ctrl.last_result = SearchResult::default();
        clear_find_session();
        build_state_update(&ctrl)
    })
}

pub fn move_active(step: i32) -> FindStateUpdate {
    let nav = move_find_match(step);
    CONTROLLER.with(|c| {
        let ctrl = c.borrow();
        let mut update = build_state_update(&ctrl);
        update.state.active_index = nav.active_index;

        // Check if navigation crosses page boundary
        if let Some(target_page) = nav.active_page {
            if target_page != ctrl.current_page {
                update.navigate_to_page = Some(target_page);
            }
        }

        // Rebuild current page matches with updated active index
        update.current_page_matches = build_current_page_matches(
            &ctrl.last_result.matches,
            ctrl.current_page,
            nav.active_index,
        );

        update
    })
}

pub fn set_current_page(page: u16) -> FindStateUpdate {
    CONTROLLER.with(|c| {
        let mut ctrl = c.borrow_mut();
        ctrl.current_page = page;
        build_state_update(&ctrl)
    })
}

pub fn get_toolbar_state() -> FindToolbarState {
    CONTROLLER.with(|c| {
        let ctrl = c.borrow();
        build_toolbar_state(&ctrl)
    })
}

pub fn get_replace_requests(replacement: &str, replace_all: bool, scope: FindScope) -> Vec<ReplaceRequest> {
    CONTROLLER.with(|c| {
        let ctrl = c.borrow();
        let session = get_find_session();
        let active_index = session.active_index;

        if replace_all {
            let matches_to_replace = match scope {
                FindScope::Page => ctrl.last_result.matches.iter()
                    .filter(|m| m.page_index == ctrl.current_page && is_editable_kind(&m.kind))
                    .collect::<Vec<_>>(),
                FindScope::Document => ctrl.last_result.matches.iter()
                    .filter(|m| is_editable_kind(&m.kind))
                    .collect::<Vec<_>>(),
            };
            matches_to_replace.into_iter().map(|m| ReplaceRequest {
                page_index: m.page_index,
                region_id: m.id.clone(),
                kind: m.kind.clone(),
                original_text: m.source_text.clone(),
                query: ctrl.last_result.query.clone(),
                replacement: replacement.to_string(),
                replace_all_occurrences: true,
            }).collect()
        } else {
            let active_match = ctrl.last_result.matches.get(active_index);
            match active_match {
                Some(m) if is_editable_kind(&m.kind) => vec![ReplaceRequest {
                    page_index: m.page_index,
                    region_id: m.id.clone(),
                    kind: m.kind.clone(),
                    original_text: m.source_text.clone(),
                    query: ctrl.last_result.query.clone(),
                    replacement: replacement.to_string(),
                    replace_all_occurrences: false,
                }],
                _ => vec![],
            }
        }
    })
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn is_editable_kind(kind: &str) -> bool {
    kind == "paragraph-region" || kind == "list-item-region"
}

fn build_state_update(ctrl: &FindControllerInner) -> FindStateUpdate {
    let session = get_find_session();
    let active_index = session.active_index;
    let current_page_matches = build_current_page_matches(
        &ctrl.last_result.matches,
        ctrl.current_page,
        active_index,
    );

    FindStateUpdate {
        state: FindControllerState {
            is_open: ctrl.is_open,
            query: ctrl.last_result.query.clone(),
            scope: if session.scope == HostFindScope::Document {
                FindScope::Document
            } else {
                FindScope::Page
            },
            active_index,
            total_matches: ctrl.last_result.total_matches,
            matches: ctrl.last_result.matches.clone(),
            current_page: ctrl.current_page,
        },
        current_page_matches,
        navigate_to_page: None,
    }
}

fn build_current_page_matches(
    all_matches: &[SearchMatch],
    current_page: u16,
    active_index: usize,
) -> Vec<CurrentPageMatch> {
    all_matches
        .iter()
        .enumerate()
        .filter(|(_, m)| m.page_index == current_page)
        .map(|(global_index, m)| CurrentPageMatch {
            global_index,
            is_active: global_index == active_index,
            is_editable: is_editable_kind(&m.kind),
            box_rect: m.box_rect.clone(),
            page_width: m.page_width,
            page_height: m.page_height,
            preview_text: m.preview_text.clone(),
            id: m.id.clone(),
            kind: m.kind.clone(),
            source_text: m.source_text.clone(),
        })
        .collect()
}

fn build_toolbar_state(ctrl: &FindControllerInner) -> FindToolbarState {
    let session = get_find_session();
    let active_index = session.active_index;
    let has_matches = ctrl.last_result.total_matches > 0;
    let scope = if session.scope == HostFindScope::Document {
        FindScope::Document
    } else {
        FindScope::Page
    };

    let active_match = ctrl.last_result.matches.get(active_index);
    let can_replace_current = match (scope, active_match) {
        (FindScope::Page, Some(m)) => m.page_index == ctrl.current_page && is_editable_kind(&m.kind),
        (FindScope::Document, Some(m)) => is_editable_kind(&m.kind),
        _ => false,
    };

    let can_replace_all = match scope {
        FindScope::Page => ctrl.last_result.matches.iter()
            .any(|m| m.page_index == ctrl.current_page && is_editable_kind(&m.kind)),
        FindScope::Document => ctrl.last_result.matches.iter()
            .any(|m| is_editable_kind(&m.kind)),
    };

    let count_text = if has_matches {
        format!("{} / {}", active_index + 1, ctrl.last_result.total_matches)
    } else {
        "0 / 0".to_string()
    };

    FindToolbarState {
        is_open: ctrl.is_open,
        count_text,
        has_matches,
        can_replace_current,
        can_replace_all,
    }
}
