use thiserror::Error;

/// 领域层错误类型
#[derive(Debug, Error, Clone)]
pub enum DomainError {
    #[error("Invalid page number: {0}. Page numbers must be >= 1")]
    InvalidPageNumber(u16),
    
    #[error("Invalid bounding box: x={x}, y={y}, width={width}, height={height}. Width and height must be >= 0")]
    InvalidBoundingBox { x: f32, y: f32, width: f32, height: f32 },
    
    #[error("Invalid color: r={r}, g={g}, b={b}. Color components must be between 0.0 and 1.0")]
    InvalidColor { r: f32, g: f32, b: f32 },
    
    #[error("Document not found: {0}")]
    DocumentNotFound(String),
    
    #[error("Page not found: document={document_id}, page={page_number}")]
    PageNotFound { document_id: String, page_number: u16 },
    
    #[error("Invalid document state: {0}")]
    InvalidDocumentState(String),
    
    #[error("Operation not permitted: {0}")]
    OperationNotPermitted(String),
    
    #[error("PDF processing error: {0}")]
    PdfProcessingError(String),
    
    #[error("IO error: {0}")]
    IoError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Validation error: {0}")]
    ValidationError(String),
}

impl From<std::io::Error> for DomainError {
    fn from(err: std::io::Error) -> Self {
        DomainError::IoError(err.to_string())
    }
}

impl From<serde_json::Error> for DomainError {
    fn from(err: serde_json::Error) -> Self {
        DomainError::SerializationError(err.to_string())
    }
}

/// 领域结果类型
pub type DomainResult<T> = Result<T, DomainError>;
