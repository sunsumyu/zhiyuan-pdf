use crate::document::page_region_context::{ParagraphRegionSnapshotLine, StyleRunSnapshot, StyleSource};

pub fn make_style_run(id: &str, text: &str, style: &StyleSource) -> StyleRunSnapshot {
    StyleRunSnapshot {
        id: id.to_string(),
        text: text.to_string(),
        start: 0,
        end: text.chars().count(),
        style: style.clone(),
        width: 0.0,
        char_origins: vec![],
        char_widths: vec![],
        object_ids: vec![],
        object_indices: vec![],
    }
}

pub fn reindex_style_runs(mut runs: Vec<StyleRunSnapshot>) -> Vec<StyleRunSnapshot> {
    let mut cursor = 0;
    for run in &mut runs {
        let len = run.text.chars().count();
        run.start = cursor;
        run.end = cursor + len;
        run.width = 0.0;
        run.char_origins = vec![];
        run.char_widths = vec![];
        cursor += len;
    }
    runs
}

pub fn resolve_dominant_paragraph_style(
    runs: &[StyleRunSnapshot],
    fallback: &StyleSource,
) -> StyleSource {
    if runs.is_empty() {
        return fallback.clone();
    }
    runs[0].style.clone()
}

pub fn distribute_text_across_runs(
    prefix: &str,
    text: &str,
    previous_runs: &[StyleRunSnapshot],
    fallback_style: &StyleSource,
) -> Vec<StyleRunSnapshot> {
    if text.is_empty() {
        return vec![];
    }
    if previous_runs.len() <= 1 {
        let style = previous_runs.first().map(|r| r.style.clone()).unwrap_or_else(|| fallback_style.clone());
        return reindex_style_runs(vec![make_style_run(&format!("{}::0", prefix), text, &style)]);
    }

    let chars: Vec<char> = text.chars().collect();
    let total_prev_length = previous_runs.iter().map(|r| r.text.chars().count()).sum::<usize>().max(1);
    
    let mut cursor = 0;
    let mut next_runs = Vec::new();
    
    for (idx, run) in previous_runs.iter().enumerate() {
        let run_length = run.text.chars().count();
        let expected_length = if idx == previous_runs.len() - 1 {
            chars.len().saturating_sub(cursor)
        } else {
            let ratio = (run_length as f32) / (total_prev_length as f32);
            (ratio * (chars.len() as f32)).round() as usize
        };
        
        let take = (chars.len().saturating_sub(cursor)).min(expected_length);
        if take > 0 {
            let run_text: String = chars[cursor..cursor + take].iter().collect();
            next_runs.push(make_style_run(&format!("{}::{}", prefix, idx), &run_text, &run.style));
            cursor += take;
        }
    }

    if cursor < chars.len() {
        let tail_text: String = chars[cursor..].iter().collect();
        let tail_style = previous_runs.last().map(|r| r.style.clone()).unwrap_or_else(|| fallback_style.clone());
        next_runs.push(make_style_run(&format!("{}::tail", prefix), &tail_text, &tail_style));
    }

    reindex_style_runs(next_runs)
}

fn is_decorative_run_text(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return false;
    }
    trimmed.chars().all(|ch| crate::text::glyph_layout::is_decorative_glyph(ch))
}

fn line_selection_range(text: &str, line_index: usize, selection_start: usize, selection_end: usize) -> (usize, usize) {
    let lines: Vec<&str> = text.split('\n').collect(); // Note: \r\n not fully handled but equivalent length-wise if we just split roughly or standardize on \n
    let mut offset = 0;
    
    for (i, line_text) in lines.iter().enumerate() {
        let line_start = offset;
        let line_end = offset + line_text.chars().count(); // By char count for selection consistency
        
        if i == line_index {
            let line_len = line_text.chars().count();
            let start = (selection_start.saturating_sub(line_start)).clamp(0, line_len);
            let end = (selection_end.saturating_sub(line_start)).clamp(0, line_len);
            return (start, end);
        }
        offset = line_end + 1; // +1 for the newline char
    }
    (0, 0)
}

pub fn preserve_changed_line_styles(
    region_id: &str,
    line_index: usize,
    previous_line: Option<&ParagraphRegionSnapshotLine>,
    line: &ParagraphRegionSnapshotLine,
    line_text: &str,
    selection_start: usize,
    selection_end: usize,
) -> Vec<StyleRunSnapshot> {
    let previous_runs = previous_line.map(|l| l.style_runs.clone()).unwrap_or_default();
    let mut decorative_prefix_runs = Vec::new();
    let mut decorative_prefix_text = String::new();
    
    let first_body_run = previous_runs.iter().find(|run| !is_decorative_run_text(&run.text));
    
    for run in &previous_runs {
        if !is_decorative_run_text(&run.text) {
            break;
        }
        decorative_prefix_runs.push(run.clone());
        decorative_prefix_text.push_str(&run.text);
    }
    
    let current_chars: Vec<char> = line_text.chars().collect();
    let decorative_prefix_char_len = decorative_prefix_text.chars().count();
    
    let current_has_same_decorative_prefix = decorative_prefix_char_len > 0 && 
        current_chars.len() >= decorative_prefix_char_len && 
        current_chars[..decorative_prefix_char_len].iter().collect::<String>() == decorative_prefix_text;
        
    if previous_runs.len() <= 1 {
        let fallback_style = previous_line.map(|l| StyleSource {
            font_name: l.font_name.clone(),
            font_size: l.font_size,
            color: l.color.clone(),
            is_bold: l.is_bold,
            is_italic: l.is_italic,
            is_underline: l.is_underline,
            font_hints: l.font_hints.clone(),
            render_mode: l.render_mode.unwrap_or(0),
            char_spacing: l.char_spacing,
            scale_x: l.scale_x,
        }).unwrap_or_else(|| StyleSource {
            font_name: line.font_name.clone(),
            font_size: line.font_size,
            color: line.color.clone(),
            is_bold: line.is_bold,
            is_italic: line.is_italic,
            is_underline: line.is_underline,
            font_hints: line.font_hints.clone(),
            render_mode: line.render_mode.unwrap_or(0),
            char_spacing: line.char_spacing,
            scale_x: line.scale_x,
        });
        
        let style = resolve_dominant_paragraph_style(
            if previous_runs.is_empty() { &line.style_runs } else { &previous_runs },
            &fallback_style
        );
        return reindex_style_runs(vec![make_style_run(&format!("{}::line::{}::run::0", region_id, line_index), line_text, &style)]);
    }
    
    let (range_start, _range_end) = line_selection_range(line_text, 0, selection_start, selection_end);
    let active_offset = range_start.min(current_chars.len());
    
    let mut active_run = previous_runs[0].clone();
    let mut previous_cursor = 0;
    for run in &previous_runs {
        previous_cursor += run.text.chars().count();
        if active_offset <= previous_cursor {
            active_run = run.clone();
            break;
        }
    }
    
    let mut result = Vec::new();
    if current_has_same_decorative_prefix {
        let preserved_body_start = first_body_run
            .and_then(|r| r.char_origins.first().cloned())
            .map(|o| o.max(0.0));
            
        let prefix_runs_len = decorative_prefix_runs.len();
        for (run_index, run) in decorative_prefix_runs.into_iter().enumerate() {
            let is_last = run_index == prefix_runs_len - 1;
            let mut adj_run = run.clone();
            adj_run.id = format!("{}::line::{}::decorative-prefix::{}", region_id, line_index, run_index);
            if is_last {
                if let Some(bs) = preserved_body_start {
                    adj_run.width = adj_run.width.max(bs);
                }
            }
            result.push(adj_run);
        }
    }
    
    let body_text = if current_has_same_decorative_prefix {
        current_chars[decorative_prefix_char_len..].iter().collect::<String>()
    } else {
        line_text.to_string()
    };
    
    if !body_text.is_empty() {
        result.push(make_style_run(
            &format!("{}::line::{}::changed", region_id, line_index),
            &body_text,
            &active_run.style
        ));
    }
    
    // Filter out empty and reindex
    let mut cursor = 0;
    let mut final_result = Vec::new();
    for mut run in result {
        if run.text.is_empty() { continue; }
        let len = run.text.chars().count();
        run.start = cursor;
        run.end = cursor + len;
        cursor += len;
        final_result.push(run);
    }
    final_result
}
