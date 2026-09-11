//! 物理字形空间微观排版引擎 (Micro-Typography & Glyph Layout Engine)
//!
//! # Overview (核心职责)
//! 当 `analyzer` 在宏观层面确立了段落 (Paragraph) 与行 (Line) 的结构后，
//! `glyph_layout` 负责深入到最底层的单字符 (Char) 或单字模 (Glyph) 级别。
//! 它主要解决两大挑战：
//! 1. **缝隙还原计算 (Gap Heuristics)**: PDF 格式本质是不存储空格字符的（如 ' '），往往依靠前置字宽加上巨大的坐标位移跳跃来表现。
//!    本模块通过探知坐标系中跨字符的 "悬崖式跳变"，结合 CJK、标点等特征，安全地塞入虚拟控制空格字符。
//! 2. **跨域光标映射 (Hit-Testing & Caret Resolution)**: 在富文本编辑器（React/DOM）的字符位置索引，
//!    和物理渲染画布绝对 X 坐标之间进行相互映射。
//!    
//! # Constraints (边界条件)
//! 本域的所有逻辑都假定输入的数据已经完全遵守了 `Y-Down` 的底线防线，这里不再干涉任何垂直 Y 维度的极性问题。

use crate::models::{
    FieldHitBatchRequest, FieldHitMatch, FieldHitRequest, FieldHitResolution, FieldPartKind,
    LayoutRun, ParagraphEditContext,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct DecorativePrefixLayout {
    pub text: String,
    pub char_len: usize,
    pub width: f32,
    pub runs: Vec<LayoutRun>,
}

/// 记录原始离散 PDF 文本域与平顺化 (Flattened) DOM 字符串之间的索引映射地图。
///
/// # Architectural Invariant
/// 发向浏览器前端 `contenteditable` 的字符串往往经历了系统强行的间隙补充（例如虚拟空格）。
/// 这会导致 DOM 选区得到的 `length` 远大于真实 PDF 游程的字元数量。
/// 此表负责记录 `raw`（原汁原味的 PDF 字符序）与 `reconstructed`（混入虚拟控制字符的复合序列）
/// 之间的双向绑定 O(1) 查表。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EditorSessionTextPlan {
    /// 经过微观排版启发式补丁处理后的最终可编辑连续字符串。
    pub text: String,
    /// 逐字元落点解析的横截面框追踪集合。
    pub slots: Vec<EditorGlyphSlot>,
    raw_to_reconstructed: Vec<usize>,
    reconstructed_to_raw: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum EditorGlyphSlotKind {
    /// 原生，其对应 PDF 内容流底层有着明确真实指令发射特征的字模数据实体。
    #[default]
    Glyph,
    /// 由间隙启发引擎强制插空造出的幽灵控制字符（如虚拟空格）。
    Gap,
}

/// 承载单字符宽度计算与物理边界描绘的最小视觉结构槽。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EditorGlyphSlot {
    pub kind: EditorGlyphSlotKind,
    pub ch: char,
    /// 如果属于原生物理段落，承载其在原始向量流字典里的顺位倒排索引。
    pub raw_char_index: Option<usize>,
    pub left: f32,
    pub right: f32,
}

impl EditorSessionTextPlan {
    pub fn map_raw_to_reconstructed(&self, raw_index: usize) -> usize {
        self.raw_to_reconstructed
            .get(raw_index)
            .copied()
            .or_else(|| self.raw_to_reconstructed.last().copied())
            .unwrap_or(0)
    }

    pub fn reconstructed_char_count(&self) -> usize {
        self.text.chars().count()
    }

    pub fn map_reconstructed_to_raw(&self, reconstructed_index: usize) -> usize {
        self.reconstructed_to_raw
            .get(reconstructed_index)
            .copied()
            .or_else(|| self.reconstructed_to_raw.last().copied())
            .unwrap_or(0)
    }
}

pub fn is_decorative_glyph(ch: char) -> bool {
    matches!(ch, '•' | '●' | '▪' | '◦' | '·' | '○' | '-' | '▶' | '➤')
}

pub fn is_decorative_text(text: &str) -> bool {
    let trimmed = text.trim();
    !trimmed.is_empty() && trimmed.chars().all(is_decorative_glyph)
}

fn is_cjk_unified(ch: char) -> bool {
    matches!(
        ch as u32,
        0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xF900..=0xFAFF
            | 0x3040..=0x30FF
            | 0x31F0..=0x31FF
            | 0xAC00..=0xD7AF
    )
}

fn is_open_punctuation(ch: char) -> bool {
    matches!(ch, '(' | '[' | '{' | '（' | '【' | '《' | '「' | '『')
}

fn is_close_punctuation(ch: char) -> bool {
    matches!(
        ch,
        ')' | ']'
            | '}'
            | '）'
            | '】'
            | '》'
            | '」'
            | '』'
            | ','
            | '，'
            | '.'
            | '。'
            | ';'
            | '；'
            | ':'
            | '：'
            | '!'
            | '！'
            | '?'
            | '？'
    )
}

fn should_allow_synthetic_gap(prev_last: char, next_first: char) -> bool {
    if is_open_punctuation(prev_last)
        || is_open_punctuation(next_first)
        || is_close_punctuation(next_first)
    {
        return false;
    }

    if is_cjk_unified(prev_last) && is_cjk_unified(next_first) {
        return false;
    }

    true
}

fn is_spacing_punctuation(ch: char) -> bool {
    matches!(ch, ':' | '：' | ',' | '，' | ';' | '；')
}

fn is_ascii_word_start(ch: char) -> bool {
    ch.is_ascii_alphanumeric()
}

fn estimated_gap_source_advance(prev: char, next: char, typical_advance: f32) -> f32 {
    let advance = typical_advance.max(1.0);
    if is_spacing_punctuation(prev) && is_ascii_word_start(next) {
        return advance * 0.45;
    }
    advance * 0.82
}

fn should_insert_gap_from_origin_delta(
    prev: char,
    next: char,
    origin_delta: f32,
    typical_advance: f32,
) -> bool {
    if !origin_delta.is_finite() || origin_delta <= 0.0 {
        return false;
    }
    let expected_advance = estimated_gap_source_advance(prev, next, typical_advance);
    let estimated_gap = origin_delta - expected_advance;
    let threshold = if is_spacing_punctuation(prev) && is_ascii_word_start(next) {
        (typical_advance * 0.08).max(0.4)
    } else {
        (typical_advance * 0.32).max(1.0)
    };
    estimated_gap > threshold
}

/// 尝试回退推测某一段游程在其本身没有完整 `char_origins` 精确映射表背景下，每个字符平均占有的 X 轴间幅。
///
/// # Algorithmic Complexity
/// 此计算由于依赖字符串的 `chars().count()` 和可能的包围盒求距，其被嵌套入更高阶的 O(N) 循环中。
/// 总体开销为 O(N)，属于可接受的轻量级后备测算。
pub fn infer_run_advance(run: &LayoutRun) -> f32 {
    if run.char_origins.len() >= 2 {
        let deltas: Vec<f32> = run
            .char_origins
            .windows(2)
            .map(|pair| pair[1] - pair[0])
            .filter(|delta| delta.is_finite() && *delta > 0.0)
            .collect();
        if let Some(delta) = deltas.first() {
            return *delta;
        }
    }
    let glyph_count = run.text.chars().count().max(1) as f32;
    let run_width = (run.bbox.right - run.bbox.left).max(1.0);
    run_width / glyph_count
}

/// 提供在特定文本块落定某个 `caret_index` 逻辑光标位标时，预测其相对于包围盒左锚点的绝对 X 轴物理漂移。
///
/// # Overview (架构机制)
/// 传统的 HTML `<input>` 会自动管理光标渲染，但在基于 Canvas 和自绘引擎的混合编辑态架构中，
/// 必须手动完成从 `String Index` 到 `Absolute Pixel` 的转化。
///
/// 该方法内部使用前缀和 (Prefix Sum) 技巧逐游程消耗掉字符数，当命中目标游程内部时，
/// 提取对应的原生物理 `char_origins` 做到 `O(N)` 像素级高精定位。
pub fn compute_run_aware_caret_left(session: &ParagraphEditContext, caret_index: usize) -> f32 {
    let mut consumed = 0usize;
    for run in &session.paragraph.runs {
        let glyph_count = run.text.chars().count();
        let local_x = run.origin_x - session.anchor_bbox.left;
        if caret_index <= consumed + glyph_count {
            let in_run = caret_index.saturating_sub(consumed);
            if in_run == 0 {
                return local_x;
            }
            let fallback_advance = infer_run_advance(run);
            if in_run >= glyph_count {
                if let Some(last_origin) = run.char_origins.last() {
                    return local_x + *last_origin + fallback_advance;
                }
                return local_x + ((glyph_count as f32) * fallback_advance);
            }
            if let Some(origin) = run.char_origins.get(in_run) {
                return local_x + *origin;
            }
            return local_x + ((in_run as f32) * fallback_advance);
        }
        consumed += glyph_count;
    }
    session
        .paragraph
        .runs
        .last()
        .map(|run| {
            let local_x = run.origin_x - session.anchor_bbox.left;
            let glyph_count = run.text.chars().count();
            let fallback_advance = infer_run_advance(run);
            if let Some(last_origin) = run.char_origins.last() {
                local_x + *last_origin + fallback_advance
            } else {
                local_x + ((glyph_count as f32) * fallback_advance)
            }
        })
        .unwrap_or(0.0)
}

/// 反向投射命中测试 (Hit-Testing Layout Reversal): 根据物理点击坐标，推导它究竟穿透了那个逻辑字符缝隙。
///
/// # 算法策略
/// 使用 `O(N)` 穷举所有游程及其包含的字元槽，维护最近距离 (Nearest Neighbor Euclidean Distance)。
/// 由于 PDF 单段通常字符不超过 100~200，此处不引入基于二分查找或者 Quad-Tree 的过早优化。
pub fn resolve_caret_index_for_click(
    session: &ParagraphEditContext,
    click_x_from_anchor_left: f32,
) -> usize {
    let mut best_index = 0usize;
    let mut best_distance = f32::INFINITY;
    let mut consumed = 0usize;

    let mut update_best = |x: f32, index: usize| {
        let distance = (click_x_from_anchor_left - x).abs();
        if distance < best_distance {
            best_distance = distance;
            best_index = index;
        }
    };

    for run in &session.paragraph.runs {
        let glyph_count = run.text.chars().count();
        if glyph_count == 0 {
            continue;
        }
        let local_run_x = run.origin_x - session.anchor_bbox.left;
        let inferred_advance = infer_run_advance(run);

        update_best(local_run_x, consumed);
        for glyph_index in 1..=glyph_count {
            let glyph_x = if glyph_index >= glyph_count {
                local_run_x
                    + run
                        .char_origins
                        .last()
                        .copied()
                        .unwrap_or(((glyph_count - 1) as f32) * inferred_advance)
                    + inferred_advance
            } else {
                local_run_x
                    + run
                        .char_origins
                        .get(glyph_index)
                        .copied()
                        .unwrap_or((glyph_index as f32) * inferred_advance)
            };
            update_best(glyph_x, consumed + glyph_index);
        }
        consumed += glyph_count;
    }

    best_index
}

pub fn resolve_field_hit_for_click(request: &FieldHitRequest) -> FieldHitResolution {
    let hit_in_label = request.click_page_x < request.projection.value_box.left;
    let active_part = if hit_in_label {
        FieldPartKind::Key
    } else {
        FieldPartKind::Value
    };

    let active_box = if hit_in_label {
        request.projection.label_box
    } else {
        request.projection.value_box
    };
    let active_text = if hit_in_label {
        request.editable_key_text.as_str()
    } else {
        request.editable_value_text.as_str()
    };
    let active_session = if hit_in_label {
        request.key_session.as_ref()
    } else {
        request.value_session.as_ref()
    };

    let click_x_from_anchor_left =
        (request.click_page_x - active_box.left).clamp(0.0, active_box.width.max(0.0));

    let initial_caret_index = active_session
        .map(|session| resolve_caret_index_for_click(session, click_x_from_anchor_left))
        .unwrap_or_else(|| active_text.chars().count());

    let measured_key_width = request
        .key_session
        .as_ref()
        .map(|session| (session.anchor_bbox.right - session.anchor_bbox.left).max(24.0))
        .unwrap_or_else(|| request.projection.label_box.width.max(24.0));

    let measured_value_width = request
        .value_session
        .as_ref()
        .map(|session| (session.anchor_bbox.right - session.anchor_bbox.left).max(24.0))
        .unwrap_or_else(|| request.projection.value_box.width.max(24.0));

    FieldHitResolution {
        active_part,
        initial_caret_index,
        measured_key_width,
        measured_value_width,
    }
}

fn rect_contains_point(
    left: f32,
    top: f32,
    width: f32,
    height: f32,
    x: f32,
    y: f32,
    tolerance: f32,
) -> bool {
    x >= left - tolerance
        && x <= left + width + tolerance
        && y >= top - tolerance
        && y <= top + height + tolerance
}

pub fn resolve_field_hit_target_for_click(request: &FieldHitBatchRequest) -> Option<FieldHitMatch> {
    const HIT_TOLERANCE: f32 = 5.0;

    request
        .targets
        .iter()
        .enumerate()
        .find_map(|(target_index, target)| {
            if !rect_contains_point(
                target.projection.text_box.left,
                target.projection.text_box.top,
                target.projection.text_box.width,
                target.projection.text_box.height,
                request.click_page_x,
                request.click_page_y,
                HIT_TOLERANCE,
            ) {
                return None;
            }

            let resolution = resolve_field_hit_for_click(&FieldHitRequest {
                projection: target.projection.clone(),
                editable_key_text: target.editable_key_text.clone(),
                editable_value_text: target.editable_value_text.clone(),
                click_page_x: request.click_page_x,
                key_session: target.key_session.clone(),
                value_session: target.value_session.clone(),
            });

            Some(FieldHitMatch {
                target_index,
                resolution,
            })
        })
}

pub fn extract_decorative_prefix<F>(
    session: &ParagraphEditContext,
    looks_like_symbol_font: F,
) -> Option<DecorativePrefixLayout>
where
    F: Fn(&str) -> bool,
{
    let decorative_run_count = session
        .paragraph
        .runs
        .iter()
        .take_while(|run| {
            is_decorative_text(&run.text) || looks_like_symbol_font(&run.style.font_name)
        })
        .count();
    if decorative_run_count == 0 {
        return None;
    }
    let runs = session.paragraph.runs[..decorative_run_count].to_vec();
    let text = runs.iter().map(|run| run.text.as_str()).collect::<String>();
    let char_len = text.chars().count();
    let width = session
        .paragraph
        .runs
        .get(decorative_run_count)
        .map(|run| (run.origin_x - session.anchor_bbox.left).max(0.0))
        .unwrap_or_else(|| compute_run_aware_caret_left(session, char_len));
    Some(DecorativePrefixLayout {
        text,
        char_len,
        width,
        runs,
    })
}

fn glyph_left(run: &LayoutRun, glyph_index: usize) -> f32 {
    run.origin_x
        + run
            .char_origins
            .get(glyph_index)
            .copied()
            .unwrap_or_else(|| infer_run_advance(run) * glyph_index as f32)
}

fn glyph_right(run: &LayoutRun, glyph_index: usize, glyph_count: usize) -> f32 {
    if glyph_index + 1 < run.char_origins.len() {
        return run.origin_x + run.char_origins[glyph_index + 1];
    }
    if let Some(width) = run
        .char_widths
        .get(glyph_index)
        .copied()
        .filter(|value| value.is_finite() && *value > 0.0)
    {
        return glyph_left(run, glyph_index) + width;
    }
    if glyph_count == 1 && run.bbox.right > run.bbox.left {
        return run.bbox.right;
    }
    glyph_left(run, glyph_index) + typical_contiguous_advance(run)
}

fn glyph_visual_width(run: &LayoutRun, glyph_index: usize, glyph_count: usize) -> f32 {
    (glyph_right(run, glyph_index, glyph_count) - glyph_left(run, glyph_index)).max(1.0)
}

fn same_visual_line(prev: &LayoutRun, next: &LayoutRun) -> bool {
    let tolerance = (prev.style.font_size.max(next.style.font_size) * 0.45).max(2.0);
    (prev.origin_y - next.origin_y).abs() <= tolerance
}

fn typical_contiguous_advance(run: &LayoutRun) -> f32 {
    let mut deltas: Vec<f32> = run
        .char_origins
        .windows(2)
        .map(|pair| pair[1] - pair[0])
        .filter(|delta| delta.is_finite() && *delta > 0.0)
        .collect();
    if deltas.is_empty() {
        return infer_run_advance(run).max(1.0);
    }

    deltas.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    // Use the lower-middle advance so large PDF-positioned word gaps do not become the baseline.
    let index = ((deltas.len().saturating_sub(1)) as f32 * 0.35).round() as usize;
    deltas[index.min(deltas.len() - 1)].max(1.0)
}

fn should_insert_internal_gap_space(run: &LayoutRun, glyph_index: usize, chars: &[char]) -> bool {
    if glyph_index + 1 >= chars.len() || glyph_index + 1 >= run.char_origins.len() {
        return false;
    }

    let current = chars[glyph_index];
    let next = chars[glyph_index + 1];
    if current.is_whitespace() || next.is_whitespace() {
        return false;
    }
    if !should_allow_synthetic_gap(current, next) {
        return false;
    }

    let next_left = glyph_left(run, glyph_index + 1);
    let current_left = glyph_left(run, glyph_index);
    let origin_delta = next_left - current_left;
    if !origin_delta.is_finite() || origin_delta <= 0.0 {
        return false;
    }

    let typical_advance = typical_contiguous_advance(run);
    should_insert_gap_from_origin_delta(current, next, origin_delta, typical_advance)
}

fn should_insert_visual_gap_space(prev: &LayoutRun, next: &LayoutRun) -> bool {
    if !same_visual_line(prev, next) {
        return false;
    }

    let prev_text = prev.text.trim_end();
    let next_text = next.text.trim_start();
    if prev_text.is_empty() || next_text.is_empty() {
        return false;
    }
    let prev_last = prev_text.chars().last().unwrap_or(' ');
    let next_first = next_text.chars().next().unwrap_or(' ');
    if prev_last.is_whitespace() || next_first.is_whitespace() {
        return false;
    }
    if !should_allow_synthetic_gap(prev_last, next_first) {
        return false;
    }

    let prev_glyph_count = prev.text.chars().count().max(1);
    let next_glyph_count = next.text.chars().count().max(1);
    let prev_right = glyph_right(prev, prev_glyph_count.saturating_sub(1), prev_glyph_count);
    let next_left = glyph_left(next, 0);
    let geometric_gap = next_left - prev_right;
    if !geometric_gap.is_finite() || geometric_gap <= 0.0 {
        return false;
    }
    let prev_width = glyph_visual_width(prev, prev_glyph_count.saturating_sub(1), prev_glyph_count);
    let next_width = glyph_visual_width(next, 0, next_glyph_count);
    let reference_width = prev_width.min(next_width).max(1.0);
    let contiguous_join_gap = (prev.style.font_size * 0.08)
        .max(reference_width * 0.18)
        .max(0.9);
    geometric_gap > contiguous_join_gap
}

fn line_contextual_run_delta(runs: &[&LayoutRun], run_index: usize) -> Option<f32> {
    let target = runs.get(run_index)?;
    let mut deltas = Vec::new();

    for pair in runs.windows(2) {
        let [left, right] = pair else { continue };
        if !same_visual_line(left, right) {
            continue;
        }
        let delta = right.origin_x - left.origin_x;
        if delta.is_finite() && delta > 0.0 {
            deltas.push(delta);
        }
    }

    if deltas.is_empty() {
        let prev_delta = run_index
            .checked_sub(1)
            .and_then(|idx| runs.get(idx))
            .filter(|prev| same_visual_line(prev, target))
            .map(|prev| target.origin_x - prev.origin_x);
        let next_delta = runs
            .get(run_index + 1)
            .filter(|next| same_visual_line(target, next))
            .map(|next| next.origin_x - target.origin_x);
        return prev_delta
            .or(next_delta)
            .filter(|delta| delta.is_finite() && *delta > 0.0);
    }

    deltas.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let index = ((deltas.len().saturating_sub(1)) as f32 * 0.35).round() as usize;
    Some(deltas[index.min(deltas.len() - 1)].max(1.0))
}

fn needs_gap(prev: &LayoutRun, next: &LayoutRun, line_typical_delta: Option<f32>) -> bool {
    if !same_visual_line(prev, next) {
        return false;
    }

    let prev_text = prev.text.trim_end();
    let next_text = next.text.trim_start();
    if prev_text.is_empty() || next_text.is_empty() {
        return false;
    }
    let prev_last = prev_text.chars().last().unwrap_or(' ');
    let next_first = next_text.chars().next().unwrap_or(' ');
    if prev_last.is_whitespace() || next_first.is_whitespace() {
        return false;
    }
    if !should_allow_synthetic_gap(prev_last, next_first) {
        return false;
    }

    let prev_char_count = prev.text.chars().count();
    let next_char_count = next.text.chars().count();
    if prev_char_count == 1 && next_char_count == 1 {
        let origin_delta = next.origin_x - prev.origin_x;
        if !origin_delta.is_finite() || origin_delta <= 0.0 {
            return false;
        }
        let contextual_reference = line_typical_delta
            .map(|delta| delta.max(1.0))
            .unwrap_or_else(|| prev.style.font_size.max(next.style.font_size).max(1.0));
        return should_insert_gap_from_origin_delta(
            prev_last,
            next_first,
            origin_delta,
            contextual_reference,
        );
    }

    should_insert_visual_gap_space(prev, next)
}

pub fn build_editor_session_text_plan(session: &ParagraphEditContext) -> EditorSessionTextPlan {
    let ordered_runs: Vec<&LayoutRun> = session
        .paragraph
        .runs
        .iter()
        .filter(|run| !run.text.is_empty())
        .collect();
    let mut text = String::new();
    let mut slots = Vec::new();
    let raw_char_capacity = ordered_runs
        .iter()
        .map(|run| run.text.chars().count())
        .sum::<usize>();
    let mut raw_to_reconstructed = Vec::with_capacity(raw_char_capacity + 1);
    raw_to_reconstructed.push(0);
    let mut reconstructed_to_raw = Vec::new();
    reconstructed_to_raw.push(0);
    let mut raw_count = 0usize;
    let mut prev: Option<&LayoutRun> = None;
    let mut prev_right: Option<f32> = None;

    for (run_index, run) in ordered_runs.iter().enumerate() {
        if let Some(prev_run) = prev {
            let line_typical_delta = line_contextual_run_delta(&ordered_runs, run_index);
            if needs_gap(prev_run, run, line_typical_delta) {
                let gap_left = prev_right.unwrap_or(prev_run.bbox.right);
                let gap_right = run.bbox.left.max(gap_left);
                slots.push(EditorGlyphSlot {
                    kind: EditorGlyphSlotKind::Gap,
                    ch: ' ',
                    raw_char_index: None,
                    left: gap_left - session.anchor_bbox.left,
                    right: gap_right - session.anchor_bbox.left,
                });
                text.push(' ');
                reconstructed_to_raw.push(raw_count);
                if let Some(last) = raw_to_reconstructed.last_mut() {
                    *last = text.chars().count();
                }
            }
        }
        let chars: Vec<char> = run.text.chars().collect();
        if chars.is_empty() {
            prev = Some(run);
            continue;
        }

        if run.char_origins.len() < 2 {
            let glyph_count = chars.len();
            for (glyph_index, ch) in chars.into_iter().enumerate() {
                let left = if glyph_count == 1 {
                    run.bbox.left
                } else {
                    glyph_left(run, glyph_index)
                };
                let right = if glyph_count == 1 {
                    run.bbox.right
                } else {
                    glyph_right(run, glyph_index, glyph_count)
                };
                slots.push(EditorGlyphSlot {
                    kind: EditorGlyphSlotKind::Glyph,
                    ch,
                    raw_char_index: Some(raw_count),
                    left: left - session.anchor_bbox.left,
                    right: right - session.anchor_bbox.left,
                });
                text.push(ch);
                raw_count += 1;
                raw_to_reconstructed.push(text.chars().count());
                reconstructed_to_raw.push(raw_count);
                prev_right = Some(right);
            }
            prev = Some(run);
            continue;
        }

        for (index, ch) in chars.iter().enumerate() {
            let glyph_count = chars.len();
            let left = glyph_left(run, index);
            let right = glyph_right(run, index, glyph_count);
            slots.push(EditorGlyphSlot {
                kind: EditorGlyphSlotKind::Glyph,
                ch: *ch,
                raw_char_index: Some(raw_count),
                left: left - session.anchor_bbox.left,
                right: right - session.anchor_bbox.left,
            });
            text.push(*ch);
            raw_count += 1;
            raw_to_reconstructed.push(text.chars().count());
            reconstructed_to_raw.push(raw_count);
            if should_insert_internal_gap_space(run, index, &chars) {
                let next_left = glyph_left(run, index + 1);
                slots.push(EditorGlyphSlot {
                    kind: EditorGlyphSlotKind::Gap,
                    ch: ' ',
                    raw_char_index: None,
                    left: right - session.anchor_bbox.left,
                    right: next_left - session.anchor_bbox.left,
                });
                text.push(' ');
                reconstructed_to_raw.push(raw_count);
                if let Some(last) = raw_to_reconstructed.last_mut() {
                    *last = text.chars().count();
                }
            }
            prev_right = Some(right);
        }
        prev = Some(run);
    }

    EditorSessionTextPlan {
        text,
        slots,
        raw_to_reconstructed,
        reconstructed_to_raw,
    }
}

pub fn has_suspicious_run_geometry<F, G>(
    session: &ParagraphEditContext,
    is_symbol_font: F,
    measure_run_width: G,
) -> bool
where
    F: Fn(&str) -> bool,
    G: Fn(&LayoutRun) -> f32,
{
    session.paragraph.runs.iter().any(|run| {
        if is_decorative_text(&run.text) || is_symbol_font(&run.style.font_name) {
            return false;
        }
        let bbox_width = (run.bbox.right - run.bbox.left).abs().max(1.0);
        let measured_width = measure_run_width(run).max(1.0);
        bbox_width > measured_width * 3.0 || bbox_width < measured_width * 0.4
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── Predicate Functions ─────────────────────────────────────────────

    #[test]
    fn is_decorative_glyph_known_chars() {
        let decorative = ['•', '●', '▪', '◦', '·', '○', '-', '▶'];
        for ch in &decorative {
            assert!(
                is_decorative_glyph(*ch),
                "expected '{}' to be decorative",
                ch
            );
        }
    }

    #[test]
    fn is_decorative_glyph_non_decorative() {
        let non_decorative = ['a', '张', '1', '(', '。'];
        for ch in &non_decorative {
            assert!(
                !is_decorative_glyph(*ch),
                "expected {} to be non-decorative",
                ch
            );
        }
    }

    #[test]
    fn is_decorative_text_single_glyph() {
        assert!(is_decorative_text("•"));
        assert!(is_decorative_text("●"));
    }

    #[test]
    fn is_decorative_text_multiple_glyphs() {
        assert!(is_decorative_text("•••"));
        assert!(is_decorative_text("●●●"));
    }

    #[test]
    fn is_decorative_text_with_whitespace() {
        assert!(is_decorative_text(" • "));
        assert!(is_decorative_text("\t●\n"));
    }

    #[test]
    fn is_decorative_text_empty() {
        assert!(!is_decorative_text(""));
        assert!(!is_decorative_text("   "));
    }

    #[test]
    fn is_decorative_text_mixed() {
        assert!(!is_decorative_text("•a"));
        assert!(!is_decorative_text("a•"));
        assert!(!is_decorative_text("• hello"));
    }

    #[test]
    fn is_cjk_unified_chinese() {
        assert!(is_cjk_unified('张'));
        assert!(is_cjk_unified('三'));
        assert!(is_cjk_unified('中'));
    }

    #[test]
    fn is_cjk_unified_japanese() {
        assert!(is_cjk_unified('あ'));
        assert!(is_cjk_unified('カ'));
        assert!(is_cjk_unified('ン'));
    }

    #[test]
    fn is_cjk_unified_korean() {
        assert!(is_cjk_unified('한'));
        assert!(is_cjk_unified('국'));
    }

    #[test]
    fn is_cjk_unified_non_cjk() {
        assert!(!is_cjk_unified('a'));
        assert!(!is_cjk_unified('1'));
        assert!(!is_cjk_unified('('));
    }

    #[test]
    fn is_open_punctuation_ascii() {
        assert!(is_open_punctuation('('));
        assert!(is_open_punctuation('['));
        assert!(is_open_punctuation('{'));
    }

    #[test]
    fn is_open_punctuation_cjk() {
        assert!(is_open_punctuation('（'));
        assert!(is_open_punctuation('【'));
        assert!(is_open_punctuation('《'));
        assert!(is_open_punctuation('「'));
        assert!(is_open_punctuation('『'));
    }

    #[test]
    fn is_open_punctuation_non_punct() {
        assert!(!is_open_punctuation('a'));
        assert!(!is_open_punctuation(')'));
        assert!(!is_open_punctuation('。'));
    }

    #[test]
    fn is_close_punctuation_ascii() {
        assert!(is_close_punctuation(')'));
        assert!(is_close_punctuation(']'));
        assert!(is_close_punctuation('}'));
        assert!(is_close_punctuation(','));
        assert!(is_close_punctuation('.'));
        assert!(is_close_punctuation(';'));
        assert!(is_close_punctuation(':'));
        assert!(is_close_punctuation('!'));
        assert!(is_close_punctuation('?'));
    }

    #[test]
    fn is_close_punctuation_cjk() {
        assert!(is_close_punctuation('）'));
        assert!(is_close_punctuation('】'));
        assert!(is_close_punctuation('》'));
        assert!(is_close_punctuation('」'));
        assert!(is_close_punctuation('』'));
        assert!(is_close_punctuation('，'));
        assert!(is_close_punctuation('。'));
        assert!(is_close_punctuation('；'));
        assert!(is_close_punctuation('：'));
        assert!(is_close_punctuation('！'));
        assert!(is_close_punctuation('？'));
    }

    #[test]
    fn should_allow_synthetic_gap_cjk_pair() {
        // CJK pairs should NOT have synthetic gap
        assert!(!should_allow_synthetic_gap('张', '三'));
        assert!(!should_allow_synthetic_gap('中', '国'));
    }

    #[test]
    fn should_allow_synthetic_gap_open_punct() {
        // Open punctuation should NOT have synthetic gap
        assert!(!should_allow_synthetic_gap('a', '('));
        assert!(!should_allow_synthetic_gap('(', 'b'));
        assert!(!should_allow_synthetic_gap('a', '（'));
    }

    #[test]
    fn should_allow_synthetic_gap_close_punct() {
        // Close punctuation at next position should NOT have synthetic gap
        assert!(!should_allow_synthetic_gap('a', ')'));
        assert!(!should_allow_synthetic_gap('a', '。'));
    }

    #[test]
    fn should_allow_synthetic_gap_normal_pair() {
        // Normal ASCII pairs SHOULD have synthetic gap
        assert!(should_allow_synthetic_gap('a', 'b'));
    }

    // ─── EditorSessionTextPlan Bidirectional Mapping ────────────────────

    #[test]
    fn text_plan_empty() {
        let plan = EditorSessionTextPlan::default();
        assert_eq!(plan.map_raw_to_reconstructed(0), 0);
        assert_eq!(plan.map_reconstructed_to_raw(0), 0);
        assert_eq!(plan.reconstructed_char_count(), 0);
    }

    #[test]
    fn text_plan_roundtrip_consistency() {
        // Test that mapping is consistent in both directions
        let plan = EditorSessionTextPlan {
            text: "abc".to_string(),
            slots: vec![],
            raw_to_reconstructed: vec![0, 1, 2, 3],
            reconstructed_to_raw: vec![0, 1, 2, 3],
        };

        // Round-trip: raw -> reconstructed -> raw
        for raw in 0..=3 {
            let reconstructed = plan.map_raw_to_reconstructed(raw);
            let back_to_raw = plan.map_reconstructed_to_raw(reconstructed);
            assert_eq!(back_to_raw, raw, "round-trip failed for raw={}", raw);
        }

        // Round-trip: reconstructed -> raw -> reconstructed
        for recon in 0..=3 {
            let raw = plan.map_reconstructed_to_raw(recon);
            let back_to_recon = plan.map_raw_to_reconstructed(raw);
            assert_eq!(
                back_to_recon, recon,
                "round-trip failed for reconstructed={}",
                recon
            );
        }
    }

    #[test]
    fn text_plan_gap_insertion_mapping() {
        // Simulate: raw "ab" with a gap inserted between -> reconstructed "a b"
        // raw indices: [0, 1] -> reconstructed indices: [0, 2]
        let plan = EditorSessionTextPlan {
            text: "a b".to_string(),
            slots: vec![],
            raw_to_reconstructed: vec![0, 2], // raw[0]->recon[0], raw[1]->recon[2]
            reconstructed_to_raw: vec![0, 0, 1], // recon[0]->raw[0], recon[1]->raw[0], recon[2]->raw[1]
        };

        assert_eq!(plan.map_raw_to_reconstructed(0), 0);
        assert_eq!(plan.map_raw_to_reconstructed(1), 2);

        assert_eq!(plan.map_reconstructed_to_raw(0), 0);
        assert_eq!(plan.map_reconstructed_to_raw(1), 0); // gap char maps to previous raw
        assert_eq!(plan.map_reconstructed_to_raw(2), 1);

        assert_eq!(plan.reconstructed_char_count(), 3);
    }

    #[test]
    fn text_plan_boundary_overflow() {
        let plan = EditorSessionTextPlan {
            text: "abc".to_string(),
            slots: vec![],
            raw_to_reconstructed: vec![0, 1, 2],
            reconstructed_to_raw: vec![0, 1, 2],
        };

        // Out-of-bounds should return last element
        assert_eq!(plan.map_raw_to_reconstructed(100), 2);
        assert_eq!(plan.map_reconstructed_to_raw(100), 2);
    }

    // ─── Infer Run Advance ───────────────────────────────────────────────

    #[test]
    fn infer_run_advance_from_bbox() {
        let run = LayoutRun {
            id: "test-1".to_string(),
            text: "abc".to_string(),
            style: crate::models::RunStyle {
                font_name: "Arial".to_string(),
                font_size: 12.0,
                color: "#000000".to_string(),
                is_bold: false,
                is_italic: false,
                ..Default::default()
            },
            bbox: crate::models::BoundingBox {
                left: 10.0,
                top: 0.0,
                right: 40.0,
                bottom: 12.0,
            },
            origin_x: 10.0,
            origin_y: 0.0,
            char_origins: vec![],
            char_widths: vec![],
            object_ids: vec![],
            object_indices: vec![],
        };
        // Advance should be bbox width / char count = 30 / 3 = 10.0
        let advance = infer_run_advance(&run);
        assert!((advance - 10.0).abs() < 0.01, "advance={}", advance);
    }

    #[test]
    fn infer_run_advance_single_char() {
        let run = LayoutRun {
            id: "test-2".to_string(),
            text: "a".to_string(),
            style: crate::models::RunStyle {
                font_name: "Arial".to_string(),
                font_size: 12.0,
                color: "#000000".to_string(),
                is_bold: false,
                is_italic: false,
                ..Default::default()
            },
            bbox: crate::models::BoundingBox {
                left: 0.0,
                top: 0.0,
                right: 8.0,
                bottom: 12.0,
            },
            origin_x: 0.0,
            origin_y: 0.0,
            char_origins: vec![],
            char_widths: vec![],
            object_ids: vec![],
            object_indices: vec![],
        };
        // Single char: advance = bbox width / 1 = 8.0
        let advance = infer_run_advance(&run);
        assert!((advance - 8.0).abs() < 0.01, "advance={}", advance);
    }

    // ─── Gap Insertion Logic ─────────────────────────────────────────────

    #[test]
    fn needs_gap_large_coordinate_jump() {
        // Two runs with large coordinate jump should need gap
        let prev = LayoutRun {
            id: "prev".to_string(),
            text: "abc".to_string(),
            style: crate::models::RunStyle {
                font_name: "Arial".to_string(),
                font_size: 12.0,
                color: "#000000".to_string(),
                is_bold: false,
                is_italic: false,
                ..Default::default()
            },
            bbox: crate::models::BoundingBox {
                left: 0.0,
                top: 0.0,
                right: 30.0,
                bottom: 12.0,
            },
            origin_x: 0.0,
            origin_y: 0.0,
            char_origins: vec![],
            char_widths: vec![],
            object_ids: vec![],
            object_indices: vec![],
        };
        let next = LayoutRun {
            id: "next".to_string(),
            text: "xyz".to_string(),
            style: crate::models::RunStyle {
                font_name: "Arial".to_string(),
                font_size: 12.0,
                color: "#000000".to_string(),
                is_bold: false,
                is_italic: false,
                ..Default::default()
            },
            bbox: crate::models::BoundingBox {
                left: 100.0,
                top: 0.0,
                right: 130.0,
                bottom: 12.0,
            },
            origin_x: 100.0,
            origin_y: 0.0,
            char_origins: vec![],
            char_widths: vec![],
            object_ids: vec![],
            object_indices: vec![],
        };
        let typical_delta = 5.0;
        assert!(needs_gap(&prev, &next, Some(typical_delta)));
    }

    #[test]
    fn needs_gap_small_coordinate_jump() {
        // Two runs with small coordinate jump should NOT need gap
        // Use very small gap (1.0) which is less than contiguous_join_gap
        let prev = LayoutRun {
            id: "prev".to_string(),
            text: "abc".to_string(),
            style: crate::models::RunStyle {
                font_name: "Arial".to_string(),
                font_size: 12.0,
                color: "#000000".to_string(),
                is_bold: false,
                is_italic: false,
                ..Default::default()
            },
            bbox: crate::models::BoundingBox {
                left: 0.0,
                top: 0.0,
                right: 30.0,
                bottom: 12.0,
            },
            origin_x: 0.0,
            origin_y: 0.0,
            // Relative offsets from origin_x
            char_origins: vec![0.0, 10.0, 20.0, 30.0],
            char_widths: vec![10.0, 10.0, 10.0],
            object_ids: vec![],
            object_indices: vec![],
        };
        let next = LayoutRun {
            id: "next".to_string(),
            text: "xyz".to_string(),
            style: crate::models::RunStyle {
                font_name: "Arial".to_string(),
                font_size: 12.0,
                color: "#000000".to_string(),
                is_bold: false,
                is_italic: false,
                ..Default::default()
            },
            bbox: crate::models::BoundingBox {
                left: 31.0,
                top: 0.0,
                right: 61.0,
                bottom: 12.0,
            },
            origin_x: 31.0,
            origin_y: 0.0,
            // Relative offsets from origin_x (31.0)
            char_origins: vec![0.0, 10.0, 20.0, 30.0],
            char_widths: vec![10.0, 10.0, 10.0],
            object_ids: vec![],
            object_indices: vec![],
        };
        let typical_delta = 10.0;
        // geometric_gap = next_left - prev_right = (31+0) - (0+30) = 1.0
        // contiguous_join_gap ≈ max(0.96, 1.8, 0.9) = 1.8
        // 1.0 < 1.8, so no gap
        assert!(!needs_gap(&prev, &next, Some(typical_delta)));
    }

    // ─── Visual Line Detection ───────────────────────────────────────────

    #[test]
    fn same_visual_line_same_y() {
        let prev = LayoutRun {
            id: "prev".to_string(),
            text: "abc".to_string(),
            style: crate::models::RunStyle::default(),
            bbox: crate::models::BoundingBox {
                left: 0.0,
                top: 10.0,
                right: 30.0,
                bottom: 22.0,
            },
            origin_x: 0.0,
            origin_y: 10.0,
            char_origins: vec![],
            char_widths: vec![],
            object_ids: vec![],
            object_indices: vec![],
        };
        let next = LayoutRun {
            id: "next".to_string(),
            text: "xyz".to_string(),
            style: crate::models::RunStyle::default(),
            bbox: crate::models::BoundingBox {
                left: 40.0,
                top: 10.0,
                right: 70.0,
                bottom: 22.0,
            },
            origin_x: 40.0,
            origin_y: 10.0,
            char_origins: vec![],
            char_widths: vec![],
            object_ids: vec![],
            object_indices: vec![],
        };
        assert!(same_visual_line(&prev, &next));
    }

    #[test]
    fn same_visual_line_different_y() {
        let prev = LayoutRun {
            id: "prev".to_string(),
            text: "abc".to_string(),
            style: crate::models::RunStyle::default(),
            bbox: crate::models::BoundingBox {
                left: 0.0,
                top: 10.0,
                right: 30.0,
                bottom: 22.0,
            },
            origin_x: 0.0,
            origin_y: 10.0,
            char_origins: vec![],
            char_widths: vec![],
            object_ids: vec![],
            object_indices: vec![],
        };
        let next = LayoutRun {
            id: "next".to_string(),
            text: "xyz".to_string(),
            style: crate::models::RunStyle::default(),
            bbox: crate::models::BoundingBox {
                left: 0.0,
                top: 30.0,
                right: 30.0,
                bottom: 42.0,
            },
            origin_x: 0.0,
            origin_y: 30.0,
            char_origins: vec![],
            char_widths: vec![],
            object_ids: vec![],
            object_indices: vec![],
        };
        assert!(!same_visual_line(&prev, &next));
    }

    // ─── Has Suspicious Run Geometry ─────────────────────────────────────

    #[test]
    fn has_suspicious_run_geometry_normal() {
        let session = ParagraphEditContext {
            anchor_bbox: crate::models::BoundingBox::default(),
            paragraph: crate::models::LayoutParagraph {
                id: "test".to_string(),
                bbox: crate::models::BoundingBox::default(),
                style: crate::models::ParagraphStyle::default(),
                runs: vec![LayoutRun {
                    id: "run-1".to_string(),
                    text: "abc".to_string(),
                    style: crate::models::RunStyle::default(),
                    bbox: crate::models::BoundingBox {
                        left: 0.0,
                        top: 0.0,
                        right: 30.0,
                        bottom: 12.0,
                    },
                    origin_x: 0.0,
                    origin_y: 0.0,
                    char_origins: vec![],
                    char_widths: vec![],
                    object_ids: vec![],
                    object_indices: vec![],
                }],
                object_ids: vec![],
                origin_x: 0.0,
                origin_y: 0.0,
                wrap_width: 0.0,
            },
        };
        assert!(!has_suspicious_run_geometry(
            &session,
            |_| false,
            |run| run.bbox.right - run.bbox.left,
        ));
    }

    #[test]
    fn has_suspicious_run_geometry_oversized_bbox() {
        let session = ParagraphEditContext {
            anchor_bbox: crate::models::BoundingBox::default(),
            paragraph: crate::models::LayoutParagraph {
                id: "test".to_string(),
                bbox: crate::models::BoundingBox::default(),
                style: crate::models::ParagraphStyle::default(),
                runs: vec![LayoutRun {
                    id: "run-1".to_string(),
                    text: "abc".to_string(),
                    style: crate::models::RunStyle::default(),
                    bbox: crate::models::BoundingBox {
                        left: 0.0,
                        top: 0.0,
                        right: 100.0,
                        bottom: 12.0,
                    },
                    origin_x: 0.0,
                    origin_y: 0.0,
                    char_origins: vec![],
                    char_widths: vec![],
                    object_ids: vec![],
                    object_indices: vec![],
                }],
                object_ids: vec![],
                origin_x: 0.0,
                origin_y: 0.0,
                wrap_width: 0.0,
            },
        };
        assert!(has_suspicious_run_geometry(
            &session,
            |_| false,
            |_| 10.0, // measured width is 10, bbox is 100 -> suspicious
        ));
    }

    #[test]
    fn has_suspicious_run_geometry_decorative_excluded() {
        let session = ParagraphEditContext {
            anchor_bbox: crate::models::BoundingBox::default(),
            paragraph: crate::models::LayoutParagraph {
                id: "test".to_string(),
                bbox: crate::models::BoundingBox::default(),
                style: crate::models::ParagraphStyle::default(),
                runs: vec![LayoutRun {
                    id: "run-1".to_string(),
                    text: "•••".to_string(),
                    style: crate::models::RunStyle::default(),
                    bbox: crate::models::BoundingBox {
                        left: 0.0,
                        top: 0.0,
                        right: 100.0,
                        bottom: 12.0,
                    },
                    origin_x: 0.0,
                    origin_y: 0.0,
                    char_origins: vec![],
                    char_widths: vec![],
                    object_ids: vec![],
                    object_indices: vec![],
                }],
                object_ids: vec![],
                origin_x: 0.0,
                origin_y: 0.0,
                wrap_width: 0.0,
            },
        };
        assert!(!has_suspicious_run_geometry(&session, |_| false, |_| 10.0,));
    }

    #[test]
    fn has_suspicious_run_geometry_symbol_font_excluded() {
        let session = ParagraphEditContext {
            anchor_bbox: crate::models::BoundingBox::default(),
            paragraph: crate::models::LayoutParagraph {
                id: "test".to_string(),
                bbox: crate::models::BoundingBox::default(),
                style: crate::models::ParagraphStyle::default(),
                runs: vec![LayoutRun {
                    id: "run-1".to_string(),
                    text: "abc".to_string(),
                    style: crate::models::RunStyle {
                        font_name: "Symbol".to_string(),
                        ..Default::default()
                    },
                    bbox: crate::models::BoundingBox {
                        left: 0.0,
                        top: 0.0,
                        right: 100.0,
                        bottom: 12.0,
                    },
                    origin_x: 0.0,
                    origin_y: 0.0,
                    char_origins: vec![],
                    char_widths: vec![],
                    object_ids: vec![],
                    object_indices: vec![],
                }],
                object_ids: vec![],
                origin_x: 0.0,
                origin_y: 0.0,
                wrap_width: 0.0,
            },
        };
        assert!(!has_suspicious_run_geometry(
            &session,
            |name| name == "Symbol",
            |_| 10.0,
        ));
    }
}
