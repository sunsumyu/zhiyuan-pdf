pub mod content_parser;
pub mod graphics_state;
pub mod image_builder;
pub mod metadata;
pub mod page_model;
pub mod path_resolver;
pub mod resource_reader;
pub mod utils;

pub use content_parser::parse_content_stream;
pub use graphics_state::GraphicsState;
pub(crate) use image_builder::build_image_as_jpeg;
pub use metadata::{extract_metadata, extract_page_bbox, read_page_count};
pub use page_model::{
    extract_glyph_paint_plan, extract_layout_inference, extract_vector_page_model,
};
pub use path_resolver::resolve_paths;
pub use resource_reader::{find_xobject_by_name, read_resources, FlatResources};
pub use utils::{multiply_matrices, operands_to_f32};
