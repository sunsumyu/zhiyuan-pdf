use crate::common::debug::truncate_debug_text;
use crate::editor::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use crate::editor::draft_layout::build_persisted_overlay_render_plan;
use crate::editor::paragraph_overlay::ParagraphRenderOverlay;
use crate::editor::replacement_region::build_region;
use crate::editor::session::ActiveEditorTarget;
use crate::editor::text_geometry::measure_text_width as measure_text_width_shared;
use crate::render::canvas::CanvasRenderer;
use pdf_viewer_core::models::{VectorPageModel, VisualMarkerContent};

pub(crate) fn path_bbox_summary(
    path: &pdf_viewer_core::models::VectorPathObject,
) -> Option<(f32, f32)> {
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;

    for segment in &path.segments {
        for [x, y] in &segment.points {
            min_x = min_x.min(*x);
            min_y = min_y.min(*y);
            max_x = max_x.max(*x);
            max_y = max_y.max(*y);
        }
    }

    if min_x.is_finite() && min_y.is_finite() && max_x.is_finite() && max_y.is_finite() {
        Some(((max_x - min_x).max(0.0), (max_y - min_y).max(0.0)))
    } else {
        None
    }
}

fn summarize_overlay_render_plan(
    plan: &crate::editor::draft_layout::EditorDraftRenderPlan,
) -> String {
    plan.layout
        .lines
        .iter()
        .take(4)
        .enumerate()
        .map(|(line_index, line)| {
            let runs = line
                .runs
                .iter()
                .take(8)
                .enumerate()
                .map(|(run_index, run)| {
                    let first_origin = run.char_origins.first().copied().unwrap_or(f32::NAN);
                    let last_origin = run.char_origins.last().copied().unwrap_or(f32::NAN);
                    format!(
                        "r{run_index}('{}' x={:.2} origins={} first={:.2} last={:.2})",
                        truncate_debug_text(&run.text, 18),
                        run.origin_x,
                        run.char_origins.len(),
                        first_origin,
                        last_origin,
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "line{line_index}(base={:.2}, off={:.2}, width={:.2}, text='{}', {runs})",
                line.baseline_y,
                line.offset_x,
                line.width,
                truncate_debug_text(&line.text, 40),
            )
        })
        .collect::<Vec<_>>()
        .join(" || ")
}

fn count_overlay_underline_runs(
    plan: &crate::editor::draft_layout::EditorDraftRenderPlan,
) -> usize {
    plan.layout
        .lines
        .iter()
        .flat_map(|line| line.runs.iter())
        .filter(|run| run.style.is_underline)
        .count()
}

fn draw_graphic_markers(
    renderer: &CanvasRenderer,
    document_plan: &pdf_viewer_core::edit::document_plan::EditContext,
    vector_model: Option<&VectorPageModel>,
    image_provider: &js_sys::Map,
) {
    let Some(vector_model) = vector_model else {
        return;
    };
    for marker in &document_plan.graphic_markers {
        let VisualMarkerContent::Graphic { object_index, .. } = &marker.content else {
            continue;
        };
        let Some(object) = vector_model.objects.get(*object_index) else {
            continue;
        };
        renderer.draw_vector_object(object, Some(*object_index), image_provider, None);
        dbg_event(
            "paint.overlay",
            "graphic-marker-redraw",
            vec![
                dbg_field("paragraphId", document_plan.target_id.as_str()),
                dbg_field("objectIndex", *object_index),
            ],
        );
    }
}

pub(crate) fn draw_active_editor_shell_overlay_page(
    renderer: &CanvasRenderer,
    overlay: &ParagraphRenderOverlay,
    vector_model: Option<&VectorPageModel>,
    image_provider: &js_sys::Map,
    marker_text_override: Option<&str>,
) {
    let active_target = &overlay.target;
    if overlay.replaces_source {
        draw_persisted_paragraph_overlay_page(
            renderer,
            active_target,
            &overlay.draft_text,
            vector_model,
            image_provider,
            marker_text_override,
            "active-editor-page-canvas",
        );
        return;
    }

    let shell_bbox = active_target.scene.shell_bbox;
    let replacement_region = build_region(active_target);
    let occlusion_bbox = replacement_region.text_clear_bbox;
    let shell_width = (shell_bbox.right - shell_bbox.left).max(1.0);
    let shell_height = (shell_bbox.bottom - shell_bbox.top).max(1.0);
    let occlusion_width = (occlusion_bbox.right - occlusion_bbox.left).max(1.0);
    let occlusion_height = (occlusion_bbox.bottom - occlusion_bbox.top).max(1.0);
    dbg_event(
        "paint.overlay",
        "active-shell-caret-only",
        vec![
            dbg_field("paragraphId", &active_target.paragraph_id),
            dbg_field(
                "shellBBox",
                format!(
                    "[{:.2},{:.2},{:.2},{:.2}]",
                    shell_bbox.left, shell_bbox.top, shell_bbox.right, shell_bbox.bottom
                ),
            ),
            dbg_field(
                "bodyBBox",
                format!(
                    "[{:.2},{:.2},{:.2},{:.2}]",
                    active_target.scene.body_session().anchor_bbox.left,
                    active_target.scene.body_session().anchor_bbox.top,
                    active_target.scene.body_session().anchor_bbox.right,
                    active_target.scene.body_session().anchor_bbox.bottom
                ),
            ),
            dbg_field(
                "occlusionBBox",
                format!(
                    "[{:.2},{:.2},{:.2},{:.2}]",
                    occlusion_bbox.left,
                    occlusion_bbox.top,
                    occlusion_bbox.right,
                    occlusion_bbox.bottom
                ),
            ),
            dbg_field("width", shell_width),
            dbg_field("height", shell_height),
            dbg_field("occlusionWidth", occlusion_width),
            dbg_field("occlusionHeight", occlusion_height),
            dbg_field("markerTextOverride", marker_text_override.unwrap_or("none")),
            dbg_field("fillsPageCanvas", false),
            dbg_field("redrawsMarker", false),
        ],
    );
    let _ = renderer;
}

pub(crate) fn draw_persisted_paragraph_overlay_page(
    renderer: &CanvasRenderer,
    active_target: &ActiveEditorTarget,
    draft_text: &str,
    vector_model: Option<&VectorPageModel>,
    image_provider: &js_sys::Map,
    marker_text_override: Option<&str>,
    owner_label: &str,
) {
    crate::chain_trace!(
        "render.draw-overlay",
        "owner" => owner_label,
        "paragraphId" => &active_target.paragraph_id,
        "draftLen" => draft_text.chars().count(),
        "markerOverride" => marker_text_override.unwrap_or("none"),
    );
    let shell_bbox = active_target.scene.shell_bbox;
    let shell_width = (shell_bbox.right - shell_bbox.left).max(1.0);
    let shell_height = (shell_bbox.bottom - shell_bbox.top).max(1.0);
    let replacement_region = build_region(active_target);
    let source_replacement_bbox = replacement_region.text_clear_bbox;
    let replacement_width = (source_replacement_bbox.right - source_replacement_bbox.left).max(1.0);
    let replacement_height =
        (source_replacement_bbox.bottom - source_replacement_bbox.top).max(1.0);
    dbg_event(
        "paint.overlay",
        "method.draw-editor-paragraph.enter",
        vec![
            dbg_field("paragraphId", &active_target.paragraph_id),
            dbg_field("markerTextOverride", marker_text_override.unwrap_or("none")),
            dbg_field("owner", owner_label),
        ],
    );
    // 用白色遮盖 replacement 区域，隐藏 PDF 中位于文字下方的背景路径对象
    // （如蓝色/彩色填充矩形），这些路径不会被 run 级 suppress 机制覆盖。
    renderer.ctx.set_fill_style_str("#ffffff");
    renderer.ctx.fill_rect(
        source_replacement_bbox.left as f64,
        source_replacement_bbox.top as f64,
        replacement_width as f64,
        replacement_height as f64,
    );
    dbg_event(
        "paint.overlay",
        "method.draw-editor-paragraph.shell-occlusion",
        vec![
            dbg_field("paragraphId", &active_target.paragraph_id),
            dbg_field(
                "shellBBox",
                format!(
                    "[{:.2},{:.2},{:.2},{:.2}]",
                    shell_bbox.left, shell_bbox.top, shell_bbox.right, shell_bbox.bottom
                ),
            ),
            dbg_field("width", shell_width),
            dbg_field("height", shell_height),
            dbg_field(
                "sourceReplacementBBox",
                format!(
                    "[{:.2},{:.2},{:.2},{:.2}]",
                    source_replacement_bbox.left,
                    source_replacement_bbox.top,
                    source_replacement_bbox.right,
                    source_replacement_bbox.bottom
                ),
            ),
            dbg_field("sourceReplacementWidth", replacement_width),
            dbg_field("sourceReplacementHeight", replacement_height),
        ],
    );
    let document_plan = &active_target.scene.document_plan;
    let session = &document_plan.body_session;
    // marker 不再单独渲染，文本 marker 已在 build_persisted_overlay_render_plan 中统一排版。
    // 图形 marker 由 source vector object 恢复，避免正文替换时被 row path suppression 误删。
    draw_graphic_markers(renderer, document_plan, vector_model, image_provider);

    let render_plan =
        build_persisted_overlay_render_plan(document_plan, draft_text, |text, run| {
            measure_text_width_shared(&renderer.ctx, text, run)
        });

    dbg_event(
        "paint.overlay",
        "render-plan",
        vec![
            dbg_field("paragraphId", &active_target.paragraph_id),
            dbg_field("draftText", draft_text),
            dbg_field("sourceText", document_plan.source_body_text()),
            dbg_field(
                "bodyAnchor",
                format!(
                    "[{:.2},{:.2},{:.2},{:.2}]",
                    session.anchor_bbox.left,
                    session.anchor_bbox.top,
                    session.anchor_bbox.right,
                    session.anchor_bbox.bottom
                ),
            ),
            dbg_field("lineCount", render_plan.layout.lines.len()),
            dbg_field(
                "underlineRunCount",
                count_overlay_underline_runs(&render_plan),
            ),
            dbg_field("lineSummary", summarize_overlay_render_plan(&render_plan)),
        ],
    );
    for (_line_idx, line) in render_plan.layout.lines.iter().enumerate() {
        let baseline_y = session.anchor_bbox.top + line.baseline_y;
        for (_run_idx, run) in line.runs.iter().enumerate() {
            let run_x = session.anchor_bbox.left + line.offset_x + run.origin_x;
            renderer.draw_text_run(
                &run.text,
                run_x,
                baseline_y,
                run.style.font_size,
                &run.style.color,
                &run.style.font_name,
                if run.style.is_bold { "bold" } else { "normal" },
                if run.style.is_italic {
                    "italic"
                } else {
                    "normal"
                },
                false,
                run.style.scale_x,
                0,
                if run.char_origins.is_empty() {
                    None
                } else {
                    Some(&run.char_origins)
                },
            );
        }
    }
}
