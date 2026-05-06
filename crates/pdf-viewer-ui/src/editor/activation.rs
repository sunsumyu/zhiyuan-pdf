use serde::{Deserialize, Serialize};

use pdf_viewer_core::coordinate_transform::{
    ClientPoint, HostPageTransform, HostReferenceRect, PageSize,
};
use pdf_viewer_core::models::BoundingBox;

use crate::editor::bridge::{
    collect_paragraph_interaction_targets, ParagraphInteractionTarget,
};
use crate::editor::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use crate::editor::mode::{close_active_editor, get_active_editor_state};
use crate::editor::runtime::{
    find_paragraph_shell_bbox, open_editor_at_page_point, open_region_editor,
    set_editor_caret, EditorVisibilityAction,
};
use crate::editor::text_geometry::active_caret_index_at_shell_point;
use crate::page::runtime::HOST_PAGE_STATE;
use crate::document::patch_persistence::{has_persistable_patches, save_persistable_patches};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OpenEditorAtClientPointRequest {
    #[serde(alias = "paragraph_id")]
    pub paragraph_id: String,
    #[serde(alias = "clientX", alias = "client_x")]
    pub client_x: f32,
    #[serde(alias = "clientY", alias = "client_y")]
    pub client_y: f32,
    #[serde(alias = "referenceLeft", alias = "reference_left")]
    pub reference_left: f32,
    #[serde(alias = "referenceTop", alias = "reference_top")]
    pub reference_top: f32,
    #[serde(alias = "referenceWidth", alias = "reference_width")]
    pub reference_width: f32,
    #[serde(alias = "referenceHeight", alias = "reference_height")]
    pub reference_height: f32,
    #[serde(alias = "pageWidth", alias = "page_width")]
    pub page_width: f32,
    #[serde(alias = "pageHeight", alias = "page_height")]
    pub page_height: f32,
    #[serde(default, alias = "fallbackPageX", alias = "fallback_page_x")]
    pub fallback_page_x: f32,
    #[serde(default, alias = "fallbackPageY", alias = "fallback_page_y")]
    pub fallback_page_y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MoveCaretToClientPointRequest {
    #[serde(alias = "clientX", alias = "client_x")]
    pub client_x: f32,
    #[serde(alias = "clientY", alias = "client_y")]
    pub client_y: f32,
    #[serde(alias = "referenceLeft", alias = "reference_left")]
    pub reference_left: f32,
    #[serde(alias = "referenceTop", alias = "reference_top")]
    pub reference_top: f32,
    #[serde(alias = "referenceWidth", alias = "reference_width")]
    pub reference_width: f32,
    #[serde(alias = "referenceHeight", alias = "reference_height")]
    pub reference_height: f32,
    #[serde(alias = "pageWidth", alias = "page_width")]
    pub page_width: f32,
    #[serde(alias = "pageHeight", alias = "page_height")]
    pub page_height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SaveEditorSessionResult {
    pub saved: bool,
    pub had_persistable_patches: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

fn resolve_page_point_from_projected_shell(
    client_x: f32,
    client_y: f32,
    reference_left: f32,
    reference_top: f32,
    reference_width: f32,
    reference_height: f32,
    shell_bbox_left: f32,
    shell_bbox_top: f32,
    shell_bbox_right: f32,
    shell_bbox_bottom: f32,
) -> Option<(f32, f32)> {
    let transform = HostPageTransform::new(
        HostReferenceRect {
            left: reference_left,
            top: reference_top,
            width: reference_width,
            height: reference_height,
        },
        PageSize {
            width: shell_bbox_right - shell_bbox_left,
            height: shell_bbox_bottom - shell_bbox_top,
        },
    );
    let page_point = transform.client_to_page_in_box(
        ClientPoint {
            x: client_x,
            y: client_y,
        },
        BoundingBox {
            left: shell_bbox_left,
            top: shell_bbox_top,
            right: shell_bbox_right,
            bottom: shell_bbox_bottom,
        },
    );
    Some((page_point.x, page_point.y))
}

fn resolve_shell_center_page_point(shell_bbox: BoundingBox) -> (f32, f32) {
    (
        shell_bbox.left + ((shell_bbox.right - shell_bbox.left).max(0.0) * 0.5),
        shell_bbox.top + ((shell_bbox.bottom - shell_bbox.top).max(0.0) * 0.5),
    )
}

fn point_in_bbox(x: f32, y: f32, bbox: BoundingBox, tolerance: f32) -> bool {
    x >= bbox.left - tolerance
        && x <= bbox.right + tolerance
        && y >= bbox.top - tolerance
        && y <= bbox.bottom + tolerance
}

fn bbox_distance_sq(x: f32, y: f32, bbox: BoundingBox) -> f32 {
    let dx = if x < bbox.left {
        bbox.left - x
    } else if x > bbox.right {
        x - bbox.right
    } else {
        0.0
    };
    let dy = if y < bbox.top {
        bbox.top - y
    } else if y > bbox.bottom {
        y - bbox.bottom
    } else {
        0.0
    };
    (dx * dx) + (dy * dy)
}

fn resolve_target_at_page_point(
    page_x: f32,
    page_y: f32,
) -> Option<ParagraphInteractionTarget> {
    let (targets, plan_regions, plan_paragraphs, has_plan) = HOST_PAGE_STATE.with(|state: &crate::page::runtime::HostPageState| {
        let state = state.borrow();
        let has_plan = state.paint_plan.is_some();
        let (regions, paragraphs) = state
            .paint_plan
            .as_ref()
            .map(|p| {
                let r = p.regions.len();
                let pg = p.regions.iter().map(|r| r.paragraphs.len()).sum::<usize>();
                (r, pg)
            })
            .unwrap_or((0, 0));
        let targets = state
            .paint_plan
            .as_ref()
            .map(|plan| collect_paragraph_interaction_targets(plan, state.vector_model.as_ref()))
            .unwrap_or_default();
        (targets, regions, paragraphs, has_plan)
    });
    dbg_event(
        "activation.client",
        "resolve-target.state",
        vec![
            dbg_field("hasPaintPlan", has_plan),
            dbg_field("planRegions", plan_regions as u32),
            dbg_field("planParagraphs", plan_paragraphs as u32),
            dbg_field("targetCount", targets.len() as u32),
            dbg_field("pageX", page_x),
            dbg_field("pageY", page_y),
        ],
    );
    if targets.is_empty() {
        dbg_event(
            "activation.client",
            "target-hit-empty",
            vec![dbg_field("pageX", page_x), dbg_field("pageY", page_y)],
        );
        return None;
    }

    if let Some(target) = targets
        .iter()
        .find(|target| point_in_bbox(page_x, page_y, target.bbox, 4.0))
    {
        dbg_event(
            "activation.client",
            "target-hit",
            vec![
                dbg_field("paragraphId", &target.paragraph_id),
                dbg_field("pageX", page_x),
                dbg_field("pageY", page_y),
            ],
        );
        return Some(target.clone());
    }

    let nearest = targets
        .iter()
        .map(|target| (bbox_distance_sq(page_x, page_y, target.bbox), target))
        .min_by(|(left, _): &(f32, _), (right, _)| left.total_cmp(right));
    if let Some((distance_sq, target)) = nearest {
        dbg_event(
            "activation.client",
            "target-hit-nearest",
            vec![
                dbg_field("paragraphId", &target.paragraph_id),
                dbg_field("pageX", page_x),
                dbg_field("pageY", page_y),
                dbg_field("distanceSq", distance_sq),
            ],
        );
        if distance_sq <= 900.0 {
            return Some(target.clone());
        }
    }

    None
}

pub fn activate_editor_from_client_point(
    request: OpenEditorAtClientPointRequest,
) -> EditorVisibilityAction {
    let resolved_paragraph_id = if request.paragraph_id.trim().is_empty() {
        let transform = HostPageTransform::new(
            HostReferenceRect {
                left: request.reference_left,
                top: request.reference_top,
                width: request.reference_width,
                height: request.reference_height,
            },
            PageSize {
                width: request.page_width,
                height: request.page_height,
            },
        );
        let page_point = transform.client_to_page(ClientPoint {
            x: request.client_x,
            y: request.client_y,
        });
        let Some(target) = resolve_target_at_page_point(page_point.x, page_point.y) else {
            dbg_event(
                "activation.client",
                "target-hit-missing",
                vec![
                    dbg_field("clientX", request.client_x),
                    dbg_field("clientY", request.client_y),
                    dbg_field("pageX", page_point.x),
                    dbg_field("pageY", page_point.y),
                ],
            );
            return EditorVisibilityAction::default();
        };
        target.paragraph_id
    } else {
        request.paragraph_id.clone()
    };

    let Some(shell_bbox) = find_paragraph_shell_bbox(&resolved_paragraph_id) else {
        dbg_event(
            "activation.client",
            "missing-shell-bbox",
            vec![dbg_field("paragraphId", &resolved_paragraph_id)],
        );
        return EditorVisibilityAction::default();
    };

    let fallback_page_point = if request.fallback_page_x > 0.0 || request.fallback_page_y > 0.0 {
        (request.fallback_page_x, request.fallback_page_y)
    } else {
        resolve_shell_center_page_point(shell_bbox)
    };
    let (click_page_x, click_page_y) = resolve_page_point_from_projected_shell(
        request.client_x,
        request.client_y,
        request.reference_left,
        request.reference_top,
        request.reference_width,
        request.reference_height,
        shell_bbox.left,
        shell_bbox.top,
        shell_bbox.right,
        shell_bbox.bottom,
    )
    .unwrap_or(fallback_page_point);

    dbg_event(
        "activation.client",
        "resolved-open-point",
        vec![
            dbg_field("paragraphId", &request.paragraph_id),
            dbg_field("targetId", &resolved_paragraph_id),
            dbg_field("clientX", request.client_x),
            dbg_field("clientY", request.client_y),
            dbg_field(
                "shellBBox",
                format!(
                    "[{:.2},{:.2},{:.2},{:.2}]",
                    shell_bbox.left, shell_bbox.top, shell_bbox.right, shell_bbox.bottom
                ),
            ),
            dbg_field("pageX", click_page_x),
            dbg_field("pageY", click_page_y),
            dbg_field("fallbackPageX", fallback_page_point.0),
            dbg_field("fallbackPageY", fallback_page_point.1),
        ],
    );

    let primary = open_editor_at_page_point(&resolved_paragraph_id, click_page_x, click_page_y);
    if primary.changed {
        return primary;
    }

    let fallback = open_editor_at_page_point(
        &resolved_paragraph_id,
        fallback_page_point.0,
        fallback_page_point.1,
    );
    fallback
}

pub fn activate_region_editor(
    page_index: u16,
    region_id: &str,
    kind: &str,
    original_text: &str,
) -> EditorVisibilityAction {
    open_region_editor(page_index, region_id, kind, original_text)
}

pub fn move_caret_to_client_point(request: MoveCaretToClientPointRequest) -> Option<usize> {
    let active_state = get_active_editor_state()?;
    let draft_text = active_state.current_text().to_string();
    let active_target = active_state.target;
    let shell_bbox = BoundingBox {
        left: active_target.bbox_left,
        top: active_target.bbox_top,
        right: active_target.bbox_right,
        bottom: active_target.bbox_bottom,
    };
    let transform = HostPageTransform::new(
        HostReferenceRect {
            left: request.reference_left,
            top: request.reference_top,
            width: request.reference_width,
            height: request.reference_height,
        },
        PageSize {
            width: request.page_width,
            height: request.page_height,
        },
    );
    let local_point = transform.client_to_local_in_box(
        ClientPoint {
            x: request.client_x,
            y: request.client_y,
        },
        shell_bbox,
    );
    let shell_x = local_point.x;
    let shell_y = local_point.y;
    let caret_index =
        active_caret_index_at_shell_point(&active_target, &draft_text, shell_x, shell_y);
    let _ = set_editor_caret(caret_index);
    dbg_event(
        "activation.caret",
        "client-to-caret",
        vec![
            dbg_field("paragraphId", active_target.paragraph_id),
            dbg_field("shellX", shell_x),
            dbg_field("shellY", shell_y),
            dbg_field("caretIndex", caret_index),
        ],
    );
    Some(caret_index)
}

pub async fn save_editor_session(path: String, page_index: u16) -> SaveEditorSessionResult {
    let had_persistable_patches = has_persistable_patches();
    close_active_editor();
    if !had_persistable_patches {
        return SaveEditorSessionResult {
            saved: false,
            had_persistable_patches,
            error_message: None,
        };
    }

    match save_persistable_patches(path, page_index).await {
        Ok(_) => SaveEditorSessionResult {
            saved: true,
            had_persistable_patches,
            error_message: None,
        },
        Err(error) => SaveEditorSessionResult {
            saved: false,
            had_persistable_patches,
            error_message: Some(format!("{error:?}")),
        },
    }
}
