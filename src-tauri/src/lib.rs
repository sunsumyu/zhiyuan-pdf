pub mod infrastructure;
pub mod interfaces;
pub mod application;
pub mod state;


pub struct AppState {
pub pdf_documents: std::sync::Mutex<std::collections::HashMap<String, std::sync::Arc<lopdf::Document>>>,
pub pdf_light_page_cache: std::sync::Mutex<std::collections::HashMap<String, std::sync::Arc<infrastructure::pdf::models::LightPageModel>>>,
pub pdf_page_cache: std::sync::Mutex<std::collections::HashMap<String, std::sync::Arc<infrastructure::pdf::models::VectorPageModel>>>,
pub pdf_layout_cache: std::sync::Mutex<std::collections::HashMap<String, std::sync::Arc<pdf_viewer_core::models::LayoutInferenceResult>>>,
pub read_document_meta_cache: std::sync::Mutex<std::collections::HashMap<String, infrastructure::pdf_read::types::ReadDocumentMeta>>,
pub page_preview_cache: std::sync::Mutex<std::collections::HashMap<String, infrastructure::pdf_read::types::PagePreview>>,
pub pdf_transactions: std::sync::Mutex<std::collections::HashMap<String, Vec<std::sync::Arc<lopdf::Document>>>>,
pub pdf_redo_transactions: std::sync::Mutex<std::collections::HashMap<String, Vec<std::sync::Arc<lopdf::Document>>>>,
pub loading_docs: std::sync::Mutex<std::collections::HashMap<String, state::LoadingStatus>>,
pub vello_renderer: std::sync::Mutex<Option<std::sync::Arc<std::sync::Mutex<crate::infrastructure::pdf::vello_renderer::VelloRenderer>>>>,
pub pdf_materialization_reports: std::sync::Mutex<std::collections::HashMap<String, infrastructure::pdf::models::PdfMaterializationReport>>,
}
pub fn run() {
    let app_state = AppState {
        pdf_documents: std::sync::Mutex::new(std::collections::HashMap::new()),
        pdf_light_page_cache: std::sync::Mutex::new(std::collections::HashMap::new()),
        pdf_page_cache: std::sync::Mutex::new(std::collections::HashMap::new()),
        pdf_layout_cache: std::sync::Mutex::new(std::collections::HashMap::new()),
        read_document_meta_cache: std::sync::Mutex::new(std::collections::HashMap::new()),
        page_preview_cache: std::sync::Mutex::new(std::collections::HashMap::new()),
        pdf_transactions: std::sync::Mutex::new(std::collections::HashMap::new()),
        pdf_redo_transactions: std::sync::Mutex::new(std::collections::HashMap::new()),
        loading_docs: std::sync::Mutex::new(std::collections::HashMap::new()),
vello_renderer: std::sync::Mutex::new(None),
        pdf_materialization_reports: std::sync::Mutex::new(std::collections::HashMap::new()),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            interfaces::pdf::read_metadata,
            interfaces::pdf::open_pdf,
            interfaces::pdf::read_pdf,
            interfaces::pdf::probe_pdf,
            interfaces::pdf::clear_cache,
            interfaces::pdf::read_preview,
            interfaces::pdf::prefetch_preview,
            interfaces::pdf::read_page_info,
            interfaces::pdf::read_vector,
            interfaces::pdf::resolve_layout,
            interfaces::pdf::save_pdf,
            interfaces::pdf::read_materialization_report,
            interfaces::pdf::apply_text_patches,
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
            interfaces::pdf::apply_batch_replace,
            interfaces::pdf::apply_replace,
            interfaces::pdf::read_glyph_plan,
            interfaces::pdf::read_images,
            interfaces::pdf::resolve_caret,
            interfaces::pdf::resolve_hit,
            interfaces::pdf::resolve_hit_target,
            interfaces::pdf::resolve_projection,
            interfaces::pdf::resolve_params,
            interfaces::pdf::create_demo_pdf,
            interfaces::pdf::render_tile,
            interfaces::pdf::set_log_level,
            interfaces::pdf::get_asset_url,
            interfaces::pdf::pick_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
