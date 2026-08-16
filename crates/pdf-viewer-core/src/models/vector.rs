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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_color_index: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke_color_index: Option<u8>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct VectorPalette {
    #[serde(default)]
    pub colors: Vec<String>,
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
    #[serde(default)]
    pub palette: VectorPalette,
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

    /// 解压调色板：将 palette index 替换回实际颜色字符串。
    /// 在 WASM 入口 `initPageContext` 处序列化后立即调用，确保
    /// 下游渲染器（canvas.rs）拿到的 fill_color / stroke_color 非 None。
    pub fn decompress_palette(&mut self) {
        if self.palette.colors.is_empty() {
            return;
        }
        for obj in &mut self.objects {
            if let VectorRenderObject::Path(p) = obj {
                if let Some(idx) = p.fill_color_index.take() {
                    if let Some(color) = self.palette.colors.get(idx as usize) {
                        p.fill_color = Some(color.clone());
                    }
                }
                if let Some(idx) = p.stroke_color_index.take() {
                    if let Some(color) = self.palette.colors.get(idx as usize) {
                        p.stroke_color = Some(color.clone());
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_path(fill: bool, stroke: bool, fill_color: Option<&str>, stroke_color: Option<&str>) -> VectorPathObject {
        VectorPathObject {
            id: "test".into(),
            segments: vec![],
            fill_color: fill_color.map(|s| s.to_string()),
            stroke_color: stroke_color.map(|s| s.to_string()),
            fill,
            stroke,
            stroke_width: 1.0,
            z_index: 0,
            fill_color_index: None,
            stroke_color_index: None,
        }
    }

    #[test]
    fn decompress_palette_resolves_fill_color_index() {
        let mut model = VectorPageModel {
            palette: VectorPalette { colors: vec!["#0000ff".into()] },
            objects: vec![
                VectorRenderObject::Path(VectorPathObject {
                    fill_color_index: Some(0),
                    ..make_path(true, false, None, None)
                }),
            ],
            ..Default::default()
        };
        model.decompress_palette();
        let p = match &model.objects[0] {
            VectorRenderObject::Path(p) => p,
            _ => unreachable!(),
        };
        assert_eq!(p.fill_color.as_deref(), Some("#0000ff"));
        assert_eq!(p.fill_color_index, None);
    }

    #[test]
    fn decompress_palette_resolves_stroke_color_index() {
        let mut model = VectorPageModel {
            palette: VectorPalette { colors: vec!["#ff0000".into(), "#00ff00".into()] },
            objects: vec![
                VectorRenderObject::Path(VectorPathObject {
                    stroke_color_index: Some(1),
                    ..make_path(false, true, None, None)
                }),
            ],
            ..Default::default()
        };
        model.decompress_palette();
        let p = match &model.objects[0] {
            VectorRenderObject::Path(p) => p,
            _ => unreachable!(),
        };
        assert_eq!(p.stroke_color.as_deref(), Some("#00ff00"));
        assert_eq!(p.stroke_color_index, None);
    }

    #[test]
    fn decompress_palette_handles_empty_palette() {
        let mut model = VectorPageModel {
            palette: VectorPalette::default(),
            objects: vec![
                VectorRenderObject::Path(VectorPathObject {
                    fill_color: Some("#aaa".into()),
                    ..make_path(true, false, None, None)
                }),
            ],
            ..Default::default()
        };
        model.decompress_palette();
        let p = match &model.objects[0] {
            VectorRenderObject::Path(p) => p,
            _ => unreachable!(),
        };
        assert_eq!(p.fill_color.as_deref(), Some("#aaa"));
    }

    #[test]
    fn decompress_palette_preserves_inline_colors() {
        let mut model = VectorPageModel {
            palette: VectorPalette { colors: vec!["#ff0000".into()] },
            objects: vec![
                VectorRenderObject::Path(VectorPathObject {
                    fill_color: Some("#123456".into()),
                    ..make_path(true, false, None, None)
                }),
            ],
            ..Default::default()
        };
        model.decompress_palette();
        let p = match &model.objects[0] {
            VectorRenderObject::Path(p) => p,
            _ => unreachable!(),
        };
        assert_eq!(p.fill_color.as_deref(), Some("#123456"));
    }

    #[test]
    fn decompress_palette_out_of_range_index_does_not_panic() {
        let mut model = VectorPageModel {
            palette: VectorPalette { colors: vec!["#fff".into()] },
            objects: vec![
                VectorRenderObject::Path(VectorPathObject {
                    fill_color_index: Some(99),
                    ..make_path(true, false, None, None)
                }),
            ],
            ..Default::default()
        };
        model.decompress_palette();
        let p = match &model.objects[0] {
            VectorRenderObject::Path(p) => p,
            _ => unreachable!(),
        };
        assert_eq!(p.fill_color, None);
    }

    #[test]
    fn decompress_palette_multiple_paths() {
        let mut model = VectorPageModel {
            palette: VectorPalette { colors: vec!["#aaa".into(), "#bbb".into(), "#ccc".into()] },
            objects: vec![
                VectorRenderObject::Path(VectorPathObject {
                    fill_color_index: Some(0),
                    stroke_color_index: Some(2),
                    ..make_path(true, true, None, None)
                }),
                VectorRenderObject::Path(VectorPathObject {
                    fill_color: Some("#inline".into()),
                    ..make_path(true, false, None, None)
                }),
                VectorRenderObject::Path(VectorPathObject {
                    stroke_color_index: Some(1),
                    ..make_path(false, true, None, None)
                }),
            ],
            ..Default::default()
        };
        model.decompress_palette();
        let p0 = match &model.objects[0] {
            VectorRenderObject::Path(p) => p,
            _ => unreachable!(),
        };
        assert_eq!(p0.fill_color.as_deref(), Some("#aaa"));
        assert_eq!(p0.stroke_color.as_deref(), Some("#ccc"));
        let p1 = match &model.objects[1] {
            VectorRenderObject::Path(p) => p,
            _ => unreachable!(),
        };
        assert_eq!(p1.fill_color.as_deref(), Some("#inline"));
        let p2 = match &model.objects[2] {
            VectorRenderObject::Path(p) => p,
            _ => unreachable!(),
        };
        assert_eq!(p2.stroke_color.as_deref(), Some("#bbb"));
    }

    #[test]
    fn serde_json_round_trip_with_palette_fields() {
        let json = r##"{
            "pageIndex": 0, "width": 595, "height": 842,
            "palette": {"colors": ["#ff0000"]},
            "objects": [{"type":"path","id":"p1","segments":[],
                "fillColor":null, "fillColorIndex":0,
                "strokeColor":null, "strokeColorIndex":null,
                "fill":true, "stroke":false, "strokeWidth":1.0, "zIndex":0}]
        }"##;
        let model: VectorPageModel = serde_json::from_str(json).unwrap();
        assert_eq!(model.palette.colors, vec!["#ff0000"]);
        match &model.objects[0] {
            VectorRenderObject::Path(p) => {
                assert_eq!(p.fill_color_index, Some(0));
                assert_eq!(p.fill_color, None);
            }
            _ => panic!("expected path"),
        }
    }
}
