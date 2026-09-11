use crate::models::PageState;

use crate::edit::bridge::{collect_paragraph_interaction_targets, ParagraphInteractionTarget};

pub fn resolve_region_target_from_page_state(
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
    let targets = collect_paragraph_interaction_targets(plan, vector_model);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edit::bridge::ParagraphInteractionTarget;
    use crate::models::BoundingBox;

    // ─── is_supported_region_kind ────────────────────────────────────────

    #[test]
    fn is_supported_region_kind_paragraph() {
        assert!(is_supported_region_kind("paragraph-region"));
    }

    #[test]
    fn is_supported_region_kind_list_item() {
        assert!(is_supported_region_kind("list-item-region"));
    }

    #[test]
    fn is_supported_region_kind_unsupported() {
        assert!(!is_supported_region_kind("image-region"));
        assert!(!is_supported_region_kind(""));
        assert!(!is_supported_region_kind("paragraph"));
    }

    // ─── normalize_target_text ───────────────────────────────────────────

    #[test]
    fn normalize_target_text_removes_whitespace() {
        assert_eq!(normalize_target_text("hello world"), "helloworld");
        assert_eq!(normalize_target_text("  spaces  "), "spaces");
    }

    #[test]
    fn normalize_target_text_converts_colon() {
        assert_eq!(normalize_target_text("key：value"), "key:value");
    }

    #[test]
    fn normalize_target_text_trim() {
        assert_eq!(normalize_target_text("  hello  "), "hello");
    }

    #[test]
    fn normalize_target_text_empty() {
        assert_eq!(normalize_target_text(""), "");
        assert_eq!(normalize_target_text("   "), "");
    }

    // ─── resolve_region_text_target ──────────────────────────────────────

    fn test_target(
        paragraph_id: &str,
        region_id: &str,
        page_index: u16,
        text: &str,
    ) -> ParagraphInteractionTarget {
        ParagraphInteractionTarget {
            paragraph_id: paragraph_id.to_string(),
            region_id: region_id.to_string(),
            page_index,
            text: text.to_string(),
            target_indices: vec![],
            bbox: BoundingBox::default(),
            font_family: "Arial".to_string(),
            font_size: 12.0,
            font_weight: "normal".to_string(),
            font_style: "normal".to_string(),
            color: "#000000".to_string(),
            text_decoration: "none".to_string(),
        }
    }

    #[test]
    fn resolve_region_text_target_exact_match() {
        let targets = vec![
            test_target("p-1", "r-1", 0, "hello world"),
            test_target("p-2", "r-1", 0, "foo bar"),
        ];
        let result = resolve_region_text_target(&targets, 0, "r-1", "hello world");
        assert!(result.is_some());
        assert_eq!(result.unwrap().paragraph_id, "p-1");
    }

    #[test]
    fn resolve_region_text_target_whitespace_insensitive() {
        let targets = vec![test_target("p-1", "r-1", 0, "hello world")];
        // Should match even with different whitespace
        let result = resolve_region_text_target(&targets, 0, "r-1", "helloworld");
        assert!(result.is_some());
    }

    #[test]
    fn resolve_region_text_target_no_match() {
        let targets = vec![
            test_target("p-1", "r-1", 0, "hello"),
            test_target("p-2", "r-1", 0, "world"),
        ];
        // Multiple targets in region, text doesn't match any
        let result = resolve_region_text_target(&targets, 0, "r-1", "completely different");
        assert!(result.is_none());
    }

    #[test]
    fn resolve_region_text_target_single_in_region() {
        let targets = vec![
            test_target("p-1", "r-1", 0, "hello"),
            test_target("p-2", "r-2", 0, "world"),
        ];
        // Only one target in r-1, should return it even if text doesn't match exactly
        let result = resolve_region_text_target(&targets, 0, "r-1", "different");
        assert!(result.is_some());
        assert_eq!(result.unwrap().paragraph_id, "p-1");
    }
}
