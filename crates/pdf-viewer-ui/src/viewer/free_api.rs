//! Viewer-domain free wasm exports retained for the TS bridge.
//!
//! 这些函数原位于 `wasm_api/viewer.rs`（已删除）。新代码请走 `ViewerSession` /
//! `RenderPipeline`。

use pdf_viewer_core::models::{GlyphPaintPlan, VectorPageModel};
use wasm_bindgen::prelude::*;

use crate::page::context;
use crate::viewer::viewer_controller;

#[wasm_bindgen(js_name = "initPageContext")]
pub fn init_page_context(
    vector_model_json: String,
    glyph_plan_json: String,
    zoom: f32,
    dpr: f32,
    viewport_left: Option<f32>,
    viewport_top: Option<f32>,
    viewport_width: Option<f32>,
    viewport_height: Option<f32>,
) {
    let vector_model: VectorPageModel =
        serde_json::from_str(&vector_model_json).unwrap_or_else(|e| {
            crate::editor::debug_trace::record_editor_debug_event(
                "wasm.init",
                "json_error",
                vec![
                    crate::editor::debug_trace::editor_debug_field("error", e.to_string()),
                    crate::editor::debug_trace::editor_debug_field(
                        "json_len",
                        vector_model_json.len(),
                    ),
                ],
            );
            VectorPageModel::default()
        });
    crate::editor::debug_trace::record_editor_debug_event(
        "wasm.init",
        "model_parsed",
        vec![crate::editor::debug_trace::editor_debug_field(
            "object_count",
            vector_model.objects.len(),
        )],
    );
    let paint_plan: GlyphPaintPlan = serde_json::from_str(&glyph_plan_json).unwrap_or_else(|e| {
        crate::editor::debug_trace::record_editor_debug_event(
            "wasm.init",
            "glyph_plan_json_error",
            vec![
                crate::editor::debug_trace::editor_debug_field("error", e.to_string()),
                crate::editor::debug_trace::editor_debug_field("json_len", glyph_plan_json.len()),
            ],
        );
        GlyphPaintPlan::default()
    });
    context::init_page_context_from_models(
        vector_model,
        paint_plan,
        zoom,
        dpr,
        viewport_left,
        viewport_top,
        viewport_width,
        viewport_height,
    );
}

#[wasm_bindgen(js_name = "setCurrentPage")]
pub fn set_current_page(page_index: u16) {
    viewer_controller::set_page(page_index);
}

/// 把 in-memory editor debug trace 全部 console.log 出来。
/// 在 DevTools console 里调用 `await window.__TAURI_INVOKE__... ` 太麻烦，
/// 这个函数是 wasm 直接暴露的，TS 侧 wrapper 直接调即可。
#[wasm_bindgen(js_name = "dumpEditorDebugTrace")]
pub fn dump_editor_debug_trace(filter_substr: Option<String>) -> u32 {
    let events = crate::editor::debug_trace::resolve_editor_debug_trace();
    let needle = filter_substr.unwrap_or_default();
    let mut printed = 0u32;
    for ev in &events {
        if !needle.is_empty() && !ev.node.contains(&needle) && !ev.action.contains(&needle) {
            continue;
        }
        let mut details = String::new();
        for f in &ev.details {
            details.push_str(&format!(" {}={}", f.key, f.value));
        }
        let line = format!("[{}] {}::{}{}", ev.seq, ev.node, ev.action, details);
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&line));
        printed += 1;
    }
    printed
}
