use crate::models::{LayoutParagraph, LayoutRun, RunStyle};
use crate::typography::font_resolver::looks_like_symbolic_font;
use serde::{Deserialize, Serialize};

/// 样式分片，表示一段具有相同样式的文本
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleSpan {
    pub text: String,
    pub style: RunStyle,
    pub is_decorative: bool,
}

/// 样式映射器：负责维护编辑态下的"文本 -> 样式"对应关系
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleMapper {
    pub spans: Vec<StyleSpan>,
}

impl StyleMapper {
    /// 从原始 PDF 的 Runs 初始化
    fn new_from_paragraph(paragraph: &LayoutParagraph) -> Self {
        let preserve_underline = should_preserve_editor_underline(paragraph);
        let mut spans = Vec::new();
        for run in &paragraph.runs {
            let mut style = run.style.clone();
            if !preserve_underline {
                style.is_underline = false;
            }
            spans.push(StyleSpan {
                text: run.text.clone(),
                style,
                is_decorative: is_decorative_text(&run.text),
            });
        }
        Self { spans }
    }

    pub fn new_from_paragraph_for_text(paragraph: &LayoutParagraph, source_text: &str) -> Self {
        let mut mapper = Self::new_from_paragraph(paragraph);
        if mapper.read_full_text() != source_text {
            mapper.update_with_text(source_text);
        }
        mapper
    }

    /// 获取当前全文本
    pub fn read_full_text(&self) -> String {
        self.spans.iter().map(|s| s.text.as_str()).collect()
    }

    /// 当用户输入新文本时，更新样式映射
    pub fn update_with_text(&mut self, new_text: &str) {
        let old_text = self.read_full_text();
        if old_text == new_text {
            return;
        }

        if new_text.is_empty() {
            // 防御性处理：防止全文删除导致白屏，保留最后一个有效样式
            let style = self
                .spans
                .first()
                .map(|s| s.style.clone())
                .unwrap_or_default();
            self.spans = vec![StyleSpan {
                text: "".to_string(),
                style,
                is_decorative: false,
            }];
            return;
        }

        let lcp_len = compute_lcp_len(&old_text, new_text);
        let remaining_old = &old_text[lcp_len..];
        let remaining_new = &new_text[lcp_len..];
        let lcs_len = compute_lcs_len(remaining_old, remaining_new);

        let mid_text_new = &new_text[lcp_len..(new_text.len() - lcs_len)];
        let old_suffix_start = old_text.len() - lcs_len;

        let mut new_spans = Vec::new();
        let mut cursor = 0;
        let mut active_style = self
            .spans
            .first()
            .map(|s| s.style.clone())
            .unwrap_or_default();

        // 1. 映射前缀
        for span in &self.spans {
            let span_len = span.text.len();
            if cursor + span_len <= lcp_len {
                new_spans.push(span.clone());
                active_style = span.style.clone();
            } else if cursor < lcp_len {
                let take = lcp_len - cursor;
                new_spans.push(StyleSpan {
                    text: span.text[..take].to_string(),
                    style: span.style.clone(),
                    is_decorative: span.is_decorative,
                });
                active_style = span.style.clone();
                break;
            }
            cursor += span_len;
        }

        // 2. 注入新内容（变动区）
        if !mid_text_new.is_empty() {
            new_spans.push(StyleSpan {
                text: mid_text_new.to_string(),
                style: active_style.clone(),
                is_decorative: false,
            });
        }

        // 3. 映射后缀（位移映射）
        let mut cursor = 0;
        for span in &self.spans {
            let span_len = span.text.len();
            let span_start = cursor;
            let span_end = cursor + span_len;

            if span_end > old_suffix_start {
                let overlap_start = span_start.max(old_suffix_start);
                let rel_start = overlap_start - span_start;
                // 必须检查字节边界以防中文字符切断
                if rel_start < span.text.len() {
                    let tail = &span.text[rel_start..];
                    new_spans.push(StyleSpan {
                        text: tail.to_string(),
                        style: span.style.clone(),
                        is_decorative: span.is_decorative,
                    });
                }
            }
            cursor += span_len;
        }

        self.spans = merge_adjacent_spans(new_spans);
    }

    pub fn set_bold_all(&mut self, bold: bool) {
        for span in &mut self.spans {
            span.style.is_bold = bold;
        }
        self.spans = merge_adjacent_spans(self.spans.clone());
    }

    pub fn set_italic_all(&mut self, italic: bool) {
        for span in &mut self.spans {
            span.style.is_italic = italic;
        }
        self.spans = merge_adjacent_spans(self.spans.clone());
    }

    pub fn set_underline_all(&mut self, underline: bool) {
        for span in &mut self.spans {
            span.style.is_underline = underline;
        }
        self.spans = merge_adjacent_spans(self.spans.clone());
    }

    pub fn set_color_all(&mut self, color: &str) {
        for span in &mut self.spans {
            span.style.color = color.to_string();
        }
        self.spans = merge_adjacent_spans(self.spans.clone());
    }

    pub fn set_font_name_all(&mut self, font_name: &str) {
        for span in &mut self.spans {
            span.style.font_name = font_name.to_string();
        }
        self.spans = merge_adjacent_spans(self.spans.clone());
    }

    pub fn set_font_size_all(&mut self, font_size: f32) {
        for span in &mut self.spans {
            span.style.font_size = font_size;
        }
        self.spans = merge_adjacent_spans(self.spans.clone());
    }

    pub fn set_char_spacing_all(&mut self, char_spacing: f32) {
        for span in &mut self.spans {
            span.style.char_spacing = char_spacing;
        }
        self.spans = merge_adjacent_spans(self.spans.clone());
    }

    pub fn is_bold_any(&self) -> bool {
        self.spans.iter().any(|s| s.style.is_bold)
    }

    pub fn is_italic_any(&self) -> bool {
        self.spans.iter().any(|s| s.style.is_italic)
    }

    pub fn is_bold_all(&self) -> bool {
        !self.spans.is_empty() && self.spans.iter().all(|s| s.style.is_bold)
    }

    pub fn is_italic_all(&self) -> bool {
        !self.spans.is_empty() && self.spans.iter().all(|s| s.style.is_italic)
    }

    pub fn is_underline_any(&self) -> bool {
        self.spans.iter().any(|s| s.style.is_underline)
    }

    pub fn is_underline_all(&self) -> bool {
        !self.spans.is_empty() && self.spans.iter().all(|s| s.style.is_underline)
    }

    pub fn dominant_style(&self) -> RunStyle {
        self.spans
            .iter()
            .find(|span| !span.text.trim().is_empty() && !span.is_decorative)
            .or_else(|| self.spans.first())
            .map(|span| span.style.clone())
            .unwrap_or_default()
    }

    pub fn has_style_changes_against_paragraph(&self, paragraph: &LayoutParagraph) -> bool {
        let current_text = self.read_full_text();
        let source_mapper = Self::new_from_paragraph_for_text(paragraph, &current_text);
        !style_spans_have_same_paint_style(&self.spans, &source_mapper.spans)
    }

    /// 转换为排版引擎可识别的 Runs
    pub fn to_layout_runs(&self) -> Vec<LayoutRun> {
        self.spans
            .iter()
            .enumerate()
            .map(|(i, span)| {
                LayoutRun {
                    id: format!("run-{}", i),
                    text: span.text.clone(),
                    style: span.style.clone(),
                    // 坐标由排版引擎动态生成，此处设为 0
                    bbox: Default::default(),
                    origin_x: 0.0,
                    origin_y: 0.0,
                    char_origins: Vec::new(),
                    char_widths: Vec::new(),
                    object_ids: Vec::new(),
                    object_indices: Vec::new(),
                }
            })
            .collect()
    }
}

pub fn should_preserve_editor_underline(paragraph: &LayoutParagraph) -> bool {
    let mut visible_char_count = 0usize;
    let mut underline_char_count = 0usize;

    for run in &paragraph.runs {
        if run.text.trim().is_empty()
            || is_decorative_text(&run.text)
            || looks_like_symbolic_font(&run.style.font_name)
        {
            continue;
        }

        let char_count = run.text.chars().count();
        visible_char_count += char_count;
        if run.style.is_underline {
            underline_char_count += char_count;
        }
    }

    visible_char_count > 0 && (underline_char_count as f32 / visible_char_count as f32) >= 0.8
}

fn compute_lcp_len(a: &str, b: &str) -> usize {
    a.chars()
        .zip(b.chars())
        .take_while(|(ca, cb)| ca == cb)
        .map(|(c, _)| c.len_utf8())
        .sum()
}

fn compute_lcs_len(a: &str, b: &str) -> usize {
    let ar: Vec<char> = a.chars().rev().collect();
    let br: Vec<char> = b.chars().rev().collect();
    ar.iter()
        .zip(br.iter())
        .take_while(|(ca, cb)| ca == cb)
        .map(|(c, _)| c.len_utf8())
        .sum()
}

fn merge_adjacent_spans(spans: Vec<StyleSpan>) -> Vec<StyleSpan> {
    if spans.is_empty() {
        return spans;
    }
    let mut result = Vec::new();
    let mut current = spans[0].clone();

    for span in spans.into_iter().skip(1) {
        if is_style_equal(&current.style, &span.style)
            && current.is_decorative == span.is_decorative
        {
            current.text.push_str(&span.text);
        } else {
            result.push(current);
            current = span;
        }
    }
    result.push(current);
    result
}

fn style_spans_have_same_paint_style(left: &[StyleSpan], right: &[StyleSpan]) -> bool {
    let left_chars = expand_style_signature_by_char(left);
    let right_chars = expand_style_signature_by_char(right);
    if !left_chars.is_empty() || !right_chars.is_empty() {
        return left_chars == right_chars;
    }

    let left_style = left.first().map(|span| (&span.style, span.is_decorative));
    let right_style = right.first().map(|span| (&span.style, span.is_decorative));
    left_style == right_style
}

fn expand_style_signature_by_char(spans: &[StyleSpan]) -> Vec<(RunStyle, bool)> {
    let mut signature = Vec::new();
    for span in spans {
        for _ in span.text.chars() {
            signature.push((span.style.clone(), span.is_decorative));
        }
    }
    signature
}

fn is_style_equal(s1: &RunStyle, s2: &RunStyle) -> bool {
    s1.is_bold == s2.is_bold
        && s1.is_italic == s2.is_italic
        && s1.is_underline == s2.is_underline
        && s1.font_size == s2.font_size
        && s1.color == s2.color
}

fn is_decorative_text(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return false;
    }
    trimmed
        .chars()
        .all(|c| ['•', '●', '▪', '◦', '·', '○', '-', '▶', '➤'].contains(&c))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{BoundingBox, LayoutParagraph, LayoutRun, RunStyle};

    fn create_test_mapper(segments: &[(&str, bool)]) -> StyleMapper {
        let mut spans = Vec::new();
        for (text, bold) in segments {
            let mut style = RunStyle::default();
            style.is_bold = *bold;
            style.font_size = 12.0;
            spans.push(StyleSpan {
                text: text.to_string(),
                style,
                is_decorative: false,
            });
        }
        StyleMapper { spans }
    }

    fn create_test_run(text: &str) -> LayoutRun {
        LayoutRun {
            text: text.to_string(),
            style: RunStyle {
                font_name: "MicrosoftYaHei".to_string(),
                font_size: 12.0,
                color: "#000000".to_string(),
                is_bold: false,
                is_italic: false,
                is_underline: false,
                char_spacing: 0.0,
                scale_x: 1.0,
            },
            bbox: BoundingBox::default(),
            ..Default::default()
        }
    }

    #[test]
    fn test_deletion_at_head() {
        let mut mapper = create_test_mapper(&[("ABC", true), ("DEF", false)]);
        mapper.update_with_text("BCDEF");
        assert_eq!(mapper.spans[0].text, "BC");
        assert_eq!(mapper.spans[0].style.is_bold, true);
        assert_eq!(mapper.spans[1].text, "DEF");
    }

    #[test]
    fn test_deletion_multi_byte_chinese() {
        let mut mapper = create_test_mapper(&[("专业：", true), ("计算机", false)]);
        mapper.update_with_text("专：计算机");
        assert_eq!(mapper.spans[0].text, "专：");
        assert_eq!(mapper.spans[0].style.is_bold, true);
        assert_eq!(mapper.spans[1].text, "计算机");
    }

    #[test]
    fn test_full_deletion_protection() {
        let mut mapper = create_test_mapper(&[("Hello", true)]);
        mapper.update_with_text("");
        assert_eq!(mapper.spans.len(), 1);
        assert_eq!(mapper.spans[0].text, "");
        assert_eq!(mapper.spans[0].style.is_bold, true);
    }

    #[test]
    fn canonical_gap_reconstruction_is_not_a_style_change() {
        let paragraph = LayoutParagraph {
            runs: vec![create_test_run("编程语言:"), create_test_run("Rust")],
            ..Default::default()
        };
        let mapper = StyleMapper::new_from_paragraph_for_text(&paragraph, "编程语言: Rust");

        assert_eq!(mapper.read_full_text(), "编程语言: Rust");
        assert!(!mapper.has_style_changes_against_paragraph(&paragraph));
    }
}
