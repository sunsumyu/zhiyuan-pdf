use serde::{Deserialize, Serialize};

use super::styled_run::StyledRun;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct VectorPathSegment {
    pub command: String,
    #[serde(default)]
    pub points: Vec<[f32; 2]>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct VectorPathObject {
    pub id: String,
    #[serde(default)]
    pub segments: Vec<VectorPathSegment>,
    pub fill_color: Option<String>,
    pub stroke_color: Option<String>,
    #[serde(default)]
    pub fill: bool,
    #[serde(default)]
    pub stroke: bool,
    #[serde(default)]
    pub stroke_width: f32,
    #[serde(default)]
    pub z_index: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct VectorImageObject {
    pub id: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    #[serde(default)]
    pub z_index: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct VectorTextObject {
    pub id: String,
    #[serde(default)]
    pub runs: Vec<StyledRun>,
    #[serde(default)]
    pub z_index: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum VectorRenderObject {
    Text(VectorTextObject),
    Path(VectorPathObject),
    Image(VectorImageObject),
}

/// 大一统的渲染原语容器版本。包含了当前页面所有的纯几何/排版对象。
///
/// # Overview (架构定位)
/// 它是整个 Core 离线计算与前端的骨架。承载着从 Wasm 到 TS 层的桥接数据负担。
/// 这里的 `objects` 不包含深层语义（不知道什么是段落），只知道这里有一堆字，有一群几何图形。
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct VectorPageModel {
    pub page_index: u16,
    pub width: f32,
    pub height: f32,
    #[serde(default)]
    pub objects: Vec<VectorRenderObject>,
}

impl VectorPageModel {
    /// 文档级空间标准化关卡 (The Great Normalization Gate)
    ///
    /// 这是 `Y-Up` 历史遗留问题被阻绝在外的最后防线。
    /// 当底层的拉流解析器 (Stream Parser) 组装完原始树后，必须调用这个方法。
    /// 一旦该方法执行完毕，模型连带其挂载的子流（Text, Paths, Images）将不可逆地转换为安全的 Y-Down 坐标域。
    pub fn flip_y(&mut self) {
        let h = self.height;
        for obj in &mut self.objects {
            match obj {
                VectorRenderObject::Text(t) => {
                    for run in &mut t.runs {
                        run.flip_y(h);
                    }
                }
                VectorRenderObject::Path(p) => {
                    for seg in &mut p.segments {
                        for pt in &mut seg.points {
                            pt[1] = h - pt[1];
                        }
                    }
                }
                VectorRenderObject::Image(img) => {
                    img.y = h - (img.y + img.height);
                }
            }
        }
    }
}
