pub fn truncate_debug_text(text: &str, limit: usize) -> String {
    let mut output = String::new();
    let mut chars = text.chars();
    for _ in 0..limit {
        if let Some(ch) = chars.next() {
            output.push(ch);
        } else {
            break;
        }
    }
    if chars.next().is_some() {
        output.push_str("...");
    }
    output
}
