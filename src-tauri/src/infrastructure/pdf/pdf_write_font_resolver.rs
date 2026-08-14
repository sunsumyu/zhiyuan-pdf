use crate::infrastructure::pdf::font::ttc::extract_ttc_face_as_ttf;
use crate::infrastructure::pdf::pdf_font::{CMap, ParsedFont};
use fontdb::{Database, Family, Query, Source, Stretch, Style, Weight};
use lopdf::{Dictionary, Document, Object, Stream};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use ttf_parser::{Face, GlyphId};

#[derive(Clone)]
pub struct PdfTextWriteFont {
    pub font_alias: Vec<u8>,
    pub parsed_font: ParsedFont,
    encoding: PdfTextWriteEncoding,
    source_label: String,
}

#[derive(Clone)]
enum PdfTextWriteEncoding {
    OriginalPdfFont,
    TrueTypeGlyphIds {
        font_bytes: Arc<Vec<u8>>,
        face_index: u32,
    },
}
impl PdfTextWriteFont {
    pub fn encode_text(&self, text: &str) -> Result<Vec<u8>, String> {
        match &self.encoding {
            PdfTextWriteEncoding::OriginalPdfFont => Ok(self.parsed_font.encode_text(text)),
            PdfTextWriteEncoding::TrueTypeGlyphIds {
                font_bytes,
                face_index,
            } => encode_text_as_glyph_ids(font_bytes, *face_index, text),
        }
    }
    pub fn source_label(&self) -> &str {
        &self.source_label
    }
}
struct ResolvedFontProgram {
    family_name: String,
    post_script_name: String,
    source_label: String,
    font_bytes: Arc<Vec<u8>>,
    face_index: u32,
    glyphs: Vec<(char, u16, f32)>,
    bbox: [f32; 4],
    ascent: f32,
    descent: f32,
    cap_height: f32,
    italic_angle: f32,
    weight: i32,
}
pub fn resolve_text_write_font(
    doc: &mut Document,
    page_id: lopdf::ObjectId,
    current_font_alias: &[u8],
    current_font: Option<&ParsedFont>,
    text: &str,
) -> Result<PdfTextWriteFont, String> {
    if let Some(font) = current_font {
        if can_pdf_font_encode_text(font, text) {
            println!(
                "[PDF-WRITE-FONT][reuse-original] font='{}' alias={} text='{}'",
                font.name,
                String::from_utf8_lossy(current_font_alias),
                truncate_log(text, 80)
            );
            return Ok(PdfTextWriteFont {
                font_alias: current_font_alias.to_vec(),
                parsed_font: font.clone(),
                encoding: PdfTextWriteEncoding::OriginalPdfFont,
                source_label: format!("original-pdf-font:{}", font.name),
            });
        }
    }

    let preferred_names = build_preferred_font_names(current_font);
    let resolved = resolve_full_font_program(&preferred_names, text)?;
    let alias = ensure_resolved_font_in_page(doc, page_id, &resolved, text)?;
    let parsed_font = parsed_font_from_resolved_program(&resolved);

    println!(
        "[PDF-WRITE-FONT][resolved] alias={} source={} family='{}' ps='{}' text='{}'",
        String::from_utf8_lossy(&alias),
        resolved.source_label,
        resolved.family_name,
        resolved.post_script_name,
        truncate_log(text, 80)
    );

    Ok(PdfTextWriteFont {
        font_alias: alias,
        parsed_font,
        encoding: PdfTextWriteEncoding::TrueTypeGlyphIds {
            font_bytes: resolved.font_bytes,
            face_index: resolved.face_index,
        },
        source_label: resolved.source_label,
    })
}
fn can_pdf_font_encode_text(font: &ParsedFont, text: &str) -> bool {
    let mut has_visible = false;
    for ch in text.chars() {
        if ch == '\n' || ch == '\r' || ch == '\t' {
            continue;
        }
        has_visible = true;
        if !font.can_encode(ch) {
            return false;
        }
    }
    has_visible
}
fn build_preferred_font_names(current_font: Option<&ParsedFont>) -> Vec<String> {
    let mut names = Vec::new();
    if let Some(font) = current_font {
        push_font_name_variants(&mut names, &font.name);
        if let Some(ps) = &font.post_script_name {
            push_font_name_variants(&mut names, ps);
        }
        if let Some(family) = &font.family_hint {
            push_font_name_variants(&mut names, family);
        }
    }

    for fallback in [
        "Microsoft YaHei",
        "寰蒋闆呴粦",
        "SimSun",
        "瀹嬩綋",
        "SimHei",
        "榛戜綋",
        "Noto Sans CJK SC",
        "Source Han Sans SC",
        "Arial Unicode MS",
        "Arial",
    ] {
        push_unique(&mut names, fallback.to_string());
    }

    names
}
fn push_font_name_variants(out: &mut Vec<String>, name: &str) {
    let stripped = strip_subset_prefix(name).trim();
    if stripped.is_empty() {
        return;
    }
    push_unique(out, stripped.to_string());
    push_unique(out, stripped.replace('-', " "));
    push_unique(out, stripped.replace('_', " "));
    push_unique(out, stripped.replace(' ', ""));

    let lower = stripped.to_ascii_lowercase();
    if lower.contains("microsoftyahei") || lower.contains("msyh") || stripped == "寰蒋闆呴粦"
    {
        push_unique(out, "Microsoft YaHei".to_string());
        push_unique(out, "寰蒋闆呴粦".to_string());
    }
    if lower.contains("simsun") || stripped == "瀹嬩綋" {
        push_unique(out, "SimSun".to_string());
        push_unique(out, "瀹嬩綋".to_string());
    }
    if lower.contains("simhei") || stripped == "榛戜綋" {
        push_unique(out, "SimHei".to_string());
        push_unique(out, "榛戜綋".to_string());
    }
}
fn push_unique(out: &mut Vec<String>, value: String) {
    if value.trim().is_empty() {
        return;
    }
    if !out
        .iter()
        .any(|existing| existing.eq_ignore_ascii_case(&value))
    {
        out.push(value);
    }
}
fn strip_subset_prefix(name: &str) -> &str {
    if name.len() > 7
        && name.as_bytes().get(6) == Some(&b'+')
        && name[..6].chars().all(|ch| ch.is_ascii_uppercase())
    {
        &name[7..]
    } else {
        name
    }
}
fn resolve_full_font_program(
    preferred_names: &[String],
    text: &str,
) -> Result<ResolvedFontProgram, String> {
    let mut db = Database::new();
    db.load_system_fonts();
    load_managed_font_dirs(&mut db);

    let mut tried = Vec::new();
    for family in preferred_names {
        tried.push(family.clone());
        if let Some(resolved) = resolve_family_from_db(&db, family, text) {
            return Ok(resolved);
        }
    }

    let missing = describe_missing_from_candidate_pool(&db, text);
    Err(format!(
        "PdfWriteFontResolver failed: no usable full font covers target text. tried=[{}] missing={}",
        tried.join(", "),
        missing
    ))
}
fn load_managed_font_dirs(db: &mut Database) {
    for dir in [
        "assets/fonts",
        "resources/fonts",
        "src-tauri/fonts",
        "src-tauri/resources/fonts",
    ] {
        let path = Path::new(dir);
        if path.exists() {
            db.load_fonts_dir(path);
        }
    }
}
fn resolve_family_from_db(db: &Database, family: &str, text: &str) -> Option<ResolvedFontProgram> {
    let families = [Family::Name(family)];
    let query = Query {
        families: &families,
        weight: Weight::NORMAL,
        stretch: Stretch::Normal,
        style: Style::Normal,
    };
    let Some(id) = db.query(&query) else {
        return try_known_font_files(family, text, 0);
    };
    let (source, face_index) = db.face_source(id)?;
    let source_label = source_label(&source, family);
    db.with_face_data(id, |data, index| {
        build_resolved_program_from_face_data(data, index, family, &source_label, text)
    })?
    .or_else(|| try_known_font_files(family, text, face_index))
}
fn try_known_font_files(
    family: &str,
    text: &str,
    preferred_face_index: u32,
) -> Option<ResolvedFontProgram> {
    let lower = family.to_ascii_lowercase();
    let mut paths = Vec::<PathBuf>::new();
    if lower.contains("yahei") || family == "寰蒋闆呴粦" || lower.contains("microsoft") {
        paths.extend([
            PathBuf::from(r"C:\Windows\Fonts\msyh.ttc"),
            PathBuf::from(r"C:\Windows\Fonts\msyh.ttf"),
        ]);
    }
    if lower.contains("simsun") || family == "瀹嬩綋" {
        paths.extend([
            PathBuf::from(r"C:\Windows\Fonts\simsun.ttc"),
            PathBuf::from(r"C:\Windows\Fonts\simsun.ttf"),
        ]);
    }
    if lower.contains("simhei") || family == "榛戜綋" {
        paths.push(PathBuf::from(r"C:\Windows\Fonts\simhei.ttf"));
    }

    for path in paths {
        let Ok(data) = std::fs::read(&path) else {
            continue;
        };
        if let Some(program) = build_resolved_program_from_face_data(
            &data,
            preferred_face_index,
            family,
            &format!("known-file:{}", path.display()),
            text,
        ) {
            return Some(program);
        }
    }
    None
}
fn build_resolved_program_from_face_data(
    data: &[u8],
    face_index: u32,
    requested_family: &str,
    source_label: &str,
    text: &str,
) -> Option<ResolvedFontProgram> {
    let standalone = normalize_font_program_for_pdf(data, face_index)?;
    let font_bytes = Arc::new(standalone);
    let face = Face::parse(font_bytes.as_slice(), 0).ok()?;
    if !font_covers_text(&face, text) {
        println!(
            "[PDF-WRITE-FONT][reject-coverage] family='{}' source={} missing={}",
            requested_family,
            source_label,
            missing_chars_for_face(&face, text)
        );
        return None;
    }

    let glyphs = collect_glyphs(&face, text)?;
    let units = face.units_per_em().max(1) as f32;
    let bbox = face.global_bounding_box();
    let post_script_name =
        post_script_name(&face).unwrap_or_else(|| sanitize_pdf_name(requested_family));

    let ascent = face.ascender() as f32 / units * 1000.0;
    let descent = face.descender() as f32 / units * 1000.0;
    let italic_angle = face.italic_angle();
    let weight = face.weight().to_number() as i32;

    Some(ResolvedFontProgram {
        family_name: requested_family.to_string(),
        post_script_name,
        source_label: source_label.to_string(),
        font_bytes,
        face_index: 0,
        glyphs,
        bbox: [
            bbox.x_min as f32 / units * 1000.0,
            bbox.y_min as f32 / units * 1000.0,
            bbox.x_max as f32 / units * 1000.0,
            bbox.y_max as f32 / units * 1000.0,
        ],
        ascent,
        descent,
        cap_height: ascent,
        italic_angle,
        weight,
    })
}
fn normalize_font_program_for_pdf(data: &[u8], face_index: u32) -> Option<Vec<u8>> {
    match data.get(0..4) {
        Some(b"\x00\x01\x00\x00") | Some(b"true") => Some(data.to_vec()),
        Some(b"ttcf") => extract_ttc_face_as_ttf(data, face_index).ok(),
        Some(b"OTTO") => {
            println!("[PDF-WRITE-FONT][reject-cff] OpenType CFF requires FontFile3 writer");
            None
        }
        _ => None,
    }
}
fn font_covers_text(face: &Face<'_>, text: &str) -> bool {
    text.chars()
        .all(|ch| ch == '\n' || ch == '\r' || ch == '\t' || face.glyph_index(ch).is_some())
}
fn missing_chars_for_face(face: &Face<'_>, text: &str) -> String {
    let mut missing = BTreeSet::new();
    for ch in text.chars() {
        if ch == '\n' || ch == '\r' || ch == '\t' {
            continue;
        }
        if face.glyph_index(ch).is_none() {
            missing.insert(format!("'{}'(U+{:04X})", ch, ch as u32));
        }
    }
    missing.into_iter().collect::<Vec<_>>().join(", ")
}
fn describe_missing_from_candidate_pool(db: &Database, text: &str) -> String {
    let sample = text
        .chars()
        .filter(|ch| !matches!(ch, '\n' | '\r' | '\t'))
        .map(|ch| format!("'{}'(U+{:04X})", ch, ch as u32))
        .take(24)
        .collect::<Vec<_>>()
        .join(", ");
    format!("target_chars=[{}] font_faces={}", sample, db.len())
}
fn collect_glyphs(face: &Face<'_>, text: &str) -> Option<Vec<(char, u16, f32)>> {
    let mut seen = HashSet::<u16>::new();
    let mut out = Vec::new();
    let units = face.units_per_em().max(1) as f32;
    for ch in text.chars() {
        if ch == '\n' || ch == '\r' || ch == '\t' {
            continue;
        }
        let gid = face.glyph_index(ch)?.0;
        if seen.insert(gid) {
            let advance = face
                .glyph_hor_advance(GlyphId(gid))
                .unwrap_or(face.units_per_em());
            out.push((ch, gid, advance as f32 / units * 1000.0));
        }
    }
    Some(out)
}
fn encode_text_as_glyph_ids(
    font_bytes: &[u8],
    face_index: u32,
    text: &str,
) -> Result<Vec<u8>, String> {
    let face = Face::parse(font_bytes, face_index)
        .map_err(|err| format!("failed to parse resolved write font: {:?}", err))?;
    let mut encoded = Vec::new();
    for ch in text.chars() {
        if ch == '\n' || ch == '\r' || ch == '\t' {
            continue;
        }
        let gid = face
            .glyph_index(ch)
            .ok_or_else(|| {
                format!(
                    "resolved write font cannot encode '{}'(U+{:04X})",
                    ch, ch as u32
                )
            })?
            .0;
        encoded.push((gid >> 8) as u8);
        encoded.push((gid & 0xFF) as u8);
    }
    Ok(encoded)
}
fn parsed_font_from_resolved_program(resolved: &ResolvedFontProgram) -> ParsedFont {
    let mut widths = std::collections::HashMap::new();
    let mut pairs = Vec::new();
    for (ch, gid, width) in &resolved.glyphs {
        widths.insert(*gid as u32, *width);
        widths.insert(*ch as u32, *width);
        pairs.push((*gid, ch.to_string()));
    }
    ParsedFont {
        name: resolved.family_name.clone(),
        base_font: resolved.post_script_name.clone(),
        post_script_name: Some(resolved.post_script_name.clone()),
        family_hint: Some(resolved.family_name.clone()),
        font_subtype: Some("Type0".to_string()),
        embedded_font_key: Some(resolved.source_label.clone()),
        has_embedded_font_file: true,
        has_to_unicode_cmap: true,
        widths,
        default_width: 1000.0,
        cmap: Some(CMap::from_codepoint_pairs(pairs)),
        hints: None,
    }
}
fn ensure_resolved_font_in_page(
    doc: &mut Document,
    page_id: lopdf::ObjectId,
    resolved: &ResolvedFontProgram,
    text: &str,
) -> Result<Vec<u8>, String> {
    let alias = build_font_alias(&resolved.post_script_name, text);
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
        let font_obj = build_type0_font_object(doc, resolved)?;
        font_dict.set(alias.clone(), Object::Reference(font_obj));
        println!(
            "[PDF-WRITE-FONT][embed] alias={} ps='{}' source={} glyphs={} bytes={}",
            String::from_utf8_lossy(&alias),
            resolved.post_script_name,
            resolved.source_label,
            resolved.glyphs.len(),
            resolved.font_bytes.len()
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
fn build_type0_font_object(
    doc: &mut Document,
    resolved: &ResolvedFontProgram,
) -> Result<lopdf::ObjectId, String> {
    let font_file_id = {
        let mut dict = Dictionary::new();
        dict.set("Length1", resolved.font_bytes.len() as i64);
        doc.add_object(Object::Stream(Stream::new(
            dict,
            resolved.font_bytes.as_ref().clone(),
        )))
    };

    let ps_name = sanitize_pdf_name(&resolved.post_script_name);
    let mut descriptor = Dictionary::new();
    descriptor.set("Type", Object::Name(b"FontDescriptor".to_vec()));
    descriptor.set("FontName", Object::Name(ps_name.as_bytes().to_vec()));
    descriptor.set(
        "Flags",
        if resolved.italic_angle.abs() > 0.1 {
            68
        } else {
            4
        },
    );
    descriptor.set(
        "FontBBox",
        Object::Array(
            resolved
                .bbox
                .iter()
                .map(|value| Object::Real(*value))
                .collect(),
        ),
    );
    descriptor.set("ItalicAngle", Object::Real(resolved.italic_angle));
    descriptor.set("Ascent", Object::Real(resolved.ascent));
    descriptor.set("Descent", Object::Real(resolved.descent));
    descriptor.set("CapHeight", Object::Real(resolved.cap_height));
    descriptor.set(
        "StemV",
        Object::Integer(if resolved.weight >= 700 { 120 } else { 80 }),
    );
    descriptor.set("FontFile2", Object::Reference(font_file_id));
    let descriptor_id = doc.add_object(Object::Dictionary(descriptor));

    let mut descendant = Dictionary::new();
    descendant.set("Type", Object::Name(b"Font".to_vec()));
    descendant.set("Subtype", Object::Name(b"CIDFontType2".to_vec()));
    descendant.set("BaseFont", Object::Name(ps_name.as_bytes().to_vec()));
    descendant.set("CIDToGIDMap", Object::Name(b"Identity".to_vec()));
    descendant.set("DW", Object::Integer(1000));
    descendant.set("W", Object::Array(build_width_array(resolved)));
    descendant.set("FontDescriptor", Object::Reference(descriptor_id));

    let mut cid_system_info = Dictionary::new();
    cid_system_info.set("Registry", Object::string_literal("Adobe"));
    cid_system_info.set("Ordering", Object::string_literal("Identity"));
    cid_system_info.set("Supplement", Object::Integer(0));
    descendant.set("CIDSystemInfo", Object::Dictionary(cid_system_info));
    let descendant_id = doc.add_object(Object::Dictionary(descendant));

    let to_unicode_id = doc.add_object(Object::Stream(Stream::new(
        Dictionary::new(),
        build_to_unicode_cmap(&ps_name, resolved).into_bytes(),
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
fn build_width_array(resolved: &ResolvedFontProgram) -> Vec<Object> {
    let mut sorted = resolved
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
fn build_to_unicode_cmap(ps_name: &str, resolved: &ResolvedFontProgram) -> String {
    let mut entries = BTreeMap::<u16, String>::new();
    for (ch, gid, _) in &resolved.glyphs {
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
fn build_font_alias(ps_name: &str, text: &str) -> Vec<u8> {
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
fn post_script_name(face: &Face<'_>) -> Option<String> {
    face.names()
        .into_iter()
        .find(|name| name.name_id == ttf_parser::name_id::POST_SCRIPT_NAME)
        .and_then(|name| name.to_string())
}
fn sanitize_pdf_name(value: &str) -> String {
    let mut out = value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-' || *ch == '_')
        .collect::<String>();
    if out.is_empty() {
        out = "HsaWriteFont".to_string();
    }
    if out.len() > 48 {
        out.truncate(48);
    }
    out
}
fn source_label(source: &Source, requested_family: &str) -> String {
    match source {
        Source::File(path) => format!("fontdb:{}:{}", requested_family, path.display()),
        Source::Binary(_) => format!("fontdb:{}:<binary>", requested_family),
        Source::SharedFile(path, _) => format!("fontdb:{}:{}", requested_family, path.display()),
    }
}
fn truncate_log(value: &str, limit: usize) -> String {
    let mut out = String::new();
    let mut chars = value.chars();
    for _ in 0..limit {
        if let Some(ch) = chars.next() {
            out.push(ch);
        } else {
            break;
        }
    }
    if chars.next().is_some() {
        out.push_str("...");
    }
    out
}
