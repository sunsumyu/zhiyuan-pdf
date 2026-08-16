use crate::infrastructure::pdf::layout_analyzer::LayoutGraphAnalyzer;
use crate::infrastructure::pdf::models::{
    LayoutInferenceResult, NativeTextModel, NativeVectorPageModel, PageDisplayList, RenderObject,
    StyledRun,
};
use std::time::Instant;

pub fn resolve_display_list(
    doc: &lopdf::Document,
    page_index: u16,
) -> Result<PageDisplayList, String> {
    crate::pdf_log!(2, "[PDF-Vector] Pure Vector Extraction starting...");
    crate::infrastructure::pdf::pdf_read::resolve_paths(doc, page_index as u32).map(
        |(objects, text_runs, width, height)| {
            crate::pdf_log!(
                2,
                "[PDF-Vector] Extraction SUCCESS: paths/images={}, text_runs={}, size={}x{}",
                objects.len(),
                text_runs.len(),
                width,
                height
            );
            PageDisplayList {
                page_index,
                width,
                height,
                objects,
                text_runs,
            }
        },
    )
}

/// 解析 PDF 单页的纯向量数据 (V206.55 - Optimized Memory Cache 版本)
pub fn resolve_model(
    doc: &lopdf::Document,
    page_index: u16,
) -> Result<NativeVectorPageModel, String> {
    let display_list = match resolve_display_list(doc, page_index) {
        Ok(display_list) => display_list,
        Err(e) => {
            crate::log_step!("[PDF-LOPDF-ERR] Failed: {}", e);
            PageDisplayList {
                page_index,
                width: 595.0,
                height: 842.0,
                objects: Vec::new(),
                text_runs: Vec::new(),
            }
        }
    };
    build_vector_page_model_from_display_list(&display_list)
}

pub fn build_vector_page_model_from_display_list(
    display_list: &PageDisplayList,
) -> Result<NativeVectorPageModel, String> {
    let start_total = Instant::now();
    let page_index = display_list.page_index;
    let mut render_objects = display_list.objects.clone();
    let text_runs = display_list.text_runs.clone();
    let pw = display_list.width;
    let ph = display_list.height;

    // 按 Z-index 排序
    render_objects.sort_by_key(|o| match o {
        RenderObject::Text(t) => t.z_index,
        RenderObject::Path(p) => p.z_index,
        RenderObject::Image(i) => i.z_index,
    });

    if !text_runs.is_empty() {
        let mut runs = text_runs;
        let _start_reflow = Instant::now();

        // 维持 V195 文本聚合逻辑
        runs.sort_by(|a, b| b.ty.partial_cmp(&a.ty).unwrap_or(std::cmp::Ordering::Equal));
        let mut grouped_runs: Vec<Vec<StyledRun>> = Vec::new();
        if let Some(first) = runs.first() {
            let mut current_group = vec![first.clone()];
            for run in runs.iter().skip(1) {
                // Same visual line AND same style: only runs that share font,
                // color and size may merge into one text object, otherwise
                // bold/regular or black/colored neighbours fuse into one run.
                let last = current_group.last().unwrap();
                let can_group = (run.ty - last.ty).abs() < 0.5
                    && run.font_name == last.font_name
                    && run.embedded_font_key == last.embedded_font_key
                    && run.color == last.color
                    && (run.font_size - last.font_size).abs() < 0.1;
                if can_group {
                    current_group.push(run.clone());
                } else {
                    grouped_runs.push(current_group);
                    current_group = vec![run.clone()];
                }
            }
            grouped_runs.push(current_group);
        }

        for mut group in grouped_runs {
            group.sort_by(|a, b| a.tx.partial_cmp(&b.tx).unwrap_or(std::cmp::Ordering::Equal));
            let mut line_text = String::new();
            let first = group[0].clone();
            let mut total_width = 0.0;
            for run in &group {
                line_text.push_str(&run.text);
                total_width += run.width;
            }

            let object_indices: Vec<usize> = group.iter().map(|r| r.z_index).collect();
            // Combine per-char geometry across the whole group, expressed
            // relative to the first run's origin (dx from first.tx, dy from
            // first.ty) so multi-run lines keep every glyph's position.
            let mut combined_origins = Vec::new();
            let mut combined_widths = Vec::new();
            let mut combined_codes = Vec::new();
            let base_tx = first.tx;
            let base_ty = first.ty;

            for run in &group {
                let dx = run.tx - base_tx;
                let dy = run.ty - base_ty;
                if run.char_origins.is_empty() {
                    let run_char_count = if !run.pdf_char_codes.is_empty() {
                        run.pdf_char_codes.len()
                    } else {
                        run.text.chars().count()
                    };
                    let avg_w = if run_char_count > 0 && run.width > 0.0 {
                        run.width / run_char_count as f32
                    } else {
                        run.font_size * 0.6
                    };
                    let mut cur_x = dx;
                    for _ in 0..run_char_count {
                        combined_origins.push([cur_x, dy]);
                        cur_x += avg_w;
                    }
                } else {
                    for &orig in &run.char_origins {
                        combined_origins.push([dx + orig, dy]);
                    }
                }
                combined_widths.extend_from_slice(&run.char_widths);
                combined_codes.extend_from_slice(&run.pdf_char_codes);
            }

            crate::pdf_log!(
                3,
                "[PDF-Vector] Grouped text line: '{}' at ({:.1}, {:.1})",
                line_text,
                first.tx,
                first.ty
            );
            render_objects.push(RenderObject::Text(NativeTextModel {
                id: format!("text_{}_{}", page_index, render_objects.len()),
                text: line_text,
                left: first.tx,
                top: first.ty, // UNIFIED: Raw PDF Y
                baseline_y: first.ty,
                width: total_width,
                height: first.font_size,
                font_size: first.font_size,
                font_name: first.font_name.clone(),
                color: first.color.clone(),
                stroke_color: first.stroke_color.clone(),
                stroke_width: first.stroke_width,
                scale_x: first.a,
                shear_x: first.c,
                shear_y: first.b,
                scale_y: first.d,
                tx: first.tx,
                ty: first.ty,
                char_spacing: first.char_spacing,
                horizontal_scaling: first.horizontal_scaling,
                is_bold: first.is_bold,
                is_italic: first.is_italic,
                runs: group,
                object_indices,
                z_index: first.z_index,
                font_hints: first.font_hints.clone(),
                font_post_script_name: first.font_post_script_name.clone(),
                font_family_hint: first.font_family_hint.clone(),
                font_subtype: first.font_subtype.clone(),
                embedded_font_key: first.embedded_font_key.clone(),
                has_embedded_font_file: first.has_embedded_font_file,
                has_to_unicode_cmap: first.has_to_unicode_cmap,
                char_origins: combined_origins,
                char_widths: combined_widths,
                pdf_char_codes: combined_codes,
                rendering_mode: first.render_mode as i32,
                ..Default::default()
            }));
        }
    }

    // --- Phase 5: Advanced Layout Analysis (V263) ---
    // Instead of simple bundling, we perform spatial semantics extraction.
    {
        use crate::infrastructure::pdf::models::{LayoutAlignment, LayoutRole};

        let mut text_objs: Vec<&mut NativeTextModel> = render_objects
            .iter_mut()
            .filter_map(|obj| {
                if let RenderObject::Text(t) = obj {
                    Some(t)
                } else {
                    None
                }
            })
            .collect();

        if !text_objs.is_empty() {
            // Sort by Y (top to bottom)
            text_objs.sort_by(|a, b| b.ty.partial_cmp(&a.ty).unwrap_or(std::cmp::Ordering::Equal));

            let mut current_block_id = 0;
            let mut i = 0;
            while i < text_objs.len() {
                let mut cluster_indices = vec![i];
                let mut j = i + 1;

                // --- Pass 1: Vertical Clustering (Physical Block Reconstruction) ---
                while j < text_objs.len() {
                    let prev_idx = cluster_indices.last().unwrap();
                    let (_prev_tx, prev_ty, _prev_width, prev_fs) = {
                        let prev = &text_objs[*prev_idx];
                        (prev.tx, prev.ty, prev.width, prev.font_size)
                    };
                    let curr = &text_objs[j];
                    let v_gap = (prev_ty - curr.ty).abs();
                    // Inter-line gap should be around 1.0-1.6 line height
                    if v_gap < prev_fs * 1.6 {
                        cluster_indices.push(j);
                        j += 1;
                    } else {
                        break;
                    }
                }

                let block_id = format!("b_{}_{}", page_index, current_block_id);

                // --- Pass 2: Alignment & Role Inference ---
                let cluster_objs: Vec<&&mut NativeTextModel> =
                    cluster_indices.iter().map(|&idx| &text_objs[idx]).collect();

                // Compute Alignment
                let mut alignment = LayoutAlignment::Left;
                let lefts: Vec<f32> = cluster_objs.iter().map(|o| o.tx).collect();
                let avg_left = lefts.iter().sum::<f32>() / lefts.len() as f32;
                let std_dev_left = (lefts.iter().map(|&l| (l - avg_left).powi(2)).sum::<f32>()
                    / lefts.len() as f32)
                    .sqrt();

                if std_dev_left < 2.0 {
                    alignment = LayoutAlignment::Left;
                } else {
                    // Check Center Alignment via midpoints
                    let mids: Vec<f32> =
                        cluster_objs.iter().map(|o| o.tx + o.width / 2.0).collect();
                    let avg_mid = mids.iter().sum::<f32>() / mids.len() as f32;
                    let std_dev_mid = (mids.iter().map(|&m| (m - avg_mid).powi(2)).sum::<f32>()
                        / mids.len() as f32)
                        .sqrt();
                    if std_dev_mid < 5.0 {
                        alignment = LayoutAlignment::Center;
                    }
                }

                // Compute Role
                let mut role = LayoutRole::Paragraph;
                let first_obj = &text_objs[cluster_indices[0]];
                let clean_text = first_obj.text.trim();

                if cluster_indices.len() == 1 {
                    if first_obj.is_bold && !clean_text.contains(':') && !clean_text.contains('：')
                    {
                        role = LayoutRole::SectionHeader;
                    } else if first_obj.font_size > 14.0 && current_block_id == 0 {
                        role = LayoutRole::Title;
                    }
                }

                if clean_text.starts_with('●')
                    || clean_text.starts_with('路')
                    || clean_text.starts_with('-')
                    || (clean_text.len() > 2
                        && clean_text.chars().next().map_or(false, |c| c.is_digit(10))
                        && clean_text.contains('.'))
                {
                    role = LayoutRole::ListItem;
                } else if clean_text.contains(':') || clean_text.contains('：') {
                    role = LayoutRole::KvField;
                }

                // Apply to all objects in the block
                for &idx in &cluster_indices {
                    text_objs[idx].paragraph_id = Some(block_id.clone());
                    text_objs[idx].role = Some(role);
                    text_objs[idx].alignment = Some(alignment);
                    text_objs[idx].indent = Some(text_objs[idx].tx - avg_left);
                }

                current_block_id += 1;
                i = j;
            }
        }
    }

    // --- Phase 6: Smart Palette Compression (V197) ---
    let mut palette_colors = Vec::new();
    let mut palette_fonts = Vec::new();

    if render_objects.len() > 100 {
        use rayon::prelude::*;
        let mut color_freq = std::collections::HashMap::new();
        let mut font_freq = std::collections::HashMap::new();

        // Pass 1: Frequency Analysis (Using references to avoid cloning)
        for obj in &render_objects {
            match obj {
                RenderObject::Text(t) => {
                    *color_freq.entry(&t.color).or_insert(0) += 1;
                    *font_freq.entry(&t.font_name).or_insert(0) += 1;
                }
                RenderObject::Path(p) => {
                    if let Some(c) = &p.fill_color {
                        *color_freq.entry(c).or_insert(0) += 1;
                    }
                    if let Some(c) = &p.stroke_color {
                        *color_freq.entry(c).or_insert(0) += 1;
                    }
                }
                _ => {}
            }
        }

        // Threshold: Only compress if repeats > 4 times
        palette_colors = color_freq
            .into_iter()
            .filter(|(_, count)| *count > 4)
            .map(|(val, _)| val.clone())
            .collect();
        palette_fonts = font_freq
            .into_iter()
            .filter(|(_, count)| *count > 4)
            .map(|(val, _)| val.clone())
            .collect();

        // Pass 2: Parallel Compression & Mapping
        {
            let palette_colors_ref = &palette_colors;
            let palette_fonts_ref = &palette_fonts;
            render_objects.par_iter_mut().for_each(|obj| match obj {
                RenderObject::Text(t) => {
                    if let Some(pos) = palette_colors_ref.iter().position(|c| c == &t.color) {
                        t.color_index = Some(pos as u8);
                        t.color = String::new();
                    }
                    if let Some(pos) = palette_fonts_ref.iter().position(|f| f == &t.font_name) {
                        t.font_index = Some(pos as u8);
                        t.font_name = String::new();
                    }
                }
                RenderObject::Path(p) => {
                    if let Some(fc) = &p.fill_color {
                        if let Some(pos) = palette_colors_ref.iter().position(|c| c == fc) {
                            p.fill_color_index = Some(pos as u8);
                            p.fill_color = None;
                        }
                    }
                    if let Some(sc) = &p.stroke_color {
                        if let Some(pos) = palette_colors_ref.iter().position(|c| c == sc) {
                            p.stroke_color_index = Some(pos as u8);
                            p.stroke_color = None;
                        }
                    }
                }
                _ => {}
            });
        }

        // --- Phase 7: Parallel Sorting (Strictly Preserving Z-Index) ---
        render_objects.par_sort_by(|a, b| {
            let z_a = match a {
                RenderObject::Text(t) => t.z_index,
                RenderObject::Path(p) => p.z_index,
                RenderObject::Image(i) => i.z_index,
            };
            let z_b = match b {
                RenderObject::Text(t) => t.z_index,
                RenderObject::Path(p) => p.z_index,
                RenderObject::Image(i) => i.z_index,
            };
            z_a.cmp(&z_b)
        });
    }

    // --- Phase 8: Occlusion Culling ---
    // V1 "full-page masking" culling was disabled (drain commented out) and has
    // been removed; it only produced logs. Revisit with real culling if IPC
    // payload size ever becomes the bottleneck.

    let palette = crate::infrastructure::pdf::models::VectorPalette {
        colors: palette_colors,
        fonts: palette_fonts,
    };

    let mut model = NativeVectorPageModel {
        page_index,
        width: pw,
        height: ph,
        objects: render_objects,
        palette,
        background_image: None,
    };
    model.flip_y();

    crate::pdf_log!(
        2,
        "[PROF] Vector Model Ready: {} objects, Palette size: (C:{}, F:{}). Total Time: {:?}",
        model.objects.len(),
        model.palette.colors.len(),
        model.palette.fonts.len(),
        start_total.elapsed()
    );

    Ok(model)
}

/// 执行 V3 级布局推断 (三阶段图驱动)
pub fn resolve_layout_inference(
    doc: &lopdf::Document,
    page_index: u16,
) -> Result<LayoutInferenceResult, String> {
    let display_list = resolve_display_list(doc, page_index)
        .map_err(|e| format!("Extraction failed: {}", e))?;
    resolve_layout_inference_from_display_list(&display_list)
}

pub fn resolve_layout_inference_from_display_list(
    display_list: &PageDisplayList,
) -> Result<LayoutInferenceResult, String> {
    crate::pdf_log!(2, "[PDF-V3] Starting Layout Inference...");

    let mut result = LayoutGraphAnalyzer::new(
        display_list.page_index,
        display_list.width,
        display_list.height,
    )
    .analyze(display_list.text_runs.clone());

    result.flip_y();

    crate::pdf_log!(
        2,
        "[PDF-V3] Inference complete: {} regions identified.",
        result.regions.len()
    );

    Ok(result)
}
