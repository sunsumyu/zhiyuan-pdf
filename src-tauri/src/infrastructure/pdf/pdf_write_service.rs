use crate::infrastructure::pdf::models::PdfModifications;
use crate::infrastructure::pdf::save_engine::apply_pdf_commands;
use crate::infrastructure::pdf::commands::PdfEditCommand;
use crate::log_step;
use std::fs;
use std::sync::Arc;
use lopdf::Document as LopdfDocument;

pub struct PdfWriteService;

impl PdfWriteService {
    /// 保存PDF文档到磁盘
    pub async fn save_pdf(
        state: tauri::State<'_, crate::AppState>,
        path: String,
        modifications: PdfModifications,
    ) -> Result<(), String> {
        log_step!("[PDF][save_pdf] START path={}", path);
        
        let working_path = {
            let docs = state.pdf_documents.lock().unwrap();
            let current_doc = docs
                .get(&path)
                .ok_or_else(|| "Document not found in cache".to_string())?;
            let mut modified_doc = (**current_doc).clone();
            drop(docs); // 释放锁
            
            // 应用修改
            for patch in modifications.text_patches {
                let commands: Vec<Box<dyn PdfEditCommand>> = vec![
                    Box::new(crate::infrastructure::pdf::commands::ReplaceTextCommand { patch })
                ];
                modified_doc = apply_pdf_commands(modified_doc, 0, commands)
                    .map_err(|e| format!("Failed to apply text patch: {}", e))?;
            }
            
            // 保存到工作路径
            let working_path = crate::infrastructure::pdf::pdf_read_service::PdfReadService::get_working_path(&path);
            modified_doc
                .save(&working_path)
                .map_err(|e| format!("Failed to save working copy: {}", e))?;
            
            working_path
        };

        // 更新内存缓存
        let new_doc = tokio::task::spawn_blocking(move || {
            LopdfDocument::load(&working_path)
                .map(Arc::new)
                .map_err(|e| format!("Failed to reload saved document: {}", e))
        })
        .await
        .map_err(|e| e.to_string())??;

        {
            let mut cache = state.pdf_documents.lock().unwrap();
            cache.insert(path.clone(), new_doc);
        }

        // 清除相关缓存
        Self::invalidate_caches(&state, &path);

        log_step!("[PDF][save_pdf] SUCCESS");
        Ok(())
    }

    /// 回滚PDF文档到上一个版本
    pub async fn rollback_pdf(
        state: tauri::State<'_, crate::AppState>,
        path: &str,
    ) -> Result<(), String> {
        log_step!("[PDF][rollback_pdf] START path={}", path);

        // 获取历史记录
        let previous_doc = {
            let mut txs = state.pdf_transactions.lock().unwrap();
            let history = txs.get_mut(path);
            
            if let Some(history) = history {
                if history.len() > 1 {
                    history.pop(); // 移除当前版本
                    history.last().cloned()
                } else {
                    return Err("No previous version available for rollback".to_string());
                }
            } else {
                return Err("No history found for document".to_string());
            }
        };

        if let Some(previous_doc) = previous_doc {
            // 保存到磁盘
            let working_path = crate::infrastructure::pdf::pdf_read_service::PdfReadService::get_working_path(path);
            let mut doc_clone = (*previous_doc).clone();
            
            tokio::task::spawn_blocking(move || {
                doc_clone.save(&working_path)
            })
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| format!("Failed to save rollback document: {}", e))?;

            // 更新内存缓存
            {
                let mut cache = state.pdf_documents.lock().unwrap();
                cache.insert(path.to_string(), previous_doc);
            }

            // 清除相关缓存
            Self::invalidate_caches(&state, path);

            // 清除重做历史
            {
                let mut redo = state.pdf_redo_transactions.lock().unwrap();
                redo.remove(path);
            }

            log_step!("[PDF][rollback_pdf] SUCCESS");
            Ok(())
        } else {
            Err("No previous document available for rollback".to_string())
        }
    }

    /// 重做PDF文档操作
    pub async fn redo_pdf(
        state: tauri::State<'_, crate::AppState>,
        path: &str,
    ) -> Result<(), String> {
        log_step!("[PDF][redo_pdf] START path={}", path);

        // 获取重做历史
        let redo_doc = {
            let mut redo = state.pdf_redo_transactions.lock().unwrap();
            redo.get(path).cloned()
        };

        if let Some(redo_doc) = redo_doc.as_ref().and_then(|v| v.last()).map(|arc| (**arc).clone()) {
            // 保存当前版本到撤销历史
            {
                let docs = state.pdf_documents.lock().unwrap();
                if let Some(current_doc) = docs.get(path) {
                    let mut txs = state.pdf_transactions.lock().unwrap();
                    let history = txs.entry(path.to_string()).or_insert_with(Vec::new);
                    history.push(current_doc.clone());
                    if history.len() > 20 {
                        history.remove(0);
                    }
                }
            }

            // 应用重做版本
            let working_path = crate::infrastructure::pdf::pdf_read_service::PdfReadService::get_working_path(path);
            let mut doc_clone = redo_doc.clone();
            
            tokio::task::spawn_blocking(move || {
                doc_clone.save(&working_path)
            })
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| format!("Failed to save redo document: {}", e))?;

            // 更新内存缓存
            {
                let mut cache = state.pdf_documents.lock().unwrap();
                cache.insert(path.to_string(), Arc::new(redo_doc));
            }

            // 清除相关缓存
            Self::invalidate_caches(&state, path);

            // 清除重做历史
            {
                let mut redo = state.pdf_redo_transactions.lock().unwrap();
                redo.remove(path);
            }

            log_step!("[PDF][redo_pdf] SUCCESS");
            Ok(())
        } else {
            Err("No redo operation available".to_string())
        }
    }

    /// 生成演示PDF文档
    pub fn generate_demo_pdf(path: &str) -> Result<String, String> {
        let pdf_content = b"%PDF-1.7
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj
3 0 obj
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>
endobj
4 0 obj
<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>
endobj
5 0 obj
<< /Length 59 >>
stream
BT
/F1 24 Tf
100 700 Td
(Demo) Tj
ET
endstream
endobj
xref
0 6
0000000000 65535 f 
0000000010 00000 n 
0000000079 00000 n 
0000000173 00000 n 
0000000301 00000 n 
0000000380 00000 n 
trailer
<< /Size 6 /Root 1 0 R >>
startxref
456
%%EOF";

        fs::write(path, pdf_content).map_err(|_| "Failed to write demo PDF".to_string())?;
        Ok("Demo PDF generated successfully".to_string())
    }

    /// 清除所有相关缓存
    fn invalidate_caches(state: &tauri::State<'_, crate::AppState>, path: &str) {
        // 清除轻量页面缓存
        {
            let mut cache = state.pdf_light_page_cache.lock().unwrap();
            cache.retain(|key, _| !key.starts_with(&format!("light::{}::", path)));
        }

        // 清除页面缓存
        {
            let mut cache = state.pdf_page_cache.lock().unwrap();
            cache.retain(|key, _| !key.starts_with(&format!("{}::", path)));
        }

        // 清除布局缓存
        {
            let mut cache = state.pdf_layout_cache.lock().unwrap();
            cache.retain(|key, _| !key.starts_with(&format!("{}::", path)));
        }
    }
}
