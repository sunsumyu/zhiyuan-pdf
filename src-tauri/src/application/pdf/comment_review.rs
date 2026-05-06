use crate::application::pdf::page_annotation::{list_page_comments, PdfPageCommentItem};
use crate::interfaces::pdf::ensure_document_loaded;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PdfCommentReviewRequest {
pub page_index: Option<u16>,
    #[serde(default)]
pub query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PdfCommentReviewPageSummary {
pub page_index: u16,
pub total_comments: usize,
pub filtered_comments: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PdfCommentReviewResult {
pub total_comments: usize,
pub filtered_comments: usize,
pub pages_with_comments: usize,
pub summaries: Vec<PdfCommentReviewPageSummary>,
pub comments: Vec<PdfPageCommentItem>,
}
pub(crate) async fn review_document_comments(
    app_state: &crate::AppState,
    path: &str,
    request: &PdfCommentReviewRequest,
) -> Result<PdfCommentReviewResult, String> {
    ensure_document_loaded(app_state, path).await?;

    let page_count = {
        let docs = app_state.pdf_documents.lock().unwrap();
        let doc = docs
            .get(path)
            .cloned()
            .ok_or_else(|| "Document not found in cache".to_string())?;
        doc.get_pages().len() as u16
    };

    let target_pages: Vec<u16> = if let Some(page_index) = request.page_index {
        if page_index >= page_count {
            return Err(format!(
                "Comment review page index out of range: page={} total_pages={}",
                page_index, page_count
            ));
        }
        vec![page_index]
    } else {
        (0..page_count).collect()
    };

    let query = request.query.trim().to_lowercase();
    let has_query = !query.is_empty();
    let mut total_comments = 0usize;
    let mut filtered_comments = Vec::new();
    let mut summaries = Vec::new();

    for page_index in target_pages {
        let page_comments = list_page_comments(app_state, path, page_index).await?;
        let page_total = page_comments.comments.len();
        total_comments += page_total;

        let mut page_filtered = Vec::new();
        for comment in page_comments.comments {
            if has_query && !comment.contents.to_lowercase().contains(&query) {
                continue;
            }
            page_filtered.push(comment);
        }

        if page_total > 0 || !page_filtered.is_empty() {
            summaries.push(PdfCommentReviewPageSummary {
                page_index,
                total_comments: page_total,
                filtered_comments: page_filtered.len(),
            });
        }
        filtered_comments.extend(page_filtered);
    }

    Ok(PdfCommentReviewResult {
        total_comments,
        filtered_comments: filtered_comments.len(),
        pages_with_comments: summaries
            .iter()
            .filter(|item| item.total_comments > 0)
            .count(),
        summaries,
        comments: filtered_comments,
    })
}

#[cfg(test)]
mod tests {
use super::*;

    #[test]
fn review_request_defaults_to_document_scope() {
        let request = PdfCommentReviewRequest::default();
        assert_eq!(request.page_index, None);
        assert!(request.query.is_empty());
    }
}
