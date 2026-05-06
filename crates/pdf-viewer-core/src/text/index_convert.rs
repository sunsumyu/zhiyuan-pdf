pub fn utf16_offset_to_char_index(text: &str, utf16_offset: usize) -> usize {
    let mut consumed_units = 0usize;
    for (char_index, ch) in text.chars().enumerate() {
        let char_units = ch.len_utf16();
        if consumed_units.saturating_add(char_units) > utf16_offset {
            return char_index;
        }
        consumed_units = consumed_units.saturating_add(char_units);
    }
    text.chars().count()
}

pub fn char_index_to_utf16_offset(text: &str, char_index: usize) -> usize {
    text.chars().take(char_index).map(|ch| ch.len_utf16()).sum()
}

#[cfg(test)]
mod tests {
    use super::{char_index_to_utf16_offset, utf16_offset_to_char_index};

    #[test]
    fn converts_between_dom_utf16_offsets_and_rust_char_indexes() {
        let text = "A💡中";

        assert_eq!(char_index_to_utf16_offset(text, 0), 0);
        assert_eq!(char_index_to_utf16_offset(text, 1), 1);
        assert_eq!(char_index_to_utf16_offset(text, 2), 3);
        assert_eq!(char_index_to_utf16_offset(text, 3), 4);

        assert_eq!(utf16_offset_to_char_index(text, 0), 0);
        assert_eq!(utf16_offset_to_char_index(text, 1), 1);
        assert_eq!(utf16_offset_to_char_index(text, 2), 1);
        assert_eq!(utf16_offset_to_char_index(text, 3), 2);
        assert_eq!(utf16_offset_to_char_index(text, 4), 3);
    }
}
