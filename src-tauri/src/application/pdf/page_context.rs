use crate::infrastructure::pdf::models::{
    NativeTextModel as VectorTextModel, RenderObject, VectorPageModel,
};
use pdf_viewer_core::models::{
    NativePageModel, NativePageObject, NativeTextModel as CoreNativeTextModel,
};
use pdf_viewer_core::document::page_region_context::{build_page_region_context, PageRegionContextOutput};
pub(crate)
fn build_page_region_context_from_vector_model(
    page_model: &VectorPageModel,
) -> PageRegionContextOutput {
    let native_page = native_page_from_vector_model(page_model);
    build_page_region_context(&native_page)
}
pub(crate)
fn native_page_from_vector_model(page_model: &VectorPageModel) -> NativePageModel {
    NativePageModel {
        page_index: page_model.page_index,
        width: page_model.width,
        height: page_model.height,
        objects: page_model
            .objects
            .iter()
            .filter_map(|object| match object {
                RenderObject::Text(text) => {
                    Some(NativePageObject::Text(native_text_from_vector_text(text)))
                }
                RenderObject::Path(_) | RenderObject::Image(_) => None,
            })
            .collect(),
    }
}
fn native_text_from_vector_text(text: &VectorTextModel) -> CoreNativeTextModel {
    CoreNativeTextModel {
        r#type: "text".to_string(),
        id: text.id.clone(),
        text: text.text.clone(),
        tx: text.tx,
        ty: text.ty,
        width: text.width,
        height: text.height,
        font_size: text.font_size,
        font_name: text.font_name.clone(),
        color: text.color.clone(),
        is_bold: text.is_bold,
        is_italic: text.is_italic,
        is_underline: text.is_underline,
        runs: text.runs.clone(),
        z_index: text.z_index,
        font_hints: text.font_hints.clone(),
        object_indices: text.object_indices.clone(),
        paragraph_id: text.paragraph_id.clone(),
        wrap_width: text.wrap_width,
        min_tx: text.min_tx,
        render_mode: text.rendering_mode as i64,
        char_spacing: text.char_spacing,
        horizontal_scaling: text.horizontal_scaling,
        role: text.role,
        alignment: text.alignment,
    }
}
