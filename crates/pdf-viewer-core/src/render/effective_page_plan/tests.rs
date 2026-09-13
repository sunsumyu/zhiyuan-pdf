//! 原内联测试模块（自 effective_page_plan.rs 原样搬移，closeout #06）。

use super::{
    build_effective_glyph_render_plan, build_effective_vector_render_plan,
    EffectiveGlyphRenderEntry, EffectiveVectorRenderEntry,
};
use crate::edit::active_target::ActiveEditorTarget;
use crate::edit::paragraph_overlay::{ParagraphRenderOverlay, ParagraphRenderOverlayOwner};
use crate::edit::paragraph_scene::ParagraphEditorScene;
use crate::models::{
    BoundingBox, EditorControlStyle, GlyphPaintParagraph, GlyphPaintPlan, GlyphPaintRegion,
    GlyphPaintRun, LayoutMode, LayoutParagraph, LayoutRole, LayoutRun, ParagraphEditContext,
    StyledRun, VectorPageModel, VectorPathObject, VectorPathSegment, VectorRenderObject,
    VectorTextObject,
};

fn horizontal_stroked_path(id: &str, y: f32) -> VectorRenderObject {
    horizontal_stroked_path_between(id, 80.0, 340.0, y)
}

fn horizontal_stroked_path_between(id: &str, left: f32, right: f32, y: f32) -> VectorRenderObject {
    VectorRenderObject::Path(VectorPathObject {
        id: id.to_string(),
        segments: vec![
            VectorPathSegment {
                command: "move".to_string(),
                points: vec![[left, y]],
            },
            VectorPathSegment {
                command: "line".to_string(),
                points: vec![[right, y]],
            },
        ],
        stroke: true,
        stroke_width: 4.0,
        stroke_color: Some("#0070c0".to_string()),
        ..Default::default()
    })
}

fn active_overlay_for_body(body_bbox: BoundingBox) -> ParagraphRenderOverlay {
    let target = ActiveEditorTarget {
        paragraph_id: "p-1".to_string(),
        scene: ParagraphEditorScene {
            shell_bbox: BoundingBox {
                left: 40.0,
                top: 96.0,
                right: 360.0,
                bottom: 116.0,
            },
            body_session: ParagraphEditContext {
                anchor_bbox: body_bbox,
                paragraph: LayoutParagraph::default(),
            },
            ..Default::default()
        },
        ..Default::default()
    };

    ParagraphRenderOverlay {
        owner: ParagraphRenderOverlayOwner::ActiveEditorShell,
        target,
        source_object_indices: Vec::new(),
        graphic_markers: Vec::new(),
        source_text: "body".to_string(),
        draft_text: "body".to_string(),
        replaces_source: true,
        marker_text_override: None,
    }
}

fn active_overlay_for_source_object(object_id: &str) -> ParagraphRenderOverlay {
    let mut overlay = active_overlay_for_body(BoundingBox {
        left: 90.0,
        top: 100.0,
        right: 330.0,
        bottom: 112.0,
    });
    overlay.target.scene.original_runs.clear();
    overlay.target.scene.body_session.paragraph.runs = vec![LayoutRun {
        id: "body-run".to_string(),
        text: "编程语言: Rust".to_string(),
        object_ids: vec![object_id.to_string()],
        bbox: overlay.target.scene.body_session.anchor_bbox,
        ..Default::default()
    }];
    overlay
}

fn persisted_overlay_for_source_object(object_id: &str) -> ParagraphRenderOverlay {
    let mut overlay = active_overlay_for_source_object(object_id);
    overlay.owner = ParagraphRenderOverlayOwner::PersistedPageCanvas;
    overlay
}

fn text_object_without_run_ids(id: &str) -> VectorRenderObject {
    VectorRenderObject::Text(VectorTextObject {
        id: id.to_string(),
        runs: vec![StyledRun {
            text: "编程语言: Rust".to_string(),
            tx: 90.0,
            ty: 112.0,
            width: 240.0,
            font_size: 12.0,
            object_id: None,
            ..Default::default()
        }],
        ..Default::default()
    })
}

fn glyph_plan_without_run_ids() -> GlyphPaintPlan {
    let bbox = BoundingBox {
        left: 90.0,
        top: 100.0,
        right: 330.0,
        bottom: 112.0,
    };
    GlyphPaintPlan {
        page_index: 0,
        width: 595.0,
        height: 842.0,
        regions: vec![GlyphPaintRegion {
            id: "r-1".to_string(),
            kind: LayoutRole::Paragraph,
            layout_mode: LayoutMode::Flow,
            bbox,
            paragraphs: vec![GlyphPaintParagraph {
                id: "p-1".to_string(),
                region_id: "r-1".to_string(),
                bbox,
                style: Default::default(),
                editor_session: ParagraphEditContext {
                    anchor_bbox: bbox,
                    paragraph: LayoutParagraph::default(),
                },
                control_style: EditorControlStyle::default(),
                semantic_role: Default::default(),
                runs: vec![GlyphPaintRun {
                    id: "glyph-run-1".to_string(),
                    page_index: 0,
                    region_id: "r-1".to_string(),
                    paragraph_id: "p-1".to_string(),
                    text: "编程语言: Rust".to_string(),
                    bbox,
                    origin_x: 90.0,
                    origin_y: 112.0,
                    font_size: 12.0,
                    object_ids: Vec::new(),
                    ..Default::default()
                }],
            }],
            object_ids: Vec::new(),
        }],
        external_objects: Vec::new(),
    }
}

#[test]
fn suppresses_zero_height_path() {
    let model = VectorPageModel {
        width: 595.0,
        height: 842.0,
        objects: vec![horizontal_stroked_path("blue-row-bar", 106.0)],
        ..Default::default()
    };
    let overlay = active_overlay_for_body(BoundingBox {
        left: 90.0,
        top: 100.0,
        right: 330.0,
        bottom: 112.0,
    });
    let viewport = BoundingBox {
        left: 0.0,
        top: 0.0,
        right: 595.0,
        bottom: 842.0,
    };

    let entries = build_effective_vector_render_plan(&model, None, &viewport, &[overlay]);

    assert!(
        entries.iter().all(|entry| !matches!(
            entry,
            EffectiveVectorRenderEntry::Object {
                object_index: 0,
                ..
            }
        )),
        "active editor must remove stroked horizontal source paths that cross the editable body"
    );
    assert!(entries
        .iter()
        .any(|entry| matches!(entry, EffectiveVectorRenderEntry::ParagraphOverlay(_))));
}

#[test]
fn keeps_section_divider() {
    let model = VectorPageModel {
        width: 595.0,
        height: 842.0,
        objects: vec![horizontal_stroked_path("section-divider", 118.0)],
        ..Default::default()
    };
    let overlay = active_overlay_for_body(BoundingBox {
        left: 90.0,
        top: 100.0,
        right: 330.0,
        bottom: 112.0,
    });
    let viewport = BoundingBox {
        left: 0.0,
        top: 0.0,
        right: 595.0,
        bottom: 842.0,
    };

    let entries = build_effective_vector_render_plan(&model, None, &viewport, &[overlay]);

    assert!(
        entries.iter().any(|entry| matches!(
            entry,
            EffectiveVectorRenderEntry::Object {
                object_index: 0,
                ..
            }
        )),
        "decorative divider paths outside the editable row must remain on the page canvas"
    );
}

#[test]
fn keeps_nearby_divider() {
    let model = VectorPageModel {
        width: 595.0,
        height: 842.0,
        objects: vec![horizontal_stroked_path("near-divider", 116.0)],
        ..Default::default()
    };
    let overlay = active_overlay_for_body(BoundingBox {
        left: 90.0,
        top: 100.0,
        right: 330.0,
        bottom: 112.0,
    });
    let viewport = BoundingBox {
        left: 0.0,
        top: 0.0,
        right: 595.0,
        bottom: 842.0,
    };

    let entries = build_effective_vector_render_plan(&model, None, &viewport, &[overlay]);

    assert!(
        entries.iter().any(|entry| matches!(
            entry,
            EffectiveVectorRenderEntry::Object { object_index: 0, .. }
        )),
        "decorative divider paths below the editable row must not be reclassified as editable-row decoration"
    );
}

#[test]
fn suppresses_descender_path() {
    let model = VectorPageModel {
        width: 595.0,
        height: 842.0,
        objects: vec![horizontal_stroked_path("descender-blue-row-bar", 113.8)],
        ..Default::default()
    };
    let overlay = active_overlay_for_body(BoundingBox {
        left: 90.0,
        top: 100.0,
        right: 330.0,
        bottom: 112.0,
    });
    let viewport = BoundingBox {
        left: 0.0,
        top: 0.0,
        right: 595.0,
        bottom: 842.0,
    };

    let entries = build_effective_vector_render_plan(&model, None, &viewport, &[overlay]);

    assert!(
        entries.iter().all(|entry| !matches!(
            entry,
            EffectiveVectorRenderEntry::Object { object_index: 0, .. }
        )),
        "row-level source decorations that overlap the glyph descender band must be removed with the edited text"
    );
}

#[test]
fn suppresses_text_without_ids() {
    let model = VectorPageModel {
        width: 595.0,
        height: 842.0,
        objects: vec![text_object_without_run_ids("text-object-1")],
        ..Default::default()
    };
    let overlay = active_overlay_for_source_object("text-object-1");
    let viewport = BoundingBox {
        left: 0.0,
        top: 0.0,
        right: 595.0,
        bottom: 842.0,
    };

    let entries = build_effective_vector_render_plan(&model, None, &viewport, &[overlay]);

    assert!(
        entries.iter().all(|entry| !matches!(
            entry,
            EffectiveVectorRenderEntry::Object {
                object_index: 0,
                ..
            }
        )),
        "active editor must suppress the source text object when run-level ids are unavailable"
    );
    assert!(entries
        .iter()
        .any(|entry| matches!(entry, EffectiveVectorRenderEntry::ParagraphOverlay(_))));
}

#[test]
fn spatially_suppresses_text() {
    let model = VectorPageModel {
        width: 595.0,
        height: 842.0,
        objects: vec![text_object_without_run_ids("unmatched-text-object")],
        ..Default::default()
    };
    let overlay = active_overlay_for_body(BoundingBox {
        left: 90.0,
        top: 100.0,
        right: 330.0,
        bottom: 112.0,
    });
    let viewport = BoundingBox {
        left: 0.0,
        top: 0.0,
        right: 595.0,
        bottom: 842.0,
    };

    let entries = build_effective_vector_render_plan(&model, None, &viewport, &[overlay]);

    assert!(
        entries.iter().all(|entry| !matches!(
            entry,
            EffectiveVectorRenderEntry::Object { object_index: 0, .. }
        )),
        "changed edit mode must still remove source text when PDF extraction cannot provide stable source object ids"
    );
}

#[test]
fn keeps_matching_text() {
    let model = VectorPageModel {
        width: 595.0,
        height: 842.0,
        objects: vec![text_object_without_run_ids("unmatched-text-object")],
        ..Default::default()
    };
    let mut overlay = active_overlay_for_body(BoundingBox {
        left: 90.0,
        top: 100.0,
        right: 330.0,
        bottom: 112.0,
    });
    overlay.replaces_source = false;
    let viewport = BoundingBox {
        left: 0.0,
        top: 0.0,
        right: 595.0,
        bottom: 842.0,
    };

    let entries = build_effective_vector_render_plan(&model, None, &viewport, &[overlay]);

    assert!(
        entries.iter().any(|entry| matches!(
            entry,
            EffectiveVectorRenderEntry::Object {
                object_index: 0,
                ..
            }
        )),
        "clean caret-only edit mode must not spatially suppress source text"
    );
}

#[test]
fn keeps_source_text() {
    let model = VectorPageModel {
        width: 595.0,
        height: 842.0,
        objects: vec![text_object_without_run_ids("text-object-1")],
        ..Default::default()
    };
    let mut overlay = active_overlay_for_source_object("text-object-1");
    overlay.replaces_source = false;
    let viewport = BoundingBox {
        left: 0.0,
        top: 0.0,
        right: 595.0,
        bottom: 842.0,
    };

    let entries = build_effective_vector_render_plan(&model, None, &viewport, &[overlay]);

    assert!(
        entries.iter().any(|entry| matches!(
            entry,
            EffectiveVectorRenderEntry::Object {
                object_index: 0,
                ..
            }
        )),
        "clean edit mode must keep the original PDF text painter visible"
    );
}

#[test]
fn suppresses_path_only() {
    let model = VectorPageModel {
        width: 595.0,
        height: 842.0,
        objects: vec![
            horizontal_stroked_path("blue-row-bar", 106.0),
            text_object_without_run_ids("unmatched-text-object"),
        ],
        ..Default::default()
    };
    let mut overlay = active_overlay_for_body(BoundingBox {
        left: 90.0,
        top: 100.0,
        right: 330.0,
        bottom: 112.0,
    });
    overlay.replaces_source = false;
    let viewport = BoundingBox {
        left: 0.0,
        top: 0.0,
        right: 595.0,
        bottom: 842.0,
    };

    let entries = build_effective_vector_render_plan(&model, None, &viewport, &[overlay]);

    assert!(
        entries.iter().all(|entry| !matches!(
            entry,
            EffectiveVectorRenderEntry::Object {
                object_index: 0,
                ..
            }
        )),
        "clean edit mode must still remove source-row blue path artifacts"
    );
    assert!(
        entries.iter().any(|entry| matches!(
            entry,
            EffectiveVectorRenderEntry::Object {
                object_index: 1,
                ..
            }
        )),
        "clean edit mode must keep the original PDF text painter visible"
    );
}

#[test]
fn spatially_suppresses_glyphs() {
    let plan = glyph_plan_without_run_ids();
    let overlay = active_overlay_for_body(BoundingBox {
        left: 90.0,
        top: 100.0,
        right: 330.0,
        bottom: 112.0,
    });
    let viewport = BoundingBox {
        left: 0.0,
        top: 0.0,
        right: 595.0,
        bottom: 842.0,
    };

    let entries = build_effective_glyph_render_plan(&plan, &viewport, &[overlay]);

    let paragraph = entries
        .iter()
        .find_map(|entry| match entry {
            EffectiveGlyphRenderEntry::Paragraph(reference) => Some(reference),
            _ => None,
        })
        .expect("glyph paragraph should remain in the render plan");
    assert!(
        paragraph.suppressed_run_indices.contains(&0),
        "changed edit mode must spatially suppress source glyph runs when source ids are missing"
    );
    assert!(entries
        .iter()
        .any(|entry| matches!(entry, EffectiveGlyphRenderEntry::ParagraphOverlay(_))));
}

#[test]
fn keeps_matching_glyphs() {
    let plan = glyph_plan_without_run_ids();
    let mut overlay = active_overlay_for_body(BoundingBox {
        left: 90.0,
        top: 100.0,
        right: 330.0,
        bottom: 112.0,
    });
    overlay.replaces_source = false;
    let viewport = BoundingBox {
        left: 0.0,
        top: 0.0,
        right: 595.0,
        bottom: 842.0,
    };

    let entries = build_effective_glyph_render_plan(&plan, &viewport, &[overlay]);

    let paragraph = entries
        .iter()
        .find_map(|entry| match entry {
            EffectiveGlyphRenderEntry::Paragraph(reference) => Some(reference),
            _ => None,
        })
        .expect("glyph paragraph should remain in the render plan");
    assert!(
        paragraph.suppressed_run_indices.is_empty(),
        "clean caret-only edit mode must not spatially suppress source glyph text"
    );
}

#[test]
fn overlay_suppresses_glyphs() {
    let plan = glyph_plan_without_run_ids();
    let mut overlay = active_overlay_for_body(BoundingBox {
        left: 90.0,
        top: 100.0,
        right: 330.0,
        bottom: 112.0,
    });
    overlay.owner = ParagraphRenderOverlayOwner::PersistedPageCanvas;
    let viewport = BoundingBox {
        left: 0.0,
        top: 0.0,
        right: 595.0,
        bottom: 842.0,
    };

    let entries = build_effective_glyph_render_plan(&plan, &viewport, &[overlay]);

    let paragraph = entries
        .iter()
        .find_map(|entry| match entry {
            EffectiveGlyphRenderEntry::Paragraph(reference) => Some(reference),
            _ => None,
        })
        .expect("glyph paragraph should remain in the render plan");
    assert!(
        paragraph.suppressed_run_indices.contains(&0),
        "committed replacement must spatially suppress fallback glyph text when source ids are missing"
    );
    assert!(
        matches!(
            entries.last(),
            Some(EffectiveGlyphRenderEntry::ParagraphOverlay(_))
        ),
        "persisted replacement overlay must render after the fallback glyph paragraph"
    );
}

#[test]
fn overlay_renders_last() {
    let model = VectorPageModel {
        width: 595.0,
        height: 842.0,
        objects: vec![
            text_object_without_run_ids("text-object-1"),
            horizontal_stroked_path("later-blue-path", 106.0),
        ],
        ..Default::default()
    };
    let overlay = persisted_overlay_for_source_object("text-object-1");
    let viewport = BoundingBox {
        left: 0.0,
        top: 0.0,
        right: 595.0,
        bottom: 842.0,
    };

    let entries = build_effective_vector_render_plan(&model, None, &viewport, &[overlay]);

    assert!(
        matches!(entries.last(), Some(EffectiveVectorRenderEntry::ParagraphOverlay(_))),
        "persisted replacement text must be painted after later PDF paths so paths cannot cover edited text after leaving edit mode"
    );
}

#[test]
fn overlay_suppresses_path() {
    let model = VectorPageModel {
        width: 595.0,
        height: 842.0,
        objects: vec![
            text_object_without_run_ids("text-object-1"),
            horizontal_stroked_path("committed-blue-row-bar", 106.0),
        ],
        ..Default::default()
    };
    let overlay = persisted_overlay_for_source_object("text-object-1");
    let viewport = BoundingBox {
        left: 0.0,
        top: 0.0,
        right: 595.0,
        bottom: 842.0,
    };

    let entries = build_effective_vector_render_plan(&model, None, &viewport, &[overlay]);

    assert!(
        entries.iter().all(|entry| !matches!(
            entry,
            EffectiveVectorRenderEntry::Object { object_index: 1, .. }
        )),
        "committed paragraph replacement must remove row-level PDF paths, not only active editor paths"
    );
}

#[test]
fn keeps_right_tile_suppressed() {
    let model = VectorPageModel {
        width: 595.0,
        height: 842.0,
        objects: vec![horizontal_stroked_path_between(
            "right-tile-blue-row-bar",
            430.0,
            560.0,
            106.0,
        )],
        ..Default::default()
    };
    let overlay = persisted_overlay_for_source_object("text-object-1");
    let right_tile_viewport = BoundingBox {
        left: 400.0,
        top: 80.0,
        right: 595.0,
        bottom: 140.0,
    };

    let entries =
        build_effective_vector_render_plan(&model, None, &right_tile_viewport, &[overlay]);

    assert!(
        entries.iter().all(|entry| !matches!(
            entry,
            EffectiveVectorRenderEntry::Object { object_index: 0, .. }
        )),
        "replacement effect region must cover row-level path suppression even when the viewport tile is outside the editor shell"
    );
    assert!(entries
        .iter()
        .any(|entry| matches!(entry, EffectiveVectorRenderEntry::ParagraphOverlay(_))));
}

/// 关键回归测试：当 PDF 的 list-item 把 marker (●) 和 body 放在同一个文本对象里时，
/// 编辑后 marker run 必须被保留（不能被 spatial suppress 干掉）。
#[test]
fn keeps_list_marker() {
    // 模拟真实 PDF：单个文本对象，runs[0] = "●", runs[1..] = body 字符
    let body_left = 90.0;
    let body_right = 330.0;
    let body_top = 100.0;
    let body_bottom = 112.0;
    let marker_x = 70.0; // marker 在 body 左侧 20px

    let mut runs = vec![StyledRun {
        text: "●".to_string(),
        tx: marker_x,
        ty: body_bottom,
        width: 10.0,
        font_size: 12.0,
        object_id: None,
        ..Default::default()
    }];
    // 模拟 body 的若干 run（每个字符一个）
    let body_chars = ["编", "程", "语", "言", ":", "R", "u", "s", "t"];
    let mut x = body_left;
    for ch in body_chars {
        runs.push(StyledRun {
            text: ch.to_string(),
            tx: x,
            ty: body_bottom,
            width: 12.0,
            font_size: 12.0,
            object_id: None,
            ..Default::default()
        });
        x += 12.0;
    }
    let total_run_count = runs.len();

    let model = VectorPageModel {
        width: 595.0,
        height: 842.0,
        objects: vec![VectorRenderObject::Text(VectorTextObject {
            id: "text-with-marker".to_string(),
            runs,
            ..Default::default()
        })],
        ..Default::default()
    };
    let overlay = persisted_overlay_for_source_object("text-with-marker");
    let viewport = BoundingBox {
        left: 0.0,
        top: 0.0,
        right: 595.0,
        bottom: 842.0,
    };
    let _ = (body_left, body_right, body_top); // silence unused

    let entries = build_effective_vector_render_plan(&model, None, &viewport, &[overlay]);

    // 整对象不应该被 skip — 应该有一个 Object entry
    let obj_entry = entries.iter().find_map(|e| {
        if let EffectiveVectorRenderEntry::Object {
            object_index,
            suppressed_text_runs,
        } = e
        {
            if *object_index == 0 {
                Some(suppressed_text_runs)
            } else {
                None
            }
        } else {
            None
        }
    });
    let suppressed = obj_entry
        .expect("marker text object must remain in render plan (entire object got suppressed!)");

    // marker run (index 0) 不能被 suppress
    let marker_run = match &model.objects[0] {
        VectorRenderObject::Text(t) => &t.runs[0],
        _ => unreachable!(),
    };
    assert!(
        !suppressed.suppresses_run(0, marker_run),
        "marker run (●) must NOT be suppressed; suppressed_runs={:?}",
        suppressed
    );

    // body runs 应该被 suppress
    let body_run_1 = match &model.objects[0] {
        VectorRenderObject::Text(t) => &t.runs[1],
        _ => unreachable!(),
    };
    assert!(
        suppressed.suppresses_run(1, body_run_1),
        "body run must be suppressed"
    );

    // 不能全部 run 都被 suppress（否则整对象会被 should_skip_entire_object 干掉）
    let suppressed_count = (0..total_run_count)
        .filter(|i| {
            let run = match &model.objects[0] {
                VectorRenderObject::Text(t) => &t.runs[*i],
                _ => unreachable!(),
            };
            suppressed.suppresses_run(*i, run)
        })
        .count();
    assert!(
        suppressed_count < total_run_count,
        "not all runs should be suppressed; suppressed {}/{}",
        suppressed_count,
        total_run_count
    );
}

/// 回归测试：当文本对象前有非文本对象（path/image）时，
/// z_index 和数组位置不同，suppression 必须仍然生效。
/// 这是 z_index vs array-position mismatch bug 的精确回归保护。
#[test]
fn handles_z_index_order() {
    // objects[0] = Path (z_index=0)
    // objects[1] = Text (z_index=5)  ← array pos 1, z_index 5
    let model = VectorPageModel {
        width: 595.0,
        height: 842.0,
        objects: vec![
            VectorRenderObject::Path(VectorPathObject::default()),
            VectorRenderObject::Text(VectorTextObject {
                id: "text-z5".to_string(),
                z_index: 5,
                runs: vec![StyledRun {
                    text: "Hello world".to_string(),
                    tx: 90.0,
                    ty: 112.0,
                    width: 200.0,
                    font_size: 12.0,
                    ..Default::default()
                }],
            }),
        ],
        ..Default::default()
    };
    // overlay has object_indices = {5} (the z_index, NOT the array position 1)
    let mut overlay = active_overlay_for_body(BoundingBox {
        left: 90.0,
        top: 100.0,
        right: 330.0,
        bottom: 112.0,
    });
    overlay.source_object_indices = vec![5];
    overlay.target.scene.body_session.paragraph.runs = vec![LayoutRun {
        id: "run-0".to_string(),
        text: "Hello world".to_string(),
        object_ids: vec!["text-z5".to_string()],
        object_indices: vec![5],
        bbox: BoundingBox {
            left: 90.0,
            top: 100.0,
            right: 290.0,
            bottom: 112.0,
        },
        ..Default::default()
    }];

    let viewport = BoundingBox {
        left: 0.0,
        top: 0.0,
        right: 595.0,
        bottom: 842.0,
    };
    let entries = build_effective_vector_render_plan(&model, None, &viewport, &[overlay]);

    // The text object (array pos 1, z_index 5) must be suppressed
    let has_unsuppressed_text = entries.iter().any(|e| {
        matches!(
            e,
            EffectiveVectorRenderEntry::Object { object_index: 1, suppressed_text_runs }
            if suppressed_text_runs.run_indices.is_empty()
        )
    });
    assert!(
        !has_unsuppressed_text,
        "text object at array position 1 / z_index 5 must be suppressed; \
         entries: {:?}",
        entries
            .iter()
            .map(|e| match e {
                EffectiveVectorRenderEntry::Object {
                    object_index,
                    suppressed_text_runs,
                } => format!(
                    "Object(idx={}, suppressed_runs={:?})",
                    object_index, suppressed_text_runs.run_indices
                ),
                EffectiveVectorRenderEntry::ParagraphOverlay(_) => "ParagraphOverlay".to_string(),
            })
            .collect::<Vec<_>>()
    );
    // overlay must have been inserted
    assert!(
        entries
            .iter()
            .any(|e| matches!(e, EffectiveVectorRenderEntry::ParagraphOverlay(_))),
        "overlay must be inserted"
    );
}
