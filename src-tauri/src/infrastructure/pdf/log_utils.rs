use std::sync::atomic::{AtomicU8, Ordering};

/// Logging levels
/// 0: SILENT (No logs)
/// 1: INFO (Key steps only)
/// 2: DEBUG (Detailed steps)
/// 3: TRACE (Audit logs, font metrics, etc.)
pub static PDF_LOG_LEVEL: AtomicU8 = AtomicU8::new(1);
pub fn set_pdf_log_level(level: u8) {
    PDF_LOG_LEVEL.store(level, Ordering::SeqCst);
}
pub fn get_pdf_log_level() -> u8 {
    PDF_LOG_LEVEL.load(Ordering::SeqCst)
}

#[macro_export]
macro_rules! pdf_log {
    ($level:expr, $($arg:tt)*) => {{
        if $crate::infrastructure::pdf::log_utils::get_pdf_log_level() >= $level {
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
        let _span = $crate::infrastructure::pdf::log_utils::ProfileSpan::new($name);
    };
}
