use crate::application::pdf::edit_commands::{
    apply_highlight_annotation, apply_text_comment, ensure_document_loaded,
};
use crate::application::pdf::page_context::build_page_region_context_from_vector_model;
use crate::infrastructure::pdf::annotation_store::{read_page_comments, read_page_highlights};
use crate::infrastructure::pdf::page_intermediate_service::PdfPageIntermediateService;
use crate::log_step;
use pdf_viewer_core::document::page_region_context::{BoundingBoxOutput, PageRegionContextOutput};
use serde::{Deserialize, Serialize};

pub use pdf_viewer_core::annotation::{
    PdfDeleteAnnotationRequest, PdfDeleteAnnotationResult, PdfPageAnnotationBox,
    PdfPageAnnotationTarget, PdfPageAnnotationTargetResult, PdfPageCommentItem, PdfPageCommentList,
    PdfRegionCommentRequest, PdfRegionCommentResult, PdfUpdateCommentRequest,
    PdfUpdateCommentResult,
};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PdfPageHighlightItem {
    pub id: String,
    pub page_index: u16,
    pub page_width: f32,
    pub page_height: f32,
    pub color: [f32; 3],
    pub box_rect: PdfPageAnnotationBox,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PdfPageHighlightList {
    pub page_index: u16,
    pub page_width: f32,
    pub page_height: f32,
    pub highlights: Vec<PdfPageHighlightItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfRegionHighlightRequest {
    pub page_index: u16,
    pub region_id: String,
    pub kind: String,
    #[serde(default = "default_highlight_color")]
    pub color: [f32; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PdfRegionHighlightResult {
    pub added: bool,
    pub page_index: u16,
    pub region_id: String,
}
fn default_highlight_color() -> [f32; 3] {
    [1.0, 0.92, 0.4]
}
pub(crate) fn collect_page_annotation_targets(
    page_context: &PageRegionContextOutput,
    page_index: u16,
    page_width: f32,
    page_height: f32,
) -> PdfPageAnnotationTargetResult {
    let mut targets = Vec::new();

    for region in &page_context.paragraph_regions {
        targets.push(PdfPageAnnotationTarget {
            id: region.id.clone(),
            kind: "paragraph-region".to_string(),
            page_index: region.page_index,
            page_width,
            page_height,
            label: summarize_label(&region.text),
            box_rect: from_region_box(&region.projection.region_box),
        });
    }

    for region in &page_context.list_item_regions {
        let label = region
            .body_text
            .clone()
            .unwrap_or_else(|| region.text.clone());
        targets.push(PdfPageAnnotationTarget {
            id: region.id.clone(),
            kind: "list-item-region".to_string(),
            page_index: region.page_index,
            page_width,
            page_height,
            label: summarize_label(&label),
            box_rect: from_region_box(&region.projection.region_box),
        });
    }

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
            targets.push(PdfPageAnnotationTarget {
                id: group.id.clone(),
                kind: "field-row".to_string(),
                page_index: row.page_index,
                page_width,
                page_height,
                label: summarize_label(&full_text),
                box_rect: from_region_box(&group.projection.text_box),
            });
        }
    }

    PdfPageAnnotationTargetResult {
        page_index,
        page_width,
        page_height,
        targets,
    }
}
pub(crate) async fn list_page_annotation_targets(
    app_state: &crate::AppState,
    path: &str,
    page_index: u16,
) -> Result<PdfPageAnnotationTargetResult, String> {
    let page_model = PdfPageIntermediateService::resolve_vector_page_model_from_app_state(
        app_state,
        path.to_string(),
        page_index,
        1.0,
        None,
    )
    .await?;
    let page_context = build_page_region_context_from_vector_model(&page_model);
    Ok(collect_page_annotation_targets(
        &page_context,
        page_index,
        page_model.width,
        page_model.height,
    ))
}
pub(crate) async fn list_page_highlights(
    app_state: &crate::AppState,
    path: &str,
    page_index: u16,
) -> Result<PdfPageHighlightList, String> {
    ensure_document_loaded(app_state, path).await?;
    let page_model = PdfPageIntermediateService::resolve_vector_page_model_from_app_state(
        app_state,
        path.to_string(),
        page_index,
        1.0,
        None,
    )
    .await?;
    let doc = {
        let docs = app_state.docs.pdf_documents.lock().unwrap();
        docs.get(path)
            .cloned()
            .ok_or_else(|| "Document not found in cache".to_string())?
    };
    let highlights =
        tokio::task::spawn_blocking(move || read_page_highlights(&doc, (page_index + 1) as u32))
            .await
            .map_err(|err| err.to_string())??;

    Ok(PdfPageHighlightList {
        page_index,
        page_width: page_model.width,
        page_height: page_model.height,
        highlights: highlights
            .into_iter()
            .map(|item| PdfPageHighlightItem {
                id: item.id,
                page_index,
                page_width: page_model.width,
                page_height: page_model.height,
                color: item.color,
                box_rect: PdfPageAnnotationBox {
                    left: item.rect[0],
                    top: item.rect[1],
                    width: item.rect[2],
                    height: item.rect[3],
                },
            })
            .collect(),
    })
}
pub(crate) async fn list_page_comments(
    app_state: &crate::AppState,
    path: &str,
    page_index: u16,
) -> Result<PdfPageCommentList, String> {
    ensure_document_loaded(app_state, path).await?;
    let page_model = PdfPageIntermediateService::resolve_vector_page_model_from_app_state(
        app_state,
        path.to_string(),
        page_index,
        1.0,
        None,
    )
    .await?;
    let doc = {
        let docs = app_state.docs.pdf_documents.lock().unwrap();
        docs.get(path)
            .cloned()
            .ok_or_else(|| "Document not found in cache".to_string())?
    };
    let comments =
        tokio::task::spawn_blocking(move || read_page_comments(&doc, (page_index + 1) as u32))
            .await
            .map_err(|err| err.to_string())??;

    Ok(PdfPageCommentList {
        page_index,
        page_width: page_model.width,
        page_height: page_model.height,
        comments: comments
            .into_iter()
            .map(|item| PdfPageCommentItem {
                id: item.id,
                page_index,
                page_width: page_model.width,
                page_height: page_model.height,
                color: item.color,
                contents: item.contents,
                box_rect: PdfPageAnnotationBox {
                    left: item.rect[0],
                    top: item.rect[1],
                    width: item.rect[2],
                    height: item.rect[3],
                },
            })
            .collect(),
    })
}
pub(crate) async fn add_region_highlight(
    app_state: &crate::AppState,
    path: &str,
    request: &PdfRegionHighlightRequest,
) -> Result<PdfRegionHighlightResult, String> {
    let page_model = PdfPageIntermediateService::resolve_vector_page_model_from_app_state(
        app_state,
        path.to_string(),
        request.page_index,
        1.0,
        None,
    )
    .await?;
    let page_context = build_page_region_context_from_vector_model(&page_model);
    let target_box = resolve_region_box(&page_context, &request.region_id, &request.kind)
        .ok_or_else(|| {
            format!(
                "Highlight target not found: page={} region={} kind={}",
                request.page_index, request.region_id, request.kind
            )
        })?;

    log_step!(
        "[PDF-ANNOT][add] page={} region={} kind={} rect=({}, {}, {}, {})",
        request.page_index,
        request.region_id,
        request.kind,
        target_box.left,
        target_box.top,
        target_box.width,
        target_box.height
    );

    apply_highlight_annotation(
        app_state,
        path.to_string(),
        request.page_index,
        [
            target_box.left,
            target_box.top,
            target_box.width,
            target_box.height,
        ],
        request.color,
    )
    .await?;

    Ok(PdfRegionHighlightResult {
        added: true,
        page_index: request.page_index,
        region_id: request.region_id.clone(),
    })
}
pub(crate) async fn add_region_comment(
    app_state: &crate::AppState,
    path: &str,
    request: &PdfRegionCommentRequest,
) -> Result<PdfRegionCommentResult, String> {
    let trimmed_contents = request.contents.trim();
    if trimmed_contents.is_empty() {
        return Err("Comment content cannot be empty".to_string());
    }

    let page_model = PdfPageIntermediateService::resolve_vector_page_model_from_app_state(
        app_state,
        path.to_string(),
        request.page_index,
        1.0,
        None,
    )
    .await?;
    let page_context = build_page_region_context_from_vector_model(&page_model);
    let target_box = resolve_region_box(&page_context, &request.region_id, &request.kind)
        .ok_or_else(|| {
            format!(
                "Comment target not found: page={} region={} kind={}",
                request.page_index, request.region_id, request.kind
            )
        })?;

    log_step!(
        "[PDF-COMMENT][add] page={} region={} kind={} rect=({}, {}, {}, {})",
        request.page_index,
        request.region_id,
        request.kind,
        target_box.left,
        target_box.top,
        target_box.width,
        target_box.height
    );

    apply_text_comment(
        app_state,
        path.to_string(),
        request.page_index,
        [
            target_box.left,
            target_box.top,
            target_box.width,
            target_box.height,
        ],
        request.color,
        trimmed_contents.to_string(),
    )
    .await?;

    Ok(PdfRegionCommentResult {
        added: true,
        page_index: request.page_index,
        region_id: request.region_id.clone(),
    })
}
pub(crate) async fn delete_page_annotation(
    app_state: &crate::AppState,
    path: &str,
    request: &PdfDeleteAnnotationRequest,
) -> Result<PdfDeleteAnnotationResult, String> {
    let annot_id = parse_annotation_object_id(&request.annotation_id)?;

    crate::application::pdf::edit_commands::delete_annotation_internal(
        app_state,
        path.to_string(),
        request.page_index,
        annot_id,
    )
    .await?;

    Ok(PdfDeleteAnnotationResult {
        deleted: true,
        page_index: request.page_index,
        annotation_id: request.annotation_id.clone(),
    })
}
pub(crate) async fn update_page_comment(
    app_state: &crate::AppState,
    path: &str,
    request: &PdfUpdateCommentRequest,
) -> Result<PdfUpdateCommentResult, String> {
    let trimmed_contents = request.contents.trim();
    if trimmed_contents.is_empty() {
        return Err("Comment content cannot be empty".to_string());
    }

    let annot_id = parse_annotation_object_id(&request.annotation_id)?;
    crate::application::pdf::edit_commands::update_text_comment(
        app_state,
        path.to_string(),
        request.page_index,
        annot_id,
        trimmed_contents.to_string(),
    )
    .await?;

    Ok(PdfUpdateCommentResult {
        updated: true,
        page_index: request.page_index,
        annotation_id: request.annotation_id.clone(),
    })
}
fn resolve_region_box(
    page_context: &PageRegionContextOutput,
    region_id: &str,
    kind: &str,
) -> Option<PdfPageAnnotationBox> {
    match kind {
        "paragraph-region" => page_context
            .paragraph_regions
            .iter()
            .find(|region| region.id == region_id)
            .map(|region| from_region_box(&region.projection.region_box)),
        "list-item-region" => page_context
            .list_item_regions
            .iter()
            .find(|region| region.id == region_id)
            .map(|region| from_region_box(&region.projection.region_box)),
        "field-row" => page_context
            .field_rows
            .iter()
            .flat_map(|row| row.groups.iter())
            .find(|group| group.id == region_id)
            .map(|group| from_region_box(&group.projection.text_box)),
        _ => None,
    }
}
fn from_region_box(region_box: &BoundingBoxOutput) -> PdfPageAnnotationBox {
    PdfPageAnnotationBox {
        left: region_box.left,
        top: region_box.top,
        width: region_box.width,
        height: region_box.height,
    }
}
fn summarize_label(text: &str) -> String {
    let mut chars = text.trim().chars();
    let mut out = String::new();
    for _ in 0..48 {
        if let Some(ch) = chars.next() {
            out.push(ch);
        } else {
            break;
        }
    }
    if chars.next().is_some() {
        out.push_str("...");
    }
    out
}
fn parse_annotation_object_id(value: &str) -> Result<(u32, u16), String> {
    let (object_part, generation_part) = value
        .split_once('-')
        .ok_or_else(|| format!("Invalid annotation id: {}", value))?;
    let object_id = object_part
        .parse::<u32>()
        .map_err(|err| format!("Invalid annotation object id {}: {}", value, err))?;
    let generation = generation_part
        .parse::<u16>()
        .map_err(|err| format!("Invalid annotation generation {}: {}", value, err))?;
    Ok((object_id, generation))
}
