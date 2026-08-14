use crate::infrastructure::pdf_read::types::{ClassificationReason, PdfDocumentKind};

const SCANNED_MIN_PAGE_COVERAGE: f32 = 0.70;
const SCANNED_MIN_WIDTH_RATIO: f32 = 0.75;
const SCANNED_MIN_HEIGHT_RATIO: f32 = 0.75;
const OCR_SCAN_MIN_AVG_PAGE_BYTES: u64 = 180 * 1024;

pub(crate) struct ClassificationDecision {
    pub(crate) kind: PdfDocumentKind,
    pub(crate) confidence: f32,
    pub(crate) allow_scan_preview_first_paint: bool,
    pub(crate) reason: ClassificationReason,
}

impl ClassificationDecision {
    pub(crate) fn unknown() -> Self {
        Self {
            kind: PdfDocumentKind::Unknown,
            confidence: 0.0,
            allow_scan_preview_first_paint: false,
            reason: ClassificationReason::Unknown,
        }
    }
}

pub(crate) fn qualifies_as_scanned_page(
    page_width: f32,
    page_height: f32,
    image_width: u32,
    image_height: u32,
) -> bool {
    if page_width <= 1.0 || page_height <= 1.0 || image_width == 0 || image_height == 0 {
        return false;
    }

    let page_area = page_width * page_height;
    let image_area = image_width as f32 * image_height as f32;
    let coverage = image_area / page_area;
    let width_ratio = image_width as f32 / page_width;
    let height_ratio = image_height as f32 / page_height;

    coverage >= SCANNED_MIN_PAGE_COVERAGE
        && width_ratio >= SCANNED_MIN_WIDTH_RATIO
        && height_ratio >= SCANNED_MIN_HEIGHT_RATIO
}

pub(crate) fn likely_ocr_scanned_document(
    avg_page_bytes: u64,
    image_covers_page: bool,
    has_text_content: bool,
    has_font_resources: bool,
) -> bool {
    image_covers_page
        && avg_page_bytes >= OCR_SCAN_MIN_AVG_PAGE_BYTES
        && has_text_content
        && has_font_resources
}

fn scanned_confidence(
    avg_page_bytes: u64,
    image_covers_page: bool,
    has_text_content: bool,
    has_font_resources: bool,
) -> f32 {
    let mut score = 0.0f32;

    if image_covers_page {
        score += 0.55;
    }
    if avg_page_bytes >= OCR_SCAN_MIN_AVG_PAGE_BYTES {
        score += 0.25;
    }
    if !has_text_content {
        score += 0.20;
    } else if has_font_resources {
        score -= 0.05;
    }

    score.clamp(0.0, 1.0)
}

pub(crate) fn classify_open_decision(
    avg_page_bytes: u64,
    image_covers_page: bool,
    has_text_content: bool,
    has_font_resources: bool,
) -> ClassificationDecision {
    if image_covers_page && !(has_text_content || has_font_resources) {
        return ClassificationDecision {
            kind: PdfDocumentKind::Scanned,
            confidence: 1.0,
            allow_scan_preview_first_paint: true,
            reason: ClassificationReason::FullPageImageNoText,
        };
    }

    if likely_ocr_scanned_document(
        avg_page_bytes,
        image_covers_page,
        has_text_content,
        has_font_resources,
    ) {
        return ClassificationDecision {
            kind: PdfDocumentKind::Scanned,
            confidence: scanned_confidence(
                avg_page_bytes,
                image_covers_page,
                has_text_content,
                has_font_resources,
            ),
            allow_scan_preview_first_paint: true,
            reason: ClassificationReason::FullPageImageWithOcrLayer,
        };
    }

    if has_text_content {
        return ClassificationDecision {
            kind: PdfDocumentKind::Vector,
            confidence: 0.95,
            allow_scan_preview_first_paint: false,
            reason: ClassificationReason::TextOperatorsDominant,
        };
    }

    if has_font_resources {
        return ClassificationDecision {
            kind: PdfDocumentKind::Vector,
            confidence: 0.90,
            allow_scan_preview_first_paint: false,
            reason: ClassificationReason::FontResourcesDominant,
        };
    }

    ClassificationDecision {
        kind: PdfDocumentKind::Unknown,
        confidence: 0.0,
        allow_scan_preview_first_paint: false,
        reason: ClassificationReason::LowConfidenceFallback,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f32, expected: f32) {
        assert!((actual - expected).abs() < 1e-6, "actual={actual} expected={expected}");
    }

    #[test]
    fn classify_full_page_image_without_text_is_scanned() {
        let d = classify_open_decision(0, true, false, false);
        assert!(matches!(d.kind, PdfDocumentKind::Scanned));
        assert_close(d.confidence, 1.0);
        assert!(d.allow_scan_preview_first_paint);
        assert!(matches!(d.reason, ClassificationReason::FullPageImageNoText));
    }

    #[test]
    fn classify_ocr_layer_is_scanned_with_computed_confidence() {
        let d = classify_open_decision(OCR_SCAN_MIN_AVG_PAGE_BYTES, true, true, true);
        assert!(matches!(d.kind, PdfDocumentKind::Scanned));
        assert_close(d.confidence, 0.75);
        assert!(d.allow_scan_preview_first_paint);
        assert!(matches!(d.reason, ClassificationReason::FullPageImageWithOcrLayer));
    }

    #[test]
    fn classify_text_content_wins_even_with_covering_image() {
        // 图片盖满 + 有文本 + 无字体 + 页均字节小: OCR 分支不满足,落到文本分支
        let d = classify_open_decision(0, true, true, false);
        assert!(matches!(d.kind, PdfDocumentKind::Vector));
        assert_close(d.confidence, 0.95);
        assert!(!d.allow_scan_preview_first_paint);
        assert!(matches!(d.reason, ClassificationReason::TextOperatorsDominant));
    }

    #[test]
    fn classify_font_resources_only_is_vector() {
        let d = classify_open_decision(0, false, false, true);
        assert!(matches!(d.kind, PdfDocumentKind::Vector));
        assert_close(d.confidence, 0.90);
        assert!(!d.allow_scan_preview_first_paint);
        assert!(matches!(d.reason, ClassificationReason::FontResourcesDominant));
    }

    #[test]
    fn classify_no_signal_is_low_confidence_unknown() {
        let d = classify_open_decision(0, false, false, false);
        assert!(matches!(d.kind, PdfDocumentKind::Unknown));
        assert_close(d.confidence, 0.0);
        assert!(!d.allow_scan_preview_first_paint);
        assert!(matches!(d.reason, ClassificationReason::LowConfidenceFallback));
    }

    #[test]
    fn unknown_decision_matches_open_fallback() {
        let d = ClassificationDecision::unknown();
        assert!(matches!(d.kind, PdfDocumentKind::Unknown));
        assert_close(d.confidence, 0.0);
        assert!(!d.allow_scan_preview_first_paint);
        assert!(matches!(d.reason, ClassificationReason::Unknown));
    }

    #[test]
    fn qualifies_accepts_at_exact_ratios() {
        // 750/1000 = 0.75 恰好达阈值,覆盖率 0.75 >= 0.70
        assert!(qualifies_as_scanned_page(1000.0, 1000.0, 750, 1000));
        // 837x837/1000x1000 覆盖率 0.700569 恰好过 0.70
        assert!(qualifies_as_scanned_page(1000.0, 1000.0, 837, 837));
    }

    #[test]
    fn qualifies_rejects_just_below_each_threshold() {
        // 宽比 0.749 < 0.75
        assert!(!qualifies_as_scanned_page(1000.0, 1000.0, 749, 1000));
        // 覆盖率 0.698896 < 0.70(宽高比均达标)
        assert!(!qualifies_as_scanned_page(1000.0, 1000.0, 836, 836));
    }

    #[test]
    fn qualifies_rejects_degenerate_dimensions() {
        assert!(!qualifies_as_scanned_page(1.0, 1000.0, 837, 837));
        assert!(!qualifies_as_scanned_page(1000.0, 1000.0, 0, 837));
        assert!(!qualifies_as_scanned_page(1000.0, 1000.0, 837, 0));
    }

    #[test]
    fn likely_ocr_requires_all_four_conditions() {
        assert!(likely_ocr_scanned_document(180 * 1024, true, true, true));
        // 页均字节差 1
        assert!(!likely_ocr_scanned_document(180 * 1024 - 1, true, true, true));
        assert!(!likely_ocr_scanned_document(180 * 1024, false, true, true));
        assert!(!likely_ocr_scanned_document(180 * 1024, true, false, true));
        assert!(!likely_ocr_scanned_document(180 * 1024, true, true, false));
    }

    #[test]
    fn scanned_confidence_accumulates_and_clamps() {
        // 覆盖 0.55 + 无文本 0.20
        assert_close(scanned_confidence(0, true, false, false), 0.75);
        // 覆盖 0.55 + 字节 0.25 - 字体 0.05
        assert_close(scanned_confidence(180 * 1024, true, true, true), 0.75);
        // 无加分项,文本+字体扣 0.05 -> 钳到 0
        assert_close(scanned_confidence(0, false, true, true), 0.0);
        // 仅无文本 0.20
        assert_close(scanned_confidence(0, false, false, false), 0.20);
    }
}
