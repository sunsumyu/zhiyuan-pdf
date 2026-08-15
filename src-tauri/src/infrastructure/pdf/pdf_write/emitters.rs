use super::PersistedTextLinePlan;
use lopdf::Object;

// ── PDF operation emitters (command pattern) ──────────────────────────────
// Each function returns a Vec<Operation> expressing one rendering intent,
// decoupling the high-level "draw a text line / underline" semantics from
// the low-level PDF operator sequencing in `apply_batch_reflow_to_doc`.

/// Geometry of a single underline stroke, ready to emit as PDF path operators.
pub(crate) struct UnderlineSpec {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) width: f32,
    pub(crate) stroke_width: f32,
    pub(crate) color: String,
}

/// Emit the PDF text operators for one reflow line: color, text matrix, font, show.
/// Returns the operations plus an optional underline spec when the line needs one.
pub(crate) fn emit_text_line_ops(run: &PersistedTextLinePlan, user_unit: f32) -> (Vec<lopdf::content::Operation>, Option<UnderlineSpec>) {
    let mut ops = Vec::new();
    let h_scale = run.horizontal_scaling / 100.0;
    let adj_tx = run.tx / user_unit;
    let adj_ty = run.ty / user_unit;
    let adj_width = run.width / user_unit;
    let adj_font_size = run.font_size / user_unit;

    if let Some([red, green, blue]) = crate::infrastructure::pdf::color::parse_pdf(&run.color) {
        ops.push(lopdf::content::Operation::new(
            "rg",
            vec![Object::Real(red), Object::Real(green), Object::Real(blue)],
        ));
        ops.push(lopdf::content::Operation::new(
            "RG",
            vec![Object::Real(red), Object::Real(green), Object::Real(blue)],
        ));
    }

    // Preserve the original render mode; stroke-enabled modes get a faux-bold
    // stroke weight proportional to the font size.
    if run.render_mode != 0 {
        ops.push(lopdf::content::Operation::new(
            "Tr",
            vec![Object::Integer(run.render_mode as i64)],
        ));
        if run.render_mode == 1 || run.render_mode == 2 {
            let bold_stroke = (adj_font_size * 0.03).max(0.3);
            ops.push(lopdf::content::Operation::new(
                "w",
                vec![Object::Real(bold_stroke)],
            ));
        }
    }

    ops.push(lopdf::content::Operation::new(
        "Tm",
        vec![
            Object::Real(h_scale),
            Object::Real(0.0),
            Object::Real(0.0),
            Object::Real(1.0),
            Object::Real(adj_tx),
            Object::Real(adj_ty),
        ],
    ));
    ops.push(lopdf::content::Operation::new(
        "Tf",
        vec![
            Object::Name(run.font_alias.clone()),
            Object::Real(adj_font_size),
        ],
    ));
    ops.push(lopdf::content::Operation::new(
        "Tj",
        vec![Object::String(
            run.encoded_bytes.clone(),
            lopdf::StringFormat::Hexadecimal,
        )],
    ));
    if run.render_mode != 0 {
        ops.push(lopdf::content::Operation::new(
            "Tr",
            vec![Object::Integer(0)],
        ));
    }

    let underline = if run.is_underline && adj_width > 0.0 {
        Some(UnderlineSpec {
            x: adj_tx,
            y: adj_ty + (adj_font_size * 0.12),
            width: adj_width,
            stroke_width: (adj_font_size * 0.055).max(0.6),
            color: run.color.clone(),
        })
    } else {
        None
    };

    (ops, underline)
}

/// Emit the PDF path operators for one underline stroke: color, width, move, line, stroke.
pub(crate) fn emit_underline_ops(spec: &UnderlineSpec) -> Vec<lopdf::content::Operation> {
    let mut ops = Vec::new();
    if let Some([r, g, b]) = crate::infrastructure::pdf::color::parse_pdf(&spec.color) {
        ops.push(lopdf::content::Operation::new(
            "RG",
            vec![Object::Real(r), Object::Real(g), Object::Real(b)],
        ));
    }
    ops.push(lopdf::content::Operation::new("w", vec![Object::Real(spec.stroke_width)]));
    ops.push(lopdf::content::Operation::new(
        "m",
        vec![Object::Real(spec.x), Object::Real(spec.y)],
    ));
    ops.push(lopdf::content::Operation::new(
        "l",
        vec![Object::Real(spec.x + spec.width), Object::Real(spec.y)],
    ));
    ops.push(lopdf::content::Operation::new("S", vec![]));
    ops
}

/// Emit the full deferred-text block: graphics state setup, text lines, underlines, teardown.
/// `page_height` drives the Y-flip cm matrix; `user_unit` scales coordinates.
pub(crate) fn emit_deferred_text_block(
    lines: &[PersistedTextLinePlan],
    page_height: f32,
    user_unit: f32,
) -> Vec<lopdf::content::Operation> {
    let mut ops = Vec::new();
    // Graphics state: save, flip-Y, reset char/word spacing + horizontal scaling, begin text.
    ops.push(lopdf::content::Operation::new("q", vec![]));
    ops.push(lopdf::content::Operation::new(
        "cm",
        vec![
            Object::Real(1.0),
            Object::Real(0.0),
            Object::Real(0.0),
            Object::Real(-1.0),
            Object::Real(0.0),
            Object::Real(page_height),
        ],
    ));
    ops.push(lopdf::content::Operation::new("Tc", vec![Object::Real(0.0)]));
    ops.push(lopdf::content::Operation::new("Tw", vec![Object::Real(0.0)]));
    ops.push(lopdf::content::Operation::new("Tz", vec![Object::Real(100.0)]));
    ops.push(lopdf::content::Operation::new("BT", vec![]));

    let mut rendered = std::collections::HashSet::new();
    let mut underlines: Vec<UnderlineSpec> = Vec::new();
    for run in lines {
        if !rendered.insert((run.patch_idx, run.line_seq)) {
            continue;
        }
        let (line_ops, underline) = emit_text_line_ops(run, user_unit);
        ops.extend(line_ops);
        if let Some(spec) = underline {
            underlines.push(spec);
        }
    }
    ops.push(lopdf::content::Operation::new("ET", vec![]));

    for spec in &underlines {
        ops.extend(emit_underline_ops(spec));
    }
    ops.push(lopdf::content::Operation::new("Q", vec![]));
    ops
}
