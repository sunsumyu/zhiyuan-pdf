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
pub(crate) fn repair_and_load(raw: &[u8]) -> Result<Document, String> {
    // Find the last occurrence of "startxref" in the file
    let content = String::from_utf8_lossy(raw);

    // Strategy 3a: Trim trailing garbage after %%EOF
    if let Some(eof_pos) = content.rfind("%%EOF") {
        let trimmed_end = eof_pos + 5; // "%%EOF".len()
                                       // Skip any trailing newlines
        let trimmed_end = raw.len().min(trimmed_end + 2);
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

    // Strategy 3b: Find startxref and verify the offset points to valid xref/obj
    if let Some(startxref_pos) = content.rfind("startxref") {
        let after_startxref = &content[startxref_pos + 9..];
        let offset_str = after_startxref
            .trim_start()
            .lines()
            .next()
            .unwrap_or("")
            .trim();
        if let Ok(xref_offset) = offset_str.parse::<usize>() {
            // Verify the offset is within file bounds
            if xref_offset < raw.len() {
                let at_offset = &content[xref_offset..];
                // If it points to "xref" or a valid obj, the startxref is correct
                // Try scanning backwards for an earlier valid xref
                if !at_offset.starts_with("xref") && !at_offset.contains("obj") {
                    // The startxref offset is wrong - try to find actual xref location
                    if let Some(real_xref) = content
                        .rfind("\nxref\n")
                        .or_else(|| content.rfind("\nxref\r"))
                    {
                        let real_offset = real_xref + 1; // skip the leading newline
                        crate::log_step!(
                            "[PDF][repair] Fixing startxref from {} to {}",
                            xref_offset,
                            real_offset
                        );
                        let mut repaired = raw.to_vec();
                        let new_startxref = format!("startxref\n{}\n%%EOF\n", real_offset);
                        // Replace from startxref_pos to end
                        repaired.truncate(startxref_pos);
                        repaired.extend_from_slice(new_startxref.as_bytes());
                        if let Ok(doc) = Document::load_mem(&repaired) {
                            return Ok(doc);
                        }
                    }
                }
            }
        }
    }

    // Strategy 3c: If the file has cross-reference streams (PDF 1.5+),
    // look for the last "obj" that contains /Type /XRef
    // This handles PDFs that use xref streams instead of traditional xref tables
    if content.contains("/Type /XRef") || content.contains("/Type/XRef") {
        // Find the byte offset of the last xref stream object
        let search_patterns = ["/Type /XRef", "/Type/XRef"];
        let mut last_xref_stream_pos = None;
        for pattern in &search_patterns {
            if let Some(pos) = content.rfind(pattern) {
                // Walk backwards to find the "N N obj" header
                let before = &content[..pos];
                if let Some(obj_line_start) = before.rfind('\n') {
                    let candidate_start = before[..obj_line_start]
                        .rfind('\n')
                        .map(|p| p + 1)
                        .unwrap_or(0);
                    let obj_header = &before[candidate_start..obj_line_start];
                    if obj_header.trim().ends_with("obj") {
                        last_xref_stream_pos = Some(candidate_start);
                    }
                }
            }
        }
        if let Some(xref_pos) = last_xref_stream_pos {
            // Build a repaired file with correct startxref pointing to this object
            let mut repaired = raw.to_vec();
            let new_tail = format!("\nstartxref\n{}\n%%EOF\n", xref_pos);
            // Find where to append - after the last endobj or at end
            if let Some(last_endobj) = content.rfind("endobj") {
                let append_pos = last_endobj + 6;
                repaired.truncate(append_pos);
                repaired.extend_from_slice(new_tail.as_bytes());
                if let Ok(doc) = Document::load_mem(&repaired) {
                    return Ok(doc);
                }
            }
        }
    }

    Err("PDF repair strategies exhausted".to_string())
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
        // and no "obj" in the remainder - forcing the 3b relocation path.
        let trailer_pos = bytes
            .windows(7)
            .rposition(|w| w == b"trailer")
            .expect("serialized PDF contains a trailer");
        let startxref_pos = bytes
            .windows(9)
            .rposition(|w| w == b"startxref")
            .expect("serialized PDF contains startxref");
        let mut num_start = startxref_pos + 9;
        while num_start < bytes.len() && (bytes[num_start] as char).is_ascii_whitespace() {
            num_start += 1;
        }
        let num_end = num_start
            + bytes[num_start..]
                .iter()
                .take_while(|b| b.is_ascii_digit())
                .count();

        let mut corrupted = Vec::with_capacity(bytes.len());
        corrupted.extend_from_slice(&bytes[..num_start]);
        corrupted.extend_from_slice(trailer_pos.to_string().as_bytes());
        corrupted.extend_from_slice(&bytes[num_end..]);

        let doc = repair_and_load(&corrupted);
        assert!(doc.is_ok(), "3b should relocate startxref: {:?}", doc.err());
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
