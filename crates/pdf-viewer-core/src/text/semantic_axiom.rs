use crate::models::{EditableSegment, NativeTextModel, SemanticRole};

pub struct AxiomEngine {}

impl AxiomEngine {
    pub fn infer_role(
        segment: &EditableSegment,
        parent_model: &NativeTextModel,
        page_height: f32,
    ) -> SemanticRole {
        // 1. 几何特征分析 (Geometry-based Axioms)
        let is_top_area = segment.ty < page_height * 0.15;
        let _is_paged_centered =
            (segment.tx - parent_model.width / 2.0).abs() < parent_model.width * 0.1;

        let mut avg_font_size = 12.0;
        if !parent_model.runs.is_empty() {
            avg_font_size = parent_model.runs.iter().map(|r| r.font_size).sum::<f32>()
                / parent_model.runs.len() as f32;
        }

        let is_large_font = segment.font_size > avg_font_size * 1.35;

        // 公理 A: 特大号顶部文字通常是标题
        if is_top_area && is_large_font {
            return SemanticRole::Title;
        }

        // 2. K-V 内容特征分析 (Content-based Axioms)
        if let Some(group) = &segment.field_group {
            let label = group.label_text.to_lowercase();
            let value = group.value_text.trim();

            // 模式匹配：日期 (Date)
            if label.contains("日期") || label.contains("date") || label.contains("time") {
                return SemanticRole::Date;
            }
            if value.chars().filter(|c| c.is_ascii_digit()).count() >= 6
                && (value.contains('-') || value.contains('/') || value.contains('.'))
            {
                return SemanticRole::Date;
            }

            // 模式匹配：金额 (Amount)
            if label.contains("金额")
                || label.contains("amount")
                || label.contains("价")
                || label.contains("total")
            {
                return SemanticRole::Amount;
            }
            if value.starts_with('¥')
                || value.starts_with('$')
                || (value.contains('.')
                    && value.chars().all(|c: char| {
                        c.is_ascii_digit()
                            || c == '.'
                            || c == ','
                            || c == ' '
                            || c == '¥'
                            || c == '$'
                    }))
            {
                if !value.is_empty() {
                    return SemanticRole::Amount;
                }
            }

            // 模式匹配：联系方式 (Contact)
            if label.contains("电话")
                || label.contains("tel")
                || label.contains("phone")
                || label.contains("手机")
            {
                return SemanticRole::PhoneNumber;
            }
            if label.contains("邮") || label.contains("email") || label.contains("mail") {
                return SemanticRole::Email;
            }

            // 模式匹配：地址 (Address)
            if label.contains("地址") || label.contains("address") || label.contains("地点") {
                return SemanticRole::Address;
            }

            return SemanticRole::GenericField;
        }

        // 3. 正文特征
        if segment.text.chars().count() > 40 {
            return SemanticRole::BodyText;
        }

        SemanticRole::None
    }
}
