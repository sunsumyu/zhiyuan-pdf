use crate::infrastructure::pdf::color::{blend, parse_rgb, parse_vello};
use crate::infrastructure::pdf::glyph_mapping;
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
                    let bez_path = path_segments_to_bez_path(&path.segments);

                    if path.fill {
                        let color = parse_vello(
                            path.fill_color.as_deref().unwrap_or("#000000"),
                            path.alpha,
                        );
                        scene.fill(Fill::NonZero, flip_y, color, None, &bez_path);
                    }
                    if path.stroke {
                        let color = parse_vello(
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
                    self.draw_text_bitmap_deprecated(&mut img, text_model, zoom, height);
                }
                RenderObject::Image(image_model) => {
                    self.draw_image_cpu(&mut img, image_model, zoom, height);
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

        let (rw, rh) = (resized.width() as i32, resized.height() as i32);
        blend_span(
            img,
            target_x,
            target_y,
            rw,
            rh,
            SpanSource::Rgba {
                data: resized.as_raw(),
            },
            [0, 0, 0],
        );
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
        canvas_h: u32,
    ) {
        if text_is_non_painting(text.rendering_mode) {
            return;
        }
        // --- DYNAMIC FONT SIZE: Handle matrix scale (scale_y) ---
        let font_size = glyph_mapping::real_font_size(text) * zoom;

        let metrics = Metrics::new(
            font_size,
            if font_size > 0.1 {
                font_size * 1.2
            } else {
                1.0
            },
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
        let (cr, cg, cb) = parse_rgb(if text.color.is_empty() {
            "#000000"
        } else {
            &text.color
        });

        // 3. WHOLE-LINE RENDERING: Restores Hinting and "Solid" Contrast
        let mut buffer = Buffer::new(&mut self.font_system, metrics);
        buffer.set_text(&mut self.font_system, &text.text, attrs, Shaping::Advanced);
        buffer.shape_until_scroll(&mut self.font_system, false);

        // --- DIAGNOSTIC LOGGING ---
        let matched_font = self.first_matched_font_name(&buffer);
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
                    self.composite_glyph(img, &glyph_img, gx, gy, cr, cg, cb);
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
    ) {
        let gw = glyph_img.placement.width as i32;
        let gh = glyph_img.placement.height as i32;
        if gw == 0 || gh == 0 {
            return;
        }

        let tint = [cr, cg, cb];
        match glyph_img.content {
            cosmic_text::SwashContent::Mask => blend_span(
                img,
                gx,
                gy,
                gw,
                gh,
                SpanSource::AlphaMask {
                    data: &glyph_img.data,
                },
                tint,
            ),
            cosmic_text::SwashContent::Color => blend_span(
                img,
                gx,
                gy,
                gw,
                gh,
                SpanSource::Rgba {
                    data: &glyph_img.data,
                },
                tint,
            ),
            cosmic_text::SwashContent::SubpixelMask => blend_span(
                img,
                gx,
                gy,
                gw,
                gh,
                SpanSource::SubpixelMask {
                    data: &glyph_img.data,
                },
                tint,
            ),
        }
    }
    fn text_fill_color(&self, text: &NativeTextModel) -> Color {
        parse_vello(
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
        parse_vello(text.stroke_color.as_deref().unwrap_or(fallback), text.alpha)
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

/// Source pixel layout for [`blend_span`].
enum SpanSource<'a> {
    /// 8-bit alpha coverage, one byte per pixel; tinted with a constant color.
    AlphaMask { data: &'a [u8] },
    /// RGB subpixel coverage, three bytes per pixel; alpha is the G channel.
    SubpixelMask { data: &'a [u8] },
    /// RGBA8, four bytes per pixel; per-pixel color and alpha.
    Rgba { data: &'a [u8] },
}

impl SpanSource<'_> {
    /// `(alpha, color)` of the pixel at `(px, py)` in a span `w` pixels wide,
    /// or `None` past the end of the source data.
    fn pixel(&self, px: i32, py: i32, w: i32, tint: [u8; 3]) -> Option<(f32, [u8; 3])> {
        match *self {
            SpanSource::AlphaMask { data } => {
                let alpha = *data.get((py * w + px) as usize)? as f32 / 255.0;
                Some((alpha, tint))
            }
            SpanSource::SubpixelMask { data } => {
                let idx = ((py * w + px) * 3) as usize;
                let g = *data.get(idx + 1)?;
                Some((g as f32 / 255.0, tint))
            }
            SpanSource::Rgba { data } => {
                let idx = ((py * w + px) * 4) as usize;
                let r = *data.get(idx)?;
                let g = *data.get(idx + 1)?;
                let b = *data.get(idx + 2)?;
                let a = *data.get(idx + 3)?;
                Some((a as f32 / 255.0, [r, g, b]))
            }
        }
    }
}

/// Blend a `w` x `h` span of source pixels onto the canvas at `(dst_x, dst_y)`,
/// clipping to the canvas bounds. Near-fully-transparent pixels (alpha < 0.01)
/// are skipped; the destination alpha becomes opaque.
fn blend_span(
    img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    dst_x: i32,
    dst_y: i32,
    w: i32,
    h: i32,
    source: SpanSource<'_>,
    tint: [u8; 3],
) {
    let (canvas_w, canvas_h) = (img.width() as i32, img.height() as i32);
    for py in 0..h {
        for px in 0..w {
            let Some((alpha, color)) = source.pixel(px, py, w, tint) else {
                continue;
            };
            if alpha < 0.01 {
                continue;
            }
            let fx = dst_x + px;
            let fy = dst_y + py;
            if fx >= 0 && fx < canvas_w && fy >= 0 && fy < canvas_h {
                let pixel = img.get_pixel_mut(fx as u32, fy as u32);
                let bg = pixel.0;
                pixel.0 = [
                    blend(bg[0], color[0], alpha),
                    blend(bg[1], color[1], alpha),
                    blend(bg[2], color[2], alpha),
                    255,
                ];
            }
        }
    }
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

/// Convert a swash glyph outline into a vello/kurbo `BezPath`.
/// Shared by the embedded-font and cosmic_text-fallback render paths.
fn outline_to_bez_path(outline: &swash::scale::outline::Outline) -> BezPath {
    use swash::zeno::Verb;
    let mut bez_path = BezPath::new();
    let mut points = outline.points().iter();
    for verb in outline.verbs() {
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
    bez_path
}

/// Convert a `NativePathModel` segment list into a vello/kurbo `BezPath`.
fn path_segments_to_bez_path(segments: &[crate::infrastructure::pdf::models::PathSegment]) -> BezPath {
    let mut bez_path = BezPath::new();
    for seg in segments {
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
                        Point::new(seg.points[0][0] as f64, seg.points[0][1] as f64),
                        Point::new(seg.points[1][0] as f64, seg.points[1][1] as f64),
                        Point::new(seg.points[2][0] as f64, seg.points[2][1] as f64),
                    );
                }
            }
            "close" => bez_path.close_path(),
            _ => {}
        }
    }
    bez_path
}

/// Whether this text run warrants verbose render-path tracing.
/// Gate: known diagnostic marker or large font size.
fn should_trace_text_render(text: &NativeTextModel) -> bool {
    text.text.contains("绠€") || text.font_size > 20.0
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

        let real_font_size = glyph_mapping::real_font_size(text);
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
        let matched_font = self.first_matched_font_name(&buffer);
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
                        let bez_path = outline_to_bez_path(&outline);
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
    /// First face name (families + weight) actually used by the shaped buffer,
    /// or "Unknown" - for diagnostic logging.
    fn first_matched_font_name(&self, buffer: &Buffer) -> String {
        for run in buffer.layout_runs() {
            for glyph in run.glyphs {
                if let Some(font) = self.font_system.db().face(glyph.font_id) {
                    return format!("{:?} {:?}", font.families, font.weight);
                }
            }
        }
        "Unknown".to_string()
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
            if should_trace_text_render(text) {
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

        let glyph_positions = glyph_mapping::build_glyph_positions(text);
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

        let real_font_size = glyph_mapping::real_font_size(text);
        let mut scaler = scale_context.builder(font_ref).hint(false).build();

        let mut drew_any_glyph = false;
        for (index, (baseline_x, baseline_y)) in glyph_positions.into_iter().enumerate() {
            let glyph_id =
                glyph_mapping::resolve_glyph_id(text, index, |code| font_ref.charmap().map(code));
            if glyph_id == 0 {
                continue;
            }
            let Some(outline) = scaler.scale_outline(glyph_id) else {
                continue;
            };

                        let bez_path = outline_to_bez_path(&outline);

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
        } else if should_trace_text_render(text) {
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

#[cfg(test)]
mod blend_span_tests {
    //! These tests pin the `blend_span` compositing primitive that replaced the
    //! four duplicated bounds-check/blend loops (three glyph-content arms plus
    //! the image compositing loop).
    use super::*;

    fn canvas(w: u32, h: u32) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
        ImageBuffer::from_pixel(w, h, Rgba([200, 100, 50, 255]))
    }

    #[test]
    fn alpha_mask_tints_with_constant_color() {
        // 2x1 mask: fully opaque + fully transparent; tint = red.
        let mut img = canvas(2, 1);
        let data = [255u8, 0];
        blend_span(
            &mut img,
            0,
            0,
            2,
            1,
            SpanSource::AlphaMask { data: &data },
            [255, 0, 0],
        );
        assert_eq!(img.get_pixel(0, 0), &Rgba([255, 0, 0, 255]));
        // Transparent pixel leaves the background untouched.
        assert_eq!(img.get_pixel(1, 0), &Rgba([200, 100, 50, 255]));
    }

    #[test]
    fn rgba_uses_per_pixel_color_and_alpha() {
        // 1x1 RGBA, half-opaque green over (200,100,50).
        let mut img = canvas(1, 1);
        let data = [0, 255, 0, 128];
        blend_span(
            &mut img,
            0,
            0,
            1,
            1,
            SpanSource::Rgba { data: &data },
            [255, 0, 0],
        );
        let p = img.get_pixel(0, 0);
        let alpha = 128.0 / 255.0;
        assert_eq!(p.0[3], 255);
        assert_eq!(p.0[0], blend(200, 0, alpha));
        assert_eq!(p.0[1], blend(100, 255, alpha));
        assert_eq!(p.0[2], blend(50, 0, alpha));
    }

    #[test]
    fn subpixel_mask_takes_alpha_from_green_channel() {
        // 1x1 subpixel (r,g,b) = (10, 255, 20): alpha 1.0 -> full tint.
        let mut img = canvas(1, 1);
        let data = [10, 255, 20];
        blend_span(
            &mut img,
            0,
            0,
            1,
            1,
            SpanSource::SubpixelMask { data: &data },
            [0, 0, 255],
        );
        assert_eq!(img.get_pixel(0, 0), &Rgba([0, 0, 255, 255]));
    }

    #[test]
    fn near_transparent_pixels_are_skipped() {
        // alpha = 2/255 < 0.01 threshold: background untouched.
        let mut img = canvas(1, 1);
        let data = [255u8, 0, 0, 2];
        blend_span(
            &mut img,
            0,
            0,
            1,
            1,
            SpanSource::Rgba { data: &data },
            [255, 0, 0],
        );
        assert_eq!(img.get_pixel(0, 0), &Rgba([200, 100, 50, 255]));
    }

    #[test]
    fn fully_off_canvas_offsets_change_nothing() {
        let mut img = canvas(2, 2);
        let data = [255u8; 16];
        blend_span(
            &mut img,
            10,
            10,
            2,
            2,
            SpanSource::Rgba { data: &data },
            [0, 0, 0],
        );
        assert_eq!(img.get_pixel(0, 0), &Rgba([200, 100, 50, 255]));
        assert_eq!(img.get_pixel(1, 1), &Rgba([200, 100, 50, 255]));
    }

    #[test]
    fn partial_overlap_blends_only_inside_canvas() {
        // 3-wide span at x = -1: span pixels 1..3 land on the 2-wide canvas.
        let mut img = canvas(2, 1);
        let data = [255u8; 3];
        blend_span(
            &mut img,
            -1,
            0,
            3,
            1,
            SpanSource::AlphaMask { data: &data },
            [255, 0, 0],
        );
        assert_eq!(img.get_pixel(0, 0), &Rgba([255, 0, 0, 255]));
        assert_eq!(img.get_pixel(1, 0), &Rgba([255, 0, 0, 255]));
    }
}
