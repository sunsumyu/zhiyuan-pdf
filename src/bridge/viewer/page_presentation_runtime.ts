export type PageTurnDecision = {
    accepted: boolean;
    pageTurnId: number;
    targetPage: number;
    previousPage: number;
    direction: -1 | 0 | 1;
    reason: string;
    rejectReason?: string | null;
    snapshot?: {
        fastFlipMode?: boolean;
        [key: string]: unknown;
    } | null;
};

export type PageVisibleDecision = {
    accepted: boolean;
    pageTurnId: number;
    pageIndex: number;
    surface: string;
    canPrefetch: boolean;
    rejectReason?: string | null;
    snapshot?: unknown;
};

export type PageAssetAdmission = {
    accepted: boolean;
    pageIndex: number;
    role: string;
    assetKind: string;
    priority: number;
    rejectReason?: string | null;
    snapshot?: unknown;
};

export type PagePrefetchTarget = {
    pageIndex: number;
    priority: number;
    direction: -1 | 0 | 1;
    assetKind: string;
};

export type PagePrefetchDecision = {
    allowed: boolean;
    anchorPage: number;
    pageTurnId: number;
    targets: PagePrefetchTarget[];
    rejectReason?: string | null;
    snapshot?: unknown;
};

export type RenderQueueAction = {
    action:
        | 'suppress'
        | 'dispatch'
        | 'appendNavigation'
        | 'replacePendingNavigation'
        | 'replacePendingNonNavigation';
    source: string;
    suppress: boolean;
    scrollDebounceMs: number;
    pendingQueueEffect: 'none' | 'append' | 'replaceAll' | 'replaceNonNavigation';
    rejectReason?: string | null;
};

export type PagePresentationRuntimeAdapter = {
    requestPageTurn: (targetPage: number, reason: string, nowMs?: number) => PageTurnDecision;
    readPageTurn: () => unknown;
    isLatestPageTurn: (pageTurnId: number, pageIndex: number) => boolean;
    markPageVisible: (pageIndex: number, surface: string) => PageVisibleDecision;
    canPrefetch: (pageIndex: number) => boolean;
    admitPageAsset: (pageIndex: number, role: string, assetKind: string) => PageAssetAdmission;
    decideAdjacentPrefetch: (anchorPage: number, pageCount: number) => PagePrefetchDecision;
    resolveRenderQueueAction: (
        source: string,
        executing: boolean,
        nowMs: number,
        lastCommitMs: number,
    ) => RenderQueueAction;
    reset: () => void;
};

type PagePresentationRuntimeDeps = {
    getWasmApi: () => any;
};

const COMMIT_SUPPRESS_MS = 120;

let runtimeHandle: any = null;

function getRuntimeHandle(getWasmApi: () => any): any {
    if (!runtimeHandle) {
        const api = getWasmApi();
        if (typeof api?.PagePresentationRuntime === 'function') {
            runtimeHandle = new api.PagePresentationRuntime();
        }
    }
    return runtimeHandle;
}

function normalizeDecision(value: any, targetPage: number, reason: string): PageTurnDecision {
    return {
        accepted: !!value?.accepted,
        pageTurnId: Number(value?.pageTurnId ?? 0),
        targetPage: Number(value?.targetPage ?? targetPage),
        previousPage: Number(value?.previousPage ?? 0),
        direction: Number(value?.direction ?? 0) as -1 | 0 | 1,
        reason: String(value?.reason ?? reason),
        rejectReason: value?.rejectReason ?? null,
        snapshot: value?.snapshot ?? null,
    };
}

function normalizeVisibleDecision(value: any, pageIndex: number, surface: string): PageVisibleDecision {
    return {
        accepted: !!value?.accepted,
        pageTurnId: Number(value?.pageTurnId ?? 0),
        pageIndex: Number(value?.pageIndex ?? pageIndex),
        surface: String(value?.surface ?? surface),
        canPrefetch: !!value?.canPrefetch,
        rejectReason: value?.rejectReason ?? null,
        snapshot: value?.snapshot ?? null,
    };
}

function normalizeAssetAdmission(value: any, pageIndex: number, role: string, assetKind: string): PageAssetAdmission {
    return {
        accepted: !!value?.accepted,
        pageIndex: Number(value?.pageIndex ?? pageIndex),
        role: String(value?.role ?? role),
        assetKind: String(value?.assetKind ?? assetKind),
        priority: Number(value?.priority ?? 0),
        rejectReason: value?.rejectReason ?? null,
        snapshot: value?.snapshot ?? null,
    };
}

function normalizePrefetchDecision(value: any, anchorPage: number): PagePrefetchDecision {
    const targets = Array.isArray(value?.targets)
        ? value.targets.map((target: any) => ({
            pageIndex: Number(target?.pageIndex ?? -1),
            priority: Number(target?.priority ?? 0),
            direction: Number(target?.direction ?? 0) as -1 | 0 | 1,
            assetKind: String(target?.assetKind ?? 'unknown'),
        })).filter((target: PagePrefetchTarget) => target.pageIndex >= 0)
        : [];
    return {
        allowed: !!value?.allowed,
        anchorPage: Number(value?.anchorPage ?? anchorPage),
        pageTurnId: Number(value?.pageTurnId ?? 0),
        targets,
        rejectReason: value?.rejectReason ?? null,
        snapshot: value?.snapshot ?? null,
    };
}

function fallbackRenderQueueAction(
    source: string,
    executing: boolean,
    nowMs: number,
    lastCommitMs: number,
): RenderQueueAction {
    const normalizedSource = ['navigation', 'scroll', 'zoom', 'editor', 'mutation', 'default'].includes(source)
        ? source
        : 'default';
    const sinceCommitMs = Number.isFinite(nowMs) && Number.isFinite(lastCommitMs)
        ? nowMs - lastCommitMs
        : COMMIT_SUPPRESS_MS;

    if (normalizedSource === 'scroll' && sinceCommitMs >= 0 && sinceCommitMs < COMMIT_SUPPRESS_MS) {
        return {
            action: 'suppress',
            source: normalizedSource,
            suppress: true,
            scrollDebounceMs: 56,
            pendingQueueEffect: 'none',
            rejectReason: 'recentCommit',
        };
    }

    if (!executing) {
        return {
            action: 'dispatch',
            source: normalizedSource,
            suppress: false,
            scrollDebounceMs: 56,
            pendingQueueEffect: 'none',
            rejectReason: null,
        };
    }

    const action = normalizedSource === 'navigation' ? 'replacePendingNavigation' : 'replacePendingNonNavigation';
    return {
        action,
        source: normalizedSource,
        suppress: false,
        scrollDebounceMs: 56,
        pendingQueueEffect: action === 'replacePendingNavigation' ? 'replaceAll' : 'replaceNonNavigation',
        rejectReason: null,
    };
}

function normalizePendingQueueEffect(value: any, action: RenderQueueAction['action']): RenderQueueAction['pendingQueueEffect'] {
    const effect = String(value ?? '');
    if (['none', 'append', 'replaceAll', 'replaceNonNavigation'].includes(effect)) {
        return effect as RenderQueueAction['pendingQueueEffect'];
    }
    switch (action) {
        case 'appendNavigation':
            return 'append';
        case 'replacePendingNavigation':
            return 'replaceAll';
        case 'replacePendingNonNavigation':
            return 'replaceNonNavigation';
        default:
            return 'none';
    }
}

function normalizeRenderQueueAction(
    value: any,
    source: string,
    executing: boolean,
    nowMs: number,
    lastCommitMs: number,
): RenderQueueAction {
    const fallback = fallbackRenderQueueAction(source, executing, nowMs, lastCommitMs);
    const action = String(value?.action ?? fallback.action);
    if (![
        'suppress',
        'dispatch',
        'appendNavigation',
        'replacePendingNavigation',
        'replacePendingNonNavigation',
    ].includes(action)) {
        return fallback;
    }
    return {
        action: action as RenderQueueAction['action'],
        source: String(value?.source ?? fallback.source),
        suppress: Boolean(value?.suppress ?? fallback.suppress),
        scrollDebounceMs: Number.isFinite(Number(value?.scrollDebounceMs))
            ? Number(value.scrollDebounceMs)
            : fallback.scrollDebounceMs,
        pendingQueueEffect: normalizePendingQueueEffect(value?.pendingQueueEffect, action as RenderQueueAction['action']),
        rejectReason: value?.rejectReason ?? null,
    };
}

export function createPagePresentationRuntimeAdapter(
    deps: PagePresentationRuntimeDeps,
): PagePresentationRuntimeAdapter {
    function runtime(): any {
        return getRuntimeHandle(deps.getWasmApi);
    }

    function requestPageTurn(targetPage: number, reason: string, nowMs?: number): PageTurnDecision {
        try {
            const now = nowMs ?? performance.now();
            const decision = runtime()?.requestPageTurn(targetPage, reason, now);
            return normalizeDecision(decision, targetPage, reason);
        } catch {
            return {
                accepted: false,
                pageTurnId: 0,
                targetPage,
                previousPage: 0,
                direction: 0,
                reason,
                rejectReason: 'runtimeUnavailable',
            };
        }
    }

    function readPageTurn(): unknown {
        try {
            return runtime()?.readPageTurn() ?? null;
        } catch {
            return null;
        }
    }

    function isLatestPageTurn(pageTurnId: number, pageIndex: number): boolean {
        try {
            return runtime()?.isLatestPageTurn(pageTurnId, pageIndex) !== false;
        } catch {
            return true;
        }
    }

    function markPageVisible(pageIndex: number, surface: string): PageVisibleDecision {
        try {
            const decision = runtime()?.markPageVisible(pageIndex, surface);
            return normalizeVisibleDecision(decision, pageIndex, surface);
        } catch {
            return {
                accepted: false,
                pageTurnId: 0,
                pageIndex,
                surface,
                canPrefetch: false,
                rejectReason: 'runtimeUnavailable',
            };
        }
    }

    function canPrefetch(pageIndex: number): boolean {
        try {
            return runtime()?.canPrefetch(pageIndex) === true;
        } catch {
            return false;
        }
    }

    function admitPageAsset(pageIndex: number, role: string, assetKind: string): PageAssetAdmission {
        try {
            const decision = runtime()?.admitPageAsset(pageIndex, role, assetKind);
            return normalizeAssetAdmission(decision, pageIndex, role, assetKind);
        } catch {
            return {
                accepted: role === 'current',
                pageIndex,
                role,
                assetKind,
                priority: role === 'current' ? 50 : 0,
                rejectReason: 'runtimeUnavailable',
            };
        }
    }

    function decideAdjacentPrefetch(anchorPage: number, pageCount: number): PagePrefetchDecision {
        try {
            const decision = runtime()?.decideAdjacentPrefetch(anchorPage, Math.min(pageCount, 65535));
            return normalizePrefetchDecision(decision, anchorPage);
        } catch {
            return {
                allowed: false,
                anchorPage,
                pageTurnId: 0,
                targets: [],
                rejectReason: 'runtimeUnavailable',
            };
        }
    }

    function resolveRenderQueueAction(
        source: string,
        executing: boolean,
        nowMs: number,
        lastCommitMs: number,
    ): RenderQueueAction {
        try {
            const action = runtime()?.resolveRenderQueueAction(source, executing, nowMs, lastCommitMs);
            return normalizeRenderQueueAction(action, source, executing, nowMs, lastCommitMs);
        } catch {
            return fallbackRenderQueueAction(source, executing, nowMs, lastCommitMs);
        }
    }

    function reset(): void {
        try {
            runtime()?.reset();
        } catch {}
    }

    return {
        requestPageTurn,
        readPageTurn,
        isLatestPageTurn,
        markPageVisible,
        canPrefetch,
        admitPageAsset,
        decideAdjacentPrefetch,
        resolveRenderQueueAction,
        reset,
    };
}
