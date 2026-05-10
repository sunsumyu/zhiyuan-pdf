use crate::infrastructure::pdf::models::VectorPageModel;
use pdf_viewer_core::document::page_region_context::{BoundingBoxOutput, PageRegionContextOutput};
use serde::{Deserialize, Serialize};
use super::page_context::build_page_region_context_from_vector_model;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PdfPageSearchRequest {
pub query: String,
    #[serde(default)]
pub case_sensitive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PdfPageSearchBox {
pub left: f32,
pub top: f32,
pub width: f32,
pub height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PdfPageSearchMatch {
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
pub box_rect: PdfPageSearchBox,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PdfPageSearchResult {
pub page_index: u16,
pub page_width: f32,
pub page_height: f32,
pub query: String,
pub total_matches: usize,
pub matches: Vec<PdfPageSearchMatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PdfDocumentSearchResult {
pub query: String,
pub total_matches: usize,
pub matches: Vec<PdfPageSearchMatch>,
}
pub(crate)
fn search_page_regions(
    page_model: &VectorPageModel,
    request: &PdfPageSearchRequest,
) -> PdfPageSearchResult {
    let query = request.query.trim().to_string();
    if query.is_empty() {
        return PdfPageSearchResult {
            page_index: page_model.page_index,
            page_width: page_model.width,
            page_height: page_model.height,
            query,
            total_matches: 0,
            matches: Vec::new(),
        };
    }

    let mut matches = search_page_matches(page_model, request);

    matches.sort_by(|left, right| {
        left.page_index
            .cmp(&right.page_index)
            .then(left.line_index.cmp(&right.line_index))
            .then(
                left.box_rect
                    .top
                    .partial_cmp(&right.box_rect.top)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
            .then(
                left.box_rect
                    .left
                    .partial_cmp(&right.box_rect.left)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
    });

    PdfPageSearchResult {
        page_index: page_model.page_index,
        page_width: page_model.width,
        page_height: page_model.height,
        query,
        total_matches: matches.len(),
        matches,
    }
}
pub(crate)
fn search_document_regions(
    page_models: &[VectorPageModel],
    request: &PdfPageSearchRequest,
) -> PdfDocumentSearchResult {
    let query = request.query.trim().to_string();
    if query.is_empty() {
        return PdfDocumentSearchResult {
            query,
            total_matches: 0,
            matches: Vec::new(),
        };
    }

    let mut matches = Vec::new();
    for page_model in page_models {
        matches.extend(search_page_matches(page_model, request));
    }

    matches.sort_by(|left, right| {
        left.page_index
            .cmp(&right.page_index)
            .then(left.line_index.cmp(&right.line_index))
            .then(
                left.box_rect
                    .top
                    .partial_cmp(&right.box_rect.top)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
            .then(
                left.box_rect
                    .left
                    .partial_cmp(&right.box_rect.left)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
    });

    PdfDocumentSearchResult {
        query,
        total_matches: matches.len(),
        matches,
    }
}
fn search_page_matches(
    page_model: &VectorPageModel,
    request: &PdfPageSearchRequest,
) -> Vec<PdfPageSearchMatch> {
    let query = request.query.trim();
    if query.is_empty() {
        return Vec::new();
    }
    let page_context = build_page_region_context_from_vector_model(page_model);
    let mut matches = Vec::new();

    collect_paragraph_matches(
        &page_context,
        page_model.width,
        page_model.height,
        query,
        request.case_sensitive,
        &mut matches,
    );
    collect_list_item_matches(
        &page_context,
        page_model.width,
        page_model.height,
        query,
        request.case_sensitive,
        &mut matches,
    );
    collect_field_row_matches(
        &page_context,
        page_model.width,
        page_model.height,
        query,
        request.case_sensitive,
        &mut matches,
    );

    matches
}
fn collect_paragraph_matches(
    page_context: &PageRegionContextOutput,
    page_width: f32,
    page_height: f32,
    query: &str,
    case_sensitive: bool,
    out: &mut Vec<PdfPageSearchMatch>,
) {
    for region in &page_context.paragraph_regions {
        if !contains_query(&region.text, query, case_sensitive) {
            continue;
        }
        out.push(PdfPageSearchMatch {
            id: region.id.clone(),
            kind: "paragraph-region".to_string(),
            page_index: region.page_index,
            page_width,
            page_height,
            line_index: region.line_index_start,
            source_text: region.text.clone(),
            preview_text: summarize_preview(&region.text),
            matched_text: query.to_string(),
            object_indices: region.object_indices.clone(),
            box_rect: from_region_box(&region.projection.region_box),
        });
    }
}
fn collect_list_item_matches(
    page_context: &PageRegionContextOutput,
    page_width: f32,
    page_height: f32,
    query: &str,
    case_sensitive: bool,
    out: &mut Vec<PdfPageSearchMatch>,
) {
    for region in &page_context.list_item_regions {
        let source_text = region.body_text.as_deref().unwrap_or(&region.text);
        if !contains_query(source_text, query, case_sensitive) {
            continue;
        }
        out.push(PdfPageSearchMatch {
            id: region.id.clone(),
            kind: "list-item-region".to_string(),
            page_index: region.page_index,
            page_width,
            page_height,
            line_index: region.line_index,
            source_text: source_text.to_string(),
            preview_text: summarize_preview(source_text),
            matched_text: query.to_string(),
            object_indices: region.object_indices.clone(),
            box_rect: from_region_box(&region.projection.region_box),
        });
    }
}
fn collect_field_row_matches(
    page_context: &PageRegionContextOutput,
    page_width: f32,
    page_height: f32,
    query: &str,
    case_sensitive: bool,
    out: &mut Vec<PdfPageSearchMatch>,
) {
    for row in &page_context.field_rows {
        for group in &row.groups {
            let full_text = if group.pair.value_text.trim().is_empty() {
                group.pair.key_text.clone()
            } else {
                format!(
                    "{} {}",
                    group.pair.key_text.trim(),
                    group.pair.value_text.trim()
                )
            };
            if !contains_query(&full_text, query, case_sensitive) {
                continue;
            }
            out.push(PdfPageSearchMatch {
                id: group.id.clone(),
                kind: "field-row".to_string(),
                page_index: row.page_index,
                page_width,
                page_height,
                line_index: row.line_index,
                source_text: full_text.clone(),
                preview_text: summarize_preview(&full_text),
                matched_text: query.to_string(),
                object_indices: group.object_indices.clone(),
                box_rect: from_region_box(&group.projection.text_box),
            });
        }
    }
}
fn contains_query(text: &str, query: &str, case_sensitive: bool) -> bool {
    if case_sensitive {
        text.contains(query)
    } else {
        text.to_lowercase().contains(&query.to_lowercase())
    }
}
fn summarize_preview(text: &str) -> String {
    let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let char_count = compact.chars().count();
    if char_count <= 96 {
        compact
    } else {
        compact.chars().take(96).collect::<String>() + "..."
    }
}
fn from_region_box(box_rect: &BoundingBoxOutput) -> PdfPageSearchBox {
    PdfPageSearchBox {
        left: box_rect.left,
        top: box_rect.top,
        width: box_rect.width,
        height: box_rect.height,
    }
}
