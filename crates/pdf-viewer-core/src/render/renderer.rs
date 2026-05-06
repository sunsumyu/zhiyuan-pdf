use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum DrawCommand {
    Text {
        text: String,
        x: f32,
        y: f32,
        font_size: f32,
        color: String,
        font_name: String,
    },
    Rect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: String,
        #[serde(default)]
        is_fill: bool,
    },
    Line {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        color: String,
        width: f32,
    },
}

pub trait PdfRenderer {
    /// 执行全量或增量绘图指令
    fn render(&mut self, commands: &[DrawCommand]);
    
    /// 清除画布
    fn clear(&mut self);
    
    /// 获取当前渲染后端名称 (e.g., "WebGPU", "Canvas2D")
    fn name(&self) -> &str;
}
