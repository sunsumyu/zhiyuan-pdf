pub mod infrastructure;
pub mod interfaces;
pub mod application;
pub mod state;


pub struct AppState {
pub pdf_documents: std::sync::Mutex<std::collections::HashMap<String, std::sync::Arc<lopdf::Document>>>,
pub pdf_light_page_cache: std::sync::Mutex<std::collections::HashMap<String, std::sync::Arc<infrastructure::multimedia::pdf::models::LightPageModel>>>,
pub pdf_page_cache: std::sync::Mutex<std::collections::HashMap<String, std::sync::Arc<infrastructure::multimedia::pdf::models::VectorPageModel>>>,
pub pdf_layout_cache: std::sync::Mutex<std::collections::HashMap<String, std::sync::Arc<pdf_viewer_core::models::LayoutInferenceResult>>>,
pub read_document_meta_cache: std::sync::Mutex<std::collections::HashMap<String, infrastructure::multimedia::pdf_read::types::ReadDocumentMeta>>,
pub page_preview_cache: std::sync::Mutex<std::collections::HashMap<String, infrastructure::multimedia::pdf_read::types::PagePreview>>,
pub pdf_transactions: std::sync::Mutex<std::collections::HashMap<String, Vec<std::sync::Arc<lopdf::Document>>>>,
pub pdf_redo_transactions: std::sync::Mutex<std::collections::HashMap<String, Vec<std::sync::Arc<lopdf::Document>>>>,
pub loading_docs: std::sync::Mutex<std::collections::HashMap<String, state::LoadingStatus>>,
pub vello_renderer: std::sync::Mutex<Option<std::sync::Arc<std::sync::Mutex<crate::infrastructure::multimedia::pdf::vello_renderer::VelloRenderer>>>>,
pub pdf_materialization_reports: std::sync::Mutex<std::collections::HashMap<String, infrastructure::multimedia::pdf::models::PdfMaterializationReport>>,
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
            interfaces::multimedia::pdf::read_metadata,
            interfaces::multimedia::pdf::open_pdf,
            interfaces::multimedia::pdf::read_pdf,
            interfaces::multimedia::pdf::probe_pdf,
            interfaces::multimedia::pdf::clear_cache,
            interfaces::multimedia::pdf::read_preview,
            interfaces::multimedia::pdf::prefetch_preview,
            interfaces::multimedia::pdf::read_page_info,
            interfaces::multimedia::pdf::read_vector,
            interfaces::multimedia::pdf::resolve_layout,
            interfaces::multimedia::pdf::save_pdf,
            interfaces::multimedia::pdf::read_materialization_report,
            interfaces::multimedia::pdf::apply_text_patches,
            interfaces::multimedia::pdf::apply_region_patches,
            interfaces::multimedia::pdf::undo,
            interfaces::multimedia::pdf::redo,
            interfaces::multimedia::pdf::find_in_page,
            interfaces::multimedia::pdf::find_in_document,
            interfaces::multimedia::pdf::read_annotation_targets,
            interfaces::multimedia::pdf::read_highlights,
            interfaces::multimedia::pdf::apply_highlight,
            interfaces::multimedia::pdf::read_comments,
            interfaces::multimedia::pdf::read_comment_review,
            interfaces::multimedia::pdf::apply_comment,
            interfaces::multimedia::pdf::delete_annotation,
            interfaces::multimedia::pdf::apply_comment_update,
            interfaces::multimedia::pdf::apply_batch_replace,
            interfaces::multimedia::pdf::apply_replace,
            interfaces::multimedia::pdf::read_glyph_plan,
            interfaces::multimedia::pdf::read_images,
            interfaces::multimedia::pdf::resolve_caret,
            interfaces::multimedia::pdf::resolve_hit,
            interfaces::multimedia::pdf::resolve_hit_target,
            interfaces::multimedia::pdf::resolve_projection,
            interfaces::multimedia::pdf::resolve_params,
            interfaces::multimedia::pdf::create_demo_pdf,
            interfaces::multimedia::pdf::render_tile,
            interfaces::multimedia::pdf::set_log_level,
            interfaces::multimedia::pdf::get_asset_url,
            interfaces::multimedia::pdf::pick_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
