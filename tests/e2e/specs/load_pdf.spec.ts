/**
 * 验证可以从测试侧通过 `window.openPdfFile(absPath)` 直接加载 PDF，
 * 跳过 Tauri 文件对话框（避免阻塞）。
 *
 * 在 wdio v7 + CJS 模式下，spec 文件 不便用 ES `import`，统一用 `require`。
 */
// eslint-disable-next-line @typescript-eslint/no-var-requires
const loadPdfPath = require('node:path') as typeof import('node:path');
// eslint-disable-next-line @typescript-eslint/no-var-requires
const loadPdfHelpers = require('../helpers/app') as typeof import('../helpers/app');

const loadPdfRepoRoot = loadPdfPath.resolve(__dirname, '..', '..', '..');
const loadPdfFixturePath = loadPdfPath.join(loadPdfRepoRoot, 'tests', 'e2e', 'fixtures', 'sample.pdf');

describe('PDF fixture loading', () => {
    before(async () => {
        await loadPdfHelpers.waitForApp();
    });

    it('exposes window.openPdfFile()', async () => {
        const exists = (await browser.execute(
            () => typeof (window as any).openPdfFile === 'function',
        )) as boolean;
        if (!exists) throw new Error('window.openPdfFile is not exposed');
    });

    it('loads the fixture PDF without the file dialog', async () => {
        console.log('[e2e] loading fixture:', loadPdfFixturePath);
        await loadPdfHelpers.loadFixturePdf(loadPdfFixturePath);
        const total = (await browser.execute(
            () => document.getElementById('pdf-total-pages')?.textContent ?? '',
        )) as string;
        console.log('[e2e] totalPages after load =', total);
        if (!total || total === '0') {
            throw new Error(`expected pdf-total-pages > 0, got "${total}"`);
        }
    });
});
