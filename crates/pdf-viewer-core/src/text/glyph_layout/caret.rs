//! run advance 推测、光标定位、点击命中、装饰前缀提取。

use crate::models::{
    FieldHitBatchRequest, FieldHitMatch, FieldHitRequest, FieldHitResolution, FieldPartKind,
    LayoutRun, ParagraphEditContext,
};

use super::text_predicates::is_decorative_text;
use super::DecorativePrefixLayout;

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
