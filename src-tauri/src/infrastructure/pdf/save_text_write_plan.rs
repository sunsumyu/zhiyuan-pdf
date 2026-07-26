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
    pub render_mode: i32,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_render_mode_and_scaling_preservation() {
        let line_plan = PersistedTextLinePlan {
            font_alias: b"F1".to_vec(),
            font_size: 12.0,
            encoded_bytes: vec![0, 65],
            tx: 10.0,
            ty: 20.0,
            width: 100.0,
            color: "#000000".to_string(),
            is_underline: false,
            horizontal_scaling: 105.0,
            render_mode: 2, // Bold / Stroke & Fill
            patch_idx: 1,
            line_seq: 0,
        };

        assert_eq!(line_plan.render_mode, 2);
        assert!((line_plan.horizontal_scaling - 105.0).abs() < 0.001);
    }
}
