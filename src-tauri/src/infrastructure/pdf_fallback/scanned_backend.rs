use crate::infrastructure::pdf::cache::PDF_IMAGE_CACHE;
use crate::infrastructure::pdf_fallback::backend::PdfReadBackend;
use crate::infrastructure::pdf_fallback::classification::{
    classify_open_decision, likely_ocr_scanned_document, qualifies_as_scanned_page,
    ClassificationDecision,
};
use crate::infrastructure::pdf_fallback::types::{PagePreview, PdfDocumentKind, ReadDocumentMeta};
use crate::log_step;
use crate::pdf_log;
use memmap2::Mmap;
use pdf::content::{parse_ops, Op};
use pdf::enc::StreamFilter;
use pdf::file::{CachedFile, FileOptions};
use pdf::object::{
    MaybeRef, Object, PagesNode, Rectangle, Ref, Resolve, Resources, Stream, XObject,
};
use pdf::primitive::Primitive;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
pub struct ScannedReadBackend;

struct LoadedScannedDocument {
    file: CachedFile<Mmap>,
    file_open: std::time::Duration,
    mmap: std::time::Duration,
    load: std::time::Duration,
    total: std::time::Duration,
}
impl ScannedReadBackend {
    pub fn new() -> Self {
        Self
    }
    fn load_document(path: &str) -> Result<LoadedScannedDocument, String> {
        let file_start = Instant::now();
        let raw = File::open(Path::new(path)).map_err(|e| ToString::to_string(&e))?;
        let file_elapsed = file_start.elapsed();
        pdf_log!(
            2,
            "[PDF-READ][scanned][detail] File::open {:?} {}",
            file_elapsed,
            path
        );

        let mmap_start = Instant::now();
        let mmap = unsafe { Mmap::map(&raw).map_err(|e| ToString::to_string(&e))? };
        let mmap_elapsed = mmap_start.elapsed();
        pdf_log!(
            2,
            "[PDF-READ][scanned][detail] Mmap::map {:?} {}",
            mmap_elapsed,
            path
        );

        let load_start = Instant::now();
        let file = FileOptions::cached()
            .load(mmap)
            .map_err(|e| ToString::to_string(&e))?;
        let load_elapsed = load_start.elapsed();
        let total_elapsed = file_elapsed + mmap_elapsed + load_elapsed;
        pdf_log!(
            2,
            "[PDF-READ][scanned][detail] load(mmap) {:?} {}",
            load_elapsed,
            path
        );
        pdf_log!(
            2,
            "[PDF-READ][scanned][detail] load_document total {:?} {}",
            total_elapsed,
            path
        );
        Ok(LoadedScannedDocument {
            file,
            file_open: file_elapsed,
            mmap: mmap_elapsed,
            load: load_elapsed,
            total: total_elapsed,
        })
    }
    fn resolve_page_tree(
        resolver: &impl Resolve,
        kids: &[Ref<PagesNode>],
        target_page: u32,
        inherited_resources: Option<MaybeRef<Resources>>,
        inherited_media_box: Option<Rectangle>,
    ) -> Result<(Rectangle, MaybeRef<Resources>, bool), String> {
        let mut pos = 0u32;

        for kid in kids {
            let primitive = resolver
                .resolve(kid.get_inner())
                .map_err(|e| ToString::to_string(&e))?;
            let mut dict = primitive
                .into_dictionary()
                .map_err(|e| ToString::to_string(&e))?;
            let node_type = dict
                .require("PagesNode", "Type")
                .map_err(|e| ToString::to_string(&e))?
                .as_name()
                .map_err(|e| ToString::to_string(&e))?
                .to_string();

            match node_type.as_str() {
                "Pages" => {
                    let count = dict
                        .get("Count")
                        .ok_or_else(|| "Pages node missing Count".to_string())?
                        .as_u32()
                        .map_err(|e| ToString::to_string(&e))?;
                    if target_page >= pos + count {
                        pos += count;
                        continue;
                    }

                    let child_kids: Vec<Ref<PagesNode>> = Vec::from_primitive(
                        dict.get("Kids")
                            .ok_or_else(|| "Pages node missing Kids".to_string())?
                            .clone(),
                        resolver,
                    )
                    .map_err(|e| ToString::to_string(&e))?;

                    let child_resources = dict
                        .get("Resources")
                        .map(|p| MaybeRef::<Resources>::from_primitive(p.clone(), resolver))
                        .transpose()
                        .map_err(|e| ToString::to_string(&e))?
                        .or(inherited_resources.clone());

                    let child_media_box = dict
                        .get("MediaBox")
                        .map(|p| Rectangle::from_primitive(p.clone(), resolver))
                        .transpose()
                        .map_err(|e| ToString::to_string(&e))?
                        .or(inherited_media_box);

                    return Self::resolve_page_tree(
                        resolver,
                        &child_kids,
                        target_page - pos,
                        child_resources,
                        child_media_box,
                    );
                }
                "Page" => {
                    if pos != target_page {
                        pos += 1;
                        continue;
                    }

                    let media_box = dict
                        .get("MediaBox")
                        .map(|p| Rectangle::from_primitive(p.clone(), resolver))
                        .transpose()
                        .map_err(|e| ToString::to_string(&e))?
                        .or(inherited_media_box)
                        .ok_or_else(|| "Page missing MediaBox".to_string())?;

                    let resources = dict
                        .get("Resources")
                        .map(|p| MaybeRef::<Resources>::from_primitive(p.clone(), resolver))
                        .transpose()
                        .map_err(|e| ToString::to_string(&e))?
                        .or(inherited_resources.clone())
                        .ok_or_else(|| "Page missing Resources".to_string())?;

                    let has_text = dict
                        .get("Contents")
                        .map(|contents| Self::page_has_text_content(resolver, contents))
                        .transpose()?
                        .unwrap_or(false);

                    return Ok((media_box, resources, has_text));
                }
                other => {
                    return Err(format!("Unexpected page tree nodetype: {other}"));
                }
            }
        }

        Err(format!("Page {target_page} out of bounds"))
    }
    fn read_page_context(
        file: &pdf::file::CachedFile<Mmap>,
        page_index: u16,
    ) -> Result<(f32, f32, MaybeRef<Resources>, bool), String> {
        let root_pages = &file.get_root().pages;
        let resolver = file.resolver();
        let (media_box, resources, has_text) = Self::resolve_page_tree(
            &resolver,
            &root_pages.kids,
            page_index as u32,
            root_pages.resources.clone(),
            root_pages.media_box,
        )?;
        let width = (media_box.right - media_box.left).abs();
        let height = (media_box.top - media_box.bottom).abs();
        Ok((width, height, resources, has_text))
    }
    fn cache_jpeg(bytes: Arc<[u8]>) -> String {
        let cache_start = Instant::now();
        let asset_id = ::uuid::Uuid::new_v4().to_string();
        let byte_len = bytes.len();
        let mut cache = PDF_IMAGE_CACHE.lock().unwrap();
        cache.insert(asset_id.clone(), bytes);
        pdf_log!(
            2,
            "[PDF-READ][scanned][detail] cache_jpeg mode=arc bytes={} took {:?}",
            byte_len,
            cache_start.elapsed()
        );
        format!("http://pdfasset.localhost/{}", asset_id)
    }
    fn page_has_text_content(
        resolver: &impl Resolve,
        primitive: &Primitive,
    ) -> Result<bool, String> {
        let content = match pdf::content::Content::from_primitive(primitive.clone(), resolver) {
            Ok(content) => content,
            Err(_) => {
                return Ok(Self::primitive_may_contain_text_operators(
                    resolver, primitive,
                ))
            }
        };

        let mut data = Vec::new();
        for part in &content.parts {
            let part_data = part.data(resolver).map_err(|e| ToString::to_string(&e))?;
            data.extend_from_slice(&part_data);
        }

        match parse_ops(&data, resolver) {
            Ok(ops) => Ok(ops.iter().any(Self::is_text_op)),
            Err(_) => Ok(Self::bytes_may_contain_text_operators(&data)),
        }
    }
    fn primitive_may_contain_text_operators(
        resolver: &impl Resolve,
        primitive: &Primitive,
    ) -> bool {
        match primitive {
            Primitive::Reference(id) => resolver
                .resolve(*id)
                .map(|resolved| Self::primitive_may_contain_text_operators(resolver, &resolved))
                .unwrap_or(false),
            Primitive::Array(items) => items
                .iter()
                .any(|item| Self::primitive_may_contain_text_operators(resolver, item)),
            Primitive::Stream(_) => {
                let stream = Stream::<()>::from_primitive(primitive.clone(), resolver).ok();
                let data = stream.and_then(|stream| stream.data(resolver).ok());
                data.as_deref()
                    .map(Self::bytes_may_contain_text_operators)
                    .unwrap_or(false)
            }
            _ => false,
        }
    }
    fn bytes_may_contain_text_operators(bytes: &[u8]) -> bool {
        const TOKENS: [&[u8]; 6] = [b" BT", b"\nBT", b"\rBT", b" Tj", b" TJ", b" Tf"];
        TOKENS
            .iter()
            .any(|token| bytes.windows(token.len()).any(|window| window == *token))
    }
    fn is_text_op(op: &Op) -> bool {
        matches!(
            op,
            Op::BeginText
                | Op::EndText
                | Op::TextFont { .. }
                | Op::TextNewline
                | Op::TextDraw { .. }
                | Op::TextDrawAdjusted { .. }
                | Op::MoveTextPosition { .. }
                | Op::SetTextMatrix { .. }
                | Op::CharSpacing { .. }
                | Op::WordSpacing { .. }
                | Op::TextScaling { .. }
                | Op::Leading { .. }
                | Op::TextRenderMode { .. }
                | Op::TextRise { .. }
        )
    }
}
impl PdfReadBackend for ScannedReadBackend {
    fn open(&self, path: &str) -> Result<ReadDocumentMeta, String> {
        let total = Instant::now();
        let LoadedScannedDocument {
            file,
            file_open: file_open_ms,
            mmap: mmap_ms,
            load: load_ms,
            total: load_total_ms,
        } = Self::load_document(path)?;

        let count_start = Instant::now();
        let page_count = file.num_pages() as usize;
        let count_elapsed = count_start.elapsed();
        let avg_page_bytes = if page_count > 0 {
            std::fs::metadata(path)
                .ok()
                .map(|m| m.len() / page_count as u64)
                .unwrap_or(0)
        } else {
            0
        };

        let decision = if page_count > 0 {
            let (width, height, resources, has_text_content) = Self::read_page_context(&file, 0)?;
            let has_font_resources = !resources.fonts.is_empty();
            let mut image_covers_page = false;
            for xref in resources.xobjects.values() {
                let obj = file
                    .resolver()
                    .get(*xref)
                    .map_err(|e| ToString::to_string(&e))?;
                let XObject::Image(img) = &*obj else {
                    continue;
                };
                if qualifies_as_scanned_page(width, height, img.width, img.height) {
                    image_covers_page = true;
                    break;
                }
            }
            classify_open_decision(
                avg_page_bytes,
                image_covers_page,
                has_text_content,
                has_font_resources,
            )
        } else {
            ClassificationDecision::unknown()
        };

        log_step!(
            "[PDF-READ][scanned][open] pages={} avg_page_bytes={} confidence={:.2} allow_scan_preview_first_paint={} reason={:?} total={:?} file_open={:?} mmap={:?} load={:?} load_total={:?} count={:?} path={}",
            page_count,
            avg_page_bytes,
            decision.confidence,
            decision.allow_scan_preview_first_paint,
            decision.reason,
            total.elapsed(),
            file_open_ms,
            mmap_ms,
            load_ms,
            load_total_ms,
            count_elapsed,
            path
        );
        Ok(ReadDocumentMeta {
            doc_id: path.to_string(),
            path: path.to_string(),
            page_count,
            kind: decision.kind,
            confidence: decision.confidence,
            allow_scan_preview_first_paint: decision.allow_scan_preview_first_paint,
            classification_reason: decision.reason,
        })
    }
    fn read_page_preview(&self, path: &str, page_index: u16) -> Result<PagePreview, String> {
        let total = Instant::now();
        let LoadedScannedDocument {
            file,
            file_open: file_open_ms,
            mmap: mmap_ms,
            load: load_ms,
            total: load_total_ms,
        } = Self::load_document(path)?;
        let page_count = file.num_pages().max(1) as u64;
        let avg_page_bytes = std::fs::metadata(path)
            .ok()
            .map(|m| m.len() / page_count)
            .unwrap_or(0);

        let page_start = Instant::now();
        let (width, height, resources, has_text_content) =
            Self::read_page_context(&file, page_index)?;
        let page_elapsed = page_start.elapsed();

        let resources_start = Instant::now();
        let xobject_count = resources.xobjects.len();
        let has_font_resources = !resources.fonts.is_empty();
        let resources_elapsed = resources_start.elapsed();

        let resolver = file.resolver();
        let scan_start = Instant::now();
        let mut image_count = 0usize;
        let mut best_area = 0u64;
        let mut best_filter = "None";
        let mut best_bytes = 0usize;
        let mut best_raw_elapsed = std::time::Duration::default();
        let mut cache_elapsed = std::time::Duration::default();
        let mut best_image_width = 0u32;
        let mut best_image_height = 0u32;
        let mut image_url: Option<String> = None;

        for xref in resources.xobjects.values() {
            let obj = resolver.get(*xref).map_err(|e| ToString::to_string(&e))?;
            let XObject::Image(img) = &*obj else {
                continue;
            };
            image_count += 1;

            let area = img.width as u64 * img.height as u64;
            let raw_start = Instant::now();
            let (data, filter) = img
                .raw_image_data(&resolver)
                .map_err(|e| ToString::to_string(&e))?;
            let raw_elapsed = raw_start.elapsed();
            let filter_name = match filter {
                Some(StreamFilter::DCTDecode(_)) => "DCTDecode",
                Some(StreamFilter::JBIG2Decode(_)) => "JBIG2Decode",
                Some(StreamFilter::JPXDecode) => "JPXDecode",
                Some(StreamFilter::FlateDecode(_)) => "FlateDecode",
                Some(StreamFilter::CCITTFaxDecode(_)) => "CCITTFaxDecode",
                Some(StreamFilter::ASCII85Decode) => "ASCII85Decode",
                Some(StreamFilter::ASCIIHexDecode) => "ASCIIHexDecode",
                Some(StreamFilter::LZWDecode(_)) => "LZWDecode",
                Some(StreamFilter::RunLengthDecode) => "RunLengthDecode",
                Some(StreamFilter::Crypt) => "Crypt",
                None => "None",
            };
            pdf_log!(
                2,
                "[PDF-READ][scanned][detail] candidate image {}x{} filter={} bytes={} raw_image_data {:?} page={} path={}",
                img.width,
                img.height,
                filter_name,
                data.len(),
                raw_elapsed,
                page_index,
                path
            );

            if area <= best_area {
                continue;
            }

            if matches!(filter, Some(StreamFilter::DCTDecode(_))) {
                best_area = area;
                best_filter = filter_name;
                best_bytes = data.len();
                best_raw_elapsed = raw_elapsed;
                best_image_width = img.width;
                best_image_height = img.height;
                let image_covers_page =
                    qualifies_as_scanned_page(width, height, img.width, img.height);
                if image_covers_page
                    && ((!has_text_content && !has_font_resources)
                        || likely_ocr_scanned_document(
                            avg_page_bytes,
                            image_covers_page,
                            has_text_content,
                            has_font_resources,
                        ))
                {
                    let cache_start = Instant::now();
                    image_url = Some(Self::cache_jpeg(data));
                    cache_elapsed = cache_start.elapsed();
                } else {
                    image_url = None;
                    cache_elapsed = std::time::Duration::default();
                }
            }
        }

        let scan_elapsed = scan_start.elapsed();

        let ready = image_url.is_some();
        let preview = PagePreview {
            doc_id: path.to_string(),
            page_index,
            width,
            height,
            image_url,
            kind: PdfDocumentKind::Scanned,
            ready,
        };

        log_step!(
            "[PDF-READ][scanned][page] page={} ready={} has_text={} has_fonts={} avg_page_bytes={} total={:?} file_open={:?} mmap={:?} load={:?} load_total={:?} page_ctx={:?} resources={:?} xobjects={} images={} scan={:?} best_filter={} best_bytes={} best_image={}x{} best_raw={:?} cache={:?} path={}",
            page_index,
            ready,
            has_text_content,
            has_font_resources,
            avg_page_bytes,
            total.elapsed(),
            file_open_ms,
            mmap_ms,
            load_ms,
            load_total_ms,
            page_elapsed,
            resources_elapsed,
            xobject_count,
            image_count,
            scan_elapsed,
            best_filter,
            best_bytes,
            best_image_width,
            best_image_height,
            best_raw_elapsed,
            cache_elapsed,
            path
        );
        Ok(preview)
    }
}

#[cfg(test)]
mod tests {
    use super::ScannedReadBackend;

    #[test]
    fn bytes_may_contain_text_operators_matches_all_six_tokens() {
        for sample in [
            &b"q BT /F1 12 Tf (hi) Tj Q"[..],
            b"\nBT /F2 10 Tf [(a) -20 (b)] TJ ET",
            b"\rBT 1 0 0 1 10 10 Tm",
            b"/Image Do Tj end",
            b"re f TJ",
            b"BT ET Tf",
        ] {
            assert!(
                ScannedReadBackend::bytes_may_contain_text_operators(sample),
                "{sample:?}"
            );
        }
    }

    #[test]
    fn bytes_may_contain_text_operators_requires_leading_separator() {
        // 令牌都带前缀分隔符:无空白前缀的算子串、过短输入、非文本内容均不匹配
        assert!(!ScannedReadBackend::bytes_may_contain_text_operators(b"BT"));
        assert!(!ScannedReadBackend::bytes_may_contain_text_operators(b"BTTjTJTf"));
        assert!(!ScannedReadBackend::bytes_may_contain_text_operators(b""));
        assert!(!ScannedReadBackend::bytes_may_contain_text_operators(
            b"q 100 0 0 100 0 0 cm /Im0 Do Q"
        ));
        assert!(!ScannedReadBackend::bytes_may_contain_text_operators(
            b"0.5 0.5 0.5 rg 100 100 200 150 re f"
        ));
    }
}
