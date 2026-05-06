// Comment facade — frozen v1 TS bindings against `crate::comment::facade`.
// See docs/api-contract.md.

import { getWasmApi } from '../shared/wasm_loader';

export type StubResult = { implemented: boolean; error: string };

function call<T>(name: string, ...args: unknown[]): T | null {
    const api = getWasmApi();
    const fn = (api as any)[name];
    if (typeof fn !== 'function') return null;
    try { return args.length ? fn(...args) : fn(); } catch { return null; }
}

async function callAsync<T>(name: string, ...args: unknown[]): Promise<T | null> {
    const api = getWasmApi();
    const fn = (api as any)[name];
    if (typeof fn !== 'function') return null;
    try { return (await (args.length ? fn(...args) : fn())) as T; } catch { return null; }
}

// Stable — session
export function facadeCommentClearReviewSession(): void { call('commentFacadeClearReviewSession'); }
export function facadeCommentReadReviewSession(): unknown { return call('commentFacadeReadReviewSession'); }

// Stable — listings
export function facadeCommentListPageComments(path: string, pageIndex: number) {
    return callAsync('commentFacadeListPageComments', path, pageIndex);
}
export function facadeCommentListPageAnnotationTargets(path: string, pageIndex: number) {
    return callAsync('commentFacadeListPageAnnotationTargets', path, pageIndex);
}

// Stable — review pipeline
export function facadeCommentReviewDocument(path: string, request: unknown) {
    return callAsync('commentFacadeReviewDocument', path, request);
}
export function facadeCommentLoadReview(path: string, currentPage: number) {
    return callAsync('commentFacadeLoadReview', path, currentPage);
}
export function facadeCommentLoadOverlay(path: string, currentPage: number) {
    return callAsync('commentFacadeLoadOverlay', path, currentPage);
}
export function facadeCommentLoadTargetOverlay(path: string, currentPage: number) {
    return callAsync('commentFacadeLoadTargetOverlay', path, currentPage);
}
export function facadeCommentSetPanelOpenAndLoad(path: string, currentPage: number, panelOpen: boolean) {
    return callAsync('commentFacadeSetPanelOpenAndLoad', path, currentPage, panelOpen);
}
export function facadeCommentTogglePanelAndLoad(path: string, currentPage: number) {
    return callAsync('commentFacadeTogglePanelAndLoad', path, currentPage);
}
export function facadeCommentSetScopeAndLoad(path: string, currentPage: number, scope: 'page' | 'document') {
    return callAsync('commentFacadeSetScopeAndLoad', path, currentPage, scope);
}
export function facadeCommentSetQueryAndLoad(path: string, currentPage: number, query: string) {
    return callAsync('commentFacadeSetQueryAndLoad', path, currentPage, query);
}
export function facadeCommentSelectAndLoad(path: string, currentPage: number, selectedCommentId: string | null) {
    return callAsync('commentFacadeSelectAndLoad', path, currentPage, selectedCommentId);
}

// Stable — mutation
export function facadeCommentAddRegionComment(path: string, request: unknown) {
    return callAsync('commentFacadeAddRegionComment', path, request);
}
export function facadeCommentDeleteAnnotation(path: string, request: unknown) {
    return callAsync('commentFacadeDeleteAnnotation', path, request);
}
export function facadeCommentUpdateComment(path: string, request: unknown) {
    return callAsync('commentFacadeUpdateComment', path, request);
}

// Stubs
export function facadeCommentReplyComment(path: string, parentId: string, contents: string): StubResult | null {
    return call('commentFacadeReplyComment', path, parentId, contents);
}
export function facadeCommentSetResolved(path: string, annotationId: string, resolved: boolean): StubResult | null {
    return call('commentFacadeSetResolved', path, annotationId, resolved);
}
export function facadeCommentExport(path: string, format: string): StubResult | null {
    return call('commentFacadeExport', path, format);
}
export function facadeCommentImport(targetPath: string, sourcePath: string): StubResult | null {
    return call('commentFacadeImport', targetPath, sourcePath);
}

