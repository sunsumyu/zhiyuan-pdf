// ─────────────────────────────────────────────────────────────────────────────
// viewer_session.ts — adapter over the ViewerSession struct API.
//
// Migrated 2025: the prior raw `wasm.get_viewer_session()` /
// `wasm.set_viewer_document()` / `wasm.set_current_page()` / etc. free-function
// calls are now routed through the struct-based `ViewerSession` exported from
// `crates/pdf-viewer-ui/src/viewer/viewer_api.rs`. Singleton handle — all state
// lives in the wasm `HOST_VIEWER_SESSION` thread_local.
//
// The adapter shape is preserved for backwards compatibility with all the
// existing callers in `pdf_runtime.ts` etc. — only the implementation changed.
// ─────────────────────────────────────────────────────────────────────────────

export type ViewerSessionSnapshot = {
    path: string | null;
    currentPage: number;
    pageCount: number;
    currentZoom: number;
    documentRevision: number;
    pageWidth: number;
    pageHeight: number;
};

export type ViewerSessionAdapter = {
    read: () => ViewerSessionSnapshot;
    setDocument: (path: string, pageCount: number, initialZoom: number) => void;
    reset: () => void;
    setCurrentPage: (pageIndex: number) => void;
    setCurrentZoom: (zoom: number) => void;
    setPageDimensions: (pageWidth: number, pageHeight: number) => void;
};

type ViewerSessionDeps = {
    getWasmApi: () => any;
    getFallbackPageWidth: () => number;
    getFallbackPageHeight: () => number;
};

let _session: any = null;

function getViewerSession(getWasmApi: () => any): any {
    if (!_session) {
        const api = getWasmApi() as any;
        if (typeof api?.ViewerSession === 'function') {
            _session = new api.ViewerSession();
        }
    }
    return _session;
}

export function createViewerSessionAdapter(deps: ViewerSessionDeps): ViewerSessionAdapter {
    function session(): any { return getViewerSession(deps.getWasmApi); }

    function read(): ViewerSessionSnapshot {
        try {
            const snap = session()?.read();
            return {
                path: snap?.path ?? null,
                currentPage: snap?.currentPage ?? 0,
                pageCount: snap?.pageCount ?? 0,
                currentZoom: snap?.currentZoom ?? 1.0,
                documentRevision: Number(snap?.documentRevision ?? 0),
                pageWidth: snap?.pageWidth ?? deps.getFallbackPageWidth(),
                pageHeight: snap?.pageHeight ?? deps.getFallbackPageHeight(),
            };
        } catch {
            return {
                path: null,
                currentPage: 0,
                pageCount: 0,
                currentZoom: 1.0,
                documentRevision: 0,
                pageWidth: deps.getFallbackPageWidth(),
                pageHeight: deps.getFallbackPageHeight(),
            };
        }
    }

    function setDocument(path: string, pageCount: number, initialZoom: number): void {
        session()?.setDocument(path, pageCount, initialZoom);
    }

    function reset(): void {
        session()?.reset();
    }

    function setCurrentPage(pageIndex: number): void {
        session()?.setCurrentPage(pageIndex);
        try {
            const snap = read();
            const indicator = document.getElementById('pdf-page-indicator');
            if (indicator) {
                indicator.textContent = `Page ${pageIndex + 1} / ${snap.pageCount}`;
            }
            const currentPageInput = document.getElementById('pdf-current-page-input') as HTMLInputElement | null;
            if (currentPageInput) {
                currentPageInput.value = String(pageIndex + 1);
            }
        } catch {}
    }

    function setCurrentZoom(zoom: number): void {
        session()?.setCurrentZoom(zoom);
    }

    function setPageDimensions(pageWidth: number, pageHeight: number): void {
        session()?.setPageDimensions(pageWidth, pageHeight);
    }

    return {
        read,
        setDocument,
        reset,
        setCurrentPage,
        setCurrentZoom,
        setPageDimensions,
    };
}


