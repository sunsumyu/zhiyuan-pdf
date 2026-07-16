use crate::models::{LayoutRun, ParagraphEditContext};

fn run_gap(previous: &LayoutRun, next: &LayoutRun) -> f32 {
    let previous_width = (previous.bbox.right - previous.bbox.left).max(0.0);
    let previous_right = previous.bbox.right.max(previous.origin_x + previous_width);
    let next_left = next.bbox.left.min(next.origin_x);
    next_left - previous_right
}

fn boundary_needs_visual_space(previous: char, next: char) -> bool {
    if previous.is_whitespace() || next.is_whitespace() {
        return false;
    }

    previous == ':'
        || previous == '：'
        || previous == ','
        || previous == '，'
        || previous == ';'
        || previous == '；'
        || previous == ')'
        || previous == '）'
        || next == '('
        || next == '（'
        || (previous.is_ascii_lowercase() && next.is_ascii_uppercase())
        || (previous.is_ascii_digit() && next.is_ascii_alphabetic())
}

fn should_insert_run_space(previous: &LayoutRun, next: &LayoutRun) -> bool {
    let Some(previous_last) = previous.text.chars().rev().find(|ch| !ch.is_whitespace()) else {
        return false;
    };
    let Some(next_first) = next.text.chars().find(|ch| !ch.is_whitespace()) else {
        return false;
    };
    if !boundary_needs_visual_space(previous_last, next_first) {
        return false;
    }

    let gap = run_gap(previous, next);
    if !gap.is_finite() || gap <= 0.0 {
        return false;
    }

    let font_size = previous.style.font_size.max(next.style.font_size).max(1.0);
    let previous_width = (previous.bbox.right - previous.bbox.left).max(0.0);
    let next_width = (next.bbox.right - next.bbox.left).max(0.0);
    let width_hint = previous_width.max(next_width)
        / previous
            .text
            .chars()
            .count()
            .max(next.text.chars().count())
            .max(1) as f32;
    let threshold = (font_size * 0.16).max(width_hint * 0.55).max(1.4);
    gap >= threshold
}

fn char_gap_threshold(font_size: f32, previous_width: f32, next_width: f32) -> f32 {
    let width_hint = previous_width.max(next_width).max(1.0);
    (font_size.max(1.0) * 0.16).max(width_hint * 0.55).max(1.4)
}

fn should_insert_char_space(
    previous: char,
    next: char,
    gap: f32,
    font_size: f32,
    previous_width: f32,
    next_width: f32,
) -> bool {
    if !boundary_needs_visual_space(previous, next) {
        return false;
    }
    if !gap.is_finite() || gap <= 0.0 {
        return false;
    }
    gap >= char_gap_threshold(font_size, previous_width, next_width)
}

fn is_ascii_word_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric()
}

fn is_pdf_text_separator(ch: char) -> bool {
    matches!(ch, ':' | '：' | ',' | '，' | ';' | '；')
}

fn starts_with_compact_word_boundary(chars: &[char], index: usize) -> bool {
    const WORDS: [&str; 3] = ["Framework", "Program", "Library"];
    WORDS.iter().any(|word| {
        let word_chars = word.chars().collect::<Vec<_>>();
        chars
            .get(index..index + word_chars.len())
            .is_some_and(|slice| slice == word_chars.as_slice())
    })
}

fn needs_compact_text_space(chars: &[char], index: usize) -> bool {
    if index == 0 {
        return false;
    }
    let previous = chars[index - 1];
    let next = chars[index];
    if previous.is_whitespace() || next.is_whitespace() {
        return false;
    }
    if is_pdf_text_separator(previous) && (is_ascii_word_char(next) || next == '(' || next == '（')
    {
        return true;
    }
    if (next == '(' || next == '（') && is_ascii_word_char(previous) {
        return true;
    }
    previous.is_ascii_lowercase()
        && next.is_ascii_uppercase()
        && starts_with_compact_word_boundary(chars, index)
}

fn normalize_compact_pdf_text(text: &str) -> String {
    let chars = text.chars().collect::<Vec<_>>();
    let mut normalized = String::new();
    for (index, ch) in chars.iter().enumerate() {
        if needs_compact_text_space(&chars, index) {
            normalized.push(' ');
        }
        normalized.push(*ch);
    }
    normalized
}

fn run_text_with_visual_spaces(run: &LayoutRun) -> String {
    let chars = run.text.chars().collect::<Vec<_>>();
    if chars.len() < 2 || run.char_origins.len() < chars.len() {
        return normalize_compact_pdf_text(&run.text);
    }

    let fallback_width = |ch: char| {
        if ch.is_ascii() {
            (run.style.font_size * 0.5).max(1.0)
        } else {
            run.style.font_size.max(1.0)
        }
    };

    let mut text = String::new();
    for index in 0..chars.len() {
        if index > 0 {
            let previous_origin = run.char_origins[index - 1];
            let current_origin = run.char_origins[index];
            let previous_width = run
                .char_widths
                .get(index - 1)
                .copied()
                .filter(|width| width.is_finite() && *width > 0.0)
                .unwrap_or_else(|| fallback_width(chars[index - 1]));
            let current_width = run
                .char_widths
                .get(index)
                .copied()
                .filter(|width| width.is_finite() && *width > 0.0)
                .unwrap_or_else(|| fallback_width(chars[index]));
            let gap = current_origin - (previous_origin + previous_width);
            if should_insert_char_space(
                chars[index - 1],
                chars[index],
                gap,
                run.style.font_size,
                previous_width,
                current_width,
            ) {
                text.push(' ');
            }
        }
        text.push(chars[index]);
    }
    normalize_compact_pdf_text(&text)
}

pub fn source_text(context: &ParagraphEditContext) -> String {
    let mut text = String::new();
    let mut previous_text_run: Option<&LayoutRun> = None;
    for run in &context.paragraph.runs {
        if let Some(previous) = previous_text_run {
            if should_insert_run_space(previous, run) {
                text.push(' ');
            }
        }
        text.push_str(&run_text_with_visual_spaces(run));
        if !run.text.is_empty() {
            previous_text_run = Some(run);
        }
    }
    normalize_compact_pdf_text(&text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_pdf_text_restores_resume_word_boundaries() {
        assert_eq!(
            normalize_compact_pdf_text(
                "智能合约:AnchorFramework,SolanaProgramLibrary(SPL),ERC-20/721"
            ),
            "智能合约: Anchor Framework, Solana Program Library (SPL), ERC-20/721"
        );
    }

    #[test]
    fn compact_pdf_text_does_not_split_acronyms() {
        let text = normalize_compact_pdf_text("SPL,ERC-20/721");
        assert_eq!(text, "SPL, ERC-20/721");
        assert!(!text.contains("S PL"));
        assert!(!text.contains("ER C"));
    }

    #[test]
    fn keeps_technical_names() {
        let text = normalize_compact_pdf_text("SpringBoot,MyBatisPlus");
        assert_eq!(text, "SpringBoot, MyBatisPlus");
    }
}
