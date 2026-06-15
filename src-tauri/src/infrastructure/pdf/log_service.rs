use std::collections::VecDeque;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Mutex, OnceLock};

/// Logging levels
/// 0: SILENT (No logs)
/// 1: INFO (Key steps only)
/// 2: DEBUG (Detailed steps)
/// 3: TRACE (Audit logs, font metrics, etc.)
pub static PDF_LOG_LEVEL: AtomicU8 = AtomicU8::new(1);
const PDF_EVENT_LOG_LIMIT: usize = 512;
static PDF_EVENT_LOG: OnceLock<Mutex<VecDeque<String>>> = OnceLock::new();
pub static PDF_EVENT_LOG_MUTEX: Mutex<()> = Mutex::new(());

pub fn set_pdf_log_level(level: u8) {
    PDF_LOG_LEVEL.store(level, Ordering::SeqCst);
}
pub fn get_pdf_log_level() -> u8 {
    PDF_LOG_LEVEL.load(Ordering::SeqCst)
}

pub fn clear_pdf_event_log() {
    let mut events = PDF_EVENT_LOG
        .get_or_init(|| Mutex::new(VecDeque::new()))
        .lock()
        .unwrap();
    events.clear();
}

pub fn read_pdf_event_log() -> Vec<String> {
    let events = PDF_EVENT_LOG
        .get_or_init(|| Mutex::new(VecDeque::new()))
        .lock()
        .unwrap();
    events.iter().cloned().collect()
}

const ANSI_RESET: &str = "\x1b[0m";
const ANSI_DIM: &str = "\x1b[2m";
const ANSI_TRACE: &str = "\x1b[90m";
const ANSI_DEBUG: &str = "\x1b[36m";
const ANSI_INFO: &str = "\x1b[32m";
const ANSI_WARN: &str = "\x1b[33m";
const ANSI_ERROR: &str = "\x1b[31m";
const ANSI_LAYER: &str = "\x1b[95m";

fn timestamp() -> String {
    chrono::Local::now().format("%H:%M:%S%.3f").to_string()
}

fn level_label(level: u8) -> &'static str {
    match level {
        0 | 1 => "INFO",
        2 => "DEBUG",
        3 => "TRACE",
        4 => "WARN",
        _ => "ERROR",
    }
}

fn level_color(label: &str) -> &'static str {
    match label {
        "TRACE" => ANSI_TRACE,
        "DEBUG" => ANSI_DEBUG,
        "INFO" => ANSI_INFO,
        "WARN" => ANSI_WARN,
        "ERROR" => ANSI_ERROR,
        _ => ANSI_INFO,
    }
}

fn layer_for_event(event: &str) -> &'static str {
    let event_lower = event.to_ascii_lowercase();
    if event_lower.contains("document") {
        "DOC"
    } else if event_lower.contains("pageasset") || event_lower.contains("bundle") {
        "ASSET"
    } else if event_lower.contains("preview") {
        "PREVIEW"
    } else if event_lower.contains("cache") {
        "CACHE"
    } else {
        "PDF"
    }
}

fn format_fields(fields: &[(&str, String)]) -> String {
    fields
        .iter()
        .map(|(key, value)| format!("{}={}", key, value))
        .collect::<Vec<_>>()
        .join(" ")
}

fn format_layered_line(level: &str, layer: &str, event: &str, fields: &[(&str, String)]) -> String {
    let field_text = format_fields(fields);
    let suffix = if field_text.is_empty() {
        String::new()
    } else {
        format!(" {}", field_text)
    };
    format!(
        "{}{}{} {}{:<5}{} {}[{:<8}]{} {}{}",
        ANSI_DIM,
        timestamp(),
        ANSI_RESET,
        level_color(level),
        level,
        ANSI_RESET,
        ANSI_LAYER,
        layer,
        ANSI_RESET,
        event,
        suffix
    )
}

fn format_plain_event_line(
    level: &str,
    layer: &str,
    event: &str,
    fields: &[(&str, String)],
) -> String {
    let field_text = format_fields(fields);
    let suffix = if field_text.is_empty() {
        String::new()
    } else {
        format!(" {}", field_text)
    };
    format!("{} [{}] {}{}", level, layer, event, suffix)
}

fn record_pdf_event(line: String) {
    let mut events = PDF_EVENT_LOG
        .get_or_init(|| Mutex::new(VecDeque::new()))
        .lock()
        .unwrap();
    if events.len() >= PDF_EVENT_LOG_LIMIT {
        events.pop_front();
    }
    events.push_back(line);
}

pub fn log_pdf_event(level: u8, event: &str, fields: &[(&str, String)]) {
    if get_pdf_log_level() < level {
        return;
    }
    let label = level_label(level);
    let layer = layer_for_event(event);
    record_pdf_event(format_plain_event_line(label, layer, event, fields));
    let line = format_layered_line(label, layer, event, fields);
    eprintln!("{}", line);
}

pub fn log_terminal_message(message: &str) {
    eprintln!("{}", message);
}

pub struct PdfEventSpan {
    level: u8,
    event: &'static str,
    fields: Vec<(&'static str, String)>,
    start: std::time::Instant,
    finished: bool,
}

impl PdfEventSpan {
    pub fn begin(level: u8, event: &'static str, fields: Vec<(&'static str, String)>) -> Self {
        let span = Self {
            level,
            event,
            fields,
            start: std::time::Instant::now(),
            finished: false,
        };
        let event_name = format!("{}.begin", span.event);
        log_pdf_event(span.level, &event_name, &span.fields);
        span
    }

    pub fn finish(mut self, result: &str, extra_fields: Vec<(&'static str, String)>) {
        self.finished = true;
        let mut fields = self.fields.clone();
        fields.push(("result", result.to_string()));
        fields.push(("elapsedMs", self.start.elapsed().as_millis().to_string()));
        fields.extend(extra_fields);
        let event_name = format!("{}.end", self.event);
        log_pdf_event(self.level, &event_name, &fields);
    }
}

impl Drop for PdfEventSpan {
    fn drop(&mut self) {
        if self.finished {
            return;
        }
        let mut fields = self.fields.clone();
        fields.push(("result", "aborted".to_string()));
        fields.push(("elapsedMs", self.start.elapsed().as_millis().to_string()));
        let event_name = format!("{}.end", self.event);
        log_pdf_event(self.level, &event_name, &fields);
    }
}

#[macro_export]
macro_rules! pdf_log {
    ($level:expr, $($arg:tt)*) => {{
        if $crate::infrastructure::pdf::log_service::get_pdf_log_level() >= $level {
            eprintln!($($arg)*);
        }
    }};
}

#[macro_export]
macro_rules! log_step {
    ($($arg:tt)*) => {
        $crate::pdf_log!(1, $($arg)*)
    };
}

#[macro_export]
macro_rules! log_audit {
    ($($arg:tt)*) => {
        $crate::pdf_log!(3, $($arg)*)
    };
}
pub struct ProfileSpan {
    name: &'static str,
    start: std::time::Instant,
}
impl ProfileSpan {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            start: std::time::Instant::now(),
        }
    }
}
impl Drop for ProfileSpan {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();
        if get_pdf_log_level() >= 1 {
            eprintln!("[PROF][SPAN] {} took {:?}", self.name, elapsed);
        }
    }
}

#[macro_export]
macro_rules! prof_span {
    ($name:expr) => {
        let _span = $crate::infrastructure::pdf::log_service::ProfileSpan::new($name);
    };
}

// --- Bridge to pdf-viewer-core unified trace system ---
//
// Adapter pattern: implements the core `TraceSubscriber` trait so events
// emitted from the pure-Rust core (via `trace::emit` / `trace_span!`) are
// routed through the existing colored terminal output + ring buffer.
// Install once at startup via `install_core_trace_bridge()`.

use pdf_viewer_core::common::trace::{TraceEvent, TraceLevel, TraceSubscriber};

/// Adapter that forwards core trace events into this module's log_pdf_event.
pub struct LogServiceSubscriber;

fn level_to_u8(level: TraceLevel) -> u8 {
    match level {
        TraceLevel::Info => 1,
        TraceLevel::Debug => 2,
        TraceLevel::Trace => 3,
    }
}

impl TraceSubscriber for LogServiceSubscriber {
    fn on_event(&self, event: &TraceEvent) {
        // node + action form the dotted event name (e.g. "doc.open.begin").
        let event_name = format!("{}.{}", event.node, event.action);
        // Borrow fields as (&str, String) for the existing log_pdf_event API.
        // The owned Strings live in `owned` for the duration of this call.
        let owned: Vec<String> = event.fields.iter().map(|f| f.key.to_string()).collect();
        let pairs: Vec<(&str, String)> = owned
            .iter()
            .zip(event.fields.iter().map(|f| f.value.clone()))
            .map(|(k, v)| (k.as_str(), v))
            .collect();
        log_pdf_event(level_to_u8(event.level), &event_name, &pairs);
    }
}

/// Install the bridge so core trace events flow into this log service.
/// Call once during app initialization. Idempotent.
pub fn install_core_trace_bridge() {
    pdf_viewer_core::common::trace::set_subscriber(Some(Box::new(LogServiceSubscriber)));
}
