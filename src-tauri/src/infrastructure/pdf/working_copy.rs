use lazy_static::lazy_static;
use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};

lazy_static! {
    static ref WORKING_COPIES: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
    static ref COPY_LOCKS: Mutex<HashMap<String, Arc<Mutex<()>>>> = Mutex::new(HashMap::new());
}

pub(crate) fn resolve_working_path(original_path: &str) -> String {
    let total_start = std::time::Instant::now();
    let (working_path, lock) = {
        let mut copies = WORKING_COPIES.lock().unwrap();
        let mut locks = COPY_LOCKS.lock().unwrap();

        let digest = md5::compute(original_path);
        let hashed_name = format!("{:x}.pdf", digest);
        let wp = std::env::temp_dir()
            .join(format!("working_{}", hashed_name))
            .to_string_lossy()
            .to_string();

        copies
            .entry(original_path.to_string())
            .or_insert(wp.clone());
        let l = locks
            .entry(original_path.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone();
        (wp, l)
    };
    let _locked = lock.lock().unwrap();
    let elapsed = total_start.elapsed();
    crate::pdf_log!(
        2,
        "[WORKING_COPY] resolve_working_path({}) = {} ({}ms)",
        original_path,
        working_path,
        elapsed.as_millis()
    );
    working_path
}

pub(crate) fn release_working_copy(path: &str) {
    let working_path = {
        let mut copies = WORKING_COPIES.lock().unwrap();
        copies.remove(path)
    };
    {
        let mut locks = COPY_LOCKS.lock().unwrap();
        locks.remove(path);
    }
    if let Some(working_path) = working_path {
        let _ = fs::remove_file(&working_path);
        crate::log_step!("[PDF][Release] Removed working copy for {}", path);
    }
}
pub(crate) fn clear_working_copies() {
    let mut copies = WORKING_COPIES.lock().unwrap();
    copies.clear();
    let mut locks = COPY_LOCKS.lock().unwrap();
    locks.clear();
}
