/// UTF-16 偏移量与 char 索引的双向转换类型定义。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharIndex(pub usize);

impl CharIndex {
    /// 将 char 索引转换为 UTF-16 偏移量。
    pub fn to_utf16(&self, text: &str) -> Utf16Offset {
        Utf16Offset(text.chars().take(self.0).map(|ch| ch.len_utf16()).sum())
    }
}

impl From<usize> for CharIndex {
    fn from(index: usize) -> Self {
        Self(index)
    }
}

impl From<CharIndex> for usize {
    fn from(index: CharIndex) -> Self {
        index.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Utf16Offset(pub usize);

impl Utf16Offset {
    /// 将 UTF-16 偏移量转换为 char 索引。
    pub fn to_char(&self, text: &str) -> CharIndex {
        let mut consumed = 0usize;
        for (char_index, ch) in text.chars().enumerate() {
            if consumed.saturating_add(ch.len_utf16()) > self.0 {
                return CharIndex(char_index);
            }
            consumed += ch.len_utf16();
        }
        CharIndex(text.chars().count())
    }
}

impl From<usize> for Utf16Offset {
    fn from(offset: usize) -> Self {
        Self(offset)
    }
}

impl From<Utf16Offset> for usize {
    fn from(offset: Utf16Offset) -> Self {
        offset.0
    }
}

#[cfg(test)]
mod tests {
    use super::{CharIndex, Utf16Offset};

    #[test]
    fn converts_utf16_indexes() {
        let text = "A💡中";

        // char -> utf16
        assert_eq!(CharIndex(0).to_utf16(text), Utf16Offset(0));
        assert_eq!(CharIndex(1).to_utf16(text), Utf16Offset(1));
        assert_eq!(CharIndex(2).to_utf16(text), Utf16Offset(3));
        assert_eq!(CharIndex(3).to_utf16(text), Utf16Offset(4));

        // utf16 -> char
        assert_eq!(Utf16Offset(0).to_char(text), CharIndex(0));
        assert_eq!(Utf16Offset(1).to_char(text), CharIndex(1));
        assert_eq!(Utf16Offset(2).to_char(text), CharIndex(1));
        assert_eq!(Utf16Offset(3).to_char(text), CharIndex(2));
        assert_eq!(Utf16Offset(4).to_char(text), CharIndex(3));
    }
}
