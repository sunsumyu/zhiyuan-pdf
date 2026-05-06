// Document facade — frozen v1 TS bindings.
// See docs/api-contract.md.

import { getWasmApi } from '../shared/wasm_loader';

export type DocumentOpenRequest = {
    path: string;
    initialZoom?: number;
};

export type DocumentPickRequest = {
    initialZoom?: number;
};

export type DocumentSessionOpenRequest = {
    path: string | null;
    pageCount: number;
    initialZoom: number;
    pageWidth?: number;
    pageHeight?: number;
};

export type StubResult = { implemented: boolean; error: string };

function call<T>(name: string, ...args: unknown[]): T | null {
    const api = getWasmApi();
    const fn = (api as any)[name];
    if (typeof fn !== 'function') return null;
    try {
        return args.length > 0 ? fn(...args) : fn();
    } catch {
        return null;
    }
}

async function callAsync<T>(name: string, ...args: unknown[]): Promise<T | null> {
    const api = getWasmApi();
    const fn = (api as any)[name];
    if (typeof fn !== 'function') return null;
    try {
        return (await (args.length > 0 ? fn(...args) : fn())) as T;
    } catch {
        return null;
    }
}

// ─── Stable ──────────────────────────────────────────────────────────────────

export async function facadeDocumentOpen(req: DocumentOpenRequest): Promise<unknown> {
    return callAsync('documentFacadeOpen', req);
}

export async function facadeDocumentPick(req: DocumentPickRequest): Promise<unknown> {
    return callAsync('documentFacadePick', req);
}

export function facadeDocumentClose(width: number, height: number): unknown {
    return call('documentFacadeClose', width, height);
}

export function facadeDocumentUndo(): unknown {
    return call('documentFacadeUndo');
}

export function facadeDocumentRedo(): unknown {
    return call('documentFacadeRedo');
}

export async function facadeDocumentRotate(delta: number): Promise<unknown> {
    return callAsync('documentFacadeRotate', delta);
}

export function facadeDocumentRequestRefresh(reason: string, frameRequest: unknown): unknown {
    return call('documentFacadeRequestRefresh', reason, frameRequest);
}

export function facadeDocumentBumpRevision(reason: string): number {
    const r = call<number>('documentFacadeBumpRevision', reason);
    return typeof r === 'number' ? r : 0;
}

export function facadeDocumentOpenSession(req: DocumentSessionOpenRequest): unknown {
    return call('documentFacadeOpenSession', req);
}

export function facadeDocumentResetSession(width: number, height: number): unknown {
    return call('documentFacadeResetSession', width, height);
}

export function facadeDocumentApplyPatch(patch: unknown): void {
    const api = getWasmApi();
    (api as any)['documentFacadeApplyPatch']?.(patch);
}

export function facadeDocumentBuildRegionPatch(
    pageIndex: number,
    regionId: string,
    kind: string,
    originalText: string,
    newText: string,
): unknown {
    return call('documentFacadeBuildRegionPatch', pageIndex, regionId, kind, originalText, newText);
}

export function facadeDocumentApplyRegionReplacements(
    replacements: unknown[],
    frameRequest: unknown,
): unknown {
    return call('documentFacadeApplyRegionReplacements', replacements, frameRequest);
}

// ─── Stubs (reserved) ────────────────────────────────────────────────────────

export function facadeDocumentInsertPage(index: number, sourcePath?: string): StubResult | null {
    return call('documentFacadeInsertPage', index, sourcePath ?? null);
}

export function facadeDocumentRemovePage(index: number): StubResult | null {
    return call('documentFacadeRemovePage', index);
}

export function facadeDocumentMovePage(from: number, to: number): StubResult | null {
    return call('documentFacadeMovePage', from, to);
}

export function facadeDocumentRotatePage(index: number, delta: number): StubResult | null {
    return call('documentFacadeRotatePage', index, delta);
}

export function facadeDocumentReadMetadata(): StubResult | null {
    return call('documentFacadeReadMetadata');
}

export function facadeDocumentSetMetadata(metadata: unknown): StubResult | null {
    return call('documentFacadeSetMetadata', metadata);
}

export function facadeDocumentExportPages(
    pageIndices: number[],
    format: string,
    outputPath: string,
): StubResult | null {
    return call('documentFacadeExportPages', pageIndices, format, outputPath);
}

export function facadeDocumentSetPassword(owner: string, user: string): StubResult | null {
    return call('documentFacadeSetPassword', owner, user);
}

export function facadeDocumentRemovePassword(): StubResult | null {
    return call('documentFacadeRemovePassword');
}

export function facadeDocumentReadOutline(): StubResult | null {
    return call('documentFacadeReadOutline');
}

export function facadeDocumentSetOutline(outline: unknown): StubResult | null {
    return call('documentFacadeSetOutline', outline);
}

export function facadeDocumentFlatten(): StubResult | null {
    return call('documentFacadeFlatten');
}

export function facadeDocumentReadFormFields(): StubResult | null {
    return call('documentFacadeReadFormFields');
}

export function facadeDocumentFillFormField(name: string, value: string): StubResult | null {
    return call('documentFacadeFillFormField', name, value);
}

export function facadeDocumentReadSignatures(): StubResult | null {
    return call('documentFacadeReadSignatures');
}

export function facadeDocumentReadAttachments(): StubResult | null {
    return call('documentFacadeReadAttachments');
}

