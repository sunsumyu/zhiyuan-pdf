use std::collections::{HashMap, HashSet};
use pdf_viewer_core::models::FontHints;
use pdf_viewer_core::typography::engine::TypographyEngine;
use pdf_viewer_core::typography::matcher::{
    build_match_request_with_descriptor, choose_top_matches, resolve_system_or_fallback_font,
};
use pdf_viewer_core::typography::models::{
    PdfEmbeddedFontKind, PdfFontDescriptor, PdfFontMatchRequest, PdfFontSourceKind,
    ResolvedPdfFont, SystemFontCandidate,
};
use crate::infrastructure::pdf::models::NativeTextModel;
pub struct PdfSystemFontMatcher {
    candidates: Vec<SystemFontCandidate>,
    cache: HashMap<String, ResolvedPdfFont>,
    logged_keys: HashSet<String>,
    fallback_family: String,
}
impl PdfSystemFontMatcher {
pub fn new(candidates: Vec<SystemFontCandidate>, fallback_family:
impl Into<String>) -> Self {
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
            source_kind: if text.has_embedded_font_program {
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
            has_embedded_program: text.has_embedded_font_program,
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

        println!(
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
                println!(
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
        text.has_embedded_font_program,
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

#[cfg(test)]
mod tests {
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
fn matcher_resolves_native_text_with_descriptor_cache() {
        let candidate = SystemFontCandidate {
            family_name: "Microsoft YaHei".to_string(),
            full_name: Some("Microsoft YaHei".to_string()),
            post_script_name: Some("MicrosoftYaHei".to_string()),
            coverage_score: 40,
            ..Default::default()
        };
        let mut matcher = PdfSystemFontMatcher::new(vec![candidate], "Microsoft YaHei");
        let text = NativeTextModel {
            font_name: "寰蒋闆呴粦".to_string(),
            font_post_script_name: Some("MicrosoftYaHei".to_string()),
            has_embedded_font_program: true,
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
