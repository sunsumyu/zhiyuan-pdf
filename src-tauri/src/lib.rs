pub mod app_state;
pub mod error;
pub mod infrastructure;
pub mod interfaces;
pub mod application;
pub mod state;

pub use app_state::{AppState, CacheStore, DocumentStore, HistoryStore, RendererState};
pub use error::{PdfError, PdfResult};

pub fn run() {
    let app_state = AppState::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        // Debug 构建下自动打开 DevTools，方便从控制台调用 window.verifyEditorBugs()。
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                use tauri::Manager;
                if let Some(window) = app.get_webview_window("main") {
                    window.open_devtools();
                }
            }
            let _ = app;
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
            interfaces::pdf::create_demo_pdf,
            interfaces::pdf::set_log_level,
            interfaces::pdf::get_asset_url,
            interfaces::pdf::pick_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
