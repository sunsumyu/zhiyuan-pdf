use crate::glyph_layout::is_decorative_glyph;
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

    let mut marker_end = 0usize;
    while marker_end < chars.len() && is_decorative_glyph(chars[marker_end]) {
        marker_end += 1;
    }

    if marker_end > 0 {
        let mut body_start = marker_end;
        while body_start < chars.len() && chars[body_start].is_whitespace() {
            body_start += 1;
        }
        return ListTextSemantic {
            has_marker: true,
            kind: ListMarkerKind::Bullet,
            marker_text: chars[..marker_end].iter().collect(),
            body_text: chars[body_start..].iter().collect(),
            marker_char_len: marker_end,
            body_char_start: body_start,
        };
    }

    if let Some((marker_end, body_start)) = extract_numbering_prefix(&chars) {
        return ListTextSemantic {
            has_marker: true,
            kind: ListMarkerKind::Numbering,
            marker_text: chars[..marker_end].iter().collect(),
            body_text: chars[body_start..].iter().collect(),
            marker_char_len: marker_end,
            body_char_start: body_start,
        };
    }

    ListTextSemantic {
        has_marker: false,
        kind: ListMarkerKind::None,
        marker_text: String::new(),
        body_text: text.to_string(),
        marker_char_len: 0,
        body_char_start: 0,
    }
}
