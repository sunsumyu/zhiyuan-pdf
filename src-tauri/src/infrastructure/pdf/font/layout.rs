use super::parse::ParsedFont;

pub fn break_text_into_lines(
    text: &str,
    runs: Option<&Vec<pdf_viewer_core::models::LayoutRun>>,
    font: &ParsedFont,
    font_size: f32,
    max_width: f32,
    align: Option<pdf_viewer_core::models::LayoutAlignment>,
    line_height: Option<f32>,
    char_spacing: f32,
    scale_x: f32,
) -> pdf_viewer_core::geometry::layout_engine::ParagraphLayout {
    use pdf_viewer_core::geometry::layout_engine::layout_paragraph;
    use pdf_viewer_core::models::{
        LayoutAlignment, LayoutParagraph, LayoutRun, ParagraphStyle, RunStyle,
    };

    let layout_runs = if let Some(r) = runs {
        r.clone()
    } else {
        vec![LayoutRun {
            id: "patch-run-0".into(),
            text: text.to_string(),
            style: RunStyle {
                font_size,
                char_spacing,
                scale_x,
                ..Default::default()
            },
            ..Default::default()
        }]
    };

    let paragraph = LayoutParagraph {
        id: "patch-para-0".into(),
        runs: layout_runs,
        style: ParagraphStyle {
            align: align.unwrap_or(LayoutAlignment::Left),
            line_height: line_height.unwrap_or(1.2).max(0.8),
            ..Default::default()
        },
        ..Default::default()
    };

    layout_paragraph(&paragraph, max_width, |run_text, _| {
        font.resolve_text_width(run_text, font_size, char_spacing, scale_x)
    })
}
