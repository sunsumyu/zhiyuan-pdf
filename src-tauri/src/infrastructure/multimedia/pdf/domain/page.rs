use crate::infrastructure::multimedia::pdf::domain::{types::*, errors::*};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 页面领域模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    number: PageNumber,
    bbox: BoundingBox,
    rotation: i32, // 旋转角度，90度的倍数
    annotations: HashMap<String, Annotation>,
    content: PageContent,
    metadata: PageMetadata,
}

/// 页面内容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PageContent {
    /// 矢量内容（PDF原生）
    Vector {
        paths: Vec<VectorPath>,
        text_runs: Vec<TextRun>,
        images: Vec<Image>,
    },
    /// 光栅内容（位图）
    Raster {
        image_url: String,
        width: u32,
        height: u32,
    },
    /// 混合内容
    Mixed {
        vector_content: Box<PageContent>,
        raster_overlays: Vec<RasterOverlay>,
    },
}

/// 矢量路径
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorPath {
    pub id: String,
    pub commands: Vec<PathCommand>,
    pub bbox: BoundingBox,
    pub style: PathStyle,
}

/// 路径命令
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PathCommand {
    MoveTo(Point),
    LineTo(Point),
    CurveTo(Point, Point, Point), // 控制点1, 控制点2, 终点
    ClosePath,
}

/// 路径样式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathStyle {
    pub stroke_color: Option<Color>,
    pub fill_color: Option<Color>,
    pub stroke_width: f32,
    pub dash_pattern: Option<Vec<f32>>,
}

/// 文本运行
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextRun {
    pub id: String,
    pub text: String,
    pub bbox: BoundingBox,
    pub font_name: String,
    pub font_size: f32,
    pub color: Color,
    pub position: Point,
}

/// 图像
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    pub id: String,
    pub bbox: BoundingBox,
    pub data_url: String,
    pub width: u32,
    pub height: u32,
}

/// 光栅覆盖层
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RasterOverlay {
    pub id: String,
    pub bbox: BoundingBox,
    pub image_url: String,
    pub opacity: f32,
}

/// 页面元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageMetadata {
    pub width: f32,
    pub height: f32,
    pub dpi: f32,
    pub color_space: String,
}

/// 批注基类
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Annotation {
    Highlight(HighlightAnnotation),
    Text(TextAnnotation),
    Shape(ShapeAnnotation),
}

/// 高亮批注
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightAnnotation {
    pub id: String,
    pub bbox: BoundingBox,
    pub color: Color,
    pub text: Option<String>,
}

/// 文本批注
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextAnnotation {
    pub id: String,
    pub bbox: BoundingBox,
    pub color: Color,
    pub contents: String,
    pub author: Option<String>,
}

/// 形状批注
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapeAnnotation {
    pub id: String,
    pub bbox: BoundingBox,
    pub color: Color,
    pub shape_type: ShapeType,
}

/// 形状类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShapeType {
    Rectangle,
    Circle,
    Line,
    Arrow,
}

impl Page {
    /// 创建新页面
    pub fn new(number: PageNumber, bbox: BoundingBox) -> DomainResult<Self> {
        let metadata = PageMetadata {
            width: bbox.width,
            height: bbox.height,
            dpi: 72.0,
            color_space: "RGB".to_string(),
        };
        
        Ok(Page {
            number,
            bbox,
            rotation: 0,
            annotations: HashMap::new(),
            content: PageContent::Vector {
                paths: Vec::new(),
                text_runs: Vec::new(),
                images: Vec::new(),
            },
            metadata,
        })
    }
    
    /// 获取页码
    pub fn number(&self) -> PageNumber {
        self.number
    }
    
    /// 获取页面边界框
    pub fn bbox(&self) -> BoundingBox {
        self.bbox
    }
    
    /// 获取旋转角度
    pub fn rotation(&self) -> i32 {
        self.rotation
    }
    
    /// 旋转页面
    pub fn rotate(&mut self, delta_degrees: i32) -> DomainResult<()> {
        // 确保旋转角度是90的倍数
        if delta_degrees % 90 != 0 {
            return Err(DomainError::ValidationError(
                "Rotation must be a multiple of 90 degrees".to_string()
            ));
        }
        
        self.rotation = (self.rotation + delta_degrees) % 360;
        
        // 如果旋转90或270度，需要交换宽高
        if self.rotation % 180 != 0 {
            std::mem::swap(&mut self.bbox.width, &mut self.bbox.height);
            std::mem::swap(&mut self.metadata.width, &mut self.metadata.height);
        }
        
        Ok(())
    }
    
    /// 获取页面内容
    pub fn content(&self) -> &PageContent {
        &self.content
    }
    
    /// 获取页面元数据
    pub fn metadata(&self) -> &PageMetadata {
        &self.metadata
    }
    
    /// 获取所有批注
    pub fn annotations(&self) -> &HashMap<String, Annotation> {
        &self.annotations
    }
    
    /// 添加批注
    pub fn add_annotation(&mut self, annotation: Annotation) -> DomainResult<()> {
        let id = match &annotation {
            Annotation::Highlight(a) => a.id.clone(),
            Annotation::Text(a) => a.id.clone(),
            Annotation::Shape(a) => a.id.clone(),
        };
        
        if self.annotations.contains_key(&id) {
            return Err(DomainError::ValidationError(
                format!("Annotation with id '{}' already exists", id)
            ));
        }
        
        self.annotations.insert(id, annotation);
        Ok(())
    }
    
    /// 移除批注
    pub fn remove_annotation(&mut self, id: &str) -> DomainResult<Annotation> {
        self.annotations.remove(id)
            .ok_or_else(|| DomainError::ValidationError(
                format!("Annotation with id '{}' not found", id)
            ))
    }
    
    /// 获取指定批注
    pub fn get_annotation(&self, id: &str) -> DomainResult<&Annotation> {
        self.annotations.get(id)
            .ok_or_else(|| DomainError::ValidationError(
                format!("Annotation with id '{}' not found", id)
            ))
    }
    
    /// 更新批注
    pub fn update_annotation(&mut self, annotation: Annotation) -> DomainResult<()> {
        let id = match &annotation {
            Annotation::Highlight(a) => a.id.clone(),
            Annotation::Text(a) => a.id.clone(),
            Annotation::Shape(a) => a.id.clone(),
        };
        
        if !self.annotations.contains_key(&id) {
            return Err(DomainError::ValidationError(
                format!("Annotation with id '{}' not found", id)
            ));
        }
        
        self.annotations.insert(id, annotation);
        Ok(())
    }
    
    /// 添加高亮批注
    pub fn add_highlight(&mut self, bbox: BoundingBox, color: Color, text: Option<String>) -> DomainResult<String> {
        let id = format!("highlight_{}", uuid::Uuid::new_v4());
        let annotation = Annotation::Highlight(HighlightAnnotation {
            id: id.clone(),
            bbox,
            color,
            text,
        });
        
        self.add_annotation(annotation)?;
        Ok(id)
    }
    
    /// 添加文本批注
    pub fn add_text_annotation(&mut self, bbox: BoundingBox, color: Color, contents: String, author: Option<String>) -> DomainResult<String> {
        let id = format!("text_{}", uuid::Uuid::new_v4());
        let annotation = Annotation::Text(TextAnnotation {
            id: id.clone(),
            bbox,
            color,
            contents,
            author,
        });
        
        self.add_annotation(annotation)?;
        Ok(id)
    }
    
    /// 检查点是否在页面内
    pub fn contains_point(&self, x: f32, y: f32) -> bool {
        self.bbox.contains_point(x, y)
    }
    
    /// 获取页面在指定旋转后的边界框
    pub fn rotated_bbox(&self) -> BoundingBox {
        match self.rotation {
            0 | 180 => self.bbox,
            90 | 270 => BoundingBox {
                x: self.bbox.x,
                y: self.bbox.y,
                width: self.bbox.height,
                height: self.bbox.width,
            },
            _ => self.bbox, // 不应该发生，但提供默认值
        }
    }
    
    /// 验证页面数据
    pub fn validate(&self) -> DomainResult<()> {
        // 检查边界框
        if self.bbox.width <= 0.0 || self.bbox.height <= 0.0 {
            return Err(DomainError::InvalidBoundingBox {
                x: self.bbox.x,
                y: self.bbox.y,
                width: self.bbox.width,
                height: self.bbox.height,
            });
        }
        
        // 检查旋转角度
        if self.rotation % 90 != 0 {
            return Err(DomainError::ValidationError(
                "Page rotation must be a multiple of 90 degrees".to_string()
            ));
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_page_creation() -> DomainResult<()> {
        let bbox = BoundingBox::new(0.0, 0.0, 595.0, 842.0)?;
        let page = Page::new(PageNumber::new(1)?, bbox)?;
        
        assert_eq!(page.number().as_one_based(), 1);
        assert_eq!(page.rotation(), 0);
        assert_eq!(page.annotations().len(), 0);
        
        Ok(())
    }
    
    #[test]
    fn test_page_rotation() -> DomainResult<()> {
        let bbox = BoundingBox::new(0.0, 0.0, 595.0, 842.0)?;
        let mut page = Page::new(PageNumber::new(1)?, bbox)?;
        
        page.rotate(90)?;
        assert_eq!(page.rotation(), 90);
        assert_eq!(page.bbox().width, 842.0);
        assert_eq!(page.bbox().height, 595.0);
        
        page.rotate(90)?;
        assert_eq!(page.rotation(), 180);
        assert_eq!(page.bbox().width, 595.0);
        assert_eq!(page.bbox().height, 842.0);
        
        Ok(())
    }
    
    #[test]
    fn test_add_annotation() -> DomainResult<()> {
        let bbox = BoundingBox::new(0.0, 0.0, 595.0, 842.0)?;
        let mut page = Page::new(PageNumber::new(1)?, bbox)?;
        
        let highlight_bbox = BoundingBox::new(100.0, 100.0, 200.0, 50.0)?;
        let color = Color::new(1.0, 0.0, 0.0)?;
        let id = page.add_highlight(highlight_bbox, color, Some("test".to_string()))?;
        
        assert_eq!(page.annotations().len(), 1);
        assert!(page.get_annotation(&id).is_ok());
        
        Ok(())
    }
}
