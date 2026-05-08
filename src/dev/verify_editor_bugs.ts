/**
 * 应用内自验证：Bug 1（点空白退出且保留修改）+ Bug 2（首次点击 caret 落点正确）。
 *
 * 用法：
 *   1. 启动应用，打开任意带文字的 PDF（File → Open）。
 *   2. 点击工具栏的「Add Text」按钮进入编辑模式。
 *   3. 打开 DevTools Console（Tauri 默认未开 devtools，可改 build 模式或在 src-tauri
 *      的 main.rs 加 webview.open_devtools()）。在控制台运行：
 *         await window.verifyEditorBugs()
 *   4. 控制台输出 [VERIFY-PASS] 或 [VERIFY-FAIL]，并打印详情。
 *
 * 不依赖任何 webdriver / playwright / wdio。
 */

const TEXTAREA_ID = 'pdf-editor-textarea-vector';
const INSERT_MARKER = '__VERIFY_E2E__';

// 简单 sleep。
function sleep(ms: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms));
}

// 在某个 (clientX, clientY) 上 dispatch 一系列鼠标事件，模拟用户点击。
// 直接 dispatch 可以绕过 Tauri webview 的 IPC，命中我们自己绑定的 pointerdown listener。
function dispatchClickAtViewportPoint(x: number, y: number): void {
    const target = document.elementFromPoint(x, y) as HTMLElement | null;
    if (!target) {
        console.warn('[verify] no element at', x, y);
        return;
    }
    const init: MouseEventInit = {
        bubbles: true,
        cancelable: true,
        clientX: x,
        clientY: y,
        button: 0,
        view: window,
    };
    target.dispatchEvent(new PointerEvent('pointerdown', { ...init, pointerType: 'mouse' }));
    target.dispatchEvent(new MouseEvent('mousedown', init));
    target.dispatchEvent(new MouseEvent('mouseup', init));
    target.dispatchEvent(new MouseEvent('click', init));
}

function getActiveTextarea(): HTMLTextAreaElement | null {
    return document.getElementById(TEXTAREA_ID) as HTMLTextAreaElement | null;
}

function isEditorOpen(): boolean {
    const ta = getActiveTextarea();
    if (!ta) return false;
    // 编辑器未激活时，shell 通常 display:none 或 visibility:hidden。
    const shell = ta.closest<HTMLElement>('[data-pdf-editor-shell]') ?? ta.parentElement;
    if (!shell) return true;
    const style = window.getComputedStyle(shell);
    return style.display !== 'none' && style.visibility !== 'hidden';
}

interface ParagraphBox {
    el: HTMLElement;
    rect: DOMRect;
    paragraphId: string;
}

function listParagraphBoxes(): ParagraphBox[] {
    const els = Array.from(document.querySelectorAll<HTMLElement>('[data-paragraph-id]'));
    return els
        .map((el) => ({ el, rect: el.getBoundingClientRect(), paragraphId: el.dataset.paragraphId ?? '' }))
        // 必须可见 + 有面积 + 不在屏幕外。
        .filter((b) => b.rect.width > 20 && b.rect.height > 8 && b.rect.right > 0 && b.rect.bottom > 0)
        .sort((a, b) => a.rect.top - b.rect.top);
}

function findBlankPoint(paragraphBoxes: ParagraphBox[]): { x: number; y: number } | null {
    // 在所有段落 bbox 之外、视口范围内取一个点。简单策略：取最后一个段落下方 80px。
    const last = paragraphBoxes[paragraphBoxes.length - 1];
    if (!last) return null;
    const x = last.rect.left + 30;
    const y = last.rect.bottom + 80;
    if (y > window.innerHeight - 20) return null;
    // 验证那个点不在任何段落 bbox 里。
    const hit = document.elementFromPoint(x, y);
    if (hit && (hit as HTMLElement).closest('[data-paragraph-id]')) return null;
    return { x, y };
}

interface VerifyResult {
    name: string;
    pass: boolean;
    detail: Record<string, unknown>;
}

async function verifyBug1BlankClickExitsAndPersists(
    target: ParagraphBox,
    blank: { x: number; y: number },
): Promise<VerifyResult> {
    const clickPoint = {
        x: target.rect.left + target.rect.width / 2,
        y: target.rect.top + target.rect.height / 2,
    };
    const detail: Record<string, unknown> = {
        paragraphId: target.paragraphId,
        clickPoint,
        blankPoint: blank,
    };

    // 1. 点击段落 → 编辑器应打开。
    dispatchClickAtViewportPoint(clickPoint.x, clickPoint.y);
    await sleep(300);
    const ta1 = getActiveTextarea();
    if (!ta1 || !isEditorOpen()) {
        return { name: 'Bug-1', pass: false, detail: { ...detail, step: 'open editor', tafound: !!ta1 } };
    }
    detail.initialValue = ta1.value;

    // 2. 在 textarea 中插入测试文本（通过原生 setter + input 事件触发上层逻辑）。
    const proto = Object.getPrototypeOf(ta1);
    const setter = Object.getOwnPropertyDescriptor(proto, 'value')?.set;
    setter?.call(ta1, ta1.value + INSERT_MARKER);
    ta1.dispatchEvent(new Event('input', { bubbles: true }));
    await sleep(200);
    detail.afterInsertValue = ta1.value;

    // 3. 点击空白 → 编辑器应关闭。
    dispatchClickAtViewportPoint(blank.x, blank.y);
    await sleep(500);
    const stillOpen = isEditorOpen();
    detail.editorOpenAfterBlankClick = stillOpen;
    if (stillOpen) {
        return { name: 'Bug-1', pass: false, detail: { ...detail, step: 'editor did not close on blank click' } };
    }

    // 4. 重新点击段落 → 验证插入文字是否仍在 textarea 初值里。
    dispatchClickAtViewportPoint(clickPoint.x, clickPoint.y);
    await sleep(400);
    const ta2 = getActiveTextarea();
    detail.reopenedValue = ta2?.value;
    if (!ta2 || !isEditorOpen()) {
        return { name: 'Bug-1', pass: false, detail: { ...detail, step: 'reopen failed' } };
    }
    if (!ta2.value.includes(INSERT_MARKER)) {
        return { name: 'Bug-1', pass: false, detail: { ...detail, step: 'edits lost after blank click' } };
    }
    return { name: 'Bug-1', pass: true, detail };
}

async function verifyBug2CaretLandsAtClickPosition(target: ParagraphBox): Promise<VerifyResult> {
    // 先清场：如果当前编辑器打开，点空白关掉。
    if (isEditorOpen()) {
        const blank = findBlankPoint(listParagraphBoxes());
        if (blank) dispatchClickAtViewportPoint(blank.x, blank.y);
        await sleep(400);
    }

    // 在段落文字水平中点（约 30%） 处点击 —— 期望 caret 不在 0 也不在末尾。
    const clickX = target.rect.left + target.rect.width * 0.3;
    const clickY = target.rect.top + target.rect.height / 2;

    dispatchClickAtViewportPoint(clickX, clickY);
    // 等待 scheduleOpenFocusStabilization 的异步 caret 矫正（rAF + 120ms）。
    await sleep(400);

    const ta = getActiveTextarea();
    if (!ta) {
        return { name: 'Bug-2', pass: false, detail: { step: 'textarea not focused', clickX, clickY } };
    }
    const valueLen = ta.value.length;
    const caret = ta.selectionStart ?? -1;
    const detail = {
        paragraphId: target.paragraphId,
        clickX,
        clickY,
        valueLen,
        caret,
        ratio: valueLen > 0 ? caret / valueLen : null,
    };

    // 如果文字非空，点 30% 处的 caret 应大约在 [10%, 60%] 区间。容忍范围考虑首字符宽度差异。
    if (valueLen === 0) {
        return { name: 'Bug-2', pass: true, detail: { ...detail, note: 'paragraph empty, skipped ratio check' } };
    }
    const ratio = caret / valueLen;
    const pass = ratio > 0.05 && ratio < 0.7;
    return { name: 'Bug-2', pass, detail };
}

function diagDom(): Record<string, unknown> {
    const ids = [
        'pdf-page-container',
        'pdf-interaction-layer',
        'pdf-interaction-root-vector',
        'pdf-editor-target-layer-vector',
        'pdf-editor-shell-vector',
        'pdf-editor-textarea-vector',
        'pdf-vector-main-canvas',
    ];
    const out: Record<string, unknown> = {};
    for (const id of ids) {
        const el = document.getElementById(id);
        out[id] = el ? { tag: el.tagName, childCount: el.childElementCount } : 'MISSING';
    }
    return out;
}

async function ensureCleanEditorMode(): Promise<void> {
    // 1. 如有正在激活的段落编辑器，按 Esc 退出，使交互层重新显示。
    const activeTa = getActiveTextarea();
    if (activeTa && document.activeElement === activeTa) {
        console.log('[verify] active textarea found, sending Esc');
        activeTa.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }));
        await sleep(500);
    }

    const toggle = (window as any).toggleAddTextMode as (() => Promise<void> | void) | undefined;
    if (typeof toggle !== 'function') {
        throw new Error('window.toggleAddTextMode 未挂载');
    }

    // 2. 通过观测 DOM 推断当前是否处于编辑模式：
    //    - target layer display === 'block' OR 有 [data-paragraph-id] children → 开
    //    - 都没有 → 关
    function isEditModeOn(): boolean {
        if (listParagraphBoxes().length > 0) return true;
        const layer = document.querySelector<HTMLElement>('.pdf-interaction-target-layer');
        if (layer && getComputedStyle(layer).display === 'block') return true;
        return false;
    }

    if (!isEditModeOn()) {
        console.log('[verify] turning edit mode ON');
        await toggle();
        await sleep(800);
    } else {
        console.log('[verify] edit mode appears already on, forcing refresh: off→on');
        await toggle();
        await sleep(400);
        await toggle();
        await sleep(800);
    }
}

export async function verifyEditorBugs(): Promise<{ pass: boolean; results: VerifyResult[] }> {
    console.log('[verify] starting…');
    try {
        await ensureCleanEditorMode();
    } catch (err) {
        console.error('[VERIFY-FAIL]', 'ensureCleanEditorMode failed:', err);
        return { pass: false, results: [{ name: 'precondition', pass: false, detail: { err: String(err) } }] };
    }
    const boxes = listParagraphBoxes();
    if (boxes.length === 0) {
        const dom = diagDom();
        const layer = document.querySelector<HTMLElement>('.pdf-interaction-target-layer');
        const detail = {
            msg: 'no [data-paragraph-id] elements found',
            domState: dom,
            layerDisplay: layer ? getComputedStyle(layer).display : null,
            layerChildren: layer?.childElementCount ?? null,
        };
        console.error('[VERIFY-FAIL]', detail);
        return { pass: false, results: [{ name: 'precondition', pass: false, detail }] };
    }

    // 取第一个段落作为主目标；找一个空白点。
    const primary = boxes[0];
    const blank = findBlankPoint(boxes);
    if (!blank) {
        const msg = 'cannot find a non-paragraph blank point inside viewport — 请滚动让最后段落上方有空白。';
        console.error('[VERIFY-FAIL]', msg);
        return { pass: false, results: [{ name: 'precondition', pass: false, detail: { msg } }] };
    }

    console.log('[verify] primary paragraph =', primary.paragraphId, primary.rect);
    console.log('[verify] blank point =', blank);

    const results: VerifyResult[] = [];
    results.push(await verifyBug1BlankClickExitsAndPersists(primary, blank));
    results.push(await verifyBug2CaretLandsAtClickPosition(primary));

    const allPass = results.every((r) => r.pass);
    if (allPass) {
        console.log('%c[VERIFY-PASS]', 'color:green;font-weight:bold', results);
    } else {
        console.error('[VERIFY-FAIL]', results);
    }
    return { pass: allPass, results };
}

// 自动挂到 window 供控制台直接调用。
(window as any).verifyEditorBugs = verifyEditorBugs;
console.log('[verify] window.verifyEditorBugs() ready');
