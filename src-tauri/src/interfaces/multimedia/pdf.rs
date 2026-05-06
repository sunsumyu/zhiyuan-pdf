use crate::application::pdf::comment_review::{PdfCommentReviewRequest, PdfCommentReviewResult};
use crate::application::pdf::page_annotation::{
    PdfDeleteAnnotationRequest, PdfDeleteAnnotationResult, PdfPageAnnotationTargetResult,
    PdfPageCommentList, PdfPageHighlightList, PdfRegionCommentRequest, PdfRegionCommentResult,
    PdfRegionHighlightRequest, PdfRegionHighlightResult, PdfUpdateCommentRequest,
    PdfUpdateCommentResult,
};
use crate::application::pdf::page_replace::{
    PdfDocumentReplaceRequest, PdfDocumentReplaceResult, PdfRegionReplaceRequest,
    PdfRegionReplaceResult,
};
use crate::application::pdf::page_search::{
    PdfDocumentSearchResult, PdfPageSearchRequest, PdfPageSearchResult,
};
use crate::infrastructure::multimedia::pdf::commands::PdfEditCommand;
use crate::infrastructure::multimedia::pdf::engine::{PdfDocumentService, PdfEditorGeometryService, PdfPageModelService};
use crate::infrastructure::multimedia::pdf_read::facade::PdfReadFacade;
use crate::infrastructure::multimedia::pdf_read::types::{PagePreview, ReadDocumentMeta};
use crate::infrastructure::multimedia::pdf::models::{
    GlyphPaintPlan, LayoutInferenceResult, LightPageModel, PdfMaterializationReport, PdfMetadata,
    PdfModifications, VectorPageModel, RenderObject, NativeTextModel,
};
use crate::log_step;
use crate::pdf_log;
use pdf_viewer_core::persistence_models::PersistableRegionPatch;
// Removed unused base64 imports
use tauri::command;
pub(crate) async fn ensure_document_loaded(
    app_state: &crate::AppState,
    path: &str,
) -> Result<(), String> {
    {
        let cache = app_state.pdf_documents.lock().unwrap();
        if cache.contains_key(path) {
            return Ok(());
        }
    }

    let working_path = PdfDocumentService::get_working_path(path);
    let path_for_load = path.to_string();
    let loaded_doc = tokio::task::spawn_blocking(move || {
        lopdf::Document::load(&working_path)
            .map(std::sync::Arc::new)
            .map_err(|e| format!("Lopdf Load Error for {}: {}", path_for_load, e))
    })
    .await
    .map_err(|e| e.to_string())??;

    let mut cache = app_state.pdf_documents.lock().unwrap();
    cache.insert(path.to_string(), loaded_doc);
    Ok(())
}

/// 灏嗗彲鎸佷箙鍖栫殑鍖哄煙琛ヤ竵搴旂敤鍒版寚瀹氱殑 PDF 鏂囦欢涓€?
///
/// 璇ュ嚱鏁版帴鏀朵竴缁勫尯鍩熻ˉ涓侊紝鏋勫缓鏉愭枡鍖栬鍒掍互纭畾鏈夋晥鐨勬枃鏈噸鎺掓搷浣滐紝
/// 骞跺皢杩欎簺鎿嶄綔杞崲涓?PDF 缂栬緫鍛戒护鎵ц銆?
///
/// # 鍙傛暟
///
/// * `app_state` - 搴旂敤绋嬪簭鐘舵€佸紩鐢紝鐢ㄤ簬璁块棶蹇呰鐨勪笂涓嬫枃鍜岃祫婧愩€?
/// * [path](file://e:\chain\nushell-enhanced\src\types.ts#L20-L20) - 鐩爣 PDF 鏂囦欢鐨勮矾寰勩€?
/// * `page_index` - 闇€瑕佸簲鐢ㄨˉ涓佺殑椤甸潰绱㈠紩銆?
/// * `patches` - 鍖呭惈寰呭簲鐢ㄦ洿鏀圭殑鍙寔涔呭寲鍖哄煙琛ヤ竵鍚戦噺銆?
///
/// # 杩斿洖鍊?
///
/// 濡傛灉鎿嶄綔鎴愬姛锛岃繑鍥?`Ok(())`锛涘鏋滃彂鐢熼敊璇紝杩斿洖鍖呭惈閿欒淇℃伅鐨?`Err(String)`銆
pub(crate) async fn execute_region_patches(
    app_state: &crate::AppState,
    path: String,
    page_index: u16,
    patches: Vec<PersistableRegionPatch>,
) -> Result<(), String> {
use crate::infrastructure::multimedia::pdf::region_materializer::build_region_materialization_plan;

    // 璁板綍姣忎釜杈撳叆琛ヤ竵鐨勮缁嗕俊鎭互渚胯皟璇曞拰杩借釜
    for patch in &patches {
        log_step!(
            "[V3-SAVE-CMD][patch] page={} region={} source={} targets={:?} text='{}'",
            patch.page_index,
            patch.region_id,
            patch.source,
            patch.target_indices,
            truncate_for_log(&patch.new_text, 64)
        );
    }

    // 鏋勫缓鍖哄煙鏉愭枡鍖栬鍒掞紝璁＄畻鏈夋晥鐨勬枃鏈噸鎺掓搷浣?
    let materialization_plan = build_region_materialization_plan(&patches, &[]);
    let mut commands: Vec<Box<dyn PdfEditCommand>> = Vec::new();

    // 灏嗘湁鏁堢殑鏂囨湰閲嶆帓鎿嶄綔杞崲涓烘壒閲?PDF 缂栬緫鍛戒护
    if !materialization_plan.effective_text_reflows.is_empty() {
        commands.push(Box::new(
            crate::infrastructure::multimedia::pdf::commands::BatchTextReflowCommand {
                patches: materialization_plan.effective_text_reflows,
            },
        ));
    }

    execute_pdf_commands_with_app_state(app_state, path, page_index, commands).await
}
pub(crate) async fn apply_highlight_annotation(
    app_state: &crate::AppState,
    path: String,
    page_index: u16,
    rect: [f32; 4],
    color: [f32; 3],
) -> Result<(), String> {
    let commands: Vec<Box<dyn PdfEditCommand>> = vec![Box::new(
        crate::infrastructure::multimedia::pdf::commands::AddHighlightCommand {
            page_num: (page_index + 1) as u32,
            rect,
            color,
        },
    )];

    execute_pdf_commands_with_app_state(app_state, path, page_index, commands).await
}
pub(crate) async fn apply_text_comment(
    app_state: &crate::AppState,
    path: String,
    page_index: u16,
    rect: [f32; 4],
    color: [f32; 3],
    contents: String,
) -> Result<(), String> {
    let commands: Vec<Box<dyn PdfEditCommand>> = vec![Box::new(
        crate::infrastructure::multimedia::pdf::commands::AddCommentCommand {
            page_num: (page_index + 1) as u32,
            rect,
            color,
            contents,
        },
    )];

    execute_pdf_commands_with_app_state(app_state, path, page_index, commands).await
}
pub(crate) async fn delete_annotation_internal(
    app_state: &crate::AppState,
    path: String,
    page_index: u16,
    annot_id: (u32, u16),
) -> Result<(), String> {
    let commands: Vec<Box<dyn PdfEditCommand>> = vec![Box::new(
        crate::infrastructure::multimedia::pdf::commands::DeleteAnnotationCommand {
            page_num: (page_index + 1) as u32,
            annot_id,
        },
    )];

    execute_pdf_commands_with_app_state(app_state, path, page_index, commands).await
}
pub(crate) async fn update_text_comment(
    app_state: &crate::AppState,
    path: String,
    page_index: u16,
    annot_id: (u32, u16),
    contents: String,
) -> Result<(), String> {
    let commands: Vec<Box<dyn PdfEditCommand>> = vec![Box::new(
        crate::infrastructure::multimedia::pdf::commands::UpdateCommentCommand {
            page_num: (page_index + 1) as u32,
            annot_id,
            contents,
        },
    )];

    execute_pdf_commands_with_app_state(app_state, path, page_index, commands).await
}
/*
pub(crate) async fn validate_patch_writable(
    app_state: &crate::AppState,
    path: &str,
    patch: &PersistableRegionPatch,
) -> Result<(), String> {
use crate::infrastructure::multimedia::pdf::region_materializer::build_region_materialization_plan;

    let doc = {
        let docs = app_state.pdf_documents.lock().unwrap();
        docs.get(path)
            .cloned()
            .ok_or_else(|| "Document not found in cache".to_string())?
    };
    let materialization_plan = build_region_materialization_plan(&[patch.clone()], &[]);
    tokio::task::spawn_blocking(move || {
        let mut doc_clone = (*doc).clone();
        for reflow in materialization_plan.effective_text_reflows {
            doc_clone
                .apply_atomic_reflow_to_doc(
                    (reflow.page_index + 1) as u32,
                    &reflow.target_indices,
                    &reflow.new_text,
                    reflow.new_runs.clone(),
                    reflow.displacement_y,
                    reflow.wrap_width,
                    reflow.alignment,
                    reflow.line_height,
                    reflow.char_spacing,
                    reflow.horizontal_scaling,
                )
                .map_err(|e| format!("PersistableRegionPatch Error (Atomic): {}", e))?;
        }
        Ok(())
    })
    .await
    .map_err(|err| err.to_string())?
}
*/
fn truncate_for_log(value: &str, limit: usize) -> String {
    let mut chars = value.chars();
    let mut out = String::new();
    for _ in 0..limit {
        if let Some(ch) = chars.next() {
            out.push(ch);
        } else {
            break;
        }
    }
    if chars.next().is_some() {
        out.push_str("...");
    }
    out
}

#[command]
pub async fn read_metadata(
    state: tauri::State<'_, crate::AppState>,
    path: String,
) -> Result<PdfMetadata, String> {
    PdfPageModelService::get_pdf_metadata(state, &path).await
}

#[command]
pub async fn open_pdf(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, crate::AppState>,
    path: String,
) -> Result<usize, String> {
    PdfDocumentService::open_pdf(app_handle, state, &path).await
}

#[command]
pub fn clear_cache(state: tauri::State<'_, crate::AppState>) -> Result<(), String> {
    PdfDocumentService::release_all_pdf_resources(&state);
    {
        let mut cache = state.read_document_meta_cache.lock().unwrap();
        cache.clear();
    }
    {
        let mut cache = state.page_preview_cache.lock().unwrap();
        cache.clear();
    }
    Ok(())
}

#[command]
pub async fn read_pdf(
    state: tauri::State<'_, crate::AppState>,
    path: String,
) -> Result<ReadDocumentMeta, String> {
    let total_start = std::time::Instant::now();
    if let Some(meta) = state
        .read_document_meta_cache
        .lock()
        .unwrap()
        .get(&path)
        .cloned()
    {
        log_step!(
            "[PDF-READ][cmd][open] cache_hit=true total={:?} pages={} path={}",
            total_start.elapsed(),
            meta.page_count,
            path
        );
        return Ok(meta);
    }

    let path_for_task = path.clone();
    pdf_log!(2, "[PDF-READ][cmd][open][detail] spawn path={}", path);
    let meta = tokio::task::spawn_blocking(move || {
        let facade = PdfReadFacade::new();
        facade.open(&path_for_task)
    })
    .await
    .map_err(|e: tokio::task::JoinError| e.to_string())??;

    state
        .read_document_meta_cache
        .lock()
        .unwrap()
        .insert(path.clone(), meta.clone());
    log_step!(
        "[PDF-READ][cmd][open] cache_hit=false total={:?} pages={} kind={:?} path={}",
        total_start.elapsed(),
        meta.page_count,
        meta.kind,
        path
    );
    Ok(meta)
}

#[command]
pub async fn probe_pdf(
    state: tauri::State<'_, crate::AppState>,
    path: String,
) -> Result<ReadDocumentMeta, String> {
    let total_start = std::time::Instant::now();
    if let Some(meta) = state
        .read_document_meta_cache
        .lock()
        .unwrap()
        .get(&path)
        .cloned()
    {
        log_step!(
            "[PDF-READ][cmd][probe] cache_hit=true total={:?} pages={} kind={:?} path={}",
            total_start.elapsed(),
            meta.page_count,
            meta.kind,
            path
        );
        return Ok(meta);
    }

    let path_for_task = path.clone();
    let meta = tokio::task::spawn_blocking(move || {
        let facade = PdfReadFacade::new();
        facade.probe_kind_fast(&path_for_task)
    })
    .await
    .map_err(|e: tokio::task::JoinError| e.to_string())??;

    state
        .read_document_meta_cache
        .lock()
        .unwrap()
        .insert(path.clone(), meta.clone());

    log_step!(
        "[PDF-READ][cmd][probe] cache_hit=false total={:?} pages={} kind={:?} path={}",
        total_start.elapsed(),
        meta.page_count,
        meta.kind,
        path
    );
    Ok(meta)
}

#[command]
pub async fn read_preview(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    page_index: u16,
) -> Result<PagePreview, String> {
    let total_start = std::time::Instant::now();
    let cache_key = format!("{}::{}", path, page_index);
    if let Some(preview) = state
        .page_preview_cache
        .lock()
        .unwrap()
        .get(&cache_key)
        .cloned()
    {
        log_step!(
            "[PDF-READ][cmd][page] cache_hit=true page={} ready={} total={:?} path={}",
            page_index,
            preview.ready,
            total_start.elapsed(),
            path
        );
        return Ok(preview);
    }

    let path_for_task = path.clone();
    pdf_log!(
        2,
        "[PDF-READ][cmd][page][detail] spawn page={} path={}",
        page_index,
        path
    );
    let preview = tokio::task::spawn_blocking(move || {
        let facade = PdfReadFacade::new();
        facade.get_page_preview(&path_for_task, page_index)
    })
    .await
    .map_err(|e: tokio::task::JoinError| e.to_string())??;

    state
        .page_preview_cache
        .lock()
        .unwrap()
        .insert(cache_key, preview.clone());
    log_step!(
        "[PDF-READ][cmd][page] cache_hit=false page={} ready={} total={:?} width={} height={} path={}",
        page_index,
        preview.ready,
        total_start.elapsed(),
        preview.width,
        preview.height,
        path
    );
    Ok(preview)
}

#[command]
pub async fn prefetch_preview(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    page_index: u16,
) -> Result<(), String> {
    let cache_key = format!("{}::{}", path, page_index);
    if state
        .page_preview_cache
        .lock()
        .unwrap()
        .contains_key(&cache_key)
    {
        pdf_log!(
            2,
            "[PDF-READ][cmd][prefetch][detail] cache-hit page={} path={}",
            page_index,
            path
        );
        return Ok(());
    }

    let path_for_task = path.clone();
    let preview = tokio::task::spawn_blocking(move || {
        let facade = PdfReadFacade::new();
        facade.get_page_preview(&path_for_task, page_index)
    })
    .await
    .map_err(|e: tokio::task::JoinError| e.to_string())??;

    state
        .page_preview_cache
        .lock()
        .unwrap()
        .insert(cache_key, preview.clone());

    log_step!(
        "[PDF-READ][cmd][prefetch] page={} ready={} width={} height={} path={}",
        page_index,
        preview.ready,
        preview.width,
        preview.height,
        path
    );
    Ok(())
}

#[command]
pub async fn save_pdf(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    modifications: PdfModifications,
) -> Result<(), String> {
    PdfDocumentService::save_pdf(state, &path, modifications).await
}

#[command]
pub fn read_materialization_report(
    state: tauri::State<'_, crate::AppState>,
    path: String,
) -> Result<Option<PdfMaterializationReport>, String> {
    PdfDocumentService::read_last_pdf_materialization_report(state, &path)
}

#[command]
/// 鎻愪氦瀵?PDF 鏂囦欢鐨勭紪杈戝唴瀹广€?
///
/// 璇ュ嚱鏁版帴鏀朵竴绯诲垪鏂囨湰琛ヤ竵锛坧atches锛夛紝灏嗗叾杞崲涓?PDF 缂栬緫鍛戒护锛?
/// 骞跺紓姝ユ墽琛岃繖浜涘懡浠や互鏇存柊鎸囧畾璺緞涓嬬殑 PDF 鏂囦欢銆?
///
/// # 鍙傛暟
///
/// * `state` - Tauri 搴旂敤鐨勭姸鎬佺鐞嗗櫒锛岀敤浜庤闂簲鐢ㄤ笂涓嬫枃鍜岃祫婧愩€?
/// * `path` - 鐩爣 PDF 鏂囦欢鐨勬枃浠剁郴缁熻矾寰勩€?
/// * `page_index` - 闇€瑕佸簲鐢ㄧ紪杈戠殑椤甸潰绱㈠紩锛堜粠 0 寮€濮嬶級銆?
/// * `patches` - 鍖呭惈鏂囨湰鏇挎崲淇℃伅鐨勮ˉ涓佸垪琛紝姣忎釜琛ヤ竵鎻忚堪浜嗛渶瑕佷慨鏀圭殑鏂囨湰鍐呭銆?
pub async fn apply_text_patches(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    page_index: u16,
    patches: Vec<crate::infrastructure::multimedia::pdf::models::TextPatch>,
) -> Result<(), String> {
    println!(
        ">>>>> [ENTRY] commit_document_edits | path={} | count={}",
        path,
        patches.len()
    );
use crate::log_step;
    log_step!(
        "[PDF-SAVE-CMD] Received commit_document_edits: path={} page={} patches={}",
        path,
        page_index,
        patches.len()
    );
use crate::infrastructure::multimedia::pdf::commands::PdfEditCommand;
use crate::infrastructure::multimedia::pdf::commands::ReplaceTextCommand;

    // 灏嗘枃鏈ˉ涓佽浆鎹负鍙墽琛岀殑 PDF 缂栬緫鍛戒护瀵硅薄
    let mut commands: Vec<Box<dyn PdfEditCommand>> = Vec::new();
    for patch in patches {
        commands.push(Box::new(ReplaceTextCommand { patch }));
    }

    execute_pdf_commands_with_app_state(&state, path, page_index, commands).await
}

#[command]
pub async fn apply_region_patches(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    page_index: u16,
    patches: Vec<PersistableRegionPatch>,
) -> Result<(), String> {
    println!(
        ">>>>> [ENTRY] apply_region_patches | path={} | count={}",
        path,
        patches.len()
    );
use crate::log_step;
    log_step!(
        "[V3-SAVE-CMD] Applying region patches: path={} page={} count={}",
        path,
        page_index,
        patches.len()
    );
    execute_region_patches(&state, path, page_index, patches).await
}

async fn execute_pdf_commands(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    page_index: u16,
    commands: Vec<Box<dyn PdfEditCommand>>,
) -> Result<(), String> {
    execute_pdf_commands_with_app_state(&state, path, page_index, commands).await
}

async fn execute_pdf_commands_with_app_state(
    app_state: &crate::AppState,
    path: String,
    page_index: u16,
    commands: Vec<Box<dyn PdfEditCommand>>,
) -> Result<(), String> {
    println!(
        ">>>>> [CORE] execute_pdf_commands | path={} | cmd_count={}",
        path,
        commands.len()
    );
    let save_path = path.clone();

    // 1. Manage Transaction History for Undo
    {
        let docs = app_state.pdf_documents.lock().unwrap();
        let mut txs = app_state.pdf_transactions.lock().unwrap();
        if let Some(current_doc) = docs.get(&save_path) {
            let history = txs.entry(save_path.clone()).or_insert_with(Vec::new);
            history.push(current_doc.clone());
            if history.len() > 20 {
                history.remove(0);
            } // Cap history
            log_step!(
                "[PDF-SAVE] Pushed current doc to history. Stack size: {}",
                history.len()
            );
        }
    }
    {
        let mut redo = app_state.pdf_redo_transactions.lock().unwrap();
        redo.remove(&save_path);
    }

    // 2. Apply commands to in-memory clone
    let mut new_doc = {
        let docs = app_state.pdf_documents.lock().unwrap();
        let current_doc = docs
            .get(&save_path)
            .ok_or_else(|| "Document not found in cache".to_string())?;
        let doc_clone = (**current_doc).clone();
        crate::infrastructure::multimedia::pdf::save_engine::apply_pdf_commands(
            doc_clone, page_index, commands,
        )?
    };

    // 3. Save to disk
    new_doc
        .save(&save_path)
        .map_err(|e| format!("Disk Save Failure: {}", e))?;
    log_step!("[PDF-SAVE] Saved to disk: {}", save_path);

    // 4. Update memory cache and invalidate view caches
    {
        let mut docs = app_state.pdf_documents.lock().unwrap();
        docs.insert(save_path.clone(), std::sync::Arc::new(new_doc));
    }

    let light_prefix = format!("light::{}::", save_path);
    let mut light_page_cache = app_state.pdf_light_page_cache.lock().unwrap();
    light_page_cache.retain(|key, _| !key.starts_with(&light_prefix));
    drop(light_page_cache);

    let prefix = format!("{}::", save_path);
    let mut page_cache = app_state.pdf_page_cache.lock().unwrap();
    page_cache.retain(|key, _| !key.starts_with(&prefix));
    drop(page_cache);

    let mut layout_cache = app_state.pdf_layout_cache.lock().unwrap();
    layout_cache.retain(|key: &String, _| !key.starts_with(&prefix));
    drop(layout_cache);

    Ok(())
}

#[command]
pub async fn undo(
    state: tauri::State<'_, crate::AppState>,
    path: String,
) -> Result<(), String> {
    PdfDocumentService::rollback_pdf(state, &path).await
}

#[command]
pub async fn redo(
    state: tauri::State<'_, crate::AppState>,
    path: String,
) -> Result<(), String> {
    PdfDocumentService::redo_pdf(state, &path).await
}

#[command]
pub async fn read_page_info(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    page_index: u16,
) -> Result<LightPageModel, String> {
    PdfPageModelService::get_light_page_model(state, path, page_index).await
}

#[command]
pub async fn read_vector(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    page_index: u16,
    target_zoom: Option<f32>,
) -> Result<VectorPageModel, String> {
    PdfPageModelService::get_vector_page_model(state, path, page_index, target_zoom.unwrap_or(1.0)).await
}

#[command]
pub async fn find_in_page(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    page_index: u16,
    query: String,
    case_sensitive: Option<bool>,
) -> Result<PdfPageSearchResult, String> {
    let page_model = PdfPageModelService::get_vector_page_model_from_app_state(&state, path, page_index, 1.0).await?;
    Ok(crate::application::pdf::page_search::search_page_regions(
        &page_model,
        &PdfPageSearchRequest {
            query,
            case_sensitive: case_sensitive.unwrap_or(false),
        },
    ))
}

#[command]
pub async fn find_in_document(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    page_count: usize,
    query: String,
    case_sensitive: Option<bool>,
) -> Result<PdfDocumentSearchResult, String> {
    let request = PdfPageSearchRequest {
        query,
        case_sensitive: case_sensitive.unwrap_or(false),
    };
    let mut page_models = Vec::with_capacity(page_count);
    for page_index in 0..page_count {
        let page_model = PdfPageModelService::get_vector_page_model_from_app_state(
            &state,
            path.clone(),
            page_index as u16,
            1.0,
        )
        .await?;
        page_models.push(page_model);
    }
    Ok(crate::application::pdf::page_search::search_document_regions(&page_models, &request))
}

#[command]
pub async fn read_annotation_targets(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    page_index: u16,
) -> Result<PdfPageAnnotationTargetResult, String> {
    crate::application::pdf::page_annotation::list_page_annotation_targets(&state, &path, page_index).await
}

#[command]
pub async fn read_highlights(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    page_index: u16,
) -> Result<PdfPageHighlightList, String> {
    crate::application::pdf::page_annotation::list_page_highlights(&state, &path, page_index).await
}

#[command]
pub async fn apply_highlight(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    request: PdfRegionHighlightRequest,
) -> Result<PdfRegionHighlightResult, String> {
    crate::application::pdf::page_annotation::add_region_highlight(&state, &path, &request).await
}

#[command]
pub async fn read_comments(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    page_index: u16,
) -> Result<PdfPageCommentList, String> {
    crate::application::pdf::page_annotation::list_page_comments(&state, &path, page_index).await
}

#[command]
pub async fn read_comment_review(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    request: PdfCommentReviewRequest,
) -> Result<PdfCommentReviewResult, String> {
    crate::application::pdf::comment_review::review_document_comments(&state, &path, &request).await
}

#[command]
pub async fn apply_comment(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    request: PdfRegionCommentRequest,
) -> Result<PdfRegionCommentResult, String> {
    crate::application::pdf::page_annotation::add_region_comment(&state, &path, &request).await
}

#[command]
pub async fn delete_annotation(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    request: PdfDeleteAnnotationRequest,
) -> Result<PdfDeleteAnnotationResult, String> {
    crate::application::pdf::page_annotation::delete_page_annotation(&state, &path, &request).await
}

#[command]
pub async fn apply_comment_update(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    request: PdfUpdateCommentRequest,
) -> Result<PdfUpdateCommentResult, String> {
    crate::application::pdf::page_annotation::update_page_comment(&state, &path, &request).await
}

#[command]
pub async fn apply_batch_replace(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    page_count: usize,
    query: String,
    replacement: String,
    case_sensitive: Option<bool>,
) -> Result<PdfDocumentReplaceResult, String> {
    crate::application::pdf::page_replace::replace_document_regions(
        &state,
        &path,
        page_count,
        &PdfDocumentReplaceRequest {
            query,
            replacement,
            case_sensitive: case_sensitive.unwrap_or(false),
        },
    )
    .await
}

#[command]
pub async fn apply_replace(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    page_index: u16,
    region_id: String,
    kind: String,
    original_text: String,
    query: String,
    replacement: String,
    case_sensitive: Option<bool>,
) -> Result<PdfRegionReplaceResult, String> {
    crate::application::pdf::page_replace::replace_region_match(
        &state,
        &path,
        &PdfRegionReplaceRequest {
            page_index,
            region_id,
            kind,
            original_text,
            query,
            replacement,
            case_sensitive: case_sensitive.unwrap_or(false),
        },
    )
    .await
}

#[command]
pub async fn resolve_layout(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    page_index: u16,
) -> Result<LayoutInferenceResult, String> {
    PdfEditorGeometryService::get_layout_inference(state, path, page_index).await
}

#[command]
pub async fn read_glyph_plan(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    page_index: u16,
) -> Result<GlyphPaintPlan, String> {
    PdfEditorGeometryService::get_glyph_paint_plan(state, path, page_index).await
}

#[command]
pub fn read_images(
    path: String,
) -> Result<std::collections::HashMap<String, String>, String> {
    Ok(PdfEditorGeometryService::get_image_cache(&path))
}

#[command]
pub fn resolve_caret(
    session: pdf_viewer_core::models::EditorSession,
    click_x_from_anchor_left: f32,
) -> Result<usize, String> {
    PdfEditorGeometryService::resolve_editor_caret_index(session, click_x_from_anchor_left)
}

#[command]
pub fn resolve_hit(
    request: pdf_viewer_core::models::FieldHitRequest,
) -> Result<pdf_viewer_core::models::FieldHitResolution, String> {
    PdfEditorGeometryService::resolve_field_hit(request)
}

#[command]
pub fn resolve_hit_target(
    request: pdf_viewer_core::models::FieldHitBatchRequest,
) -> Result<Option<pdf_viewer_core::models::FieldHitMatch>, String> {
    PdfEditorGeometryService::resolve_field_hit_target(request)
}

#[command]
pub fn resolve_projection(
    request: pdf_viewer_core::models::FieldProjectionRequest,
) -> Result<pdf_viewer_core::models::FieldProjection, String> {
    PdfEditorGeometryService::resolve_field_projection(request)
}

#[command]
pub fn resolve_params(
    request: pdf_viewer_core::models::FieldEditorParamsRequest,
) -> Result<pdf_viewer_core::models::FieldEditorParams, String> {
    PdfEditorGeometryService::resolve_field_editor_params(request)
}

#[command]
pub fn create_demo_pdf(path: String) -> Result<String, String> {
    PdfDocumentService::generate_demo_pdf(&path)
}
#[command]
pub async fn render_tile(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    page_index: u16,
    zoom: f32,
    width: u32,
    height: u32,
) -> Result<String, String> {
    // 1. Get lopdf paths and text runs from cache
    let (mut objects, runs, _pw, _ph) = {
        let cache = state.pdf_documents.lock().unwrap();
        let doc = cache
            .get(&path)
            .ok_or_else(|| format!("Doc not in cache: {}", path))?;
        crate::infrastructure::multimedia::pdf::pdf_read::resolve_paths(
            doc,
            page_index as u32,
        )
        .unwrap_or_else(|_| (Vec::new(), Vec::new(), 595.0, 842.0))
    };

    // [SLOT V3] Merge text runs into objects for the rendereruse crate::infrastructure::multimedia::pdf::models::{NativeTextModel, RenderObject};
    for run in runs {
        let text_id = run
            .object_id
            .clone()
            .unwrap_or_else(|| format!("text_{}_{}", page_index, objects.len()));
        objects.push(RenderObject::Text(NativeTextModel {
            id: text_id,
            text: run.text,
            font_size: run.font_size,
            tx: run.tx,
            ty: run.ty,
            color: run.color,
            stroke_color: run.stroke_color,
            stroke_width: run.stroke_width,
            font_name: run.font_name,
            is_bold: run.is_bold,
            is_italic: run.is_italic,
            font_post_script_name: run.font_post_script_name,
            font_family_hint: run.font_family_hint,
            font_subtype: run.font_subtype,
            embedded_font_key: run.embedded_font_key,
            has_embedded_font_program: run.has_embedded_font_program,
            has_to_unicode_cmap: run.has_to_unicode_cmap,
            scale_x: run.a,
            scale_y: run.d,
            rendering_mode: run.render_mode as i32,
            char_origins: run
                .char_origins
                .into_iter()
                .map(|x| [run.tx + x, run.ty])
                .collect(),
            char_widths: run.char_widths,
            pdf_char_codes: run.pdf_char_codes,
            ..Default::default()
        }));
    }

    // 2. Get or init renderer (handle poisoned mutex from prior panics)
    let needs_init = {
        let opt = state.vello_renderer.lock().unwrap_or_else(|e| {
            eprintln!("[PDF-VELLO] Recovering poisoned mutex");
            e.into_inner()
        });
        opt.is_none()
    }; // MutexGuard dropped here, before any await
    if needs_init {
        let new_renderer =
            crate::infrastructure::multimedia::pdf::vello_renderer::VelloRenderer::new().await?;
        let mut opt = state
            .vello_renderer
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if opt.is_none() {
            *opt = Some(std::sync::Arc::new(std::sync::Mutex::new(new_renderer)));
        }
    }

    let mut vello_renderer_opt = state.vello_renderer.lock().unwrap_or_else(|e| {
        eprintln!("[PDF-VELLO] Recovering poisoned mutex (render phase)");
        e.into_inner()
    });
    let renderer_arc = vello_renderer_opt
        .as_mut()
        .ok_or("Renderer initialization failed")?;

    // 3. Render at zoomed size
    let render_w = ((width as f32 * zoom) as u32).max(1);
    let render_h = ((height as f32 * zoom) as u32).max(1);
    let mut renderer = renderer_arc.lock().map_err(|e| e.to_string())?;
    let png_bytes = renderer.render_objects_to_png(&objects, render_w, render_h, zoom)?;

    // 4. Base64
    use base64::{engine::general_purpose, Engine as _};
    Ok(format!(
        "data:image/png;base64,{}",
        general_purpose::STANDARD.encode(png_bytes)
    ))
}

#[command]
pub fn set_log_level(level: u8) {
    crate::infrastructure::multimedia::pdf::log_utils::set_pdf_log_level(level);
}
#[command]
pub fn get_asset_url(path: String) -> String {
    let binding = path.replace("\\", "/");
    let encoded = urlencoding::encode(&binding);
    format!("http://asset.localhost/{}", encoded.replace("%2F", "/"))
}

#[command]
pub async fn pick_file(app_handle: tauri::AppHandle) -> Result<Option<String>, String> {
use tauri_plugin_dialog::DialogExt;
    let file_path = app_handle
        .dialog()
        .file()
        .add_filter("PDF Documents", &["pdf"])
        .blocking_pick_file();
    Ok(file_path.map(|p| p.to_string()))
}

