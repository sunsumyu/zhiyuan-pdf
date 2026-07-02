use crate::text::glyph_layout::is_decorative_glyph;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ListMarkerKind {
    #[default]
    None,
    Bullet,
    Numbering,
    Symbol,
    Custom,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListTextSemantic {
    pub has_marker: bool,
    pub kind: ListMarkerKind,
    pub marker_text: String,
    pub body_text: String,
    pub marker_char_len: usize,
    pub body_char_start: usize,
}

fn extract_numbering_prefix(chars: &[char]) -> Option<(usize, usize)> {
    if chars.is_empty() {
        return None;
    }

    let mut index = 0usize;
    let has_open_paren = matches!(chars[index], '(' | '（');
    if has_open_paren {
        index += 1;
    }
    let token_start = index;
    while index < chars.len()
        && (chars[index].is_ascii_digit()
            || chars[index].is_ascii_alphabetic()
            || matches!(chars[index], '一'..='十' | 'Ⅰ'..='Ⅻ' | 'ⅰ'..='ⅻ'))
    {
        index += 1;
    }
    if index == token_start {
        return None;
    }

    if index >= chars.len() {
        return None;
    }

    let has_marker_delimiter = matches!(chars[index], '.' | ')' | '）' | '、' | '-' | ':' | '：');
    if !has_marker_delimiter {
        return None;
    }
    if has_open_paren && !matches!(chars[index], ')' | '）') {
        return None;
    }
    index += 1;

    let mut body_start = index;
    while body_start < chars.len() && chars[body_start].is_whitespace() {
        body_start += 1;
    }
    Some((index, body_start))
}

pub fn parse_numbering_value(marker_text: &str) -> Option<usize> {
    let chars: Vec<char> = marker_text.chars().collect();
    if chars.is_empty() {
        return None;
    }

    let mut index = 0usize;
    if matches!(chars[index], '(' | '（') {
        index += 1;
    }
    let token_start = index;
    while index < chars.len() && chars[index].is_ascii_digit() {
        index += 1;
    }
    if index == token_start {
        return None;
    }
    chars[token_start..index]
        .iter()
        .collect::<String>()
        .parse::<usize>()
        .ok()
}

pub fn format_numbering_marker(value: usize, template: Option<&str>) -> String {
    let template = template.unwrap_or("").trim();
    if template.is_empty() {
        return format!("{value}.");
    }

    let has_open_ascii_paren = template.starts_with('(');
    let has_open_fullwidth_paren = template.starts_with('（');
    let has_close_ascii_paren = template.contains(')');
    let has_close_fullwidth_paren = template.contains('）');
    let delimiter = if template.contains('、') {
        "、"
    } else if template.contains('）') {
        "）"
    } else if template.contains(')') {
        ")"
    } else if template.contains(':') {
        ":"
    } else if template.contains('：') {
        "："
    } else if template.contains('-') {
        "-"
    } else {
        "."
    };

    if has_open_ascii_paren && has_close_ascii_paren {
        format!("({value})")
    } else if has_open_fullwidth_paren && has_close_fullwidth_paren {
        format!("（{value}）")
    } else if has_open_ascii_paren {
        format!("({value}{delimiter}")
    } else if has_open_fullwidth_paren {
        format!("（{value}{delimiter}")
    } else {
        format!("{value}{delimiter}")
    }
}

pub fn derive_list_text_semantics(text: &str) -> ListTextSemantic {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return ListTextSemantic {
            has_marker: false,
            kind: ListMarkerKind::None,
            marker_text: String::new(),
            body_text: String::new(),
            marker_char_len: 0,
            body_char_start: 0,
        };
    }

    // 提取 body_text 的辅助：从 chars[lo..hi] 中剥离末尾的装饰字符及其前后的空白
    fn strip_trailing_decorative(chars: &[char], lo: usize, hi: usize) -> usize {
        let mut end = hi;
        loop {
            let changed_before = end;
            // 先剥末尾空白
            while end > lo && chars[end - 1].is_whitespace() {
                end -= 1;
            }
            // 再剥末尾装饰字符
            while end > lo && is_decorative_glyph(chars[end - 1]) {
                end -= 1;
            }
            if end == changed_before {
                break;
            }
        }
        end
    }

    // 1. 检测行首的装饰字符作为 marker
    let mut marker_end = 0usize;
    while marker_end < chars.len() && is_decorative_glyph(chars[marker_end]) {
        marker_end += 1;
    }

    if marker_end > 0 {
        let mut body_start = marker_end;
        while body_start < chars.len() && chars[body_start].is_whitespace() {
            body_start += 1;
        }
        let body_end = strip_trailing_decorative(&chars, body_start, chars.len());
        return ListTextSemantic {
            has_marker: true,
            kind: ListMarkerKind::Bullet,
            marker_text: chars[..marker_end].iter().collect(),
            body_text: chars[body_start..body_end].iter().collect(),
            marker_char_len: marker_end,
            body_char_start: body_start,
        };
    }

    // 2. 检测编号前缀（在剥离 trailing decorative 之前，避免吃掉 "1." 的 "."）
    if let Some((num_marker_end, body_start)) = extract_numbering_prefix(&chars) {
        let body_end = strip_trailing_decorative(&chars, body_start, chars.len());
        return ListTextSemantic {
            has_marker: true,
            kind: ListMarkerKind::Numbering,
            marker_text: chars[..num_marker_end].iter().collect(),
            body_text: chars[body_start..body_end].iter().collect(),
            marker_char_len: num_marker_end,
            body_char_start: body_start,
        };
    }

    // 3. 没有 marker，仅剥离末尾装饰字符及其前后空白
    let body_end = strip_trailing_decorative(&chars, 0, chars.len());
    if body_end < chars.len() {
        ListTextSemantic {
            has_marker: false,
            kind: ListMarkerKind::None,
            marker_text: String::new(),
            body_text: chars[..body_end].iter().collect(),
            marker_char_len: 0,
            body_char_start: 0,
        }
    } else {
        ListTextSemantic {
            has_marker: false,
            kind: ListMarkerKind::None,
            marker_text: String::new(),
            body_text: text.to_string(),
            marker_char_len: 0,
            body_char_start: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_trailing_decorative_glyph_from_body() {
        // 核心回归测试：末尾的装饰字符（如 ●）不应出现在 body_text 中
        let text = "• 正文内容●";
        let semantics = derive_list_text_semantics(text);

        assert!(semantics.has_marker);
        assert_eq!(semantics.marker_text, "•");
        assert_eq!(semantics.body_text, "正文内容");
        assert!(!semantics.body_text.contains("●"));
    }

    #[test]
    fn strips_trailing_decorative_without_leading_marker() {
        // 没有 marker 时，末尾装饰字符仍需剥离
        let text = "正文结尾●";
        let semantics = derive_list_text_semantics(text);

        assert!(!semantics.has_marker);
        assert_eq!(semantics.body_text, "正文结尾");
        assert!(!semantics.body_text.contains("●"));
    }

    #[test]
    fn preserves_body_without_trailing_decorative() {
        // 没有 trailing decorative 时，行为不变
        let text = "• 正文内容";
        let semantics = derive_list_text_semantics(text);

        assert!(semantics.has_marker);
        assert_eq!(semantics.marker_text, "•");
        assert_eq!(semantics.body_text, "正文内容");
    }

    #[test]
    fn handles_both_leading_and_trailing_decorative() {
        // 行首和行末都有装饰字符
        let text = "•● 正文 ●•";
        let semantics = derive_list_text_semantics(text);

        assert!(semantics.has_marker);
        assert_eq!(semantics.marker_text, "•●");
        assert_eq!(semantics.body_text, "正文");
    }

    #[test]
    fn strips_trailing_whitespace_before_decorative() {
        // 末尾空白 + 装饰字符都应剥离
        let text = "• 正文  ● ";
        let semantics = derive_list_text_semantics(text);

        assert!(semantics.has_marker);
        assert_eq!(semantics.body_text, "正文");
    }

    #[test]
    fn numbering_with_trailing_decorative() {
        // 编号列表 + 末尾装饰字符
        let text = "1. 编号内容●";
        let semantics = derive_list_text_semantics(text);

        assert!(semantics.has_marker);
        assert_eq!(semantics.kind, ListMarkerKind::Numbering);
        assert_eq!(semantics.marker_text, "1.");
        assert_eq!(semantics.body_text, "编号内容");
    }

    #[test]
    fn multiple_trailing_decorative_glyphs() {
        // 多个末尾装饰字符
        let text = "• 正文●●●";
        let semantics = derive_list_text_semantics(text);

        assert!(semantics.has_marker);
        assert_eq!(semantics.body_text, "正文");
    }

    #[test]
    fn empty_after_stripping_all_decorative() {
        // 全是装饰字符
        let text = "●●●";
        let semantics = derive_list_text_semantics(text);

        assert!(semantics.has_marker);
        assert_eq!(semantics.marker_text, "●●●");
        assert_eq!(semantics.body_text, "");
    }
}
