// 共享 helper（CommonJS 纯 JS）—— wdio v7 + Mocha 不替我们对 require() 进来的
// .ts 文件做 ts-node 编译，故用 .js 直接跑。spec 文件可在 TypeScript 侧用
// `require('../helpers/app')` 配合 JSDoc 拿到类型。

/**
 * 等待 Tauri 应用 webview 完成主导航并挂载 DOM。
 * @returns {Promise<void>}
 */
async function waitForApp() {
    await browser.waitUntil(
        async () => {
            const handles = await browser.getWindowHandles();
            for (const h of handles) {
                await browser.switchToWindow(h);
                const href = await browser.execute(() => location.href);
                if (typeof href === 'string' && href !== 'about:blank' && href !== '') {
                    const hasRoot = await browser.execute(
                        () => !!document.querySelector('#pdf-viewer-root'),
                    );
                    if (hasRoot) {
                        await browser.execute(() => {
                            window.__PDF_DIAGNOSTICS_VERBOSE = true;
                            window.__PDF_LAYOUT_TRACE_VERBOSE = true;
                        });
                        return true;
                    }
                }
            }
            return false;
        },
        {
            timeout: 30_000,
            interval: 500,
            timeoutMsg: 'app HTML never loaded (#pdf-viewer-root not found within 30s)',
        },
    );
}

/**
 * 跳过文件对话框直接通过 `window.openPdfFile()` 加载 PDF。
 * @param {string} absPath 绝对路径
 * @returns {Promise<void>}
 */
async function loadFixturePdf(absPath) {
    const normalized = absPath.replace(/\\/g, '/');
    const callResult = await browser.executeAsync(
        function (p, done) {
            const fn = window.openPdfFile;
            if (typeof fn !== 'function') {
                done({ ok: false, error: 'window.openPdfFile not exposed' });
                return;
            }
            Promise.resolve()
                .then(() => fn(p))
                .then(() => done({ ok: true }))
                .catch((err) =>
                    done({ ok: false, error: err && err.message ? err.message : String(err) }),
                );
        },
        normalized,
    );
    if (!callResult || !callResult.ok) {
        throw new Error(
            `openPdfFile rejected: ${(callResult && callResult.error) || '(unknown)'}`,
        );
    }

    try {
        await browser.waitUntil(
            async () => {
                const state = await browser.execute(() => {
                    const img = document.getElementById('pdf-render-target');
                    const total = (document.getElementById('pdf-total-pages') || {}).textContent || '';
                    const hasCanvasRendered = img instanceof HTMLCanvasElement ? img.width > 300 : false;
                    return {
                        hasSrc: !!(img && img.src && img.src.length > 0) || hasCanvasRendered,
                        totalPages: total,
                    };
                });
                return state.hasSrc || (state.totalPages !== '' && state.totalPages !== '0');
            },
            { timeout: 15_000, interval: 500, timeoutMsg: 'PDF never rendered after openPdfFile' },
        );
    } catch (err) {
        try {
            const history = await browser.execute(() => window.__PDF_DIAGNOSTICS_HISTORY || []);
            console.error('\n[E2E-DIAGNOSTICS-HISTORY-START]');
            for (const entry of history) {
                console.error(`  ${entry.timestamp} ${entry.level} [${entry.layer}] ${entry.event}: ${JSON.stringify(entry.fields)}`);
            }
            console.error('[E2E-DIAGNOSTICS-HISTORY-END]\n');
        } catch (diagErr) {
            console.error('Failed to retrieve webview diagnostic history:', diagErr);
        }
        throw err;
    }
}

module.exports = { waitForApp, loadFixturePdf };
