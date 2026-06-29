use std::collections::BTreeMap;

use pdf_viewer_core::models::{GlyphPaintParagraph, GlyphPaintPlan, PageState, VectorPageModel};
use pdf_viewer_core::text::list_semantics::{
    derive_list_text_semantics, format_numbering_marker, parse_numbering_value, ListMarkerKind,
};

use crate::editor::bridge::build_rich_patch;
use crate::editor::edit_target::get_base_paragraph_id;
use crate::editor::engine_state::LiveEditorParagraphState;
use crate::ui_state_store::{with_patch_state, GlobalPatchState};
use pdf_viewer_core::persistence::models::PersistableRegionPatch;

#[derive(Debug, Clone, Default)]
struct EffectiveListState {
    kind: ListMarkerKind,
    marker_text: String,
}

#[derive(Debug, Clone)]
struct ParagraphListContext<'a> {
    base_paragraph_id: String,
    effective: EffectiveListState,
    source_marker_text: Option<String>,
    _paragraph: &'a GlyphPaintParagraph,
}

pub fn resolve_marker_text(
    active_state: &LiveEditorParagraphState,
    page_state: &PageState,
) -> Option<String> {
    let overrides = collect_marker_overrides(page_state.paint_plan.as_ref(), Some(active_state));
    overrides
        .get(get_base_paragraph_id(active_state.paragraph_id()))
        .cloned()
        .flatten()
}

pub fn collect_marker_overrides(
    plan: Option<&GlyphPaintPlan>,
    active_state: Option<&LiveEditorParagraphState>,
) -> BTreeMap<String, Option<String>> {
    let Some(plan) = plan else {
        return BTreeMap::new();
    };
    let ordered_paragraphs = collect_ordered_page_paragraphs(plan);
    with_patch_state(|state| {
        let contexts = ordered_paragraphs
            .into_iter()
            .map(|paragraph| build_paragraph_list_context(state, paragraph, active_state))
            .collect::<Vec<_>>();
        build_numbering_override_map(&contexts)
    })
}

pub fn reconcile_numbering_patches(
    plan: &GlyphPaintPlan,
    vector_model: Option<&VectorPageModel>,
    patches: Vec<PersistableRegionPatch>,
) -> Vec<PersistableRegionPatch> {
    let overrides = collect_marker_overrides(Some(plan), None);
    let ordered_paragraphs = collect_ordered_page_paragraphs(plan);

    let mut paragraph_patches = Vec::<PersistableRegionPatch>::new();
    let mut auxiliary_patches = Vec::<PersistableRegionPatch>::new();
    let mut patch_index_by_base = BTreeMap::<String, usize>::new();

    for patch in patches {
        if matches!(
            patch.source.as_str(),
            "paragraph-region" | "list-item-region"
        ) {
            let base_paragraph_id = get_base_paragraph_id(&patch.region_id).to_string();
            patch_index_by_base.insert(base_paragraph_id, paragraph_patches.len());
            paragraph_patches.push(patch);
        } else {
            auxiliary_patches.push(patch);
        }
    }

    for paragraph in ordered_paragraphs {
        let base_paragraph_id = get_base_paragraph_id(&paragraph.id).to_string();
        let Some(desired_marker_text) = overrides.get(&base_paragraph_id).cloned().flatten() else {
            continue;
        };
        if derive_list_text_semantics(&desired_marker_text).kind != ListMarkerKind::Numbering {
            continue;
        }

        if let Some(existing_index) = patch_index_by_base.get(&base_paragraph_id).copied() {
            if let Some(existing_patch) = paragraph_patches.get_mut(existing_index) {
                if existing_patch.new_marker_text.as_deref() != Some(desired_marker_text.as_str()) {
                    existing_patch.new_marker_text = Some(desired_marker_text.clone());
                }
            }
            continue;
        }

        let source_text = paragraph
            .runs
            .iter()
            .map(|run| run.text.as_str())
            .collect::<String>();
        let source_semantics = derive_list_text_semantics(&source_text);
        if source_semantics.kind != ListMarkerKind::Numbering {
            continue;
        }

        let Some(mut derived_patch) = build_rich_patch(
            plan,
            vector_model,
            &paragraph.id,
            source_semantics.body_text,
            None,
        ) else {
            continue;
        };
        derived_patch.new_marker_text = Some(desired_marker_text);
        patch_index_by_base.insert(base_paragraph_id, paragraph_patches.len());
        paragraph_patches.push(derived_patch);
    }

    paragraph_patches.extend(auxiliary_patches);
    paragraph_patches
}

fn build_numbering_override_map(
    contexts: &[ParagraphListContext<'_>],
) -> BTreeMap<String, Option<String>> {
    let mut overrides = BTreeMap::new();
    let mut numbering_sequence: Option<usize> = None;
    let mut numbering_template: Option<String> = None;

    for context in contexts {
        let base_paragraph_id = context.base_paragraph_id.clone();
        match context.effective.kind {
            ListMarkerKind::Numbering => {
                let explicit_number = parse_numbering_value(&context.effective.marker_text);
                let next_value = match (numbering_sequence, explicit_number) {
                    (Some(previous), _) => previous.saturating_add(1),
                    (None, Some(explicit)) => explicit,
                    (None, None) => 1,
                };
                let template = context
                    .source_marker_text
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                    .map(|value| value.to_string())
                    .or_else(|| {
                        (!context.effective.marker_text.trim().is_empty())
                            .then(|| context.effective.marker_text.clone())
                    })
                    .or_else(|| numbering_template.clone());
                let marker_text = format_numbering_marker(next_value, template.as_deref());
                numbering_sequence = Some(next_value);
                numbering_template = Some(marker_text.clone());
                overrides.insert(base_paragraph_id, Some(marker_text));
            }
            ListMarkerKind::Bullet | ListMarkerKind::Symbol | ListMarkerKind::Custom => {
                numbering_sequence = None;
                numbering_template = None;
                let marker_text = resolve_symbolic_marker_text(
                    context.effective.marker_text.as_str(),
                    context.source_marker_text.as_deref(),
                );
                overrides.insert(base_paragraph_id, Some(marker_text));
            }
            ListMarkerKind::None => {
                numbering_sequence = None;
                numbering_template = None;
                overrides.insert(base_paragraph_id, None);
            }
        }
    }

    overrides
}

fn build_paragraph_list_context<'a>(
    state: &GlobalPatchState,
    paragraph: &'a GlyphPaintParagraph,
    active_state: Option<&LiveEditorParagraphState>,
) -> ParagraphListContext<'a> {
    let base_paragraph_id = get_base_paragraph_id(&paragraph.id).to_string();
    let active_marker = active_state
        .filter(|active| get_base_paragraph_id(active.paragraph_id()) == base_paragraph_id);

    let source_semantics = derive_list_text_semantics(
        &paragraph
            .runs
            .iter()
            .map(|run| run.text.as_str())
            .collect::<String>(),
    );
    let source_marker_text = source_semantics
        .has_marker
        .then_some(source_semantics.marker_text.clone());

    let effective = if let Some(active_state) = active_marker {
        EffectiveListState {
            kind: active_state.active_list_kind(),
            marker_text: active_state
                .target
                .scene
                .marker()
                .map(|marker| marker.text.clone())
                .unwrap_or_default(),
        }
    } else if let Some(patch) = resolve_patch_for_base_paragraph(state, &base_paragraph_id) {
        let marker_text = patch
            .new_marker_text
            .clone()
            .or_else(|| patch.marker_text.clone())
            .unwrap_or_default();
        let semantics = derive_list_text_semantics(&marker_text);
        EffectiveListState {
            kind: semantics.kind,
            marker_text,
        }
    } else {
        EffectiveListState {
            kind: source_semantics.kind,
            marker_text: source_semantics.marker_text,
        }
    };

    ParagraphListContext {
        base_paragraph_id,
        effective,
        source_marker_text,
        _paragraph: paragraph,
    }
}

fn resolve_symbolic_marker_text(
    effective_marker_text: &str,
    source_marker_text: Option<&str>,
) -> String {
    let effective = effective_marker_text.trim();
    if !effective.is_empty() {
        effective.to_string()
    } else {
        let source = source_marker_text.unwrap_or("").trim();
        if source.is_empty() {
            "•".to_string()
        } else {
            source.to_string()
        }
    }
}

fn collect_ordered_page_paragraphs(plan: &GlyphPaintPlan) -> Vec<&GlyphPaintParagraph> {
    let mut ordered = plan
        .regions
        .iter()
        .flat_map(|region| region.paragraphs.iter())
        .collect::<Vec<_>>();
    ordered.sort_by(|left, right| {
        left.bbox
            .top
            .total_cmp(&right.bbox.top)
            .then_with(|| left.bbox.left.total_cmp(&right.bbox.left))
            .then_with(|| left.id.cmp(&right.id))
    });
    ordered
}

fn resolve_patch_for_base_paragraph<'a>(
    state: &'a GlobalPatchState,
    base_paragraph_id: &str,
) -> Option<&'a pdf_viewer_core::persistence::models::PersistableRegionPatch> {
    state.paragraph_patches.get(base_paragraph_id).or_else(|| {
        state
            .paragraph_patches
            .values()
            .find(|patch| get_base_paragraph_id(&patch.region_id) == base_paragraph_id)
    })
}
