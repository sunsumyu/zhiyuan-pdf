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

#[cfg(test)]
mod tests {
    use super::*;

    // ── replace_query_matches ───────────────────────────────────────────

    #[test]
    fn replace_single_occurrence() {
        let result = replace_query_matches(
            "hello world",
            "world",
            "rust",
            SearchReplaceOptions {
                case_sensitive: true,
                replace_all_occurrences: false,
            },
        );
        assert_eq!(result, Some("hello rust".to_string()));
    }

    #[test]
    fn replace_all_occurrences() {
        let result = replace_query_matches(
            "foo bar foo baz foo",
            "foo",
            "qux",
            SearchReplaceOptions {
                case_sensitive: true,
                replace_all_occurrences: true,
            },
        );
        assert_eq!(result, Some("qux bar qux baz qux".to_string()));
    }

    #[test]
    fn replace_case_insensitive() {
        let result = replace_query_matches(
            "Hello HELLO hello",
            "hello",
            "hi",
            SearchReplaceOptions {
                case_sensitive: false,
                replace_all_occurrences: true,
            },
        );
        assert_eq!(result, Some("hi hi hi".to_string()));
    }

    #[test]
    fn replace_no_match() {
        let result = replace_query_matches(
            "hello world",
            "xyz",
            "abc",
            SearchReplaceOptions {
                case_sensitive: true,
                replace_all_occurrences: false,
            },
        );
        assert!(result.is_none());
    }

    #[test]
    fn replace_empty_query() {
        let result = replace_query_matches(
            "hello",
            "",
            "x",
            SearchReplaceOptions {
                case_sensitive: true,
                replace_all_occurrences: false,
            },
        );
        assert!(result.is_none());
    }

    #[test]
    fn replace_whitespace_only_query() {
        let result = replace_query_matches(
            "hello",
            "   ",
            "x",
            SearchReplaceOptions {
                case_sensitive: true,
                replace_all_occurrences: false,
            },
        );
        assert!(result.is_none());
    }

    #[test]
    fn replace_unicode() {
        let result = replace_query_matches(
            "你好世界",
            "世界",
            "Rust",
            SearchReplaceOptions {
                case_sensitive: true,
                replace_all_occurrences: false,
            },
        );
        // Function replaces "世界" with "Rust" without adding space
        assert_eq!(result, Some("你好Rust".to_string()));
    }

    // ─── slice_chars ─────────────────────────────────────────────────────

    #[test]
    fn slice_chars_basic() {
        assert_eq!(slice_chars("hello", 1, 4), "ell");
    }

    #[test]
    fn slice_chars_unicode() {
        assert_eq!(slice_chars("你好世界", 1, 3), "好世");
    }

    #[test]
    fn slice_chars_empty() {
        assert_eq!(slice_chars("hello", 2, 2), "");
    }

    #[test]
    fn slice_chars_out_of_bounds() {
        assert_eq!(slice_chars("hello", 0, 100), "hello");
    }
}
