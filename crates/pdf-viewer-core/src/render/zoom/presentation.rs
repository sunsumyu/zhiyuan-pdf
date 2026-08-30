//! Zoom presentation state machine (ADR-0002).
//!
//! Single logical writer for all zoom-gesture geometry. Produces `SurfaceOp`s
//! that the ui-crate adapter executes via web-sys. Pure logic — no DOM here —
//! so the invariants are testable natively:
//!
//! - I1 visual-size continuity: `layout_width × css_scale == page × visual_zoom`
//!   for ANY `layout_zoom` — a committed frame can never change visual size.
//! - I2 anchor continuity: the anchor page point stays exactly under the
//!   cursor viewport position across ticks and committed frames.
//! - I3 single active surface: a gesture always ends up with exactly one
//!   visible surface (the vector container); the raster sibling is hidden.
//! - I4 settle: transform cleared, geometry ownership handed back.

use serde::{Deserialize, Serialize};

use crate::render::zoom::decision::resolve_css_transform_string;

/// DOM surfaces the presenter may drive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SurfaceId {
    /// Vector page container (`pdf-page-container`) — the only active surface
    /// during a gesture.
    VectorContainer,
    /// Raster/preview surface (`pdf-render-target`) — a `width:100%` sibling
    /// of the vector container inside the wrapper. Never transformed: it is
    /// hidden at gesture start (I3).
    RasterTarget,
}

/// The un-transformed layout box the active surface currently represents:
/// the page rendered at `layout_zoom`, placed at (left, top) in wrapper
/// coordinates. Visual size = width × css_scale where css_scale is applied
/// on top by the presenter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceLayout {
    pub layout_zoom: f32,
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
}

/// Geometry writes — the ONLY vocabulary the presenter speaks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SurfaceOp {
    SetBox {
        surface: SurfaceId,
        left: f32,
        top: f32,
        width: f32,
        height: f32,
    },
    SetTransform {
        surface: SurfaceId,
        transform: String,
    },
    SetDisplay {
        surface: SurfaceId,
        display: String,
    },
}

/// A committed frame's layout (from the render pipeline's frame plan).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CommittedLayout {
    pub display_zoom: f32,
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
    pub scroll_left: f32,
    pub scroll_top: f32,
}

/// The cursor-anchored transform for the active surface layout.
///
/// Derivation: the anchor page point A sits at local position `A × layout_zoom`
/// inside the surface. After `translate(tx) scale(s)` (origin 0 0) its viewport
/// position is `left − scroll + tx + A × layout_zoom × s`. Setting that equal
/// to the cursor and noting `layout_zoom × s == visual_zoom` gives:
///
/// ```text
/// tx = cursor − left + scroll − anchor_page × visual_zoom
/// ```
///
/// independent of layout_zoom — which is exactly why I1/I2 hold structurally.
///
/// ADR-0004 (revised): `s` MUST interpolate visual_zoom / layout_zoom. It is
/// not the source of blur — it is the bridge between the bitmap's coordinate
/// space (layout_zoom) and the visual space (visual_zoom). The render pipeline
/// renders at visual_zoom (resolve_preview_render_zoom), so at every commit
/// layout_zoom == visual_zoom and s returns to 1.0 — zero stretch at rest.
/// Forcing s = 1.0 breaks invariant I1: the translate term assumes content at
/// visual_zoom while the bitmap stays at layout_zoom, so content flies and
/// sizes jump between commits (observed as "缩放乱跳" with double surfaces).
pub fn anchored_translate(
    layout: &SurfaceLayout,
    anchor_page: (f32, f32),
    visual_zoom: f32,
    cursor: (f32, f32),
    scroll: (f32, f32),
) -> (f32, f32, f32) {
    let s = if layout.layout_zoom.is_finite() && layout.layout_zoom > 0.0 {
        visual_zoom / layout.layout_zoom
    } else {
        1.0
    };
    let tx = cursor.0 - layout.left + scroll.0 - anchor_page.0 * visual_zoom;
    let ty = cursor.1 - layout.top + scroll.1 - anchor_page.1 * visual_zoom;
    (tx, ty, s)
}

/// Transform op for the active surface.
pub fn transform_op(
    layout: &SurfaceLayout,
    anchor_page: (f32, f32),
    visual_zoom: f32,
    cursor: (f32, f32),
    scroll: (f32, f32),
) -> SurfaceOp {
    let (tx, ty, s) = anchored_translate(layout, anchor_page, visual_zoom, cursor, scroll);
    SurfaceOp::SetTransform {
        surface: SurfaceId::VectorContainer,
        transform: resolve_css_transform_string(tx, ty, s),
    }
}

/// Gesture start (I3): exactly one visible surface. The raster sibling is a
/// `width:100%` element that tracks the wrapper box; during the animation the
/// wrapper box is driven by committed frames while the container's visual
/// state is driven by transforms — the two can never stay in sync, so the
/// raster must simply go. The container keeps its last settled bitmap
/// (`display:none` does not clear a canvas), so the switch is seamless.
pub fn begin_gesture_ops(raster_visible: bool) -> Vec<SurfaceOp> {
    if !raster_visible {
        return Vec::new();
    }
    vec![
        SurfaceOp::SetDisplay {
            surface: SurfaceId::RasterTarget,
            display: "none".into(),
        },
        SurfaceOp::SetDisplay {
            surface: SurfaceId::VectorContainer,
            display: "block".into(),
        },
    ]
}

/// Apply a committed frame: the active surface takes the new layout box, then
/// the re-derived cursor-anchored transform (I1/I2). Returns the ops plus the
/// new layout for bookkeeping (`last_rendered_zoom` tracking).
pub fn committed_frame_ops(
    frame: &CommittedLayout,
    anchor_page: (f32, f32),
    visual_zoom: f32,
    cursor: (f32, f32),
) -> (Vec<SurfaceOp>, SurfaceLayout) {
    let new_layout = SurfaceLayout {
        layout_zoom: frame.display_zoom,
        left: frame.left,
        top: frame.top,
        width: frame.width,
        height: frame.height,
    };
    let ops = vec![
        SurfaceOp::SetBox {
            surface: SurfaceId::VectorContainer,
            left: frame.left,
            top: frame.top,
            width: frame.width,
            height: frame.height,
        },
        transform_op(&new_layout, anchor_page, visual_zoom, cursor, (frame.scroll_left, frame.scroll_top)),
    ];
    (ops, new_layout)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE_W: f32 = 595.3;
    const PAGE_H: f32 = 841.9;

    fn layout(zoom: f32) -> SurfaceLayout {
        SurfaceLayout {
            layout_zoom: zoom,
            left: 0.0,
            top: 0.0,
            width: PAGE_W * zoom,
            height: PAGE_H * zoom,
        }
    }

    /// I1: visual size must equal page × visual_zoom for ANY layout_zoom.
    /// ADR-0004 revised: s interpolates visual/layout so this holds at every
    /// point of the animation; at each commit layout_zoom == visual_zoom so
    /// s == 1.0 exactly (no stretch at rest — sharp settle).
    #[test]
    fn i1_visual_size_independent_of_layout_zoom() {
        for layout_zoom in [0.4_f32, 0.7, 1.0, 1.35, 2.0] {
            let l = layout(layout_zoom);
            let visual = 1.23_f32;
            let (_, _, s) = anchored_translate(&l, (0.0, 0.0), visual, (0.0, 0.0), (0.0, 0.0));
            let visual_w = l.width * s;
            let visual_h = l.height * s;
            assert!((visual_w - PAGE_W * visual).abs() < 0.001, "layout_zoom={layout_zoom}: {visual_w}");
            assert!((visual_h - PAGE_H * visual).abs() < 0.001, "layout_zoom={layout_zoom}: {visual_h}");
        }
    }

    /// ADR-0004: when the render pipeline has committed at visual_zoom
    /// (layout_zoom == visual_zoom), s must be exactly 1.0 — zero stretch.
    #[test]
    fn adr0004_no_stretch_when_layout_matches_visual() {
        let visual = 1.23_f32;
        let l = layout(visual);
        let (_, _, s) = anchored_translate(&l, (0.0, 0.0), visual, (0.0, 0.0), (0.0, 0.0));
        assert!((s - 1.0).abs() < 0.0001, "s must be 1.0 at committed zoom: {s}");
    }

    /// I1 across a committed-frame sequence that mimics the failing scenario:
    /// animation at visual=0.59 while commits land at display_zoom 0.2/0.45/1.0.
    #[test]
    fn i1_visual_size_continuous_across_committed_frames() {
        let visual = 0.5946_f32;
        let anchor = (300.0_f32, 400.0_f32);
        let cursor = (656.0_f32, 370.0_f32);
        let mut prev_w = PAGE_W * visual;
        for frame_zoom in [0.2_f32, 0.45, 1.0] {
            let frame = CommittedLayout {
                display_zoom: frame_zoom,
                left: (cursor.0 - anchor.0 * frame_zoom).max(0.0),
                top: (cursor.1 - anchor.1 * frame_zoom).max(0.0),
                width: PAGE_W * frame_zoom,
                height: PAGE_H * frame_zoom,
                scroll_left: 0.0,
                scroll_top: 0.0,
            };
            let (ops, new_layout) = committed_frame_ops(&frame, anchor, visual, cursor);
            assert!(ops.iter().any(|op| matches!(op, SurfaceOp::SetBox { .. })));
            let (_, _, s) = anchored_translate(&new_layout, anchor, visual, cursor, (0.0, 0.0));
            let visual_w = new_layout.width * s;
            assert!(
                (visual_w - prev_w).abs() < 0.001,
                "zoom={frame_zoom}: {visual_w} vs {prev_w}"
            );
            prev_w = visual_w;
        }
    }

    /// I2: the anchor page point must land exactly at the cursor, both from
    /// the transform formula and after a committed frame re-layout.
    #[test]
    fn i2_anchor_stays_under_cursor() {
        let anchor = (320.0_f32, 450.0_f32);
        let cursor = (500.0_f32, 380.0_f32);
        let scroll = (40.0_f32, 25.0_f32);
        let visual = 1.7_f32;
        for layout_zoom in [0.5_f32, 1.0, 1.7] {
            let mut l = layout(layout_zoom);
            l.left = (cursor.0 - anchor.0 * layout_zoom).max(0.0);
            l.top = (cursor.1 - anchor.1 * layout_zoom).max(0.0);
            let (tx, ty, s) = anchored_translate(&l, anchor, visual, cursor, scroll);
            let viewport_x = l.left - scroll.0 + tx + anchor.0 * layout_zoom * s;
            let viewport_y = l.top - scroll.1 + ty + anchor.1 * layout_zoom * s;
            assert!((viewport_x - cursor.0).abs() < 0.01, "x @ {layout_zoom}: {viewport_x}");
            assert!((viewport_y - cursor.1).abs() < 0.01, "y @ {layout_zoom}: {viewport_y}");
        }
    }

    /// I3: gesture start hides the raster and shows the container.
    #[test]
    fn i3_begin_gesture_switches_to_primary() {
        assert_eq!(
            begin_gesture_ops(true),
            vec![
                SurfaceOp::SetDisplay {
                    surface: SurfaceId::RasterTarget,
                    display: "none".into(),
                },
                SurfaceOp::SetDisplay {
                    surface: SurfaceId::VectorContainer,
                    display: "block".into(),
                },
            ]
        );
        // Raster already hidden → no ops.
        assert!(begin_gesture_ops(false).is_empty());
    }

    /// Committed frame produces exactly one box op and one transform op for
    /// the single active surface.
    #[test]
    fn committed_frame_targets_only_primary() {
        let frame = CommittedLayout {
            display_zoom: 0.8,
            left: 12.0,
            top: 34.0,
            width: PAGE_W * 0.8,
            height: PAGE_H * 0.8,
            scroll_left: 5.0,
            scroll_top: 6.0,
        };
        let (ops, _) = committed_frame_ops(&frame, (100.0, 200.0), 1.2, (300.0, 250.0));
        assert_eq!(ops.len(), 2);
        assert!(matches!(&ops[0], SurfaceOp::SetBox { surface: SurfaceId::VectorContainer, .. }));
        assert!(matches!(&ops[1], SurfaceOp::SetTransform { surface: SurfaceId::VectorContainer, transform } if !transform.is_empty()));
    }

    /// At settle (visual == layout zoom) with the anchor layout applied, the
    /// transform must be empty: the layout alone puts the anchor at the cursor.
    #[test]
    fn transform_empty_when_visual_matches_layout() {
        let anchor = (320.0_f32, 450.0_f32);
        let cursor = (500.0_f32, 380.0_f32);
        let zoom = 1.0_f32;
        // Anchor layout (same construction as compute_anchor_viewport_layout_result):
        let l = SurfaceLayout {
            layout_zoom: zoom,
            left: (cursor.0 - anchor.0 * zoom).max(0.0),
            top: (cursor.1 - anchor.1 * zoom).max(0.0),
            width: PAGE_W * zoom,
            height: PAGE_H * zoom,
        };
        let scroll = (
            (l.left + anchor.0 * zoom - cursor.0).max(0.0),
            (l.top + anchor.1 * zoom - cursor.1).max(0.0),
        );
        let (tx, ty, s) = anchored_translate(&l, anchor, zoom, cursor, scroll);
        assert!((s - 1.0).abs() < 0.001);
        assert!(tx.abs() < 0.01, "tx={tx}");
        assert!(ty.abs() < 0.01, "ty={ty}");
        let op = transform_op(&l, anchor, zoom, cursor, scroll);
        match op {
            SurfaceOp::SetTransform { transform, .. } => assert!(transform.is_empty()),
            _ => panic!("expected SetTransform"),
        }
    }
}
