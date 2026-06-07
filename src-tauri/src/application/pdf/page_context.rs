use crate::infrastructure::pdf::models::{
    NativeTextModel as VectorTextModel, NativeVectorPageModel, RenderObject,
};
use pdf_viewer_core::document::page_region_context::{
    build_page_region_context, PageRegionContextOutput,
};
use pdf_viewer_core::models::{
    NativePageModel, NativePageObject, NativeTextModel as CoreNativeTextModel,
};
pub(crate) fn build_page_region_context_from_vector_model(
    page_model: &NativeVectorPageModel,
) -> PageRegionContextOutput {
    let native_page = native_page_from_vector_model(page_model);
    build_page_region_context(&native_page)
}
pub(crate) fn native_page_from_vector_model(page_model: &NativeVectorPageModel) -> NativePageModel {
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
    text.clone()
}
