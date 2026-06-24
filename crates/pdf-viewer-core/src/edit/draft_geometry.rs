//! Draft geometry — 几何与光标转换子模块。

use crate::geometry::layout_engine::ParagraphLayout;
use crate::models::LayoutRun;
use super::draft_types::{DraftCaretLine, DraftCaretStop};

pub(super) fn align_layout_baseline(layout: &mut ParagraphLayout, target_baseline_y: f32) {
    let Some(first_line) = layout.lines.first() else {
        return;
    };
    let baseline_offset = target_baseline_y - first_line.baseline_y;
    if baseline_offset.abs() <= f32::EPSILON {
        return;
    }
    for line in &mut layout.lines {
        line.baseline_y += baseline_offset;
    }
}

pub(super) fn build_editor_draft_caret_plan_from_layout<F>(
    layout: &ParagraphLayout,
    measure_width: F,
) -> Vec<DraftCaretLine>
where
    F: Fn(&str, &LayoutRun) -> f32,
{
    let mut lines = Vec::new();
    let mut consumed = 0usize;

    for line in &layout.lines {
        let mut caret_line = DraftCaretLine {
            baseline_y: line.baseline_y,
            height: line
                .runs
                .iter()
                .map(|run| run.style.font_size.max(1.0))
                .fold(1.0, f32::max),
            stops: Vec::new(),
        };

        for run in &line.runs {
            let start_index = consumed;
            let run_origin_x = line.offset_x + run.origin_x;
            let chars: Vec<char> = run.text.chars().collect();
            let glyph_count = chars.len();
            if glyph_count == 0 {
                continue;
            }

            if !run.char_origins.is_empty() {
                let first_origin = run.char_origins.first().copied().unwrap_or(0.0);
                caret_line.stops.push(DraftCaretStop {
                    index: start_index,
                    left: run_origin_x + first_origin,
                });
                for glyph_index in 0..glyph_count {
                    let origin = run
                        .char_origins
                        .get(glyph_index)
                        .copied()
                        .unwrap_or(first_origin);
                    let right = if let Some(width) = run.char_widths.get(glyph_index).copied() {
                        origin + width
                    } else if let Some(next_origin) = run.char_origins.get(glyph_index + 1).copied()
                    {
                        next_origin
                    } else {
                        let mut buf = [0_u8; 4];
                        let glyph = chars[glyph_index].encode_utf8(&mut buf);
                        origin + measure_width(glyph, run)
                    };
                    caret_line.stops.push(DraftCaretStop {
                        index: start_index + glyph_index + 1,
                        left: run_origin_x + right,
                    });
                }
                consumed += glyph_count;
                continue;
            }

            caret_line.stops.push(DraftCaretStop {
                index: start_index,
                left: run_origin_x,
            });
            let mut prefix = String::new();
            for (glyph_index, ch) in chars.iter().enumerate() {
                prefix.push(*ch);
                let prefix_width = measure_width(&prefix, run);
                caret_line.stops.push(DraftCaretStop {
                    index: start_index + glyph_index + 1,
                    left: run_origin_x + prefix_width,
                });
            }
            consumed += glyph_count;
        }

        if caret_line.stops.is_empty() {
            caret_line.stops.push(DraftCaretStop {
                index: consumed,
                left: line.offset_x,
            });
        }

        lines.push(caret_line);
    }

    if lines.is_empty() {
        lines.push(DraftCaretLine {
            baseline_y: 0.0,
            height: 1.0,
            stops: vec![DraftCaretStop {
                index: 0,
                left: 0.0,
            }],
        });
    }

    lines
}
