use super::*;
use crate::models::{
    BoundingBox, EditorControlStyle, FontSourceKind, GlyphPaintParagraph, GlyphPaintRun,
    LayoutParagraph, LayoutRun, PaintMode, ParagraphEditContext, ParagraphStyle, ResolvedFontFace,
    ResolvedFontIdentity, RunStyle, SemanticRole, StyledRun, SymbolClass, VectorImageObject,
    VectorPageModel, VectorPathObject, VectorPathSegment, VectorRenderObject, VectorTextObject,
    VisualMarkerContent,
};
use crate::persistence::models::PersistableSemanticBlockSummary;
use crate::text::glyph_layout::build_editor_session_text_plan;

const CANONICAL_MIXED_TEXT: &str =
    "智能合约: Anchor Framework, Solana Program Library (SPL), ERC-20/721";

fn test_style() -> RunStyle {
    RunStyle {
        font_name: "Microsoft YaHei".to_string(),
        font_size: 10.0,
        color: "#000000".to_string(),
        is_bold: false,
        is_italic: false,
        is_underline: false,
        char_spacing: 0.0,
        scale_x: 1.0,
    }
}

fn test_bbox(left: f32, width: f32) -> BoundingBox {
    BoundingBox {
        left,
        top: 40.0,
        right: left + width,
        bottom: 52.0,
    }
}

fn test_layout_run(id: &str, text: &str, left: f32, width: f32) -> LayoutRun {
    LayoutRun {
        id: id.to_string(),
        text: text.to_string(),
        style: test_style(),
        bbox: test_bbox(left, width),
        origin_x: left,
        origin_y: 50.0,
        char_origins: Vec::new(),
        char_widths: Vec::new(),
        object_ids: vec!["obj-1".to_string()],
        object_indices: vec![0],
    }
}

fn layout_with_gaps(
    id: &str,
    text: &str,
    left: f32,
    origins: Vec<f32>,
    widths: Vec<f32>,
) -> LayoutRun {
    let right = origins
        .iter()
        .zip(widths.iter())
        .map(|(origin, width)| origin + width)
        .fold(left, f32::max);
    let mut run = test_layout_run(id, text, left, (right - left).max(1.0));
    run.char_origins = origins;
    run.char_widths = widths;
    run
}

fn session_from_runs(runs: Vec<LayoutRun>) -> ParagraphEditContext {
    session_from_runs_with_id("p1", runs)
}

fn session_from_runs_with_id(id: &str, runs: Vec<LayoutRun>) -> ParagraphEditContext {
    let anchor_bbox = runs.iter().fold(
        BoundingBox {
            left: f32::INFINITY,
            top: f32::INFINITY,
            right: f32::NEG_INFINITY,
            bottom: f32::NEG_INFINITY,
        },
        |acc, run| BoundingBox {
            left: acc.left.min(run.bbox.left),
            top: acc.top.min(run.bbox.top),
            right: acc.right.max(run.bbox.right),
            bottom: acc.bottom.max(run.bbox.bottom),
        },
    );

    ParagraphEditContext {
        anchor_bbox,
        paragraph: LayoutParagraph {
            id: id.to_string(),
            bbox: anchor_bbox,
            origin_x: anchor_bbox.left,
            origin_y: anchor_bbox.top,
            wrap_width: (anchor_bbox.right - anchor_bbox.left).max(1.0),
            runs,
            ..Default::default()
        },
    }
}

fn mixed_runs() -> Vec<LayoutRun> {
    vec![
        test_layout_run("r0", "智能合约: ", 0.0, 50.0),
        test_layout_run("r1", "A", 50.0, 5.0),
        test_layout_run("r2", "nchor", 58.0, 25.0),
        test_layout_run("r3", " ", 83.0, 4.0),
        test_layout_run("r4", "Fram", 87.0, 20.0),
        test_layout_run("r5", "ew", 110.0, 10.0),
        test_layout_run("r6", "ork", 123.0, 15.0),
        test_layout_run("r7", ", ", 138.0, 6.0),
        test_layout_run("r8", "S", 144.0, 5.0),
        test_layout_run("r9", "olana Program Library (", 152.0, 110.0),
        test_layout_run("r10", "S", 262.0, 5.0),
        test_layout_run("r11", "PL)", 270.0, 15.0),
        test_layout_run("r12", ", ER", 285.0, 20.0),
        test_layout_run("r13", "C", 308.0, 5.0),
        test_layout_run("r14", "-20/721", 316.0, 35.0),
    ]
}

#[test]
fn preserves_canonical_source() {
    let session = session_from_runs(mixed_runs());
    let reconstructed = build_editor_session_text_plan(&session).text;

    assert!(reconstructed.contains("A nchor"));
    assert!(reconstructed.contains("Fram ew ork"));
    assert!(reconstructed.contains("S PL"));
    assert!(reconstructed.contains("ER C -20"));

    let document_plan = build_editor_document_plan_from_session(&session);

    assert_eq!(document_plan.source_body_text(), CANONICAL_MIXED_TEXT);
    assert!(!document_plan.source_body_text().contains("A nchor"));
    assert_ne!(document_plan.source_body_text(), reconstructed);
}

#[test]
fn restores_visual_gaps() {
    let session = session_from_runs(vec![
        test_layout_run("r0", "智能合约:", 0.0, 46.0),
        test_layout_run("r1", "A", 51.0, 5.0),
        test_layout_run("r2", "nchor", 59.0, 25.0),
        test_layout_run("r3", "Framework,", 90.0, 58.0),
        test_layout_run("r4", "Solana", 154.0, 36.0),
        test_layout_run("r5", "Program", 196.0, 42.0),
        test_layout_run("r6", "Library", 244.0, 38.0),
        test_layout_run("r7", "(SPL),", 288.0, 32.0),
        test_layout_run("r8", "ERC-20/721", 326.0, 54.0),
    ]);

    let document_plan = build_editor_document_plan_from_session(&session);

    assert_eq!(
        document_plan.source_body_text(),
        "智能合约: Anchor Framework, Solana Program Library (SPL), ERC-20/721"
    );
    assert!(!document_plan.source_body_text().contains("A nchor"));
}

#[test]
fn restores_run_spaces() {
    let text = "智能合约:AnchorFramework,SolanaProgramLibrary(SPL),ERC-20/721";
    let chars = text.chars().collect::<Vec<_>>();
    let mut x = 0.0;
    let mut origins = Vec::new();
    let mut widths = Vec::new();
    for index in 0..chars.len() {
        origins.push(x);
        let width = if chars[index].is_ascii() { 5.0 } else { 10.0 };
        widths.push(width);
        x += width;
        if index + 1 < chars.len()
            && matches!(
                (chars[index], chars[index + 1]),
                (':', 'A')
                    | ('r', 'F')
                    | (',', 'S')
                    | ('a', 'P')
                    | ('m', 'L')
                    | ('y', '(')
                    | (',', 'E')
            )
        {
            x += 4.0;
        }
    }
    let session = session_from_runs(vec![layout_with_gaps(
        "single-run",
        text,
        0.0,
        origins,
        widths,
    )]);

    let document_plan = build_editor_document_plan_from_session(&session);

    assert_eq!(
        document_plan.source_body_text(),
        "智能合约: Anchor Framework, Solana Program Library (SPL), ERC-20/721"
    );
    assert!(!document_plan.source_body_text().contains("A nchor"));
    assert!(!document_plan.source_body_text().contains("S PL"));
    assert!(!document_plan.source_body_text().contains("ER C"));
}

fn test_resolved_font() -> ResolvedFontFace {
    ResolvedFontFace {
        identity: ResolvedFontIdentity {
            raw_name: "Microsoft YaHei".to_string(),
            canonical_family: "Microsoft YaHei".to_string(),
            style_name: "Regular".to_string(),
            weight: 400,
            is_italic: false,
            symbol_class: SymbolClass::None,
            subset_stripped: false,
        },
        render_family: "Microsoft YaHei".to_string(),
        metrics_family: "Microsoft YaHei".to_string(),
        source: FontSourceKind::SystemMatched,
        confidence: 1.0,
    }
}

fn test_paint_run(id: &str, text: &str, left: f32, width: f32) -> GlyphPaintRun {
    GlyphPaintRun {
        id: id.to_string(),
        page_index: 0,
        region_id: "region-1".to_string(),
        paragraph_id: "p1".to_string(),
        text: text.to_string(),
        bbox: test_bbox(left, width),
        origin_x: left,
        origin_y: 50.0,
        char_origins: Vec::new(),
        color: "#000000".to_string(),
        resolved_font: test_resolved_font(),
        font_size: 10.0,
        scale_x: 1.0,
        is_bold: false,
        is_italic: false,
        is_underline: false,
        paint_mode: PaintMode::Fill,
        object_ids: vec!["obj-1".to_string()],
        object_indices: vec![0],
    }
}

fn test_styled_run(text: &str, left: f32, width: f32, z_index: usize) -> StyledRun {
    StyledRun {
        text: text.to_string(),
        color: "#000000".to_string(),
        tx: left,
        ty: 50.0,
        width,
        font_size: 10.0,
        font_name: "Microsoft YaHei".to_string(),
        a: 1.0,
        d: 1.0,
        horizontal_scaling: 1.0,
        z_index,
        object_id: Some("obj-1".to_string()),
        ..Default::default()
    }
}

fn paragraph_from_session(session: ParagraphEditContext) -> GlyphPaintParagraph {
    GlyphPaintParagraph {
        id: session.paragraph.id.clone(),
        region_id: "region-1".to_string(),
        bbox: session.anchor_bbox,
        style: ParagraphStyle::default(),
        editor_session: session,
        editor_session_v2: None,
        control_style: EditorControlStyle::default(),
        semantic_role: SemanticRole::None,
        runs: Vec::new(),
    }
}

#[test]
fn marker_split_preserves_semantic_advance_and_body_width() {
    let session = session_from_runs(vec![
        test_layout_run("marker", "1. ", 0.0, 15.0),
        test_layout_run("body", "Body", 24.0, 40.0),
    ]);

    let split = split_editor_session(
        &session,
        "1. ".chars().count(),
        crate::text::list_semantics::ListMarkerKind::Numbering,
    )
    .expect("semantic marker should split");
    let marker = split.marker.expect("marker should be present");
    let body_text = split
        .body_session
        .paragraph
        .runs
        .iter()
        .map(|run| run.text.as_str())
        .collect::<String>();

    assert_eq!(marker.text, "1. ");
    assert_eq!(marker.advance, 24.0);
    assert_eq!(body_text, "Body");
    assert_eq!(split.body_session.paragraph.bbox.left, 24.0);
    assert_eq!(split.body_session.paragraph.wrap_width, 40.0);
}

#[test]
fn semantic_block_adapter_keeps_marker_out_of_body() {
    let mut marker_run = test_layout_run("marker", "●", 0.0, 10.0);
    marker_run.object_indices = vec![1];
    let mut body_run = test_layout_run("body", "Body", 24.0, 40.0);
    body_run.object_indices = vec![2];
    let session = session_from_runs(vec![marker_run, body_run]);

    let split = split_editor_session(
        &session,
        "●".chars().count(),
        crate::text::list_semantics::ListMarkerKind::Bullet,
    )
    .expect("semantic marker should split");
    let plan = EditContext {
        target_id: "list-item-1".to_string(),
        base_paragraph_id: "paragraph-1".to_string(),
        shell_bbox: session.anchor_bbox,
        source_body_text: "Body".to_string(),
        body_text_plan: build_editor_session_text_plan(&split.body_session),
        body_session: split.body_session,
        marker: split.marker,
        ..Default::default()
    };

    let block = plan.semantic_block();
    assert_eq!(block.body_text(), "Body");
    assert_eq!(
        block.primary_marker().and_then(|m| m.text_content()),
        Some("●")
    );
    assert!(block.validation.valid, "{:?}", block.validation.errors);
    assert_eq!(block.provenance.body_object_indices, vec![2]);
    assert_eq!(block.provenance.marker_object_indices, vec![1]);
}

#[test]
fn persisted_semantic_override_restores_marker_after_inference_loss() {
    // Simulate reparse where marker inference produced no marker (e.g. PDF run order
    // put body before marker). The persisted summary must win.
    let session = session_from_runs(vec![test_layout_run("body", "Body", 0.0, 40.0)]);
    let plan = EditContext {
        target_id: "list-item-1".to_string(),
        base_paragraph_id: "paragraph-1".to_string(),
        shell_bbox: session.anchor_bbox,
        source_body_text: "Body".to_string(),
        body_text_plan: build_editor_session_text_plan(&session),
        body_session: session,
        marker: None,
        ..Default::default()
    };

    let summary = PersistableSemanticBlockSummary {
        block_id: "list-item-1".to_string(),
        region_id: "list-item-1".to_string(),
        kind: "list-item".to_string(),
        body_text: "Body".to_string(),
        marker_text: Some("●".to_string()),
        body_object_indices: vec![2],
        marker_object_indices: vec![1],
        graphic_marker_object_indices: Vec::new(),
        is_cross_paragraph: false,
    };

    let restored = apply_persisted_semantic_override(plan, Some(&summary));
    assert_eq!(restored.source_body_text, "Body");
    let marker = restored
        .marker
        .as_ref()
        .expect("override must restore marker");
    assert_eq!(marker.text, "●");
}

#[test]
fn marker_split_maps_visual_body_start_to_raw_index() {
    let session = session_from_runs(vec![
        test_layout_run("marker", "1.", 0.0, 10.0),
        test_layout_run("body", "Body", 24.0, 40.0),
    ]);
    let full_source_text = "1. Body";
    let full_text_plan = build_editor_session_text_plan(&session);
    let paragraph = paragraph_from_session(session.clone());

    let split = resolve_marker_split(&paragraph, &session, &full_source_text, &full_text_plan, None);
    let marker = split.marker.expect("numbering marker should split");
    let body_text = split
        .body_session
        .paragraph
        .runs
        .iter()
        .map(|run| run.text.as_str())
        .collect::<String>();

    assert_eq!(marker.text, "1.");
    assert_eq!(body_text, "Body");
}

#[test]
fn geometric_marker_synthesis_accepts_only_same_line_left_candidates() {
    let full_session = session_from_runs_with_id(
        "p-geo",
        vec![
            test_layout_run("marker", "•", 10.0, 8.0),
            test_layout_run("body", "Body", 30.0, 40.0),
        ],
    );
    let body_session =
        session_from_runs_with_id("p-geo", vec![test_layout_run("body", "Body", 30.0, 40.0)]);
    let paragraph = paragraph_from_session(full_session);

    let marker = super::marker::synthesize_marker_from_paragraph(&paragraph, &body_session)
        .expect("left same-line bullet should synthesize");
    assert_eq!(marker.text, "•");
    assert_eq!(marker.advance, 0.0);
    assert_eq!(
        body_session.anchor_bbox.left + marker.advance,
        body_session
            .paragraph
            .runs
            .iter()
            .find(|run| !run.text.is_empty())
            .map(|run| run.origin_x)
            .unwrap(),
        "geometric marker advance must be body offset from anchor, not marker-to-body gap"
    );

    let different_line = session_from_runs_with_id(
        "p-geo-line",
        vec![
            {
                let mut run = test_layout_run("marker", "•", 10.0, 8.0);
                run.origin_y = 10.0;
                run
            },
            test_layout_run("body", "Body", 30.0, 40.0),
        ],
    );
    let different_line_paragraph = paragraph_from_session(different_line);
    assert!(
        super::marker::synthesize_marker_from_paragraph(&different_line_paragraph, &body_session)
            .is_none(),
        "candidate on a different baseline must not synthesize"
    );

    let right_side = session_from_runs_with_id(
        "p-geo-right",
        vec![
            test_layout_run("body", "Body", 30.0, 40.0),
            test_layout_run("marker", "•", 80.0, 8.0),
        ],
    );
    let right_side_paragraph = paragraph_from_session(right_side);
    assert!(
        super::marker::synthesize_marker_from_paragraph(&right_side_paragraph, &body_session)
            .is_none(),
        "candidate to the right of body must not synthesize"
    );
}

#[test]
fn prefers_vector_source() {
    let polluted_paint_run = test_paint_run("paint-1", "智能合约: A nchor", 0.0, 90.0);
    let paint_session = session_from_runs(vec![test_layout_run(
        "paint-layout-1",
        "智能合约: A nchor",
        0.0,
        90.0,
    )]);
    let paragraph = GlyphPaintParagraph {
        id: "p1".to_string(),
        region_id: "region-1".to_string(),
        bbox: paint_session.anchor_bbox,
        style: ParagraphStyle::default(),
        editor_session: paint_session,
        editor_session_v2: None,
        control_style: EditorControlStyle::default(),
        semantic_role: SemanticRole::None,
        runs: vec![polluted_paint_run],
    };
    let vector_model = VectorPageModel {
        page_index: 0,
        width: 400.0,
        height: 200.0,
        objects: vec![VectorRenderObject::Text(VectorTextObject {
            id: "obj-1".to_string(),
            runs: vec![
                test_styled_run("智能合约: ", 0.0, 50.0, 0),
                test_styled_run("A", 50.0, 5.0, 1),
                test_styled_run("nchor", 55.0, 25.0, 2),
            ],
            z_index: 0,
        })],
    };

    let document_plan = from_target_id(&paragraph, Some(&vector_model), "p1", None)
        .expect("document plan should use vector source");

    assert_eq!(document_plan.source_body_text(), "智能合约: Anchor");
    assert!(!document_plan.source_body_text().contains("A nchor"));
}

#[test]
fn keeps_overlay_source() {
    let source_session = session_from_runs(vec![test_layout_run(
        "source-layout-1",
        "编程语言: Rust (Solana/Anchor), Solidity (Ethereum)",
        0.0,
        260.0,
    )]);
    let mut patched_display_run = test_paint_run(
        "patched-display-1",
        "编程语言: Rust (Sona/Anchor), Solidity (Ethereum)",
        0.0,
        32.0,
    );
    patched_display_run.object_ids.clear();
    patched_display_run.object_indices.clear();

    let paragraph = GlyphPaintParagraph {
        id: "p1".to_string(),
        region_id: "region-1".to_string(),
        bbox: source_session.anchor_bbox,
        style: ParagraphStyle::default(),
        editor_session: source_session,
        editor_session_v2: None,
        control_style: EditorControlStyle::default(),
        semantic_role: SemanticRole::None,
        runs: vec![patched_display_run],
    };
    let vector_model = VectorPageModel {
        page_index: 0,
        width: 400.0,
        height: 200.0,
        objects: vec![VectorRenderObject::Text(VectorTextObject {
            id: "obj-1".to_string(),
            runs: vec![test_styled_run(
                "编程语言: Rust (Solana/Anchor), Solidity (Ethereum)",
                0.0,
                260.0,
                0,
            )],
            z_index: 0,
        })],
    };

    let document_plan = from_target_id(&paragraph, Some(&vector_model), "p1", None)
        .expect("persisted overlay target should recover the original vector source");

    assert_eq!(
        document_plan.source_body_text(),
        "编程语言: Rust (Solana/Anchor), Solidity (Ethereum)"
    );
    assert!(
        document_plan.body_session.anchor_bbox.right >= 259.0,
        "source geometry must come from the original vector text, not the shortened patched display run"
    );
}

#[test]
fn uses_vector_geometry() {
    let paint_session = session_from_runs(vec![test_layout_run(
        "paint-layout-1",
        "编程语言:Rust(Solana/Anchor),Solidity(Ethereum)",
        0.0,
        240.0,
    )]);
    let mut paint_run = test_paint_run(
        "paint-run-1",
        "编程语言:Rust(Solana/Anchor),Solidity(Ethereum)",
        0.0,
        240.0,
    );
    paint_run.object_ids.clear();
    paint_run.object_indices.clear();

    let paragraph = GlyphPaintParagraph {
        id: "p1".to_string(),
        region_id: "region-1".to_string(),
        bbox: paint_session.anchor_bbox,
        style: ParagraphStyle::default(),
        editor_session: paint_session,
        editor_session_v2: None,
        control_style: EditorControlStyle::default(),
        semantic_role: SemanticRole::None,
        runs: vec![paint_run],
    };
    let vector_model = VectorPageModel {
        page_index: 0,
        width: 400.0,
        height: 200.0,
        objects: vec![VectorRenderObject::Text(VectorTextObject {
            id: "unlinked-vector-object".to_string(),
            runs: vec![test_styled_run(
                "编程语言: Rust (Solana/Anchor), Solidity (Ethereum)",
                0.0,
                260.0,
                0,
            )],
            z_index: 0,
        })],
    };

    let document_plan = from_target_id(&paragraph, Some(&vector_model), "p1", None)
        .expect("geometry fallback should recover vector source");

    assert_eq!(
        document_plan.source_body_text(),
        "编程语言: Rust (Solana/Anchor), Solidity (Ethereum)"
    );
}

fn bullet_image(id: &str, x: f32, y: f32, size: f32) -> VectorRenderObject {
    VectorRenderObject::Image(VectorImageObject {
        id: id.to_string(),
        x,
        y,
        width: size,
        height: size,
        z_index: 1,
    })
}

fn decorative_path(id: &str, x: f32, y: f32, width: f32, height: f32) -> VectorRenderObject {
    VectorRenderObject::Path(VectorPathObject {
        id: id.to_string(),
        segments: vec![VectorPathSegment {
            command: "move".to_string(),
            points: vec![[x, y], [x + width, y + height]],
        }],
        fill_color: Some("#ff0000".to_string()),
        stroke_color: None,
        fill: true,
        stroke: false,
        stroke_width: 0.0,
        z_index: 1,
    })
}

#[test]
fn detects_graphic_marker_alongside_body() {
    // Body 文本位于 x=30..120；左侧 x=10..18 放置一个 8x8 的图形 bullet。
    let session = session_from_runs(vec![test_layout_run("body", "Body", 30.0, 90.0)]);
    let paragraph = paragraph_from_session(session.clone());
    let vector_model = VectorPageModel {
        page_index: 0,
        width: 400.0,
        height: 200.0,
        objects: vec![
            bullet_image("bullet-img", 10.0, 44.0, 8.0),
            decorative_path("deco-bar", 0.0, 48.0, 400.0, 2.0),
        ],
    };

    let plan =
        from_target_id(&paragraph, Some(&vector_model), "p1", None).expect("plan should resolve");

    let graphic_markers = &plan.graphic_markers;
    assert_eq!(
        graphic_markers.len(),
        1,
        "only the small bullet image should be detected as a graphic marker"
    );
    let marker = &graphic_markers[0];
    let VisualMarkerContent::Graphic { object_index, .. } = &marker.content else {
        panic!("expected graphic marker content");
    };
    assert_eq!(*object_index, 0, "marker should reference the bullet image");
    assert!(marker.contains_object_index(0));
}

#[test]
fn graphic_marker_keeps_shell_bbox_extent() {
    let session = session_from_runs(vec![test_layout_run("body", "Body", 30.0, 90.0)]);
    let paragraph = paragraph_from_session(session.clone());
    let vector_model = VectorPageModel {
        page_index: 0,
        width: 400.0,
        height: 200.0,
        objects: vec![bullet_image("bullet-img", 10.0, 44.0, 8.0)],
    };

    let plan =
        from_target_id(&paragraph, Some(&vector_model), "p1", None).expect("plan should resolve");

    assert!(plan.shell_bbox.left <= 10.0);
    assert!(plan.shell_bbox.right >= 120.0);
}
