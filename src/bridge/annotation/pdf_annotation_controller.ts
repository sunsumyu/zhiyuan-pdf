import type { DocumentEditApi } from '../document/document_edit_api';
import { getWasmApi, targetInvokeV3 } from '../shared/wasm_loader';
import { setToolbarButtonActive } from '../viewer/pdf_viewer_dom';

function resolvePercentageRect(
    rect: { left: number; top: number; width: number; height: number },
    pageWidth: number,
    pageHeight: number,
) {
    const wasm = getWasmApi() as any;
    if (wasm?.GeometryApi) {
        const geo = new wasm.GeometryApi();
        return geo.toPercentageRect(rect.left, rect.top, rect.width, rect.height, pageWidth, pageHeight);
    }
    const pw = Math.max(1, pageWidth);
    const ph = Math.max(1, pageHeight);
    return {
        left: (rect.left / pw) * 100,
        top: (rect.top / ph) * 100,
        width: (rect.width / pw) * 100,
        height: (rect.height / ph) * 100,
    };
}

type ViewerSessionSnapshot = {
    path: string | null;
    currentPage: number;
};

type PdfPageAnnotationTarget = {
    id: string;
    kind: string;
    pageIndex: number;
    pageWidth: number;
    pageHeight: number;
    label: string;
    boxRect: {
        left: number;
        top: number;
        width: number;
        height: number;
    };
};

type PdfPageAnnotationTargetResult = {
    targets: PdfPageAnnotationTarget[];
};

type PdfPageHighlightItem = {
    id: string;
    pageIndex: number;
    pageWidth: number;
    pageHeight: number;
    color: [number, number, number];
    boxRect: {
        left: number;
        top: number;
        width: number;
        height: number;
    };
};

type PdfPageHighlightList = {
    highlights: PdfPageHighlightItem[];
};

type CreatePdfAnnotationControllerDeps = {
    getViewerSession: () => ViewerSessionSnapshot;
    documentEdits: DocumentEditApi;
};

export type PdfAnnotationController = {
    toggle: () => Promise<void>;
    refresh: () => Promise<void>;
    clear: () => void;
};

const DEFAULT_HIGHLIGHT_COLOR: [number, number, number] = [1.0, 0.92, 0.4];

function getNodes() {
    return {
        toggle: document.getElementById('pdf-highlight-btn') as HTMLButtonElement | null,
        overlay: document.getElementById('pdf-annotation-overlay') as HTMLElement | null,
        targets: document.getElementById('pdf-annotation-target-overlay') as HTMLElement | null,
    };
}

function colorToCss(color: [number, number, number], alpha: number): string {
    const channel = (value: number) => Math.max(0, Math.min(255, Math.round(value * 255)));
    return `rgba(${channel(color[0])}, ${channel(color[1])}, ${channel(color[2])}, ${alpha})`;
}

export function createPdfAnnotationController(
    deps: CreatePdfAnnotationControllerDeps,
): PdfAnnotationController {
    let enabled = false;
    let busy = false;
    let lastLoadedKey: string | null = null;
    let needsReload = true;

    function syncButtonState(): void {
        const nodes = getNodes();
        if (!nodes.toggle) return;
        setToolbarButtonActive(nodes.toggle, enabled);
    }

    function clear(): void {
        const nodes = getNodes();
        if (nodes.overlay) {
            nodes.overlay.innerHTML = '';
        }
        if (nodes.targets) {
            nodes.targets.innerHTML = '';
            nodes.targets.style.pointerEvents = 'none';
        }
        enabled = false;
        busy = false;
        lastLoadedKey = null;
        needsReload = false;
        syncButtonState();
    }

    function renderPersistedHighlights(highlights: PdfPageHighlightItem[]): void {
        const nodes = getNodes();
        if (!nodes.overlay) return;
        nodes.overlay.innerHTML = '';

        for (const highlight of highlights) {
            const pageWidth = Math.max(1, highlight.pageWidth || 1);
            const pageHeight = Math.max(1, highlight.pageHeight || 1);
            const node = document.createElement('button');
            node.type = 'button';
            node.dataset.highlightId = highlight.id;
            node.title = '点击删除这条高亮';
            node.style.position = 'absolute';
            node.style.pointerEvents = 'auto';
            const pct = resolvePercentageRect(highlight.boxRect, pageWidth, pageHeight);
            node.style.left = `${pct.left}%`;
            node.style.top = `${pct.top}%`;
            node.style.width = `${pct.width}%`;
            node.style.height = `${pct.height}%`;
            node.style.borderRadius = '4px';
            node.style.background = colorToCss(highlight.color, 0.26);
            node.style.border = `1px solid ${colorToCss(highlight.color, 0.65)}`;
            node.style.boxSizing = 'border-box';
            node.style.cursor = 'pointer';
            node.addEventListener('click', (event) => {
                event.preventDefault();
                event.stopPropagation();
                const shouldDelete = window.confirm('删除这条高亮标注？');
                if (shouldDelete) {
                    void deleteHighlight(highlight);
                }
            });
            nodes.overlay.appendChild(node);
        }
    }

    async function deleteHighlight(highlight: PdfPageHighlightItem): Promise<void> {
        if (busy) return;
        const session = deps.getViewerSession();
        if (!session.path) return;
        busy = true;
        try {
            needsReload = true;
            await targetInvokeV3('delete_annotation', {
                path: session.path,
                request: {
                    pageIndex: highlight.pageIndex,
                    annotationId: highlight.id,
                },
            });
            await deps.documentEdits.refreshDocument('highlight');
            await refresh();
        } finally {
            busy = false;
        }
    }

    async function addRegionHighlight(target: PdfPageAnnotationTarget): Promise<void> {
        if (busy) return;
        const session = deps.getViewerSession();
        if (!session.path) return;
        busy = true;
        try {
            needsReload = true;
            await targetInvokeV3('apply_highlight', {
                path: session.path,
                request: {
                    pageIndex: target.pageIndex,
                    regionId: target.id,
                    kind: target.kind,
                    color: DEFAULT_HIGHLIGHT_COLOR,
                },
            });
            await deps.documentEdits.refreshDocument('highlight');
            await refresh();
        } finally {
            busy = false;
        }
    }

    function renderAnnotationTargets(targets: PdfPageAnnotationTarget[]): void {
        const nodes = getNodes();
        if (!nodes.targets) return;
        nodes.targets.innerHTML = '';
        nodes.targets.style.pointerEvents = enabled ? 'auto' : 'none';
        if (!enabled) return;

        for (const target of targets) {
            const pageWidth = Math.max(1, target.pageWidth || 1);
            const pageHeight = Math.max(1, target.pageHeight || 1);
            const node = document.createElement('div');
            node.dataset.annotationTargetId = target.id;
            node.style.position = 'absolute';
            const pct = resolvePercentageRect(target.boxRect, pageWidth, pageHeight);
            node.style.left = `${pct.left}%`;
            node.style.top = `${pct.top}%`;
            node.style.width = `${pct.width}%`;
            node.style.height = `${pct.height}%`;
            node.style.borderRadius = '6px';
            node.style.background = 'rgba(249, 226, 175, 0.08)';
            node.style.border = '1px dashed rgba(249, 226, 175, 0.7)';
            node.style.boxSizing = 'border-box';
            node.style.cursor = 'pointer';
            node.title = target.label;
            node.addEventListener('click', (event) => {
                event.preventDefault();
                event.stopPropagation();
                void addRegionHighlight(target);
            });
            nodes.targets.appendChild(node);
        }
    }

    async function refresh(): Promise<void> {
        const session = deps.getViewerSession();
        const nodes = getNodes();
        if (!session.path) {
            if (nodes.overlay) nodes.overlay.innerHTML = '';
            if (nodes.targets) nodes.targets.innerHTML = '';
            syncButtonState();
            return;
        }
        const nextKey = `${session.path}::${session.currentPage}::${enabled ? 'on' : 'off'}`;
        if (!needsReload && lastLoadedKey === nextKey) {
            syncButtonState();
            return;
        }

        const highlights = await targetInvokeV3('read_highlights', {
            path: session.path,
            pageIndex: session.currentPage,
        }) as PdfPageHighlightList;
        renderPersistedHighlights(highlights.highlights ?? []);

        if (enabled) {
            const targetResult = await targetInvokeV3('read_annotation_targets', {
                path: session.path,
                pageIndex: session.currentPage,
            }) as PdfPageAnnotationTargetResult;
            renderAnnotationTargets(targetResult.targets ?? []);
        } else if (nodes.targets) {
            nodes.targets.innerHTML = '';
            nodes.targets.style.pointerEvents = 'none';
        }

        lastLoadedKey = nextKey;
        needsReload = false;
        syncButtonState();
    }

    async function toggle(): Promise<void> {
        enabled = !enabled;
        needsReload = true;
        await refresh();
    }

    return {
        toggle,
        refresh,
        clear,
    };
}

