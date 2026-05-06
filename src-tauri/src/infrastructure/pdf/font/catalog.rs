use pdf_viewer_core::typography::models::SystemFontCandidate;

#[cfg(windows)]
use std::collections::HashSet;

#[cfg(windows)]
use windows_sys::Win32::{
    Foundation::LPARAM,
    Graphics::Gdi::{
        EnumFontFamiliesExW, GetDC, ReleaseDC, DEFAULT_CHARSET, FF_ROMAN, FF_SCRIPT, LOGFONTW,
        TEXTMETRICW, TMPF_FIXED_PITCH,
    },
};

#[cfg(windows)]
pub fn load_system_font_candidates() -> Vec<SystemFontCandidate> {
    unsafe extern "system"
fn enum_font_proc(
        logfont: *const LOGFONTW,
        text_metric: *const TEXTMETRICW,
        _font_type: u32,
        lparam: LPARAM,
    ) -> i32 {
        if logfont.is_null() || text_metric.is_null() || lparam == 0 {
            return 1;
        }

        let fonts = &mut *(lparam as *mut Vec<SystemFontCandidate>);
        let logfont = &*logfont;
        let text_metric = &*text_metric;
        let family_name = wide_to_string(&logfont.lfFaceName);
        if family_name.is_empty() {
            return 1;
        }

        let family_bits = (text_metric.tmPitchAndFamily & 0xF0) as u32;
        let is_fixed_pitch = (text_metric.tmPitchAndFamily & TMPF_FIXED_PITCH) == 0;
        let is_serif = family_bits == FF_ROMAN as u32 || family_bits == FF_SCRIPT as u32;
        let is_symbolic = logfont.lfCharSet == 2;
        let coverage_score = estimate_windows_coverage_score(&family_name, is_symbolic);

        fonts.push(SystemFontCandidate {
            family_name,
            full_name: None,
            post_script_name: None,
            style_name: Some(if logfont.lfItalic != 0 {
                "Italic".to_string()
            } else if logfont.lfWeight >= 700 {
                "Bold".to_string()
            } else {
                "Regular".to_string()
            }),
            weight: logfont.lfWeight,
            is_italic: logfont.lfItalic != 0,
            is_fixed_pitch,
            is_serif,
            is_symbolic,
            coverage_score,
        });

        1
    }

    let hdc = unsafe { GetDC(std::ptr::null_mut()) };
    if hdc.is_null() {
        return Vec::new();
    }

    let mut logfont = LOGFONTW {
        lfCharSet: DEFAULT_CHARSET,
        ..unsafe { std::mem::zeroed() }
    };
    let mut fonts = Vec::<SystemFontCandidate>::new();
    unsafe {
        EnumFontFamiliesExW(
            hdc,
            &mut logfont,
            Some(enum_font_proc),
            &mut fonts as *mut _ as isize,
            0,
        );
        ReleaseDC(std::ptr::null_mut(), hdc);
    }

    enrich_and_dedupe_candidates(fonts)
}

#[cfg(not(windows))]
pub fn load_system_font_candidates() -> Vec<SystemFontCandidate> {
    Vec::new()
}

#[cfg(windows)]
fn enrich_and_dedupe_candidates(fonts: Vec<SystemFontCandidate>) -> Vec<SystemFontCandidate> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();

    for candidate in fonts.into_iter().flat_map(expand_candidate_aliases) {
        let key = format!(
            "{}|{}|{}|{}|{}|{}|{}",
            candidate.family_name.to_ascii_lowercase(),
            candidate
                .full_name
                .as_deref()
                .unwrap_or_default()
                .to_ascii_lowercase(),
            candidate
                .post_script_name
                .as_deref()
                .unwrap_or_default()
                .to_ascii_lowercase(),
            candidate.weight,
            candidate.is_italic,
            candidate.is_fixed_pitch,
            candidate.is_symbolic
        );
        if seen.insert(key) {
            deduped.push(candidate);
        }
    }

    deduped.sort_by(|left, right| {
        right
            .coverage_score
            .cmp(&left.coverage_score)
            .then_with(|| left.family_name.cmp(&right.family_name))
            .then_with(|| left.weight.cmp(&right.weight))
    });

    deduped
}

#[cfg(not(windows))]
fn enrich_and_dedupe_candidates(fonts: Vec<SystemFontCandidate>) -> Vec<SystemFontCandidate> {
    fonts
}

#[cfg(windows)]
fn wide_to_string(buffer: &[u16]) -> String {
    let len = buffer
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..len]).trim().to_string()
}

#[cfg(windows)]
fn expand_candidate_aliases(candidate: SystemFontCandidate) -> Vec<SystemFontCandidate> {
    let mut variants = Vec::new();
    let family = candidate.family_name.clone();
    let normalized_style = candidate
        .style_name
        .clone()
        .unwrap_or_else(|| infer_style_name(candidate.weight, candidate.is_italic));
    let base_postscript = sanitize_postscript_token(&family);

    let mut primary = candidate.clone();
    primary.style_name = Some(normalized_style.clone());
    primary.full_name = Some(build_full_name(&family, &normalized_style));
    primary.post_script_name = Some(build_postscript_name(&base_postscript, &normalized_style));
    variants.push(primary);

    for alias in alias_families(&family) {
        let mut variant = candidate.clone();
        variant.style_name = Some(normalized_style.clone());
        variant.full_name = Some(build_full_name(alias, &normalized_style));
        variant.post_script_name = Some(build_postscript_name(
            &sanitize_postscript_token(alias),
            &normalized_style,
        ));
        variant.coverage_score = variant
            .coverage_score
            .max(estimate_windows_coverage_score(alias, variant.is_symbolic));
        variants.push(variant);
    }

    variants
}

#[cfg(not(windows))]
fn expand_candidate_aliases(candidate: SystemFontCandidate) -> Vec<SystemFontCandidate> {
    vec![candidate]
}
fn infer_style_name(weight: i32, is_italic: bool) -> String {
    match (weight >= 700, is_italic) {
        (true, true) => "Bold Italic".to_string(),
        (true, false) => "Bold".to_string(),
        (false, true) => "Italic".to_string(),
        (false, false) => "Regular".to_string(),
    }
}
fn build_full_name(family: &str, style: &str) -> String {
    if style.eq_ignore_ascii_case("regular") {
        family.to_string()
    } else {
        format!("{} {}", family, style)
    }
}
fn build_postscript_name(family_token: &str, style: &str) -> String {
    let style_token = sanitize_postscript_token(style);
    if style.eq_ignore_ascii_case("regular") {
        family_token.to_string()
    } else {
        format!("{}-{}", family_token, style_token)
    }
}
fn sanitize_postscript_token(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_alphanumeric() || ('\u{4e00}'..='\u{9fff}').contains(ch))
        .collect()
}
fn alias_families(family_name: &str) -> &'static [&'static str] {
    let lower = family_name.to_ascii_lowercase();
    if lower.contains("simsun") || lower.contains("song") || family_name == "瀹嬩綋" {
        return &["SimSun", "瀹嬩綋", "Songti SC", "STSong"];
    }
    if lower.contains("simhei") || lower.contains("hei") || family_name == "榛戜綋" {
        return &["SimHei", "榛戜綋", "Heiti SC", "STHeiti"];
    }
    if lower.contains("yahei") || family_name == "寰蒋闆呴粦" {
        return &["Microsoft YaHei", "寰蒋闆呴粦", "MSYH"];
    }
    if lower.contains("kaiti") || family_name == "妤蜂綋" {
        return &["KaiTi", "妤蜂綋", "Kaiti SC", "STKaiti"];
    }
    if lower.contains("fangsong") || family_name == "浠垮畫" {
        return &["FangSong", "浠垮畫", "STFangsong"];
    }
    if lower.contains("times") {
        return &["Times New Roman", "Times"];
    }
    if lower.contains("arial") {
        return &["Arial", "ArialMT"];
    }
    if lower.contains("courier") {
        return &["Courier New", "CourierNewPSMT", "Courier"];
    }
    if lower.contains("segoe ui symbol") || lower.contains("symbol") {
        return &["Segoe UI Symbol", "Symbol"];
    }
    &[]
}
fn estimate_windows_coverage_score(family_name: &str, is_symbolic: bool) -> u16 {
    let lower = family_name.to_ascii_lowercase();
    if is_symbolic {
        return 10;
    }
    if lower.contains("yahei")
        || lower.contains("simsun")
        || lower.contains("simhei")
        || lower.contains("fangsong")
        || lower.contains("kaiti")
        || lower.contains("song")
        || lower.contains("hei")
    {
        return 40;
    }
    if lower.contains("arial") || lower.contains("times") || lower.contains("courier") {
        return 24;
    }
    if lower.contains("consolas") || lower.contains("verdana") || lower.contains("tahoma") {
        return 18;
    }
    8
}
