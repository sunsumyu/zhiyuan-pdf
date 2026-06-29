use crate::infrastructure::pdf::models::{NativeTextModel, RenderObject};
use crate::infrastructure::pdf::{color_utils, path_utils};
use cosmic_text::{Buffer, FontSystem, Metrics, Shaping, SwashCache};
use image::{ImageBuffer, Rgba};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use swash::proxy::MetricsProxy;
use swash::scale::ScaleContext;
use vello::kurbo::{Affine, BezPath, Stroke, Vec2};
use vello::peniko::{Color, Fill};
use vello::{Renderer, RendererOptions, Scene};
use wgpu::{
    Backends, BufferDescriptor, BufferUsages, DeviceDescriptor, Extent3d, ImageCopyTexture,
    ImageDataLayout, Instance, InstanceDescriptor, Limits, MapMode, PowerPreference,
    RequestAdapterOptions, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
};

pub mod font_resolver;

pub struct VelloRenderer {
    pub(super) device: Arc<wgpu::Device>,
    pub(super) queue: Arc<wgpu::Queue>,
    pub(super) renderer: Renderer,
    pub(super) font_system: FontSystem,
    pub(super) swash_cache: SwashCache,
    pub(super) font_file_cache: HashMap<std::path::PathBuf, Arc<Vec<u8>>>,
    pub(super) font_matcher: crate::infrastructure::pdf::font::matching::PdfSystemFontMatcher,
}

pub(super) fn text_fill_enabled(render_mode: i32) -> bool {
    matches!(render_mode, 0 | 2 | 4 | 6)
}
pub(super) fn text_stroke_enabled(render_mode: i32) -> bool {
    matches!(render_mode, 1 | 2 | 5 | 6)
}
pub(super) fn text_is_non_painting(render_mode: i32) -> bool {
    matches!(render_mode, 3 | 7)
}

/// Whether this text run warrants verbose render-path tracing.
/// Gate: known diagnostic marker or large font size.
pub(super) fn should_trace_text_render(text: &NativeTextModel) -> bool {
    text.text.contains("绠€") || text.font_size > 20.0
}

impl VelloRenderer {
    pub async fn new() -> Result<Self, String> {
        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::all(),
            ..Default::default()
        });
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                ..Default::default()
            })
            .await
            .ok_or("Failed to find wgpu adapter")?;

        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: Some("Vello Headless Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .map_err(|e| e.to_string())?;

        let renderer = Renderer::new(
            &device,
            RendererOptions {
                surface_format: None,
                use_cpu: false,
                antialiasing_support: vello::AaSupport::all(),
                num_init_threads: None,
            },
        )
        .map_err(|e| e.to_string())?;

        Ok(Self {
            device: Arc::new(device),
            queue: Arc::new(queue),
            renderer,
            font_system: FontSystem::new(),
            swash_cache: SwashCache::new(),
            font_file_cache: HashMap::new(),
            font_matcher: crate::infrastructure::pdf::font::matching::PdfSystemFontMatcher::new(
                crate::infrastructure::pdf::font::catalog::load_system_font_candidates(),
                "Microsoft YaHei",
            ),
        })
    }

    /// Two-phase rendering:
    /// Phase 1 (GPU): Vello renders paths + images
    /// Phase 2 (CPU): cosmic_text + swash renders text as pixel coverage
    pub fn render_objects_to_png(
        &mut self,
        objects: &[RenderObject],
        width: u32,
        height: u32,
        zoom: f32,
    ) -> Result<Vec<u8>, String> {
        let mut scene = Scene::new();
        let mut scale_context = ScaleContext::new();
        let mut vector_rendered_text_ids = HashSet::new();

        // ── Standard PDF Coordinate Transform (Flip Y + Zoom) ──
        let flip_y = Affine::scale_non_uniform(zoom as f64, -zoom as f64)
            .then_translate(vello::kurbo::Vec2::new(0.0, height as f64));

        for object in objects {
            match object {
                RenderObject::Path(path) => {
                    let bez_path = path_utils::path_segments_to_bez_path(&path.segments);

                    if path.fill {
                        let color = color_utils::parse_hex_vello_color(
                            path.fill_color.as_deref().unwrap_or("#000000"),
                            path.alpha,
                        );
                        scene.fill(Fill::NonZero, flip_y, color, None, &bez_path);
                    }
                    if path.stroke {
                        let color = color_utils::parse_hex_vello_color(
                            path.stroke_color.as_deref().unwrap_or("#000000"),
                            path.alpha,
                        );
                        scene.stroke(
                            &Stroke::new(path.stroke_width as f64),
                            flip_y,
                            color,
                            None,
                            &bez_path,
                        );
                    }
                }
                RenderObject::Text(text) => {
                    if self.draw_text_vector(&mut scene, &mut scale_context, text, flip_y) {
                        if !text.id.is_empty() {
                            vector_rendered_text_ids.insert(text.id.clone());
                        }
                    }
                }
                _ => {}
            }
        }

        let rgba_data = self.perform_vello_render_raw(&scene, width, height)?;

        // ── Phase 2: CPU overlay for text + legacy images ──
        let mut img = ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, rgba_data)
            .ok_or("Failed to create image buffer from Vello output")?;

        for object in objects {
            match object {
                RenderObject::Text(text_model) => {
                    if !text_model.id.is_empty()
                        && vector_rendered_text_ids.contains(&text_model.id)
                    {
                        continue;
                    }
                    self.draw_text_bitmap_deprecated(&mut img, text_model, zoom, width, height);
                }
                RenderObject::Image(image_model) => {
                    self.draw_image_cpu(&mut img, image_model, zoom, width, height);
                }
                _ => {}
            }
        }

        // ── Phase 3: Encode to PNG ──
        let mut png_data = Vec::new();
        img.write_to(
            &mut std::io::Cursor::new(&mut png_data),
            image::ImageFormat::Png,
        )
        .map_err(|e| e.to_string())?;
        Ok(png_data)
    }

    /// CPU image rendering for icons/logos.
    fn draw_image_cpu(
        &mut self,
        img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
        model: &crate::infrastructure::pdf::models::NativeImageModel,
        zoom: f32,
        canvas_w: u32,
        canvas_h: u32,
    ) {
        let asset_id = if let Some(id) = model.data_url.strip_prefix("http://pdfasset.localhost/") {
            id
        } else {
            return;
        };

        let image_data = {
            let cache = crate::infrastructure::pdf::cache::PDF_IMAGE_CACHE
                .lock()
                .unwrap();
            cache.get(asset_id).cloned()
        };

        let raw_bytes = match image_data {
            Some(bytes) => bytes,
            None => return,
        };

        let src_img = match image::load_from_memory(&raw_bytes) {
            Ok(i) => i.to_rgba8(),
            Err(_) => return,
        };

        let target_w = (model.width * zoom).abs() as u32;
        let target_h = (model.height * zoom).abs() as u32;

        if target_w == 0 || target_h == 0 {
            return;
        }

        let target_x = (model.x * zoom) as i32;
        let target_y = (canvas_h as f32 - (model.y + model.height) * zoom) as i32;

        let resized = if src_img.width() != target_w || src_img.height() != target_h {
            image::imageops::resize(
                &src_img,
                target_w,
                target_h,
                image::imageops::FilterType::Lanczos3,
            )
        } else {
            src_img
        };

        for py in 0..target_h {
            for px in 0..target_w {
                let fx = target_x + px as i32;
                let fy = target_y + py as i32;

                if fx >= 0 && fx < canvas_w as i32 && fy >= 0 && fy < canvas_h as i32 {
                    let src_pixel = resized.get_pixel(px, py);
                    if src_pixel[3] == 0 {
                        continue;
                    }

                    let dst_pixel = img.get_pixel_mut(fx as u32, fy as u32);
                    let alpha = src_pixel[3] as f32 / 255.0;

                    dst_pixel.0 = [
                        color_utils::blend(dst_pixel[0], src_pixel[0], alpha),
                        color_utils::blend(dst_pixel[1], src_pixel[1], alpha),
                        color_utils::blend(dst_pixel[2], src_pixel[2], alpha),
                        255,
                    ];
                }
            }
        }
    }

    /// CPU text rendering using cosmic_text's SwashCache for glyph rasterization.
    fn draw_text_bitmap_deprecated(
        &mut self,
        img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
        text: &NativeTextModel,
        zoom: f32,
        canvas_w: u32,
        canvas_h: u32,
    ) {
        if text_is_non_painting(text.rendering_mode) {
            return;
        }
        let real_font_size = if text.scale_y.abs() > 1.0 {
            text.scale_y.abs()
        } else {
            text.font_size
        };
        let font_size = real_font_size * zoom;

        let metrics = Metrics::new(
            font_size,
            if font_size > 0.1 {
                font_size * 1.2
            } else {
                1.0
            },
        );
        let mut buffer = Buffer::new(&mut self.font_system, metrics);
        buffer.set_size(
            &mut self.font_system,
            Some(canvas_w as f32),
            Some(canvas_h as f32),
        );
        let mut attrs = cosmic_text::Attrs::new();
        if text.is_bold {
            attrs = attrs.weight(cosmic_text::Weight::BOLD);
        }
        if text.is_italic {
            attrs = attrs.style(cosmic_text::Style::Italic);
        }
        let resolved_font = self.resolve_pdf_font(text);
        attrs = attrs.family(self.resolve_cosmic_family(text, &resolved_font));

        let (cr, cg, cb) = color_utils::parse_hex_color_rgb(if text.color.is_empty() {
            "#000000"
        } else {
            &text.color
        });

        let mut buffer = Buffer::new(&mut self.font_system, metrics);
        buffer.set_text(&mut self.font_system, &text.text, attrs, Shaping::Advanced);
        buffer.shape_until_scroll(&mut self.font_system, false);

        let mut matched_font = "Unknown".to_string();
        for run in buffer.layout_runs() {
            for glyph in run.glyphs {
                if let Some(font) = self.font_system.db().face(glyph.font_id) {
                    matched_font = format!("{:?} {:?}", font.families, font.weight);
                    break;
                }
            }
        }
        if should_trace_text_render(text) {
            println!(
                "[FONT-MATCH] REQ: '{}' | MATCHED: '{}' | RENDER_MODE: {} | TEXT: '{}'",
                text.font_name,
                matched_font,
                text.rendering_mode,
                if text.text.len() > 10 {
                    format!("{}...", &text.text[..10])
                } else {
                    text.text.clone()
                }
            );
        }

        let base_x = text.tx * zoom;
        let base_y = canvas_h as f32 - text.ty * zoom;

        for run in buffer.layout_runs() {
            for glyph in run.glyphs {
                let physical = glyph.physical((0., 0.), 1.0);
                if let Some(glyph_img) = self
                    .swash_cache
                    .get_image_uncached(&mut self.font_system, physical.cache_key)
                {
                    let gx = base_x as i32 + physical.x + glyph_img.placement.left;
                    let gy = base_y as i32 + physical.y - glyph_img.placement.top;
                    self.composite_glyph(img, &glyph_img, gx, gy, cr, cg, cb, canvas_w, canvas_h);
                }
            }
        }
    }

    /// Helper to composite a single glyph mask/color image onto the PNG buffer
    fn composite_glyph(
        &self,
        img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
        glyph_img: &cosmic_text::SwashImage,
        gx: i32,
        gy: i32,
        cr: u8,
        cg: u8,
        cb: u8,
        canvas_w: u32,
        canvas_h: u32,
    ) {
        let gw = glyph_img.placement.width as i32;
        let gh = glyph_img.placement.height as i32;
        if gw == 0 || gh == 0 {
            return;
        }

        match glyph_img.content {
            cosmic_text::SwashContent::Mask => {
                for py in 0..gh {
                    for px in 0..gw {
                        let alpha = glyph_img.data[(py * gw + px) as usize] as f32 / 255.0;
                        if alpha < 0.01 {
                            continue;
                        }
                        let fx = gx + px;
                        let fy = gy + py;
                        if fx >= 0 && fx < canvas_w as i32 && fy >= 0 && fy < canvas_h as i32 {
                            let pixel = img.get_pixel_mut(fx as u32, fy as u32);
                            let bg = pixel.0;
                            pixel.0 = [
                                color_utils::blend(bg[0], cr, alpha),
                                color_utils::blend(bg[1], cg, alpha),
                                color_utils::blend(bg[2], cb, alpha),
                                255,
                            ];
                        }
                    }
                }
            }
            cosmic_text::SwashContent::Color => {
                for py in 0..gh {
                    for px in 0..gw {
                        let idx = ((py * gw + px) * 4) as usize;
                        if idx + 3 >= glyph_img.data.len() {
                            break;
                        }
                        let sa = glyph_img.data[idx + 3] as f32 / 255.0;
                        if sa < 0.01 {
                            continue;
                        }
                        let fx = gx + px;
                        let fy = gy + py;
                        if fx >= 0 && fx < canvas_w as i32 && fy >= 0 && fy < canvas_h as i32 {
                            let sr = glyph_img.data[idx];
                            let sg = glyph_img.data[idx + 1];
                            let sb = glyph_img.data[idx + 2];
                            let pixel = img.get_pixel_mut(fx as u32, fy as u32);
                            let bg = pixel.0;
                            pixel.0 = [
                                color_utils::blend(bg[0], sr, sa),
                                color_utils::blend(bg[1], sg, sa),
                                color_utils::blend(bg[2], sb, sa),
                                255,
                            ];
                        }
                    }
                }
            }
            cosmic_text::SwashContent::SubpixelMask => {
                for py in 0..gh {
                    for px in 0..gw {
                        let idx = ((py * gw + px) * 3) as usize;
                        if idx + 2 >= glyph_img.data.len() {
                            break;
                        }
                        let alpha = glyph_img.data[idx + 1] as f32 / 255.0;
                        if alpha < 0.01 {
                            continue;
                        }
                        let fx = gx + px;
                        let fy = gy + py;
                        if fx >= 0 && fx < canvas_w as i32 && fy >= 0 && fy < canvas_h as i32 {
                            let pixel = img.get_pixel_mut(fx as u32, fy as u32);
                            let bg = pixel.0;
                            pixel.0 = [
                                color_utils::blend(bg[0], cr, alpha),
                                color_utils::blend(bg[1], cg, alpha),
                                color_utils::blend(bg[2], cb, alpha),
                                255,
                            ];
                        }
                    }
                }
            }
        }
    }
    fn text_fill_color(&self, text: &NativeTextModel) -> Color {
        color_utils::parse_hex_vello_color(
            if text.color.is_empty() {
                "#000000"
            } else {
                &text.color
            },
            text.alpha,
        )
    }
    fn text_stroke_color(&self, text: &NativeTextModel) -> Color {
        let fallback = if text.color.is_empty() {
            "#000000"
        } else {
            &text.color
        };
        color_utils::parse_hex_vello_color(
            text.stroke_color.as_deref().unwrap_or(fallback),
            text.alpha,
        )
    }
    fn text_stroke_width(&self, text: &NativeTextModel) -> f64 {
        let width = if text.stroke_width > 0.0 {
            text.stroke_width as f64
        } else {
            (text.font_size.max(1.0) * 0.02) as f64
        };
        width.max(0.1)
    }
    fn paint_text_outline(
        &self,
        scene: &mut Scene,
        mut path: BezPath,
        transform: Affine,
        text: &NativeTextModel,
    ) -> bool {
        if text_is_non_painting(text.rendering_mode) {
            return true;
        }

        path.apply_affine(transform);

        let mut painted = false;
        if text_fill_enabled(text.rendering_mode) {
            scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                self.text_fill_color(text),
                None,
                &path,
            );
            painted = true;
        }
        if text_stroke_enabled(text.rendering_mode) {
            scene.stroke(
                &Stroke::new(self.text_stroke_width(text)),
                Affine::IDENTITY,
                self.text_stroke_color(text),
                None,
                &path,
            );
            painted = true;
        }

        painted
    }
    fn raw_outline_transform(
        &self,
        flip_y: Affine,
        baseline_x: f32,
        baseline_y: f32,
        font_size: f32,
        units_per_em: f64,
    ) -> Affine {
        let scale = if units_per_em > 0.0 {
            font_size as f64 / units_per_em
        } else {
            1.0
        };
        flip_y
            * Affine::translate(Vec2::new(baseline_x as f64, baseline_y as f64))
            * Affine::scale(scale)
    }

    /// GPU render paths/images via Vello, returns raw RGBA pixel data
    fn perform_vello_render_raw(
        &mut self,
        scene: &Scene,
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>, String> {
        if width == 0 || height == 0 {
            return Err("Cannot render 0-sized image".to_string());
        }

        let max_dimension = self.device.limits().max_texture_dimension_2d;
        if width > max_dimension || height > max_dimension {
            return Err(format!(
                "Requested render target {}x{} exceeds GPU texture limit {}. Reduce zoom or switch to tiled rendering.",
                width, height, max_dimension
            ));
        }

        let texture_desc = TextureDescriptor {
            label: Some("Vello Target Texture"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::RENDER_ATTACHMENT
                | TextureUsages::COPY_SRC
                | TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        };
        let texture = self.device.create_texture(&texture_desc);
        let view = texture.create_view(&Default::default());

        self.renderer
            .render_to_texture(
                &self.device,
                &self.queue,
                &scene,
                &view,
                &vello::RenderParams {
                    base_color: Color::WHITE,
                    width,
                    height,
                    antialiasing_method: vello::AaConfig::Msaa16,
                },
            )
            .map_err(|e| e.to_string())?;

        let unpadded_bytes_per_row = width * 4;
        let align = 256u32;
        let padded_bytes_per_row = (unpadded_bytes_per_row + align - 1) / align * align;
        let buffer_size = (padded_bytes_per_row * height) as wgpu::BufferAddress;

        let buffer = self.device.create_buffer(&BufferDescriptor {
            label: Some("Vello Output Buffer"),
            size: buffer_size,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &buffer,
                layout: ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(Some(encoder.finish()));

        let slice = buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(MapMode::Read, move |v| tx.send(v).unwrap());
        self.device.poll(wgpu::Maintain::Wait);
        rx.recv().unwrap().map_err(|e| e.to_string())?;

        let padded_data = slice.get_mapped_range();

        let data = if padded_bytes_per_row != unpadded_bytes_per_row {
            let mut unpadded = Vec::with_capacity((unpadded_bytes_per_row * height) as usize);
            for row in 0..height {
                let start = (row * padded_bytes_per_row) as usize;
                let end = start + unpadded_bytes_per_row as usize;
                unpadded.extend_from_slice(&padded_data[start..end]);
            }
            unpadded
        } else {
            padded_data.to_vec()
        };
        drop(padded_data);

        Ok(data)
    }

    /// Render text as sharp vector paths using swash outlines.
    fn draw_text_vector(
        &mut self,
        scene: &mut Scene,
        scale_context: &mut ScaleContext,
        text: &NativeTextModel,
        flip_y: Affine,
    ) -> bool {
        let resolved_font = self.resolve_pdf_font(text);
        if should_trace_text_render(text) {
            crate::pdf_log!(
                3,
                "[PDF-TEXT-PLAN] text='{}' request='{}' resolved_family={:?} preferred={:?} can_embedded={} key={:?} render_mode={} stroke_width={} stroke_color={:?}",
                preview_text(&text.text),
                text.font_name,
                resolved_font.matched_family,
                resolved_font.preferred_render_kind,
                resolved_font.can_attempt_embedded_render,
                text.embedded_font_key,
                text.rendering_mode,
                text.stroke_width,
                text.stroke_color
            );
        }
        if self.draw_embedded_text_vector(scene, scale_context, text, &resolved_font, flip_y) {
            if should_trace_text_render(text) {
                crate::pdf_log!(
                    3,
                    "[FONT-MATCH] REQ: '{}' | MATCHED: 'EMBEDDED({})' | RENDER_MODE: {} | TEXT: '{}'",
                    text.font_name,
                    text.embedded_font_key.as_deref().unwrap_or("unknown"),
                    text.rendering_mode,
                    if text.text.len() > 10 {
                        format!("{}...", &text.text[..10])
                    } else {
                        text.text.clone()
                    }
                );
            }
            return true;
        }
        if text_is_non_painting(text.rendering_mode) {
            return true;
        }

        let has_suspect = text.text.chars().any(|c| c as u32 > 0x7F);
        if has_suspect {
            crate::pdf_log!(
                3,
                "[COSMIC-FALLBACK] font='{}' subtype={:?} has_cmap={} text={:?} codepoints={:?}",
                text.font_name,
                text.font_subtype,
                text.has_to_unicode_cmap,
                &text.text,
                text.text
                    .chars()
                    .map(|c| format!("U+{:04X}", c as u32))
                    .collect::<Vec<_>>(),
            );
        }

        let real_font_size = if text.scale_y.abs() > 1.0 {
            text.scale_y.abs()
        } else {
            text.font_size
        };
        let metrics = Metrics::new(real_font_size, real_font_size * 1.2);

        let mut attrs = cosmic_text::Attrs::new();
        if text.is_bold {
            attrs = attrs.weight(cosmic_text::Weight::BOLD);
        }
        if text.is_italic {
            attrs = attrs.style(cosmic_text::Style::Italic);
        }
        attrs = attrs.family(self.resolve_cosmic_family(text, &resolved_font));

        let mut buffer = Buffer::new(&mut self.font_system, metrics);
        buffer.set_text(&mut self.font_system, &text.text, attrs, Shaping::Advanced);
        buffer.shape_until_scroll(&mut self.font_system, false);

        let layout_runs: Vec<_> = buffer.layout_runs().collect();
        let mut matched_font = "Unknown".to_string();
        for run in &layout_runs {
            for glyph in run.glyphs {
                if let Some(font) = self.font_system.db().face(glyph.font_id) {
                    matched_font = format!("{:?} {:?}", font.families, font.weight);
                    break;
                }
            }
            if matched_font != "Unknown" {
                break;
            }
        }
        if should_trace_text_render(text) {
            crate::pdf_log!(
                3,
                "[FONT-MATCH] REQ: '{}' | MATCHED: '{}' | RENDER_MODE: {} | TEXT: '{}'",
                text.font_name,
                matched_font,
                text.rendering_mode,
                if text.text.len() > 10 {
                    format!("{}...", &text.text[..10])
                } else {
                    text.text.clone()
                }
            );
        }
        let mut drew_any_glyph = false;

        for run in layout_runs {
            for glyph in run.glyphs {
                let font_id = glyph.font_id;
                if let Some(font_face) = self.font_system.db().face(font_id) {
                    let index = font_face.index;
                    let source = &font_face.source;

                    let data_arc_binary;
                    let data_arc_file;

                    let data: &[u8] = match source {
                        cosmic_text::fontdb::Source::Binary(arc) => {
                            data_arc_binary = Some(arc.clone());
                            data_arc_binary.as_ref().unwrap().as_ref().as_ref()
                        }
                        cosmic_text::fontdb::Source::File(path) => {
                            let cache_entry = self.font_file_cache.get(path).cloned();
                            let data_arc = match cache_entry {
                                Some(arc) => arc,
                                None => {
                                    if let Ok(bytes) = std::fs::read(path) {
                                        let arc = Arc::new(bytes);
                                        // Cache it
                                        // Wait, self is not mut in this context but self is mut in draw_text_vector
                                        // self.font_file_cache.insert(path.clone(), arc.clone());
                                        arc
                                    } else {
                                        continue;
                                    }
                                }
                            };
                            data_arc_file = Some(data_arc);
                            data_arc_file.as_ref().unwrap().as_slice()
                        }
                        _ => continue,
                    };

                    if let Some(font_ref) = swash::FontRef::from_index(data, index as usize) {
                        let units_per_em = MetricsProxy::from_font(&font_ref).units_per_em() as f64;
                        let mut scaler = scale_context.builder(font_ref).hint(false).build();
                        let Some(outline) = scaler.scale_outline(glyph.glyph_id) else {
                            continue;
                        };

                        let bez_path = path_utils::outline_to_bez_path(&outline);

                        let final_transform = self.raw_outline_transform(
                            flip_y,
                            text.tx + glyph.x,
                            text.ty,
                            real_font_size,
                            units_per_em,
                        );

                        if self.paint_text_outline(scene, bez_path, final_transform, text) {
                            drew_any_glyph = true;
                        }
                    }
                }
            }
        }

        drew_any_glyph
    }
}

pub(super) fn preview_text(text: &str) -> String {
    const LIMIT: usize = 16;
    let mut chars = text.chars();
    let preview: String = chars.by_ref().take(LIMIT).collect();
    if chars.next().is_some() {
        format!("{}...", preview)
    } else {
        preview
    }
}
