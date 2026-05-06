use crate::infrastructure::multimedia::pdf::domain::{types::*, errors::*, page::*};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// 获取当前时间的RFC 3339格式字符串
fn now_rfc3339() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    // 简化为 Unix 时间戳格式（避免引入 chrono 依赖）
    format!("{}Z", secs)
}

/// 文档元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub keywords: Option<String>,
    pub creator: Option<String>,
    pub producer: Option<String>,
    pub creation_date: Option<String>,
    pub modification_date: Option<String>,
}

impl DocumentMetadata {
    pub fn new() -> Self {
        DocumentMetadata {
            title: None,
            author: None,
            subject: None,
            keywords: None,
            creator: None,
            producer: None,
            creation_date: None,
            modification_date: None,
        }
    }
    
    pub fn with_title(mut self, title: String) -> Self {
        self.title = Some(title);
        self
    }
    
    pub fn with_author(mut self, author: String) -> Self {
        self.author = Some(author);
        self
    }
}

impl Default for DocumentMetadata {
    fn default() -> Self {
        Self::new()
    }
}

/// 文档领域模型 - 聚合根
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    id: DocumentId,
    metadata: DocumentMetadata,
    pages: HashMap<PageNumber, Page>,
    version: u32,
    created_at: String,
    modified_at: String,
}

impl Document {
    /// 创建新文档
    pub fn new(id: DocumentId) -> DomainResult<Self> {
        let now = now_rfc3339();
        
        Ok(Document {
            id,
            metadata: DocumentMetadata::new(),
            pages: HashMap::new(),
            version: 1,
            created_at: now.clone(),
            modified_at: now,
        })
    }
    
    /// 从现有数据创建文档
    pub fn from_existing(
        id: DocumentId,
        metadata: DocumentMetadata,
        pages: Vec<Page>,
    ) -> DomainResult<Self> {
        let now = now_rfc3339();
        let mut page_map = HashMap::new();
        
        for page in pages {
            let page_num = page.number();
            if page_map.contains_key(&page_num) {
                return Err(DomainError::InvalidDocumentState(
                    format!("Duplicate page number: {}", page_num)
                ));
            }
            page_map.insert(page_num, page);
        }
        
        Ok(Document {
            id,
            metadata,
            pages: page_map,
            version: 1,
            created_at: now.clone(),
            modified_at: now,
        })
    }
    
    /// 获取文档ID
    pub fn id(&self) -> &DocumentId {
        &self.id
    }
    
    /// 获取文档元数据
    pub fn metadata(&self) -> &DocumentMetadata {
        &self.metadata
    }
    
    /// 更新文档元数据
    pub fn update_metadata(&mut self, metadata: DocumentMetadata) -> DomainResult<()> {
        self.metadata = metadata;
        self.touch();
        Ok(())
    }
    
    /// 获取文档版本
    pub fn version(&self) -> u32 {
        self.version
    }
    
    /// 获取创建时间
    pub fn created_at(&self) -> &str {
        &self.created_at
    }
    
    /// 获取修改时间
    pub fn modified_at(&self) -> &str {
        &self.modified_at
    }
    
    /// 获取页面数量
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }
    
    /// 获取所有页码
    pub fn page_numbers(&self) -> Vec<PageNumber> {
        let mut numbers: Vec<PageNumber> = self.pages.keys().copied().collect();
        numbers.sort();
        numbers
    }
    
    /// 获取指定页面
    pub fn get_page(&self, page_number: PageNumber) -> DomainResult<&Page> {
        self.pages.get(&page_number)
            .ok_or_else(|| DomainError::PageNotFound {
                document_id: self.id.as_str().to_string(),
                page_number: page_number.as_one_based(),
            })
    }
    
    /// 获取可变页面引用
    pub fn get_page_mut(&mut self, page_number: PageNumber) -> DomainResult<&mut Page> {
        self.touch();
        self.pages.get_mut(&page_number)
            .ok_or_else(|| DomainError::PageNotFound {
                document_id: self.id.as_str().to_string(),
                page_number: page_number.as_one_based(),
            })
    }
    
    /// 添加页面
    pub fn add_page(&mut self, page: Page) -> DomainResult<()> {
        let page_num = page.number();
        
        if self.pages.contains_key(&page_num) {
            return Err(DomainError::InvalidDocumentState(
                format!("Page {} already exists", page_num)
            ));
        }
        
        self.pages.insert(page_num, page);
        self.touch();
        Ok(())
    }
    
    /// 移除页面
    pub fn remove_page(&mut self, page_number: PageNumber) -> DomainResult<Page> {
        self.touch();
        self.pages.remove(&page_number)
            .ok_or_else(|| DomainError::PageNotFound {
                document_id: self.id.as_str().to_string(),
                page_number: page_number.as_one_based(),
            })
    }
    
    /// 替换页面
    pub fn replace_page(&mut self, page: Page) -> DomainResult<()> {
        let page_num = page.number();
        
        if !self.pages.contains_key(&page_num) {
            return Err(DomainError::PageNotFound {
                document_id: self.id.as_str().to_string(),
                page_number: page_num.as_one_based(),
            });
        }
        
        self.pages.insert(page_num, page);
        self.touch();
        Ok(())
    }
    
    /// 旋转页面
    pub fn rotate_page(&mut self, page_number: PageNumber, delta_degrees: i32) -> DomainResult<()> {
        let page = self.get_page_mut(page_number)?;
        page.rotate(delta_degrees)?;
        self.touch();
        Ok(())
    }
    
    /// 检查文档是否有效
    pub fn validate(&self) -> DomainResult<()> {
        if self.pages.is_empty() {
            return Err(DomainError::InvalidDocumentState(
                "Document must have at least one page".to_string()
            ));
        }
        
        // 检查页码连续性
        let mut expected_page = 1;
        for page_number in self.page_numbers() {
            if page_number.as_one_based() != expected_page {
                return Err(DomainError::InvalidDocumentState(
                    format!("Non-sequential page numbers. Expected {}, found {}", 
                           expected_page, page_number.as_one_based())
                ));
            }
            expected_page += 1;
        }
        
        Ok(())
    }
    
    /// 创建文档快照（用于撤销/重做）
    pub fn snapshot(&self) -> DocumentSnapshot {
        DocumentSnapshot {
            document: self.clone(),
            timestamp: now_rfc3339(),
        }
    }
    
    /// 更新修改时间
    fn touch(&mut self) {
        self.modified_at = now_rfc3339();
        self.version += 1;
    }
}

/// 文档快照 - 用于撤销/重做功能
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSnapshot {
    pub document: Document,
    pub timestamp: String,
}

impl DocumentSnapshot {
    pub fn new(document: Document) -> Self {
        Self {
            document,
            timestamp: now_rfc3339(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_document_creation() {
        let doc = Document::new("test.pdf".into()).unwrap();
        assert_eq!(doc.id().as_str(), "test.pdf");
        assert_eq!(doc.page_count(), 0);
        assert_eq!(doc.version(), 1);
    }
    
    #[test]
    fn test_add_page() -> DomainResult<()> {
        let mut doc = Document::new("test.pdf".into())?;
        let page = Page::new(PageNumber::new(1)?, BoundingBox::new(0.0, 0.0, 595.0, 842.0)?)?;
        
        doc.add_page(page)?;
        assert_eq!(doc.page_count(), 1);
        
        Ok(())
    }
    
    #[test]
    fn test_duplicate_page_error() -> DomainResult<()> {
        let mut doc = Document::new("test.pdf".into())?;
        let page1 = Page::new(PageNumber::new(1)?, BoundingBox::new(0.0, 0.0, 595.0, 842.0)?)?;
        let page2 = Page::new(PageNumber::new(1)?, BoundingBox::new(0.0, 0.0, 595.0, 842.0)?)?;
        
        doc.add_page(page1)?;
        assert!(doc.add_page(page2).is_err());
        
        Ok(())
    }
}
