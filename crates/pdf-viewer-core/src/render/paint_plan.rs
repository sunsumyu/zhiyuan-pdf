use crate::typography::font_resolver::resolve_font_face;
use crate::models::{
    BoundingBox, EditorControlStyle, EditorSession, FieldEditorParamsRequest, FieldEditorParams,
    FontHints, GlyphPaintParagraph, GlyphPaintPlan, GlyphPaintRegion, GlyphPaintRun, LayoutInferenceResult,
    LayoutParagraph, LayoutRun, PaintMode, ResolvedFontFace,
};

fn paint_mode_from_render_mode(render_mode: i64) -> PaintMode {
    match render_mode {
        1 => PaintMode::Stroke,
        2 => PaintMode::FillStroke,
        _ => PaintMode::Fill,
    }
}

fn resolve_run_font(run: &LayoutRun) -> ResolvedFontFace {
    let hints = FontHints {
        flags: 0,
        weight: if run.style.is_bold { 700 } else { 400 },
        italic_angle: if run.style.is_italic { -12.0 } else { 0.0 },
        ascent: run.style.font_size,
        descent: 0.0,
        cap_height: 0.0,
        x_height: 0.0,
        is_fixed_pitch: false,
        is_serif: false,
        is_italic: run.style.is_italic,
        is_bold: run.style.is_bold,
    };
    resolve_font_face(&run.style.font_name, Some(&hints))
}

fn build_paint_run(
    page_index: u16,
    region_id: &str,
    paragraph_id: &str,
    run: &LayoutRun,
) -> GlyphPaintRun {
    GlyphPaintRun {
        id: run.id.clone(),
        page_index,
        region_id: region_id.to_string(),
        paragraph_id: paragraph_id.to_string(),
        text: run.text.clone(),
        bbox: run.bbox,
        origin_x: run.origin_x,
        origin_y: run.origin_y,
        char_origins: run.char_origins.clone(),
        color: run.style.color.clone(),
        resolved_font: resolve_run_font(run),
        font_size: run.style.font_size,
        scale_x: run.style.scale_x,
        is_bold: run.style.is_bold,
        is_italic: run.style.is_italic,
        is_underline: run.style.is_underline,
        paint_mode: paint_mode_from_render_mode(0),
        object_ids: run.object_ids.clone(),
        object_indices: run.object_indices.clone(),
    }
}

fn build_editor_session(paragraph: &crate::models::LayoutParagraph) -> EditorSession {
    let mut normalized_paragraph = paragraph.clone();
    normalized_paragraph.runs = paragraph
        .runs
        .iter()
        .map(|run| {
            let mut normalized_run = run.clone();
            normalized_run.style.font_name = resolve_run_font(run).render_family;
            normalized_run
        })
        .collect();

    let anchor_bbox = paragraph
        .runs
        .iter()
        .fold(paragraph.runs.first().map(|run| run.bbox).unwrap_or_default(), |acc, run| {
            crate::models::BoundingBox {
                left: acc.left.min(run.bbox.left),
                top: acc.top.min(run.bbox.top),
                right: acc.right.max(run.bbox.right),
                bottom: acc.bottom.max(run.bbox.bottom),
            }
        });
    EditorSession {
        anchor_bbox,
        paragraph: normalized_paragraph,
    }
}

fn is_decorative_text(text: &str) -> bool {
    let trimmed = text.trim();
    !trimmed.is_empty()
        && trimmed
            .chars()
            .all(|ch| matches!(ch, '•' | '●' | '▪' | '◦' | '·' | '○' | '-' | '▶' | '➤'))
}

fn build_control_style(paragraph: &crate::models::LayoutParagraph) -> EditorControlStyle {
    let preferred_run = paragraph
        .runs
        .iter()
        .find(|run| {
            let resolved = resolve_run_font(run);
            !is_decorative_text(&run.text)
                && resolved.identity.symbol_class == crate::models::SymbolClass::None
        })
        .or_else(|| paragraph.runs.iter().find(|run| !is_decorative_text(&run.text)))
        .or_else(|| paragraph.runs.first());

    if let Some(run) = preferred_run {
        let resolved = resolve_run_font(run);
        return EditorControlStyle {
            font_family: resolved.render_family,
            font_size: run.style.font_size,
            font_weight: if run.style.is_bold { "bold".to_string() } else { "normal".to_string() },
            font_style: if run.style.is_italic { "italic".to_string() } else { "normal".to_string() },
            color: run.style.color.clone(),
            text_decoration: if run.style.is_underline { "underline".to_string() } else { "none".to_string() },
        };
    }

    EditorControlStyle::default()
}

pub fn build_field_editor_params(request: &FieldEditorParamsRequest) -> FieldEditorParams {
    if request.runs.is_empty() {
        return FieldEditorParams::default();
    }

    let paragraph = LayoutParagraph {
        id: request.paragraph_id.clone(),
        bbox: request.anchor_bbox,
        style: crate::models::ParagraphStyle {
            align: crate::models::LayoutAlignment::Left,
            line_height: request.line_height.max(1.0),
            first_line_indent: 0.0,
            left_indent: 0.0,
            tab_stops: vec![],
        },
        runs: request
            .runs
            .iter()
            .map(|run| LayoutRun {
                id: run.id.clone(),
                text: run.text.clone(),
                style: crate::models::RunStyle {
                    font_name: run
                        .resolved_font
                        .render_family
                        .clone(),
                    font_size: run.font_size,
                    color: run.color.clone(),
                    is_bold: run.is_bold,
                    is_italic: run.is_italic,
                    is_underline: run.is_underline,
                    char_spacing: 0.0,
                    scale_x: run.scale_x.max(0.01),
                },
                bbox: BoundingBox {
                    left: run.bbox.left,
                    top: run.bbox.top,
                    right: run.bbox.right,
                    bottom: run.bbox.bottom,
                },
                origin_x: run.origin_x,
                origin_y: run.origin_y,
                char_origins: run.char_origins.clone(),
                char_widths: vec![],
                object_ids: run.object_ids.clone(),
                object_indices: run.object_indices.clone(),
            })
            .collect(),
        object_ids: request
            .runs
            .iter()
            .flat_map(|run| run.object_ids.clone())
            .collect(),
        origin_x: request.anchor_bbox.left,
        origin_y: request.anchor_bbox.top,
        wrap_width: (request.anchor_bbox.right - request.anchor_bbox.left).max(1.0),
    };

    let session = EditorSession {
        anchor_bbox: request.anchor_bbox,
        paragraph: paragraph.clone(),
    };

    FieldEditorParams {
        session: Some(session),
        control_style: Some(build_control_style(&paragraph)),
    }
}

pub fn build_glyph_paint_plan(layout: &LayoutInferenceResult) -> GlyphPaintPlan {
    let regions = layout
        .regions
        .iter()
        .map(|region| {
            let paragraphs = region
                .paragraphs
                .iter()
                .map(|paragraph| {
                    let bbox = paragraph.runs.iter().fold(paragraph.runs.first().map(|run| run.bbox).unwrap_or_default(), |acc, run| {
                        crate::models::BoundingBox {
                            left: acc.left.min(run.bbox.left),
                            top: acc.top.min(run.bbox.top),
                            right: acc.right.max(run.bbox.right),
                            bottom: acc.bottom.max(run.bbox.bottom),
                        }
                    });

                    GlyphPaintParagraph {
                        id: paragraph.id.clone(),
                        region_id: region.id.clone(),
                        bbox,
                        style: paragraph.style.clone(),
                        editor_session: build_editor_session(paragraph),
                        control_style: build_control_style(paragraph),
                        semantic_role: region.semantic_role.clone(),
                        runs: paragraph
                            .runs
                            .iter()
                            .map(|run| build_paint_run(layout.page_index, &region.id, &paragraph.id, run))
                            .collect(),
                    }
                })
                .collect();

            GlyphPaintRegion {
                id: region.id.clone(),
                kind: region.kind,
                layout_mode: region.layout_mode,
                bbox: region.bbox,
                paragraphs,
                object_ids: region.object_ids.clone(),
            }
        })
        .collect();

    GlyphPaintPlan {
        page_index: layout.page_index,
        width: layout.width,
        height: layout.height,
        regions,
        external_objects: vec![],
    }
}
