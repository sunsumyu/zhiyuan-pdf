use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FontHints {
    pub flags: i32,
    pub weight: i32,
    pub italic_angle: f32,
    pub ascent: f32,
    pub descent: f32,
    pub cap_height: f32,
    pub x_height: f32,
    pub is_fixed_pitch: bool,
    pub is_serif: bool,
    pub is_italic: bool,
    pub is_bold: bool,
    pub is_symbolic: bool,
    pub is_script: bool,
    pub is_all_cap: bool,
    pub is_small_cap: bool,
    pub is_force_bold: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FontSourceKind {
    Embedded,
    SystemMatched,
    Substituted,
    #[default]
    Fallback,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SymbolClass {
    #[default]
    None,
    Symbol,
    Dingbat,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedFontIdentity {
    pub raw_name: String,
    pub canonical_family: String,
    pub style_name: String,
    pub weight: i32,
    pub is_italic: bool,
    pub symbol_class: SymbolClass,
    pub subset_stripped: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedFontFace {
    pub identity: ResolvedFontIdentity,
    pub render_family: String,
    pub metrics_family: String,
    pub source: FontSourceKind,
    pub confidence: f32,
}
