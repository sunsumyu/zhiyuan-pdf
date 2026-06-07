use crate::infrastructure::pdf::models::{NativeTextModel, RenderObject};
use cosmic_text::{Buffer, FontSystem, Metrics, Shaping, SwashCache};
use image::{ImageBuffer, Rgba};
use pdf_viewer_core::typography::models::ResolvedPdfFont;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use swash::proxy::MetricsProxy;
use swash::scale::ScaleContext;
use vello::kurbo::{Affine, BezPath, Point, Stroke, Vec2};
use vello::peniko::{Color, Fill};
use vello::{Renderer, RendererOptions, Scene};
use wgpu::{
    Backends, BufferDescriptor, BufferUsages, DeviceDescriptor, Extent3d, ImageCopyTexture,
    ImageDataLayout, Instance, InstanceDescriptor, Limits, MapMode, PowerPreference,
    RequestAdapterOptions, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
};
pub struct VelloRenderer {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    renderer: Renderer,
    font_system: FontSystem,
    swash_cache: SwashCache,
    font_file_cache: HashMap<std::path::PathBuf, Arc<Vec<u8>>>,
    font_matcher: crate::infrastructure::pdf::font::matching::PdfSystemFontMatcher,
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

        // 鈹€鈹€ Standard PDF Coordinate Transform (Flip Y + Zoom) 鈹€鈹€
        let flip_y = Affine::scale_non_uniform(zoom as f64, -zoom as f64)
            .then_translate(vello::kurbo::Vec2::new(0.0, height as f64));

        for object in objects {
            match object {
                RenderObject::Path(path) => {
                    let mut bez_path = BezPath::new();
                    for seg in &path.segments {
                        match seg.command.as_str() {
                            "move" => {
                                if let Some(p) = seg.points.get(0) {
                                    bez_path.move_to(Point::new(p[0] as f64, p[1] as f64));
                                }
                            }
                            "line" => {
                                if let Some(p) = seg.points.get(0) {
                                    bez_path.line_to(Point::new(p[0] as f64, p[1] as f64));
                                }
                            }
                            "bezier" => {
                                if seg.points.len() == 3 {
                                    bez_path.curve_to(
                                        Point::new(
                                            seg.points[0][0] as f64,
                                            seg.points[0][1] as f64,
                                        ),
                                        Point::new(
                                            seg.points[1][0] as f64,
                                            seg.points[1][1] as f64,
                                        ),
                                        Point::new(
                                            seg.points[2][0] as f64,
                                            seg.points[2][1] as f64,
                                        ),
                                    );
                                }
                            }
                            "close" => bez_path.close_path(),
                            _ => {}
                        }
                    }

                    if path.fill {
                        let color = parse_hex_vello_color(
                            path.fill_color.as_deref().unwrap_or("#000000"),
                            path.alpha,
                        );
                        scene.fill(Fill::NonZero, flip_y, color, None, &bez_path);
                    }
                    if path.stroke {
                        let color = parse_hex_vello_color(
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

        // 鈹€鈹€ Phase 2: CPU overlay for text + legacy images 鈹€鈹€
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

        // 鈹€鈹€ Phase 3: Encode to PNG 鈹€鈹€
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
        // 1. Extract asset ID from local URL
        let asset_id = if let Some(id) = model.data_url.strip_prefix("http://pdfasset.localhost/") {
            id
        } else {
            return;
        };

        // 2. Fetch from cache
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

        // 3. Decode image
        let src_img = match image::load_from_memory(&raw_bytes) {
            Ok(i) => i.to_rgba8(),
            Err(_) => return,
        };

        // 4. Calculate target rectangle (PDF -> Canvas)
        // PDF: y-up. Canvas: y-down.
        let target_w = (model.width * zoom).abs() as u32;
        let target_h = (model.height * zoom).abs() as u32;

        if target_w == 0 || target_h == 0 {
            return;
        }

        // Use the transformation matrix (a, b, c, d, e, f) or simplified x, y
        // For simple icons, x/y + width/height is usually enough
        let target_x = (model.x * zoom) as i32;
        let target_y = (canvas_h as f32 - (model.y + model.height) * zoom) as i32;

        // 5. Resize and composite
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
                    } // Skip transparent

                    let dst_pixel = img.get_pixel_mut(fx as u32, fy as u32);
                    let alpha = src_pixel[3] as f32 / 255.0;

                    dst_pixel.0 = [
                        blend(dst_pixel[0], src_pixel[0], alpha),
                        blend(dst_pixel[1], src_pixel[1], alpha),
                        blend(dst_pixel[2], src_pixel[2], alpha),
                        255,
                    ];
                }
            }
        }
    }

    /// CPU text rendering using cosmic_text's SwashCache for glyph rasterization.
    /// This bypasses Vello entirely for text, using proven pixel-based rasterization
    /// that correctly handles CJK fonts via cosmic_text's font fallback.
    /// [DEPRECATED] Hardware rasterization was producing fuzzy/thin text.
    /// Now weuse draw_text_vector instead.
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
        // --- DYNAMIC FONT SIZE: Handle matrix scale (scale_y) ---
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
        // 1. Configure attributes based on PDF metadata.
        // PDF render mode / faux-bold are paint semantics, not font-face selection hints.
        let mut attrs = cosmic_text::Attrs::new();
        if text.is_bold {
            attrs = attrs.weight(cosmic_text::Weight::BOLD);
        }
        if text.is_italic {
            attrs = attrs.style(cosmic_text::Style::Italic);
        }
        let resolved_font = self.resolve_pdf_font(text);
        attrs = attrs.family(self.resolve_cosmic_family(text, &resolved_font));

        // 2. Parse text color
        let (cr, cg, cb) = parse_hex_color_rgb(if text.color.is_empty() {
            "#000000"
        } else {
            &text.color
        });

        // 3. WHOLE-LINE RENDERING: Restores Hinting and "Solid" Contrast
        let mut buffer = Buffer::new(&mut self.font_system, metrics);
        buffer.set_text(&mut self.font_system, &text.text, attrs, Shaping::Advanced);
        buffer.shape_until_scroll(&mut self.font_system, false);

        // --- DIAGNOSTIC LOGGING ---
        let mut matched_font = "Unknown".to_string();
        for run in buffer.layout_runs() {
            for glyph in run.glyphs {
                if let Some(font) = self.font_system.db().face(glyph.font_id) {
                    matched_font = format!("{:?} {:?}", font.families, font.weight);
                    break;
                }
            }
        }
        if text.text.contains("绠€") || text.font_size > 20.0 {
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

        // --- POSITIONING FIX: PDF tx/ty are absolute baseline origins ---
        let base_x = text.tx * zoom;
        let base_y = canvas_h as f32 - text.ty * zoom; // Exact PDF Baseline

        for run in buffer.layout_runs() {
            // Glyph 'y' is relative to the baseline.
            // In cosmic-text, for a single-line buffer, the baseline offset was already handled during shaping.
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
                            // Solid composite (avoid multiple passes if possible, but keep alpha for antialiasing)
                            pixel.0 = [
                                blend(bg[0], cr, alpha),
                                blend(bg[1], cg, alpha),
                                blend(bg[2], cb, alpha),
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
                                blend(bg[0], sr, sa),
                                blend(bg[1], sg, sa),
                                blend(bg[2], sb, sa),
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
                                blend(bg[0], cr, alpha),
                                blend(bg[1], cg, alpha),
                                blend(bg[2], cb, alpha),
                                255,
                            ];
                        }
                    }
                }
            }
        }
    }
    fn text_fill_color(&self, text: &NativeTextModel) -> Color {
        parse_hex_vello_color(
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
        parse_hex_vello_color(text.stroke_color.as_deref().unwrap_or(fallback), text.alpha)
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

        // wgpu requires bytes_per_row aligned to COPY_BYTES_PER_ROW_ALIGNMENT (256)
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

        // Strip row padding if needed
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
}

/// Alpha blend: result = bg * (1 - alpha) + fg * alpha
#[inline]
fn blend(bg: u8, fg: u8, alpha: f32) -> u8 {
    ((bg as f32 * (1.0 - alpha)) + (fg as f32 * alpha)) as u8
}
fn parse_hex_color_rgb(hex: &str) -> (u8, u8, u8) {
    if hex.len() < 7 {
        return (0, 0, 0);
    }
    let r = u8::from_str_radix(&hex[1..3], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[3..5], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[5..7], 16).unwrap_or(0);
    (r, g, b)
}
fn parse_hex_vello_color(hex: &str, alpha: f32) -> Color {
    let (r, g, b) = parse_hex_color_rgb(hex);
    Color::rgba8(r, g, b, (alpha * 255.0) as u8)
}
fn text_fill_enabled(render_mode: i32) -> bool {
    matches!(render_mode, 0 | 2 | 4 | 6)
}
fn text_stroke_enabled(render_mode: i32) -> bool {
    matches!(render_mode, 1 | 2 | 5 | 6)
}
fn text_is_non_painting(render_mode: i32) -> bool {
    matches!(render_mode, 3 | 7)
}
impl VelloRenderer {
    /// Render text as sharp vector paths using swash outlines.
    /// [DEFINITIVE FIX] This implementation correctly normalizes Font Units to Pixels.
    fn draw_text_vector(
        &mut self,
        scene: &mut Scene,
        scale_context: &mut ScaleContext,
        text: &NativeTextModel,
        flip_y: Affine,
    ) -> bool {
        let resolved_font = self.resolve_pdf_font(text);
        if text.text.contains("绠€") || text.font_size > 20.0 {
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
            if text.text.contains("绠€") || text.font_size > 20.0 {
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

        // Stage-3 trace: embedded render failed, falling back to cosmic_text
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
        if text.text.contains("绠€") || text.font_size > 20.0 {
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
                            if let Some(arc) = self.font_file_cache.get(path) {
                                data_arc_file = Some(arc.clone());
                                data_arc_file.as_ref().unwrap().as_ref()
                            } else {
                                match std::fs::read(path) {
                                    Ok(bytes) => {
                                        let arc = Arc::new(bytes);
                                        self.font_file_cache.insert(path.clone(), arc.clone());
                                        data_arc_file = Some(arc);
                                        data_arc_file.as_ref().unwrap().as_ref()
                                    }
                                    Err(_) => continue,
                                }
                            }
                        }
                        _ => continue,
                    };

                    let font_ref = swash::FontRef::from_index(data, index as usize).unwrap();
                    let units_per_em = MetricsProxy::from_font(&font_ref).units_per_em() as f64;
                    if units_per_em <= 0.0 {
                        continue;
                    }
                    let mut scaler = scale_context.builder(font_ref).hint(false).build();

                    if let Some(outline) = scaler.scale_outline(glyph.glyph_id) {
                        let mut bez_path = BezPath::new();
                        let mut points = outline.points().iter();
                        for verb in outline.verbs() {
                            use swash::zeno::Verb;
                            match verb {
                                Verb::MoveTo => {
                                    if let Some(p) = points.next() {
                                        bez_path.move_to(Point::new(p.x as f64, p.y as f64));
                                    }
                                }
                                Verb::LineTo => {
                                    if let Some(p) = points.next() {
                                        bez_path.line_to(Point::new(p.x as f64, p.y as f64));
                                    }
                                }
                                Verb::QuadTo => {
                                    if let (Some(c), Some(p)) = (points.next(), points.next()) {
                                        bez_path.quad_to(
                                            Point::new(c.x as f64, c.y as f64),
                                            Point::new(p.x as f64, p.y as f64),
                                        );
                                    }
                                }
                                Verb::CurveTo => {
                                    if let (Some(c1), Some(c2), Some(p)) =
                                        (points.next(), points.next(), points.next())
                                    {
                                        bez_path.curve_to(
                                            Point::new(c1.x as f64, c1.y as f64),
                                            Point::new(c2.x as f64, c2.y as f64),
                                            Point::new(p.x as f64, p.y as f64),
                                        );
                                    }
                                }
                                Verb::Close => bez_path.close_path(),
                            }
                        }
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
    fn resolve_pdf_font(&mut self, text: &NativeTextModel) -> ResolvedPdfFont {
        self.font_matcher.resolve_native_text(text)
    }
    fn draw_embedded_text_vector(
        &mut self,
        scene: &mut Scene,
        scale_context: &mut ScaleContext,
        text: &NativeTextModel,
        resolved_font: &ResolvedPdfFont,
        flip_y: Affine,
    ) -> bool {
        if text_is_non_painting(text.rendering_mode) {
            return true;
        }
        if !resolved_font.can_attempt_embedded_render {
            if text.text.contains("绠€") || text.font_size > 20.0 {
                println!(
                    "[PDF-EMBEDDED] skip can_attempt=false text='{}' font='{}' key={:?} subtype={:?}",
                    preview_text(&text.text),
                    text.font_name,
                    text.embedded_font_key,
                    text.font_subtype
                );
            }
            return false;
        }

        let Some(font_key) = text.embedded_font_key.as_deref() else {
            println!(
                "[PDF-EMBEDDED] skip missing-key text='{}' font='{}' subtype={:?}",
                preview_text(&text.text),
                text.font_name,
                text.font_subtype
            );
            return false;
        };
        let font_bytes = {
            let cache = crate::infrastructure::pdf::cache::PDF_FONT_PROGRAM_CACHE
                .lock()
                .unwrap();
            cache.get(font_key).cloned()
        };
        let Some(font_bytes) = font_bytes else {
            println!(
                "[PDF-EMBEDDED] skip missing-cache-entry key='{}' text='{}' font='{}'",
                font_key,
                preview_text(&text.text),
                text.font_name
            );
            return false;
        };

        let Some(font_ref) = swash::FontRef::from_index(font_bytes.as_slice(), 0) else {
            println!(
                "[PDF-EMBEDDED] skip invalid-font-ref key='{}' text='{}' font='{}'",
                font_key,
                preview_text(&text.text),
                text.font_name
            );
            return false;
        };
        let units_per_em = MetricsProxy::from_font(&font_ref).units_per_em() as f64;
        if units_per_em <= 0.0 {
            println!(
                "[PDF-EMBEDDED] skip invalid-upem key='{}' text='{}' font='{}'",
                font_key,
                preview_text(&text.text),
                text.font_name
            );
            return false;
        }

        let glyph_positions = self.build_embedded_glyph_positions(text);
        if glyph_positions.is_empty() {
            println!(
                "[PDF-EMBEDDED] skip no-glyph-positions text='{}' font='{}' char_origins={} char_widths={} codes={}",
                preview_text(&text.text),
                text.font_name,
                text.char_origins.len(),
                text.char_widths.len(),
                text.pdf_char_codes.len()
            );
            return false;
        }

        let real_font_size = if text.scale_y.abs() > 1.0 {
            text.scale_y.abs()
        } else {
            text.font_size
        };
        let mut scaler = scale_context.builder(font_ref).hint(false).build();

        let mut drew_any_glyph = false;
        for (index, (baseline_x, baseline_y)) in glyph_positions.into_iter().enumerate() {
            let glyph_id = self.resolve_embedded_glyph_id(text, &font_ref, index);
            if glyph_id == 0 {
                continue;
            }
            let Some(outline) = scaler.scale_outline(glyph_id) else {
                continue;
            };

            let mut bez_path = BezPath::new();
            let mut points = outline.points().iter();
            for verb in outline.verbs() {
                use swash::zeno::Verb;
                match verb {
                    Verb::MoveTo => {
                        if let Some(p) = points.next() {
                            bez_path.move_to(Point::new(p.x as f64, p.y as f64));
                        }
                    }
                    Verb::LineTo => {
                        if let Some(p) = points.next() {
                            bez_path.line_to(Point::new(p.x as f64, p.y as f64));
                        }
                    }
                    Verb::QuadTo => {
                        if let (Some(c), Some(p)) = (points.next(), points.next()) {
                            bez_path.quad_to(
                                Point::new(c.x as f64, c.y as f64),
                                Point::new(p.x as f64, p.y as f64),
                            );
                        }
                    }
                    Verb::CurveTo => {
                        if let (Some(c1), Some(c2), Some(p)) =
                            (points.next(), points.next(), points.next())
                        {
                            bez_path.curve_to(
                                Point::new(c1.x as f64, c1.y as f64),
                                Point::new(c2.x as f64, c2.y as f64),
                                Point::new(p.x as f64, p.y as f64),
                            );
                        }
                    }
                    Verb::Close => bez_path.close_path(),
                }
            }

            let final_transform = self.raw_outline_transform(
                flip_y,
                baseline_x,
                baseline_y,
                real_font_size,
                units_per_em,
            );

            if self.paint_text_outline(scene, bez_path, final_transform, text) {
                drew_any_glyph = true;
            }
        }

        if !drew_any_glyph {
            println!(
                "[PDF-EMBEDDED] skip no-outlines text='{}' font='{}' key='{}' subtype={:?} codes={:?}",
                preview_text(&text.text),
                text.font_name,
                font_key,
                text.font_subtype,
                text.pdf_char_codes
            );
        } else if text.text.contains("绠€") || text.font_size > 20.0 {
            println!(
                "[PDF-EMBEDDED] success text='{}' font='{}' key='{}' subtype={:?} codes={:?}",
                preview_text(&text.text),
                text.font_name,
                font_key,
                text.font_subtype,
                text.pdf_char_codes
            );
        }

        drew_any_glyph
    }
    fn build_embedded_glyph_positions(&self, text: &NativeTextModel) -> Vec<(f32, f32)> {
        let glyph_count = self.embedded_glyph_count(text);
        if glyph_count == 0 {
            return Vec::new();
        }

        if text.char_origins.len() == glyph_count {
            return text
                .char_origins
                .iter()
                .map(|origin| (origin[0], origin[1]))
                .collect();
        }

        if text.char_widths.len() == glyph_count {
            let mut positions = Vec::with_capacity(glyph_count);
            let mut current_x = text.tx;
            for width in &text.char_widths {
                positions.push((current_x, text.ty));
                current_x += *width;
            }
            return positions;
        }

        if glyph_count == 1 {
            return vec![(text.tx, text.ty)];
        }

        Vec::new()
    }
    fn embedded_glyph_count(&self, text: &NativeTextModel) -> usize {
        if !text.pdf_char_codes.is_empty() {
            return text.pdf_char_codes.len();
        }
        text.text.chars().count()
    }
    fn resolve_embedded_glyph_id(
        &self,
        text: &NativeTextModel,
        font_ref: &swash::FontRef<'_>,
        glyph_index: usize,
    ) -> u16 {
        let ch_for_log = text.text.chars().nth(glyph_index);
        let raw_code_for_log = text.pdf_char_codes.get(glyph_index).copied();
        let is_suspect = ch_for_log.map(|c| c as u32 > 0x7F).unwrap_or(false);

        if let Some(raw_code) = text.pdf_char_codes.get(glyph_index).copied() {
            if let Some(mapped) = self.resolve_cached_cid_glyph_id(text, raw_code) {
                if is_suspect {
                    crate::pdf_log!(
                        3,
                        "[GLYPH-RESOLVE] font='{}' idx={} raw=0x{:04X} ch={:?}(U+{:04X}) -> CID_MAP gid={}",
                        text.font_name, glyph_index, raw_code,
                        ch_for_log, ch_for_log.map(|c| c as u32).unwrap_or(0), mapped
                    );
                }
                return mapped;
            }
            let charmap_gid = font_ref.charmap().map(raw_code);
            if charmap_gid != 0 {
                if is_suspect {
                    crate::pdf_log!(
                        3,
                        "[GLYPH-RESOLVE] font='{}' idx={} raw=0x{:04X} ch={:?}(U+{:04X}) -> RAW_CHARMAP gid={}",
                        text.font_name, glyph_index, raw_code,
                        ch_for_log, ch_for_log.map(|c| c as u32).unwrap_or(0), charmap_gid
                    );
                }
                return charmap_gid;
            }
        }

        if let Some(ch) = text.text.chars().nth(glyph_index) {
            if !ch.is_control() && !ch.is_whitespace() {
                let glyph_id = font_ref.charmap().map(ch);
                if glyph_id != 0 {
                    if is_suspect {
                        crate::pdf_log!(
                            3,
                            "[GLYPH-RESOLVE] font='{}' idx={} raw={:?} ch={:?}(U+{:04X}) -> UNICODE_CHARMAP gid={}",
                            text.font_name, glyph_index, raw_code_for_log, ch, ch as u32, glyph_id
                        );
                    }
                    return glyph_id;
                }
            }
        }

        if let Some(raw_code) = text.pdf_char_codes.get(glyph_index).copied() {
            if self.prefers_pdf_code_glyph_mapping(text)
                && raw_code > 0
                && raw_code <= u16::MAX as u32
            {
                if is_suspect {
                    crate::pdf_log!(
                        3,
                        "[GLYPH-RESOLVE] font='{}' idx={} raw=0x{:04X} ch={:?}(U+{:04X}) -> DIRECT_CODE gid={}",
                        text.font_name, glyph_index, raw_code,
                        ch_for_log, ch_for_log.map(|c| c as u32).unwrap_or(0), raw_code as u16
                    );
                }
                return raw_code as u16;
            }
        }

        if is_suspect {
            crate::pdf_log!(
                3,
                "[GLYPH-RESOLVE] font='{}' idx={} raw={:?} ch={:?}(U+{:04X}) -> FAILED gid=0 (will skip or fallback to cosmic)",
                text.font_name, glyph_index, raw_code_for_log,
                ch_for_log, ch_for_log.map(|c| c as u32).unwrap_or(0)
            );
        }
        0
    }
    fn resolve_cached_cid_glyph_id(&self, text: &NativeTextModel, raw_code: u32) -> Option<u16> {
        let font_key = text.embedded_font_key.as_deref()?;
        let cache = crate::infrastructure::pdf::cache::PDF_FONT_GLYPH_MAP_CACHE
            .lock()
            .ok()?;
        let glyph_map = cache.get(font_key)?;

        if let Some(gid) = glyph_map.cid_to_gid.get(&raw_code).copied() {
            return Some(gid);
        }
        if glyph_map.identity && raw_code > 0 && raw_code <= u16::MAX as u32 {
            return Some(raw_code as u16);
        }

        None
    }
    fn prefers_pdf_code_glyph_mapping(&self, text: &NativeTextModel) -> bool {
        let Some(subtype) = text.font_subtype.as_deref() else {
            return false;
        };
        let lower = subtype.trim().trim_start_matches('/').to_ascii_lowercase();
        matches!(lower.as_str(), "truetype" | "opentype" | "type1")
    }
    fn resolve_cosmic_family<'a>(
        &self,
        text: &NativeTextModel,
        resolved_font: &'a ResolvedPdfFont,
    ) -> cosmic_text::Family<'a> {
        if let Some(matched_family) = resolved_font.matched_family.as_deref() {
            return cosmic_text::Family::Name(matched_family);
        }

        if text
            .font_hints
            .as_ref()
            .map(|value| value.is_serif)
            .unwrap_or(false)
            || text.font_name.to_ascii_lowercase().contains("serif")
            || text.font_name.to_ascii_lowercase().contains("roman")
        {
            return cosmic_text::Family::Serif;
        }

        cosmic_text::Family::SansSerif
    }
}
fn preview_text(text: &str) -> String {
    const LIMIT: usize = 16;
    let mut chars = text.chars();
    let preview: String = chars.by_ref().take(LIMIT).collect();
    if chars.next().is_some() {
        format!("{}...", preview)
    } else {
        preview
    }
}
