// engine.rs — Re-exports for backward compatibility.
// Actual implementations are in document_service.rs, page_model_service.rs, geometry_service.rs.
pub use super::document_service::PdfDocumentService;
pub use super::geometry_service::PdfEditorGeometryService;
pub use super::page_intermediate_service::PdfPageIntermediateService;
pub use super::page_model_service::PdfPageModelService;
