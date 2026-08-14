use crate::infrastructure::pdf::pdf_write_font::SystemFont;
use lopdf::{Dictionary, Document, Object, Stream};
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

use super::sanitize_pdf_name;

/// Embed `font` into the page's resources as a Type0 font (if not already
/// present under the derived alias) and return the resource key to use for
/// writing text.
pub(crate) fn ensure_font_in_page(
    doc: &mut Document,
    page_id: lopdf::ObjectId,
    font: &SystemFont,
    text: &str,
) -> Result<Vec<u8>, String> {
    let alias = font_alias(&font.post_script_name, text);
    let (mut page_dict, res_id) = page_resources(doc, page_id)?;
    let mut res_dict = page_resource_dictionary(doc, &page_dict)?;
    let font_ref = res_dict
        .get(b"Font")
        .ok()
        .and_then(|value| value.as_reference().ok());
    let mut font_dict = if let Some(id) = font_ref {
        doc.get_dictionary(id)
            .map_err(|err| err.to_string())?
            .clone()
    } else {
        res_dict
            .get(b"Font")
            .ok()
            .and_then(|value| value.as_dict().ok())
            .cloned()
            .unwrap_or_else(Dictionary::new)
    };

    if font_dict.get(&alias).is_err() {
        let font_obj = type0_font_object(doc, font)?;
        font_dict.set(alias.clone(), Object::Reference(font_obj));
        println!(
            "[PDF-WRITE-FONT][embed] alias={} ps='{}' source={} glyphs={} bytes={}",
            String::from_utf8_lossy(&alias),
            font.post_script_name,
            font.source_label,
            font.glyphs.len(),
            font.font_bytes.len()
        );
    }

    if let Some(id) = font_ref {
        doc.set_object(id, Object::Dictionary(font_dict));
    } else {
        res_dict.set("Font", Object::Dictionary(font_dict));
    }

    if let Some(id) = res_id {
        doc.set_object(id, Object::Dictionary(res_dict));
    } else {
        page_dict.set("Resources", Object::Dictionary(res_dict));
        doc.set_object(page_id, Object::Dictionary(page_dict));
    }

    Ok(alias)
}
fn type0_font_object(
    doc: &mut Document,
    font: &SystemFont,
) -> Result<lopdf::ObjectId, String> {
    let font_file_id = {
        let mut dict = Dictionary::new();
        dict.set("Length1", font.font_bytes.len() as i64);
        doc.add_object(Object::Stream(Stream::new(
            dict,
            font.font_bytes.as_ref().clone(),
        )))
    };

    let ps_name = sanitize_pdf_name(&font.post_script_name);
    let mut descriptor = Dictionary::new();
    descriptor.set("Type", Object::Name(b"FontDescriptor".to_vec()));
    descriptor.set("FontName", Object::Name(ps_name.as_bytes().to_vec()));
    descriptor.set(
        "Flags",
        if font.italic_angle.abs() > 0.1 {
            68
        } else {
            4
        },
    );
    descriptor.set(
        "FontBBox",
        Object::Array(
            font.bbox
                .iter()
                .map(|value| Object::Real(*value))
                .collect(),
        ),
    );
    descriptor.set("ItalicAngle", Object::Real(font.italic_angle));
    descriptor.set("Ascent", Object::Real(font.ascent));
    descriptor.set("Descent", Object::Real(font.descent));
    descriptor.set("CapHeight", Object::Real(font.cap_height));
    descriptor.set(
        "StemV",
        Object::Integer(if font.weight >= 700 { 120 } else { 80 }),
    );
    descriptor.set("FontFile2", Object::Reference(font_file_id));
    let descriptor_id = doc.add_object(Object::Dictionary(descriptor));

    let mut descendant = Dictionary::new();
    descendant.set("Type", Object::Name(b"Font".to_vec()));
    descendant.set("Subtype", Object::Name(b"CIDFontType2".to_vec()));
    descendant.set("BaseFont", Object::Name(ps_name.as_bytes().to_vec()));
    descendant.set("CIDToGIDMap", Object::Name(b"Identity".to_vec()));
    descendant.set("DW", Object::Integer(1000));
    descendant.set("W", Object::Array(width_array(font)));
    descendant.set("FontDescriptor", Object::Reference(descriptor_id));

    let mut cid_system_info = Dictionary::new();
    cid_system_info.set("Registry", Object::string_literal("Adobe"));
    cid_system_info.set("Ordering", Object::string_literal("Identity"));
    cid_system_info.set("Supplement", Object::Integer(0));
    descendant.set("CIDSystemInfo", Object::Dictionary(cid_system_info));
    let descendant_id = doc.add_object(Object::Dictionary(descendant));

    let to_unicode_id = doc.add_object(Object::Stream(Stream::new(
        Dictionary::new(),
        to_unicode_cmap(&ps_name, font).into_bytes(),
    )));

    let mut type0 = Dictionary::new();
    type0.set("Type", Object::Name(b"Font".to_vec()));
    type0.set("Subtype", Object::Name(b"Type0".to_vec()));
    type0.set("BaseFont", Object::Name(ps_name.as_bytes().to_vec()));
    type0.set("Encoding", Object::Name(b"Identity-H".to_vec()));
    type0.set(
        "DescendantFonts",
        Object::Array(vec![Object::Reference(descendant_id)]),
    );
    type0.set("ToUnicode", Object::Reference(to_unicode_id));

    Ok(doc.add_object(Object::Dictionary(type0)))
}

/// The `W` array: consecutive glyph ids collapsed into a single run.
fn width_array(font: &SystemFont) -> Vec<Object> {
    let mut sorted = font
        .glyphs
        .iter()
        .map(|(_, gid, width)| (*gid as u32, *width))
        .collect::<Vec<_>>();
    sorted.sort_by_key(|(gid, _)| *gid);
    sorted.dedup_by_key(|(gid, _)| *gid);

    let mut out = Vec::new();
    let mut index = 0;
    while index < sorted.len() {
        let start = sorted[index].0;
        let mut widths = vec![Object::Real(sorted[index].1)];
        let mut next = index + 1;
        while next < sorted.len() && sorted[next].0 == sorted[next - 1].0 + 1 {
            widths.push(Object::Real(sorted[next].1));
            next += 1;
        }
        out.push(Object::Integer(start as i64));
        out.push(Object::Array(widths));
        index = next;
    }
    out
}

/// ToUnicode CMap mapping each glyph id back to its character, batched 100
/// entries per bfchar chunk.
fn to_unicode_cmap(ps_name: &str, font: &SystemFont) -> String {
    let mut entries = BTreeMap::<u16, String>::new();
    for (ch, gid, _) in &font.glyphs {
        entries.entry(*gid).or_insert_with(|| utf16be_hex(*ch));
    }

    let mut out = String::new();
    out.push_str("/CIDInit /ProcSet findresource begin\n");
    out.push_str("12 dict begin\nbegincmap\n");
    out.push_str("/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n");
    out.push_str(&format!("/CName /{}-UCS def\n", ps_name));
    out.push_str("/CMapType 2 def\n");
    out.push_str("1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n");

    for chunk in entries.into_iter().collect::<Vec<_>>().chunks(100) {
        out.push_str(&format!("{} beginbfchar\n", chunk.len()));
        for (gid, unicode_hex) in chunk {
            out.push_str(&format!("<{:04X}> <{}>\n", gid, unicode_hex));
        }
        out.push_str("endbfchar\n");
    }

    out.push_str("endcmap\nCMapName currentdict /CMap defineresource pop\nend end\n");
    out
}
fn utf16be_hex(ch: char) -> String {
    let mut tmp = [0u16; 2];
    ch.encode_utf16(&mut tmp)
        .iter()
        .map(|unit| format!("{:04X}", unit))
        .collect::<Vec<_>>()
        .join("")
}
fn page_resources(
    doc: &Document,
    page_id: lopdf::ObjectId,
) -> Result<(Dictionary, Option<lopdf::ObjectId>), String> {
    let page_dict = doc
        .get_dictionary(page_id)
        .map_err(|err| err.to_string())?
        .clone();
    let res_id = page_dict
        .get(b"Resources")
        .ok()
        .and_then(|value| value.as_reference().ok());
    Ok((page_dict, res_id))
}
fn page_resource_dictionary(doc: &Document, page_dict: &Dictionary) -> Result<Dictionary, String> {
    if let Ok(Object::Reference(id)) = page_dict.get(b"Resources") {
        return doc
            .get_dictionary(*id)
            .map(|dict| dict.clone())
            .map_err(|err| err.to_string());
    }
    Ok(page_dict
        .get(b"Resources")
        .ok()
        .and_then(|value| value.as_dict().ok())
        .cloned()
        .unwrap_or_else(Dictionary::new))
}

/// Resource key for an embedded font: derived from the postscript name and
/// the sorted, deduplicated character set of the target text.
fn font_alias(ps_name: &str, text: &str) -> Vec<u8> {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    ps_name.hash(&mut hasher);
    let mut chars = text
        .chars()
        .filter(|ch| !matches!(ch, '\n' | '\r' | '\t'))
        .collect::<Vec<_>>();
    chars.sort_unstable();
    chars.dedup();
    chars.hash(&mut hasher);
    let hash = hasher.finish();
    format!("HSAW_{}_{:016X}", sanitize_pdf_name(ps_name), hash).into_bytes()
}

#[cfg(test)]
mod embed_tests {
    use super::*;

    fn system_font(glyphs: Vec<(char, u16, f32)>) -> SystemFont {
        SystemFont {
            family_name: "Test".to_string(),
            post_script_name: "TestFont".to_string(),
            source_label: "test".to_string(),
            font_bytes: std::sync::Arc::new(vec![]),
            face_index: 0,
            glyphs,
            bbox: [0.0; 4],
            ascent: 0.0,
            descent: 0.0,
            cap_height: 0.0,
            italic_angle: 0.0,
            weight: 400,
        }
    }

    #[test]
    fn width_array_groups_consecutive_gids_and_splits_gaps() {
        let font = system_font(vec![('a', 10, 500.0), ('b', 11, 510.0), ('c', 12, 520.0), ('x', 20, 600.0)]);
        let arr = width_array(&font);
        assert_eq!(arr.len(), 4);
        assert_eq!(arr[0], Object::Integer(10));
        match &arr[1] {
            Object::Array(run) => {
                assert_eq!(run.len(), 3);
                assert_eq!(run[2], Object::Real(520.0));
            }
            other => panic!("expected array run, got {:?}", other),
        }
        assert_eq!(arr[2], Object::Integer(20));
        match &arr[3] {
            Object::Array(run) => assert_eq!(run.len(), 1),
            other => panic!("expected singleton run, got {:?}", other),
        }
    }

    #[test]
    fn width_array_keeps_first_width_for_duplicate_gid() {
        let font = system_font(vec![('a', 7, 500.0), ('b', 7, 900.0)]);
        let arr = width_array(&font);
        assert_eq!(arr.len(), 2);
        match &arr[1] {
            Object::Array(run) => assert_eq!(run[0], Object::Real(500.0)),
            other => panic!("expected array run, got {:?}", other),
        }
    }

    #[test]
    fn font_alias_is_deterministic_and_charset_order_invariant() {
        let a1 = font_alias("TestFont", "abc");
        let a2 = font_alias("TestFont", "cba");
        let a3 = font_alias("TestFont", "abcd");
        assert_eq!(a1, a2);
        assert_ne!(a1, a3);
    }

    #[test]
    fn font_alias_ignores_whitespace_and_duplicates() {
        let a1 = font_alias("TestFont", "a\nb");
        let a2 = font_alias("TestFont", "ab");
        let a3 = font_alias("TestFont", "aaab");
        assert_eq!(a1, a2);
        assert_eq!(a1, a3);
    }

    #[test]
    fn to_unicode_cmap_maps_gids_to_bmp_and_astral_chars() {
        let font = system_font(vec![('A', 5, 0.0), ('中', 6, 0.0), ('😀', 7, 0.0)]);
        let cmap = to_unicode_cmap("TestFont", &font);
        assert!(cmap.starts_with("/CIDInit /ProcSet findresource begin\n"));
        assert!(cmap.ends_with("endcmap\nCMapName currentdict /CMap defineresource pop\nend end\n"));
        assert!(cmap.contains("3 beginbfchar"));
        assert!(cmap.contains("<0005> <0041>"));
        assert!(cmap.contains("<0006> <4E2D>"));
        assert!(cmap.contains("<0007> <D83DDE00>"));
    }

    #[test]
    fn to_unicode_cmap_batches_at_100_entries() {
        let glyphs = (0..250)
            .map(|g| (char::from_u32(0xE000 + g as u32).unwrap(), g as u16, 500.0))
            .collect();
        let font = system_font(glyphs);
        let cmap = to_unicode_cmap("TestFont", &font);
        assert!(cmap.contains("100 beginbfchar"));
        assert!(cmap.contains("50 beginbfchar"));
        assert!(!cmap.contains("250 beginbfchar"));
    }

    #[test]
    fn utf16be_hex_handles_bmp_and_astral() {
        assert_eq!(utf16be_hex('A'), "0041");
        assert_eq!(utf16be_hex('中'), "4E2D");
        assert_eq!(utf16be_hex('😀'), "D83DDE00");
    }
}