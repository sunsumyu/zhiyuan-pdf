use crate::infrastructure::pdf::pdf_font::ParsedFont;
use crate::infrastructure::pdf::pdf_write_font::SystemFont;
use fontdb::{Database, Family, Query, Source, Stretch, Style, Weight};
use std::path::{Path, PathBuf};

use super::face;

/// Order of font names to look up: variants of the PDF's current font first,
/// then a fixed CJK-aware fallback list.
pub(crate) fn candidate_font_names(current_font: Option<&ParsedFont>) -> Vec<String> {
    let mut names = Vec::new();
    if let Some(font) = current_font {
        for variant in name_variants(&font.name) {
            push_unique(&mut names, variant);
        }
        if let Some(ps) = &font.post_script_name {
            for variant in name_variants(ps) {
                push_unique(&mut names, variant);
            }
        }
        if let Some(family) = &font.family_hint {
            for variant in name_variants(family) {
                push_unique(&mut names, variant);
            }
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

/// All spellings of a font name worth looking up: separators normalized,
/// whitespace stripped, and mojibake (GBK-misdecoded) spellings mapped to
/// the canonical Windows font names they were corrupted from.
fn name_variants(name: &str) -> Vec<String> {
    let mut variants = Vec::new();
    let stripped = strip_subset_prefix(name).trim();
    if stripped.is_empty() {
        return variants;
    }
    push_unique(&mut variants, stripped.to_string());
    push_unique(&mut variants, stripped.replace('-', " "));
    push_unique(&mut variants, stripped.replace('_', " "));
    push_unique(&mut variants, stripped.replace(' ', ""));

    let lower = stripped.to_ascii_lowercase();
    if lower.contains("microsoftyahei") || lower.contains("msyh") || stripped == "寰蒋闆呴粦" {
        push_unique(&mut variants, "Microsoft YaHei".to_string());
        push_unique(&mut variants, "寰蒋闆呴粦".to_string());
    }
    if lower.contains("simsun") || stripped == "瀹嬩綋" {
        push_unique(&mut variants, "SimSun".to_string());
        push_unique(&mut variants, "瀹嬩綋".to_string());
    }
    if lower.contains("simhei") || stripped == "榛戜綋" {
        push_unique(&mut variants, "SimHei".to_string());
        push_unique(&mut variants, "榛戜綋".to_string());
    }
    variants
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
pub(crate) fn strip_subset_prefix(name: &str) -> &str {
    if name.len() > 7
        && name.as_bytes().get(6) == Some(&b'+')
        && name[..6].chars().all(|ch| ch.is_ascii_uppercase())
    {
        &name[7..]
    } else {
        name
    }
}

/// Search the system font database and managed font directories for the
/// first family that covers every character of `text`, preferring the face
/// matching `target_weight` (bold originals resolve to bold faces).
pub(crate) fn find_system_font(
    preferred_names: &[String],
    text: &str,
    target_weight: Weight,
) -> Result<SystemFont, String> {
    let mut db = Database::new();
    db.load_system_fonts();
    load_managed_font_dirs(&mut db);

    let mut tried = Vec::new();
    for family in preferred_names {
        tried.push(family.clone());
        if let Some(font) = query_fontdb(&db, family, text, target_weight) {
            return Ok(font);
        }
    }

    let missing = missing_text_diagnostics(&db, text);
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
fn query_fontdb(
    db: &Database,
    family: &str,
    text: &str,
    target_weight: Weight,
) -> Option<SystemFont> {
    let families = [Family::Name(family)];
    let query = Query {
        families: &families,
        weight: target_weight,
        stretch: Stretch::Normal,
        style: Style::Normal,
    };
    let Some(id) = db.query(&query) else {
        return probe_known_font_files(family, text, 0, target_weight);
    };
    let (source, face_index) = db.face_source(id)?;
    let source_label = source_label(&source, family);
    db.with_face_data(id, |data, index| {
        face::font_from_bytes(data, index, family, &source_label, text)
    })?
    .or_else(|| probe_known_font_files(family, text, face_index, target_weight))
}

/// Fallback for families the font database misses: read the hardcoded
/// Windows font files for the CJK fonts whose names we know, and return
/// the first one that works.
fn probe_known_font_files(
    family: &str,
    text: &str,
    preferred_face_index: u32,
    target_weight: Weight,
) -> Option<SystemFont> {
    let lower = family.to_ascii_lowercase();
    let mut paths = Vec::<PathBuf>::new();
    if lower.contains("yahei") || family == "寰蒋闆呴粦" || lower.contains("microsoft") {
        if target_weight >= Weight::BOLD {
            paths.push(PathBuf::from(r"C:\Windows\Fonts\msyhbd.ttc"));
        }
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
        if let Some(font) = face::font_from_bytes(
            &data,
            preferred_face_index,
            family,
            &format!("known-file:{}", path.display()),
            text,
        ) {
            return Some(font);
        }
    }
    None
}
fn missing_text_diagnostics(db: &Database, text: &str) -> String {
    let sample = text
        .chars()
        .filter(|ch| !matches!(ch, '\n' | '\r' | '\t'))
        .map(|ch| format!("'{}'(U+{:04X})", ch, ch as u32))
        .take(24)
        .collect::<Vec<_>>()
        .join(", ");
    format!("target_chars=[{}] font_faces={}", sample, db.len())
}
fn source_label(source: &Source, requested_family: &str) -> String {
    match source {
        Source::File(path) => format!("fontdb:{}:{}", requested_family, path.display()),
        Source::Binary(_) => format!("fontdb:{}:<binary>", requested_family),
        Source::SharedFile(path, _) => format!("fontdb:{}:{}", requested_family, path.display()),
    }
}

#[cfg(test)]
mod finder_tests {
    use super::*;
    use crate::infrastructure::pdf::pdf_font::ParsedFont;
    use std::collections::HashMap;

    fn parsed_font(name: &str) -> ParsedFont {
        ParsedFont {
            name: name.to_string(),
            base_font: name.to_string(),
            font_subtype: None,
            cmap: None,
            widths: HashMap::new(),
            default_width: 1000.0,
            hints: None,
            post_script_name: None,
            family_hint: None,
            embedded_font_key: None,
            has_embedded_font_file: false,
            has_to_unicode_cmap: false,
        }
    }

    #[test]
    fn strip_subset_prefix_requires_six_uppercase_chars() {
        assert_eq!(strip_subset_prefix("ABCDEF+SimSun"), "SimSun");
        assert_eq!(strip_subset_prefix("SimSun"), "SimSun");
        assert_eq!(strip_subset_prefix("ABCDE+SimSun"), "ABCDE+SimSun");
        assert_eq!(strip_subset_prefix("abcdef+SimSun"), "abcdef+SimSun");
    }

    #[test]
    fn name_variants_expand_separators() {
        assert_eq!(name_variants("ABCDEF+MS_YaHei"), vec!["MS_YaHei", "MS YaHei"]);
        assert_eq!(name_variants("ABCDEF+MS-YaHei"), vec!["MS-YaHei", "MS YaHei"]);
        assert_eq!(name_variants("ABCDEF+MS YaHei"), vec!["MS YaHei", "MSYaHei"]);
    }

    #[test]
    fn name_variants_map_mojibake_yahei_to_canonical() {
        let variants = name_variants("寰蒋闆呴粦");
        assert_eq!(variants, vec!["寰蒋闆呴粦", "Microsoft YaHei"]);
    }

    #[test]
    fn candidate_names_favor_current_font_then_fallbacks() {
        let font = parsed_font("SimSun");
        let names = candidate_font_names(Some(&font));
        assert_eq!(names[0], "SimSun");
        assert!(names.contains(&"瀹嬩綋".to_string()));
        assert_eq!(names.iter().filter(|n| *n == "SimSun").count(), 1);
        assert!(names.contains(&"Microsoft YaHei".to_string()));
    }

    #[test]
    fn candidate_names_dedupe_case_insensitively() {
        let font = parsed_font("simsun");
        let names = candidate_font_names(Some(&font));
        assert!(names.contains(&"simsun".to_string()));
        assert!(!names.contains(&"SimSun".to_string()));
    }

    #[test]
    fn candidate_names_without_current_font_start_with_fallbacks() {
        let names = candidate_font_names(None);
        assert_eq!(names[0], "Microsoft YaHei");
        assert!(names.contains(&"Arial".to_string()));
    }
}
