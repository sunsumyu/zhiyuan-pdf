#[derive(Debug, Clone, Copy, Default)]
pub struct SearchReplaceOptions {
    pub case_sensitive: bool,
    pub replace_all_occurrences: bool,
}

pub fn replace_query_matches(
    text: &str,
    query: &str,
    replacement: &str,
    options: SearchReplaceOptions,
) -> Option<String> {
    let normalized_query = query.trim();
    if normalized_query.is_empty() {
        return None;
    }

    let text_chars = text.chars().collect::<Vec<_>>();
    let query_chars = normalized_query.chars().collect::<Vec<_>>();
    if query_chars.is_empty() || text_chars.len() < query_chars.len() {
        return None;
    }

    let mut ranges = Vec::new();
    let mut cursor = 0usize;
    while cursor + query_chars.len() <= text_chars.len() {
        if matches_query_at(&text_chars, cursor, &query_chars, options.case_sensitive) {
            ranges.push((cursor, cursor + query_chars.len()));
            if options.replace_all_occurrences {
                cursor += query_chars.len().max(1);
            } else {
                break;
            }
        } else {
            cursor += 1;
        }
    }

    if ranges.is_empty() {
        return None;
    }

    let mut result = String::new();
    let mut start = 0usize;
    for (left, right) in ranges {
        result.push_str(&slice_chars(text, start, left));
        result.push_str(replacement);
        start = right;
    }
    result.push_str(&slice_chars(text, start, text_chars.len()));

    if result == text {
        None
    } else {
        Some(result)
    }
}

fn matches_query_at(
    text_chars: &[char],
    start: usize,
    query_chars: &[char],
    case_sensitive: bool,
) -> bool {
    query_chars.iter().enumerate().all(|(offset, query_char)| {
        match text_chars.get(start + offset) {
            Some(text_char) if case_sensitive => *text_char == *query_char,
            Some(text_char) => {
                text_char.to_lowercase().collect::<String>()
                    == query_char.to_lowercase().collect::<String>()
            }
            None => false,
        }
    })
}

fn slice_chars(text: &str, start: usize, end: usize) -> String {
    text.chars()
        .skip(start)
        .take(end.saturating_sub(start))
        .collect()
}
