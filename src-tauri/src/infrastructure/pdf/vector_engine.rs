use crate::infrastructure::pdf::layout_analyzer::LayoutGraphAnalyzer;
use crate::infrastructure::pdf::models::{
    LayoutInferenceResult, NativeTextModel, RenderObject, StyledRun, NativeVectorPageModel,
};
use crate::log_step;
use std::time::Instant;

/// 瑙ｆ瀽 PDF 鍗曢〉鐨勭函鍚戦噺鏁版嵁 (V206.55 - Optimized Memory Cache 鐗
pub fn get_vector_page_model_with_doc(
    doc: &lopdf::Document,
    page_index: u16,
) -> Result<NativeVectorPageModel, String> {
    let start_total = Instant::now();
    log_step!("[PDF-Vector] Pure Vector Extraction starting...");

    let (mut render_objects, text_runs, pw, ph) =
        match crate::infrastructure::pdf::pdf_read::resolve_paths(
            &doc,
            page_index as u32,
        ) {
            Ok(res) => {
                log_step!("[PDF-Vector] Extraction SUCCESS: paths/images={}, text_runs={}, size={}x{}", res.0.len(), res.1.len(), res.2, res.3);
                res
            },
            Err(e) => {
                log_step!("[PDF-LOPDF-ERR] Failed: {}", e);
                (Vec::new(), Vec::new(), 595.0, 842.0)
            }
        };

    // 鎸?Z-index 鎺掑簭
    render_objects.sort_by_key(|o| match o {
        RenderObject::Text(t) => t.z_index,
        RenderObject::Path(p) => p.z_index,
        RenderObject::Image(i) => i.z_index,
    });

    if !text_runs.is_empty() {
        let mut runs = text_runs;
        let _start_reflow = Instant::now();

        // 缁存寔 V195 鏂囨湰鑱氬悎閫昏緫
        runs.sort_by(|a, b| b.ty.partial_cmp(&a.ty).unwrap_or(std::cmp::Ordering::Equal));
        let mut grouped_runs: Vec<Vec<StyledRun>> = Vec::new();
        if let Some(first) = runs.first() {
            let mut current_group = vec![first.clone()];
            let mut last_y = first.ty;
            for run in runs.iter().skip(1) {
                // V256: ARCHITECTURAL FIX - High-precision sub-pixel line grouping (0.5 threshold)
                // This prevents merging "Name" and "Address" into the same row if they drift even 1pt.
                if (run.ty - last_y).abs() < 0.5 {
                    current_group.push(run.clone());
                } else {
                    grouped_runs.push(current_group);
                    current_group = vec![run.clone()];
                    last_y = run.ty;
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
            log_step!("[PDF-Vector] Grouped text line: '{}' at ({:.1}, {:.1})", line_text, first.tx, first.ty);
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
                has_embedded_font_program: first.has_embedded_font_program,
                has_to_unicode_cmap: first.has_to_unicode_cmap,
                char_origins: first.char_origins.iter().map(|&x| [x, 0.0]).collect(),
                char_widths: first.char_widths.clone(),
                pdf_char_codes: first.pdf_char_codes.clone(),
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

        // --- Phase 7: Parallel Style-Based Sorting ---
        render_objects.par_sort_by(|a, b| {
            let style_a = match a {
                RenderObject::Text(t) => (0, t.font_index, t.color_index),
                RenderObject::Path(p) => (1, p.fill_color_index, p.stroke_color_index),
                RenderObject::Image(_) => (2, None, None),
            };
            let style_b = match b {
                RenderObject::Text(t) => (0, t.font_index, t.color_index),
                RenderObject::Path(p) => (1, p.fill_color_index, p.stroke_color_index),
                RenderObject::Image(_) => (2, None, None),
            };
            style_a.cmp(&style_b)
        });
    }

    // --- Phase 8: Occlusion Culling (V1: Full-page Masking) ---
    // If a top-level object (like a background image) covers the whole page,
    // we can ignore everything below it to save IPC and Render cycles.
    if render_objects.len() > 50 {
        let mut cull_idx = None;
        for (i, obj) in render_objects.iter().enumerate().rev() {
            let is_opaque_mask = match obj {
                RenderObject::Image(img) => {
                    // Check if image covers ~98% of the page area
                    img.width >= pw * 0.98 && img.height >= ph * 0.98
                }
                _ => false,
            };
            if is_opaque_mask {
                cull_idx = Some(i);
                break;
            }
        }
        if let Some(idx) = cull_idx {
            if idx > 0 {
                let dropped = idx;
                log_step!(
                    "[PROF] Occlusion Culling: Dropped {} objects hidden beneath full-page mask at index {}",
                    dropped,
                    idx
                );
                log_step!(
                    "[PDF-CULL] Disabling drain for debug: would have culled {} objects",
                    idx
                );
                // render_objects.drain(0..idx);
            }
        }
    }

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
    
    log_step!(
        "[PROF] Vector Model Ready: {} objects, Palette size: (C:{}, F:{}). Total Time: {:?}",
        model.objects.len(),
        model.palette.colors.len(),
        model.palette.fonts.len(),
        start_total.elapsed()
    );

    Ok(model)
}

/// 鎵ц V3 绾у竷灞€鎺ㄦ柇 (涓夐樁娈靛浘椹卞姩)
pub fn get_layout_inference(
    doc: &lopdf::Document,
    page_index: u16,
) -> Result<LayoutInferenceResult, String> {
    log_step!("[PDF-V3] Starting Layout Inference...");

    // 1. 鐗╃悊鎻愬彇
    let (_, text_runs, pw, ph) =
        match crate::infrastructure::pdf::pdf_read::resolve_paths(
            &doc,
            page_index as u32,
        ) {
            Ok(res) => res,
            Err(e) => return Err(format!("Extraction failed: {}", e)),
        };

    // 2. 初始化 V3 分析器
    let mut result = LayoutGraphAnalyzer::new(page_index, pw, ph).analyze(text_runs);

    // 3. 执行 Y 轴翻转，统一到 Y-Down 坐标系
    result.flip_y();

    log_step!(
        "[PDF-V3] Inference complete: {} regions identified.",
        result.regions.len()
    );

    Ok(result)
}
