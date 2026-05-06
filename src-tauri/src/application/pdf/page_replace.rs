use serde::{Deserialize, Serialize};
use serde_json::Value;
use pdf_viewer_core::page_region_context::{
    ListItemRegionOutput, ParagraphRegionOutput, StyleRunSnapshot,
};
use pdf_viewer_core::persistence_models::PersistableRegionPatch;
use pdf_viewer_core::text::search_replace::{replace_query_matches, SearchReplaceOptions};
use crate::application::pdf::page_context::build_page_region_context_from_vector_model;
use crate::application::pdf::region_patch_service::apply_region_patch_batch;
use crate::infrastructure::multimedia::pdf::engine::PdfPageModelService;
use crate::interfaces::multimedia::pdf::ensure_document_loaded;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PdfDocumentReplaceRequest {
pub query: String,
pub replacement: String,
    #[serde(default)]
pub case_sensitive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PdfDocumentReplaceResult {
pub applied_count: usize,
pub skipped_count: usize,
pub touched_pages: Vec<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PdfRegionReplaceRequest {
pub page_index: u16,
pub region_id: String,
pub kind: String,
pub original_text: String,
pub query: String,
pub replacement: String,
    #[serde(default)]
pub case_sensitive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PdfRegionReplaceResult {
pub applied: bool,
pub page_index: u16,
}
pub(crate) async fn replace_document_regions(
    state: &crate::AppState,
    path: &str,
    page_count: usize,
    request: &PdfDocumentReplaceRequest,
) -> Result<PdfDocumentReplaceResult, String> {
    let query = normalize_replace_text(&request.query);
    if query.is_empty() {
        return Ok(PdfDocumentReplaceResult::default());
    }

    ensure_document_loaded(state, path).await?;

    let mut all_patches = Vec::new();
    let skipped_count = 0usize;

    for page_index in 0..page_count {
        let page_model = PdfPageModelService::get_vector_page_model_from_app_state(
            state,
            path.to_string(),
            page_index as u16,
            1.0,
        )
        .await?;
        let page_context = build_page_region_context_from_vector_model(&page_model);

        for region in &page_context.paragraph_regions {
            if let Some(next_text) = replace_query_matches(
                &region.text,
                &query,
                &request.replacement,
                SearchReplaceOptions {
                    case_sensitive: request.case_sensitive,
                    replace_all_occurrences: true,
                },
            ) {
                all_patches.push(build_paragraph_region_patch(region, &next_text)?);
            }
        }

        for region in &page_context.list_item_regions {
            let body_text = region.body_text.as_deref().unwrap_or(&region.text);
            if let Some(next_text) = replace_query_matches(
                body_text,
                &query,
                &request.replacement,
                SearchReplaceOptions {
                    case_sensitive: request.case_sensitive,
                    replace_all_occurrences: true,
                },
            ) {
                all_patches.push(build_list_item_region_patch(region, body_text, &next_text)?);
            }
        }
    }

    let apply_result = apply_region_patch_batch(state, path, all_patches).await?;

    Ok(PdfDocumentReplaceResult {
        applied_count: apply_result.applied_patch_count,
        skipped_count,
        touched_pages: apply_result.touched_pages,
    })
}
pub(crate) async fn replace_region_match(
    state: &crate::AppState,
    path: &str,
    request: &PdfRegionReplaceRequest,
) -> Result<PdfRegionReplaceResult, String> {
    ensure_document_loaded(state, path).await?;
    let page_model = PdfPageModelService::get_vector_page_model_from_app_state(
        state,
        path.to_string(),
        request.page_index,
        1.0,
    )
    .await?;
    let page_context = build_page_region_context_from_vector_model(&page_model);

    let patch = match request.kind.as_str() {
        "paragraph-region" => page_context
            .paragraph_regions
            .iter()
            .find(|region| region.id == request.region_id)
            .and_then(|region| {
                replace_query_matches(
                    &request.original_text,
                    &request.query,
                    &request.replacement,
                    SearchReplaceOptions {
                        case_sensitive: request.case_sensitive,
                        replace_all_occurrences: false,
                    },
                )
                .and_then(|next_text| build_paragraph_region_patch(region, &next_text).ok())
            }),
        "list-item-region" => page_context
            .list_item_regions
            .iter()
            .find(|region| region.id == request.region_id)
            .and_then(|region| {
                replace_query_matches(
                    &request.original_text,
                    &request.query,
                    &request.replacement,
                    SearchReplaceOptions {
                        case_sensitive: request.case_sensitive,
                        replace_all_occurrences: false,
                    },
                )
                .and_then(|next_text| {
                    build_list_item_region_patch(region, &request.original_text, &next_text).ok()
                })
            }),
        _ => None,
    };

    let Some(patch) = patch else {
        return Ok(PdfRegionReplaceResult {
            applied: false,
            page_index: request.page_index,
        });
    };

    apply_region_patch_batch(state, path, vec![patch]).await?;

    Ok(PdfRegionReplaceResult {
        applied: true,
        page_index: request.page_index,
    })
}
fn build_paragraph_region_patch(
    region: &ParagraphRegionOutput,
    replacement: &str,
) -> Result<PersistableRegionPatch, String> {
    Ok(PersistableRegionPatch {
        patch_key: region.id.clone(),
        page_index: region.page_index,
        region_id: region.id.clone(),
        original_text: region.text.clone(),
        new_text: normalize_replace_text(replacement),
        new_runs: None,
        source: "paragraph-region".to_string(),
        marker_text: None,
        new_marker_text: None,
        snapshot: Some(
            patch_paragraph_snapshot(region, replacement)
                .map_err(|err| format!("paragraph snapshot serialize failed: {}", err))?,
        ),
        kind: Some("paragraph".to_string()),
        pair_id: None,
        group_id: None,
        field_kind: None,
        field_name: None,
        original_value_text: None,
        new_value_text: None,
        target_indices: region.object_indices.clone(),
        full_target_indices: Vec::new(),
        displacement_y: None,
        wrap_width: Some(region.wrap_width),
        align: None,
        line_height: None,
        char_spacing: region.char_spacing,
        horizontal_scaling: region.scale_x,
    })
}
fn build_list_item_region_patch(
    region: &ListItemRegionOutput,
    original_text: &str,
    replacement: &str,
) -> Result<PersistableRegionPatch, String> {
    let body_object_indices = collect_object_indices_from_runs(&region.style_runs);
    let target_indices = if body_object_indices.is_empty() {
        region.object_indices.clone()
    } else {
        body_object_indices
    };

    Ok(PersistableRegionPatch {
        patch_key: region.id.clone(),
        page_index: region.page_index,
        region_id: region.id.clone(),
        original_text: normalize_replace_text(original_text),
        new_text: normalize_replace_text(replacement),
        new_runs: None,
        source: "list-item-region".to_string(),
        marker_text: region
            .marker_text
            .clone()
            .map(|value| normalize_replace_text(&value)),
        new_marker_text: None,
        snapshot: Some(
            patch_list_item_snapshot(region, replacement)
                .map_err(|err| format!("list-item snapshot serialize failed: {}", err))?,
        ),
        kind: Some("list-item".to_string()),
        pair_id: None,
        group_id: None,
        field_kind: None,
        field_name: None,
        original_value_text: None,
        new_value_text: None,
        target_indices,
        full_target_indices: region.object_indices.clone(),
        displacement_y: None,
        wrap_width: Some(region.wrap_width),
        align: None,
        line_height: None,
        char_spacing: region.char_spacing,
        horizontal_scaling: region.scale_x,
    })
}
fn patch_paragraph_snapshot(
    region: &ParagraphRegionOutput,
    replacement: &str,
) -> Result<Value, serde_json::Error> {
    let mut snapshot = serde_json::to_value(region)?;
    if let Some(object) = snapshot.as_object_mut() {
        object.insert(
            "text".to_string(),
            Value::String(normalize_replace_text(replacement)),
        );
        if let Some(lines) = object.get_mut("lines").and_then(Value::as_array_mut) {
            let line_texts = region
                .lines
                .iter()
                .map(|line| normalize_replace_text(&line.text))
                .collect::<Vec<_>>();
            let rebalanced = rebalance_text_across_lines(replacement, &line_texts);
            for (index, line) in lines.iter_mut().enumerate() {
                if let Some(line_object) = line.as_object_mut() {
                    let value = rebalanced.get(index).cloned().unwrap_or_default();
                    line_object.insert("renderedText".to_string(), Value::String(value.clone()));
                    line_object.insert("text".to_string(), Value::String(value));
                }
            }
        }
    }
    Ok(snapshot)
}
fn patch_list_item_snapshot(
    region: &ListItemRegionOutput,
    replacement: &str,
) -> Result<Value, serde_json::Error> {
    let mut snapshot = serde_json::to_value(region)?;
    if let Some(object) = snapshot.as_object_mut() {
        let marker = normalize_replace_text(region.marker_text.as_deref().unwrap_or_default());
        let body = normalize_replace_text(replacement);
        let combined = if marker.is_empty() {
            body.clone()
        } else if body.is_empty() {
            marker.clone()
        } else {
            format!("{}{}", marker, body)
        };
        object.insert("text".to_string(), Value::String(combined));
        if object.contains_key("bodyText") {
            object.insert("bodyText".to_string(), Value::String(body));
        }
    }
    Ok(snapshot)
}
fn normalize_replace_text(value: &str) -> String {
    value
        .replace("\r\n", "\n")
        .replace('\u{00a0}', " ")
        .trim()
        .to_string()
}
fn collect_object_indices_from_runs(runs: &[StyleRunSnapshot]) -> Vec<usize> {
    let mut indices = runs
        .iter()
        .flat_map(|run| run.object_indices.iter().copied())
        .collect::<Vec<_>>();
    indices.sort_unstable();
    indices.dedup();
    indices
}
fn rebalance_text_across_lines(next_text: &str, original_line_texts: &[String]) -> Vec<String> {
    let normalized = normalize_replace_text(next_text);
    if original_line_texts.len() <= 1 {
        return vec![normalized];
    }

    let explicit_lines = normalized
        .split('\n')
        .map(normalize_replace_text)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    if explicit_lines.len() == original_line_texts.len() {
        return explicit_lines;
    }

    let total_chars = normalized.chars().count();
    let total_reference = original_line_texts
        .iter()
        .map(|line| line.chars().count().max(1))
        .sum::<usize>()
        .max(1);
    let mut parts = Vec::new();
    let mut cursor = 0usize;

    for (index, line) in original_line_texts.iter().enumerate() {
        if index == original_line_texts.len() - 1 {
            parts.push(slice_chars(&normalized, cursor, total_chars));
            break;
        }
        let weight = line.chars().count().max(1);
        let remaining = total_chars.saturating_sub(cursor);
        let target_len = ((remaining as f32) * (weight as f32 / total_reference as f32))
            .round()
            .max(1.0) as usize;
        let tentative = (cursor + target_len).min(total_chars);
        let cut = find_boundary_near(&normalized, tentative);
        parts.push(slice_chars(&normalized, cursor, cut));
        cursor = cut;
    }

    parts
        .into_iter()
        .enumerate()
        .map(|(index, part)| {
            let normalized_part = normalize_replace_text(&part);
            if normalized_part.is_empty() {
                original_line_texts.get(index).cloned().unwrap_or_default()
            } else {
                normalized_part
            }
        })
        .collect()
}
fn find_boundary_near(text: &str, index: usize) -> usize {
    let chars = text.chars().collect::<Vec<_>>();
    for offset in 0..=24usize {
        let right = index + offset;
        if right < chars.len() && is_boundary_char(chars[right]) {
            return right + 1;
        }
        if index >= offset {
            let left = index - offset;
            if left > 0 && is_boundary_char(chars[left - 1]) {
                return left;
            }
        }
    }
    index.min(chars.len())
}
fn is_boundary_char(ch: char) -> bool {
    ch.is_whitespace() || matches!(
        ch,
        ',' | '.' | ';' | ':' | '!' | '?' | '，' | '。' | '；' | '：' | '！' | '？' | '、'
    )
}
fn slice_chars(text: &str, start: usize, end: usize) -> String {
    text.chars()
        .skip(start)
        .take(end.saturating_sub(start))
        .collect()
}
