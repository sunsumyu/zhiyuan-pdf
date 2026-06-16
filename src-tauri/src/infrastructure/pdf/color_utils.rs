use vello::peniko::Color;

/// Alpha-blend a foreground value onto a background value.
pub fn blend(bg: u8, fg: u8, alpha: f32) -> u8 {
    ((bg as f32 * (1.0 - alpha)) + (fg as f32 * alpha)) as u8
}

/// Parse a `#rrggbb` hex color string into (r, g, b) bytes.
/// Returns (0, 0, 0) for invalid or short input.
pub fn parse_hex_color_rgb(hex: &str) -> (u8, u8, u8) {
    if hex.len() < 7 {
        return (0, 0, 0);
    }
    let r = u8::from_str_radix(&hex[1..3], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[3..5], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[5..7], 16).unwrap_or(0);
    (r, g, b)
}

/// Parse a `#rrggbb` hex color and wrap as a vello `Color` with the given alpha.
pub fn parse_hex_vello_color(hex: &str, alpha: f32) -> Color {
    let (r, g, b) = parse_hex_color_rgb(hex);
    Color::rgba8(r, g, b, (alpha * 255.0) as u8)
}

/// Parse a `#rrggbb` or `rrggbb` hex string into normalized [0..1] float RGB.
pub fn parse_pdf_hex_color(color: &str) -> Option<[f32; 3]> {
    let hex = color.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some([r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0])
}
