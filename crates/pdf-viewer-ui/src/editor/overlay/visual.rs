use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

use crate::render::canvas::CanvasRenderer;
use crate::editor::source_geometry::source_line_visual_bbox_for_caret;
use crate::editor::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use crate::editor::host_runtime::get_state as get_editor_host_state;
use crate::editor::mode::get_active_editor_state;
use crate::editor::text_geometry::active_caret_visual;
use crate::zoom::zoom_controller::get_zoom_state;

const EDITOR_Y_BUFFER: f32 = 1.0;
const CARET_WIDTH: f32 = 1.5;

fn sanitize_projection_zoom(value: f32, fallback: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else if fallback.is_finite() && fallback > 0.0 {
        fallback
    } else {
        1.0
    }
}

fn resolve_editor_projection_zoom(requested_display_zoom: f32) -> f32 {
    let runtime_zoom = get_editor_host_state().last_display_zoom;
    let zoom_state = get_zoom_state();
    if zoom_state.preview_host.preview_active
        || (zoom_state.target_zoom - zoom_state.visual_zoom).abs() >= 0.001
    {
        return sanitize_projection_zoom(zoom_state.last_rendered_zoom, runtime_zoom);
    }

    sanitize_projection_zoom(requested_display_zoom, runtime_zoom)
}

fn scene_shell_width(active_target: &crate::editor::session::ActiveEditorTarget) -> f32 {
    (active_target.scene.shell_bbox.right - active_target.scene.shell_bbox.left).max(1.0)
}

fn scene_shell_height(active_target: &crate::editor::session::ActiveEditorTarget) -> f32 {
    (active_target.scene.shell_bbox.bottom - active_target.scene.shell_bbox.top).max(1.0)
}

fn body_left_offset(active_target: &crate::editor::session::ActiveEditorTarget) -> f32 {
    (active_target.scene.body_session.anchor_bbox.left - active_target.scene.shell_bbox.left)
        .max(0.0)
}

fn body_top_offset(active_target: &crate::editor::session::ActiveEditorTarget) -> f32 {
    (active_target.scene.body_session.anchor_bbox.top - active_target.scene.shell_bbox.top).max(0.0)
}

fn source_line_bbox_for_caret(
    active_target: &crate::editor::session::ActiveEditorTarget,
    caret: crate::editor::text_geometry::EditorCaretVisualPosition,
) -> Option<pdf_viewer_core::models::BoundingBox> {
    source_line_visual_bbox_for_caret(&active_target.scene.body_session, caret.baseline_y)
}

fn resolve_caret_rect(
    active_target: &crate::editor::session::ActiveEditorTarget,
    caret: crate::editor::text_geometry::EditorCaretVisualPosition,
) -> (f32, f32) {
    if let Some(line_bbox) = source_line_bbox_for_caret(active_target, caret) {
        let body_top = active_target.scene.body_session.anchor_bbox.top;
        let top = (line_bbox.top - body_top).max(0.0);
        let height = (line_bbox.bottom - line_bbox.top).max(1.0);
        return (top, height);
    }

    (
        (caret.baseline_y - caret.height).max(0.0),
        caret.height.max(1.0),
    )
}

pub fn render_active_editor_canvas(
    canvas_js: wasm_bindgen::JsValue,
    display_zoom: f32,
    _draft_text: String,
    _caret_index: u32,
) -> bool {
    let Some(active_state) = get_active_editor_state() else {
        return false;
    };
    let caret_index = active_state.normalized_caret_index();
    let normalized_text = active_state.current_text().to_string();
    let underline_active = active_state.is_underline_active();
    let explicit_underline_active = active_state.has_style_changes() && underline_active;
    let replaces_source = active_state.requires_source_replacement();
    let active_target = active_state.target;
    let Ok(canvas) = canvas_js.dyn_into::<HtmlCanvasElement>() else {
        return false;
    };

    let projection_zoom = resolve_editor_projection_zoom(display_zoom);
    let shell_width_px = scene_shell_width(&active_target);
    let shell_height_px = scene_shell_height(&active_target);
    dbg_event(
        "visual.paint",
        "active-editor",
        vec![
            dbg_field("paragraphId", active_target.paragraph_id.as_str()),
            dbg_field("sourceColor", active_target.color.as_str()),
            dbg_field(
                "sourceTextDecoration",
                active_target.text_decoration.as_str(),
            ),
            dbg_field("sourceUnderline", underline_active),
            dbg_field("explicitUnderline", explicit_underline_active),
            dbg_field("replacesSource", replaces_source),
            dbg_field("bodyTopOffset", body_top_offset(&active_target)),
            dbg_field("shellWidth", shell_width_px),
            dbg_field("shellHeight", shell_height_px),
            dbg_field("displayZoom", display_zoom),
            dbg_field("projectionZoom", projection_zoom),
        ],
    );
    let source_underline_run_count = active_target
        .scene
        .body_session
        .paragraph
        .runs
        .iter()
        .filter(|run| run.style.is_underline)
        .count();
    dbg_event(
        "visual.paint",
        "style-flags",
        vec![
            dbg_field("paragraphId", active_target.paragraph_id.as_str()),
            dbg_field("sourceUnderline", underline_active),
            dbg_field("explicitUnderline", explicit_underline_active),
            dbg_field("replacesSource", replaces_source),
            dbg_field("sourceUnderlineRunCount", source_underline_run_count),
            dbg_field(
                "targetTextDecoration",
                active_target.text_decoration.as_str(),
            ),
            dbg_field("sourceColor", active_target.color.as_str()),
        ],
    );
    let css_width = shell_width_px * projection_zoom;
    let css_height = (shell_height_px + (EDITOR_Y_BUFFER * 2.0)) * projection_zoom;
    let mut renderer = CanvasRenderer::new_overlay(canvas);
    // The active editor shell is only an input/caret/text overlay. Page content
    // occlusion belongs to the Rust page render plan, otherwise this transparent
    // host canvas can accidentally cover nearby decorative PDF paths and leave a
    // partial blue line visible outside the shell.
    renderer.transparent_surface = true;
    renderer.sync_size(css_width, css_height, projection_zoom);
    renderer.clear_dirty_rect(
        0.0,
        0.0,
        shell_width_px,
        shell_height_px + (EDITOR_Y_BUFFER * 2.0),
    );

    let body_left_offset = body_left_offset(&active_target);
    let body_top_offset = body_top_offset(&active_target);
    let caret = active_caret_visual(&active_target, &normalized_text, caret_index);
    let (caret_top, caret_height) = resolve_caret_rect(&active_target, caret);
    let draw_caret = |renderer: &CanvasRenderer| {
        renderer.ctx.set_fill_style_str("#111827");
        renderer.ctx.fill_rect(
            (body_left_offset + caret.left) as f64,
            (body_top_offset + caret_top) as f64,
            CARET_WIDTH as f64,
            caret_height as f64,
        );
    };

    if !replaces_source {
        draw_caret(&renderer);
        return true;
    }

    dbg_event(
        "visual.paint",
        "shell-caret-only",
        vec![
            dbg_field("paragraphId", active_target.paragraph_id.as_str()),
            dbg_field("drawUnderline", false),
            dbg_field("paintOwner", "page-canvas"),
            dbg_field("draftText", normalized_text.as_str()),
            dbg_field("caretBaselineY", caret.baseline_y),
            dbg_field("caretHeight", caret_height),
            dbg_field("caretTop", caret_top),
        ],
    );

    draw_caret(&renderer);
    true
}
