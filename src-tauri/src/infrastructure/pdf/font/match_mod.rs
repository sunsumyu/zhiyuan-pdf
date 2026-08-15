use super::parse::ParsedFont;
use super::SystemFont;
use crate::infrastructure::pdf::models::NativeTextModel;
use pdf_viewer_core::models::FontHints;
use pdf_viewer_core::typography::engine::TypographyEngine;
use pdf_viewer_core::typography::matcher::{
    build_match_request_with_descriptor, choose_top_matches, resolve_system_or_fallback_font,
};
use pdf_viewer_core::typography::models::{
    PdfEmbeddedFontKind, PdfFontDescriptor, PdfFontMatchRequest, PdfFontSourceKind,
    ResolvedPdfFont, SystemFontCandidate,
};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use fontdb::{Database, Family, Query, Source, Stretch, Style, Weight};

// ── PdfSystemFontMatcher (from matching.rs) ──

pub struct PdfSystemFontMatcher {
    candidates: Vec<SystemFontCandidate>,
    cache: HashMap<String, ResolvedPdfFont>,
    logged_keys: HashSet<String>,
    fallback_family: String,
}

impl PdfSystemFontMatcher {
    pub fn new(candidates: Vec<SystemFontCandidate>, fallback_family: impl Into<String>) -> Self {
        Self {
            candidates,
            cache: HashMap::new(),
            logged_keys: HashSet::new(),
            fallback_family: fallback_family.into(),
        }
    }
    pub fn resolve(&mut self, pdf_font_name: &str, hints: Option<&FontHints>) -> ResolvedPdfFont {
        let cache_key = build_cache_key(pdf_font_name, hints);
        if let Some(existing) = self.cache.get(&cache_key) {
            return existing.clone();
        }

        let engine = TypographyEngine::new(&self.candidates, &self.fallback_family);
        let resolved = engine.resolve_pdf_font(pdf_font_name, hints);
        self.maybe_log_resolution(&cache_key, pdf_font_name, &resolved, None);
        self.cache.insert(cache_key, resolved.clone());
        resolved
    }
    pub fn candidate_count(&self) -> usize {
        self.candidates.len()
    }
    pub fn resolve_native_text(&mut self, text: &NativeTextModel) -> ResolvedPdfFont {
        let cache_key = build_native_text_cache_key(text);
        if let Some(existing) = self.cache.get(&cache_key) {
            return existing.clone();
        }

        let descriptor = PdfFontDescriptor {
            source_kind: if text.has_embedded_font_file {
                Some(if text.font_name.contains('+') {
                    PdfFontSourceKind::EmbeddedSubset
                } else {
                    PdfFontSourceKind::EmbeddedFull
                })
            } else {
                None
            },
            embedded_font_kind: map_embedded_font_kind(text.font_subtype.as_deref()),
            font_subtype: text.font_subtype.clone(),
            weight: text
                .font_hints
                .as_ref()
                .map(|value| value.weight)
                .unwrap_or(400),
            is_italic: text
                .font_hints
                .as_ref()
                .map(|value| value.is_italic)
                .unwrap_or(text.is_italic),
            is_fixed_pitch: text
                .font_hints
                .as_ref()
                .map(|value| value.is_fixed_pitch)
                .unwrap_or(false),
            is_serif: text
                .font_hints
                .as_ref()
                .map(|value| value.is_serif)
                .unwrap_or(text.is_serif),
            has_embedded_font_file: text.has_embedded_font_file,
            has_to_unicode_cmap: text.has_to_unicode_cmap,
            post_script_name: text.font_post_script_name.clone(),
            family_hint: text.font_family_hint.clone(),
        };

        let request = build_match_request_with_descriptor(
            &text.font_name,
            text.font_hints.as_ref(),
            descriptor,
        );
        let resolved =
            resolve_system_or_fallback_font(&request, &self.candidates, &self.fallback_family);
        self.maybe_log_resolution(&cache_key, &text.font_name, &resolved, Some(&request));
        self.cache.insert(cache_key, resolved.clone());
        resolved
    }
    fn maybe_log_resolution(
        &mut self,
        cache_key: &str,
        pdf_font_name: &str,
        resolved: &ResolvedPdfFont,
        request: Option<&PdfFontMatchRequest>,
    ) {
        if !self.logged_keys.insert(cache_key.to_string()) {
            return;
        }

        let is_low_confidence = resolved.confidence_score < 100;
        let is_fallback = resolved
            .source_kind
            .as_ref()
            .map(|kind| {
                matches!(
                    kind,
                    pdf_viewer_core::typography::models::PdfFontSourceKind::Fallback
                )
            })
            .unwrap_or(false);

        if !is_low_confidence && !is_fallback {
            return;
        }

        let reasons = if resolved.reasons.is_empty() {
            "no reasons recorded".to_string()
        } else {
            resolved
                .reasons
                .iter()
                .map(|reason| format!("{}:{}({})", reason.code, reason.detail, reason.score_delta))
                .collect::<Vec<_>>()
                .join("; ")
        };

        crate::pdf_log!(
            2,
            "[PDF-FONT-MATCH] request='{}' matched='{}' source={:?} score={} reasons={}",
            pdf_font_name,
            resolved.matched_family.as_deref().unwrap_or("<none>"),
            resolved.source_kind,
            resolved.confidence_score,
            reasons
        );

        if let Some(request) = request {
            let top = choose_top_matches(request, &self.candidates, 3);
            if !top.is_empty() {
                let ranked = top
                    .into_iter()
                    .map(|item| format!("{}:{}", item.candidate.family_name, item.score))
                    .collect::<Vec<_>>()
                    .join(", ");
                crate::pdf_log!(
                    2,
                    "[PDF-FONT-MATCH-TOP] request='{}' top_candidates=[{}]",
                    pdf_font_name, ranked
                );
            }
        }
    }
}

fn build_cache_key(pdf_font_name: &str, hints: Option<&FontHints>) -> String {
    match hints {
        Some(value) => format!(
            "{}|{}|{}|{}|{}|{}",
            pdf_font_name,
            value.weight,
            value.is_italic,
            value.is_fixed_pitch,
            value.is_serif,
            value.is_bold
        ),
        None => format!("{}|no-hints", pdf_font_name),
    }
}

fn build_native_text_cache_key(text: &NativeTextModel) -> String {
    format!(
        "{}|{:?}|{:?}|{:?}|{:?}|{}|{}",
        build_cache_key(&text.font_name, text.font_hints.as_ref()),
        text.font_post_script_name,
        text.font_family_hint,
        text.font_subtype,
        text.embedded_font_key,
        text.has_embedded_font_file,
        text.has_to_unicode_cmap
    )
}

fn map_embedded_font_kind(subtype: Option<&str>) -> Option<PdfEmbeddedFontKind> {
    let subtype = subtype?;
    let lower = subtype.trim().trim_start_matches('/').to_ascii_lowercase();
    match lower.as_str() {
        "type1" => Some(PdfEmbeddedFontKind::Type1),
        "truetype" => Some(PdfEmbeddedFontKind::TrueType),
        "cidfonttype0" | "type0" => Some(PdfEmbeddedFontKind::CidType0),
        "cidfonttype2" => Some(PdfEmbeddedFontKind::CidType2),
        "type1c" | "opentype" => Some(PdfEmbeddedFontKind::OpenType),
        _ => Some(PdfEmbeddedFontKind::Unknown),
    }
}

// ── Finder functions (from finder.rs) ──

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
        "寰蒋闆呴粦",
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
    if lower.contains("microsoftyahei") || lower.contains("msyh") || stripped == "寰蒋闆呴粦" {
        push_unique(&mut variants, "Microsoft YaHei".to_string());
        push_unique(&mut variants, "寰蒋闆呴粦".to_string());
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
        super::face::font_from_bytes(data, index, family, &source_label, text)
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
        if let Some(font) = super::face::font_from_bytes(
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

// ── Tests ──

#[cfg(test)]
mod matcher_tests {
    use super::*;
    use crate::infrastructure::pdf::models::NativeTextModel;

    #[test]
    fn matcher_caches_resolved_font() {
        let candidate = SystemFontCandidate {
            family_name: "SimSun".to_string(),
            coverage_score: 40,
            ..Default::default()
        };
        let mut matcher = PdfSystemFontMatcher::new(vec![candidate], "Microsoft YaHei");

        let first = matcher.resolve("SimSun", None);
        let second = matcher.resolve("SimSun", None);

        assert_eq!(first.matched_family.as_deref(), Some("SimSun"));
        assert_eq!(second.matched_family.as_deref(), Some("SimSun"));
        assert_eq!(matcher.cache.len(), 1);
    }

    #[test]
    fn uses_descriptor_cache() {
        let candidate = SystemFontCandidate {
            family_name: "Microsoft YaHei".to_string(),
            full_name: Some("Microsoft YaHei".to_string()),
            post_script_name: Some("MicrosoftYaHei".to_string()),
            coverage_score: 40,
            ..Default::default()
        };
        let mut matcher = PdfSystemFontMatcher::new(vec![candidate], "Microsoft YaHei");
        let text = NativeTextModel {
            font_name: "寰蒋闆呴粦".to_string(),
            font_post_script_name: Some("MicrosoftYaHei".to_string()),
            has_embedded_font_file: true,
            has_to_unicode_cmap: true,
            font_subtype: Some("TrueType".to_string()),
            ..Default::default()
        };

        let first = matcher.resolve_native_text(&text);
        let second = matcher.resolve_native_text(&text);

        assert_eq!(first.matched_family.as_deref(), Some("Microsoft YaHei"));
        assert_eq!(second.matched_family.as_deref(), Some("Microsoft YaHei"));
        assert_eq!(matcher.cache.len(), 1);
        assert!(first.can_attempt_embedded_render);
    }
}

#[cfg(test)]
mod finder_tests {
    use super::*;
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
        let variants = name_variants("寰蒋闆呴粦");
        assert_eq!(variants, vec!["寰蒋闆呴粦", "Microsoft YaHei"]);
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
