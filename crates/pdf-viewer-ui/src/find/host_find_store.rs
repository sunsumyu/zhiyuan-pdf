//! Host-side find session snapshot store.
//!
//! 该文件原位于 `viewer/find_store.rs`（命名与 `find/find_store.rs` 冲突），
//! 已迁移至 find 域并重命名为 `host_find_store`，以恢复域自包含。
//!
//! 与 `find::find_store` 的区别：
//! - `find::find_store`：FindController 内部状态（搜索结果、替换队列等）
//! - `find::host_find_store`：宿主侧（viewer）读取的 find session snapshot
//!
//! ViewerSession 通过本模块读取当前 find 会话状态以驱动 UI（match counter / 上下匹配跳转）。

use std::cell::RefCell;

// Re-export pure data structures from core.
pub use pdf_viewer_core::render::find_state::*;

thread_local! {
    pub static FIND_SESSION: RefCell<HostFindSession> =
        RefCell::new(HostFindSession::default());
}

pub fn clear_find_session() {
    FIND_SESSION.with(|session| {
        *session.borrow_mut() = HostFindSession::default();
    });
}

pub fn get_find_session() -> HostFindSession {
    FIND_SESSION.with(|session| session.borrow().clone())
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

    FIND_SESSION.with(|session| {
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
    FIND_SESSION.with(|session| {
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
