export type ViewerSessionSnapshot = {
    path: string | null;
    currentPage: number;
    pageCount: number;
    currentZoom: number;
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

export function createViewerSessionAdapter(deps: ViewerSessionDeps): ViewerSessionAdapter {
    function read(): ViewerSessionSnapshot {
        try {
            const wasm = deps.getWasmApi();
            const session = wasm.get_viewer_session();
            return {
                path: session?.path ?? null,
                currentPage: session?.currentPage ?? 0,
                pageCount: session?.pageCount ?? 0,
                currentZoom: session?.currentZoom ?? 1.0,
                pageWidth: session?.pageWidth ?? deps.getFallbackPageWidth(),
                pageHeight: session?.pageHeight ?? deps.getFallbackPageHeight(),
            };
        } catch {
            return {
                path: null,
                currentPage: 0,
                pageCount: 0,
                currentZoom: 1.0,
                pageWidth: deps.getFallbackPageWidth(),
                pageHeight: deps.getFallbackPageHeight(),
            };
        }
    }

    function setDocument(path: string, pageCount: number, initialZoom: number): void {
        const wasm = deps.getWasmApi();
        wasm.set_viewer_document(path, pageCount, initialZoom);
    }

    function reset(): void {
        const wasm = deps.getWasmApi();
        wasm.reset_viewer_session();
    }

    function setCurrentPage(pageIndex: number): void {
        const wasm = deps.getWasmApi();
        wasm.set_current_page(pageIndex);
    }

    function setCurrentZoom(zoom: number): void {
        const wasm = deps.getWasmApi();
        wasm.set_current_zoom(zoom);
    }

    function setPageDimensions(pageWidth: number, pageHeight: number): void {
        const wasm = deps.getWasmApi();
        wasm.set_page_dimensions(pageWidth, pageHeight);
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


