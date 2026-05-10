//! Comment / Review session host snapshot store.
//!
//! 该文件原位于 `viewer/review_store.rs`，但被 `comment/` 与 `review/` 两个域共用，
//! 强制它们 `use crate::viewer::review_store::*` 违反域自包含。已迁移至 review 域。
//!
//! 服务对象：
//! - `comment::comment_api::CommentManager` 读写评论目标快照
//! - `review::review_api::ReviewSession`（未来）读写审阅 panel 状态

use std::cell::RefCell;

// Re-export pure data structures from core.
pub use pdf_viewer_core::render::comment_review_state::*;

thread_local! {
    pub static HOST_COMMENT_REVIEW_SESSION: RefCell<HostCommentReviewSession> =
        RefCell::new(HostCommentReviewSession::default());
}

pub fn clear_comment_review_session() {
    replace_comment_review_session(HostCommentReviewSession::default());
}

pub fn get_comment_review_session() -> HostCommentReviewSession {
    HOST_COMMENT_REVIEW_SESSION.with(|session| session.borrow().clone())
}

fn replace_comment_review_session(
    next: HostCommentReviewSession,
) -> HostCommentReviewSession {
    HOST_COMMENT_REVIEW_SESSION.with(|session| {
        *session.borrow_mut() = next.clone();
    });
    next
}

fn update_comment_review_session(
    update: impl FnOnce(&mut HostCommentReviewSession),
) -> HostCommentReviewSession {
    let mut next = get_comment_review_session();
    update(&mut next);
    replace_comment_review_session(next)
}

pub fn set_comment_review_panel_open(panel_open: bool) -> HostCommentReviewSession {
    update_comment_review_session(|session| {
        session.panel_open = panel_open;
    })
}

pub fn toggle_comment_review_panel() -> HostCommentReviewSession {
    update_comment_review_session(|session| {
        session.panel_open = !session.panel_open;
    })
}

pub fn set_comment_review_scope(scope: HostCommentReviewScope) -> HostCommentReviewSession {
    update_comment_review_session(|session| {
        session.scope = scope;
    })
}

pub fn set_comment_review_query(query: String) -> HostCommentReviewSession {
    update_comment_review_session(|session| {
        session.query = query;
    })
}

pub fn select_comment_review_comment(
    selected_comment_id: Option<String>,
) -> HostCommentReviewSession {
    update_comment_review_session(|session| {
        session.selected_comment_id = selected_comment_id;
    })
}
