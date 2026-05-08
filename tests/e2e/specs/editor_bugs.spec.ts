/**
 * Bug 1：点击空白处应退出编辑模式并保留已输入文字。
 * Bug 2：首次点击文字时光标应直接落在点击位置，而不需要再点一次。
 *
 * 当前现状：
 *   - 框架（tauri-driver + wdio + 真实 webview2）已能驱动 app 并自动加载 PDF。
 *   - 缺一份「带已知坐标可编辑文字段落」的 PDF fixture：
 *       现有 `tests/e2e/fixtures/sample.pdf` 仅 597 字节（占位），编辑流程跑不起来。
 *   - 因此本文件 用 `it.skip` 留骨架，需要后续：
 *       (1) 准备一份在 (x, y, w, h) 已知段落、已知文字内容的 PDF fixture；
 *       (2) 把下面 STUB 部分替换为真实坐标 + 期望文本；
 *       (3) 把 `it.skip` 改回 `it`。
 *
 * 参考已修 commit：
 *   - Plan-1 严格 hit-test：crates/pdf-viewer-ui/src/editor/activation.rs
 *     `strict_hit_test` + miss 时 `commit_pending_edit_if_any`。
 *   - Plan-2 caret 修正：src/bridge/editor/editor_host.ts
 *     `scheduleOpenFocusStabilization` 异步重设 `selectionStart/End`。
 */
// eslint-disable-next-line @typescript-eslint/no-var-requires
const editorBugsPath = require('node:path') as typeof import('node:path');
// eslint-disable-next-line @typescript-eslint/no-var-requires
const editorBugsHelpers = require('../helpers/app') as typeof import('../helpers/app');

const editorBugsRepoRoot = editorBugsPath.resolve(__dirname, '..', '..', '..');
const editorBugsFixturePath = editorBugsPath.join(
    editorBugsRepoRoot,
    'tests',
    'e2e',
    'fixtures',
    'sample.pdf',
);

/**
 * STUB —— 需要真实 fixture 时替换：
 *   `paragraphPoint`：在某段落正中央的 PDF 坐标 → CSS 像素 偏移（相对 viewport）
 *   `blankPoint`：同页内非段落区域的偏移
 *   `caretClickX`：点击距段落起始的水平像素偏移，预期落在 `expectedCaretIndex` 字符上
 */
const PARAGRAPH_POINT = { x: 400, y: 200 };
const BLANK_POINT = { x: 400, y: 700 };
const CARET_CLICK_OFFSET_X = 50;
const EXPECTED_CARET_INDEX_RANGE: [number, number] = [3, 8]; // 容忍 ±2

/**
 * 通过 dispatchEvent 在 #pdf-render-target 上触发 pointerdown / mousedown / click。
 * Tauri webview 不响应 webdriver 的 raw mouse 事件时，DOM 事件是更可靠的回退。
 */
async function clickAtViewportPoint(x: number, y: number): Promise<void> {
    await browser.execute(
        (cx: number, cy: number) => {
            const target = document.elementFromPoint(cx, cy) as HTMLElement | null;
            if (!target) return;
            const init = { bubbles: true, cancelable: true, clientX: cx, clientY: cy, button: 0 };
            target.dispatchEvent(new PointerEvent('pointerdown', { ...init, pointerType: 'mouse' }));
            target.dispatchEvent(new MouseEvent('mousedown', init));
            target.dispatchEvent(new MouseEvent('mouseup', init));
            target.dispatchEvent(new MouseEvent('click', init));
        },
        x,
        y,
    );
}

async function readActiveTextarea(): Promise<{ value: string; selectionStart: number } | null> {
    return (await browser.execute(() => {
        const ta = document.querySelector('textarea[data-pdf-editor-textarea]') as HTMLTextAreaElement | null
            ?? document.querySelector('.pdf-editor-textarea') as HTMLTextAreaElement | null
            ?? document.querySelector('textarea:focus') as HTMLTextAreaElement | null;
        if (!ta) return null;
        return { value: ta.value, selectionStart: ta.selectionStart };
    })) as { value: string; selectionStart: number } | null;
}

describe('Editor interaction bugs', () => {
    before(async () => {
        await editorBugsHelpers.waitForApp();
        await editorBugsHelpers.loadFixturePdf(editorBugsFixturePath);
    });

    // 把 .skip 去掉，并保证 fixture 含有 PARAGRAPH_POINT 处的可编辑段落。
    it.skip('Bug 1: clicking blank area exits editor and preserves edits', async () => {
        await clickAtViewportPoint(PARAGRAPH_POINT.x, PARAGRAPH_POINT.y);
        await browser.pause(400);

        // 输入测试文本
        await browser.execute(() => {
            const ta = document.querySelector('textarea[data-pdf-editor-textarea]') as HTMLTextAreaElement | null;
            if (!ta) throw new Error('editor textarea not focused after click');
            ta.value = `${ta.value}__E2E_INSERT__`;
            ta.dispatchEvent(new Event('input', { bubbles: true }));
        });

        // 点空白
        await clickAtViewportPoint(BLANK_POINT.x, BLANK_POINT.y);
        await browser.pause(400);

        // 编辑器应已退出
        const stillOpen = (await browser.execute(
            () => !!document.querySelector('textarea[data-pdf-editor-textarea]'),
        )) as boolean;
        if (stillOpen) throw new Error('editor still open after blank click — Bug 1 regressed');

        // 已输入的文字应仍在文档内（通过查询 wasm-side 渲染或重新点击该段落看 textarea 初始值）
        await clickAtViewportPoint(PARAGRAPH_POINT.x, PARAGRAPH_POINT.y);
        await browser.pause(400);
        const ta = await readActiveTextarea();
        if (!ta) throw new Error('cannot reopen paragraph editor for verification');
        if (!ta.value.includes('__E2E_INSERT__')) {
            throw new Error(`edits not preserved across blank-click exit: textarea value="${ta.value}"`);
        }
    });

    it.skip('Bug 2: caret on first click lands at click position', async () => {
        const targetX = PARAGRAPH_POINT.x + CARET_CLICK_OFFSET_X;
        await clickAtViewportPoint(targetX, PARAGRAPH_POINT.y);

        // 等异步 caret 矫正稳定
        await browser.waitUntil(
            async () => {
                const ta = await readActiveTextarea();
                if (!ta) return false;
                const [lo, hi] = EXPECTED_CARET_INDEX_RANGE;
                return ta.selectionStart >= lo && ta.selectionStart <= hi;
            },
            {
                timeout: 5000,
                interval: 100,
                timeoutMsg: `caret never settled in expected range ${EXPECTED_CARET_INDEX_RANGE}`,
            },
        );
    });
});
