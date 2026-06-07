//! 宏观页面域上下文树投影与序列化桥接 (Page Region Context & Projection)
//!
//! # Overview
//! 在 PDF 文档解析的管道末端，原生的 `VectorPageModel` 被送入本模块。
//! 它的核心任务是完成 **"物理态" 到 "纯渲染态 (Snapshot)"** 的结构降维。
//!
//! # Architectural Rationale (架构缘由)
//! 为什么我们需要多出这么多带有 `Snapshot` 和 `Output` 后缀的类型？
//! 因为 Rust 端拥有的复杂指针引用、生命周期约束和庞大的全局 PDF 字体上下文，
//! 无法直接穿越 FFI 边界或者 WASM IPC 总线去到 React 前端。
//! 此模块定义的结构体全部为**绝对展平 (Flattened) 数据传输对象 (DTO)**。
//!
//! # Invariants
//! 全部结构体默认 `Y-Down` 坐标系，`top` 值随肉眼视界向下增加。

use super::list_item_region_builder::build_list_item_region;
pub use super::page_region_models::*;
use crate::models::{
    EditableSegment, FontHints, LayoutRole, NativePageModel, NativePageObject, NativeTextModel,
    SemanticRole, StyledRun,
};

fn read_object_display_text(obj: &NativeTextModel) -> String {
    if !obj.runs.is_empty() {
        obj.runs.iter().map(|run| run.text.as_str()).collect()
    } else {
        obj.text.clone()
    }
}

fn chars_count(text: &str) -> usize {
    text.chars().count()
}

fn resolve_run_visible_glyph_width(run: &StyledRun, parent: &NativeTextModel) -> f32 {
    if run.width > 0.0 {
        run.width
    } else if parent.width > 0.0 && !parent.text.is_empty() {
        parent.width / chars_count(&parent.text).max(1) as f32
    } else {
        run.font_size.max(parent.font_size).max(1.0)
    }
}

fn build_style_source(
    font_name: String,
    font_size: f32,
    color: String,
    is_bold: bool,
    is_italic: bool,
    is_underline: bool,
    font_hints: Option<FontHints>,
    render_mode: i64,
    char_spacing: f32,
    scale_x: f32,
) -> StyleSource {
    StyleSource {
        font_name,
        font_size,
        color,
        is_bold,
        is_italic,
        is_underline,
        font_hints,
        render_mode,
        char_spacing,
        scale_x,
    }
}

fn build_style_runs_from_text_object(
    obj: &NativeTextModel,
    line_key: &str,
) -> Vec<StyleRunSnapshot> {
    let line_left = obj.tx;
    let mut char_offset = 0usize;
    let mut runs_out = Vec::new();

    if !obj.runs.is_empty() {
        for (run_index, run) in obj.runs.iter().enumerate() {
            let run_text = run.text.clone();
            let glyph_count = chars_count(&run_text);
            if glyph_count == 0 {
                continue;
            }
            let run_left = run.tx - line_left;
            let measured_width = if run.width > 0.0 {
                run.width
            } else {
                resolve_run_visible_glyph_width(run, obj)
            };
            let fallback_char_width = run
                .char_widths
                .iter()
                .copied()
                .find(|value| value.is_finite() && *value > 0.0)
                .unwrap_or_else(|| {
                    if measured_width > 0.0 {
                        measured_width / glyph_count as f32
                    } else {
                        obj.width.max(run.font_size).max(1.0) / glyph_count as f32
                    }
                });

            let next_char_origins = if !run.char_origins.is_empty() {
                run.char_origins
                    .iter()
                    .map(|origin| run_left + origin)
                    .collect::<Vec<_>>()
            } else {
                (0..glyph_count)
                    .map(|char_index| run_left + (char_index as f32 * fallback_char_width))
                    .collect::<Vec<_>>()
            };
            let next_char_widths = if run.char_widths.len() == glyph_count {
                run.char_widths.clone()
            } else {
                (0..glyph_count)
                    .map(|char_index| {
                        let explicit = run.char_widths.get(char_index).copied().unwrap_or_default();
                        if explicit.is_finite() && explicit > 0.0 {
                            explicit
                        } else if char_index + 1 < next_char_origins.len() {
                            let delta =
                                next_char_origins[char_index + 1] - next_char_origins[char_index];
                            if delta.is_finite() && delta > 0.0 {
                                delta
                            } else {
                                fallback_char_width
                            }
                        } else {
                            fallback_char_width
                        }
                    })
                    .collect::<Vec<_>>()
            };
            let visible_width = if !next_char_origins.is_empty() && !next_char_widths.is_empty() {
                let first = next_char_origins[0];
                let last_index = usize::min(next_char_origins.len(), next_char_widths.len()) - 1;
                let right = next_char_origins[last_index] + next_char_widths[last_index];
                (right - first).max(0.0)
            } else {
                measured_width
            };
            let length = glyph_count;
            runs_out.push(StyleRunSnapshot {
                id: format!("{line_key}::run::{run_index}"),
                text: run_text,
                start: char_offset,
                end: char_offset + length,
                style: build_style_source(
                    if run.font_name.is_empty() {
                        obj.font_name.clone()
                    } else {
                        run.font_name.clone()
                    },
                    if run.font_size > 0.0 {
                        run.font_size
                    } else {
                        obj.font_size
                    },
                    if run.color.is_empty() {
                        obj.color.clone()
                    } else {
                        run.color.clone()
                    },
                    run.is_bold || obj.is_bold,
                    run.is_italic || obj.is_italic,
                    run.is_underline || obj.is_underline,
                    run.font_hints.clone().or_else(|| obj.font_hints.clone()),
                    if run.render_mode != 0 {
                        run.render_mode
                    } else {
                        obj.render_mode
                    },
                    run.char_spacing,
                    run.horizontal_scaling,
                ),
                width: visible_width,
                char_origins: next_char_origins,
                char_widths: next_char_widths,
                object_ids: run
                    .object_id
                    .clone()
                    .map(|value| vec![value])
                    .unwrap_or_else(|| vec![obj.id.clone()]),
                object_indices: if obj.object_indices.contains(&run.z_index) {
                    vec![run.z_index]
                } else if run.z_index == 0 && obj.object_indices.len() == 1 {
                    obj.object_indices.clone()
                } else {
                    vec![run.z_index]
                },
            });
            char_offset += length;
        }
        if !runs_out.is_empty() {
            return runs_out;
        }
    }

    let text = read_object_display_text(obj);
    let glyph_count = chars_count(&text);
    let char_width = if glyph_count > 0 {
        obj.width / glyph_count as f32
    } else {
        obj.font_size
    };
    vec![StyleRunSnapshot {
        id: format!("{line_key}::run::0"),
        text,
        start: 0,
        end: glyph_count,
        style: build_style_source(
            obj.font_name.clone(),
            obj.font_size,
            obj.color.clone(),
            obj.is_bold,
            obj.is_italic,
            obj.is_underline,
            obj.font_hints.clone(),
            obj.render_mode,
            obj.char_spacing,
            obj.horizontal_scaling,
        ),
        width: obj.width,
        char_origins: (0..glyph_count).map(|i| i as f32 * char_width).collect(),
        char_widths: vec![],
        object_ids: vec![obj.id.clone()],
        object_indices: obj.object_indices.clone(),
    }]
}

fn build_paragraph_line_from_text_object(
    obj: &NativeTextModel,
    line_index: usize,
    page_height: f32,
) -> ParagraphLineOutput {
    let text = read_object_display_text(obj);
    let style_runs =
        build_style_runs_from_text_object(obj, &format!("{}::line::{}", obj.id, line_index));

    let region_box = BoundingBoxOutput {
        left: obj.tx,
        top: page_height - obj.ty - obj.height,
        width: obj.width.max(1.0),
        height: obj.height.max(1.0),
    };
    let line_boxes = vec![ParagraphLineProjectionOutput {
        line_index,
        left: obj.tx,
        top: page_height - obj.ty - obj.height,
        width: obj.width.max(1.0),
        height: obj.height.max(1.0),
        baseline_y: page_height - obj.ty,
    }];
    let projection = ParagraphProjectionOutput {
        region_id: obj.paragraph_id.clone().unwrap_or_else(|| obj.id.clone()),
        kind: "paragraph".into(),
        region_box,
        line_boxes: line_boxes.clone(),
        tight_line_boxes: line_boxes,
    };

    ParagraphLineOutput {
        line_index,
        text,
        left: obj.tx,
        right: obj.tx + obj.width,
        top: obj.ty + obj.height,
        bottom: obj.ty,
        font_name: obj.font_name.clone(),
        font_size: obj.font_size,
        color: obj.color.clone(),
        is_bold: obj.is_bold,
        is_italic: obj.is_italic,
        is_underline: obj.is_underline,
        char_spacing: obj.runs.get(0).map(|r| r.char_spacing).unwrap_or(0.0),
        scale_x: obj
            .runs
            .get(0)
            .map(|r| r.horizontal_scaling)
            .unwrap_or(100.0),
        font_hints: obj.font_hints.clone(),
        render_mode: obj.render_mode,
        object_ids: vec![obj.id.clone()],
        object_indices: obj.object_indices.clone(),
        width: obj.width,
        char_origins: style_runs
            .iter()
            .flat_map(|run| run.char_origins.clone())
            .collect(),
        char_widths: style_runs
            .iter()
            .flat_map(|run| run.char_widths.clone())
            .collect(),
        style_runs,
        projection,
    }
}

fn infer_scene_hint(text_objects: &[NativeTextModel]) -> String {
    let combined = text_objects
        .iter()
        .take(40)
        .map(|obj| read_object_display_text(obj).trim().to_string())
        .collect::<Vec<_>>()
        .join(" ");
    if ["简历", "求职意向", "教育背景", "专业技能", "核心优势"]
        .iter()
        .any(|token| combined.contains(token))
    {
        "resume".into()
    } else if ["姓名", "性别", "电话", "邮箱", "地址"]
        .iter()
        .any(|token| combined.contains(token))
    {
        "form-like".into()
    } else {
        "generic".into()
    }
}

fn is_standalone_paragraph_candidate(obj: &NativeTextModel) -> bool {
    let text = read_object_display_text(obj);
    let trimmed = text.trim();
    obj.font_size >= 18.0
        || matches!(
            obj.role,
            Some(LayoutRole::SectionHeader | LayoutRole::Title)
        )
        || (obj.is_bold
            && chars_count(trimmed) <= 12
            && !trimmed.contains('：')
            && !trimmed.contains(':'))
}

fn should_merge_paragraph_objects(previous: &NativeTextModel, current: &NativeTextModel) -> bool {
    let previous_key = previous.paragraph_id.as_deref().unwrap_or(&previous.id);
    let current_key = current.paragraph_id.as_deref().unwrap_or(&current.id);
    if previous_key != current_key {
        return false;
    }
    if is_standalone_paragraph_candidate(previous) || is_standalone_paragraph_candidate(current) {
        return false;
    }
    let left_delta = (previous.tx - current.tx).abs();
    let vertical_gap = (previous.ty - current.ty).abs();
    let font_delta = (previous.font_size - current.font_size).abs();
    left_delta <= 24.0
        && font_delta <= 2.0
        && vertical_gap <= previous.font_size.max(current.font_size) * 2.4
}

fn build_paragraph_region_from_objects(
    objects: &[NativeTextModel],
    page_index: u16,
    line_index_by_object_id: &std::collections::HashMap<String, usize>,
    page_height: f32,
) -> ParagraphRegionOutput {
    let mut sorted_objects = objects.to_vec();
    sorted_objects.sort_by(|a, b| {
        let a_line = line_index_by_object_id
            .get(&a.id)
            .copied()
            .unwrap_or_default();
        let b_line = line_index_by_object_id
            .get(&b.id)
            .copied()
            .unwrap_or_default();
        a_line
            .cmp(&b_line)
            .then_with(|| a.tx.partial_cmp(&b.tx).unwrap_or(std::cmp::Ordering::Equal))
    });
    let lines = sorted_objects
        .iter()
        .map(|obj| {
            build_paragraph_line_from_text_object(
                obj,
                line_index_by_object_id
                    .get(&obj.id)
                    .copied()
                    .unwrap_or_default(),
                page_height,
            )
        })
        .collect::<Vec<_>>();
    let left = lines.iter().map(|line| line.left).fold(f32::MAX, f32::min);
    let right = lines.iter().map(|line| line.right).fold(f32::MIN, f32::max);
    let top_pdf = lines.iter().map(|line| line.top).fold(f32::MIN, f32::max);
    let bottom_pdf = lines
        .iter()
        .map(|line| line.bottom)
        .fold(f32::MAX, f32::min);

    let region_box = BoundingBoxOutput {
        left,
        top: page_height - top_pdf,
        width: (right - left).max(1.0),
        height: (top_pdf - bottom_pdf).max(1.0),
    };
    let line_boxes: Vec<ParagraphLineProjectionOutput> = lines
        .iter()
        .flat_map(|line| line.projection.line_boxes.clone())
        .collect();

    let id = sorted_objects
        .first()
        .and_then(|obj| obj.paragraph_id.clone().or_else(|| Some(obj.id.clone())))
        .unwrap_or_else(|| {
            format!(
                "paragraph-{}-{}",
                page_index,
                lines
                    .first()
                    .map(|line| line.line_index)
                    .unwrap_or_default()
            )
        });

    let projection = ParagraphProjectionOutput {
        region_id: id.clone(),
        kind: "paragraph".into(),
        region_box,
        line_boxes: line_boxes.clone(),
        tight_line_boxes: line_boxes,
    };

    ParagraphRegionOutput {
        kind: "paragraph".to_string(),
        id,
        page_index,
        line_index_start: lines
            .first()
            .map(|line| line.line_index)
            .unwrap_or_default(),
        line_index_end: lines.last().map(|line| line.line_index).unwrap_or_default(),
        left,
        right,
        top: top_pdf,
        bottom: bottom_pdf,
        text: lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        lines: lines.clone(),
        object_ids: sorted_objects.iter().map(|obj| obj.id.clone()).collect(),
        object_indices: sorted_objects
            .iter()
            .flat_map(|obj| obj.object_indices.clone())
            .collect(),
        width: (right - left).max(1.0),
        char_origins: lines
            .iter()
            .flat_map(|line| line.char_origins.clone())
            .collect(),
        char_widths: lines
            .iter()
            .flat_map(|line| line.char_widths.clone())
            .collect(),
        wrap_width: (right - left).max(1.0),
        char_spacing: lines.first().map(|l| l.char_spacing).unwrap_or(0.0),
        scale_x: lines.first().map(|l| l.scale_x).unwrap_or(1.0),
        projection,
    }
}
fn split_key_value_text(text: &str) -> (String, String) {
    for delimiter in ['：', ':'] {
        if let Some(idx) = text.find(delimiter) {
            let key = text[..idx].to_string();
            let value = text[idx + delimiter.len_utf8()..].to_string();
            return (key, value);
        }
    }
    (text.to_string(), String::new())
}

pub fn build_page_region_context(page: &NativePageModel) -> PageRegionContextOutput {
    let mut text_objects = page
        .objects
        .iter()
        .filter_map(|object| match object {
            NativePageObject::Text(text) => Some(text.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    text_objects.sort_by(|a, b| {
        let delta = b.ty - a.ty;
        if delta.abs() > 0.5 {
            b.ty.partial_cmp(&a.ty).unwrap_or(std::cmp::Ordering::Equal)
        } else {
            a.tx.partial_cmp(&b.tx).unwrap_or(std::cmp::Ordering::Equal)
        }
    });

    let mut line_regions = Vec::new();
    let mut paragraph_groups: Vec<Vec<NativeTextModel>> = Vec::new();
    let mut paragraph_line_index_by_object_id = std::collections::HashMap::new();
    let mut list_item_regions = Vec::new();
    let mut list_item_by_object_id = std::collections::HashMap::new();
    let mut active_paragraph_group: Vec<NativeTextModel> = Vec::new();

    let flush_paragraph_group =
        |groups: &mut Vec<Vec<NativeTextModel>>, active: &mut Vec<NativeTextModel>| {
            if !active.is_empty() {
                groups.push(std::mem::take(active));
            }
        };

    for (idx, obj) in text_objects.iter().cloned().enumerate() {
        let top = page.height - obj.ty - obj.height;
        let projection = LineProjectionOutput {
            line_index: idx,
            left: obj.tx,
            top,
            width: obj.width,
            height: obj.height,
        };
        let role = obj.role.unwrap_or(LayoutRole::Paragraph);
        let is_field = matches!(role, LayoutRole::KvField);
        let mut field_row = None;
        if is_field {
            let (key_text, value_text) = split_key_value_text(&obj.text);
            let group = FieldRowRegionGroupOutput {
                id: format!("group-{}", obj.id),
                segment_key: obj.id.clone(),
                object_ids: vec![obj.id.clone()],
                object_indices: obj.object_indices.clone(),
                run_keys: obj
                    .runs
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format!("{}::{}", obj.id, i))
                    .collect(),
                first_object_id: obj.id.clone(),
                field_name: key_text.clone(),
                field_kind: "unknown".into(),
                column_index: 0,
                left: obj.tx,
                right: obj.tx + obj.width,
                slot_left: obj.tx,
                slot_right: obj.tx + obj.width,
                label_left: obj.tx,
                label_right: obj.tx + obj.width,
                value_left: obj.tx,
                value_right: obj.tx + obj.width,
                top: obj.ty + obj.height,
                bottom: obj.ty,
                pair: KeyValuePairOutput {
                    id: format!("pair-{}", obj.id),
                    field_name: key_text.clone(),
                    field_kind: "unknown".into(),
                    key_text: key_text.clone(),
                    value_text: value_text.clone(),
                    key_style: build_style_source(
                        obj.font_name.clone(),
                        obj.font_size,
                        obj.color.clone(),
                        obj.is_bold,
                        obj.is_italic,
                        obj.is_underline,
                        obj.font_hints.clone(),
                        obj.render_mode,
                        obj.char_spacing,
                        obj.horizontal_scaling,
                    ),
                    value_style: build_style_source(
                        obj.font_name.clone(),
                        obj.font_size,
                        obj.color.clone(),
                        obj.is_bold,
                        obj.is_italic,
                        obj.is_underline,
                        obj.font_hints.clone(),
                        obj.render_mode,
                        obj.char_spacing,
                        obj.horizontal_scaling,
                    ),
                    key_object_ids: vec![obj.id.clone()],
                    value_object_ids: vec![obj.id.clone()],
                    key_run_keys: vec![],
                    value_run_keys: vec![],
                    key_object_indices: obj.object_indices.clone(),
                    value_object_indices: obj.object_indices.clone(),
                    key_box: KeyBox {
                        left: obj.tx,
                        right: obj.tx + obj.width * 0.3,
                        top: obj.ty + obj.height,
                        bottom: obj.ty,
                    },
                    value_box: KeyBox {
                        left: obj.tx + obj.width * 0.3,
                        right: obj.tx + obj.width,
                        top: obj.ty + obj.height,
                        bottom: obj.ty,
                    },
                },
                segment: EditableSegment {
                    key: obj.id.clone(),
                    object_id: obj.id.clone(),
                    start_run_index: 0,
                    end_run_index: obj.runs.len().saturating_sub(1),
                    run_indices: (0..obj.runs.len()).collect(),
                    text: obj.text.clone(),
                    width: obj.width,
                    tx: obj.tx,
                    ty: obj.ty,
                    font_size: obj.font_size,
                    font_name: obj.font_name.clone(),
                    is_bold: obj.is_bold,
                    is_italic: obj.is_italic,
                    is_underline: obj.is_underline,
                    char_spacing: obj.char_spacing,
                    scale_x: obj.horizontal_scaling,
                    color: obj.color.clone(),
                    font_hints: obj.font_hints.clone(),
                    object_indices: obj.object_indices.clone(),
                    char_origins: vec![],
                    char_widths: vec![],
                    field_group: None,
                    semantic_role: SemanticRole::None,
                },
                projection: FieldGroupProjectionOutput {
                    text_box: BoundingBoxOutput {
                        left: obj.tx,
                        top: page.height - obj.ty - obj.height,
                        width: obj.width,
                        height: obj.height,
                    },
                    shell_box: BoundingBoxOutput {
                        left: obj.tx,
                        top: page.height - obj.ty - obj.height,
                        width: obj.width,
                        height: obj.height,
                    },
                    label_box: BoundingBoxOutput {
                        left: obj.tx,
                        top: page.height - obj.ty - obj.height,
                        width: obj.width * 0.3,
                        height: obj.height,
                    },
                    value_box: BoundingBoxOutput {
                        left: obj.tx + obj.width * 0.3,
                        top: page.height - obj.ty - obj.height,
                        width: obj.width * 0.7,
                        height: obj.height,
                    },
                    editor_box: BoundingBoxOutput {
                        left: obj.tx,
                        top: page.height - obj.ty - obj.height,
                        width: obj.width,
                        height: obj.height,
                    },
                },
            };
            field_row = Some(FieldRowRegionOutput {
                id: obj.id.clone(),
                page_index: page.page_index,
                line_index: idx,
                left: obj.tx,
                right: obj.tx + obj.width,
                top: obj.ty + obj.height,
                bottom: obj.ty,
                confidence: 1.0,
                semantic_reason: "backend-sovereign".into(),
                column_bands: vec![],
                groups: vec![group],
            });
        }

        line_regions.push(LineRegionModelOutput {
            id: obj.id.clone(),
            kind: if is_field {
                "field-row".into()
            } else {
                "free-text".into()
            },
            page_index: page.page_index,
            line_index: idx,
            objects: vec![obj.clone()],
            projection,
            field_row,
            paragraph_region: None,
            list_item_region: None,
        });

        if is_field {
            flush_paragraph_group(&mut paragraph_groups, &mut active_paragraph_group);
            continue;
        }

        if matches!(role, LayoutRole::ListItem) {
            flush_paragraph_group(&mut paragraph_groups, &mut active_paragraph_group);
            let list_item = build_list_item_region(
                &obj,
                page.page_index,
                idx,
                page.height,
                read_object_display_text(&obj),
                build_style_runs_from_text_object(&obj, &format!("{}::line::{}", obj.id, idx)),
            );
            list_item_by_object_id.insert(obj.id.clone(), list_item.clone());
            list_item_regions.push(list_item);
            continue;
        }

        if active_paragraph_group.is_empty() {
            active_paragraph_group.push(obj.clone());
        } else if should_merge_paragraph_objects(active_paragraph_group.last().unwrap(), &obj) {
            active_paragraph_group.push(obj.clone());
        } else {
            flush_paragraph_group(&mut paragraph_groups, &mut active_paragraph_group);
            active_paragraph_group.push(obj.clone());
        }
        paragraph_line_index_by_object_id.insert(obj.id.clone(), idx);
    }
    flush_paragraph_group(&mut paragraph_groups, &mut active_paragraph_group);

    let paragraph_regions = paragraph_groups
        .iter()
        .filter(|objects| !objects.is_empty())
        .map(|objects| {
            build_paragraph_region_from_objects(
                objects,
                page.page_index,
                &paragraph_line_index_by_object_id,
                page.height,
            )
        })
        .collect::<Vec<_>>();
    let mut paragraph_region_by_object_id = std::collections::HashMap::new();
    for region in &paragraph_regions {
        for object_id in &region.object_ids {
            paragraph_region_by_object_id.insert(object_id.clone(), region.clone());
        }
    }
    for line_region in &mut line_regions {
        if line_region.field_row.is_some() {
            continue;
        }
        let first_object = line_region.objects.first().cloned();
        if let Some(first_object) = first_object {
            if let Some(list_item_region) = list_item_by_object_id.get(&first_object.id).cloned() {
                line_region.list_item_region = Some(list_item_region);
            } else if let Some(paragraph_region) =
                paragraph_region_by_object_id.get(&first_object.id).cloned()
            {
                line_region.paragraph_region = Some(paragraph_region);
            }
        }
    }

    PageRegionContextOutput {
        scene_hint: infer_scene_hint(&text_objects),
        text_objects: text_objects.clone(),
        visual_lines: text_objects.iter().cloned().map(|obj| vec![obj]).collect(),
        field_rows: line_regions
            .iter()
            .filter_map(|line| line.field_row.clone())
            .collect(),
        paragraph_regions,
        list_item_regions,
        line_regions,
    }
}
