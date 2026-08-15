// Shared lazy singletons for the wasm struct-API session handles.
//
// `DocumentSession` and `ReviewSession` are stateless command proxies (all
// state lives in wasm thread_locals), but each bridge module used to keep its
// own module-level instance, so "singleton" was per-module, not per-app.
// Construct these handles only via these getters.
//
// Other session handles (`CommentManager`, `FindSession`, `ViewerSession`,
// `PagePresentationRuntime`) are constructed in exactly one module each, so
// they keep their local lazy getters for now.

import { getWasmApi } from './wasm_loader';
import type { DocumentSession, ReviewSession } from '../../../crates/pdf-viewer-ui/pkg/pdf_viewer_ui';

let documentSession: DocumentSession | null = null;
let reviewSession: ReviewSession | null = null;

export function getDocumentSession(): DocumentSession | null {
    if (!documentSession) {
        const api = getWasmApi();
        if (typeof api?.DocumentSession === 'function') {
            documentSession = new api.DocumentSession();
        }
    }
    return documentSession;
}

export function getReviewSession(): ReviewSession | null {
    if (!reviewSession) {
        const api = getWasmApi();
        if (typeof api?.ReviewSession === 'function') {
            reviewSession = new api.ReviewSession();
        }
    }
    return reviewSession;
}
