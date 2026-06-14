import type { RenderReason } from './frame_plan';
import { emitPdfDiagnostic } from '../shared/diagnostics';
import type { PagePresentationRuntimeAdapter } from '../viewer/page_presentation_runtime';

export type RenderSource = 'navigation' | 'scroll' | 'zoom' | 'editor' | 'mutation' | 'default';

export type RenderRequestContext = {
    pageTurnId?: number;
    targetPage?: number;
};

export type RenderRequest = RenderRequestContext & {
    source: RenderSource;
    reason: RenderReason;
    issuedAt: number;
};

export type RenderSchedulerDeps = {
    executeRender: (request: RenderRequest) => Promise<void>;
    pagePresentationRuntime: PagePresentationRuntimeAdapter;
    presentPagePreview?: (pageIndex: number) => Promise<boolean>;
};

export type RenderScheduler = {
    requestRender: (source: RenderSource, reason?: RenderReason, context?: RenderRequestContext) => Promise<void>;
    notifyCommit: () => void;
    reset: () => void;
};

type QueuedRenderRequest = {
    request: RenderRequest;
    resolve: () => void;
};

export function createRenderScheduler(deps: RenderSchedulerDeps): RenderScheduler {
    // --- Debounce state ---
    let scrollTimerId: number | null = null;
    let scrollRafId: number | null = null;
    let scrollResolvers: Array<() => void> = [];

    // --- Serialization state ---
    let executing = false;
    let pendingQueue: QueuedRenderRequest[] = [];

    // --- Commit suppression state ---
    let lastCommitTs = 0;

    function resolveQueueAction(source: RenderSource) {
        return deps.pagePresentationRuntime.resolveRenderQueueAction(
            source,
            executing,
            performance.now(),
            lastCommitTs,
        );
    }

    function isScrollSuppressed(): boolean {
        return resolveQueueAction('scroll').suppress;
    }

    function dispatch(request: RenderRequest): Promise<void> {
        const action = resolveQueueAction(request.source);
        if (action.action === 'suppress') {
            return Promise.resolve();
        }

        if (request.source === 'navigation' && Number.isFinite(request.targetPage as number) && deps.presentPagePreview) {
            void deps.presentPagePreview(request.targetPage as number);
        }

        if (action.action !== 'dispatch') {
            return new Promise<void>((resolve) => {
                if (action.pendingQueueEffect === 'append') {
                    pendingQueue.push({ request, resolve });
                    emitPdfDiagnostic('render-flow', 'render-scheduler.navigation-queued', {
                        pageTurnId: request.pageTurnId,
                        targetPage: request.targetPage,
                        queueSize: pendingQueue.length,
                    }, { verboseOnly: true });
                    return;
                }

                if (action.pendingQueueEffect === 'replaceAll') {
                    const replaced = pendingQueue.length;
                    pendingQueue.splice(0).forEach((queued) => queued.resolve());
                    pendingQueue = [{ request, resolve }];
                    emitPdfDiagnostic('render-flow', 'render-scheduler.navigation-replaced', {
                        pageTurnId: request.pageTurnId,
                        targetPage: request.targetPage,
                        replaced,
                        queueSize: pendingQueue.length,
                    }, { verboseOnly: true });
                    return;
                }

                if (action.pendingQueueEffect === 'replaceNonNavigation') {
                    const keptNavigation = pendingQueue.filter((queued) => queued.request.source === 'navigation');
                    const replaced = pendingQueue.length - keptNavigation.length;
                    pendingQueue
                        .filter((queued) => queued.request.source !== 'navigation')
                        .forEach((queued) => queued.resolve());
                    pendingQueue = [...keptNavigation, { request, resolve }];
                    if (replaced > 0) {
                        emitPdfDiagnostic('render-flow', 'render-scheduler.non-navigation-replaced', {
                            source: request.source,
                            reason: request.reason,
                            replaced,
                            queueSize: pendingQueue.length,
                        }, { verboseOnly: true });
                    }
                    return;
                }

                resolve();
            });
        }

        executing = true;
        return deps.executeRender(request).finally(() => {
            executing = false;
            const next = pendingQueue.shift();
            if (next) {
                dispatch(next.request).then(() => {
                    next.resolve();
                });
            }
        });
    }

    function makeRequest(
        source: RenderSource,
        reason: RenderReason,
        context: RenderRequestContext = {},
    ): RenderRequest {
        return {
            source,
            reason,
            pageTurnId: context.pageTurnId,
            targetPage: context.targetPage,
            issuedAt: performance.now(),
        };
    }

    function requestScroll(request: RenderRequest): Promise<void> {
        const action = resolveQueueAction('scroll');
        if (action.suppress) return Promise.resolve();
        const scrollDebounceMs = Number.isFinite(action.scrollDebounceMs)
            ? action.scrollDebounceMs
            : 56;

        return new Promise<void>((resolve) => {
            scrollResolvers.push(resolve);
            if (scrollTimerId !== null) {
                clearTimeout(scrollTimerId);
            }
            scrollTimerId = window.setTimeout(() => {
                scrollTimerId = null;
                if (scrollRafId !== null) return;
                scrollRafId = requestAnimationFrame(() => {
                    scrollRafId = null;
                    if (isScrollSuppressed()) {
                        const resolvers = scrollResolvers.splice(0);
                        resolvers.forEach((r) => r());
                        return;
                    }
                    const resolvers = scrollResolvers.splice(0);
                    dispatch(request).then(() => {
                        resolvers.forEach((r) => r());
                    });
                });
            }, scrollDebounceMs);
        });
    }

    function requestRender(
        source: RenderSource,
        reason: RenderReason = 'default',
        context: RenderRequestContext = {},
    ): Promise<void> {
        const request = makeRequest(source, reason, context);
        switch (source) {
            case 'navigation':
                // Route navigation immediately to dispatch without debounce delay
                return dispatch(request);
            case 'scroll':
                return requestScroll(request);
            default:
                // zoom, editor, mutation, default → dispatch immediately
                return dispatch(request);
        }
    }

    function notifyCommit(): void {
        lastCommitTs = performance.now();
    }

    function reset(): void {
        if (scrollTimerId !== null) {
            clearTimeout(scrollTimerId);
            scrollTimerId = null;
        }
        if (scrollRafId !== null) {
            cancelAnimationFrame(scrollRafId);
            scrollRafId = null;
        }
        scrollResolvers.splice(0).forEach((r) => r());

        pendingQueue.splice(0).forEach((queued) => queued.resolve());
        executing = false;
        lastCommitTs = 0;
    }

    return { requestRender, notifyCommit, reset };
}
