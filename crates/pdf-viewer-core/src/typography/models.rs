use serde::{Deserialize, Serialize};

use crate::models::FontHints;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PdfFontSourceKind {
    EmbeddedSubset,
    EmbeddedFull,
    SystemMatched,
    Fallback,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RenderFontKind {
    Embedded,
    System,
    Fallback,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PdfEmbeddedFontKind {
    Type1,
    TrueType,
    CidType0,
    CidType2,
    OpenType,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedPdfFontIdentity {
    pub raw_name: String,
    pub clean_name: String,
    pub canonical_family: String,
    pub style_name: String,
    pub subset_tag: Option<String>,
    pub is_symbolic: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct PdfFontDescriptor {
    pub source_kind: Option<PdfFontSourceKind>,
    pub embedded_font_kind: Option<PdfEmbeddedFontKind>,
    pub font_subtype: Option<String>,
    pub weight: i32,
    pub is_italic: bool,
    pub is_fixed_pitch: bool,
    pub is_serif: bool,
    pub has_embedded_program: bool,
    pub has_to_unicode_cmap: bool,
    pub post_script_name: Option<String>,
    pub family_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PdfFontMatchRequest {
    pub identity: NormalizedPdfFontIdentity,
    pub descriptor: PdfFontDescriptor,
    pub hints: Option<FontHints>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SystemFontCandidate {
    pub family_name: String,
    pub full_name: Option<String>,
    pub post_script_name: Option<String>,
    pub style_name: Option<String>,
    pub weight: i32,
    pub is_italic: bool,
    pub is_fixed_pitch: bool,
    pub is_serif: bool,
    pub is_symbolic: bool,
    pub coverage_score: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct MatchReason {
    pub code: String,
    pub detail: String,
    pub score_delta: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SystemFontMatchResult {
    pub candidate: SystemFontCandidate,
    pub score: i32,
    pub reasons: Vec<MatchReason>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedPdfFont {
    pub identity: NormalizedPdfFontIdentity,
    pub render_font_kind: Option<RenderFontKind>,
    pub source_kind: Option<PdfFontSourceKind>,
    pub preferred_render_kind: Option<RenderFontKind>,
    pub embedded_font_kind: Option<PdfEmbeddedFontKind>,
    pub font_subtype: Option<String>,
    pub can_attempt_embedded_render: bool,
    pub has_to_unicode_cmap: bool,
    pub matched_family: Option<String>,
    pub matched_post_script_name: Option<String>,
    pub confidence_score: i32,
    pub reasons: Vec<MatchReason>,
}
