use crate::models::PageState;

use crate::edit::bridge::{interaction_targets, ParagraphInteractionTarget};

pub fn resolve_region_target(
    page_state: &PageState,
    page_index: u16,
    region_id: &str,
    kind: &str,
    original_text: &str,
) -> Option<ParagraphInteractionTarget> {
    if !is_supported_region_kind(kind) {
        return None;
    }

    let plan = page_state.paint_plan.as_ref()?;
    let vector_model = page_state.vector_model.as_ref();
    let targets = interaction_targets(plan, vector_model);
    resolve_region_text_target(&targets, page_index, region_id, original_text)
}

pub fn is_supported_region_kind(kind: &str) -> bool {
    matches!(kind, "paragraph-region" | "list-item-region")
}

pub fn resolve_region_text_target(
    targets: &[ParagraphInteractionTarget],
    page_index: u16,
    region_id: &str,
    original_text: &str,
) -> Option<ParagraphInteractionTarget> {
    let original_key = normalize_target_text(original_text);
    let same_page = targets
        .iter()
        .filter(|target| target.page_index == page_index)
        .collect::<Vec<_>>();
    let region_matches = same_page
        .iter()
        .copied()
        .filter(|target| target.region_id == region_id)
        .collect::<Vec<_>>();

    if let Some(target) = region_matches
        .iter()
        .copied()
        .find(|target| normalize_target_text(&target.text) == original_key)
    {
        return Some(target.clone());
    }

    if region_matches.len() == 1 {
        return region_matches.first().map(|target| (*target).clone());
    }

    same_page
        .iter()
        .copied()
        .find(|target| normalize_target_text(&target.text) == original_key)
        .cloned()
}

fn normalize_target_text(value: &str) -> String {
    value
        .chars()
        .filter_map(|ch| {
            if ch.is_whitespace() {
                None
            } else if ch == '：' {
                Some(':')
            } else {
                Some(ch)
            }
        })
        .collect::<String>()
        .trim()
        .to_string()
}
