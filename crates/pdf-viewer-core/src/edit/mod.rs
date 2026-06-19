//! 编辑器领域模型 — 从 ui::editor 迁入的纯数据结构（无 wasm/web_sys 依赖）。
//! 构建/状态管理逻辑仍保留在 ui 侧。

pub mod active_target;
pub mod bridge;
pub mod debug_trace;
pub mod document_edit_ops;
pub mod document_plan;
pub mod document_runtime;
pub mod draft_layout;
mod draft_style;
mod draft_text_diff;
mod draft_types;
pub mod edit_target;
pub mod editor_types;
pub mod engine_state;
pub mod paragraph_overlay;
pub mod paragraph_scene;
pub mod replacement_region;
pub mod replacement_snapshot;
pub mod source_identity;
pub mod source_runs;
pub mod source_text;
pub mod target_resolution;
