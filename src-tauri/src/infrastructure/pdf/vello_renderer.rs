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

        let mut font_system = FontSystem::new();
        font_system.db_mut().load_system_fonts();

        // Also explicitly load common CJK font files in case load_system_fonts misses them.
        let cjk_font_paths = vec![
            r"C:\Windows\Fonts\msyh.ttc",
            r"C:\Windows\Fonts\msyhbd.ttc",
            r"C:\Windows\Fonts\simsun.ttc",
            r"C:\Windows\Fonts\msjh.ttc",
            r"C:\Windows\Fonts\simhei.ttf",
            r"C:\Windows\Fonts\simfang.ttf",
            r"C:\Windows\Fonts\simkai.ttf",
            r"C:\Windows\Fonts\simli.ttf",
            r"C:\Windows\Fonts\simyou.ttf",
        ];
        for path in &cjk_font_paths {
            if std::path::Path::new(path).exists() {
                if let Ok(bytes) = std::fs::read(path) {
                    font_system.db_mut().load_font_data(bytes);
                    println!("[VELLO-FONT] Loaded CJK font binary data: {}", path);
                }
            }
        }

        // Remove symbol/icon fonts from fontdb so fallback never resolves to Marlett/Wingdings/Symbol/MDL2
        let face_ids_to_remove: Vec<_> = font_system
            .db()
            .faces()
            .filter(|face| {
                face.families.iter().any(|(name, _)| {
                    let n = name.to_lowercase();
                    n.contains("marlett")
                        || n.contains("webdings")
                        || n.contains("wingding")
                        || n.contains("symbol")
                        || n.contains("mdl2")
                        || n.contains("fluent icon")
                        || n.contains("dingbat")
                        || n.contains("bookshelf")
                })
            })
            .map(|face| face.id)
            .collect();
        for id in face_ids_to_remove {
            font_system.db_mut().remove_face(id);
        }

        // Diagnostic: print all font families in fontdb to verify CJK fonts are registered
        println!("[VELLO-FONT] ===== All font families in fontdb =====");
        let mut family_names: Vec<String> = font_system.db().faces()
            .flat_map(|face| face.families.iter().map(|(name, _)| name.clone()))
            .collect();
        family_names.sort();
        family_names.dedup();
        for name in &family_names {
            println!("[VELLO-FONT]   Family: {}", name);
        }
        println!("[VELLO-FONT] Total {} unique font families", family_names.len());

        // Set fallback families to actual existing CJK fonts
        // Try both English and Chinese names since fontdb may register under either
        let has_msyh = family_names.iter().any(|n| n.contains("Microsoft YaHei") || n.contains("微软雅黑"));
        let has_simsun = family_names.iter().any(|n| n.contains("SimSun") || n.contains("宋体"));
        if has_msyh {
            font_system.db_mut().set_sans_serif_family("Microsoft YaHei");
            println!("[VELLO-FONT] Set sans-serif fallback to Microsoft YaHei");
        } else if family_names.iter().any(|n| n.contains("微软雅黑")) {
            font_system.db_mut().set_sans_serif_family("微软雅黑");
            println!("[VELLO-FONT] Set sans-serif fallback to 微软雅黑");
        }
        if has_simsun {
            font_system.db_mut().set_serif_family("SimSun");
            font_system.db_mut().set_monospace_family("SimSun");
            eprintln!("[VELLO-FONT] Set serif/monospace fallback to SimSun");
        } else if family_names.iter().any(|n| n.contains("宋体")) {
            font_system.db_mut().set_serif_family("宋体");
            font_system.db_mut().set_monospace_family("宋体");
            eprintln!("[VELLO-FONT] Set serif/monospace fallback to 宋体");
        }

        eprintln!(
            "[VELLO-FONT] Total {} font faces loaded into cosmic_text",
            font_system.db().faces().count()
        );

        Ok(Self {
            device: Arc::new(device),
            queue: Arc::new(queue),
            renderer,
            font_system,
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
        page_width: f32,
        page_height: f32,
    ) -> Result<Vec<u8>, String> {
        let mut scene = Scene::new();
        let mut scale_context = ScaleContext::new();
        let mut vector_rendered_indices = HashSet::new();

        let scale_x = if page_width > 0.0 {
            width as f64 / page_width as f64
        } else {
            1.0
        };
        let scale_y = if page_height > 0.0 {
            height as f64 / page_height as f64
        } else {
            1.0
        };
        let transform = Affine::scale_non_uniform(scale_x, scale_y);

        for (idx, object) in objects.iter().enumerate() {
            match object {
                RenderObject::Path(path) => {
                    let bez_path = path_utils::path_segments_to_bez_path(&path.segments);

                    if path.fill {
                        let color = color_utils::parse_hex_vello_color(
                            path.fill_color.as_deref().unwrap_or("#000000"),
                            path.alpha,
                        );
                        scene.fill(Fill::NonZero, transform, color, None, &bez_path);
                    }
                    if path.stroke {
                        let color = color_utils::parse_hex_vello_color(
                            path.stroke_color.as_deref().unwrap_or("#000000"),
                            path.alpha,
                        );
                        scene.stroke(
                            &Stroke::new(path.stroke_width as f64),
                            transform,
                            color,
                            None,
                            &bez_path,
                        );
                    }
                }
                RenderObject::Text(text) => {
                    if self.draw_text_vector(&mut scene, &mut scale_context, text, transform) {
                        vector_rendered_indices.insert(idx);
                    }
                }
                _ => {}
            }
        }

        let target_width = width.max(1);
        let target_height = height.max(1);

        let rgba_data = self.perform_vello_render_raw(&scene, target_width, target_height)?;

        // ── Phase 2: CPU overlay for text + legacy images ──
        let mut img = ImageBuffer::<Rgba<u8>, _>::from_raw(target_width, target_height, rgba_data)
            .ok_or("Failed to create image buffer from Vello output")?;

        let scale_f32_x = scale_x as f32;
        let scale_f32_y = scale_y as f32;

        for (idx, object) in objects.iter().enumerate() {
            match object {
                RenderObject::Text(text_model) => {
                    if vector_rendered_indices.contains(&idx) {
                        continue;
                    }
                    self.draw_text_bitmap_deprecated(&mut img, text_model, scale_f32_x, width, height);
                }
                RenderObject::Image(image_model) => {
                    self.draw_image_cpu(&mut img, image_model, scale_f32_x, scale_f32_y, width, height);
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
        scale_x: f32,
        scale_y: f32,
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

        let target_w = (model.width * scale_x).abs() as u32;
        let target_h = (model.height * scale_y).abs() as u32;

        if target_w == 0 || target_h == 0 {
            return;
        }

        let target_x = (model.x * scale_x) as i32;
        let target_y = (model.y * scale_y) as i32;

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
        println!("[CALL-BITMAP-TEXT] text='{}' rendering_mode={:?}", text.text, text.rendering_mode);
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
        let mut attrs = cosmic_text::Attrs::new();
        if text.is_bold {
            attrs = attrs.weight(cosmic_text::Weight::BOLD);
        }
        if text.is_italic {
            attrs = attrs.style(cosmic_text::Style::Italic);
        }
        let resolved_font = self.resolve_pdf_font(text);
        let family = self.resolve_cosmic_family(text, &resolved_font);
        attrs = attrs.family(family);

        let (cr, cg, cb) = color_utils::parse_hex_color_rgb(if text.color.is_empty() {
            "#000000"
        } else {
            &text.color
        });

        let mut buffer = Buffer::new(&mut self.font_system, metrics);
        buffer.set_size(
            &mut self.font_system,
            Some(canvas_w as f32),
            Some(canvas_h as f32),
        );
        buffer.set_text(&mut self.font_system, &text.text, attrs, Shaping::Advanced);
        buffer.shape_until_scroll(&mut self.font_system, false);

        let base_x = text.tx * zoom;
        let base_y = text.ty * zoom;

        for run in buffer.layout_runs() {
            for glyph in run.glyphs {
                let physical = glyph.physical((0.0, 0.0), 1.0);
                if let Some(font_face) = self.font_system.db().face(physical.cache_key.font_id) {
                    println!(
                        "[VELLO-GLYPH] text='{}' font_name='{}' font_id={:?} face_families={:?} glyph_id={}",
                        preview_text(&text.text),
                        text.font_name,
                        physical.cache_key.font_id,
                        font_face.families,
                        glyph.glyph_id
                    );
                }
                if let Some(glyph_img) = self
                    .swash_cache
                    .get_image_uncached(&mut self.font_system, physical.cache_key)
                {
                    let gx = base_x as i32 + physical.x + glyph_img.placement.left;
                    let gy = (base_y as i32) - glyph_img.placement.top;
                    if glyph_img.placement.width > 0 && glyph_img.placement.height > 0 {
                        println!("    [BITMAP-TEXT] '{}' glyph at gx={}, gy={} (canvas {}x{})", text.text, gx, gy, canvas_w, canvas_h);
                    }
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

        println!("    [DATA-LEN] content={:?} len={} gw*gh={} (gw={}, gh={})", glyph_img.content, glyph_img.data.len(), gw * gh, gw, gh);

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
        let fill_color = self.text_fill_color(text);
        if text_fill_enabled(text.rendering_mode) {
            scene.fill(
                Fill::EvenOdd,
                Affine::IDENTITY,
                fill_color,
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
        transform: Affine,
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
        transform
            * Affine::translate(Vec2::new(baseline_x as f64, baseline_y as f64))
            * Affine::scale_non_uniform(scale, -scale)
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
        transform: Affine,
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
        if self.draw_embedded_text_vector(scene, scale_context, text, &resolved_font, transform) {
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
        let total_glyphs: usize = layout_runs.iter().map(|r| r.glyphs.len()).sum();
        println!(
            "[TEXT-VECTOR-TRACE] text='{}' font='{}' runs={} glyphs={} color='{}' mode={} tx={:.2} ty={:.2}",
            preview_text(&text.text),
            text.font_name,
            layout_runs.len(),
            total_glyphs,
            text.color,
            text.rendering_mode,
            text.tx,
            text.ty
        );
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
        if text.text.contains("Rust") || text.text.contains("编程语言") {
            println!("[COSMIC-SHAPE-DEBUG] text='{}' req_font='{}' matched_font='{}'", text.text, text.font_name, matched_font);
        }
        let mut drew_any_glyph = false;

        for run in layout_runs {
            for glyph in run.glyphs {
                let font_id = glyph.font_id;
                if let Some(font_face) = self.font_system.db().face(font_id) {
                    if text.text.contains("Rust") {
                        println!("[GLYPH-POS-DEBUG] text='{}' gid={} gx={} gy={} tx={} ty={}", text.text, glyph.glyph_id, glyph.x, glyph.y, text.tx, text.ty);
                    }
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
                            transform,
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
