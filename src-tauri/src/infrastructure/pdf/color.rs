//! Hex-color parsing for the PDF infrastructure.
//!
//! `parse_pdf` is *strict* - a malformed input yields `None`, so the caller
//! can reject an invalid edit spec rather than silently emit black.

/// Parse a `#rrggbb` or `rrggbb` hex string into normalized `[0.0..1.0]` float RGB.
/// Strict: returns `None` unless the hex part is exactly 6 digits (write validation).
pub fn parse_pdf(color: &str) -> Option<[f32; 3]> {
    let hex = color.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some([r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0])
}

/// Naive CMYK-to-RGB conversion: `r = (1-c)(1-k)`, etc.
/// All inputs are clamped to [0.0, 1.0]. Returns `(r, g, b)` as `u8` bytes.
pub(crate) fn cmyk_to_rgb(c: f32, m: f32, y: f32, k: f32) -> (u8, u8, u8) {
    let c = c.clamp(0.0, 1.0);
    let m = m.clamp(0.0, 1.0);
    let y = y.clamp(0.0, 1.0);
    let k = k.clamp(0.0, 1.0);
    let r = ((1.0 - c) * (1.0 - k) * 255.0) as u8;
    let g = ((1.0 - m) * (1.0 - k) * 255.0) as u8;
    let b = ((1.0 - y) * (1.0 - k) * 255.0) as u8;
    (r, g, b)
}

/// Convert a gray value [0.0, 1.0] to a `#rrggbb` hex string.
pub(crate) fn gray_to_hex(gray: f32) -> String {
    let val = (gray.clamp(0.0, 1.0) * 255.0) as u8;
    format!("#{:02x}{:02x}{:02x}", val, val, val)
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- parse_pdf (strict, write path) ---

    #[test]
    fn parse_pdf_valid_with_hash() {
        assert_eq!(parse_pdf("#ff0000"), Some([1.0, 0.0, 0.0]));
    }

    #[test]
    fn parse_pdf_valid_without_hash() {
        assert_eq!(parse_pdf("00ff00"), Some([0.0, 1.0, 0.0]));
    }

    #[test]
    fn parse_pdf_whitespace_trimmed() {
        assert_eq!(parse_pdf("  #0000ff  "), Some([0.0, 0.0, 1.0]));
    }

    #[test]
    fn parse_pdf_short_shorthand_rejected() {
        // `#fff` is 3 hex digits, not 6 -> None.
        assert_eq!(parse_pdf("#fff"), None);
    }

    #[test]
    fn parse_pdf_eight_digits_rejected() {
        assert_eq!(parse_pdf("#11223344"), None);
    }

    #[test]
    fn parse_pdf_non_hex_rejected() {
        assert_eq!(parse_pdf("zzzzzz"), None);
    }

    #[test]
    fn parse_pdf_empty_rejected() {
        assert_eq!(parse_pdf(""), None);
    }

    // --- cmyk_to_rgb ---

    #[test]
    fn cmyk_pure_cyan() {
        let (r, g, b) = cmyk_to_rgb(1.0, 0.0, 0.0, 0.0);
        assert_eq!((r, g, b), (0, 255, 255));
    }

    #[test]
    fn cmyk_pure_magenta() {
        let (r, g, b) = cmyk_to_rgb(0.0, 1.0, 0.0, 0.0);
        assert_eq!((r, g, b), (255, 0, 255));
    }

    #[test]
    fn cmyk_all_zeros_is_white() {
        let (r, g, b) = cmyk_to_rgb(0.0, 0.0, 0.0, 0.0);
        assert_eq!((r, g, b), (255, 255, 255));
    }

    #[test]
    fn cmyk_all_ones_is_black() {
        let (r, g, b) = cmyk_to_rgb(1.0, 1.0, 1.0, 1.0);
        assert_eq!((r, g, b), (0, 0, 0));
    }

    // --- gray_to_hex ---

    #[test]
    fn gray_zero_is_black() {
        assert_eq!(gray_to_hex(0.0), "#000000");
    }

    #[test]
    fn gray_one_is_white() {
        assert_eq!(gray_to_hex(1.0), "#ffffff");
    }

    #[test]
    fn gray_half_is_midgray() {
        // 0.5 * 255 = 127.5, truncated to 127 → #7f7f7f
        assert_eq!(gray_to_hex(0.5), "#7f7f7f");
    }
}
