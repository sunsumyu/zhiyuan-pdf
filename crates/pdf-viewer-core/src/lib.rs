pub mod annotation;
pub mod common;
pub mod document;
pub mod edit;
pub mod geometry;
pub mod history;
pub mod models;
pub mod persistence;
pub mod render;
pub mod text;
pub mod typography;

pub fn read_core_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
