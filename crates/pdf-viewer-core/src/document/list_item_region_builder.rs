use crate::text::list_semantics::derive_list_text_semantics;
use crate::models::NativeTextModel;

use super::page_region_models::{
    BoundingBoxOutput, ListItemRegionOutput, ParagraphLineOutput, ParagraphLineProjectionOutput,
    ParagraphProjectionOutput, StyleRunSnapshot,
};

fn chars_count(text: &str) -> usize {
    text.chars().count()
}

fn split_runs_by_body_start(
    runs: &[StyleRunSnapshot],
    body_char_start: usize,
    line_key: &str,
) -> (Vec<StyleRunSnapshot>, Vec<StyleRunSnapshot>) {
    let mut marker_runs = Vec::new();
    let mut body_runs = Vec::new();
    let mut global_cursor = 0usize;

    for (run_index, run) in runs.iter().enumerate() {
        let run_len = chars_count(&run.text);
        let run_start = global_cursor;
        let run_end = global_cursor + run_len;

        if run_end <= body_char_start {
            let mut cloned = run.clone();
            cloned.id = format!("{line_key}::marker::{run_index}");
            cloned.start = 0;
            cloned.end = run_len;
            marker_runs.push(cloned);
        } else if run_start >= body_char_start {
            let mut cloned = run.clone();
            cloned.id = format!("{line_key}::body::{run_index}");
            cloned.start = 0;
            cloned.end = run_len;
            body_runs.push(cloned);
        } else {
            let marker_count = body_char_start.saturating_sub(run_start);
            let chars = run.text.chars().collect::<Vec<_>>();

            if marker_count > 0 {
                marker_runs.push(StyleRunSnapshot {
                    id: format!("{line_key}::marker::{run_index}"),
                    text: chars[..marker_count].iter().collect(),
                    start: 0,
                    end: marker_count,
                    style: run.style.clone(),
                    width: run.width,
                    char_origins: run.char_origins.iter().copied().take(marker_count).collect(),
                    char_widths: run.char_widths.iter().copied().take(marker_count).collect(),
                    object_ids: run.object_ids.clone(),
                    object_indices: run.object_indices.clone(),
                });
            }

            if marker_count < chars.len() {
                let body_origins = run
                    .char_origins
                    .iter()
                    .copied()
                    .skip(marker_count)
                    .collect::<Vec<_>>();
                let first_origin = body_origins.first().copied().unwrap_or_default();
                body_runs.push(StyleRunSnapshot {
                    id: format!("{line_key}::body::{run_index}"),
                    text: chars[marker_count..].iter().collect(),
                    start: 0,
                    end: chars.len() - marker_count,
                    style: run.style.clone(),
                    width: run.width,
                    char_origins: body_origins.into_iter().map(|value| value - first_origin).collect(),
                    char_widths: run.char_widths.iter().copied().skip(marker_count).collect(),
                    object_ids: run.object_ids.clone(),
                    object_indices: run.object_indices.clone(),
                });
            }
        }

        global_cursor = run_end;
    }

    (marker_runs, body_runs)
}

fn resolve_body_left(
    line: &ParagraphLineOutput,
    body_char_start: usize,
    marker_runs: &[StyleRunSnapshot],
) -> f32 {
    if let Some(origin) = line.char_origins.get(body_char_start) {
        return line.left + *origin;
    }

    if let Some(last_marker_run) = marker_runs.last() {
        if !last_marker_run.char_origins.is_empty() {
            let last_index = last_marker_run.char_origins.len() - 1;
            return line.left
                + last_marker_run.char_origins[last_index]
                + last_marker_run
                    .char_widths
                    .get(last_index)
                    .copied()
                    .unwrap_or_default();
        }
    }

    line.left + marker_runs.iter().map(|run| run.width).sum::<f32>()
}

pub(crate) fn build_list_item_region(
    obj: &NativeTextModel,
    page_index: u16,
    line_index: usize,
    page_height: f32,
    text: String,
    raw_style_runs: Vec<StyleRunSnapshot>,
) -> ListItemRegionOutput {
    let line_key = format!("{}::line::{}", obj.id, line_index);
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
        kind: "list-item".into(),
        region_box: BoundingBoxOutput {
            left: obj.tx,
            top: page_height - obj.ty - obj.height,
            width: obj.width.max(1.0),
            height: obj.height.max(1.0),
        },
        line_boxes: line_boxes.clone(),
        tight_line_boxes: line_boxes,
    };

    let raw_line = ParagraphLineOutput {
        line_index,
        text: text.clone(),
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
        font_hints: obj.font_hints.clone(),
        render_mode: obj.render_mode,
        object_ids: vec![obj.id.clone()],
        object_indices: obj.object_indices.clone(),
        width: obj.width,
        char_origins: raw_style_runs
            .iter()
            .flat_map(|run| run.char_origins.clone())
            .collect(),
        char_widths: raw_style_runs
            .iter()
            .flat_map(|run| run.char_widths.clone())
            .collect(),
        style_runs: raw_style_runs.clone(),
        char_spacing: obj.char_spacing,
        scale_x: obj.horizontal_scaling,
        projection: projection.clone(),
    };

    let semantics = derive_list_text_semantics(&text);
    let (marker_runs, body_runs) = if semantics.has_marker {
        split_runs_by_body_start(&raw_style_runs, semantics.body_char_start, &line_key)
    } else {
        (vec![], raw_style_runs.clone())
    };
    let body_left = semantics
        .has_marker
        .then(|| resolve_body_left(&raw_line, semantics.body_char_start, &marker_runs));

    ListItemRegionOutput {
        kind: "list-item".to_string(),
        id: obj.paragraph_id.clone().unwrap_or_else(|| obj.id.clone()),
        wrap_width: obj.wrap_width.unwrap_or(obj.width),
        page_index,
        line_index,
        left: obj.tx,
        right: obj.tx + obj.width,
        top: obj.ty + obj.height,
        bottom: obj.ty,
        text: text.clone(),
        marker_text: semantics.has_marker.then_some(semantics.marker_text.clone()),
        marker_char_len: semantics.has_marker.then_some(semantics.marker_char_len),
        body_char_start: semantics.has_marker.then_some(semantics.body_char_start),
        body_text: semantics.has_marker.then_some(semantics.body_text.clone()),
        body_left,
        label_text: if semantics.has_marker {
            semantics.marker_text.clone()
        } else {
            String::new()
        },
        value_text: if semantics.has_marker {
            semantics.body_text.clone()
        } else {
            text
        },
        font_name: obj.font_name.clone(),
        font_size: obj.font_size,
        color: obj.color.clone(),
        is_bold: obj.is_bold,
        is_italic: obj.is_italic,
        font_hints: obj.font_hints.clone(),
        render_mode: obj.render_mode,
        char_spacing: obj.char_spacing,
        scale_x: obj.horizontal_scaling,
        object_ids: vec![obj.id.clone()],
        object_indices: obj.object_indices.clone(),
        width: obj.width,
        char_origins: raw_line.char_origins.clone(),
        char_widths: raw_line.char_widths.clone(),
        marker_runs: semantics.has_marker.then_some(marker_runs),
        style_runs: if semantics.has_marker { body_runs } else { raw_style_runs },
        projection,
    }
}
