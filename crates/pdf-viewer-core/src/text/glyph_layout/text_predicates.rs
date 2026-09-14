//! CJK/标点判定 + 合成间隙启发式谓词。

pub fn is_decorative_glyph(ch: char) -> bool {
    matches!(ch, '•' | '●' | '▪' | '◦' | '·' | '○' | '-' | '▶' | '➤')
}

pub fn is_decorative_text(text: &str) -> bool {
    let trimmed = text.trim();
    !trimmed.is_empty() && trimmed.chars().all(is_decorative_glyph)
}

pub(crate) fn is_cjk_unified(ch: char) -> bool {
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

pub(crate) fn is_open_punctuation(ch: char) -> bool {
    matches!(ch, '(' | '[' | '{' | '（' | '【' | '《' | '「' | '『')
}

pub(crate) fn is_close_punctuation(ch: char) -> bool {
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

pub(crate) fn should_allow_synthetic_gap(prev_last: char, next_first: char) -> bool {
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

pub(crate) fn should_insert_gap_from_origin_delta(
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
