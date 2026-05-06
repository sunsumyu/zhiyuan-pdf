use pdf_viewer_core::coordinate_transform::{HostPageTransform, HostReferenceRect, PageSize};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DomRectLike {
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DomPointLike {
    #[serde(rename = "clientX")]
    pub client_x: f32,
    #[serde(rename = "clientY")]
    pub client_y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScalePair {
    #[serde(rename = "scaleX")]
    pub scale_x: f32,
    #[serde(rename = "scaleY")]
    pub scale_y: f32,
}

pub fn measure_dom_to_page_scale(
    reference_rect: &DomRectLike,
    page_width: f32,
    page_height: f32,
) -> ScalePair {
    let transform = HostPageTransform::new(
        HostReferenceRect {
            left: reference_rect.left,
            top: reference_rect.top,
            width: reference_rect.width,
            height: reference_rect.height,
        },
        PageSize {
            width: page_width,
            height: page_height,
        },
    );
    let scale = transform.scale();
    ScalePair {
        scale_x: scale.x,
        scale_y: scale.y,
    }
}
