import { emitPdfDiagnostic } from '../shared/diagnostics';
import { logPdfLayoutTrace } from './layout_trace';

export const VECTOR_CONTAINER_ID = 'pdf-page-container';
export const VECTOR_CANVAS_ID = 'pdf-vector-main-canvas';
export const VECTOR_BACK_CANVAS_ID = 'pdf-vector-detail-canvas';
export const VECTOR_STAGE_CANVAS_ID = 'pdf-vector-main-stage-canvas';
export const VECTOR_DETAIL_STAGE_CANVAS_ID = 'pdf-vector-detail-stage-canvas';
const VECTOR_INTERACTION_LAYER_ID = 'pdf-interaction-layer';
const VECTOR_INTERACTION_ROOT_ID = 'pdf-interaction-root-vector';

export type VectorHostRefs = {
    container: HTMLElement;
    mainCanvas: HTMLCanvasElement;
    backCanvas: HTMLCanvasElement;
    mainStageCanvas: HTMLCanvasElement;
    detailStageCanvas: HTMLCanvasElement;
};

type PresentViewportCanvasOptions = {
    showDetailOverlay: boolean;
    retainDetailOverlay?: boolean;
};

type ViewportCanvasFrame = {
    displayZoom: number;
    baseRenderZoom: number;
    displayWidth: number;
    displayHeight: number;
    viewportLeft: number;
    viewportTop: number;
    viewportWidth: number;
    viewportHeight: number;
    dpr: number;
};

export function hideLegacyRasterHost(): void {
    const img = document.getElementById('pdf-render-target') as HTMLElement | null;
    if (img) img.style.display = 'none';

    const legacyRoot = document.getElementById('pdf-interaction-root') as HTMLElement | null;
    if (legacyRoot) legacyRoot.style.display = 'none';
}

function configureCanvas(canvas: HTMLCanvasElement, zIndex: number): void {
    canvas.style.position = 'absolute';
    canvas.style.display = 'block';
    canvas.style.background = 'white';
    canvas.style.transformOrigin = '0 0';
    canvas.style.pointerEvents = 'none';
    canvas.style.zIndex = String(zIndex);
    if (!canvas.style.left) canvas.style.left = '0px';
    if (!canvas.style.top) canvas.style.top = '0px';
}

function configureStageCanvas(canvas: HTMLCanvasElement): void {
    canvas.style.cssText = [
        'position: absolute',
        'display: block',
        'left: -200000px',
        'top: -200000px',
        'visibility: hidden',
        'opacity: 0',
        'pointer-events: none',
        'z-index: -1',
    ].join(';');
}

function ensureCanvas(container: HTMLElement, id: string, zIndex: number): HTMLCanvasElement {
    let canvas = document.getElementById(id) as HTMLCanvasElement | null;
    if (!canvas) {
        canvas = document.createElement('canvas');
        canvas.id = id;
        container.appendChild(canvas);
    }
    configureCanvas(canvas, zIndex);
    return canvas;
}

function ensureStageCanvas(container: HTMLElement, id: string): HTMLCanvasElement {
    let canvas = document.getElementById(id) as HTMLCanvasElement | null;
    if (!canvas) {
        canvas = document.createElement('canvas');
        canvas.id = id;
        container.appendChild(canvas);
    }
    configureStageCanvas(canvas);
    return canvas;
}

function ensureCanvasBitmap(canvas: HTMLCanvasElement, width: number, height: number): void {
    const nextWidth = Math.max(1, Math.round(width));
    const nextHeight = Math.max(1, Math.round(height));
    if (canvas.width !== nextWidth) {
        canvas.width = nextWidth;
    }
    if (canvas.height !== nextHeight) {
        canvas.height = nextHeight;
    }
}

function applyCanvasCssBox(
    canvas: HTMLCanvasElement,
    left: number,
    top: number,
    width: number,
    height: number,
): void {
    canvas.style.left = `${left}px`;
    canvas.style.top = `${top}px`;
    canvas.style.width = `${width}px`;
    canvas.style.height = `${height}px`;
}

function hideDetailCanvas(refs: VectorHostRefs): void {
    refs.backCanvas.style.visibility = 'hidden';
    refs.backCanvas.style.opacity = '0';
    const ctx = refs.backCanvas.getContext('2d', { alpha: false });
    if (ctx) {
        ctx.setTransform(1, 0, 0, 1, 0, 0);
        ctx.clearRect(0, 0, refs.backCanvas.width, refs.backCanvas.height);
    }
}

function getPresentCanvas(refs: VectorHostRefs, useViewportTile: boolean): HTMLCanvasElement {
    return useViewportTile ? refs.backCanvas : refs.mainCanvas;
}

export function clearVectorCanvasHost(): void {
    logPdfLayoutTrace('canvas-host.clear.before');
    const container = document.getElementById(VECTOR_CONTAINER_ID);
    if (container) {
        container.style.display = 'none';
        container.style.visibility = 'hidden';
        container.style.pointerEvents = 'none';
    }

    const img = document.getElementById('pdf-render-target') as HTMLElement | null;
    if (img) {
        img.removeAttribute('src');
        img.style.display = 'none';
    }

    const legacyRoot = document.getElementById('pdf-interaction-root') as HTMLElement | null;
    if (legacyRoot) legacyRoot.style.display = '';
    logPdfLayoutTrace('canvas-host.clear.after');
}

export function ensureVectorCanvasHost(): VectorHostRefs | null {
    const wrapper = document.getElementById('pdf-content-wrapper') as HTMLElement | null;
    if (!wrapper) return null;
    logPdfLayoutTrace('canvas-host.ensure.before');

    let container = document.getElementById(VECTOR_CONTAINER_ID) as HTMLElement | null;
    const containerCreated = !container;
    if (!container) {
        container = document.createElement('div');
        container.id = VECTOR_CONTAINER_ID;
        container.style.cssText = [
            'position: relative',
            'display: block',
            'background: white',
            'overflow: hidden',
            'transform-origin: 0 0',
            'will-change: transform',
        ].join(';');
        wrapper.appendChild(container);
    }

    const mainCanvas = ensureCanvas(container, VECTOR_CANVAS_ID, 1);
    const backCanvas = ensureCanvas(container, VECTOR_BACK_CANVAS_ID, 2);
    const mainStageCanvas = ensureStageCanvas(container, VECTOR_STAGE_CANVAS_ID);
    const detailStageCanvas = ensureStageCanvas(container, VECTOR_DETAIL_STAGE_CANVAS_ID);
    if (containerCreated) {
        hideDetailCanvas({ container, mainCanvas, backCanvas, mainStageCanvas, detailStageCanvas });
    }

    let layer = document.getElementById(VECTOR_INTERACTION_LAYER_ID) as HTMLElement | null;
    if (!layer) {
        layer = document.createElement('div');
        layer.id = VECTOR_INTERACTION_LAYER_ID;
        layer.style.cssText = 'position:absolute;inset:0;pointer-events:auto;z-index:12000;';
        container.appendChild(layer);
    }

    let textLayer = document.getElementById('pdf-text-layer') as HTMLElement | null;
    if (!textLayer) {
        textLayer = document.createElement('div');
        textLayer.id = 'pdf-text-layer';
        textLayer.style.cssText = 'position:absolute;inset:0;pointer-events:auto;z-index:100;user-select:text;-webkit-user-select:text;';
        container.appendChild(textLayer);
    }

    let root = document.getElementById(VECTOR_INTERACTION_ROOT_ID) as HTMLElement | null;
    if (!root) {
        root = document.createElement('div');
        root.id = VECTOR_INTERACTION_ROOT_ID;
        root.style.cssText = 'position:absolute;inset:0;pointer-events:auto;z-index:12000;';
        layer.appendChild(root);
    }

    const refs = { container, mainCanvas, backCanvas, mainStageCanvas, detailStageCanvas };
    logPdfLayoutTrace('canvas-host.ensure.after', {
        containerCreated,
    });
    return refs;
}

export function getExistingVectorCanvasHost(): VectorHostRefs | null {
    const container = document.getElementById(VECTOR_CONTAINER_ID) as HTMLElement | null;
    const mainCanvas = document.getElementById(VECTOR_CANVAS_ID) as HTMLCanvasElement | null;
    const backCanvas = document.getElementById(VECTOR_BACK_CANVAS_ID) as HTMLCanvasElement | null;
    const mainStageCanvas = document.getElementById(VECTOR_STAGE_CANVAS_ID) as HTMLCanvasElement | null;
    const detailStageCanvas = document.getElementById(VECTOR_DETAIL_STAGE_CANVAS_ID) as HTMLCanvasElement | null;
    if (!container || !mainCanvas || !backCanvas || !mainStageCanvas || !detailStageCanvas) {
        return null;
    }
    return { container, mainCanvas, backCanvas, mainStageCanvas, detailStageCanvas };
}

export function hideVectorCanvasHostForPreview(): void {
    const container = document.getElementById(VECTOR_CONTAINER_ID) as HTMLElement | null;
    if (!container) return;
    container.style.display = 'none';
    container.style.visibility = 'hidden';
    container.style.pointerEvents = 'none';
}

export function getRenderBufferCanvas(refs: VectorHostRefs, useViewportTile: boolean): HTMLCanvasElement {
    return useViewportTile ? refs.detailStageCanvas : refs.mainStageCanvas;
}

export function applyViewportCanvasFrame(
    refs: VectorHostRefs,
    frame: ViewportCanvasFrame,
    useViewportTile: boolean,
    deferVisibleFrame = false,
): void {
    logPdfLayoutTrace('canvas-frame.apply.before', {
        frame,
        useViewportTile,
        deferVisibleFrame,
    });
    const domWidth =
        frame.displayZoom > 0.0001 && frame.baseRenderZoom > 0.0001
            ? (frame.displayWidth / frame.displayZoom) * frame.baseRenderZoom
            : frame.displayWidth;
    const domHeight =
        frame.displayZoom > 0.0001 && frame.baseRenderZoom > 0.0001
            ? (frame.displayHeight / frame.displayZoom) * frame.baseRenderZoom
            : frame.displayHeight;

    applyCanvasCssBox(refs.mainCanvas, 0, 0, domWidth, domHeight);
    applyCanvasCssBox(refs.backCanvas, frame.viewportLeft, frame.viewportTop, frame.viewportWidth, frame.viewportHeight);

    const baseScale =
        frame.displayZoom > 0.0001
            ? Math.max(frame.baseRenderZoom, 0.1) / Math.max(frame.displayZoom, 0.1)
            : 1.0;
    ensureCanvasBitmap(refs.mainStageCanvas, frame.displayWidth * baseScale * frame.dpr, frame.displayHeight * baseScale * frame.dpr);

    if (useViewportTile) {
        ensureCanvasBitmap(refs.detailStageCanvas, frame.viewportWidth * frame.dpr, frame.viewportHeight * frame.dpr);
        logPdfLayoutTrace('canvas-frame.apply.after', {
            frame,
            useViewportTile,
            deferVisibleFrame,
            mainStageWidth: refs.mainStageCanvas.width,
            mainStageHeight: refs.mainStageCanvas.height,
            detailStageWidth: refs.detailStageCanvas.width,
            detailStageHeight: refs.detailStageCanvas.height,
        });
        return;
    }

    if (!deferVisibleFrame) {
        hideDetailCanvas(refs);
    }
    logPdfLayoutTrace('canvas-frame.apply.after', {
        frame,
        useViewportTile,
        deferVisibleFrame,
        mainStageWidth: refs.mainStageCanvas.width,
        mainStageHeight: refs.mainStageCanvas.height,
        detailStageWidth: refs.detailStageCanvas.width,
        detailStageHeight: refs.detailStageCanvas.height,
    });
}

export function presentViewportCanvas(
    refs: VectorHostRefs,
    options: PresentViewportCanvasOptions,
): void {
    logPdfLayoutTrace('canvas-present.visibility.before', {
        options,
    });
    hideLegacyRasterHost();
    refs.container.style.display = 'block';
    refs.container.style.visibility = 'visible';
    refs.container.style.pointerEvents = '';
    refs.mainCanvas.style.visibility = 'visible';
    refs.mainCanvas.style.opacity = '1';

    if (options.showDetailOverlay || options.retainDetailOverlay) {
        refs.backCanvas.style.visibility = 'visible';
        refs.backCanvas.style.opacity = '1';
        emitPdfDiagnostic('present', 'canvas.visibility', {
            mainVisible: true,
            detailVisible: true,
            showDetailOverlay: !!options.showDetailOverlay,
            retainDetailOverlay: !!options.retainDetailOverlay,
        });
        logPdfLayoutTrace('canvas-present.visibility.after', {
            options,
        });
        return;
    }

    hideDetailCanvas(refs);
    emitPdfDiagnostic('present', 'canvas.visibility', {
        mainVisible: true,
        detailVisible: false,
        showDetailOverlay: false,
        retainDetailOverlay: false,
    });
    logPdfLayoutTrace('canvas-present.visibility.after', {
        options,
    });
}

export function presentViewportCanvasFromSource(
    refs: VectorHostRefs,
    sourceCanvas: HTMLCanvasElement,
    viewportWidth: number,
    viewportHeight: number,
    useViewportTile: boolean,
    viewportLeft = 0,
    viewportTop = 0,
): void {
    logPdfLayoutTrace('canvas-present.copy.before', {
        sourceWidth: sourceCanvas.width,
        sourceHeight: sourceCanvas.height,
        viewportWidth,
        viewportHeight,
        useViewportTile,
        viewportLeft,
        viewportTop,
    });
    const presentCanvas = getPresentCanvas(refs, useViewportTile);
    ensureCanvasBitmap(presentCanvas, sourceCanvas.width, sourceCanvas.height);
    applyCanvasCssBox(presentCanvas, viewportLeft, viewportTop, viewportWidth, viewportHeight);

    const ctx = presentCanvas.getContext('2d', { alpha: false });
    if (ctx) {
        ctx.setTransform(1, 0, 0, 1, 0, 0);
        ctx.drawImage(sourceCanvas, 0, 0);
    }
    logPdfLayoutTrace('canvas-present.copy.after', {
        presentCanvasId: presentCanvas.id,
        presentCanvasWidth: presentCanvas.width,
        presentCanvasHeight: presentCanvas.height,
        viewportWidth,
        viewportHeight,
        useViewportTile,
        viewportLeft,
        viewportTop,
    });
}

export function stageViewportCanvasFromSource(
    refs: VectorHostRefs,
    sourceCanvas: HTMLCanvasElement,
    viewportWidth: number,
    viewportHeight: number,
    useViewportTile: boolean,
    viewportLeft = 0,
    viewportTop = 0,
): void {
    presentViewportCanvasFromSource(
        refs,
        sourceCanvas,
        viewportWidth,
        viewportHeight,
        useViewportTile,
        viewportLeft,
        viewportTop,
    );
}

