//! 动态流式段落重排引擎 (Dynamic Flow Reflow Engine)
//!
//! # Overview
//! 传统 PDF 是一个硬编码的“定版”死格式，所有文字依靠绝对坐标描绘，一旦修改哪怕一个单词，
//! 原生 PDF 是无法自动把后面的单词“推”入下一行的。
//! 本引擎即是在结构被 `analyzer` (拓扑推断) 组装之后，提供类似于网页浏览器引擎 (Blink/WebKit)
//! 那样的 **流式容器边界内重排 (Reflow)** 算法。
//!
//! # Invariants (不变式约定)
//! - 所有的几何排版只会在水平 (X轴) 发生推挤与拆行，垂直方向依靠 `font_size` 与 `line_height` 线性累加。
//! - 禁则排版边界保证 (Kinsoku Shori)：强约束支持中日韩 (CJK) 标点的避头尾策略，例如逗号绝对不能出现在行首。

use crate::common::debug::truncate_debug_text;
use crate::edit::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use crate::models::{BoundingBox, GlyphPaintPlan, LayoutAlignment, LayoutParagraph, LayoutRun};

/// 表示在特定的容器宽度约束下，经历过物理换行算法生成的一行“视觉行”。
///
/// # Architecture
/// 它通常是一个或多个 `LayoutRun` 的碎片集合。它保存了自身排版后结算出来的横向溢出补偿（用于实现居中和两端对齐）
/// 以及基线 Y 轴。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisualLine {
    pub runs: Vec<LayoutRun>,
    pub width: f32,
    pub height: f32,
    pub baseline_y: f32,
    pub offset_x: f32,
    pub text: String,
}

/// 表示整个段落在经历流式挤压后，展开而成的视觉行瀑布流集合。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParagraphLayout {
    pub lines: Vec<VisualLine>,
    /// 折行完毕后，该段落所占用的物理逻辑总高。
    pub height: f32,
}

// [V3.1] CJK Punctuation Rules (避头尾)
const CJK_NO_START: &str =
    "!%),.:;?]}¢°'\"†‡›»〉》」』】〕〗〞〟’﹂﹁﹂﹃﹄～~！％），．：；？］｝、。";
const CJK_NO_END: &str = "([{£$¥@〈《「『【〔〖〝’﹃﹂﹁";

fn is_no_start(c: char) -> bool {
    CJK_NO_START.contains(c)
}

fn is_no_end(c: char) -> bool {
    CJK_NO_END.contains(c)
}

fn is_forced_line_break_run(run: &LayoutRun) -> bool {
    run.text == "\n" || run.id.starts_with("editor-line-break:")
}

/// 增强版流式排版主控算法 (Word-like Flow Layout Engine)
///
/// # Overview
/// 接受一个离散组装而成的复合段落，给定一个包裹限界 (Boundary Width)，对其进行贪心折行（Greedy Line Breaking）。
///
/// # Algorithmic Complexity
/// 本算法基于逐词（Run-level）迭代游走，时间复杂度为 `O(N)`，其中 N 是一段内包含的游程数量。
/// 注意，在当前的极速模式下，不支持复杂的字元级 (Character-level) 跨骑连字元折行 (Hyphenation)。
///
/// # Features
/// - **避头尾处理**: 查表法实现 CJK 禁则排版（防止标点符号孤立飘在行首行尾）。
/// - **强制中断**: 侦测预留的 `\n` 或特定回车标识实施软折断。
/// - **前导缩进**: 根据 `first_line_indent` 自适应处理首段推位，并在发生折位时重置。
pub fn layout_paragraph<F>(
    paragraph: &LayoutParagraph,
    wrap_width: f32,
    measure_width: F,
) -> ParagraphLayout
where
    F: Fn(&str, &LayoutRun) -> f32,
{
    let style = &paragraph.style;
    let mut visual_lines = Vec::new();
    let mut current_line_runs: Vec<LayoutRun> = Vec::new();

    // 第一行考虑首行缩进
    let mut line_cursor_x = style.first_line_indent + style.left_indent;

    for run in &paragraph.runs {
        if is_forced_line_break_run(run) {
            if !current_line_runs.is_empty() {
                visual_lines.push(finish_line(
                    std::mem::take(&mut current_line_runs),
                    line_cursor_x,
                    wrap_width,
                    style.align,
                    false,
                ));
            }
            line_cursor_x = style.left_indent;
            continue;
        }

        let run_text = &run.text;
        let run_width = measure_width(run_text, run);

        // 简单换行判定 (暂不处理单词拆分)
        let is_first_run_in_line = current_line_runs.is_empty();
        let available_width = wrap_width - line_cursor_x;

        let mut should_break = !is_first_run_in_line && run_width > available_width;

        // [V3.1] 避头尾检查 (CJK Punctuation Rule)
        // 如果当前是第一个字符或者是由于避头尾强制进入当前行的，不换行
        if should_break {
            if let Some(first_char) = run_text.chars().next() {
                if is_no_start(first_char) {
                    // 禁止行首：强制留在当前行 (拉字入行)
                    should_break = false;
                }
            }
            if let Some(last_run) = current_line_runs.last() {
                if let Some(last_char) = last_run.text.chars().last() {
                    if is_no_end(last_char) {
                        // 禁止行尾：强制留在当前行 (推字入下一行？此处选择简易的拉字入行)
                        should_break = false;
                    }
                }
            }
        }

        if should_break {
            // 换行：结束当前行
            visual_lines.push(finish_line(
                std::mem::take(&mut current_line_runs),
                line_cursor_x,
                wrap_width,
                style.align,
                false,
            ));

            // 下一行起始位置 (使用左缩进，不再使用首行缩进)
            line_cursor_x = style.left_indent;
            let mut next_run = run.clone();
            next_run.origin_x = line_cursor_x;
            current_line_runs.push(next_run);
            line_cursor_x += run_width;
        } else {
            // 检查 Tab Stop 逻辑 (针对 K-V 对齐)
            let mut target_x = line_cursor_x;
            if !style.tab_stops.is_empty() {
                for &stop in &style.tab_stops {
                    if line_cursor_x < stop {
                        target_x = stop;
                        break;
                    }
                }
            }

            let mut next_run = run.clone();
            next_run.origin_x = target_x;
            current_line_runs.push(next_run);
            line_cursor_x = target_x + run_width;
        }
    }

    if !current_line_runs.is_empty() {
        visual_lines.push(finish_line(
            current_line_runs,
            line_cursor_x,
            wrap_width,
            style.align,
            true,
        ));
    }

    // 垂直布局计算 (Vertical Layout)
    let mut current_y = 0.0;
    for line in &mut visual_lines {
        let h_factor = if style.line_height > 0.0 {
            style.line_height
        } else {
            1.2
        };
        let max_font_size = line
            .runs
            .iter()
            .map(|r| r.style.font_size)
            .fold(12.0, f32::max);

        line.height = max_font_size * h_factor;
        line.baseline_y = current_y + max_font_size;
        current_y += line.height;
    }

    // ── layout result diagnostic ──
    for (line_idx, line) in visual_lines.iter().enumerate() {
        let run_summary: String = line
            .runs
            .iter()
            .enumerate()
            .take(6)
            .map(|(ri, r)| {
                format!(
                    "r{}(ox={:.1} co={} text='{}')",
                    ri,
                    r.origin_x,
                    r.char_origins.len(),
                    truncate_debug_text(&r.text, 12)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        dbg_event(
            "layout.result",
            "line",
            vec![
                dbg_field("lineIndex", line_idx),
                dbg_field("baselineY", format!("{:.2}", line.baseline_y)),
                dbg_field("offsetX", format!("{:.2}", line.offset_x)),
                dbg_field("width", format!("{:.2}", line.width)),
                dbg_field("height", format!("{:.2}", line.height)),
                dbg_field("runCount", line.runs.len()),
                dbg_field("text", truncate_debug_text(&line.text, 40)),
                dbg_field("runs", run_summary),
            ],
        );
    }
    dbg_event(
        "layout.result",
        "summary",
        vec![
            dbg_field("wrapWidth", format!("{:.2}", wrap_width)),
            dbg_field("lineCount", visual_lines.len()),
            dbg_field("totalHeight", format!("{:.2}", current_y)),
            dbg_field("paragraphId", &paragraph.id),
        ],
    );
    // ── end diagnostic ──

    ParagraphLayout {
        lines: visual_lines,
        height: current_y,
    }
}

fn finish_line(
    mut runs: Vec<LayoutRun>,
    line_width: f32,
    wrap_width: f32,
    align: LayoutAlignment,
    is_last_line: bool,
) -> VisualLine {
    let mut offset_x = 0.0;
    let remaining_space = wrap_width - line_width;

    match align {
        LayoutAlignment::Center => {
            offset_x = remaining_space / 2.0;
        }
        LayoutAlignment::Right => {
            offset_x = remaining_space;
        }
        LayoutAlignment::Justify => {
            // 只有非最后一行且有多个运行块时才进行两端对齐
            if !is_last_line && runs.len() > 1 && remaining_space > 0.0 {
                let extra_gap = remaining_space / (runs.len() - 1) as f32;
                let mut current_extra = 0.0;
                for i in 1..runs.len() {
                    current_extra += extra_gap;
                    runs[i].origin_x += current_extra;
                }
            }
        }
        _ => {}
    }

    let line = VisualLine {
        text: runs.iter().map(|r| r.text.as_str()).collect(),
        runs,
        width: line_width,
        height: 0.0,
        baseline_y: 0.0,
        offset_x,
    };

    line
}

/// [NEW] 锚定排版策略 (Anchored Strategy)
/// 处理列表项符号或 KV 标签与主体内容的相对关系
pub fn layout_anchored_pair(
    anchor_run: &LayoutRun,
    body_paragraph: &LayoutParagraph,
    gap: f32,
) -> (LayoutRun, LayoutParagraph) {
    let new_anchor = anchor_run.clone();
    let mut new_body = body_paragraph.clone();

    // 将 body 放置在 anchor 之后
    let anchor_right = anchor_run.bbox.right;
    new_body.origin_x = anchor_right + gap;

    // 对齐基线 (Baseline Alignment)
    new_body.origin_y = anchor_run.origin_y;

    (new_anchor, new_body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{LayoutAlignment, LayoutRun, ParagraphStyle, RunStyle};

    fn mock_run(text: &str, _width: f32) -> LayoutRun {
        LayoutRun {
            id: "test".into(),
            text: text.into(),
            style: RunStyle {
                font_name: "Arial".into(),
                font_size: 10.0,
                color: "#000".into(),
                is_underline: false,
                ..Default::default()
            },
            bbox: BoundingBox::default(),
            origin_x: 0.0,
            origin_y: 0.0,
            ..Default::default()
        }
    }

    #[test]
    fn test_cjk_no_start_rule() {
        let mut runs = Vec::new();
        // Width 10.0 per run. Total width 40.0.
        runs.push(mock_run("Hello", 25.0));
        runs.push(mock_run("世界", 10.0));
        runs.push(mock_run("。", 5.0)); // Total 40.0.

        let paragraph = LayoutParagraph {
            runs,
            wrap_width: 38.0, // Should break before "世界" or "。"?
            ..Default::default()
        };

        // If we break before "。", it would be at the start of the next line.
        // Rule should force it to stay with "世界" or force "世界" to next line.
        let layout = layout_paragraph(&paragraph, 38.0, |_, _| 10.0); // Simple fixed width mock

        // Check if "。" is at the start of a line
        for line in layout.lines {
            if let Some(first) = line.runs.first() {
                assert!(
                    !is_no_start(first.text.chars().next().unwrap()),
                    "Punctuation at start of line: {}",
                    first.text
                );
            }
        }
    }

    #[test]
    fn test_justified_alignment() {
        let mut runs = Vec::new();
        // 5 runs, 10.0 width each.
        // Line 1: 3 runs (30.0). Wrap 40.0. Gap 10.0. 2 gaps -> 5.0 each.
        // Line 2: 2 runs.
        runs.push(mock_run("A", 10.0));
        runs.push(mock_run("B", 10.0));
        runs.push(mock_run("C", 10.0));
        runs.push(mock_run("D", 10.0));
        runs.push(mock_run("E", 10.0));

        let paragraph = LayoutParagraph {
            runs,
            style: ParagraphStyle {
                align: LayoutAlignment::Justify,
                ..Default::default()
            },
            ..Default::default()
        };

        // Wrap width 35.0.
        // Line 1: A, B, C (30.0). Remaining 5.0. 2 gaps -> 2.5 each.
        let layout = layout_paragraph(&paragraph, 35.0, |_, _| 10.0);

        let line = &layout.lines[0];
        assert_eq!(line.runs.len(), 3);
        assert_eq!(line.runs[0].origin_x, 0.0);
        assert_eq!(line.runs[1].origin_x, 12.5); // 10.0 original + 2.5 gap
        assert_eq!(line.runs[2].origin_x, 25.0); // 20.0 original + 5.0 gap
    }
}

impl ParagraphLayout {
    pub fn find_run_at_text_offset(&self, offset: usize) -> (usize, usize, usize) {
        let mut current_offset = 0;
        if self.lines.is_empty() {
            return (0, 0, 0);
        }
        for (l_idx, line) in self.lines.iter().enumerate() {
            for (r_idx, run) in line.runs.iter().enumerate() {
                let run_char_count = run.text.chars().count();
                if offset <= current_offset + run_char_count {
                    return (l_idx, r_idx, offset.saturating_sub(current_offset));
                }
                current_offset += run_char_count;
            }
        }
        let last_l = self.lines.len() - 1;
        let last_r = self.lines[last_l].runs.len().saturating_sub(1);
        let last_run_text_len = self.lines[last_l]
            .runs
            .get(last_r)
            .map(|r| r.text.chars().count())
            .unwrap_or(0);
        (last_l, last_r, last_run_text_len)
    }
}

pub fn find_paragraph_at(plan: &GlyphPaintPlan, x: f32, y: f32) -> Option<String> {
    let mut best_run_hit: Option<(f32, String)> = None;
    let mut best_paragraph_hit: Option<(f32, String)> = None;
    for region in plan.regions.iter().rev() {
        if !is_point_in_bbox(&region.bbox, x, y) {
            continue;
        }
        for paragraph in region.paragraphs.iter().rev() {
            let mut paragraph_has_run_hit = false;
            for run in paragraph.runs.iter().rev() {
                if is_point_in_bbox(&run.bbox, x, y) {
                    paragraph_has_run_hit = true;
                    let area = bbox_area(&run.bbox);
                    match &best_run_hit {
                        Some((best_area, _)) if *best_area <= area => {}
                        _ => best_run_hit = Some((area, paragraph.id.clone())),
                    }
                }
            }
            if paragraph_has_run_hit {
                continue;
            }
            if is_point_in_bbox(&paragraph.bbox, x, y) {
                let area = bbox_area(&paragraph.bbox);
                match &best_paragraph_hit {
                    Some((best_area, _)) if *best_area <= area => {}
                    _ => best_paragraph_hit = Some((area, paragraph.id.clone())),
                }
            }
        }
    }
    best_run_hit
        .map(|(_, id)| id)
        .or_else(|| best_paragraph_hit.map(|(_, id)| id))
}

pub fn is_point_in_bbox(bbox: &BoundingBox, x: f32, y: f32) -> bool {
    x >= bbox.left && x <= bbox.right && y >= bbox.top && y <= bbox.bottom
}

fn bbox_area(bbox: &BoundingBox) -> f32 {
    let width = (bbox.right - bbox.left).max(0.0);
    let height = (bbox.bottom - bbox.top).max(0.0);
    width * height
}

pub fn resolve_editor_projection(
    box_rect: &crate::models::RectBox,
    zoom: f32,
    font_size: f32,
    _page_height: f32,
) -> crate::models::FieldEditorProjection {
    // 物理投影逻辑：将 PDF 逻辑单位直接物理映射到 CSS 像素
    // 此处我们不再进行复杂的 Y 轴镜像，因为 RectBox 已是相对于容器顶部的像素预测
    crate::models::FieldEditorProjection {
        pixel_rect: crate::models::RectBox {
            left: box_rect.left * zoom,
            top: box_rect.top * zoom,
            width: box_rect.width * zoom,
            height: box_rect.height * zoom,
        },
        scale_x: 1.0, // 初始比例
        font_size: font_size * zoom,
        render_family: "sans-serif".into(),
        color: "#000000".into(),
    }
}
