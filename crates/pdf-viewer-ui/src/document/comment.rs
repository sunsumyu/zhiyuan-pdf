use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;

use crate::review::review_store::{
    read_comment_review_session, select_comment_review_comment, set_comment_review_panel_open,
    set_comment_review_query, set_comment_review_scope, toggle_comment_review_panel,
    HostCommentReviewScope, HostCommentReviewSession,
};
use crate::runtime::smart_invoke;

pub use pdf_viewer_core::annotation::{
    CommentBoxRect, CommentPercentFrame, PdfCommentOverlayDisplay, PdfCommentOverlayMarker,
    PdfCommentReviewCard, PdfCommentReviewCardAction, PdfCommentReviewPageSummary,
    PdfCommentReviewPanel, PdfCommentReviewRequest, PdfCommentReviewResult,
    PdfCommentReviewSummaryChip, PdfCommentTargetOverlayDisplay, PdfCommentTargetOverlayMarker,
    PdfDeleteAnnotationRequest, PdfDeleteAnnotationResult, PdfPageAnnotationTarget,
    PdfPageAnnotationTargetResult, PdfPageCommentItem, PdfPageCommentList, PdfRegionCommentRequest,
    PdfRegionCommentResult, PdfUpdateCommentRequest, PdfUpdateCommentResult,
};

pub type PdfCommentReviewDisplay =
    pdf_viewer_core::annotation::PdfCommentReviewDisplay<HostCommentReviewSession>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PathPageArgs {
    path: String,
    page_index: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PathRequestArgs<T> {
    path: String,
    request: T,
}

pub async fn list_page_comments(
    path: String,
    page_index: u16,
) -> Result<PdfPageCommentList, JsValue> {
    smart_invoke("read_comments", PathPageArgs { path, page_index }).await
}

pub async fn list_page_annotation_targets(
    path: String,
    page_index: u16,
) -> Result<PdfPageAnnotationTargetResult, JsValue> {
    smart_invoke("read_annotation_targets", PathPageArgs { path, page_index }).await
}

pub async fn review_document_comments(
    path: String,
    request: PdfCommentReviewRequest,
) -> Result<PdfCommentReviewResult, JsValue> {
    smart_invoke("read_comment_review", PathRequestArgs { path, request }).await
}

pub async fn load_comment_review(
    path: String,
    current_page: u16,
) -> Result<PdfCommentReviewDisplay, JsValue> {
    let session = read_comment_review_session();
    load_comment_review_from_session(path, current_page, session).await
}

pub async fn load_comment_overlay(
    path: String,
    current_page: u16,
) -> Result<PdfCommentOverlayDisplay, JsValue> {
    let session = read_comment_review_session();
    let comments = list_page_comments(path, current_page).await?;
    Ok(build_comment_overlay_display(&session, &comments.comments))
}

pub async fn load_comment_target_overlay(
    path: String,
    current_page: u16,
) -> Result<PdfCommentTargetOverlayDisplay, JsValue> {
    let targets = list_page_annotation_targets(path, current_page).await?;
    Ok(build_comment_target_overlay_display(&targets.targets))
}

pub async fn set_comment_review_panel_open_and_load(
    path: String,
    current_page: u16,
    panel_open: bool,
) -> Result<PdfCommentReviewDisplay, JsValue> {
    let session = set_comment_review_panel_open(panel_open);
    load_comment_review_from_session(path, current_page, session).await
}

pub async fn toggle_comment_review_panel_and_load(
    path: String,
    current_page: u16,
) -> Result<PdfCommentReviewDisplay, JsValue> {
    let session = toggle_comment_review_panel();
    load_comment_review_from_session(path, current_page, session).await
}

pub async fn set_comment_review_scope_and_load(
    path: String,
    current_page: u16,
    scope: HostCommentReviewScope,
) -> Result<PdfCommentReviewDisplay, JsValue> {
    let session = set_comment_review_scope(scope);
    load_comment_review_from_session(path, current_page, session).await
}

pub async fn set_comment_review_query_and_load(
    path: String,
    current_page: u16,
    query: String,
) -> Result<PdfCommentReviewDisplay, JsValue> {
    let session = set_comment_review_query(query);
    load_comment_review_from_session(path, current_page, session).await
}

pub async fn select_comment_review_and_load(
    path: String,
    current_page: u16,
    selected_comment_id: Option<String>,
) -> Result<PdfCommentReviewDisplay, JsValue> {
    let session = select_comment_review_comment(selected_comment_id);
    load_comment_review_from_session(path, current_page, session).await
}

async fn load_comment_review_from_session(
    path: String,
    current_page: u16,
    session: HostCommentReviewSession,
) -> Result<PdfCommentReviewDisplay, JsValue> {
    let page_index = match session.scope {
        HostCommentReviewScope::Document => None,
        HostCommentReviewScope::Page => Some(current_page),
    };
    let review = review_document_comments(
        path.clone(),
        PdfCommentReviewRequest {
            page_index,
            query: session.query.clone(),
        },
    )
    .await?;
    let comments = list_page_comments(path, current_page).await?;
    let panel = build_comment_review_panel(&session, &review, current_page);
    let overlay = build_comment_overlay_display(&session, &comments.comments);
    Ok(PdfCommentReviewDisplay {
        session,
        review,
        panel,
        overlay,
    })
}

fn build_comment_review_panel(
    session: &HostCommentReviewSession,
    review: &PdfCommentReviewResult,
    current_page: u16,
) -> PdfCommentReviewPanel {
    let scope_label = match session.scope {
        HostCommentReviewScope::Document => "document".to_string(),
        HostCommentReviewScope::Page => format!("page {}", current_page + 1),
    };
    let meta_text = format!(
        "{} · {} shown / {} total · {} page(s) with comments",
        scope_label, review.filtered_comments, review.total_comments, review.pages_with_comments
    );
    let summary_chips = review
        .summaries
        .iter()
        .filter(|summary| summary.total_comments > 0)
        .map(|summary| PdfCommentReviewSummaryChip {
            page_index: summary.page_index,
            label: format!(
                "P{} · {}/{}",
                summary.page_index + 1,
                summary.filtered_comments,
                summary.total_comments
            ),
        })
        .collect();
    let cards = review
        .comments
        .iter()
        .map(|comment| PdfCommentReviewCard {
            id: comment.id.clone(),
            page_index: comment.page_index,
            contents: comment.contents.clone(),
            page_label: format!("Page {}", comment.page_index + 1),
            location_label: format!(
                "x:{} y:{}",
                comment.box_rect.left.round(),
                comment.box_rect.top.round()
            ),
            helper_label: if comment.page_index == current_page {
                "Current page".to_string()
            } else {
                "Jump to page".to_string()
            },
            selected: session.selected_comment_id.as_deref() == Some(comment.id.as_str()),
            actions: build_comment_review_card_actions(),
        })
        .collect();
    PdfCommentReviewPanel {
        meta_text,
        empty: review.comments.is_empty(),
        summary_chips,
        cards,
    }
}

fn build_comment_review_card_actions() -> Vec<PdfCommentReviewCardAction> {
    vec![
        PdfCommentReviewCardAction {
            id: "jump".to_string(),
            label: "Jump".to_string(),
            tone: "primary".to_string(),
        },
        PdfCommentReviewCardAction {
            id: "edit".to_string(),
            label: "Edit".to_string(),
            tone: "success".to_string(),
        },
        PdfCommentReviewCardAction {
            id: "delete".to_string(),
            label: "Delete".to_string(),
            tone: "danger".to_string(),
        },
    ]
}

fn build_comment_overlay_display(
    session: &HostCommentReviewSession,
    comments: &[PdfPageCommentItem],
) -> PdfCommentOverlayDisplay {
    PdfCommentOverlayDisplay {
        comments: comments
            .iter()
            .map(|comment| PdfCommentOverlayMarker {
                id: comment.id.clone(),
                title: comment.contents.clone(),
                frame: build_percent_frame(
                    comment.page_width,
                    comment.page_height,
                    &comment.box_rect,
                    2.4,
                ),
                selected: session.selected_comment_id.as_deref() == Some(comment.id.as_str()),
            })
            .collect(),
    }
}

fn build_comment_target_overlay_display(
    targets: &[PdfPageAnnotationTarget],
) -> PdfCommentTargetOverlayDisplay {
    PdfCommentTargetOverlayDisplay {
        targets: targets
            .iter()
            .map(|target| PdfCommentTargetOverlayMarker {
                id: target.id.clone(),
                kind: target.kind.clone(),
                page_index: target.page_index,
                label: target.label.clone(),
                title: format!("添加批注：{}", target.label),
                frame: build_percent_frame(
                    target.page_width,
                    target.page_height,
                    &target.box_rect,
                    0.0,
                ),
            })
            .collect(),
    }
}

fn build_percent_frame(
    page_width: f32,
    page_height: f32,
    box_rect: &CommentBoxRect,
    min_percent: f32,
) -> CommentPercentFrame {
    let safe_page_width = page_width.max(1.0);
    let safe_page_height = page_height.max(1.0);
    CommentPercentFrame {
        left_percent: (box_rect.left / safe_page_width) * 100.0,
        top_percent: (box_rect.top / safe_page_height) * 100.0,
        width_percent: ((box_rect.width / safe_page_width) * 100.0).max(min_percent),
        height_percent: ((box_rect.height / safe_page_height) * 100.0).max(min_percent),
    }
}

pub async fn add_region_comment(
    path: String,
    request: PdfRegionCommentRequest,
) -> Result<PdfRegionCommentResult, JsValue> {
    smart_invoke("apply_comment", PathRequestArgs { path, request }).await
}

pub async fn delete_page_annotation(
    path: String,
    request: PdfDeleteAnnotationRequest,
) -> Result<PdfDeleteAnnotationResult, JsValue> {
    smart_invoke("delete_annotation", PathRequestArgs { path, request }).await
}

pub async fn update_page_comment(
    path: String,
    request: PdfUpdateCommentRequest,
) -> Result<PdfUpdateCommentResult, JsValue> {
    smart_invoke("apply_comment_update", PathRequestArgs { path, request }).await
}
