/**
 * Regression coverage for PagePresentationRuntime.
 *
 * Uses a tiny checked-in multi-page PDF so the rapid navigation path exercises
 * actual next/prev transitions instead of silently skipping.
 */
// eslint-disable-next-line @typescript-eslint/no-var-requires
const pagePresentationPath = require('node:path') as typeof import('node:path');
// eslint-disable-next-line @typescript-eslint/no-var-requires
const pagePresentationHelpers = require('../helpers/app') as typeof import('../helpers/app');

const pagePresentationRepoRoot = pagePresentationPath.resolve(__dirname, '..', '..', '..');
const pagePresentationFixturePath = pagePresentationPath.join(
    pagePresentationRepoRoot,
    'tests',
    'e2e',
    'fixtures',
    'multipage.pdf',
);

type DiagnosticWindow = Window & {
    __pagePresentationDiagnostics?: string[];
    __pagePresentationConsolePatched?: boolean;
    targetInvokeV3?: (command: string, args?: Record<string, unknown>) => Promise<unknown>;
    __targetInvokeV3?: (command: string, args?: Record<string, unknown>) => Promise<unknown>;
    pdfNextPage?: () => Promise<void> | void;
    pdfPrevPage?: () => Promise<void> | void;
    toggleAddTextMode?: () => Promise<void> | void;
    __pageShortcutTestAddTextEnabled?: boolean;
};

type PageSearchResult = {
    pageIndex: number;
    totalMatches: number;
    matches: Array<{
        pageIndex: number;
        sourceText: string;
        matchedText: string;
    }>;
};

type AnnotationTargetResult = {
    pageIndex: number;
    targets: Array<{
        pageIndex: number;
        label: string;
        kind: string;
    }>;
};

function normalizedFixturePath(): string {
    return pagePresentationFixturePath.replace(/\\/g, '/');
}

async function invokeTauriCommand<T>(
    command: string,
    args: Record<string, unknown> = {},
): Promise<T> {
    const result = await browser.executeAsync((cmd, commandArgs, done) => {
        const w = window as DiagnosticWindow;
        const invoke = w.targetInvokeV3 ?? w.__targetInvokeV3;
        if (typeof invoke !== 'function') {
            done({ ok: false, error: 'targetInvokeV3 not exposed' });
            return;
        }
        Promise.resolve(invoke(cmd as string, commandArgs as Record<string, unknown>))
            .then((value) => done({ ok: true, value }))
            .catch((error) =>
                done({
                    ok: false,
                    error: error && error.message ? error.message : String(error),
                }),
            );
    }, command, args) as { ok: boolean; value?: unknown; error?: string };

    if (!result?.ok) {
        throw new Error(`Tauri command ${command} failed: ${result?.error ?? 'unknown error'}`);
    }
    return result.value as T;
}

async function installDiagnosticCapture(): Promise<void> {
    await browser.execute(() => {
        const w = window as DiagnosticWindow;
        w.__pagePresentationDiagnostics = [];
        if (w.__pagePresentationConsolePatched) return;
        w.__pagePresentationConsolePatched = true;

        const originalLog = console.log.bind(console);
        const originalWarn = console.warn.bind(console);
        const originalError = console.error.bind(console);
        const capture = (level: string, args: unknown[]) => {
            const line = `[${level}] ${args.map((value) => {
                if (typeof value === 'string') return value;
                try {
                    return JSON.stringify(value);
                } catch {
                    return String(value);
                }
            }).join(' ')}`;
            if (
                line.includes('page-turn') ||
                line.includes('pageAsset') ||
                line.includes('render-scheduler') ||
                line.includes('stale')
            ) {
                w.__pagePresentationDiagnostics?.push(line);
            }
        };

        console.log = (...args: unknown[]) => {
            capture('log', args);
            originalLog(...args);
        };
        console.warn = (...args: unknown[]) => {
            capture('warn', args);
            originalWarn(...args);
        };
        console.error = (...args: unknown[]) => {
            capture('error', args);
            originalError(...args);
        };
    });
}

async function readViewerState(): Promise<{
    currentPage: number;
    totalPages: number;
    visiblePageText: string;
    diagnostics: string[];
}> {
    return browser.execute(() => {
        const w = window as DiagnosticWindow;
        const pageInput = document.getElementById('pdf-current-page-input') as HTMLInputElement | null;
        const pageText = pageInput?.value ?? '1';
        const totalText = document.getElementById('pdf-total-pages')?.textContent ?? '0';
        return {
            currentPage: Math.max(0, Number(pageText) - 1),
            totalPages: Number(totalText),
            visiblePageText: pageText,
            diagnostics: w.__pagePresentationDiagnostics ?? [],
        };
    }) as Promise<{
        currentPage: number;
        totalPages: number;
        visiblePageText: string;
        diagnostics: string[];
    }>;
}

describe('Page presentation runtime', () => {
    before(async () => {
        await pagePresentationHelpers.waitForApp();
    });

    it('keeps only the latest rapid navigation intent visible', async function () {
        await installDiagnosticCapture();
        await invokeTauriCommand<void>('clear_pdf_event_log');
        await invokeTauriCommand<void>('set_page_asset_test_delay_ms', { delayMs: 0 });
        await pagePresentationHelpers.loadFixturePdf(pagePresentationFixturePath);

        const initial = await readViewerState();
        if (initial.totalPages < 4) {
            this.skip();
            return;
        }

        await browser.executeAsync((done) => {
            const w = window as DiagnosticWindow;
            Promise.allSettled([
                w.pdfNextPage?.(),
                w.pdfNextPage?.(),
                w.pdfNextPage?.(),
                w.pdfPrevPage?.(),
            ])
                .then(() => done({ ok: true }))
                .catch((error) =>
                    done({
                        ok: false,
                        error: error && error.message ? error.message : String(error),
                    }),
                );
        });

        await browser.waitUntil(
            async () => {
                const state = await readViewerState();
                return state.currentPage === 2 && state.totalPages === 4;
            },
            {
                timeout: 10_000,
                interval: 250,
                timeoutMsg: 'rapid navigation did not settle on the latest expected page',
            },
        );

        const settled = await readViewerState();
        const errorDiagnostics = settled.diagnostics.filter((line) => line.startsWith('[error]'));
        if (errorDiagnostics.length > 0) {
            throw new Error(`unexpected page presentation console errors:\n${errorDiagnostics.join('\n')}`);
        }

        const visibleReady = settled.diagnostics.filter((line) => line.includes('page-turn.visible-ready'));
        const staleVisibleReady = visibleReady.filter((line) => !line.includes('pageTurnId=4'));
        if (staleVisibleReady.length > 0) {
            throw new Error(`stale page turns were presented:\n${staleVisibleReady.join('\n')}`);
        }
        const staleSkipped = settled.diagnostics.filter((line) =>
            line.includes('page-turn.stale-render-skipped'),
        );
        if (!staleSkipped.some((line) => line.includes('pageTurnId=1'))) {
            throw new Error(`stale running page turn was not reported:\n${settled.diagnostics.join('\n')}`);
        }
        if (!visibleReady.some((line) => line.includes('pageTurnId=4') && line.includes('visiblePage=2'))) {
            throw new Error(`latest page turn was not presented:\n${visibleReady.join('\n')}`);
        }

        const backendEvents = await invokeTauriCommand<string[]>('read_pdf_event_log');
        const currentBundleBegin = backendEvents.some((line) =>
            line.includes('pageAsset.bundle.begin') &&
            line.includes('role=current') &&
            line.includes('page=2'),
        );
        const currentBundleEnd = backendEvents.some((line) =>
            line.includes('pageAsset.bundle.end') &&
            line.includes('role=current') &&
            line.includes('page=2') &&
            line.includes('result=accepted'),
        );
        if (!currentBundleBegin || !currentBundleEnd) {
            throw new Error(`backend page asset events missing:\n${backendEvents.join('\n')}`);
        }
    });

    it('skips stale delayed vector results during rapid navigation', async function () {
        await installDiagnosticCapture();
        await invokeTauriCommand<void>('clear_pdf_event_log');
        await invokeTauriCommand<void>('set_page_asset_test_delay_ms', { delayMs: 0 });
        await pagePresentationHelpers.loadFixturePdf(pagePresentationFixturePath);

        const initial = await readViewerState();
        if (initial.totalPages < 3) {
            this.skip();
            return;
        }

        try {
            await invokeTauriCommand<void>('set_page_asset_test_delay_ms', { delayMs: 250 });
            await browser.executeAsync((done) => {
                const w = window as DiagnosticWindow;
                Promise.allSettled([
                    w.pdfNextPage?.(),
                    w.pdfNextPage?.(),
                ])
                    .then(() => done({ ok: true }))
                    .catch((error) =>
                        done({
                            ok: false,
                            error: error && error.message ? error.message : String(error),
                        }),
                    );
            });

            await browser.waitUntil(
                async () => {
                    const state = await readViewerState();
                    return state.currentPage === 2 && state.totalPages === 4;
                },
                {
                    timeout: 10_000,
                    interval: 250,
                    timeoutMsg: 'delayed rapid navigation did not settle on the latest expected page',
                },
            );

            const settled = await readViewerState();
            const staleSkipped = settled.diagnostics.filter((line) =>
                line.includes('page-turn.stale-render-skipped'),
            );
            if (!staleSkipped.some((line) => line.includes('targetPage=1'))) {
                throw new Error(`delayed stale page turn was not reported:\n${settled.diagnostics.join('\n')}`);
            }
            const visibleReady = settled.diagnostics.filter((line) => line.includes('page-turn.visible-ready'));
            const staleVisibleReady = visibleReady.filter((line) => !line.includes('targetPage=2'));
            if (staleVisibleReady.length > 0) {
                throw new Error(`delayed stale page turns were presented:\n${staleVisibleReady.join('\n')}`);
            }
            if (!visibleReady.some((line) => line.includes('targetPage=2') && line.includes('visiblePage=2'))) {
                throw new Error(`latest delayed page turn was not presented:\n${visibleReady.join('\n')}`);
            }
        } finally {
            await invokeTauriCommand<void>('set_page_asset_test_delay_ms', { delayMs: 0 });
        }
    });

    it('uses page intermediate data for real PDF search and annotation targets', async () => {
        await invokeTauriCommand<void>('set_page_asset_test_delay_ms', { delayMs: 0 });
        await pagePresentationHelpers.loadFixturePdf(pagePresentationFixturePath);
        const path = normalizedFixturePath();

        const searchResult = await invokeTauriCommand<PageSearchResult>('find_in_page', {
            path,
            pageIndex: 1,
            query: 'Page 2',
            caseSensitive: false,
        });

        if (searchResult.pageIndex !== 1 || searchResult.totalMatches < 1) {
            throw new Error(`real PDF search did not find page text: ${JSON.stringify(searchResult)}`);
        }
        if (!searchResult.matches.some((match) => match.sourceText.includes('Page 2'))) {
            throw new Error(`real PDF search returned unexpected matches: ${JSON.stringify(searchResult)}`);
        }

        const targetResult = await invokeTauriCommand<AnnotationTargetResult>('read_annotation_targets', {
            path,
            pageIndex: 1,
        });

        if (targetResult.pageIndex !== 1 || targetResult.targets.length < 1) {
            throw new Error(
                `real PDF annotation targets were not derived: ${JSON.stringify(targetResult)}`,
            );
        }
        if (!targetResult.targets.some((target) => target.label.includes('Page 2'))) {
            throw new Error(
                `real PDF annotation targets did not include page text: ${JSON.stringify(targetResult)}`,
            );
        }
    });

    it('keeps page navigation shortcuts active when add-text mode is enabled but no text is focused', async function () {
        await invokeTauriCommand<void>('set_page_asset_test_delay_ms', { delayMs: 0 });
        await pagePresentationHelpers.loadFixturePdf(pagePresentationFixturePath);

        const initial = await readViewerState();
        if (initial.totalPages < 2) {
            this.skip();
            return;
        }

        try {
            await browser.executeAsync((done) => {
                const w = window as DiagnosticWindow;
                Promise.resolve(w.toggleAddTextMode?.())
                    .then(() => {
                        w.__pageShortcutTestAddTextEnabled = true;
                        done({ ok: true });
                    })
                    .catch((error) =>
                        done({
                            ok: false,
                            error: error && error.message ? error.message : String(error),
                        }),
                    );
            });
            await browser.execute(() => {
                (document.activeElement as HTMLElement | null)?.blur?.();
                window.dispatchEvent(
                    new KeyboardEvent('keydown', {
                        key: 'PageDown',
                        bubbles: true,
                        cancelable: true,
                    }),
                );
            });

            await browser.waitUntil(
                async () => {
                    const state = await readViewerState();
                    return state.currentPage === 1;
                },
                {
                    timeout: 10_000,
                    interval: 250,
                    timeoutMsg: 'PageDown shortcut did not navigate while add-text mode was enabled',
                },
            );
        } finally {
            await browser.executeAsync((done) => {
                const w = window as DiagnosticWindow;
                if (!w.__pageShortcutTestAddTextEnabled) {
                    done({ ok: true });
                    return;
                }
                Promise.resolve(w.toggleAddTextMode?.())
                    .then(() => {
                        w.__pageShortcutTestAddTextEnabled = false;
                        done({ ok: true });
                    })
                    .catch((error) =>
                        done({
                            ok: false,
                            error: error && error.message ? error.message : String(error),
                        }),
                    );
            });
        }
    });
});
