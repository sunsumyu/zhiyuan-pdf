//! 编辑器调试事件追踪 — 从 ui::editor::debug_trace 迁入。
//! 纯 Rust thread_local 缓冲；无 wasm 依赖。

use serde::{Deserialize, Serialize};
use std::cell::RefCell;

const MAX_EDITOR_DEBUG_EVENTS: usize = 240;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EditorDebugField {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EditorDebugTraceEvent {
    pub seq: u64,
    pub node: String,
    pub action: String,
    pub details: Vec<EditorDebugField>,
}

#[derive(Debug, Default)]
struct EditorDebugTraceState {
    next_seq: u64,
    events: Vec<EditorDebugTraceEvent>,
}

thread_local! {
    static EDITOR_DEBUG_TRACE: RefCell<EditorDebugTraceState> =
        RefCell::new(EditorDebugTraceState::default());
}

pub fn editor_debug_field(key: &str, value: impl ToString) -> EditorDebugField {
    EditorDebugField {
        key: key.to_string(),
        value: value.to_string(),
    }
}

pub fn record_editor_debug_event(node: &str, action: &str, details: Vec<EditorDebugField>) {
    EDITOR_DEBUG_TRACE.with(|trace| {
        let mut trace = trace.borrow_mut();
        let seq = trace.next_seq;
        trace.next_seq = trace.next_seq.saturating_add(1);
        trace.events.push(EditorDebugTraceEvent {
            seq,
            node: node.to_string(),
            action: action.to_string(),
            details,
        });
        let overflow = trace.events.len().saturating_sub(MAX_EDITOR_DEBUG_EVENTS);
        if overflow > 0 {
            trace.events.drain(0..overflow);
        }
    });
}

pub fn resolve_editor_debug_trace() -> Vec<EditorDebugTraceEvent> {
    EDITOR_DEBUG_TRACE.with(|trace| trace.borrow().events.clone())
}
