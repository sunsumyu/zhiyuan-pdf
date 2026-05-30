use std::time::Instant;

pub mod app_state;
pub mod error;
pub mod infrastructure;
pub mod interfaces;
pub mod application;
pub mod state;

pub use app_state::{AppState, CacheStore, DocumentStore, HistoryStore, RendererState};
pub use error::{PdfError, PdfResult};

pub fn run() {
    let boot_start = Instant::now();
    eprintln!("[BOOT] Rust main() entered");

    let app_state = AppState::new();
    eprintln!("[BOOT] AppState::new completed in {:.2?}", boot_start.elapsed());

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .register_uri_scheme_protocol("pdfasset", |_ctx, request| {
            let url = request.uri().to_string();
            // On Windows Tauri maps custom scheme to http://<scheme>.localhost/
            let asset_id = url
                .strip_prefix("http://pdfasset.localhost/")
                .or_else(|| url.strip_prefix("https://pdfasset.localhost/"))
                .or_else(|| url.strip_prefix("pdfasset://localhost/"))
                .or_else(|| url.strip_prefix("pdfasset://"))
                .unwrap_or("");

            let cache = crate::infrastructure::pdf::cache::PDF_IMAGE_CACHE
                .lock()
                .unwrap();
            if let Some(data) = cache.get(asset_id) {
                let mime = if data.len() > 3 && data[0] == 0xFF && data[1] == 0xD8 {
                    "image/jpeg"
                } else {
                    "image/png"
                };
                tauri::http::Response::builder()
                    .status(200)
                    .header("Content-Type", mime)
                    .header("Access-Control-Allow-Origin", "*")
                    .body(data.to_vec())
                    .unwrap()
            } else {
                tauri::http::Response::builder()
                    .status(404)
                    .body(Vec::new())
                    .unwrap()
            }
        })
        // Debug 构建下自动打开 DevTools，方便从控制台调用 window.verifyEditorBugs()。
        .setup(move |app| {
            eprintln!("[BOOT] tauri::Builder setup() entered at {:.2?}", boot_start.elapsed());
            #[cfg(debug_assertions)]
            {
                use tauri::Manager;
                if let Some(window) = app.get_webview_window("main") {
                    window.open_devtools();
                }
            }
            let _ = app;
            eprintln!("[BOOT] tauri::Builder setup() completed at {:.2?}", boot_start.elapsed());
            Ok(())
        })
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            interfaces::pdf::open_pdf,
            interfaces::pdf::clear_cache,
            interfaces::pdf::read_preview,
            interfaces::pdf::read_vector,
            interfaces::pdf::save_pdf,
            interfaces::pdf::apply_region_patches,
            interfaces::pdf::undo,
            interfaces::pdf::redo,
            interfaces::pdf::find_in_page,
            interfaces::pdf::find_in_document,
            interfaces::pdf::read_annotation_targets,
            interfaces::pdf::read_highlights,
            interfaces::pdf::apply_highlight,
            interfaces::pdf::read_comments,
            interfaces::pdf::read_comment_review,
            interfaces::pdf::apply_comment,
            interfaces::pdf::delete_annotation,
            interfaces::pdf::apply_comment_update,
            interfaces::pdf::read_glyph_plan,
            interfaces::pdf::read_images,
            interfaces::pdf::diagnose_page,
            interfaces::pdf::create_demo_pdf,
            interfaces::pdf::set_log_level,
            interfaces::pdf::get_asset_url,
            interfaces::pdf::pick_file,
        ]);
    eprintln!("[BOOT] Tauri builder configured in {:.2?}", boot_start.elapsed());

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
