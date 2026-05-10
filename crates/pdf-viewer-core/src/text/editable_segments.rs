use crate::models::{EditableFieldGroup, EditableSegment, FieldKind, NativeTextModel, StyledRun, SemanticRole};

#[derive(Clone, Debug)]
struct FieldLabelAnchor {
    start: usize,
    end: usize,
    field_name: Option<String>,
}

#[derive(Clone, Debug)]
struct FieldGroup {
    label: FieldLabelAnchor,
    value_start: usize,
    value_end: usize,
    field_name: String,
    field_kind: FieldKind,
}

fn get_segment_patch_key(object_id: &str, start: usize, end: usize) -> String {
    format!("{object_id}::{start}-{end}")
}

fn is_colon_token(run: Option<&StyledRun>) -> bool {
    match run {
        Some(run) => matches!(run.text.trim(), ":" | "：" ),
        None => false,
    }
}

fn looks_like_short_field_token(run: &StyledRun) -> bool {
    let text = run.text.trim();
    !text.is_empty() && text.chars().count() <= 6
}

fn get_run_visible_glyph_width(run: &StyledRun, parent: &NativeTextModel) -> f32 {
    if run.width > 0.0 {
        run.width
    } else if parent.width > 0.0 && !parent.text.is_empty() {
        let count = parent.text.chars().count().max(1) as f32;
        parent.width / count
    } else {
        run.font_size.max(parent.font_size).max(1.0)
    }
}

fn get_run_style_signature(run: &StyledRun, parent: &NativeTextModel) -> String {
    let hints = run.font_hints.as_ref().or(parent.font_hints.as_ref());
    format!(
        "{}|{:.3}|{}|{}|{}|{:.3}|{:.3}|{:.3}|{:.3}|{}|{}|{}|{}",
        run.font_name,
        run.font_size,
        run.color,
        if run.is_bold { 1 } else { 0 },
        if run.is_italic { 1 } else { 0 },
        run.a,
        run.b,
        run.c,
        run.d,
        hints.map(|h| h.weight).unwrap_or_default(),
        hints.map(|h| h.italic_angle).unwrap_or_default(),
        hints.map(|h| if h.is_bold { 1 } else { 0 }).unwrap_or_default(),
        hints.map(|h| if h.is_italic { 1 } else { 0 }).unwrap_or_default(),
    )
}

fn normalize_field_label(text: &str) -> String {
    text.replace(['：', ':', ' '], "")
}

/// 通用定界符探测：不再依赖硬编码字典，仅依靠几何阵型识别 K-V 锚点
fn detect_field_label_anchors(runs: &[StyledRun]) -> Vec<FieldLabelAnchor> {
    let mut anchors = Vec::new();
    let mut i = 0;
    while i < runs.len() {
        let mut best_end = None;
        let mut combined = String::new();
        
        // 探测 [文本] + [:] 的模式
        for j in i..usize::min(runs.len(), i + 5) {
            let text = runs[j].text.trim();
            if text.is_empty() {
                break;
            }
            // 模式 A: 当前 Run 是冒号
            if is_colon_token(runs.get(j)) {
                let label = normalize_field_label(&combined);
                if !label.is_empty() && label.chars().count() <= 10 {
                    best_end = Some(j);
                }
                break;
            }
            // 模式 B: 当前 Run 以冒号结尾
            if text.ends_with('：') || text.ends_with(':') {
                combined.push_str(text);
                let label = normalize_field_label(&combined);
                if !label.is_empty() && label.chars().count() <= 10 {
                    best_end = Some(j);
                }
                break;
            }
            // 模式 C: 连续短文本块 (可能是标签的一部分)
            if !looks_like_short_field_token(&runs[j]) {
                break;
            }
            combined.push_str(text);
            if combined.chars().count() > 10 {
                break;
            }
        }

        if let Some(end) = best_end {
            anchors.push(FieldLabelAnchor {
                start: i,
                end,
                field_name: None,
            });
            i = end + 1;
        } else {
            i += 1;
        }
    }
    anchors
}

fn build_field_groups(runs: &[StyledRun]) -> Vec<FieldGroup> {
    let anchors = detect_field_label_anchors(runs);
    anchors
        .iter()
        .enumerate()
        .map(|(idx, anchor)| {
            let next_anchor = anchors.get(idx + 1);
            let value_start = anchor.end + 1;
            let value_end = next_anchor.map(|a| a.start).unwrap_or(runs.len()).saturating_sub(1);
            let field_name = anchor.field_name.clone().unwrap_or_else(|| {
                normalize_field_label(
                    &runs[anchor.start..=anchor.end]
                        .iter()
                        .map(|run| run.text.as_str())
                        .collect::<String>(),
                )
            });
            FieldGroup {
                label: anchor.clone(),
                value_start,
                value_end,
                field_name: field_name.clone(),
                field_kind: FieldKind::Unknown, // 彻底解耦业务逻辑
            }
        })
        .collect()
}

fn create_editable_segment(
    text_model: &NativeTextModel,
    start: usize,
    end: usize,
    field_group: Option<EditableFieldGroup>,
) -> EditableSegment {
    let first_run = &text_model.runs[start];
    let run_indices: Vec<usize> = (start..=end).collect();
    let text = run_indices.iter().map(|idx| text_model.runs[*idx].text.as_str()).collect::<String>();
    let mut segment_right = first_run.tx + get_run_visible_glyph_width(first_run, text_model);
    for idx in (start + 1)..=end {
        let run = &text_model.runs[idx];
        segment_right = segment_right.max(run.tx + get_run_visible_glyph_width(run, text_model));
    }
    let tx = first_run.tx;
    let runs: Vec<&StyledRun> = run_indices.iter().map(|idx| &text_model.runs[*idx]).collect();
    let has_all_origins = runs.iter().all(|run| !run.char_origins.is_empty());
    let char_origins = if has_all_origins {
        runs.iter()
            .flat_map(|run| run.char_origins.iter().map(|v| *v + (run.tx - tx)))
            .collect()
    } else {
        Vec::new()
    };
    let char_widths = if runs.iter().all(|run| !run.char_widths.is_empty()) {
        runs.iter().flat_map(|run| run.char_widths.iter().copied()).collect()
    } else {
        Vec::new()
    };
    let object_indices = run_indices.iter().map(|idx| text_model.runs[*idx].z_index).collect();

    EditableSegment {
        key: get_segment_patch_key(&text_model.id, start, end),
        object_id: text_model.id.clone(),
        start_run_index: start,
        end_run_index: end,
        run_indices,
        text,
        width: segment_right - first_run.tx,
        tx,
        ty: first_run.ty,
        font_size: first_run.font_size,
        font_name: first_run.font_name.clone(),
        is_bold: first_run.is_bold,
        is_italic: first_run.is_italic,
        is_underline: first_run.is_underline,
        char_spacing: first_run.char_spacing,
        scale_x: first_run.horizontal_scaling,
        color: first_run.color.clone(),
        font_hints: first_run.font_hints.clone().or_else(|| text_model.font_hints.clone()),
        object_indices,
        char_origins,
        char_widths,
        field_group,
        semantic_role: SemanticRole::None,
    }
}

fn build_contiguous_segments_in_range(text_model: &NativeTextModel, start: usize, end: usize) -> Vec<EditableSegment> {
    let mut segments = Vec::new();
    let mut cursor = start;
    while cursor <= end {
        let first_run = &text_model.runs[cursor];
        let style_signature = get_run_style_signature(first_run, text_model);
        let mut seg_end = cursor;
        while seg_end + 1 <= end {
            let next = &text_model.runs[seg_end + 1];
            if get_run_style_signature(next, text_model) != style_signature {
                break;
            }
            let prev = &text_model.runs[seg_end];
            let prev_visible_width = get_run_visible_glyph_width(prev, text_model);
            let expected_next_tx = prev.tx + prev_visible_width;
            let geometric_gap = next.tx - expected_next_tx;
            let visible_char_count = prev.text.chars().count().max(1) as f32;
            let avg_glyph_width = (prev_visible_width / visible_char_count).max(1.0);
            let contiguous_join_gap = (prev.font_size * 0.2).max(avg_glyph_width * 0.9).max(1.8);
            if geometric_gap > contiguous_join_gap {
                break;
            }
            seg_end += 1;
        }
        segments.push(create_editable_segment(text_model, cursor, seg_end, None));
        cursor = seg_end + 1;
    }
    segments
}

use crate::text::semantic_axiom::AxiomEngine;

pub fn build_editable_segments(text_model: &NativeTextModel, page_height: f32) -> Vec<EditableSegment> {
    if text_model.runs.is_empty() {
        return Vec::new();
    }
    let field_groups = build_field_groups(&text_model.runs);
    if field_groups.is_empty() {
        let mut segments = build_contiguous_segments_in_range(text_model, 0, text_model.runs.len() - 1);
        for seg in &mut segments {
            seg.semantic_role = AxiomEngine::infer_role(seg, text_model, page_height);
        }
        return segments;
    }

    let mut segments = Vec::new();
    let mut cursor = 0usize;
    for group in field_groups {
        if cursor < group.label.start {
            segments.extend(build_contiguous_segments_in_range(text_model, cursor, group.label.start - 1));
        }
        let group_start = group.label.start;
        let group_end = group.label.end.max(group.value_end);
        let label_text = text_model.runs[group.label.start..=group.label.end]
            .iter()
            .map(|run| run.text.as_str())
            .collect::<String>();
        let value_text = if group.value_start <= group.value_end {
            text_model.runs[group.value_start..=group.value_end]
                .iter()
                .map(|run| run.text.as_str())
                .collect::<String>()
        } else {
            String::new()
        };
        let mut segment = create_editable_segment(
            text_model,
            group_start,
            group_end,
            Some(EditableFieldGroup {
                label_text: label_text.clone(),
                value_text: value_text.clone(),
                value_start_index: label_text.chars().count(),
                field_name: group.field_name.clone(),
                field_kind: group.field_kind,
                label_start_run_index: group.label.start,
                label_end_run_index: group.label.end,
                value_start_run_index: group.value_start,
                value_end_run_index: group.value_end,
                semantic_role: SemanticRole::None, // Initial, will be refined
            }),
        );
        segment.semantic_role = AxiomEngine::infer_role(&segment, text_model, page_height);
        if let Some(ref mut gf) = segment.field_group {
            gf.semantic_role = segment.semantic_role;
        }
        segments.push(segment);
        cursor = if group.value_start > group.value_end {
            group.label.end + 1
        } else {
            group.value_end + 1
        };
    }

    if cursor < text_model.runs.len() {
        let mut tail_segments = build_contiguous_segments_in_range(text_model, cursor, text_model.runs.len() - 1);
        for seg in &mut tail_segments {
            seg.semantic_role = AxiomEngine::infer_role(seg, text_model, page_height);
        }
        segments.extend(tail_segments);
    }
    segments
}
