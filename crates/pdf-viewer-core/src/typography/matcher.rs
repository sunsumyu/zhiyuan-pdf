use crate::models::FontHints;

use super::models::{
    MatchReason, NormalizedPdfFontIdentity, PdfFontDescriptor, PdfFontMatchRequest,
    PdfFontSourceKind, RenderFontKind, ResolvedPdfFont, SystemFontCandidate, SystemFontMatchResult,
};

pub fn build_match_request(name: &str, hints: Option<&FontHints>) -> PdfFontMatchRequest {
    let identity = normalize_pdf_font_identity(name);
    let descriptor = PdfFontDescriptor {
        source_kind: if identity.subset_tag.is_some() {
            Some(PdfFontSourceKind::EmbeddedSubset)
        } else {
            None
        },
        embedded_font_kind: None,
        font_subtype: None,
        weight: hints.map(|value| value.weight).unwrap_or(400),
        is_italic: hints.map(|value| value.is_italic).unwrap_or(false),
        is_fixed_pitch: hints.map(|value| value.is_fixed_pitch).unwrap_or(false),
        is_serif: hints.map(|value| value.is_serif).unwrap_or(false),
        has_embedded_font_file: false,
        has_to_unicode_cmap: false,
        post_script_name: None,
        family_hint: None,
    };

    PdfFontMatchRequest {
        identity,
        descriptor,
        hints: hints.cloned(),
    }
}

pub fn build_match_request_with_descriptor(
    name: &str,
    hints: Option<&FontHints>,
    descriptor: PdfFontDescriptor,
) -> PdfFontMatchRequest {
    PdfFontMatchRequest {
        identity: normalize_pdf_font_identity(name),
        descriptor,
        hints: hints.cloned(),
    }
}

pub fn normalize_pdf_font_identity(name: &str) -> NormalizedPdfFontIdentity {
    let trimmed = name.trim();
    let (clean_name, subset_tag) = strip_subset_prefix(trimmed);
    let canonical_family = split_family_name(clean_name);
    let style_name = extract_style_name(clean_name);
    let lower = clean_name.to_ascii_lowercase();

    NormalizedPdfFontIdentity {
        raw_name: trimmed.to_string(),
        clean_name: clean_name.to_string(),
        canonical_family,
        style_name,
        subset_tag,
        is_symbolic: lower.contains("symbol")
            || lower.contains("wingdings")
            || lower.contains("zapfdingbats"),
    }
}

pub fn score_system_font_candidate(
    request: &PdfFontMatchRequest,
    candidate: &SystemFontCandidate,
) -> SystemFontMatchResult {
    let mut score = 0;
    let mut reasons = Vec::new();
    let request_family = request.identity.canonical_family.to_ascii_lowercase();
    let candidate_family = candidate.family_name.to_ascii_lowercase();
    let request_key = normalized_font_key(&request.identity.canonical_family);
    let candidate_key = normalized_font_key(&candidate.family_name);

    if request_family == candidate_family {
        push_reason(
            &mut reasons,
            "family_exact",
            "canonical family exact match",
            120,
        );
        score += 120;
    } else if request_key == candidate_key {
        push_reason(
            &mut reasons,
            "family_alias",
            "normalized font alias match",
            110,
        );
        score += 110;
    } else if candidate_family.contains(&request_family)
        || request_family.contains(&candidate_family)
    {
        push_reason(
            &mut reasons,
            "family_partial",
            "canonical family partial match",
            70,
        );
        score += 70;
    }

    if let Some(ps_name) = &candidate.post_script_name {
        if ps_name.eq_ignore_ascii_case(&request.identity.clean_name)
            || ps_name.eq_ignore_ascii_case(&request.identity.raw_name)
            || normalized_font_key(ps_name) == request_key
        {
            push_reason(
                &mut reasons,
                "postscript_exact",
                "postscript name exact match",
                90,
            );
            score += 90;
        }
    }

    if let Some(request_postscript) = &request.descriptor.post_script_name {
        let request_postscript_key = normalized_font_key(request_postscript);
        if !request_postscript_key.is_empty() {
            if let Some(candidate_postscript) = &candidate.post_script_name {
                if normalized_font_key(candidate_postscript) == request_postscript_key {
                    push_reason(
                        &mut reasons,
                        "postscript_descriptor",
                        "descriptor postscript match",
                        95,
                    );
                    score += 95;
                }
            }
            if normalized_font_key(&candidate.family_name) == request_postscript_key {
                push_reason(
                    &mut reasons,
                    "postscript_family_alias",
                    "descriptor postscript aligns with family alias",
                    65,
                );
                score += 65;
            }
        }
    }

    if let Some(full_name) = &candidate.full_name {
        if normalized_font_key(full_name) == request_key {
            push_reason(
                &mut reasons,
                "fullname_alias",
                "full name normalized match",
                80,
            );
            score += 80;
        }
    }

    if let Some(family_hint) = &request.descriptor.family_hint {
        let family_hint_key = normalized_font_key(family_hint);
        if !family_hint_key.is_empty() {
            if normalized_font_key(&candidate.family_name) == family_hint_key {
                push_reason(
                    &mut reasons,
                    "family_hint_match",
                    "font descriptor family hint match",
                    85,
                );
                score += 85;
            }
            if let Some(full_name) = &candidate.full_name {
                if normalized_font_key(full_name) == family_hint_key {
                    push_reason(
                        &mut reasons,
                        "family_hint_fullname",
                        "font descriptor family hint matches full name",
                        70,
                    );
                    score += 70;
                }
            }
        }
    }

    let requested_weight = request.descriptor.weight;
    let weight_delta = (requested_weight - candidate.weight).abs();
    let weight_score = (40 - (weight_delta / 10)).max(0);
    if weight_score > 0 {
        push_reason(
            &mut reasons,
            "weight_near",
            format!("weight delta {}", weight_delta),
            weight_score,
        );
        score += weight_score;
    }

    if request.descriptor.is_italic == candidate.is_italic {
        push_reason(&mut reasons, "italic_match", "italic style matches", 20);
        score += 20;
    }
    if request.descriptor.is_fixed_pitch == candidate.is_fixed_pitch {
        push_reason(
            &mut reasons,
            "mono_match",
            "fixed-pitch classification matches",
            12,
        );
        score += 12;
    }
    if request.descriptor.is_serif == candidate.is_serif {
        push_reason(
            &mut reasons,
            "serif_match",
            "serif classification matches",
            12,
        );
        score += 12;
    }
    if request.identity.is_symbolic == candidate.is_symbolic {
        push_reason(
            &mut reasons,
            "symbol_match",
            "symbolic classification matches",
            25,
        );
        score += 25;
    }

    if let Some(style_name) = &candidate.style_name {
        let request_style_key = normalized_font_key(&request.identity.style_name);
        let candidate_style_key = normalized_font_key(style_name);
        if !request_style_key.is_empty() && request_style_key == candidate_style_key {
            push_reason(&mut reasons, "style_match", "style/subfamily matches", 30);
            score += 30;
        }
    }

    if request.descriptor.has_embedded_font_file {
        push_reason(
            &mut reasons,
            "embedded_pdf_font",
            "pdf font has embedded font file; prefer close system metrics if fallback is needed",
            12,
        );
        score += 12;
    }

    if request.descriptor.has_to_unicode_cmap {
        push_reason(
            &mut reasons,
            "to_unicode_present",
            "font provides ToUnicode cmap; glyph mapping is more reliable",
            8,
        );
        score += 8;
    }

    if candidate.coverage_score > 0 {
        let coverage_delta = (candidate.coverage_score as i32).min(40);
        push_reason(
            &mut reasons,
            "coverage",
            format!("coverage score {}", candidate.coverage_score),
            coverage_delta,
        );
        score += coverage_delta;
    }

    SystemFontMatchResult {
        candidate: candidate.clone(),
        score,
        reasons,
    }
}

pub fn choose_best_match(
    request: &PdfFontMatchRequest,
    candidates: &[SystemFontCandidate],
) -> Option<SystemFontMatchResult> {
    choose_top_matches(request, candidates, 1)
        .into_iter()
        .next()
}

pub fn choose_top_matches(
    request: &PdfFontMatchRequest,
    candidates: &[SystemFontCandidate],
    limit: usize,
) -> Vec<SystemFontMatchResult> {
    let mut ranked: Vec<_> = candidates
        .iter()
        .map(|candidate| score_system_font_candidate(request, candidate))
        .collect();
    ranked.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.candidate.family_name.cmp(&right.candidate.family_name))
    });
    ranked.truncate(limit);
    ranked
}

pub fn resolve_system_or_fallback_font(
    request: &PdfFontMatchRequest,
    candidates: &[SystemFontCandidate],
    fallback_family: &str,
) -> ResolvedPdfFont {
    let can_attempt_embedded_render = request.descriptor.has_embedded_font_file
        && request.descriptor.has_to_unicode_cmap
        && request.descriptor.embedded_font_kind.is_some();

    if let Some(best) = choose_best_match(request, candidates) {
        return ResolvedPdfFont {
            identity: request.identity.clone(),
            render_font_kind: Some(RenderFontKind::System),
            source_kind: Some(PdfFontSourceKind::SystemMatched),
            preferred_render_kind: if can_attempt_embedded_render {
                Some(RenderFontKind::Embedded)
            } else {
                Some(RenderFontKind::System)
            },
            embedded_font_kind: request.descriptor.embedded_font_kind.clone(),
            font_subtype: request.descriptor.font_subtype.clone(),
            can_attempt_embedded_render,
            has_to_unicode_cmap: request.descriptor.has_to_unicode_cmap,
            matched_family: Some(best.candidate.family_name),
            matched_post_script_name: best.candidate.post_script_name,
            confidence_score: best.score,
            reasons: best.reasons,
        };
    }

    ResolvedPdfFont {
        identity: request.identity.clone(),
        render_font_kind: Some(RenderFontKind::Fallback),
        source_kind: Some(PdfFontSourceKind::Fallback),
        preferred_render_kind: if can_attempt_embedded_render {
            Some(RenderFontKind::Embedded)
        } else {
            Some(RenderFontKind::Fallback)
        },
        embedded_font_kind: request.descriptor.embedded_font_kind.clone(),
        font_subtype: request.descriptor.font_subtype.clone(),
        can_attempt_embedded_render,
        has_to_unicode_cmap: request.descriptor.has_to_unicode_cmap,
        matched_family: Some(fallback_family.to_string()),
        matched_post_script_name: None,
        confidence_score: 0,
        reasons: vec![MatchReason {
            code: "fallback".to_string(),
            detail: "no system candidate matched; fallback selected".to_string(),
            score_delta: 0,
        }],
    }
}

fn strip_subset_prefix(name: &str) -> (&str, Option<String>) {
    match name.find('+') {
        Some(6) => (&name[7..], Some(name[..6].to_string())),
        _ => (name, None),
    }
}

fn split_family_name(name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    for suffix in [
        "-bolditalic",
        " bolditalic",
        "-bold",
        " bold",
        "-italic",
        " italic",
        "-regular",
        " regular",
    ] {
        if lower.ends_with(suffix) {
            let family_len = name.len().saturating_sub(suffix.len());
            let family = name[..family_len].trim_end_matches(['-', ' ']).trim();
            return if family.is_empty() {
                name.trim().to_string()
            } else {
                family.to_string()
            };
        }
    }
    name.trim().to_string()
}

fn extract_style_name(name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    for suffix in [
        "-bolditalic",
        " bolditalic",
        "-bold",
        " bold",
        "-italic",
        " italic",
        "-regular",
        " regular",
    ] {
        if lower.ends_with(suffix) {
            return name[name.len().saturating_sub(suffix.len())..]
                .trim_matches(['-', ' '])
                .trim()
                .to_string();
        }
    }
    "Regular".to_string()
}

fn push_reason(
    reasons: &mut Vec<MatchReason>,
    code: &str,
    detail: impl Into<String>,
    score_delta: i32,
) {
    reasons.push(MatchReason {
        code: code.to_string(),
        detail: detail.into(),
        score_delta,
    });
}

fn normalized_font_key(name: &str) -> String {
    let lower = name.trim().to_ascii_lowercase();
    let mapped = match lower.as_str() {
        "simsun" | "songti" | "songtisc" | "stsong" | "宋体" => "songti",
        "simhei" | "heiti" | "heitisc" | "stheiti" | "黑体" => "heiti",
        "kaiti" | "stkaiti" | "kaitisc" | "楷体" => "kaiti",
        "fangsong" | "stfangsong" | "仿宋" => "fangsong",
        "microsoftyahei" | "yahei" | "微软雅黑" | "msyh" => "microsoftyahei",
        "timesnewroman" | "timesroman" | "times" => "timesnewroman",
        "arialmt" | "arial" => "arial",
        "couriernew" | "couriernewpsmt" | "courier" => "couriernew",
        "calibri" => "calibri",
        "tahoma" => "tahoma",
        "verdana" => "verdana",
        "segoeuisymbol" | "symbol" => "segoeuisymbol",
        "wingdings" | "wingdingsregular" => "wingdings",
        _ => "",
    };

    if !mapped.is_empty() {
        return mapped.to_string();
    }

    lower
        .chars()
        .filter(|ch| ch.is_alphanumeric() || ('\u{4e00}'..='\u{9fff}').contains(ch))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typography::models::PdfEmbeddedFontKind;

    #[test]
    fn strips_pdf_subset_prefix() {
        let identity = normalize_pdf_font_identity("ABCDEE+SimSun-Bold");
        assert_eq!(identity.clean_name, "SimSun-Bold");
        assert_eq!(identity.canonical_family, "SimSun");
        assert_eq!(identity.subset_tag.as_deref(), Some("ABCDEE"));
    }

    #[test]
    fn exact_family_match_beats_unrelated_candidate() {
        let request = build_match_request("SimSun", None);
        let exact = SystemFontCandidate {
            family_name: "SimSun".to_string(),
            coverage_score: 30,
            ..Default::default()
        };
        let unrelated = SystemFontCandidate {
            family_name: "Arial".to_string(),
            coverage_score: 30,
            ..Default::default()
        };

        let best = choose_best_match(&request, &[unrelated, exact]).expect("best match");
        assert_eq!(best.candidate.family_name, "SimSun");
    }

    #[test]
    fn chinese_aliases_normalize_to_same_key() {
        assert_eq!(normalized_font_key("SimSun"), normalized_font_key("宋体"));
        assert_eq!(
            normalized_font_key("Microsoft YaHei"),
            normalized_font_key("微软雅黑")
        );
    }

    #[test]
    fn descriptor_postscript_match_boosts_candidate() {
        let request = build_match_request_with_descriptor(
            "UnknownFont",
            None,
            PdfFontDescriptor {
                post_script_name: Some("MicrosoftYaHei".to_string()),
                ..Default::default()
            },
        );
        let exact = SystemFontCandidate {
            family_name: "Microsoft YaHei".to_string(),
            post_script_name: Some("MicrosoftYaHei".to_string()),
            coverage_score: 40,
            ..Default::default()
        };
        let other = SystemFontCandidate {
            family_name: "SimSun".to_string(),
            coverage_score: 40,
            ..Default::default()
        };

        let best = choose_best_match(&request, &[other, exact]).expect("best match");
        assert_eq!(best.candidate.family_name, "Microsoft YaHei");
    }

    #[test]
    fn accepts_embedded_cmap() {
        let request = build_match_request_with_descriptor(
            "ABCDEE+SimSun",
            None,
            PdfFontDescriptor {
                source_kind: Some(PdfFontSourceKind::EmbeddedSubset),
                embedded_font_kind: Some(PdfEmbeddedFontKind::TrueType),
                has_embedded_font_file: true,
                has_to_unicode_cmap: true,
                ..Default::default()
            },
        );
        let resolved = resolve_system_or_fallback_font(&request, &[], "Microsoft YaHei");
        assert!(resolved.can_attempt_embedded_render);
        assert_eq!(
            resolved.preferred_render_kind,
            Some(RenderFontKind::Embedded)
        );
    }
}
