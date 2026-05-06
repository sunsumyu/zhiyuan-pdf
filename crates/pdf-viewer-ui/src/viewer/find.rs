use std::cell::RefCell;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum HostFindScope {
    #[default]
    Page,
    Document,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HostFindSession {
    pub query: String,
    pub scope: HostFindScope,
    pub active_index: usize,
    pub total_matches: usize,
    pub match_pages: Vec<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HostFindNavigationResult {
    pub has_matches: bool,
    pub active_index: usize,
    pub active_page: Option<u16>,
    pub wrapped: bool,
}

thread_local! {
    pub static HOST_FIND_SESSION: RefCell<HostFindSession> =
        RefCell::new(HostFindSession::default());
}

pub fn clear_find_session() {
    HOST_FIND_SESSION.with(|session| {
        *session.borrow_mut() = HostFindSession::default();
    });
}

pub fn get_find_session() -> HostFindSession {
    HOST_FIND_SESSION.with(|session| session.borrow().clone())
}

pub fn set_find_session(
    query: String,
    scope: HostFindScope,
    match_pages: Vec<u16>,
    preferred_active_page: Option<u16>,
) -> HostFindNavigationResult {
    let total_matches = match_pages.len();
    let active_index = resolve_initial_active_index(&match_pages, preferred_active_page);
    let active_page = match_pages.get(active_index).copied();

    HOST_FIND_SESSION.with(|session| {
        *session.borrow_mut() = HostFindSession {
            query,
            scope,
            active_index,
            total_matches,
            match_pages,
        };
    });

    HostFindNavigationResult {
        has_matches: total_matches > 0,
        active_index,
        active_page,
        wrapped: false,
    }
}

pub fn move_find_match(step: i32) -> HostFindNavigationResult {
    HOST_FIND_SESSION.with(|session| {
        let mut session = session.borrow_mut();
        if session.match_pages.is_empty() {
            return HostFindNavigationResult::default();
        }

        let previous_index = session.active_index;
        let total = session.match_pages.len() as i32;
        let next_index = (session.active_index as i32 + step).rem_euclid(total) as usize;
        session.active_index = next_index;

        HostFindNavigationResult {
            has_matches: true,
            active_index: next_index,
            active_page: session.match_pages.get(next_index).copied(),
            wrapped: wrapped_between(previous_index, next_index, step),
        }
    })
}

fn resolve_initial_active_index(match_pages: &[u16], preferred_active_page: Option<u16>) -> usize {
    let Some(page) = preferred_active_page else {
        return 0;
    };
    match_pages
        .iter()
        .position(|candidate| *candidate == page)
        .unwrap_or(0)
}

fn wrapped_between(previous_index: usize, next_index: usize, step: i32) -> bool {
    if step > 0 {
        next_index < previous_index
    } else if step < 0 {
        next_index > previous_index
    } else {
        false
    }
}
