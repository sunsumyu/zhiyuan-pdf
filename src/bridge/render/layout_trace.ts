import { emitPdfDiagnostic } from '../shared/diagnostics';

type ElementSnapshot = {
    exists: boolean;
    client?: string;
    offset?: string;
    scroll?: string;
    scrollPos?: string;
    rect?: {
        left: number;
        top: number;
        width: number;
        height: number;
    };
    css?: {
        size: string;
        pos: string;
        transform: string;
        visible: string;
    };
    bitmap?: string;
};

type LayoutKeySnapshot = {
    scrollViewport?: string;
    wrapperRect: string;
    pageRect: string;
    pageLeftTop: string;
    pageCssLeftTop: string;
    pageTransform: string;
    canvasCss: string;
    canvasBitmap: string;
    stageBitmap: string;
    containerScroll: string;
};

function round(value: number): number {
    return Number.isFinite(value) ? Math.round(value * 1000) / 1000 : value;
}

function snapshotElement(id: string): ElementSnapshot {
    const element = document.getElementById(id) as HTMLElement | null;
    if (!element) return { exists: false };
    const rect = element.getBoundingClientRect();
    const style = window.getComputedStyle(element);
    const canvas = element instanceof HTMLCanvasElement ? element : null;
    return {
        exists: true,
        client: `${element.clientWidth}x${element.clientHeight}`,
        offset: `${element.offsetWidth}x${element.offsetHeight}`,
        scroll: `${element.scrollWidth}x${element.scrollHeight}`,
        scrollPos: `${round(element.scrollLeft)},${round(element.scrollTop)}`,
        rect: {
            left: round(rect.left),
            top: round(rect.top),
            width: round(rect.width),
            height: round(rect.height),
        },
        css: {
            size: `${style.width}x${style.height}`,
            pos: `${style.left},${style.top}`,
            transform: style.transform === 'none' ? 'none' : style.transform,
            visible: `${style.display}/${style.visibility}/${style.opacity}`,
        },
        bitmap: canvas ? `${canvas.width}x${canvas.height}` : undefined,
    };
}

export function readPdfLayoutSnapshot(): {
    t: number;
    dpr: number;
    viewport: string;
    scroll: ElementSnapshot;
    wrapper: ElementSnapshot;
    container: ElementSnapshot;
    mainCanvas: ElementSnapshot;
    mainStage: ElementSnapshot;
    key: LayoutKeySnapshot;
} {
    const scroll = snapshotElement('pdf-scroll-container');
    const wrapper = snapshotElement('pdf-content-wrapper');
    const container = snapshotElement('pdf-page-container');
    const mainCanvas = snapshotElement('pdf-vector-main-canvas');
    const mainStage = snapshotElement('pdf-vector-main-stage-canvas');
    return {
        t: round(performance.now()),
        dpr: window.devicePixelRatio || 1,
        viewport: `${window.innerWidth}x${window.innerHeight}`,
        scroll,
        wrapper,
        container,
        mainCanvas,
        mainStage,
        key: {
            scrollViewport: scroll.client,
            wrapperRect: wrapper.rect ? `${wrapper.rect.width}x${wrapper.rect.height}` : 'missing',
            pageRect: container.rect ? `${container.rect.width}x${container.rect.height}` : 'missing',
            pageLeftTop: container.rect ? `${container.rect.left},${container.rect.top}` : 'missing',
            pageCssLeftTop: container.css?.pos ?? 'missing',
            pageTransform: container.css?.transform ?? 'missing',
            canvasCss: mainCanvas.rect ? `${mainCanvas.rect.width}x${mainCanvas.rect.height}` : 'missing',
            canvasBitmap: mainCanvas.bitmap ?? 'missing',
            stageBitmap: mainStage.bitmap ?? 'missing',
            containerScroll: container.scroll ?? 'missing',
        },
    };
}

function compactValue(key: string, value: unknown, depth = 0): unknown {
    if (value == null || typeof value === 'number' || typeof value === 'boolean') return value;
    if (typeof value === 'string') {
        if (key.toLowerCase().includes('path')) {
            return value.split(/[\\/]/).pop() ?? value;
        }
        return value.length > 120 ? `${value.slice(0, 117)}...` : value;
    }
    if (Array.isArray(value)) {
        return depth >= 1 ? `[${value.length}]` : value.slice(0, 6).map((item) => compactValue(key, item, depth + 1));
    }
    if (typeof value !== 'object') return String(value);

    const objectValue = value as Record<string, unknown>;
    const planKeys = [
        'frameToken',
        'renderReason',
        'prepareVisibleLayout',
        'displayZoom',
        'renderZoom',
        'baseRenderZoom',
        'baseCacheZoom',
        'detailCacheZoom',
        'cssScale',
        'useViewportTile',
        'hostWidth',
        'hostHeight',
        'contentLeft',
        'contentTop',
        'scrollLeft',
        'scrollTop',
        'tileLeft',
        'tileTop',
        'tileWidth',
        'tileHeight',
        'accepted',
    ];
    if (key === 'plan' || key === 'frame' || key === 'framePlan' || key === 'renderPlan' || key === 'transition' || key === 'result') {
        return Object.fromEntries(
            planKeys
                .filter((field) => field in objectValue)
                .map((field) => [field, compactValue(field, objectValue[field], depth + 1)]),
        );
    }
    if (depth >= 2) return '{...}';
    return Object.fromEntries(
        Object.entries(objectValue)
            .slice(0, 16)
            .map(([childKey, childValue]) => [childKey, compactValue(childKey, childValue, depth + 1)]),
    );
}

function compactDetails(details: Record<string, unknown>): Record<string, unknown> {
    return Object.fromEntries(
        Object.entries(details).map(([key, value]) => [key, compactValue(key, value)]),
    );
}

function formatDetails(details: Record<string, unknown>): string {
    const compacted = compactDetails(details);
    const parts: string[] = [];
    for (const [key, value] of Object.entries(compacted)) {
        if (value == null) continue;
        if (typeof value === 'object') {
            const fields = Object.entries(value as Record<string, unknown>)
                .filter(([, fieldValue]) => fieldValue != null)
                .map(([fieldKey, fieldValue]) => `${fieldKey}:${fieldValue}`)
                .join(',');
            if (fields) parts.push(`${key}={${fields}}`);
            continue;
        }
        parts.push(`${key}=${value}`);
    }
    return parts.join(' ');
}

export function logPdfLayoutTrace(node: string, details: Record<string, unknown> = {}): void {
    const snapshot = readPdfLayoutSnapshot();
    const key = snapshot.key;
    const verbose = (window as any).__PDF_LAYOUT_TRACE_VERBOSE === true;
    const canvasMismatch =
        key.pageRect !== 'missing' &&
        key.canvasCss !== 'missing' &&
        key.pageRect !== key.canvasCss;
    const transformed = key.pageTransform !== 'missing' && key.pageTransform !== 'none';
    const keyNode = node === 'document-refresh.done';
    if (!verbose && !canvasMismatch && !transformed && !keyNode) {
        return;
    }
    const detailText = formatDetails(details);
    emitPdfDiagnostic('layout', node, {
        t: snapshot.t,
        dpr: snapshot.dpr,
        vp: snapshot.viewport,
        scroll: key.scrollViewport ?? 'missing',
        wrapper: key.wrapperRect,
        page: `${key.pageRect}@${key.pageLeftTop}`,
        pageCss: key.pageCssLeftTop,
        transform: key.pageTransform,
        canvasCss: key.canvasCss,
        bitmap: key.canvasBitmap,
        stage: key.stageBitmap,
        containerScroll: key.containerScroll,
        details: detailText,
    });
}

