use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use super::geometry::BoundingBox;
use super::layout::{LayoutRun, ParagraphEditContext};
use super::marker::{GraphicType, VisualMarker, VisualMarkerContent};
use crate::text::list_semantics::ListMarkerKind;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct SemanticBlockId(pub String);

impl Default for SemanticBlockId {
    fn default() -> Self {
        Self(String::new())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticBlock {
    pub id: SemanticBlockId,
    #[serde(default)]
    pub base_id: String,
    #[serde(default)]
    pub region_id: String,
    #[serde(default)]
    pub page_index: u16,
    pub kind: SemanticBlockKind,
    pub shell_bbox: BoundingBox,
    pub body: SemanticTextBody,
    #[serde(default)]
    pub provenance: SourceProvenanceLite,
    #[serde(default)]
    pub validation: SemanticModelValidation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum SemanticBlockKind {
    Paragraph,
    ListItem(SemanticListItem),
    FieldRow,
    Unknown,
}

impl Default for SemanticBlockKind {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticTextBody {
    pub text: String,
    pub session: ParagraphEditContext,
    pub bbox: BoundingBox,
    #[serde(default)]
    pub object_indices: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticListItem {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marker: Option<SemanticMarker>,
    #[serde(default)]
    pub graphic_markers: Vec<SemanticMarker>,
    #[serde(default)]
    pub layout: SemanticListLayout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticMarker {
    pub kind: ListMarkerKind,
    pub content: SemanticMarkerContent,
    pub bbox: BoundingBox,
    pub advance: f32,
    #[serde(default)]
    pub runs: Vec<LayoutRun>,
    #[serde(default)]
    pub object_indices: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SemanticMarkerContent {
    Text {
        text: String,
    },
    Graphic {
        object_index: usize,
        object_type: GraphicType,
        object_id: String,
    },
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticListLayout {
    pub marker_bbox: BoundingBox,
    pub body_bbox: BoundingBox,
    pub shell_bbox: BoundingBox,
    pub marker_advance: f32,
    pub body_left: f32,
    pub wrap_width: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceProvenanceLite {
    #[serde(default)]
    pub body_object_indices: Vec<usize>,
    #[serde(default)]
    pub marker_object_indices: Vec<usize>,
    #[serde(default)]
    pub graphic_marker_object_indices: Vec<usize>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticModelValidation {
    pub valid: bool,
    #[serde(default)]
    pub errors: Vec<String>,
}

impl SemanticBlock {
    pub fn paragraph(
        id: String,
        base_id: String,
        region_id: String,
        page_index: u16,
        shell_bbox: BoundingBox,
        body: SemanticTextBody,
    ) -> Self {
        let provenance = SourceProvenanceLite {
            body_object_indices: body.object_indices.clone(),
            marker_object_indices: Vec::new(),
            graphic_marker_object_indices: Vec::new(),
        };
        let mut block = Self {
            id: SemanticBlockId(id),
            base_id,
            region_id,
            page_index,
            kind: SemanticBlockKind::Paragraph,
            shell_bbox,
            body,
            provenance,
            validation: SemanticModelValidation::default(),
        };
        block.validation = block.validate();
        block
    }

    pub fn list_item(
        id: String,
        base_id: String,
        region_id: String,
        page_index: u16,
        shell_bbox: BoundingBox,
        body: SemanticTextBody,
        list_item: SemanticListItem,
    ) -> Self {
        let marker_object_indices = list_item
            .marker
            .as_ref()
            .map(|marker| marker.object_indices.clone())
            .unwrap_or_default();
        let graphic_marker_object_indices = list_item
            .graphic_markers
            .iter()
            .flat_map(|marker| marker.object_indices.iter().copied())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let provenance = SourceProvenanceLite {
            body_object_indices: body.object_indices.clone(),
            marker_object_indices,
            graphic_marker_object_indices,
        };
        let mut block = Self {
            id: SemanticBlockId(id),
            base_id,
            region_id,
            page_index,
            kind: SemanticBlockKind::ListItem(list_item),
            shell_bbox,
            body,
            provenance,
            validation: SemanticModelValidation::default(),
        };
        block.validation = block.validate();
        block
    }

    pub fn body_text(&self) -> &str {
        &self.body.text
    }

    pub fn body_session(&self) -> &ParagraphEditContext {
        &self.body.session
    }

    pub fn list_item_ref(&self) -> Option<&SemanticListItem> {
        match &self.kind {
            SemanticBlockKind::ListItem(item) => Some(item),
            _ => None,
        }
    }

    pub fn primary_marker(&self) -> Option<&SemanticMarker> {
        self.list_item_ref().and_then(|item| item.marker.as_ref())
    }

    pub fn validate(&self) -> SemanticModelValidation {
        let mut errors = Vec::new();
        if let Err(error) = validate_body_text_matches_body_runs(&self.body) {
            errors.push(error);
        }
        if let SemanticBlockKind::ListItem(item) = &self.kind {
            if let Err(error) = validate_list_item_body_excludes_marker(&self.body.text, item) {
                errors.push(error);
            }
            if let Err(error) = validate_marker_body_object_sets_do_not_overlap(
                &self.provenance.body_object_indices,
                &self.provenance.marker_object_indices,
                &self.provenance.graphic_marker_object_indices,
            ) {
                errors.push(error);
            }
        }
        SemanticModelValidation {
            valid: errors.is_empty(),
            errors,
        }
    }
}

impl SemanticTextBody {
    pub fn from_session(text: String, session: ParagraphEditContext) -> Self {
        let object_indices = collect_layout_run_object_indices(&session.paragraph.runs);
        let bbox = session.paragraph.bbox;
        Self {
            text,
            session,
            bbox,
            object_indices,
        }
    }
}

impl SemanticMarker {
    pub fn text(kind: ListMarkerKind, text: String, advance: f32, runs: Vec<LayoutRun>) -> Self {
        let bbox = compute_layout_run_bbox(&runs).unwrap_or_default();
        let object_indices = collect_layout_run_object_indices(&runs);
        Self {
            kind,
            content: SemanticMarkerContent::Text { text },
            bbox,
            advance,
            runs,
            object_indices,
        }
    }

    pub fn from_visual_marker(marker: &VisualMarker) -> Self {
        match &marker.content {
            VisualMarkerContent::Text { text, runs } => Self {
                kind: match marker.kind {
                    super::marker::VisualMarkerKind::TextNumbering => ListMarkerKind::Numbering,
                    super::marker::VisualMarkerKind::TextBullet => ListMarkerKind::Bullet,
                    super::marker::VisualMarkerKind::GraphicBullet => ListMarkerKind::Bullet,
                    super::marker::VisualMarkerKind::Custom => ListMarkerKind::Custom,
                    super::marker::VisualMarkerKind::None => ListMarkerKind::None,
                },
                content: SemanticMarkerContent::Text { text: text.clone() },
                bbox: marker.bbox,
                advance: marker.advance,
                runs: runs.clone(),
                object_indices: marker.object_indices.clone(),
            },
            VisualMarkerContent::Graphic {
                object_index,
                object_type,
                object_id,
            } => Self {
                kind: ListMarkerKind::Bullet,
                content: SemanticMarkerContent::Graphic {
                    object_index: *object_index,
                    object_type: *object_type,
                    object_id: object_id.clone(),
                },
                bbox: marker.bbox,
                advance: marker.advance,
                runs: Vec::new(),
                object_indices: marker.object_indices.clone(),
            },
        }
    }

    pub fn text_content(&self) -> Option<&str> {
        match &self.content {
            SemanticMarkerContent::Text { text } => Some(text.as_str()),
            SemanticMarkerContent::Graphic { .. } => None,
        }
    }
}

impl SemanticListItem {
    pub fn source_list_kind(&self) -> ListMarkerKind {
        self.marker
            .as_ref()
            .map(|marker| marker.kind)
            .or_else(|| self.graphic_markers.first().map(|marker| marker.kind))
            .unwrap_or(ListMarkerKind::None)
    }
}

pub fn validate_list_item_body_excludes_marker(
    body_text: &str,
    list_item: &SemanticListItem,
) -> Result<(), String> {
    let Some(marker_text) = list_item
        .marker
        .as_ref()
        .and_then(|marker| marker.text_content())
        .filter(|text| !text.is_empty())
    else {
        return Ok(());
    };

    if body_text.starts_with(marker_text) {
        return Err("list body starts with marker text".to_string());
    }
    Ok(())
}

pub fn validate_marker_body_object_sets_do_not_overlap(
    body_object_indices: &[usize],
    marker_object_indices: &[usize],
    graphic_marker_object_indices: &[usize],
) -> Result<(), String> {
    let body = body_object_indices.iter().copied().collect::<BTreeSet<_>>();
    let mut marker = marker_object_indices
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    marker.extend(graphic_marker_object_indices.iter().copied());
    let overlaps = body.intersection(&marker).copied().collect::<Vec<_>>();
    if overlaps.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "body and marker object sets overlap: {:?}",
            overlaps
        ))
    }
}

pub fn validate_body_text_matches_body_runs(body: &SemanticTextBody) -> Result<(), String> {
    let run_text = body
        .session
        .paragraph
        .runs
        .iter()
        .map(|run| run.text.as_str())
        .collect::<String>();
    if run_text.is_empty() || run_text == body.text {
        Ok(())
    } else {
        Err("semantic body text differs from body run text".to_string())
    }
}

pub fn collect_layout_run_object_indices(runs: &[LayoutRun]) -> Vec<usize> {
    runs.iter()
        .flat_map(|run| run.object_indices.iter().copied())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn compute_layout_run_bbox(runs: &[LayoutRun]) -> Option<BoundingBox> {
    let mut iter = runs.iter().filter(|run| {
        run.bbox.left.is_finite()
            && run.bbox.top.is_finite()
            && run.bbox.right.is_finite()
            && run.bbox.bottom.is_finite()
            && run.bbox.right >= run.bbox.left
            && run.bbox.bottom >= run.bbox.top
    });
    let first = iter.next()?;
    let mut bbox = first.bbox;
    for run in iter {
        bbox.left = bbox.left.min(run.bbox.left);
        bbox.top = bbox.top.min(run.bbox.top);
        bbox.right = bbox.right.max(run.bbox.right);
        bbox.bottom = bbox.bottom.max(run.bbox.bottom);
    }
    Some(bbox)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{LayoutParagraph, ParagraphStyle, RunStyle};

    fn bbox(left: f32, right: f32) -> BoundingBox {
        BoundingBox {
            left,
            top: 0.0,
            right,
            bottom: 10.0,
        }
    }

    fn run(text: &str, object_index: usize, left: f32, right: f32) -> LayoutRun {
        LayoutRun {
            id: format!("run-{object_index}"),
            text: text.to_string(),
            style: RunStyle::default(),
            bbox: bbox(left, right),
            origin_x: left,
            origin_y: 8.0,
            char_origins: Vec::new(),
            char_widths: Vec::new(),
            object_ids: Vec::new(),
            object_indices: vec![object_index],
        }
    }

    fn body_session(text: &str, object_index: usize) -> ParagraphEditContext {
        ParagraphEditContext {
            anchor_bbox: bbox(0.0, 100.0),
            paragraph: LayoutParagraph {
                id: "body".to_string(),
                bbox: bbox(20.0, 100.0),
                style: ParagraphStyle::default(),
                runs: vec![run(text, object_index, 20.0, 100.0)],
                object_ids: Vec::new(),
                origin_x: 20.0,
                origin_y: 8.0,
                wrap_width: 80.0,
            },
        }
    }

    #[test]
    fn list_item_body_excludes_text_marker() {
        let body = SemanticTextBody::from_session("Body".to_string(), body_session("Body", 2));
        let marker = SemanticMarker::text(
            ListMarkerKind::Bullet,
            "●".to_string(),
            20.0,
            vec![run("●", 1, 0.0, 10.0)],
        );
        let item = SemanticListItem {
            marker: Some(marker),
            graphic_markers: Vec::new(),
            layout: SemanticListLayout::default(),
        };
        let block = SemanticBlock::list_item(
            "b1".to_string(),
            "base".to_string(),
            "region".to_string(),
            0,
            bbox(0.0, 100.0),
            body,
            item,
        );

        assert!(block.validation.valid, "{:?}", block.validation.errors);
        assert_eq!(block.body_text(), "Body");
        assert_eq!(
            block.primary_marker().and_then(|m| m.text_content()),
            Some("●")
        );
        assert_eq!(block.provenance.body_object_indices, vec![2]);
        assert_eq!(block.provenance.marker_object_indices, vec![1]);
    }

    #[test]
    fn validation_rejects_marker_in_body_text() {
        let body = SemanticTextBody::from_session("●Body".to_string(), body_session("●Body", 2));
        let marker = SemanticMarker::text(
            ListMarkerKind::Bullet,
            "●".to_string(),
            20.0,
            vec![run("●", 1, 0.0, 10.0)],
        );
        let item = SemanticListItem {
            marker: Some(marker),
            graphic_markers: Vec::new(),
            layout: SemanticListLayout::default(),
        };

        assert!(validate_list_item_body_excludes_marker(&body.text, &item).is_err());
    }

    #[test]
    fn validation_rejects_overlapping_marker_and_body_objects() {
        let result = validate_marker_body_object_sets_do_not_overlap(&[1, 2], &[2], &[]);
        assert!(result.is_err());
    }
}
