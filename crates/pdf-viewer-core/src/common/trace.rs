//! Unified tracing infrastructure — cross-cutting concern management.
//!
//! Inspired by Rust's `tracing` crate and Spring AOP's advice model, but
//! designed for this project's constraints: pure-Rust core, WASM-safe,
//! no runtime reflection.
//!
//! ## Design
//!
//! - **TraceSubscriber trait**: pluggable backend. Default is no-op (zero cost).
//!   The existing thread_local buffer in `debug_trace.rs` becomes one implementation.
//! - **TraceLevel**: Info / Debug / Trace. Events above the configured max level
//!   are silently dropped before any field formatting runs.
//! - **TraceEvent**: structured (node, action, fields) — not formatted strings.
//!
//! ## Usage
//!
//! Call sites emit via `crate::common::trace::emit(...)` directly, or via the
//! existing `record_editor_debug_event` helper (which transparently forwards
//! into this module). The Tauri backend installs a `TraceSubscriber` at startup
//! to route events into its ring buffer and ANSI terminal output.

use std::borrow::Cow;

/// Severity levels, ordered from most to least verbose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TraceLevel {
    Info = 1,
    Debug = 2,
    Trace = 3,
}

/// A structured trace field value.
#[derive(Debug, Clone)]
pub struct TraceField {
    pub key: Cow<'static, str>,
    pub value: String,
}

impl TraceField {
    pub fn new(key: impl Into<Cow<'static, str>>, value: impl ToString) -> Self {
        Self {
            key: key.into(),
            value: value.to_string(),
        }
    }
}

/// A structured trace event.
#[derive(Debug, Clone)]
pub struct TraceEvent {
    pub level: TraceLevel,
    pub node: Cow<'static, str>,
    pub action: Cow<'static, str>,
    pub fields: Vec<TraceField>,
}

/// Pluggable backend for trace events.
/// Implementations: buffer (dev), console (browser), log_service (Tauri), no-op (prod).
pub trait TraceSubscriber {
    fn on_event(&self, event: &TraceEvent);
}

/// No-op subscriber — default. All events are silently dropped.
pub struct NoOpSubscriber;

impl TraceSubscriber for NoOpSubscriber {
    fn on_event(&self, _event: &TraceEvent) {}
}

// --- subscriber registry ---

use std::cell::RefCell;

thread_local! {
    static SUBSCRIBER: RefCell<Option<Box<dyn TraceSubscriber>>> = RefCell::new(None);
    static MAX_LEVEL: RefCell<TraceLevel> = RefCell::new(TraceLevel::Trace);
}

/// Install a subscriber (replaces any previous one).
/// Pass `None` to revert to no-op.
pub fn set_subscriber(subscriber: Option<Box<dyn TraceSubscriber>>) {
    SUBSCRIBER.with(|s| *s.borrow_mut() = subscriber);
}

/// Set the maximum level to emit. Events above this level are dropped.
pub fn set_max_level(level: TraceLevel) {
    MAX_LEVEL.with(|m| *m.borrow_mut() = level);
}

/// Core emit function. All trace calls funnel through here.
pub fn emit(
    level: TraceLevel,
    node: impl Into<Cow<'static, str>>,
    action: impl Into<Cow<'static, str>>,
    fields: Vec<TraceField>,
) {
    let pass = MAX_LEVEL.with(|m| *m.borrow() >= level);
    if !pass {
        return;
    }

    let event = TraceEvent {
        level,
        node: node.into(),
        action: action.into(),
        fields,
    };
    SUBSCRIBER.with(|s| {
        if let Some(ref sub) = *s.borrow() {
            sub.on_event(&event);
        }
    });
}

// --- convenience builders ---

pub fn field(key: impl Into<Cow<'static, str>>, value: impl ToString) -> TraceField {
    TraceField::new(key, value)
}

// --- RAII span (around-advice) ---
//
// Mirrors `tracing::span` / Spring Around advice: emit a `.begin` event on
// construction, a `.end` event with elapsed time on `finish()`, and a
// `.end` with `result=aborted` if dropped without finishing.
//
// All three events funnel through `emit()`, so any installed TraceSubscriber
// sees them uniformly.

pub struct TraceSpan {
    level: TraceLevel,
    node: String,
    action: String,
    fields: Vec<TraceField>,
    start: std::time::Instant,
    finished: bool,
}

impl TraceSpan {
    pub fn begin(
        level: TraceLevel,
        node: impl Into<String>,
        action: impl Into<String>,
        fields: Vec<TraceField>,
    ) -> Self {
        let node = node.into();
        let action = action.into();
        emit(
            level,
            node.clone(),
            format!("{}.begin", action),
            fields.clone(),
        );
        Self {
            level,
            node,
            action,
            fields,
            start: std::time::Instant::now(),
            finished: false,
        }
    }

    pub fn finish(mut self, result: &str, extra: Vec<TraceField>) {
        self.finished = true;
        let mut fields = self.fields.clone();
        fields.push(field("result", result));
        fields.push(field("elapsedMs", self.start.elapsed().as_millis()));
        fields.extend(extra);
        emit(
            self.level,
            self.node.clone(),
            format!("{}.end", self.action),
            fields,
        );
    }
}

impl Drop for TraceSpan {
    fn drop(&mut self) {
        if self.finished {
            return;
        }
        let mut fields = self.fields.clone();
        fields.push(field("result", "aborted"));
        fields.push(field("elapsedMs", self.start.elapsed().as_millis()));
        emit(
            self.level,
            self.node.clone(),
            format!("{}.end", self.action),
            fields,
        );
    }
}

/// Convenience macro: `trace_span!(TraceLevel::Info, "doc", "open", "path" => "x")`
/// expands to a `TraceSpan` bound to `_span`. Drop or `.finish()` to close.
#[macro_export]
macro_rules! trace_span {
    ($level:expr, $node:expr, $action:expr $(, $key:expr => $val:expr)* $(,)?) => {{
        let mut _fields = Vec::new();
        $( _fields.push($crate::common::trace::field($key, $val)); )+
        $crate::common::trace::TraceSpan::begin($level, $node, $action, _fields)
    }};
}
