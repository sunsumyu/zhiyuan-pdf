use crate::models::{FontHints, FontSourceKind, ResolvedFontFace, ResolvedFontIdentity, SymbolClass};

fn strip_subset_prefix(name: &str) -> (&str, bool) {
    match name.find('+') {
        Some(6) => (&name[7..], true),
        _ => (name, false),
    }
}

fn split_family_and_style(name: &str) -> (String, String) {
    let trimmed = name.trim();
    let lower = trimmed.to_ascii_lowercase();
    for suffix in ["-regular", " regular", "-bold", " bold", "-italic", " italic", "-bolditalic", " bolditalic"] {
        if lower.ends_with(suffix) {
            let family_len = trimmed.len().saturating_sub(suffix.len());
            let family = trimmed[..family_len].trim_end_matches(['-', ' ']).trim();
            let style = trimmed[family_len..].trim_matches(['-', ' ']).trim();
            return (
                if family.is_empty() { trimmed.to_string() } else { family.to_string() },
                if style.is_empty() { "Regular".to_string() } else { style.to_string() },
            );
        }
    }
    (trimmed.to_string(), "Regular".to_string())
}

fn classify_symbol_family(name_lower: &str) -> SymbolClass {
    if name_lower.contains("wingdings") || name_lower.contains("webdings") || name_lower.contains("zapfdingbats") {
        return SymbolClass::Dingbat;
    }
    if name_lower.contains("symbol") || name_lower.contains("segoe ui symbol") {
        return SymbolClass::Symbol;
    }
    SymbolClass::None
}

fn resolve_render_family(canonical_family: &str, symbol_class: SymbolClass, hints: Option<&FontHints>) -> (String, FontSourceKind, f32) {
    let lower = canonical_family.to_ascii_lowercase();
    match symbol_class {
        SymbolClass::Dingbat => {
            return ("'Wingdings', 'Segoe UI Symbol', 'Apple Symbols', 'Noto Sans Symbols', sans-serif".to_string(), FontSourceKind::Substituted, 0.72);
        }
        SymbolClass::Symbol => {
            return ("'Symbol', 'Segoe UI Symbol', 'Apple Symbols', sans-serif".to_string(), FontSourceKind::Substituted, 0.72);
        }
        SymbolClass::None => {}
    }

    if lower.contains("stkaiti") || lower.contains("kaiti") {
        return ("'KaiTi', 'Kaiti SC', '楷体', serif".to_string(), FontSourceKind::SystemMatched, 0.9);
    }
    if lower.contains("simsun") || lower.contains("songti") || lower.contains("stsong") {
        return ("'SimSun', 'Songti SC', '宋体', serif".to_string(), FontSourceKind::SystemMatched, 0.9);
    }
    if lower.contains("simhei") || lower.contains("stheiti") || lower.contains("heiti") {
        return ("'SimHei', 'Heiti SC', '黑体', sans-serif".to_string(), FontSourceKind::SystemMatched, 0.9);
    }
    if lower.contains("yahei") || lower.contains("msyh") {
        return ("'Microsoft YaHei', 'Heiti SC', sans-serif".to_string(), FontSourceKind::SystemMatched, 0.92);
    }
    if lower.contains("fangsong") || lower.contains("stfangsong") {
        return ("'FangSong', 'STFangsong', '仿宋', serif".to_string(), FontSourceKind::SystemMatched, 0.9);
    }
    if lower.contains("arial") {
        return ("'Arial', 'Helvetica Neue', sans-serif".to_string(), FontSourceKind::SystemMatched, 0.88);
    }
    if lower.contains("times") {
        return ("'Times New Roman', 'Times', serif".to_string(), FontSourceKind::SystemMatched, 0.88);
    }
    if lower.contains("courier") {
        return ("'Courier New', 'Courier', monospace".to_string(), FontSourceKind::SystemMatched, 0.88);
    }
    if lower.contains("calibri") {
        return ("'Calibri', 'Segoe UI', sans-serif".to_string(), FontSourceKind::SystemMatched, 0.84);
    }
    if lower.contains("verdana") {
        return ("'Verdana', 'Geneva', sans-serif".to_string(), FontSourceKind::SystemMatched, 0.84);
    }
    if lower.contains("tahoma") {
        return ("'Tahoma', 'Geneva', sans-serif".to_string(), FontSourceKind::SystemMatched, 0.84);
    }

    if let Some(hints) = hints {
        if hints.is_fixed_pitch {
            return ("'Courier New', 'Courier', monospace".to_string(), FontSourceKind::Fallback, 0.55);
        }
        if hints.is_serif {
            return ("'Georgia', 'Times New Roman', serif".to_string(), FontSourceKind::Fallback, 0.52);
        }
    }

    ("'Microsoft YaHei', 'PingFang SC', 'Heiti SC', Arial, sans-serif".to_string(), FontSourceKind::Fallback, 0.4)
}

pub fn resolve_font_face(name: &str, hints: Option<&FontHints>) -> ResolvedFontFace {
    let (clean_name, subset_stripped) = strip_subset_prefix(name);
    let (canonical_family, style_name) = split_family_and_style(clean_name);
    let family_lower = canonical_family.to_ascii_lowercase();
    let symbol_class = classify_symbol_family(&family_lower);
    let (render_family, source, confidence) = resolve_render_family(&canonical_family, symbol_class, hints);
    let weight = hints.map(|value| value.weight).unwrap_or(400);
    let is_italic = hints.map(|value| value.is_italic).unwrap_or_else(|| style_name.to_ascii_lowercase().contains("italic"));

    ResolvedFontFace {
        identity: ResolvedFontIdentity {
            raw_name: name.to_string(),
            canonical_family,
            style_name,
            weight,
            is_italic,
            symbol_class,
            subset_stripped,
        },
        metrics_family: render_family.clone(),
        render_family,
        source,
        confidence,
    }
}

pub fn looks_like_symbolic_font(name: &str) -> bool {
    resolve_font_face(name, None).identity.symbol_class != SymbolClass::None
}
