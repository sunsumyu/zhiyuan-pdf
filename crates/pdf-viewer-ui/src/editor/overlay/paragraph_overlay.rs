use std::collections::BTreeMap;

use pdf_viewer_core::models::{GlyphPaintPlan, VectorPageModel};

use crate::editor::bridge::build_paragraph_render_target;
use crate::editor::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use crate::editor::edit_target::edit_target_base_paragraph_id;
use crate::editor::list_format::{
    collect_marker_overrides, resolve_active_marker_text,
};
use crate::editor::mode::get_active_editor_state;
use crate::editor::replacement_snapshot::replacement_target_from_patch_snapshot;
use crate::editor::session::ActiveEditorTarget;
use crate::editor::source_identity::collect_target_source_object_indices;
use crate::page::page_store::HOST_PAGE_STATE;
use crate::state_manager::get_patch_state;

// 数据结构已迁至 pdf_viewer_core::edit::paragraph_overlay。
pub use pdf_viewer_core::edit::paragraph_overlay::{
    ParagraphRenderOverlay, ParagraphRenderOverlayOwner,
};

fn target_source_object_indices(target: &ActiveEditorTarget) -> Vec<usize> {
    collect_target_source_object_indices(target)
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

pub fn collect_paragraph_render_overlays(
    plan: &GlyphPaintPlan,
    vector_model: Option<&VectorPageModel>,
) -> Vec<ParagraphRenderOverlay> {
    let mut overlays = BTreeMap::<String, ParagraphRenderOverlay>::new();
    let active_state = get_active_editor_state();
    let marker_overrides = collect_marker_overrides(Some(plan), active_state.as_ref());

    if let Ok(state) = get_patch_state().read() {
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
                .or_else(|| replacement_target_from_patch_snapshot(patch))
                .or_else(|| build_paragraph_render_target(plan, vector_model, paragraph_id))
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
            let base_paragraph_id =
                edit_target_base_paragraph_id(&target.paragraph_id).to_string();
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
            overlays.insert(
                paragraph_id.clone(),
                ParagraphRenderOverlay {
                    owner: ParagraphRenderOverlayOwner::PersistedPageCanvas,
                    target,
                    source_object_indices,
                    source_text: patch.original_text.clone(),
                    draft_text: patch.new_text.clone(),
                    replaces_source: true,
                    marker_text_override: patch
                        .new_marker_text
                        .clone()
                        .or_else(|| marker_overrides.get(&base_paragraph_id).cloned().flatten()),
                },
            );
        }
    }

    if let Some(active_state) = active_state {
        let marker_text_override = marker_overrides
            .get(edit_target_base_paragraph_id(
                active_state.paragraph_id(),
            ))
            .cloned()
            .flatten()
            .or_else(|| {
                HOST_PAGE_STATE.with(|page_state: &crate::page::page_store::HostPageState| {
                    resolve_active_marker_text(&active_state, &page_state.borrow())
                })
            });
        overlays.insert(
            active_state.paragraph_id().to_string(),
            ParagraphRenderOverlay {
                owner: ParagraphRenderOverlayOwner::ActiveEditorShell,
                target: active_state.target.clone(),
                source_object_indices: target_source_object_indices(&active_state.target),
                source_text: active_state.target.source_body_text().to_string(),
                draft_text: active_state.current_text().to_string(),
                replaces_source: active_state.requires_source_replacement(),
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
    use crate::models::PersistableRegionPatch;
    use crate::state_manager::{apply_patch_with_history, get_patch_state};
    use pdf_viewer_core::models::{
        BoundingBox, EditorSession, GlyphPaintPlan, GlyphPaintRegion, LayoutMode, LayoutParagraph,
        LayoutRole,
    };
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
        target.scene.body_session = EditorSession {
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
        let mut s = get_patch_state().write().unwrap();
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
    }

    /// 端到端：apply_patch_with_history 后，collect_paragraph_render_overlays
    /// 必须返回一个 PersistedPageCanvas overlay，draft_text 为编辑后的新文本。
    /// 这是验证"退出编辑后修改不丢失"的核心测试。
    #[wasm_bindgen_test]
    fn persisted_patch_yields_overlay_with_new_text_after_commit() {
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

        apply_patch_with_history(patch);

        // 确认 patch 入了 state
        {
            let state = get_patch_state().read().unwrap();
            assert_eq!(
                state.paragraph_patches.len(),
                1,
                "patch must be persisted in paragraph_patches"
            );
            assert!(
                state.paragraph_replacement_targets.contains_key(paragraph_id),
                "replacement target must be persisted from snapshot"
            );
        }

        // 模拟"退出编辑后渲染"：collect_paragraph_render_overlays
        let plan = make_glyph_plan(0);
        let overlays = collect_paragraph_render_overlays(&plan, None);

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
    fn production_commit_flow_preserves_edit_after_exit() {
        clear_state();

        let paragraph_id = "p-prod-1";
        let target = make_active_editor_target(paragraph_id);

        // 模拟 commit.rs:42 显式记录 replacement target
        crate::state_manager::remember_paragraph_replacement_target(paragraph_id, target);

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
        apply_patch_with_history(patch);

        let plan = make_glyph_plan(0);
        let overlays = collect_paragraph_render_overlays(&plan, None);

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
    fn persisted_patch_skipped_when_page_index_mismatches() {
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
        apply_patch_with_history(patch);

        let plan = make_glyph_plan(0); // 当前渲染 page 0
        let overlays = collect_paragraph_render_overlays(&plan, None);
        assert_eq!(
            overlays
                .iter()
                .filter(|o| matches!(o.owner, ParagraphRenderOverlayOwner::PersistedPageCanvas))
                .count(),
            0,
            "patch from other page must be skipped"
        );
    }
}
