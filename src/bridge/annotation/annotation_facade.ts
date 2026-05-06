// Annotation facade — frozen v1 TS bindings (mostly stub).
// See docs/api-contract.md.

import { getWasmApi } from '../shared/wasm_loader';

export type StubResult = { implemented: boolean; error: string };

function call<T>(name: string, ...args: unknown[]): T | null {
    const api = getWasmApi();
    const fn = (api as any)[name];
    if (typeof fn !== 'function') return null;
    try { return args.length ? fn(...args) : fn(); } catch { return null; }
}

// Highlight
export function facadeAnnotationAddHighlight(req: unknown): StubResult | null {
    return call('annotationFacadeAddHighlight', req);
}
export function facadeAnnotationDeleteHighlight(req: unknown): StubResult | null {
    return call('annotationFacadeDeleteHighlight', req);
}
export function facadeAnnotationListHighlights(path: string, pageIndex: number): StubResult | null {
    return call('annotationFacadeListHighlights', path, pageIndex);
}

// Ink
export function facadeAnnotationAddInk(req: unknown): StubResult | null {
    return call('annotationFacadeAddInk', req);
}
export function facadeAnnotationDeleteInk(path: string, annotationId: string): StubResult | null {
    return call('annotationFacadeDeleteInk', path, annotationId);
}

// Free-text
export function facadeAnnotationAddFreeText(req: unknown): StubResult | null {
    return call('annotationFacadeAddFreeText', req);
}
export function facadeAnnotationUpdateFreeText(req: unknown): StubResult | null {
    return call('annotationFacadeUpdateFreeText', req);
}

// Stamp
export function facadeAnnotationAddStamp(req: unknown): StubResult | null {
    return call('annotationFacadeAddStamp', req);
}

// Link
export function facadeAnnotationAddLink(req: unknown): StubResult | null {
    return call('annotationFacadeAddLink', req);
}
export function facadeAnnotationListLinks(path: string, pageIndex: number): StubResult | null {
    return call('annotationFacadeListLinks', path, pageIndex);
}

// General
export function facadeAnnotationListAll(path: string, pageIndex: number): StubResult | null {
    return call('annotationFacadeListAll', path, pageIndex);
}
export function facadeAnnotationDelete(path: string, annotationId: string): StubResult | null {
    return call('annotationFacadeDelete', path, annotationId);
}
export function facadeAnnotationMove(path: string, annotationId: string, newBox: unknown): StubResult | null {
    return call('annotationFacadeMove', path, annotationId, newBox);
}
export function facadeAnnotationRestyle(path: string, annotationId: string, style: unknown): StubResult | null {
    return call('annotationFacadeRestyle', path, annotationId, style);
}

