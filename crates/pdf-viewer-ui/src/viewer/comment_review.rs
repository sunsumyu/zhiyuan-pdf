use std::cell::RefCell;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum HostCommentReviewScope {
    #[default]
    Page,
    Document,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HostCommentReviewSession {
    pub panel_open: bool,
    pub scope: HostCommentReviewScope,
    pub query: String,
    pub selected_comment_id: Option<String>,
}

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
