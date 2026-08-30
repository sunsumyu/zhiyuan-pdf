//! DOM element caching for the zoom RAF loop.
//!
//! Resolves `HtmlElement` references by ID once per loop session and caches
//! them to avoid `get_element_by_id` per frame.

use std::cell::RefCell;
use wasm_bindgen::JsCast;

/// Cached DOM elements — resolved once on first use per loop session.
pub(super) struct DomCache {
    pub container: web_sys::HtmlElement,
    pub scroll_container: web_sys::HtmlElement,
    /// Raster/preview sibling surface (`pdf-render-target`) — hidden at
    /// gesture start per ADR-0002 (I3 single active surface).
    pub raster: Option<web_sys::HtmlElement>,
}

// ─── DOM element IDs ──────────────────────────────────────────────
//
// Must match the TS bridge exactly:
//   - vector_canvas_host.ts creates the page container with id "pdf-page-container"
//   - index.html declares the static scroll container "pdf-scroll-container"
//     and the raster sibling canvas "pdf-render-target"
pub(super) const VECTOR_CONTAINER_ID: &str = "pdf-page-container";
pub(super) const SCROLL_CONTAINER_ID: &str = "pdf-scroll-container";
pub(super) const RASTER_TARGET_ID: &str = "pdf-render-target";

thread_local! {
    /// Cached DOM element references — resolved once on first use per loop session.
    static DOM_CACHE: RefCell<Option<DomCache>> = RefCell::new(None);
}

pub(super) fn with_dom_cache<R>(f: impl FnOnce(Option<&DomCache>) -> R) -> R {
    DOM_CACHE.with(|c| f(c.borrow().as_ref()))
}

pub(super) fn with_dom_cache_mut<R>(f: impl FnOnce(&mut Option<DomCache>) -> R) -> R {
    DOM_CACHE.with(|c| f(&mut *c.borrow_mut()))
}

pub(super) fn clear_dom_cache() {
    DOM_CACHE.with(|c| *c.borrow_mut() = None);
}

fn get_element_by_id(id: &str) -> Option<web_sys::Element> {
    web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id(id))
}

/// Initialize the DOM element cache. Called once per RAF loop session.
pub(super) fn init_dom_cache() {
    DOM_CACHE.with(|cache| {
        if cache.borrow().is_some() {
            return;
        }

        let container = get_element_by_id(VECTOR_CONTAINER_ID)
            .and_then(|el| el.dyn_into::<web_sys::HtmlElement>().ok());
        let scroll_container = get_element_by_id(SCROLL_CONTAINER_ID)
            .and_then(|el| el.dyn_into::<web_sys::HtmlElement>().ok());
        let raster = get_element_by_id(RASTER_TARGET_ID)
            .and_then(|el| el.dyn_into::<web_sys::HtmlElement>().ok());

        if let (Some(container), Some(scroll_container)) = (container, scroll_container) {
            // Set transform-origin once — it never changes
            let _ = container.style().set_property("transform-origin", "0 0");
            *cache.borrow_mut() = Some(DomCache { container, scroll_container, raster });
        }
    });
}
