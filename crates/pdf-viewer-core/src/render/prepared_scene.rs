//! 预处理页面场景 — 从 ui::render::prepared_scene 迁入。
//! 用于按 viewport 高效裁剪可见 vector 对象的空间索引。

use std::collections::{HashMap, HashSet};

use crate::geometry::bbox_ops::bbox_intersects;
use crate::models::{BoundingBox, GlyphPaintPlan, VectorPageModel, VectorRenderObject};
use crate::render::viewport_culling::{path_object_bbox, styled_run_bbox};

const DEFAULT_BUCKET_SIZE_POINTS: f32 = 96.0;

#[derive(Debug, Clone, Default)]
pub struct PreparedPageScene {
    sorted_vector_indices: Vec<usize>,
    vector_bboxes: Vec<Option<BoundingBox>>,
    vector_bucket_index: HashMap<(i32, i32), Vec<usize>>,
    active_text_object_ids_by_paragraph: HashMap<String, HashSet<String>>,
}

impl PreparedPageScene {
    pub fn build(
        vector_model: Option<&VectorPageModel>,
        paint_plan: Option<&GlyphPaintPlan>,
    ) -> Option<Self> {
        let mut scene = Self::default();

        if let Some(vector_model) = vector_model {
            scene.sorted_vector_indices = (0..vector_model.objects.len()).collect();
            scene
                .sorted_vector_indices
                .sort_by_key(|index| match &vector_model.objects[*index] {
                    VectorRenderObject::Text(text) => text.z_index,
                    VectorRenderObject::Path(path) => path.z_index,
                    VectorRenderObject::Image(image) => image.z_index,
                });

            scene.vector_bboxes = vector_model
                .objects
                .iter()
                .map(vector_object_bbox)
                .collect();

            for (index, bbox) in scene.vector_bboxes.iter().enumerate() {
                let Some(bbox) = bbox else {
                    continue;
                };
                for bucket_key in resolve_bucket_keys(bbox) {
                    scene
                        .vector_bucket_index
                        .entry(bucket_key)
                        .or_default()
                        .push(index);
                }
            }
        }

        if let Some(paint_plan) = paint_plan {
            for region in &paint_plan.regions {
                for paragraph in &region.paragraphs {
                    let mut object_ids = HashSet::new();
                    for run in &paragraph.runs {
                        for object_id in &run.object_ids {
                            object_ids.insert(object_id.clone());
                        }
                    }
                    scene
                        .active_text_object_ids_by_paragraph
                        .insert(paragraph.id.clone(), object_ids);
                }
            }
        }

        if scene.sorted_vector_indices.is_empty()
            && scene.active_text_object_ids_by_paragraph.is_empty()
        {
            None
        } else {
            Some(scene)
        }
    }

    pub fn visible_vector_indices(&self, viewport: &BoundingBox) -> Vec<usize> {
        if self.sorted_vector_indices.is_empty() {
            return Vec::new();
        }

        let mut visible_candidates = HashSet::<usize>::new();
        for bucket_key in resolve_bucket_keys(viewport) {
            if let Some(indices) = self.vector_bucket_index.get(&bucket_key) {
                visible_candidates.extend(indices.iter().copied());
            }
        }

        let has_bucket_hits = !visible_candidates.is_empty();
        self.sorted_vector_indices
            .iter()
            .copied()
            .filter(|index| {
                if has_bucket_hits && !visible_candidates.contains(index) {
                    return false;
                }
                self.vector_bboxes
                    .get(*index)
                    .and_then(|bbox| bbox.as_ref())
                    .map(|bbox| bbox_intersects(bbox, viewport))
                    .unwrap_or(false)
            })
            .collect()
    }

    pub fn active_text_object_ids(&self, paragraph_id: &str) -> Option<&HashSet<String>> {
        self.active_text_object_ids_by_paragraph.get(paragraph_id)
    }
}

fn vector_object_bbox(object: &VectorRenderObject) -> Option<BoundingBox> {
    match object {
        VectorRenderObject::Text(text) => {
            let mut merged: Option<BoundingBox> = None;
            for run in &text.runs {
                let bbox = styled_run_bbox(run);
                merged = Some(match merged {
                    Some(current) => BoundingBox {
                        left: current.left.min(bbox.left),
                        top: current.top.min(bbox.top),
                        right: current.right.max(bbox.right),
                        bottom: current.bottom.max(bbox.bottom),
                    },
                    None => bbox,
                });
            }
            merged
        }
        VectorRenderObject::Path(path) => path_object_bbox(path),
        VectorRenderObject::Image(image) => Some(BoundingBox {
            left: image.x,
            top: image.y,
            right: image.x + image.width.max(0.0),
            bottom: image.y + image.height.max(0.0),
        }),
    }
}

fn resolve_bucket_keys(bbox: &BoundingBox) -> Vec<(i32, i32)> {
    let min_x = (bbox.left / DEFAULT_BUCKET_SIZE_POINTS).floor() as i32;
    let max_x = (bbox.right / DEFAULT_BUCKET_SIZE_POINTS).floor() as i32;
    let min_y = (bbox.top / DEFAULT_BUCKET_SIZE_POINTS).floor() as i32;
    let max_y = (bbox.bottom / DEFAULT_BUCKET_SIZE_POINTS).floor() as i32;

    let mut keys = Vec::new();

    for bucket_y in min_y..=max_y {
        for bucket_x in min_x..=max_x {
            keys.push((bucket_x, bucket_y));
        }
    }
    keys
}
