use crate::edit::active_target::ActiveEditorTarget;
use crate::edit::document_plan::collect_all;
use crate::edit::edit_target::get_base_paragraph_id;
use crate::edit::paragraph_scene::build_target_scene;
use crate::edit::paragraph_scene::ParagraphEditorScene;
use crate::edit::replacement_snapshot::build_edit_replacement_snapshot;
use crate::edit::source_identity::collect_run_indices;
use crate::models::{GlyphPaintParagraph, GlyphPaintPlan, VectorPageModel};
use crate::persistence::models::PersistableRegionPatch;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ParagraphInteractionTarget {
    pub paragraph_id: String,
    pub region_id: String,
    pub page_index: u16,
    pub text: String,
    #[serde(default)]
    pub target_indices: Vec<usize>,
    pub bbox: crate::models::BoundingBox,
    pub font_family: String,
    pub font_size: f32,
    pub font_weight: String,
    pub font_style: String,
    pub color: String,
    #[serde(default)]
    pub text_decoration: String,
}

pub fn collect_paragraph_interaction_targets(
    plan: &GlyphPaintPlan,
    vector_model: Option<&VectorPageModel>,
) -> Vec<ParagraphInteractionTarget> {
    let mut targets = Vec::new();
    for region in &plan.regions {
        for paragraph in &region.paragraphs {
            let control_style = &paragraph.control_style;

            for document_plan in collect_all(paragraph, vector_model) {
                let target_indices =
                    resolve_target_indices_from_runs(&document_plan.original_runs, vector_model);
                targets.push(ParagraphInteractionTarget {
                    paragraph_id: document_plan.target_id.clone(),
                    region_id: paragraph.region_id.clone(),
                    page_index: plan.page_index,
                    text: document_plan.source_body_text().to_string(),
                    target_indices,
                    bbox: document_plan.shell_bbox,
                    font_family: control_style.font_family.clone(),
                    font_size: control_style.font_size.max(1.0),
                    font_weight: control_style.font_weight.clone(),
                    font_style: control_style.font_style.clone(),
                    color: control_style.color.clone(),
                    text_decoration: control_style.text_decoration.clone(),
                });
            }
        }
    }
    targets
}

pub fn build_paragraph_patch(
    plan: &GlyphPaintPlan,
    vector_model: Option<&VectorPageModel>,
    paragraph_id: &str,
    new_text: String,
) -> Option<PersistableRegionPatch> {
    build_rich_patch(plan, vector_model, paragraph_id, new_text, None)
}

pub fn build_rich_patch(
    plan: &GlyphPaintPlan,
    vector_model: Option<&VectorPageModel>,
    paragraph_id: &str,
    new_text: String,
    new_runs: Option<Vec<crate::models::LayoutRun>>,
) -> Option<PersistableRegionPatch> {
    let base_paragraph_id = get_base_paragraph_id(paragraph_id);
    for region in &plan.regions {
        for paragraph in &region.paragraphs {
            if paragraph.id != base_paragraph_id {
                continue;
            }
            let scene = build_target_scene(paragraph, vector_model, paragraph_id, None)?;
            let replacement_target = active_editor_target_from_scene(plan, paragraph, &scene);
            let original_text = scene.body_text().to_string();
            let is_list_item = scene.marker().is_some();
            let full_target_indices = if let Some(marker) = scene.marker() {
                let mut target_indices = marker
                    .runs
                    .iter()
                    .flat_map(|run| run.object_indices.iter().copied())
                    .collect::<BTreeSet<_>>();
                target_indices.extend(resolve_target_indices_from_runs(
                    scene.original_runs(),
                    vector_model,
                ));
                target_indices.into_iter().collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            let marker_text = scene.marker().map(|marker| marker.text.clone());
            let snapshot = build_edit_replacement_snapshot(
                replacement_target,
                if is_list_item {
                    "list-item"
                } else {
                    "paragraph"
                },
                new_text.clone(),
                marker_text.clone(),
                None,
            );
            return Some(PersistableRegionPatch {
                patch_key: format!(
                    "{}:{}",
                    if is_list_item {
                        "list-item"
                    } else {
                        "paragraph"
                    },
                    scene.target_id
                ),
                region_id: scene.target_id.clone(),
                page_index: plan.page_index,
                original_text,
                new_text,
                new_runs,
                source: if is_list_item {
                    "list-item-region".into()
                } else {
                    "paragraph-region".into()
                },
                marker_text,
                new_marker_text: None,
                snapshot: Some(snapshot),
                kind: Some(if is_list_item {
                    "list-item".into()
                } else {
                    "paragraph".into()
                }),
                pair_id: None,
                group_id: None,
                field_kind: None,
                field_name: None,
                original_value_text: None,
                new_value_text: None,
                target_indices: resolve_target_indices_from_runs(
                    scene.original_runs(),
                    vector_model,
                ),
                full_target_indices,
                displacement_y: None,
                wrap_width: Some(
                    scene
                        .body_session()
                        .paragraph
                        .wrap_width
                        .max(scene.shell_bbox.right - scene.shell_bbox.left),
                ),
                align: Some(paragraph.style.align),
                line_height: Some(paragraph.style.line_height.max(1.0)),
                char_spacing: 0.0,
                horizontal_scaling: 100.0,
            });
        }
    }
    None
}

fn active_editor_target_from_scene(
    plan: &GlyphPaintPlan,
    paragraph: &GlyphPaintParagraph,
    scene: &ParagraphEditorScene,
) -> ActiveEditorTarget {
    ActiveEditorTarget {
        paragraph_id: scene.target_id.clone(),
        region_id: paragraph.region_id.clone(),
        page_index: plan.page_index,
        text: scene.body_text().to_string(),
        bbox_left: scene.shell_bbox.left,
        bbox_top: scene.shell_bbox.top,
        bbox_right: scene.shell_bbox.right,
        bbox_bottom: scene.shell_bbox.bottom,
        font_family: paragraph.control_style.font_family.clone(),
        font_size: paragraph.control_style.font_size.max(1.0),
        font_weight: paragraph.control_style.font_weight.clone(),
        font_style: paragraph.control_style.font_style.clone(),
        color: paragraph.control_style.color.clone(),
        text_decoration: paragraph.control_style.text_decoration.clone(),
        initial_caret_index: scene.body_initial_caret(),
        editor_session: scene.body_session().clone(),
        scene: scene.clone(),
    }
}

pub fn build_editor_target(
    plan: &GlyphPaintPlan,
    vector_model: Option<&VectorPageModel>,
    paragraph_id: &str,
    click_page_x: f32,
    click_page_y: f32,
) -> Option<ActiveEditorTarget> {
    let base_paragraph_id = get_base_paragraph_id(paragraph_id);
    for region in &plan.regions {
        for paragraph in &region.paragraphs {
            if paragraph.id != base_paragraph_id {
                continue;
            }
            let scene = build_target_scene(
                paragraph,
                vector_model,
                paragraph_id,
                Some((click_page_x, click_page_y)),
            )?;
            return Some(ActiveEditorTarget {
                paragraph_id: scene.target_id.clone(),
                region_id: paragraph.region_id.clone(),
                page_index: plan.page_index,
                text: scene.body_text().to_string(),
                bbox_left: scene.shell_bbox.left,
                bbox_top: scene.shell_bbox.top,
                bbox_right: scene.shell_bbox.right,
                bbox_bottom: scene.shell_bbox.bottom,
                font_family: paragraph.control_style.font_family.clone(),
                font_size: paragraph.control_style.font_size.max(1.0),
                font_weight: paragraph.control_style.font_weight.clone(),
                font_style: paragraph.control_style.font_style.clone(),
                color: paragraph.control_style.color.clone(),
                text_decoration: paragraph.control_style.text_decoration.clone(),
                initial_caret_index: scene.body_initial_caret(),
                editor_session: scene.body_session().clone(),
                scene,
            });
        }
    }
    None
}

pub fn build_paragraph_render_target(
    plan: &GlyphPaintPlan,
    vector_model: Option<&VectorPageModel>,
    paragraph_id: &str,
) -> Option<ActiveEditorTarget> {
    let base_paragraph_id = get_base_paragraph_id(paragraph_id);
    for region in &plan.regions {
        for paragraph in &region.paragraphs {
            if paragraph.id != base_paragraph_id {
                continue;
            }
            let scene = build_target_scene(paragraph, vector_model, paragraph_id, None)?;
            return Some(ActiveEditorTarget {
                paragraph_id: scene.target_id.clone(),
                region_id: paragraph.region_id.clone(),
                page_index: plan.page_index,
                text: scene.body_text().to_string(),
                bbox_left: scene.shell_bbox.left,
                bbox_top: scene.shell_bbox.top,
                bbox_right: scene.shell_bbox.right,
                bbox_bottom: scene.shell_bbox.bottom,
                font_family: paragraph.control_style.font_family.clone(),
                font_size: paragraph.control_style.font_size.max(1.0),
                font_weight: paragraph.control_style.font_weight.clone(),
                font_style: paragraph.control_style.font_style.clone(),
                color: paragraph.control_style.color.clone(),
                text_decoration: paragraph.control_style.text_decoration.clone(),
                initial_caret_index: scene.body_initial_caret(),
                editor_session: scene.body_session().clone(),
                scene,
            });
        }
    }
    None
}

pub fn resolve_paragraph_shell_bbox(
    plan: &GlyphPaintPlan,
    paragraph_id: &str,
) -> Option<crate::models::BoundingBox> {
    let base_paragraph_id = get_base_paragraph_id(paragraph_id);
    for region in &plan.regions {
        for paragraph in &region.paragraphs {
            if paragraph.id != base_paragraph_id {
                continue;
            }
            let scene = build_target_scene(paragraph, None, paragraph_id, None)?;
            return Some(scene.shell_bbox);
        }
    }
    None
}

fn resolve_target_indices_from_runs(
    runs: &[crate::models::GlyphPaintRun],
    vector_model: Option<&VectorPageModel>,
) -> Vec<usize> {
    collect_run_indices(runs, vector_model)
}
