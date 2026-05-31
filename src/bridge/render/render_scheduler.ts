import type { RenderReason } from './frame_plan';

export type RenderSource = 'navigation' | 'scroll' | 'zoom' | 'editor' | 'mutation' | 'default';

export type RenderSchedulerDeps = {
    executeRender: (reason: RenderReason) => Promise<void>;
};

export type RenderScheduler = {
    requestRender: (source: RenderSource, reason?: RenderReason) => Promise<void>;
    notifyCommit: () => void;
    reset: () => void;
};

const COMMIT_SUPPRESS_MS = 120;
const SCROLL_DEBOUNCE_MS = 56;
const NAVIGATION_DEBOUNCE_MS = 60;

export function createRenderScheduler(deps: RenderSchedulerDeps): RenderScheduler {
    // --- Debounce state ---
    let navRafId: number | null = null;
    let navTimerId: number | null = null;
    let navResolvers: Array<() => void> = [];

    let scrollTimerId: number | null = null;
    let scrollRafId: number | null = null;
    let scrollResolvers: Array<() => void> = [];

    // --- Serialization state ---
    let executing = false;
    let pendingReason: RenderReason | null = null;
    let pendingResolvers: Array<() => void> = [];

    // --- Commit suppression state ---
    let lastCommitTs = 0;

    function isScrollSuppressed(): boolean {
        return performance.now() - lastCommitTs < COMMIT_SUPPRESS_MS;
    }

    function dispatch(reason: RenderReason): Promise<void> {
        if (executing) {
            // Replace any previously queued request with the latest
            pendingReason = reason;
            return new Promise<void>((resolve) => {
                pendingResolvers.push(resolve);
            });
        }

        executing = true;
        return deps.executeRender(reason).finally(() => {
            executing = false;
            if (pendingReason !== null) {
                const nextReason = pendingReason;
                const resolvers = pendingResolvers.splice(0);
                pendingReason = null;
                dispatch(nextReason).then(() => {
                    resolvers.forEach((r) => r());
                });
            }
        });
    }

    function requestNavigation(reason: RenderReason): Promise<void> {
        return new Promise<void>((resolve) => {
            navResolvers.push(resolve);
            if (navTimerId !== null) {
                clearTimeout(navTimerId);
            }
            navTimerId = window.setTimeout(() => {
                navTimerId = null;
                if (navRafId !== null) return;
                navRafId = requestAnimationFrame(() => {
                    navRafId = null;
                    const resolvers = navResolvers.splice(0);
                    dispatch(reason).then(() => {
                        resolvers.forEach((r) => r());
                    });
                });
            }, NAVIGATION_DEBOUNCE_MS);
        });
    }

    function requestScroll(reason: RenderReason): Promise<void> {
        if (isScrollSuppressed()) return Promise.resolve();

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
                    dispatch(reason).then(() => {
                        resolvers.forEach((r) => r());
                    });
                });
            }, SCROLL_DEBOUNCE_MS);
        });
    }

    function requestRender(source: RenderSource, reason: RenderReason = 'default'): Promise<void> {
        switch (source) {
            case 'navigation':
                return requestNavigation(reason);
            case 'scroll':
                return requestScroll(reason);
            default:
                // zoom, editor, mutation, default → dispatch immediately
                return dispatch(reason);
        }
    }

    function notifyCommit(): void {
        lastCommitTs = performance.now();
    }

    function reset(): void {
        if (navTimerId !== null) {
            clearTimeout(navTimerId);
            navTimerId = null;
        }
        if (navRafId !== null) {
            cancelAnimationFrame(navRafId);
            navRafId = null;
        }
        navResolvers.splice(0).forEach((r) => r());

        if (scrollTimerId !== null) {
            clearTimeout(scrollTimerId);
            scrollTimerId = null;
        }
        if (scrollRafId !== null) {
            cancelAnimationFrame(scrollRafId);
            scrollRafId = null;
        }
        scrollResolvers.splice(0).forEach((r) => r());

        pendingReason = null;
        pendingResolvers.splice(0).forEach((r) => r());
        executing = false;
        lastCommitTs = 0;
    }

    return { requestRender, notifyCommit, reset };
}
