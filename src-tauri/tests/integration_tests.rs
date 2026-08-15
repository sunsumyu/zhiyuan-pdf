//! Integration tests for PDF annotation, page operations, and metadata.
//!
//! These tests operate directly on `lopdf::Document` via the `PdfDocExt` trait,
//! and on read-side functions from `annotation_store`. No Tauri context needed.

use lopdf::{Dictionary, Object, Stream};
use pdf_viewer_standalone::infrastructure::pdf::annotation_store::{
    read_page_comments, read_page_highlights,
};
use pdf_viewer_standalone::infrastructure::pdf::pdf_write::PdfDocExt;

/// Build a valid single-page PDF using lopdf's API.
fn build_test_doc() -> lopdf::Document {
    let mut doc = lopdf::Document::with_version("1.7");

    let font_id = doc.add_object(Object::Dictionary({
        let mut d = Dictionary::new();
        d.set("Type", Object::Name(b"Font".to_vec()));
        d.set("Subtype", Object::Name(b"Type1".to_vec()));
        d.set("BaseFont", Object::Name(b"Helvetica".to_vec()));
        d
    }));

    // Create Info dict so update_metadata works
    let info_id = doc.add_object(Object::Dictionary(Dictionary::new()));

    let content_bytes = b"BT\n/F1 24 Tf\n100 700 Td\n(Hello) Tj\nET\n";
    let content_id = doc.add_object(Object::Stream(Stream::new(
        Dictionary::new(),
        content_bytes.to_vec(),
    )));

    let page_id = doc.add_object(Object::Dictionary({
        let mut d = Dictionary::new();
        d.set("Type", Object::Name(b"Page".to_vec()));
        d.set(
            "MediaBox",
            Object::Array(vec![
                Object::Integer(0),
                Object::Integer(0),
                Object::Integer(612),
                Object::Integer(792),
            ]),
        );
        let mut res = Dictionary::new();
        let mut font_dict = Dictionary::new();
        font_dict.set("F1", Object::Reference(font_id));
        res.set("Font", Object::Dictionary(font_dict));
        d.set("Resources", Object::Dictionary(res));
        d.set("Contents", Object::Reference(content_id));
        d
    }));

    let pages_id = doc.add_object(Object::Dictionary({
        let mut d = Dictionary::new();
        d.set("Type", Object::Name(b"Pages".to_vec()));
        d.set("Count", Object::Integer(1));
        d.set("Kids", Object::Array(vec![Object::Reference(page_id)]));
        d
    }));

    // Set Parent on the page
    if let Some(Object::Dictionary(ref mut dict)) = doc.objects.get_mut(&page_id) {
        dict.set("Parent", Object::Reference(pages_id));
    }

    let catalog_id = doc.add_object(Object::Dictionary({
        let mut d = Dictionary::new();
        d.set("Type", Object::Name(b"Catalog".to_vec()));
        d.set("Pages", Object::Reference(pages_id));
        d
    }));

    doc.trailer.set("Root", catalog_id);
    doc.trailer.set("Info", info_id);
    doc
}

/// Build a valid 2-page PDF.
fn build_two_page_doc() -> lopdf::Document {
    let mut doc = lopdf::Document::with_version("1.7");

    let font_id = doc.add_object(Object::Dictionary({
        let mut d = Dictionary::new();
        d.set("Type", Object::Name(b"Font".to_vec()));
        d.set("Subtype", Object::Name(b"Type1".to_vec()));
        d.set("BaseFont", Object::Name(b"Helvetica".to_vec()));
        d
    }));

    let mut page_ids = Vec::new();
    for i in 0..2 {
        let content_bytes = format!("BT\n/F1 24 Tf\n100 700 Td\n(Page{}) Tj\nET\n", i + 1);
        let content_id = doc.add_object(Object::Stream(Stream::new(
            Dictionary::new(),
            content_bytes.into_bytes(),
        )));
        let page_id = doc.add_object(Object::Dictionary({
            let mut d = Dictionary::new();
            d.set("Type", Object::Name(b"Page".to_vec()));
            d.set(
                "MediaBox",
                Object::Array(vec![
                    Object::Integer(0),
                    Object::Integer(0),
                    Object::Integer(612),
                    Object::Integer(792),
                ]),
            );
            let mut res = Dictionary::new();
            let mut font_dict = Dictionary::new();
            font_dict.set("F1", Object::Reference(font_id));
            res.set("Font", Object::Dictionary(font_dict));
            d.set("Resources", Object::Dictionary(res));
            d.set("Contents", Object::Reference(content_id));
            d
        }));
        page_ids.push(page_id);
    }

    let pages_id = doc.add_object(Object::Dictionary({
        let mut d = Dictionary::new();
        d.set("Type", Object::Name(b"Pages".to_vec()));
        d.set("Count", Object::Integer(2));
        d.set(
            "Kids",
            Object::Array(
                page_ids
                    .iter()
                    .map(|id| Object::Reference(*id))
                    .collect(),
            ),
        );
        d
    }));

    // Set Parent on each page
    for pid in &page_ids {
        if let Some(Object::Dictionary(ref mut dict)) = doc.objects.get_mut(pid) {
            dict.set("Parent", Object::Reference(pages_id));
        }
    }

    let catalog_id = doc.add_object(Object::Dictionary({
        let mut d = Dictionary::new();
        d.set("Type", Object::Name(b"Catalog".to_vec()));
        d.set("Pages", Object::Reference(pages_id));
        d
    }));

    doc.trailer.set("Root", catalog_id);
    doc
}

// ── Annotation CRUD ──────────────────────────────────────────────

#[test]
fn add_highlight_then_read_back() {
    let mut doc = build_test_doc();
    let page = 1u32;
    let rect = [10.0, 20.0, 100.0, 40.0];
    let color = [1.0, 0.92, 0.4];

    doc.add_highlight(page, rect, color)
        .expect("add_highlight should succeed");

    let highlights = read_page_highlights(&doc, page).expect("read should succeed");
    assert_eq!(highlights.len(), 1, "should find exactly one highlight");

    let h = &highlights[0];
    assert_eq!(h.color, color, "color should round-trip");
    assert!(
        h.rect[0] >= 9.0 && h.rect[0] <= 11.0,
        "left ~= 10, got {}",
        h.rect[0]
    );
    assert!(
        h.rect[2] > 90.0 && h.rect[2] <= 100.0,
        "width ~= 100, got {}",
        h.rect[2]
    );
    assert!(
        h.rect[3] > 30.0 && h.rect[3] <= 40.0,
        "height ~= 40, got {}",
        h.rect[3]
    );
}

#[test]
fn add_multiple_highlights_and_read_all() {
    let mut doc = build_test_doc();
    let page = 1u32;

    doc.add_highlight(page, [10.0, 10.0, 50.0, 20.0], [1.0, 0.0, 0.0])
        .unwrap();
    doc.add_highlight(page, [20.0, 30.0, 60.0, 10.0], [0.0, 1.0, 0.0])
        .unwrap();
    doc.add_highlight(page, [30.0, 50.0, 70.0, 15.0], [0.0, 0.0, 1.0])
        .unwrap();

    let highlights = read_page_highlights(&doc, page).unwrap();
    assert_eq!(highlights.len(), 3, "should find all three highlights");
}

#[test]
fn add_text_comment_then_read_back() {
    let mut doc = build_test_doc();
    let page = 1u32;
    let rect = [50.0, 60.0, 80.0, 30.0];
    let color = [0.42, 0.73, 0.98];
    let contents = "This is a test comment";

    doc.add_text_comment(page, rect, color, contents)
        .expect("add_text_comment should succeed");

    let comments = read_page_comments(&doc, page).expect("read should succeed");
    assert_eq!(comments.len(), 1, "should find exactly one comment");

    let c = &comments[0];
    assert_eq!(c.contents, contents, "contents should round-trip");
    assert_eq!(c.color, color, "color should round-trip");
}

#[test]
fn update_text_comment_contents() {
    let mut doc = build_test_doc();
    let page = 1u32;

    doc.add_text_comment(page, [10.0, 10.0, 50.0, 20.0], [1.0, 0.0, 0.0], "original")
        .unwrap();

    let comments = read_page_comments(&doc, page).unwrap();
    assert_eq!(comments.len(), 1);
    let parts: Vec<&str> = comments[0].id.split('-').collect();
    let obj_num: u32 = parts[0].parse().unwrap();
    let gen_num: u16 = parts[1].parse().unwrap();

    doc.update_text_comment(page, (obj_num, gen_num), "updated text")
        .expect("update should succeed");

    let comments_after = read_page_comments(&doc, page).unwrap();
    assert_eq!(comments_after.len(), 1);
    assert_eq!(
        comments_after[0].contents, "updated text",
        "contents should be updated"
    );
}

#[test]
fn delete_annotation_removes_highlight() {
    let mut doc = build_test_doc();
    let page = 1u32;

    doc.add_highlight(page, [10.0, 10.0, 50.0, 20.0], [1.0, 0.0, 0.0])
        .unwrap();

    let highlights = read_page_highlights(&doc, page).unwrap();
    assert_eq!(highlights.len(), 1);
    let parts: Vec<&str> = highlights[0].id.split('-').collect();
    let obj_num: u32 = parts[0].parse().unwrap();
    let gen_num: u16 = parts[1].parse().unwrap();

    doc.delete_annotation(page, (obj_num, gen_num))
        .expect("delete should succeed");

    let highlights_after = read_page_highlights(&doc, page).unwrap();
    assert_eq!(highlights_after.len(), 0, "highlight should be deleted");
}

#[test]
fn delete_annotation_removes_comment() {
    let mut doc = build_test_doc();
    let page = 1u32;

    doc.add_text_comment(page, [10.0, 10.0, 50.0, 20.0], [1.0, 0.0, 0.0], "to delete")
        .unwrap();

    let comments = read_page_comments(&doc, page).unwrap();
    assert_eq!(comments.len(), 1);
    let parts: Vec<&str> = comments[0].id.split('-').collect();
    let obj_num: u32 = parts[0].parse().unwrap();
    let gen_num: u16 = parts[1].parse().unwrap();

    doc.delete_annotation(page, (obj_num, gen_num)).unwrap();

    let comments_after = read_page_comments(&doc, page).unwrap();
    assert_eq!(comments_after.len(), 0, "comment should be deleted");
}

#[test]
fn mixed_highlights_and_comments_are_distinguished() {
    let mut doc = build_test_doc();
    let page = 1u32;

    doc.add_highlight(page, [10.0, 10.0, 50.0, 20.0], [1.0, 0.0, 0.0])
        .unwrap();
    doc.add_text_comment(page, [20.0, 20.0, 60.0, 30.0], [0.0, 1.0, 0.0], "note")
        .unwrap();
    doc.add_highlight(page, [30.0, 30.0, 70.0, 40.0], [0.0, 0.0, 1.0])
        .unwrap();

    let highlights = read_page_highlights(&doc, page).unwrap();
    let comments = read_page_comments(&doc, page).unwrap();
    assert_eq!(highlights.len(), 2, "should find 2 highlights");
    assert_eq!(comments.len(), 1, "should find 1 comment");
}

// ── Page Operations ──────────────────────────────────────────────

#[test]
fn delete_page_reduces_count() {
    let mut doc = build_two_page_doc();
    let pages = doc.get_pages();
    assert_eq!(pages.len(), 2, "should have 2 pages before delete");

    doc.delete_page(2).expect("delete_page should succeed");

    let pages_after = doc.get_pages();
    assert!(
        !pages_after.contains_key(&2u32),
        "page 2 should be deleted"
    );
}

#[test]
fn rotate_page_succeeds() {
    let mut doc = build_test_doc();
    doc.rotate_page(1, 90)
        .expect("rotate_page should succeed");
}

// ── Metadata ─────────────────────────────────────────────────────

#[test]
fn update_metadata_round_trips() {
    let mut doc = build_test_doc();

    doc.update_metadata("Test Title", "Test Author", "Test Subject", "test, keywords")
        .expect("update_metadata should succeed");

    let info_id = doc
        .trailer
        .get(b"Info")
        .and_then(|v| v.as_reference())
        .ok();
    assert!(info_id.is_some(), "Info dict should exist after update");

    let info_dict = doc.get_dictionary(info_id.unwrap()).unwrap();

    let title = info_dict
        .get(b"Title")
        .and_then(|v| v.as_str().map(|b| String::from_utf8_lossy(b).to_string()))
        .unwrap_or_default();
    assert_eq!(title, "Test Title");

    let author = info_dict
        .get(b"Author")
        .and_then(|v| v.as_str().map(|b| String::from_utf8_lossy(b).to_string()))
        .unwrap_or_default();
    assert_eq!(author, "Test Author");

    let subject = info_dict
        .get(b"Subject")
        .and_then(|v| v.as_str().map(|b| String::from_utf8_lossy(b).to_string()))
        .unwrap_or_default();
    assert_eq!(subject, "Test Subject");

    let keywords = info_dict
        .get(b"Keywords")
        .and_then(|v| v.as_str().map(|b| String::from_utf8_lossy(b).to_string()))
        .unwrap_or_default();
    assert_eq!(keywords, "test, keywords");
}

// ── Save / Reload persistence ────────────────────────────────────

#[test]
fn annotations_persist_through_save_and_reload() {
    let mut doc = build_test_doc();
    let page = 1u32;

    doc.add_highlight(page, [10.0, 10.0, 50.0, 20.0], [1.0, 0.0, 0.0])
        .unwrap();
    doc.add_text_comment(
        page,
        [20.0, 20.0, 60.0, 30.0],
        [0.0, 1.0, 0.0],
        "persistent note",
    )
    .unwrap();

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test_save.pdf");
    let mut buf = Vec::new();
    doc.save_to(&mut buf)
        .expect("save_to should succeed");
    std::fs::write(&path, &buf).unwrap();

    let reloaded = lopdf::Document::load(&path).expect("reloaded PDF should be valid");

    let highlights = read_page_highlights(&reloaded, page).unwrap();
    assert_eq!(highlights.len(), 1, "highlight should survive round-trip");

    let comments = read_page_comments(&reloaded, page).unwrap();
    assert_eq!(comments.len(), 1, "comment should survive round-trip");
    assert_eq!(comments[0].contents, "persistent note");
}

#[test]
fn metadata_persists_through_save_and_reload() {
    let mut doc = build_test_doc();
    doc.update_metadata("Saved Title", "Saved Author", "", "")
        .unwrap();

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test_meta.pdf");
    let mut buf = Vec::new();
    doc.save_to(&mut buf).unwrap();
    std::fs::write(&path, &buf).unwrap();

    let reloaded = lopdf::Document::load(&path).unwrap();
    let info_id = reloaded
        .trailer
        .get(b"Info")
        .and_then(|v| v.as_reference());
    assert!(info_id.is_ok(), "Info dict should exist after reload");

    let info_dict = reloaded.get_dictionary(info_id.unwrap()).unwrap();
    let title = info_dict
        .get(b"Title")
        .and_then(|v| v.as_str().map(|b| String::from_utf8_lossy(b).to_string()))
        .unwrap_or_default();
    assert_eq!(title, "Saved Title");
}

// ── Edge cases ───────────────────────────────────────────────────

#[test]
fn read_highlights_on_empty_page_returns_empty() {
    let doc = build_test_doc();
    let highlights = read_page_highlights(&doc, 1).unwrap();
    assert!(
        highlights.is_empty(),
        "page with no annotations should return empty vec"
    );
}

#[test]
fn read_comments_on_empty_page_returns_empty() {
    let doc = build_test_doc();
    let comments = read_page_comments(&doc, 1).unwrap();
    assert!(
        comments.is_empty(),
        "page with no comments should return empty vec"
    );
}

#[test]
fn read_highlights_on_nonexistent_page_errors() {
    let doc = build_test_doc();
    let result = read_page_highlights(&doc, 999);
    assert!(result.is_err(), "reading non-existent page should error");
}

#[test]
fn delete_nonexistent_annotation_does_not_panic() {
    let mut doc = build_test_doc();
    let _ = doc.delete_annotation(1, (99999, 0));
}

#[test]
fn update_comment_with_valid_contents() {
    let mut doc = build_test_doc();

    doc.add_text_comment(1, [10.0, 10.0, 50.0, 20.0], [1.0, 0.0, 0.0], "original")
        .unwrap();

    let comments = read_page_comments(&doc, 1).unwrap();
    let parts: Vec<&str> = comments[0].id.split('-').collect();
    let obj_num: u32 = parts[0].parse().unwrap();
    let gen_num: u16 = parts[1].parse().unwrap();

    doc.update_text_comment(1, (obj_num, gen_num), "not empty")
        .unwrap();
    let updated = read_page_comments(&doc, 1).unwrap();
    assert_eq!(updated[0].contents, "not empty");
}
