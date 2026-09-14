//! 原内联测试模块（自 glyph_layout.rs 原样搬移，closeout #06）。

use super::test_reexports::*;
use super::*;

use crate::models::ParagraphEditContext;

// ─── Predicate Functions ─────────────────────────────────────────────

#[test]
fn is_decorative_glyph_known_chars() {
    let decorative = ['•', '●', '▪', '◦', '·', '○', '-', '▶'];
    for ch in &decorative {
        assert!(
            is_decorative_glyph(*ch),
            "expected '{}' to be decorative",
            ch
        );
    }
}

#[test]
fn is_decorative_glyph_non_decorative() {
    let non_decorative = ['a', '张', '1', '(', '。'];
    for ch in &non_decorative {
        assert!(
            !is_decorative_glyph(*ch),
            "expected {} to be non-decorative",
            ch
        );
    }
}

#[test]
fn is_decorative_text_single_glyph() {
    assert!(is_decorative_text("•"));
    assert!(is_decorative_text("●"));
}

#[test]
fn is_decorative_text_multiple_glyphs() {
    assert!(is_decorative_text("•••"));
    assert!(is_decorative_text("●●●"));
}

#[test]
fn is_decorative_text_with_whitespace() {
    assert!(is_decorative_text(" • "));
    assert!(is_decorative_text("\t●\n"));
}

#[test]
fn is_decorative_text_empty() {
    assert!(!is_decorative_text(""));
    assert!(!is_decorative_text("   "));
}

#[test]
fn is_decorative_text_mixed() {
    assert!(!is_decorative_text("•a"));
    assert!(!is_decorative_text("a•"));
    assert!(!is_decorative_text("• hello"));
}

#[test]
fn is_cjk_unified_chinese() {
    assert!(is_cjk_unified('张'));
    assert!(is_cjk_unified('三'));
    assert!(is_cjk_unified('中'));
}

#[test]
fn is_cjk_unified_japanese() {
    assert!(is_cjk_unified('あ'));
    assert!(is_cjk_unified('カ'));
    assert!(is_cjk_unified('ン'));
}

#[test]
fn is_cjk_unified_korean() {
    assert!(is_cjk_unified('한'));
    assert!(is_cjk_unified('국'));
}

#[test]
fn is_cjk_unified_non_cjk() {
    assert!(!is_cjk_unified('a'));
    assert!(!is_cjk_unified('1'));
    assert!(!is_cjk_unified('('));
}

#[test]
fn is_open_punctuation_ascii() {
    assert!(is_open_punctuation('('));
    assert!(is_open_punctuation('['));
    assert!(is_open_punctuation('{'));
}

#[test]
fn is_open_punctuation_cjk() {
    assert!(is_open_punctuation('（'));
    assert!(is_open_punctuation('【'));
    assert!(is_open_punctuation('《'));
    assert!(is_open_punctuation('「'));
    assert!(is_open_punctuation('『'));
}

#[test]
fn is_open_punctuation_non_punct() {
    assert!(!is_open_punctuation('a'));
    assert!(!is_open_punctuation(')'));
    assert!(!is_open_punctuation('。'));
}

#[test]
fn is_close_punctuation_ascii() {
    assert!(is_close_punctuation(')'));
    assert!(is_close_punctuation(']'));
    assert!(is_close_punctuation('}'));
    assert!(is_close_punctuation(','));
    assert!(is_close_punctuation('.'));
    assert!(is_close_punctuation(';'));
    assert!(is_close_punctuation(':'));
    assert!(is_close_punctuation('!'));
    assert!(is_close_punctuation('?'));
}

#[test]
fn is_close_punctuation_cjk() {
    assert!(is_close_punctuation('）'));
    assert!(is_close_punctuation('】'));
    assert!(is_close_punctuation('》'));
    assert!(is_close_punctuation('」'));
    assert!(is_close_punctuation('』'));
    assert!(is_close_punctuation('，'));
    assert!(is_close_punctuation('。'));
    assert!(is_close_punctuation('；'));
    assert!(is_close_punctuation('：'));
    assert!(is_close_punctuation('！'));
    assert!(is_close_punctuation('？'));
}

#[test]
fn should_allow_synthetic_gap_cjk_pair() {
    // CJK pairs should NOT have synthetic gap
    assert!(!should_allow_synthetic_gap('张', '三'));
    assert!(!should_allow_synthetic_gap('中', '国'));
}

#[test]
fn should_allow_synthetic_gap_open_punct() {
    // Open punctuation should NOT have synthetic gap
    assert!(!should_allow_synthetic_gap('a', '('));
    assert!(!should_allow_synthetic_gap('(', 'b'));
    assert!(!should_allow_synthetic_gap('a', '（'));
}

#[test]
fn should_allow_synthetic_gap_close_punct() {
    // Close punctuation at next position should NOT have synthetic gap
    assert!(!should_allow_synthetic_gap('a', ')'));
    assert!(!should_allow_synthetic_gap('a', '。'));
}

#[test]
fn should_allow_synthetic_gap_normal_pair() {
    // Normal ASCII pairs SHOULD have synthetic gap
    assert!(should_allow_synthetic_gap('a', 'b'));
}

// ─── EditorSessionTextPlan Bidirectional Mapping ────────────────────

#[test]
fn text_plan_empty() {
    let plan = EditorSessionTextPlan::default();
    assert_eq!(plan.map_raw_to_reconstructed(0), 0);
    assert_eq!(plan.map_reconstructed_to_raw(0), 0);
    assert_eq!(plan.reconstructed_char_count(), 0);
}

#[test]
fn text_plan_roundtrip_consistency() {
    // Test that mapping is consistent in both directions
    let plan = EditorSessionTextPlan {
        text: "abc".to_string(),
        slots: vec![],
        raw_to_reconstructed: vec![0, 1, 2, 3],
        reconstructed_to_raw: vec![0, 1, 2, 3],
    };

    // Round-trip: raw -> reconstructed -> raw
    for raw in 0..=3 {
        let reconstructed = plan.map_raw_to_reconstructed(raw);
        let back_to_raw = plan.map_reconstructed_to_raw(reconstructed);
        assert_eq!(back_to_raw, raw, "round-trip failed for raw={}", raw);
    }

    // Round-trip: reconstructed -> raw -> reconstructed
    for recon in 0..=3 {
        let raw = plan.map_reconstructed_to_raw(recon);
        let back_to_recon = plan.map_raw_to_reconstructed(raw);
        assert_eq!(
            back_to_recon, recon,
            "round-trip failed for reconstructed={}",
            recon
        );
    }
}

#[test]
fn text_plan_gap_insertion_mapping() {
    // Simulate: raw "ab" with a gap inserted between -> reconstructed "a b"
    // raw indices: [0, 1] -> reconstructed indices: [0, 2]
    let plan = EditorSessionTextPlan {
        text: "a b".to_string(),
        slots: vec![],
        raw_to_reconstructed: vec![0, 2], // raw[0]->recon[0], raw[1]->recon[2]
        reconstructed_to_raw: vec![0, 0, 1], // recon[0]->raw[0], recon[1]->raw[0], recon[2]->raw[1]
    };

    assert_eq!(plan.map_raw_to_reconstructed(0), 0);
    assert_eq!(plan.map_raw_to_reconstructed(1), 2);

    assert_eq!(plan.map_reconstructed_to_raw(0), 0);
    assert_eq!(plan.map_reconstructed_to_raw(1), 0); // gap char maps to previous raw
    assert_eq!(plan.map_reconstructed_to_raw(2), 1);

    assert_eq!(plan.reconstructed_char_count(), 3);
}

#[test]
fn text_plan_boundary_overflow() {
    let plan = EditorSessionTextPlan {
        text: "abc".to_string(),
        slots: vec![],
        raw_to_reconstructed: vec![0, 1, 2],
        reconstructed_to_raw: vec![0, 1, 2],
    };

    // Out-of-bounds should return last element
    assert_eq!(plan.map_raw_to_reconstructed(100), 2);
    assert_eq!(plan.map_reconstructed_to_raw(100), 2);
}

// ─── Infer Run Advance ───────────────────────────────────────────────

#[test]
fn infer_run_advance_from_bbox() {
    let run = LayoutRun {
        id: "test-1".to_string(),
        text: "abc".to_string(),
        style: crate::models::RunStyle {
            font_name: "Arial".to_string(),
            font_size: 12.0,
            color: "#000000".to_string(),
            is_bold: false,
            is_italic: false,
            ..Default::default()
        },
        bbox: crate::models::BoundingBox {
            left: 10.0,
            top: 0.0,
            right: 40.0,
            bottom: 12.0,
        },
        origin_x: 10.0,
        origin_y: 0.0,
        char_origins: vec![],
        char_widths: vec![],
        object_ids: vec![],
        object_indices: vec![],
    };
    // Advance should be bbox width / char count = 30 / 3 = 10.0
    let advance = infer_run_advance(&run);
    assert!((advance - 10.0).abs() < 0.01, "advance={}", advance);
}

#[test]
fn infer_run_advance_single_char() {
    let run = LayoutRun {
        id: "test-2".to_string(),
        text: "a".to_string(),
        style: crate::models::RunStyle {
            font_name: "Arial".to_string(),
            font_size: 12.0,
            color: "#000000".to_string(),
            is_bold: false,
            is_italic: false,
            ..Default::default()
        },
        bbox: crate::models::BoundingBox {
            left: 0.0,
            top: 0.0,
            right: 8.0,
            bottom: 12.0,
        },
        origin_x: 0.0,
        origin_y: 0.0,
        char_origins: vec![],
        char_widths: vec![],
        object_ids: vec![],
        object_indices: vec![],
    };
    // Single char: advance = bbox width / 1 = 8.0
    let advance = infer_run_advance(&run);
    assert!((advance - 8.0).abs() < 0.01, "advance={}", advance);
}

// ─── Gap Insertion Logic ─────────────────────────────────────────────

#[test]
fn needs_gap_large_coordinate_jump() {
    // Two runs with large coordinate jump should need gap
    let prev = LayoutRun {
        id: "prev".to_string(),
        text: "abc".to_string(),
        style: crate::models::RunStyle {
            font_name: "Arial".to_string(),
            font_size: 12.0,
            color: "#000000".to_string(),
            is_bold: false,
            is_italic: false,
            ..Default::default()
        },
        bbox: crate::models::BoundingBox {
            left: 0.0,
            top: 0.0,
            right: 30.0,
            bottom: 12.0,
        },
        origin_x: 0.0,
        origin_y: 0.0,
        char_origins: vec![],
        char_widths: vec![],
        object_ids: vec![],
        object_indices: vec![],
    };
    let next = LayoutRun {
        id: "next".to_string(),
        text: "xyz".to_string(),
        style: crate::models::RunStyle {
            font_name: "Arial".to_string(),
            font_size: 12.0,
            color: "#000000".to_string(),
            is_bold: false,
            is_italic: false,
            ..Default::default()
        },
        bbox: crate::models::BoundingBox {
            left: 100.0,
            top: 0.0,
            right: 130.0,
            bottom: 12.0,
        },
        origin_x: 100.0,
        origin_y: 0.0,
        char_origins: vec![],
        char_widths: vec![],
        object_ids: vec![],
        object_indices: vec![],
    };
    let typical_delta = 5.0;
    assert!(needs_gap(&prev, &next, Some(typical_delta)));
}

#[test]
fn needs_gap_small_coordinate_jump() {
    // Two runs with small coordinate jump should NOT need gap
    // Use very small gap (1.0) which is less than contiguous_join_gap
    let prev = LayoutRun {
        id: "prev".to_string(),
        text: "abc".to_string(),
        style: crate::models::RunStyle {
            font_name: "Arial".to_string(),
            font_size: 12.0,
            color: "#000000".to_string(),
            is_bold: false,
            is_italic: false,
            ..Default::default()
        },
        bbox: crate::models::BoundingBox {
            left: 0.0,
            top: 0.0,
            right: 30.0,
            bottom: 12.0,
        },
        origin_x: 0.0,
        origin_y: 0.0,
        // Relative offsets from origin_x
        char_origins: vec![0.0, 10.0, 20.0, 30.0],
        char_widths: vec![10.0, 10.0, 10.0],
        object_ids: vec![],
        object_indices: vec![],
    };
    let next = LayoutRun {
        id: "next".to_string(),
        text: "xyz".to_string(),
        style: crate::models::RunStyle {
            font_name: "Arial".to_string(),
            font_size: 12.0,
            color: "#000000".to_string(),
            is_bold: false,
            is_italic: false,
            ..Default::default()
        },
        bbox: crate::models::BoundingBox {
            left: 31.0,
            top: 0.0,
            right: 61.0,
            bottom: 12.0,
        },
        origin_x: 31.0,
        origin_y: 0.0,
        // Relative offsets from origin_x (31.0)
        char_origins: vec![0.0, 10.0, 20.0, 30.0],
        char_widths: vec![10.0, 10.0, 10.0],
        object_ids: vec![],
        object_indices: vec![],
    };
    let typical_delta = 10.0;
    // geometric_gap = next_left - prev_right = (31+0) - (0+30) = 1.0
    // contiguous_join_gap ≈ max(0.96, 1.8, 0.9) = 1.8
    // 1.0 < 1.8, so no gap
    assert!(!needs_gap(&prev, &next, Some(typical_delta)));
}

// ─── Visual Line Detection ───────────────────────────────────────────

#[test]
fn same_visual_line_same_y() {
    let prev = LayoutRun {
        id: "prev".to_string(),
        text: "abc".to_string(),
        style: crate::models::RunStyle::default(),
        bbox: crate::models::BoundingBox {
            left: 0.0,
            top: 10.0,
            right: 30.0,
            bottom: 22.0,
        },
        origin_x: 0.0,
        origin_y: 10.0,
        char_origins: vec![],
        char_widths: vec![],
        object_ids: vec![],
        object_indices: vec![],
    };
    let next = LayoutRun {
        id: "next".to_string(),
        text: "xyz".to_string(),
        style: crate::models::RunStyle::default(),
        bbox: crate::models::BoundingBox {
            left: 40.0,
            top: 10.0,
            right: 70.0,
            bottom: 22.0,
        },
        origin_x: 40.0,
        origin_y: 10.0,
        char_origins: vec![],
        char_widths: vec![],
        object_ids: vec![],
        object_indices: vec![],
    };
    assert!(same_visual_line(&prev, &next));
}

#[test]
fn same_visual_line_different_y() {
    let prev = LayoutRun {
        id: "prev".to_string(),
        text: "abc".to_string(),
        style: crate::models::RunStyle::default(),
        bbox: crate::models::BoundingBox {
            left: 0.0,
            top: 10.0,
            right: 30.0,
            bottom: 22.0,
        },
        origin_x: 0.0,
        origin_y: 10.0,
        char_origins: vec![],
        char_widths: vec![],
        object_ids: vec![],
        object_indices: vec![],
    };
    let next = LayoutRun {
        id: "next".to_string(),
        text: "xyz".to_string(),
        style: crate::models::RunStyle::default(),
        bbox: crate::models::BoundingBox {
            left: 0.0,
            top: 30.0,
            right: 30.0,
            bottom: 42.0,
        },
        origin_x: 0.0,
        origin_y: 30.0,
        char_origins: vec![],
        char_widths: vec![],
        object_ids: vec![],
        object_indices: vec![],
    };
    assert!(!same_visual_line(&prev, &next));
}

// ─── Has Suspicious Run Geometry ─────────────────────────────────────

#[test]
fn has_suspicious_run_geometry_normal() {
    let session = ParagraphEditContext {
        anchor_bbox: crate::models::BoundingBox::default(),
        paragraph: crate::models::LayoutParagraph {
            id: "test".to_string(),
            bbox: crate::models::BoundingBox::default(),
            style: crate::models::ParagraphStyle::default(),
            runs: vec![LayoutRun {
                id: "run-1".to_string(),
                text: "abc".to_string(),
                style: crate::models::RunStyle::default(),
                bbox: crate::models::BoundingBox {
                    left: 0.0,
                    top: 0.0,
                    right: 30.0,
                    bottom: 12.0,
                },
                origin_x: 0.0,
                origin_y: 0.0,
                char_origins: vec![],
                char_widths: vec![],
                object_ids: vec![],
                object_indices: vec![],
            }],
            object_ids: vec![],
            origin_x: 0.0,
            origin_y: 0.0,
            wrap_width: 0.0,
        },
    };
    assert!(!has_suspicious_run_geometry(
        &session,
        |_| false,
        |run| run.bbox.right - run.bbox.left,
    ));
}

#[test]
fn has_suspicious_run_geometry_oversized_bbox() {
    let session = ParagraphEditContext {
        anchor_bbox: crate::models::BoundingBox::default(),
        paragraph: crate::models::LayoutParagraph {
            id: "test".to_string(),
            bbox: crate::models::BoundingBox::default(),
            style: crate::models::ParagraphStyle::default(),
            runs: vec![LayoutRun {
                id: "run-1".to_string(),
                text: "abc".to_string(),
                style: crate::models::RunStyle::default(),
                bbox: crate::models::BoundingBox {
                    left: 0.0,
                    top: 0.0,
                    right: 100.0,
                    bottom: 12.0,
                },
                origin_x: 0.0,
                origin_y: 0.0,
                char_origins: vec![],
                char_widths: vec![],
                object_ids: vec![],
                object_indices: vec![],
            }],
            object_ids: vec![],
            origin_x: 0.0,
            origin_y: 0.0,
            wrap_width: 0.0,
        },
    };
    assert!(has_suspicious_run_geometry(
        &session,
        |_| false,
        |_| 10.0, // measured width is 10, bbox is 100 -> suspicious
    ));
}

#[test]
fn has_suspicious_run_geometry_decorative_excluded() {
    let session = ParagraphEditContext {
        anchor_bbox: crate::models::BoundingBox::default(),
        paragraph: crate::models::LayoutParagraph {
            id: "test".to_string(),
            bbox: crate::models::BoundingBox::default(),
            style: crate::models::ParagraphStyle::default(),
            runs: vec![LayoutRun {
                id: "run-1".to_string(),
                text: "•••".to_string(),
                style: crate::models::RunStyle::default(),
                bbox: crate::models::BoundingBox {
                    left: 0.0,
                    top: 0.0,
                    right: 100.0,
                    bottom: 12.0,
                },
                origin_x: 0.0,
                origin_y: 0.0,
                char_origins: vec![],
                char_widths: vec![],
                object_ids: vec![],
                object_indices: vec![],
            }],
            object_ids: vec![],
            origin_x: 0.0,
            origin_y: 0.0,
            wrap_width: 0.0,
        },
    };
    assert!(!has_suspicious_run_geometry(&session, |_| false, |_| 10.0,));
}

#[test]
fn has_suspicious_run_geometry_symbol_font_excluded() {
    let session = ParagraphEditContext {
        anchor_bbox: crate::models::BoundingBox::default(),
        paragraph: crate::models::LayoutParagraph {
            id: "test".to_string(),
            bbox: crate::models::BoundingBox::default(),
            style: crate::models::ParagraphStyle::default(),
            runs: vec![LayoutRun {
                id: "run-1".to_string(),
                text: "abc".to_string(),
                style: crate::models::RunStyle {
                    font_name: "Symbol".to_string(),
                    ..Default::default()
                },
                bbox: crate::models::BoundingBox {
                    left: 0.0,
                    top: 0.0,
                    right: 100.0,
                    bottom: 12.0,
                },
                origin_x: 0.0,
                origin_y: 0.0,
                char_origins: vec![],
                char_widths: vec![],
                object_ids: vec![],
                object_indices: vec![],
            }],
            object_ids: vec![],
            origin_x: 0.0,
            origin_y: 0.0,
            wrap_width: 0.0,
        },
    };
    assert!(!has_suspicious_run_geometry(
        &session,
        |name| name == "Symbol",
        |_| 10.0,
    ));
}
