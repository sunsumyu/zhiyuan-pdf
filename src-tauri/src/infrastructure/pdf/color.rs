//! Hex-color parsing for the PDF infrastructure.
//!
//! Two representations, intentionally distinct:
//! - **render** (`parse_rgb` / `parse_vello`) is *lenient* - a malformed input
//!   yields black, because rendering must never crash on a bad color string.
//! - **write** (`parse_pdf`) is *strict* - a malformed input yields `None`, so
//!   the caller can reject an invalid edit spec rather than silently emit black.
//!
//! The two parsers therefore diverge on the same bad input by design (see
//! `tests::lenient_vs_strict_divergence`).

use vello::peniko::Color;

/// Parse a `#rrggbb` hex string into `(r, g, b)` bytes.
/// Lenient: returns `(0, 0, 0)` for input shorter than 7 chars (render fallback).
pub(crate) fn parse_rgb(hex: &str) -> (u8, u8, u8) {
    if hex.len() < 7 {
        return (0, 0, 0);
    }
    let r = u8::from_str_radix(&hex[1..3], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[3..5], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[5..7], 16).unwrap_or(0);
    (r, g, b)
}

/// Parse a `#rrggbb` hex string into a vello `Color` with the given alpha.
/// Lenient: delegates to `parse_rgb`, so a malformed input yields opaque black.
pub(crate) fn parse_vello(hex: &str, alpha: f32) -> Color {
    let (r, g, b) = parse_rgb(hex);
    Color::rgba8(r, g, b, (alpha * 255.0) as u8)
}

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

    // --- parse_rgb / parse_vello (lenient, render path) ---

    #[test]
    fn parse_rgb_valid() {
        assert_eq!(parse_rgb("#ff8800"), (255, 136, 0));
    }

    #[test]
    fn parse_rgb_black() {
        assert_eq!(parse_rgb("#000000"), (0, 0, 0));
    }

    #[test]
    fn parse_rgb_short_input_falls_back_to_black() {
        // Never panics on byte slicing; returns black instead.
        assert_eq!(parse_rgb("#fff"), (0, 0, 0));
    }

    #[test]
    fn parse_rgb_empty_falls_back_to_black() {
        assert_eq!(parse_rgb(""), (0, 0, 0));
    }

    #[test]
    fn parse_vello_applies_alpha() {
        assert_eq!(parse_vello("#ff0000", 1.0), Color::rgba8(255, 0, 0, 255));
    }

    #[test]
    fn parse_vello_half_alpha() {
        assert_eq!(parse_vello("#ff0000", 0.5), Color::rgba8(255, 0, 0, 127));
    }

    #[test]
    fn parse_vello_lenient_on_bad_input() {
        // Malformed input -> opaque black, no panic.
        assert_eq!(parse_vello("#fff", 1.0), Color::rgba8(0, 0, 0, 255));
    }

    // --- the divergence that justifies two parsers ---

    #[test]
    fn lenient_vs_strict_divergence() {
        // Same malformed input: render yields black (keep drawing), write yields
        // None (reject the spec). Both are correct for their consumer's contract.
        let bad = "#fff";
        assert_eq!(parse_pdf(bad), None);
        assert_eq!(parse_vello(bad, 1.0), Color::rgba8(0, 0, 0, 255));
    }
}
