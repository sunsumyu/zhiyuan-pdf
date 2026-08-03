use std::collections::BTreeMap;

use pdf_viewer_core::models::{GlyphPaintPlan, VectorPageModel};

use crate::editor::bridge::build_render_target;
use crate::editor::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use crate::editor::edit_target::base_paragraph_id;
use crate::editor::list_format::{collect_marker_overrides, resolve_marker_text};
use crate::editor::mode::read_state;
use crate::editor::replacement_snapshot::find_target;
use crate::editor::session::ActiveEditorTarget;
use crate::editor::source_identity::sorted_object_indices;
use crate::page::page_store::with_page_state;
use crate::ui_state_store::with_patch_state;

// 数据结构已迁至 pdf_viewer_core::edit::paragraph_overlay。
pub use pdf_viewer_core::edit::paragraph_overlay::{
    ParagraphRenderOverlay, ParagraphRenderOverlayOwner,
};

fn target_source_object_indices(target: &ActiveEditorTarget) -> Vec<usize> {
    sorted_object_indices(target)
}

fn persisted_patch_source_indices(
    patch_target_indices: &[usize],
    patch_full_target_indices: &[usize],
    target: &ActiveEditorTarget,
) -> Vec<usize> {
    if !patch_full_target_indices.is_empty() {
        return patch_full_target_indices.to_vec();
    }
    if !patch_target_indices.is_empty() {
        return patch_target_indices.to_vec();
    }
    target_source_object_indices(target)
}

pub fn collect_overlays(
    plan: &GlyphPaintPlan,
    vector_model: Option<&VectorPageModel>,
) -> Vec<ParagraphRenderOverlay> {
    let mut overlays = BTreeMap::<String, ParagraphRenderOverlay>::new();
    let active_state = read_state();
    let marker_overrides = collect_marker_overrides(Some(plan), active_state.as_ref());

    with_patch_state(|state| {
        dbg_event(
            "overlay.collect",
            "start",
            vec![
                dbg_field("pageIndex", plan.page_index),
                dbg_field("totalParagraphPatches", state.paragraph_patches.len()),
                dbg_field("activeEditor", active_state.is_some()),
            ],
        );
        for (paragraph_id, patch) in &state.paragraph_patches {
            if patch.page_index != plan.page_index {
                continue;
            }
            dbg_event(
                "overlay.collect",
                "persisted-patch",
                vec![
                    dbg_field("paragraphId", paragraph_id),
                    dbg_field("originalLen", patch.original_text.chars().count()),
                    dbg_field("newLen", patch.new_text.chars().count()),
                ],
            );
            let Some(target) = state
                .paragraph_replacement_targets
                .get(paragraph_id)
                .cloned()
                .or_else(|| find_target(patch))
                .or_else(|| build_render_target(plan, vector_model, paragraph_id))
            else {
                dbg_event(
                    "overlay.collect",
                    "target-resolution-failed",
                    vec![dbg_field("paragraphId", paragraph_id)],
                );
                continue;
            };
            dbg_event(
                "overlay.collect",
                "target-resolved",
                vec![dbg_field("paragraphId", paragraph_id)],
            );
            let base_id = base_paragraph_id(&target.paragraph_id).to_string();
            let source_object_indices = persisted_patch_source_indices(
                &patch.target_indices,
                &patch.full_target_indices,
                &target,
            );
            dbg_event(
                "overlay.source-indices",
                "persisted",
                vec![
                    dbg_field("paragraphId", paragraph_id),
                    dbg_field("patchTargetCount", patch.target_indices.len()),
                    dbg_field("patchFullTargetCount", patch.full_target_indices.len()),
                    dbg_field("resolvedCount", source_object_indices.len()),
                    dbg_field("resolvedIndices", format!("{:?}", source_object_indices)),
                    dbg_field("sourceText", patch.original_text.as_str()),
                    dbg_field("draftText", patch.new_text.as_str()),
                ],
            );
            let graphic_markers = target.scene.graphic_markers().to_vec();
            overlays.insert(
                paragraph_id.clone(),
                ParagraphRenderOverlay {
                    owner: ParagraphRenderOverlayOwner::PersistedPageCanvas,
                    target,
                    source_object_indices,
                    graphic_markers,
                    source_text: patch.original_text.clone(),
                    draft_text: patch.new_text.clone(),
                    replaces_source: true,
                    marker_text_override: patch
                        .new_marker_text
                        .clone()
                        .or_else(|| marker_overrides.get(&base_id).cloned().flatten()),
                },
            );
        }
    });

    if let Some(active_state) = active_state {
        let marker_text_override = marker_overrides
            .get(base_paragraph_id(active_state.paragraph_id()))
            .cloned()
            .flatten()
            .or_else(|| {
                with_page_state(|page_state| resolve_marker_text(&active_state, page_state))
            });
        let source_object_indices = target_source_object_indices(&active_state.target);
        let graphic_markers = active_state.target.scene.graphic_markers().to_vec();
        let replaces_source = active_state.requires_source_replacement();
        let source_text = active_state.target.source_body_text().to_string();
        let draft_text = active_state.current_text().to_string();

        // ── diagnostic: active overlay identity ──
        {
            use pdf_viewer_core::edit::source_identity::{
                object_ids, object_indices_set,
            };
            let obj_ids = object_ids(&active_state.target);
            let obj_indices = object_indices_set(&active_state.target);
            let orig_run_count = active_state.target.scene.original_runs().len();
            let body_run_count = active_state
                .target
                .scene
                .body_session()
                .paragraph
                .runs
                .len();
            let orig_obj_ids: Vec<String> = active_state
                .target
                .scene
                .original_runs()
                .iter()
                .flat_map(|r| r.object_ids.iter().cloned())
                .collect();
            let orig_obj_indices: Vec<usize> = active_state
                .target
                .scene
                .original_runs()
                .iter()
                .flat_map(|r| r.object_indices.iter().copied())
                .collect();
            let body_obj_ids: Vec<String> = active_state
                .target
                .scene
                .body_session()
                .paragraph
                .runs
                .iter()
                .flat_map(|r| r.object_ids.iter().cloned())
                .collect();
            let body_obj_indices: Vec<usize> = active_state
                .target
                .scene
                .body_session()
                .paragraph
                .runs
                .iter()
                .flat_map(|r| r.object_indices.iter().copied())
                .collect();
            dbg_event(
                "overlay.collect",
                "active-identity-detail",
                vec![
                    dbg_field("paragraphId", active_state.paragraph_id()),
                    dbg_field("replacesSource", replaces_source),
                    dbg_field("currentText", &draft_text),
                    dbg_field("sourceText", &source_text),
                    dbg_field("textsEqual", source_text == draft_text),
                    dbg_field("mergedObjectIds", format!("{:?}", obj_ids)),
                    dbg_field("mergedObjectIdCount", obj_ids.len()),
                    dbg_field("mergedObjectIndices", format!("{:?}", obj_indices)),
                    dbg_field("mergedObjectIndexCount", obj_indices.len()),
                    dbg_field(
                        "sourceObjectIndices",
                        format!("{:?}", source_object_indices),
                    ),
                    dbg_field("origRunCount", orig_run_count),
                    dbg_field("origRunObjectIds", format!("{:?}", orig_obj_ids)),
                    dbg_field("origRunObjectIndices", format!("{:?}", orig_obj_indices)),
                    dbg_field("bodyRunCount", body_run_count),
                    dbg_field("bodyRunObjectIds", format!("{:?}", body_obj_ids)),
                    dbg_field("bodyRunObjectIndices", format!("{:?}", body_obj_indices)),
                ],
            );
        }
        // ── end diagnostic ──

        overlays.insert(
            active_state.paragraph_id().to_string(),
            ParagraphRenderOverlay {
                owner: ParagraphRenderOverlayOwner::ActiveEditorShell,
                target: active_state.target.clone(),
                source_object_indices,
                graphic_markers,
                source_text,
                draft_text,
                replaces_source,
                marker_text_override,
            },
        );
    }

    let result: Vec<ParagraphRenderOverlay> = overlays.into_values().collect();
    let persisted_count = result
        .iter()
        .filter(|o| matches!(o.owner, ParagraphRenderOverlayOwner::PersistedPageCanvas))
        .count();
    let active_count = result
        .iter()
        .filter(|o| matches!(o.owner, ParagraphRenderOverlayOwner::ActiveEditorShell))
        .count();
    crate::chain_trace!(
        "render.collect",
        "page" => plan.page_index,
        "persisted" => persisted_count,
        "active" => active_count,
    );
    result
}

#[cfg(target_arch = "wasm32")]
#[cfg(test)]
mod persisted_overlay_tests {
    use super::*;
    use crate::ui_state_store::{record_patch, with_patch_state, with_patch_state_mut};
    use pdf_viewer_core::models::{
        BoundingBox, GlyphPaintPlan, GlyphPaintRegion, LayoutMode, LayoutParagraph, LayoutRole,
        ParagraphEditContext,
    };
    use pdf_viewer_core::persistence::models::PersistableRegionPatch;
    use serde_json::json;
    use wasm_bindgen_test::wasm_bindgen_test;

    fn make_active_editor_target(paragraph_id: &str) -> ActiveEditorTarget {
        let mut target = ActiveEditorTarget::default();
        target.paragraph_id = paragraph_id.to_string();
        target.scene.shell_bbox = BoundingBox {
            left: 40.0,
            top: 96.0,
            right: 360.0,
            bottom: 116.0,
        };
        *target.scene.body_session_mut() = ParagraphEditContext {
            anchor_bbox: BoundingBox {
                left: 90.0,
                top: 100.0,
                right: 330.0,
                bottom: 112.0,
            },
            paragraph: LayoutParagraph::default(),
        };
        target
    }

    fn make_glyph_plan(page_index: u16) -> GlyphPaintPlan {
        GlyphPaintPlan {
            page_index,
            width: 595.0,
            height: 842.0,
            regions: vec![GlyphPaintRegion {
                id: "r-1".to_string(),
                kind: LayoutRole::Paragraph,
                layout_mode: LayoutMode::Flow,
                bbox: BoundingBox {
                    left: 40.0,
                    top: 96.0,
                    right: 360.0,
                    bottom: 116.0,
                },
                paragraphs: Vec::new(),
                object_ids: Vec::new(),
            }],
            external_objects: Vec::new(),
        }
    }

    fn clear_state() {
        with_patch_state_mut(|s| {
            s.paragraph_texts.clear();
            s.paragraph_snapshots.clear();
            s.paragraph_patches.clear();
            s.paragraph_replacement_targets.clear();
            s.field_group_texts.clear();
            s.field_group_snapshots.clear();
            s.field_group_patches.clear();
            s.history.clear();
            s.redo_stack.clear();
            s.accepted_patch_keys.clear();
        });
    }

    /// 端到端：record_patch 后，collect_overlays
    /// 必须返回一个 PersistedPageCanvas overlay，draft_text 为编辑后的新文本。
    /// 这是验证"退出编辑后修改不丢失"的核心测试。
    #[wasm_bindgen_test]
    fn patch_yields_overlay() {
        clear_state();

        let paragraph_id = "p-test-1";
        let target = make_active_editor_target(paragraph_id);
        let target_json = serde_json::to_value(&target).expect("target serialise");

        let patch = PersistableRegionPatch {
            patch_key: "k1".to_string(),
            page_index: 0,
            region_id: paragraph_id.to_string(),
            original_text: "编程语言: Rust".to_string(),
            new_text: "编程语: Rust".to_string(), // 删了一个"言"
            source: "list-item-region".to_string(),
            snapshot: Some(json!({ "replacementTarget": target_json })),
            kind: Some("text".to_string()),
            ..Default::default()
        };

        record_patch(patch);

        // 确认 patch 入了 state
        with_patch_state(|state| {
            assert_eq!(
                state.paragraph_patches.len(),
                1,
                "patch must be persisted in paragraph_patches"
            );
            assert!(
                state
                    .paragraph_replacement_targets
                    .contains_key(paragraph_id),
                "replacement target must be persisted from snapshot"
            );
        });

        // 模拟"退出编辑后渲染"：collect_overlays
        let plan = make_glyph_plan(0);
        let overlays = collect_overlays(&plan, None);

        let persisted: Vec<&ParagraphRenderOverlay> = overlays
            .iter()
            .filter(|o| matches!(o.owner, ParagraphRenderOverlayOwner::PersistedPageCanvas))
            .collect();
        assert_eq!(
            persisted.len(),
            1,
            "must produce exactly 1 PersistedPageCanvas overlay; got {} overlays total",
            overlays.len()
        );
        let overlay = persisted[0];
        assert_eq!(
            overlay.draft_text, "编程语: Rust",
            "overlay draft_text must reflect edited text (got: {:?})",
            overlay.draft_text
        );
        assert_eq!(
            overlay.source_text, "编程语言: Rust",
            "overlay source_text must reflect original"
        );
        assert!(
            overlay.replaces_source,
            "persisted overlay must replace source"
        );
    }

    /// 关键回归测试：模拟生产真实路径
    /// （commit.rs 通过 remember_paragraph_replacement_target 显式存 target；
    /// patch.snapshot 不含 replacementTarget），验证 overlay 仍被正确发出。
    /// 这是验证"退出编辑后修改不丢失"的真实场景测试。
    #[wasm_bindgen_test]
    fn commit_preserves_edit() {
        clear_state();

        let paragraph_id = "p-prod-1";
        let target = make_active_editor_target(paragraph_id);

        // 模拟 commit.rs:42 显式记录 replacement target
        crate::ui_state_store::remember_target(paragraph_id, target);

        // 模拟生产 patch（snapshot 不含 replacementTarget，与真实 build_edit_replacement_snapshot 一致）
        let patch = PersistableRegionPatch {
            patch_key: "k-prod".to_string(),
            page_index: 0,
            region_id: paragraph_id.to_string(),
            original_text: "编程语言: Rust".to_string(),
            new_text: "编程语: Rust".to_string(),
            source: "list-item-region".to_string(),
            snapshot: Some(json!({
                // 真实 snapshot 不带 replacementTarget
                "schema": "editReplacementSnapshot.v3",
            })),
            kind: Some("text".to_string()),
            ..Default::default()
        };
        record_patch(patch);

        let plan = make_glyph_plan(0);
        let overlays = collect_overlays(&plan, None);

        let persisted: Vec<&ParagraphRenderOverlay> = overlays
            .iter()
            .filter(|o| matches!(o.owner, ParagraphRenderOverlayOwner::PersistedPageCanvas))
            .collect();
        assert_eq!(
            persisted.len(),
            1,
            "production commit flow must yield 1 persisted overlay; got {} total overlays",
            overlays.len()
        );
        assert_eq!(persisted[0].draft_text, "编程语: Rust");
    }

    /// 验证 page_index 不匹配时 overlay 被跳过（保护：不让其他页的 patch 串到当前页）
    #[wasm_bindgen_test]
    fn skips_mismatched_page() {
        clear_state();

        let paragraph_id = "p-test-2";
        let target = make_active_editor_target(paragraph_id);
        let target_json = serde_json::to_value(&target).unwrap();

        let patch = PersistableRegionPatch {
            patch_key: "k2".to_string(),
            page_index: 1, // patch 在 page 1
            region_id: paragraph_id.to_string(),
            original_text: "x".to_string(),
            new_text: "y".to_string(),
            source: "list-item-region".to_string(),
            snapshot: Some(json!({ "replacementTarget": target_json })),
            ..Default::default()
        };
        record_patch(patch);

        let plan = make_glyph_plan(0); // 当前渲染 page 0
        let overlays = collect_overlays(&plan, None);
        assert_eq!(
            overlays
                .iter()
                .filter(|o| matches!(o.owner, ParagraphRenderOverlayOwner::PersistedPageCanvas))
                .count(),
            0,
            "patch from other page must be skipped"
        );
    }

    /// 回归：persisted overlay 必须把 target.scene 中的 graphic_markers 透传到 overlay，
    /// 否则提交后渲染时 should_suppress 会误删图形 bullet 且 overlay 不回绘。
    #[wasm_bindgen_test]
    fn persisted_overlay_carries_graphic_markers() {
        clear_state();

        let paragraph_id = "p-graphic-1";
        let mut target = make_active_editor_target(paragraph_id);
        // 注入一个图形 marker（引用 vector object 索引 7）。
        target.scene.document_plan.graphic_markers =
            vec![pdf_viewer_core::models::VisualMarker::from_graphic(
                7,
                pdf_viewer_core::models::GraphicType::Image,
                "bullet-7".to_string(),
                BoundingBox {
                    left: 40.0,
                    top: 98.0,
                    right: 48.0,
                    bottom: 106.0,
                },
            )];
        let target_json = serde_json::to_value(&target).expect("target serialise");

        let patch = PersistableRegionPatch {
            patch_key: "k-g".to_string(),
            page_index: 0,
            region_id: paragraph_id.to_string(),
            original_text: "Body".to_string(),
            new_text: "Body2".to_string(),
            source: "list-item-region".to_string(),
            snapshot: Some(json!({ "replacementTarget": target_json })),
            kind: Some("text".to_string()),
            ..Default::default()
        };
        record_patch(patch);

        let plan = make_glyph_plan(0);
        let overlays = collect_overlays(&plan, None);
        let persisted: Vec<&ParagraphRenderOverlay> = overlays
            .iter()
            .filter(|o| matches!(o.owner, ParagraphRenderOverlayOwner::PersistedPageCanvas))
            .collect();
        assert_eq!(persisted.len(), 1, "persisted overlay must be produced");
        let overlay = persisted[0];
        assert_eq!(
            overlay.graphic_markers.len(),
            1,
            "persisted overlay must carry the graphic marker from the target scene"
        );
        assert!(overlay.graphic_markers[0].contains_object_index(7));
    }
}
