use super::DomainError;
use serde::{Deserialize, Serialize};
use std::fmt;

/// 文档标识符 - 强类型包装
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DocumentId(pub String);

impl DocumentId {
    pub fn new(id: String) -> Self {
        DocumentId(id)
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
    
    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for DocumentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for DocumentId {
    fn from(id: String) -> Self {
        DocumentId(id)
    }
}

impl From<&str> for DocumentId {
    fn from(id: &str) -> Self {
        DocumentId(id.to_string())
    }
}

/// 页码 - 强类型包装，确保页码有效
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PageNumber(pub u16);

impl PageNumber {
    pub fn new(page: u16) -> Result<Self, DomainError> {
        if page == 0 {
            Err(DomainError::InvalidPageNumber(page))
        } else {
            Ok(PageNumber(page))
        }
    }
    
    pub fn from_one_based(page: u16) -> Result<Self, DomainError> {
        Self::new(page)
    }
    
    pub fn from_zero_based(page: u16) -> Result<Self, DomainError> {
        Self::new(page + 1)
    }
    
    pub fn as_one_based(&self) -> u16 {
        self.0
    }
    
    pub fn as_zero_based(&self) -> u16 {
        self.0 - 1
    }
    
    pub fn is_first(&self) -> bool {
        self.0 == 1
    }
}

impl fmt::Display for PageNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 边界框 - 强类型几何对象
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl BoundingBox {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Result<Self, DomainError> {
        if width < 0.0 || height < 0.0 {
            Err(DomainError::InvalidBoundingBox { x, y, width, height })
        } else {
            Ok(BoundingBox { x, y, width, height })
        }
    }
    
    pub fn from_array(rect: [f32; 4]) -> Result<Self, DomainError> {
        Self::new(rect[0], rect[1], rect[2], rect[3])
    }
    
    pub fn to_array(&self) -> [f32; 4] {
        [self.x, self.y, self.width, self.height]
    }
    
    pub fn area(&self) -> f32 {
        self.width * self.height
    }
    
    pub fn contains_point(&self, x: f32, y: f32) -> bool {
        x >= self.x && x <= self.x + self.width &&
        y >= self.y && y <= self.y + self.height
    }
    
    pub fn intersects(&self, other: &BoundingBox) -> bool {
        let self_right = self.x + self.width;
        let self_bottom = self.y + self.height;
        let other_right = other.x + other.width;
        let other_bottom = other.y + other.height;
        
        !(self.x >= other_right || self_right <= other.x ||
          self.y >= other_bottom || self_bottom <= other.y)
    }
    
    pub fn union(&self, other: &BoundingBox) -> BoundingBox {
        let x1 = self.x.min(other.x);
        let y1 = self.y.min(other.y);
        let x2 = (self.x + self.width).max(other.x + other.width);
        let y2 = (self.y + self.height).max(other.y + other.height);
        
        BoundingBox {
            x: x1,
            y: y1,
            width: x2 - x1,
            height: y2 - y1,
        }
    }
}

impl fmt::Display for BoundingBox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BoundingBox(x={}, y={}, w={}, h={})", 
               self.x, self.y, self.width, self.height)
    }
}

/// 颜色 - 强类型RGB值
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl Color {
    pub fn new(r: f32, g: f32, b: f32) -> Result<Self, DomainError> {
        if !(0.0..=1.0).contains(&r) || 
           !(0.0..=1.0).contains(&g) || 
           !(0.0..=1.0).contains(&b) {
            Err(DomainError::InvalidColor { r, g, b })
        } else {
            Ok(Color { r, g, b })
        }
    }
    
    pub fn from_array(rgb: [f32; 3]) -> Result<Self, DomainError> {
        Self::new(rgb[0], rgb[1], rgb[2])
    }
    
    pub fn to_array(&self) -> [f32; 3] {
        [self.r, self.g, self.b]
    }
    
    pub fn black() -> Self {
        Color { r: 0.0, g: 0.0, b: 0.0 }
    }
    
    pub fn white() -> Self {
        Color { r: 1.0, g: 1.0, b: 1.0 }
    }
    
    pub fn red() -> Self {
        Color { r: 1.0, g: 0.0, b: 0.0 }
    }
    
    pub fn green() -> Self {
        Color { r: 0.0, g: 1.0, b: 0.0 }
    }
    
    pub fn blue() -> Self {
        Color { r: 0.0, g: 0.0, b: 1.0 }
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Color(r={:.2}, g={:.2}, b={:.2})", self.r, self.g, self.b)
    }
}

/// 坐标点 - 强类型二维坐标
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Point { x, y }
    }
    
    pub fn distance_to(&self, other: &Point) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
    
    pub fn translate(&self, dx: f32, dy: f32) -> Point {
        Point {
            x: self.x + dx,
            y: self.y + dy,
        }
    }
}

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Point(x={}, y={})", self.x, self.y)
    }
}
