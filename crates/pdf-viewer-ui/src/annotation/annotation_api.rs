//! AnnotationManager — P3 struct-based WASM API for PDF `/Annot` objects.
//!
//! Mirrors the P0–P2 pattern: zero-sized struct + camelCase methods + thin
//! delegation. Backed by the same Tauri commands that `CommentManager` uses
//! (`read_annotation_targets`, `delete_annotation`).
//!
//! **Domain split**:
//! - `AnnotationManager` — PDF-spec `/Annot` CRUD (list, get, add, update, delete, flatten).
//! - `CommentManager` — review-session UX (panel state, scoped queries, overlay loading).
//!
//! The two layers share the same backend storage; this struct exposes the
//! lower-level annotation surface, while `CommentManager` provides the
//! comment-review workflow on top.

use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;

use crate::annotation::annotation_types::{
    err_response, ok_response, Annotation, AnnotationBBox, AnnotationError, AnnotationKind,
};
use crate::document::comment;
use crate::document::comment::{PdfDeleteAnnotationRequest, PdfPageAnnotationTarget};

// ── PdfPageAnnotationTarget → Annotation conversion ─────────────

fn parse_annotation_kind(raw: &str) -> AnnotationKind {
    match raw {
        // PDF subtype names (canonical)
        "Text" | "text" | "textNote" | "note" => AnnotationKind::TextNote,
        "Highlight" | "highlight" => AnnotationKind::Highlight,
        "Underline" | "underline" => AnnotationKind::Underline,
        "Squiggly" | "squiggly" => AnnotationKind::Squiggly,
        "StrikeOut" | "strikeout" | "strike" => AnnotationKind::Strikeout,
        "Ink" | "ink" => AnnotationKind::Ink,
        "Stamp" | "stamp" => AnnotationKind::Stamp,
        "Link" | "link" => AnnotationKind::Link,
        "Widget" | "Sig" | "signature" => AnnotationKind::Signature,
        "FreeText" | "freeText" | "freetext" => AnnotationKind::FreeText,
        _ => AnnotationKind::Other,
    }
}

fn target_to_annotation(t: &PdfPageAnnotationTarget) -> Annotation {
    // Wire format uses (left, top, width, height); convert to (left, top, right, bottom).
    Annotation {
        id: t.id.clone(),
        page_index: t.page_index,
        kind: parse_annotation_kind(&t.kind),
        bbox: AnnotationBBox {
            left: t.box_rect.left,
            top: t.box_rect.top,
            right: t.box_rect.left + t.box_rect.width,
            bottom: t.box_rect.top + t.box_rect.height,
        },
        contents: None,
        author: None,
        created_at: None,
        modified_at: None,
    }
}

// ── AnnotationManager ───────────────────────────────────────────

#[wasm_bindgen]
pub struct AnnotationManager;

#[wasm_bindgen]
impl AnnotationManager {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        AnnotationManager
    }

    // ── Implemented: list, delete ───────────────────────────────

    /// List all annotations on a given page.
    ///
    /// Returns a structured `AnnotationResponse<Annotation[]>` payload.
    #[wasm_bindgen(js_name = "list")]
    pub async fn list(&self, path: String, page_index: u16) -> JsValue {
        match comment::list_page_annotation_targets(path, page_index).await {
            Ok(result) => {
                let annotations: Vec<Annotation> =
                    result.targets.iter().map(target_to_annotation).collect();
                ok_response(annotations)
            }
            Err(js_err) => err_response(AnnotationError::IoError {
                message: format!("{:?}", js_err),
            }),
        }
    }

    /// Delete an annotation by id on the given page.
    ///
    /// Returns a structured `AnnotationResponse<{ deleted: bool }>` payload.
    #[wasm_bindgen(js_name = "delete")]
    pub async fn delete(&self, path: String, page_index: u16, annotation_id: String) -> JsValue {
        let request = PdfDeleteAnnotationRequest {
            page_index,
            annotation_id: annotation_id.clone(),
        };
        match comment::delete_page_annotation(path, request).await {
            Ok(result) => to_value(&result).unwrap_or(JsValue::NULL),
            Err(js_err) => err_response(AnnotationError::IoError {
                message: format!("{:?}", js_err),
            }),
        }
    }
}

impl Default for AnnotationManager {
    fn default() -> Self {
        Self::new()
    }
}
