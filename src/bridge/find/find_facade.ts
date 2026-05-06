import { getWasmApi, targetInvokeV3 } from '../shared/wasm_loader';

// ── Search types ──

export type SearchMatch = {
    id: string;
    kind: string;
    pageIndex: number;
    pageWidth: number;
    pageHeight: number;
    lineIndex: number;
    sourceText: string;
    previewText: string;
    matchedText: string;
    objectIndices: number[];
    boxRect: {
        left: number;
        top: number;
        width: number;
        height: number;
    };
};

export type SearchResult = {
    query: string;
    totalMatches: number;
    matches: SearchMatch[];
};

export type SearchPageRequest = {
    path: string;
    pageIndex: number;
    query: string;
    caseSensitive: boolean;
};

export type SearchDocumentRequest = {
    path: string;
    pageCount: number;
    query: string;
    caseSensitive: boolean;
};

export type ReplaceRequest = {
    path: string;
    pageIndex: number;
    regionId: string;
    kind: string;
    originalText: string;
    query: string;
    replacement: string;
    caseSensitive: boolean;
};

export type ReplaceResult = {
    applied: boolean;
    pageIndex: number;
};

export type BatchReplaceRequest = {
    path: string;
    pageCount: number;
    query: string;
    replacement: string;
    caseSensitive: boolean;
};

export type BatchReplaceResult = {
    appliedCount: number;
    skippedCount: number;
    touchedPages: number[];
};

export type FindSession = {
    query: string;
    scope: string;
    pageIndices: number[];
    currentPage: number;
    activeIndex: number;
};

export type FindFacadeResult = {
    changed: boolean;
    session: FindSession | null;
    navigation: {
        activeIndex: number;
        targetPage: number | null;
    } | null;
    renderFrame: unknown | null;
};

// ── Review types ──

export type ReviewChangeEntry = {
    patchKey: string;
    pageIndex: number;
    regionId: string;
    kind: string | null;
    originalText: string;
    currentText: string;
    source: string;
};

export type ReviewFeedResult = {
    revision: number;
    pendingCount: number;
    changes: ReviewChangeEntry[];
};

export type ReviewLocateResult = {
    pageIndex: number;
    regionId: string;
    kind: string | null;
    originalText: string;
};

export type ReviewFacadeResult = {
    changed: boolean;
    feed: ReviewFeedResult | null;
    locateResult: ReviewLocateResult | null;
    renderFrame: unknown | null;
};

// ── Helpers ──

function callWasm<T>(fnName: string, arg?: unknown): T | null {
    const api = getWasmApi();
    const fn = (api as any)[fnName];
    if (typeof fn !== 'function') return null;
    try {
        return arg !== undefined ? fn(arg) : fn();
    } catch {
        return null;
    }
}

// ── Search API ──

export function findInPage(request: SearchPageRequest): SearchResult | null {
    return null;
}

export async function findInPageAsync(request: SearchPageRequest): Promise<SearchResult | null> {
    try {
        return await targetInvokeV3('find_in_page', {
            path: request.path,
            pageIndex: request.pageIndex,
            query: request.query,
            caseSensitive: request.caseSensitive,
        }) as SearchResult;
    } catch {
        return null;
    }
}

export function findInDocument(request: SearchDocumentRequest): SearchResult | null {
    return null;
}

export async function findInDocumentAsync(request: SearchDocumentRequest): Promise<SearchResult | null> {
    try {
        return await targetInvokeV3('find_in_document', {
            path: request.path,
            pageCount: request.pageCount,
            query: request.query,
            caseSensitive: request.caseSensitive,
        }) as SearchResult;
    } catch {
        return null;
    }
}

export function replaceOne(request: ReplaceRequest): ReplaceResult | null {
    return callWasm<ReplaceResult>('searchFacadeReplace', request);
}

export function replaceAll(request: BatchReplaceRequest): BatchReplaceResult | null {
    return callWasm<BatchReplaceResult>('searchFacadeBatchReplace', request);
}

export function setFindSession(session: FindSession): FindFacadeResult | null {
    return callWasm<FindFacadeResult>('searchFacadeSetSession', session);
}

export function clearFindSession(): FindFacadeResult | null {
    return callWasm<FindFacadeResult>('searchFacadeClearSession');
}

export function moveFindMatch(step: number): FindFacadeResult | null {
    return callWasm<FindFacadeResult>('searchFacadeMoveMatch', step);
}

export function getFindSession(): FindSession | null {
    return callWasm<FindSession>('searchFacadeGetSession');
}

// ── Review API ──

export function getReviewFeed(): ReviewFeedResult | null {
    return callWasm<ReviewFeedResult>('reviewFacadeReadFeed');
}

export function acceptChange(patchKey: string): ReviewFacadeResult | null {
    const r = callWasm<{ changed: boolean }>('reviewFacadeAccept', patchKey);
    return r ? { changed: r.changed, feed: null, locateResult: null, renderFrame: null } : null;
}

export function rejectChange(patchKey: string): ReviewFacadeResult | null {
    const r = callWasm<{ changed: boolean }>('reviewFacadeReject', patchKey);
    return r ? { changed: r.changed, feed: null, locateResult: null, renderFrame: null } : null;
}

export function acceptAllChanges(): ReviewFacadeResult | null {
    const r = callWasm<{ changed: boolean }>('reviewFacadeAcceptAll');
    return r ? { changed: r.changed, feed: null, locateResult: null, renderFrame: null } : null;
}

export function rejectAllChanges(): ReviewFacadeResult | null {
    const r = callWasm<{ changed: boolean }>('reviewFacadeRejectAll');
    return r ? { changed: r.changed, feed: null, locateResult: null, renderFrame: null } : null;
}

export function locateChange(patchKey: string): ReviewLocateResult | null {
    return callWasm<ReviewLocateResult>('locate_review_change', patchKey);
}

