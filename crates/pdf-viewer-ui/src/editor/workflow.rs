use pdf_viewer_core::models::PageState;
use wasm_bindgen::JsValue;

use pdf_viewer_core::edit::target_resolution::{is_supported_region_kind, resolve_region_target_from_page_state};
use pdf_viewer_core::edit::bridge::{
    build_active_editor_target, build_paragraph_patch as core_build_paragraph_patch,
    collect_paragraph_interaction_targets,
    resolve_paragraph_shell_bbox as bridge_resolve_paragraph_shell_bbox,
};
use crate::editor::session::{is_text_edit_enabled, ActiveEditorTarget};
use crate::models::PersistableRegionPatch;
use pdf_viewer_core::models::BoundingBox;

pub fn build_paragraph_interaction_targets(
    page_state: &PageState,
    editing_enabled: bool,
) -> JsValue {
    if !editing_enabled {
        return JsValue::NULL;
    }
    serde_wasm_bindgen::to_value(
        &page_state
            .paint_plan
            .as_ref()
            .map(|plan| {
                collect_paragraph_interaction_targets(plan, page_state.vector_model.as_ref())
            })
            .unwrap_or_default(),
    )
    .unwrap_or(JsValue::NULL)
}

pub fn open_paragraph_editor(
    page_state: &PageState,
    paragraph_id: &str,
    click_page_x: f32,
    click_page_y: f32,
    _visual_zoom: f32,
) -> Option<ActiveEditorTarget> {
    if !is_text_edit_enabled() {
        return None;
    }
    page_state.paint_plan.as_ref().and_then(|plan| {
        build_active_editor_target(
            plan,
            page_state.vector_model.as_ref(),
            paragraph_id,
            click_page_x,
            click_page_y,
        )
    })
}

pub fn resolve_paragraph_shell_bbox(
    page_state: &PageState,
    paragraph_id: &str,
) -> Option<BoundingBox> {
    page_state
        .paint_plan
        .as_ref()
        .and_then(|plan| bridge_resolve_paragraph_shell_bbox(plan, paragraph_id))
}

pub fn build_paragraph_patch(
    page_state: &PageState,
    paragraph_id: &str,
    new_text: String,
) -> Option<PersistableRegionPatch> {
    match (
        page_state.paint_plan.as_ref(),
        page_state.vector_model.as_ref(),
    ) {
        (Some(plan), vector_model) => {
            core_build_paragraph_patch(plan, vector_model, paragraph_id, new_text)
        }
        _ => None,
    }
}

pub fn build_region_text_patch(
    page_state: &PageState,
    page_index: u16,
    region_id: &str,
    kind: &str,
    original_text: &str,
    new_text: String,
) -> Option<PersistableRegionPatch> {
    if !is_supported_region_kind(kind) {
        return None;
    }
    let target = resolve_region_target_from_page_state(
        page_state,
        page_index,
        region_id,
        kind,
        original_text,
    )?;
    let mut patch = build_paragraph_patch(page_state, &target.paragraph_id, new_text)?;
    if patch.target_indices.is_empty() && !target.target_indices.is_empty() {
        patch.target_indices = target.target_indices.clone();
    }
    Some(patch)
}

pub fn build_active_editor_patch(
    page_state: &PageState,
    active_paragraph_id: Option<&str>,
    new_text: String,
) -> Option<PersistableRegionPatch> {
    let paragraph_id = active_paragraph_id?;
    build_paragraph_patch(page_state, paragraph_id, new_text)
}
