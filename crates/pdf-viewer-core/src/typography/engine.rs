use crate::models::FontHints;

use super::matcher::{build_match_request, resolve_system_or_fallback_font};
use super::models::{ResolvedPdfFont, SystemFontCandidate};

pub struct TypographyEngine<'a> {
    system_candidates: &'a [SystemFontCandidate],
    fallback_family: &'a str,
}

impl<'a> TypographyEngine<'a> {
    pub fn new(system_candidates: &'a [SystemFontCandidate], fallback_family: &'a str) -> Self {
        Self {
            system_candidates,
            fallback_family,
        }
    }

    pub fn resolve_pdf_font(
        &self,
        pdf_font_name: &str,
        hints: Option<&FontHints>,
    ) -> ResolvedPdfFont {
        let request = build_match_request(pdf_font_name, hints);
        resolve_system_or_fallback_font(&request, self.system_candidates, self.fallback_family)
    }
}
