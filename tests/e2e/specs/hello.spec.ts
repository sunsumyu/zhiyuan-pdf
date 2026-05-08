/**
 * Sanity check：tauri-driver + msedgedriver + Tauri v2 binary 全链路。
 */
// eslint-disable-next-line @typescript-eslint/no-var-requires
const helloAppHelpers = require('../helpers/app') as typeof import('../helpers/app');

describe('tauri-driver smoke test', () => {
    before(async () => {
        await helloAppHelpers.waitForApp();
    });

    it('connects to the webview and runs JS', async () => {
        const info = (await browser.execute(() => ({
            href: location.href,
            hasBody: !!document.body,
            bodyTag: document.body?.tagName,
            title: document.title,
            ua: navigator.userAgent,
        }))) as { href: string; hasBody: boolean; bodyTag: string; title: string; ua: string };
        console.log('[e2e] webview info =', JSON.stringify(info));
        if (!info.hasBody) throw new Error('document.body missing');
        if (info.href === 'about:blank') throw new Error('still on about:blank after waitForApp');
    });

    it('finds the app root container', async () => {
        const found = (await browser.execute(() => {
            const candidates = ['#pdf-viewer-root', '#pdf-scroll-container', '#open-btn'];
            for (const sel of candidates) {
                if (document.querySelector(sel)) return sel;
            }
            return null;
        })) as string | null;
        console.log('[e2e] matched root selector =', JSON.stringify(found));
        if (!found) throw new Error('no known app root selector matched');
    });
});
