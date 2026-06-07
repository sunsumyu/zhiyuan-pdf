#[derive(Clone)]
pub struct PersistedTextLinePlan {
    pub font_alias: Vec<u8>,
    pub font_size: f32,
    pub encoded_bytes: Vec<u8>,
    pub tx: f32,
    pub ty: f32,
    pub width: f32,
    pub color: String,
    pub is_underline: bool,
    pub horizontal_scaling: f32,
    pub patch_idx: usize,
    pub line_seq: usize,
}
pub fn truncate_for_log(value: &str, limit: usize) -> String {
    let mut chars = value.chars();
    let mut out = String::new();
    for _ in 0..limit {
        if let Some(ch) = chars.next() {
            out.push(ch);
        } else {
            break;
        }
    }
    if chars.next().is_some() {
        out.push_str("...");
    }
    out
}
