use serde::{Deserialize, Serialize};

use pdf_viewer_core::geometry::coordinate_transform::{
    ClientPoint, HostPageTransform, HostReferenceRect, PageSize,
};
use pdf_viewer_core::models::BoundingBox;

use crate::document::patch_persistence::{has_persistable_patches, save_persistable_patches};
use pdf_viewer_core::edit::bridge::{collect_paragraph_interaction_targets, ParagraphInteractionTarget};
use crate::editor::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use crate::editor::editor_controller::{
    find_paragraph_shell_bbox, open_editor_at_page_point, open_region_editor, set_editor_caret,
    EditorVisibilityAction,
};
use crate::editor::mode::{close_active_editor, read_active_editor_state};
use crate::editor::orchestrator::commit::commit_pending_edit_if_any;
use crate::editor::text_geometry::active_caret_index_at_shell_point;
use crate::page::page_store::with_page_state;

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

fn resolve_page_point_from_client(
    client_x: f32,
    client_y: f32,
    reference_left: f32,
    reference_top: f32,
    reference_width: f32,
    reference_height: f32,
    page_width: f32,
    page_height: f32,
    shell_bbox: BoundingBox,
) -> (f32, f32) {
    let transform = HostPageTransform::new(
        HostReferenceRect {
            left: reference_left,
            top: reference_top,
            width: reference_width,
            height: reference_height,
        },
        PageSize {
            width: page_width,
            height: page_height,
        },
    );
    let page_point = transform.client_to_page(ClientPoint {
        x: client_x,
        y: client_y,
    });
    // Clamp to the paragraph shell bbox so the caret stays within the target.
    let x = page_point.x.clamp(shell_bbox.left, shell_bbox.right);
    let y = page_point.y.clamp(shell_bbox.top, shell_bbox.bottom);
    (x, y)
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

fn resolve_target_at_page_point(page_x: f32, page_y: f32) -> Option<ParagraphInteractionTarget> {
    let targets = with_page_state(|state| {
        state
            .paint_plan
            .as_ref()
            .map(|plan| collect_paragraph_interaction_targets(plan, state.vector_model.as_ref()))
            .unwrap_or_default()
    });
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

    // No nearest-neighbor fallback — clicking blank area must NOT match a distant paragraph.
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
            // 点空白 = 退出编辑：先 commit pending edit 持久化当前编辑，
            // 再 close 当前 active editor，让 UI 回到 idle。
            let committed = commit_pending_edit_if_any();
            let had_active = read_active_editor_state().is_some();
            close_active_editor();
            crate::chain_trace!(
                "activate.hit-miss-exit",
                "committed" => committed,
                "hadActive" => had_active,
            );
            dbg_event(
                "activation.client",
                "target-hit-missing",
                vec![
                    dbg_field("clientX", request.client_x),
                    dbg_field("clientY", request.client_y),
                    dbg_field("pageX", page_point.x),
                    dbg_field("pageY", page_point.y),
                    dbg_field("committed", committed),
                    dbg_field("hadActive", had_active),
                ],
            );
            return EditorVisibilityAction {
                changed: had_active,
                request_visibility_render: had_active,
            };
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
    let (click_page_x, click_page_y) = resolve_page_point_from_client(
        request.client_x,
        request.client_y,
        request.reference_left,
        request.reference_top,
        request.reference_width,
        request.reference_height,
        request.page_width,
        request.page_height,
        shell_bbox,
    );

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
    crate::chain_trace!(
        "caret.diag.open",
        "targetId" => &resolved_paragraph_id,
        "clientX" => format!("{:.2}", request.client_x),
        "clientY" => format!("{:.2}", request.client_y),
        "refLeft" => format!("{:.2}", request.reference_left),
        "refTop" => format!("{:.2}", request.reference_top),
        "refW" => format!("{:.2}", request.reference_width),
        "refH" => format!("{:.2}", request.reference_height),
        "pageW" => format!("{:.2}", request.page_width),
        "pageH" => format!("{:.2}", request.page_height),
        "pageX" => format!("{:.2}", click_page_x),
        "pageY" => format!("{:.2}", click_page_y),
        "shellL" => format!("{:.2}", shell_bbox.left),
        "shellT" => format!("{:.2}", shell_bbox.top),
        "shellR" => format!("{:.2}", shell_bbox.right),
        "shellB" => format!("{:.2}", shell_bbox.bottom),
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
    let active_state = read_active_editor_state()?;
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
    // Convert client → page coordinates using full-page transform, then to local.
    let page_point = transform.client_to_page(ClientPoint {
        x: request.client_x,
        y: request.client_y,
    });
    let shell_x =
        (page_point.x - shell_bbox.left).clamp(0.0, (shell_bbox.right - shell_bbox.left).max(0.0));
    let shell_y =
        (page_point.y - shell_bbox.top).clamp(0.0, (shell_bbox.bottom - shell_bbox.top).max(0.0));

    // The click is already confirmed to be within the editor shell bounds
    // (only the shell pointerdown handler calls this function). Closing the
    // editor is handled by clicks *outside* the shell (root handler → discard).
    // Previously a blank-click guard (`is_click_on_paragraph_runs`) was here,
    // but it was too strict for multi-line paragraphs with varying line widths:
    // clicking at the end of a short line fell outside all source run bboxes
    // and mistakenly closed the editor, forcing a second click to reposition
    // the caret. The caret resolution function already handles out-of-bounds
    // positions by snapping to the nearest caret stop.

    let caret_index =
        active_caret_index_at_shell_point(&active_target, &draft_text, shell_x, shell_y);
    crate::chain_trace!(
        "caret.diag.move",
        "clientX" => format!("{:.2}", request.client_x),
        "clientY" => format!("{:.2}", request.client_y),
        "refLeft" => format!("{:.2}", request.reference_left),
        "refTop" => format!("{:.2}", request.reference_top),
        "refW" => format!("{:.2}", request.reference_width),
        "refH" => format!("{:.2}", request.reference_height),
        "pageW" => format!("{:.2}", request.page_width),
        "pageH" => format!("{:.2}", request.page_height),
        "pageX" => format!("{:.2}", page_point.x),
        "pageY" => format!("{:.2}", page_point.y),
        "shellL" => format!("{:.2}", shell_bbox.left),
        "shellT" => format!("{:.2}", shell_bbox.top),
        "shellR" => format!("{:.2}", shell_bbox.right),
        "shellB" => format!("{:.2}", shell_bbox.bottom),
        "shellX" => format!("{:.2}", shell_x),
        "shellY" => format!("{:.2}", shell_y),
        "caretIndex" => caret_index,
    );
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
