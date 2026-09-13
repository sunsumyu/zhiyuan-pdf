pub mod history;
// editor::session::session is intentional nesting (session state types live
// in their own file); flattening would rename a dozen import paths.
#[allow(clippy::module_inception)]
pub mod session;
pub use session::*;
