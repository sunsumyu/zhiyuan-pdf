use lopdf::Document;
use std::fs;

/// Public wrapper for lenient PDF loading (used from other modules).
pub fn load_pdf_public(path: &str) -> Result<Document, String> {
    load_pdf_lenient(path)
}

/// Attempt to load a PDF with multiple fallback strategies:
/// 1. Direct lopdf::Document::load (strict)
/// 2. Load from memory bytes (handles some path encoding issues)
/// 3. Repair the PDF trailer and retry from memory
pub(crate) fn load_pdf_lenient(path: &str) -> Result<Document, String> {
    // Strategy 1: Direct file load
    match Document::load(path) {
        Ok(doc) => {
            crate::log_step!(
                "[PDF][load_lenient] Strategy 1 (direct load) SUCCESS for {}",
                path
            );
            return Ok(doc);
        }
        Err(e) => {
            crate::log_step!(
                "[PDF][load_lenient] Strategy 1 (direct load) FAILED: {} - trying fallbacks",
                e
            );
        }
    }

    // Read raw bytes for subsequent strategies
    let raw_bytes = fs::read(path).map_err(|e| format!("Cannot read PDF file {}: {}", path, e))?;

    if raw_bytes.len() < 8 {
        return Err(format!(
            "PDF file too small ({} bytes): {}",
            raw_bytes.len(),
            path
        ));
    }

    // Strategy 2: Load from memory (bypasses file I/O quirks)
    match Document::load_mem(&raw_bytes) {
        Ok(doc) => {
            crate::log_step!(
                "[PDF][load_lenient] Strategy 2 (load_mem) SUCCESS for {}",
                path
            );
            return Ok(doc);
        }
        Err(e) => {
            crate::log_step!(
                "[PDF][load_lenient] Strategy 2 (load_mem) FAILED: {} - trying repair",
                e
            );
        }
    }

    // Strategy 3: Repair trailer and retry
    match repair_and_load(&raw_bytes) {
        Ok(doc) => {
            crate::log_step!(
                "[PDF][load_lenient] Strategy 3 (repair) SUCCESS for {}",
                path
            );
            return Ok(doc);
        }
        Err(e) => {
            crate::log_step!("[PDF][load_lenient] Strategy 3 (repair) FAILED: {}", e);
        }
    }

    Err(format!("All PDF loading strategies failed for {}", path))
}

/// Try to repair a PDF with invalid trailer by finding and fixing the startxref value,
/// or by synthesizing a minimal trailer if missing.
///
/// All searching operates on raw bytes: PDF payloads regularly contain binary
/// streams that are invalid UTF-8, and `String::from_utf8_lossy` would replace
/// those bytes with U+FFFD, silently shifting every byte index that follows.
pub(crate) fn repair_and_load(raw: &[u8]) -> Result<Document, String> {
    // Strategy 3a: Trim trailing garbage after %%EOF
    if let Some(eof_pos) = rfind_bytes(raw, b"%%EOF") {
        let trimmed_end = raw.len().min(eof_pos + 5 + 2); // "%%EOF" plus line ending
        if trimmed_end < raw.len() {
            let trimmed = &raw[..trimmed_end];
            crate::log_step!(
                "[PDF][repair] Trimming {} trailing bytes after %%EOF",
                raw.len() - trimmed_end
            );
            if let Ok(doc) = Document::load_mem(trimmed) {
                return Ok(doc);
            }
        }
    }

    // Strategy 3b/3c: verify the declared startxref target and, when it is
    // wrong or stale, append a corrected trailer pointing at the last xref
    // section (classic cross-reference table or PDF 1.5+ xref stream object).
    if let Some(repaired) = repair_startxref(raw) {
        if let Ok(doc) = Document::load_mem(&repaired) {
            return Ok(doc);
        }
    }

    Err("PDF repair strategies exhausted".to_string())
}

/// Append a corrected trailer whose startxref points at the last xref section.
/// Nothing is deleted: readers (lopdf included) parse startxref from the end
/// of the file, so the appended trailer wins. Returns None when the declared
/// startxref already points at the newest plausible xref section.
pub(crate) fn repair_startxref(raw: &[u8]) -> Option<Vec<u8>> {
    let declared = last_declared_startxref_offset(raw);
    let target = locate_last_xref_offset(raw)?;

    let already_correct = match declared {
        Some(offset) => verify_startxref_target(raw, offset) && offset >= target,
        None => false,
    };
    if already_correct || declared == Some(target) {
        return None;
    }

    match declared {
        Some(offset) => crate::log_step!(
            "[PDF][repair] Fixing startxref from {} to {}",
            offset,
            target
        ),
        None => crate::log_step!(
            "[PDF][repair] Adding missing startxref pointing to {}",
            target
        ),
    }
    let mut repaired = raw.to_vec();
    repaired.extend_from_slice(format!("\nstartxref\n{}\n%%EOF\n", target).as_bytes());
    Some(repaired)
}

fn rfind_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .rposition(|window| window == needle)
}

fn last_declared_startxref_offset(raw: &[u8]) -> Option<usize> {
    let keyword = rfind_bytes(raw, b"startxref")?;
    let mut cursor = keyword + b"startxref".len();
    while cursor < raw.len() && raw[cursor].is_ascii_whitespace() {
        cursor += 1;
    }
    let digits_end = cursor + raw[cursor..].iter().take_while(|b| b.is_ascii_digit()).count();
    std::str::from_utf8(&raw[cursor..digits_end])
        .ok()?
        .parse()
        .ok()
}

/// A startxref offset is plausible when the bytes at that offset locally look
/// like a cross-reference table or an indirect object header. Only a 32-byte
/// window is inspected: checking the whole remaining file would match "obj"
/// almost anywhere and defeat the check.
fn verify_startxref_target(raw: &[u8], offset: usize) -> bool {
    if offset >= raw.len() {
        return false;
    }
    let window = &raw[offset..(offset + 32).min(raw.len())];
    window.starts_with(b"xref") || window.windows(3).any(|w| w == b"obj")
}

/// Locate the byte offset of the newest xref section: the last classic
/// cross-reference table or xref stream object, whichever appears later.
fn locate_last_xref_offset(raw: &[u8]) -> Option<usize> {
    let classic = rfind_bytes(raw, b"\nxref\n")
        .or_else(|| rfind_bytes(raw, b"\nxref\r"))
        .map(|pos| pos + 1);
    let stream = locate_last_xref_stream_object(raw);
    match (classic, stream) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (found, None) | (None, found) => found,
    }
}

fn locate_last_xref_stream_object(raw: &[u8]) -> Option<usize> {
    let pattern_pos = rfind_bytes(raw, b"/Type /XRef")
        .zip(rfind_bytes(raw, b"/Type/XRef"))
        .map(|(a, b)| a.max(b))
        .or_else(|| rfind_bytes(raw, b"/Type /XRef"))
        .or_else(|| rfind_bytes(raw, b"/Type/XRef"))?;
    obj_header_start(raw, pattern_pos)
}

/// Walk backwards from `before` to the start of the nearest "N N obj" header.
/// Consuming digits and spaces covers both the two-line layout (header on its
/// own line, dictionary below) and the single-line "N 0 obj << /Type /XRef".
fn obj_header_start(raw: &[u8], before: usize) -> Option<usize> {
    let window_start = before.saturating_sub(96);
    let rel = raw[window_start..before]
        .windows(4)
        .rposition(|w| w == b" obj")?;
    let mut start = window_start + rel;
    while start > 0 && (raw[start - 1].is_ascii_digit() || raw[start - 1] == b' ') {
        start -= 1;
    }
    Some(start)
}

#[cfg(test)]
mod loader_tests {
    use super::*;
    use lopdf::dictionary;

    /// Build a structurally valid single-file PDF via lopdf itself, so the xref
    /// offsets are guaranteed correct (no hand-computed magic numbers).
    fn valid_pdf_bytes() -> Vec<u8> {
        let mut doc = Document::with_version("1.4");
        let pages_id = doc.add_object(dictionary! {
            "Type" => "Pages",
            "Count" => 0,
            "Kids" => lopdf::Object::Array(vec![]),
        });
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", lopdf::Object::Reference(catalog_id));
        // lopdf defaults to xref streams (PDF 1.5+); force the classic
        // cross-reference table so the fixture exercises the 3b repair path.
        doc.reference_table.cross_reference_type =
            lopdf::xref::XrefType::CrossReferenceTable;
        let mut buf = Vec::new();
        doc.save_to(&mut buf).expect("serialize in-memory PDF");
        buf
    }

    /// Same document shape, but serialized with the default PDF 1.5+
    /// cross-reference stream so the fixture exercises the stream repair path.
    fn valid_xref_stream_pdf_bytes() -> Vec<u8> {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.add_object(dictionary! {
            "Type" => "Pages",
            "Count" => 0,
            "Kids" => lopdf::Object::Array(vec![]),
        });
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", lopdf::Object::Reference(catalog_id));
        let mut buf = Vec::new();
        doc.save_to(&mut buf).expect("serialize in-memory PDF");
        buf
    }

    /// Rewrite the last startxref number so it declares `target`, keeping the
    /// rest of the file byte-identical.
    fn with_startxref_pointing_at(mut bytes: Vec<u8>, target: usize) -> Vec<u8> {
        let startxref_pos = bytes
            .windows(9)
            .rposition(|w| w == b"startxref")
            .expect("serialized PDF contains startxref");
        let mut num_start = startxref_pos + 9;
        while num_start < bytes.len() && bytes[num_start].is_ascii_whitespace() {
            num_start += 1;
        }
        let num_end = num_start
            + bytes[num_start..]
                .iter()
                .take_while(|b| b.is_ascii_digit())
                .count();

        let mut corrupted = Vec::with_capacity(bytes.len());
        corrupted.extend_from_slice(&bytes[..num_start]);
        corrupted.extend_from_slice(target.to_string().as_bytes());
        corrupted.extend_from_slice(&bytes[num_end..]);
        bytes = corrupted;
        bytes
    }

    #[test]
    fn repair_trims_trailing_garbage_after_eof() {
        let mut bytes = valid_pdf_bytes();
        bytes.extend_from_slice(b"JUNK-JUNK-JUNK");
        let doc = repair_and_load(&bytes);
        assert!(doc.is_ok(), "3a should trim garbage after %%EOF: {:?}", doc.err());
    }

    #[test]
    fn repair_relocates_wrong_startxref_to_real_xref() {
        let bytes = valid_pdf_bytes();
        // Point startxref at the "trailer" keyword: in bounds, not "xref",
        // and no "obj" in the local window - forcing the relocation path.
        let trailer_pos = bytes
            .windows(7)
            .rposition(|w| w == b"trailer")
            .expect("serialized PDF contains a trailer");

        let corrupted = with_startxref_pointing_at(bytes, trailer_pos);
        let doc = repair_and_load(&corrupted);
        assert!(doc.is_ok(), "3b should relocate startxref: {:?}", doc.err());
    }

    #[test]
    fn repair_relocates_wrong_startxref_to_xref_stream_object() {
        let bytes = valid_xref_stream_pdf_bytes();
        // Point startxref at the final byte: in bounds, verifies as garbage.
        let last_byte = bytes.len() - 1;
        let corrupted = with_startxref_pointing_at(bytes, last_byte);
        let doc = repair_and_load(&corrupted);
        assert!(
            doc.is_ok(),
            "repair should relocate startxref to the xref stream object: {:?}",
            doc.err()
        );
    }

    #[test]
    fn repair_handles_single_line_xref_stream_layout() {
        let bytes = valid_xref_stream_pdf_bytes();
        // Rewrite the xref stream object header from "N 0 obj\n<<" to
        // "N 0 obj <<": same length, so all serialized offsets stay valid.
        let header_join = bytes
            .windows(7)
            .rposition(|w| w == b" obj\n<<")
            .expect("xref stream object header followed by its dictionary");
        let mut single_line = bytes;
        single_line[header_join + 4] = b' ';

        // Point startxref at the final byte: in bounds, verifies as garbage.
        let last_byte = single_line.len() - 1;
        let corrupted = with_startxref_pointing_at(single_line, last_byte);
        let doc = repair_and_load(&corrupted);
        assert!(
            doc.is_ok(),
            "repair should find the single-line xref stream object: {:?}",
            doc.err()
        );
    }

    #[test]
    fn repair_survives_invalid_utf8_tail() {
        let bytes = valid_pdf_bytes();
        let trailer_pos = bytes
            .windows(7)
            .rposition(|w| w == b"trailer")
            .expect("serialized PDF contains a trailer");
        let mut corrupted = with_startxref_pointing_at(bytes, trailer_pos);
        // Invalid UTF-8 after %%EOF: under the old from_utf8_lossy approach
        // every byte index past the replacement chars shifted.
        corrupted.extend_from_slice(&[0xFF, 0xFE, 0x80, b'J', b'U', b'N', b'K']);

        let doc = repair_and_load(&corrupted);
        assert!(doc.is_ok(), "byte-level repair must not misalign: {:?}", doc.err());
    }

    #[test]
    fn repair_appends_trailer_without_deleting_content() {
        let bytes = valid_pdf_bytes();
        let trailer_pos = bytes
            .windows(7)
            .rposition(|w| w == b"trailer")
            .expect("serialized PDF contains a trailer");
        let corrupted = with_startxref_pointing_at(bytes, trailer_pos);

        let repaired = repair_startxref(&corrupted).expect("trailer should be rewritten");
        assert!(
            repaired.starts_with(&corrupted),
            "append-only repair must keep every original byte"
        );
        assert!(repaired.len() > corrupted.len());
        assert!(repaired.ends_with(b"%%EOF\n"));
    }

    #[test]
    fn repair_returns_err_when_strategies_exhausted() {
        assert!(repair_and_load(b"definitely not a pdf").is_err());
    }

    #[test]
    fn load_lenient_reads_valid_file_via_strategy_one() {
        let path = std::env::temp_dir().join("pdf_loader_lenient_valid.pdf");
        std::fs::write(&path, valid_pdf_bytes()).expect("write temp PDF");
        let result = load_pdf_lenient(path.to_str().unwrap());
        std::fs::remove_file(&path).ok();
        assert!(result.is_ok(), "valid file should load: {:?}", result.err());
    }

    #[test]
    fn load_lenient_rejects_missing_file() {
        let result = load_pdf_lenient("Z:/definitely/not/a/real/path.pdf");
        assert!(result.is_err());
    }
}
