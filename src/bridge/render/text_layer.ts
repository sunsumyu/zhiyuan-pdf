import { logPdfLayoutTrace } from './layout_trace';

/**
 * Updates the transparent DOM text layer overlay for native text selection.
 * Clears any previous nodes and instantiates new absolute-positioned divs matching
 * the Rust-computed bounding box and text runs.
 */
export function updateTextLayer(
    path: string,
    pageIndex: number,
    model: any,
    displayZoom: number,
): void {
    const textLayer = document.getElementById('pdf-text-layer');
    if (!textLayer) {
        return;
    }

    // Clear existing text layer nodes
    textLayer.innerHTML = '';

    if (!model || !Array.isArray(model.objects)) {
        logPdfLayoutTrace('text-layer.update.empty', { path, pageIndex });
        return;
    }

    const textObjects = model.objects.filter(
        (obj: any) => obj && (obj.type === 'text' || obj.type === 'Text')
    );

    logPdfLayoutTrace('text-layer.update.start', {
        path,
        pageIndex,
        textObjectCount: textObjects.length,
        displayZoom,
    });

    for (const obj of textObjects) {
        const text = obj.text || '';
        if (!text.trim()) {
            continue;
        }

        const div = document.createElement('div');
        div.textContent = text;
        div.style.position = 'absolute';
        div.style.left = `${obj.left * displayZoom}px`;
        div.style.top = `${obj.top * displayZoom}px`;
        div.style.width = `${obj.width * displayZoom}px`;
        div.style.height = `${obj.height * displayZoom}px`;
        
        const fontSize = obj.fontSize || obj.height || 12;
        div.style.fontSize = `${fontSize * displayZoom}px`;
        div.style.lineHeight = '1';
        div.style.color = 'transparent';
        div.style.backgroundColor = 'transparent';
        div.style.border = 'none';
        div.style.margin = '0';
        div.style.padding = '0';
        div.style.whiteSpace = 'pre';
        div.style.overflow = 'hidden';
        div.style.transformOrigin = 'left top';
        div.style.fontFamily = 'sans-serif';
        div.style.userSelect = 'text';
        (div.style as any).webkitUserSelect = 'text';

        textLayer.appendChild(div);
    }

    logPdfLayoutTrace('text-layer.update.complete', {
        path,
        pageIndex,
        nodeCount: textLayer.childElementCount,
    });
}
