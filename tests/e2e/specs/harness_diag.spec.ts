/** Minimal harness diagnostic: dump every window handle's page state. */
// eslint-disable-next-line @typescript-eslint/no-var-requires
const diagHelpers = require('../helpers/app') as typeof import('../helpers/app');

describe('Harness diagnostic', () => {
    it('dumps window handles and page state', async () => {
        await browser.waitUntil(async () => (await browser.getWindowHandles()).length > 0, {
            timeout: 15000,
            interval: 500,
            timeoutMsg: 'no window handles at all',
        });
        const handles = await browser.getWindowHandles();
        for (const h of handles) {
            await browser.switchToWindow(h);
            const state = await browser.execute(() => ({
                href: location.href,
                title: document.title,
                root: !!document.getElementById('pdf-viewer-root'),
                bodySnippet: document.body ? document.body.innerHTML.slice(0, 300) : '(no body)',
                openPdf: typeof (window as any).openPdfFile,
                wasmv3: typeof (window as any).wasmv3,
            }));
            console.log('[diag] handle', h, JSON.stringify(state, null, 2));
        }
        // Give the app extra time in case mounting is just slow, then re-check.
        await browser.pause(10000);
        for (const h of await browser.getWindowHandles()) {
            await browser.switchToWindow(h);
            const root = await browser.execute(() => ({
                href: location.href,
                root: !!document.getElementById('pdf-viewer-root'),
                bootErr: (window as any).__TAURI_INTERNALS__ ? 'internals-present' : 'no-internals',
            }));
            console.log('[diag+10s]', JSON.stringify(root));
        }
        void diagHelpers;
    });
});
