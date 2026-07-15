/**
 * Marker-tail regression: deleting body text must not move the list marker into
 * the body, and after commit the marker must still be reported separately.
 *
 * 这条用例依赖一个带 list item（ZapfDingbats bullet + Helvetica body）的 PDF：
 *   tests/e2e/fixtures/list_item.pdf
 *
 * 断言只走 Rust 通过 snapshot 暴露的运行时状态，不读像素：
 *   - markerText 非空且与 body 分离
 *   - 删除一个 body 字符后 markerText 仍在
 *   - commit 后重新打开 markerText 仍在且 body 不以 marker 开头
 */
// eslint-disable-next-line @typescript-eslint/no-var-requires
const markerPath = require('node:path') as typeof import('node:path');
// eslint-disable-next-line @typescript-eslint/no-var-requires
const markerHelpers = require('../helpers/app') as typeof import('../helpers/app');

const markerRepoRoot = markerPath.resolve(__dirname, '..', '..', '..');
const markerFixturePath = markerPath.join(
    markerRepoRoot,
    'tests',
    'e2e',
    'fixtures',
    'list_item.pdf',
);

interface MarkerSnapshot {
    enabled: boolean;
    activeTarget: {
        paragraphId: string;
        text: string;
        markerText?: string | null;
        markerKind?: string | null;
    } | null;
    draftText: string | null;
    caretIndex: number;
}

async function readSnapshot(): Promise<MarkerSnapshot | null> {
    return (await browser.execute(() => (window as any).__readEditorSnapshot?.() ?? null)) as
        | MarkerSnapshot
        | null;
}

function paragraphBoxes(): { rect: DOMRect; paragraphId: string }[] {
    return (browser.execute(() => {
        return Array.from(
            document.querySelectorAll<HTMLElement>('[data-paragraph-id]'),
        ).map((el) => ({
            rect: el.getBoundingClientRect().toJSON(),
            paragraphId: el.dataset.paragraphId ?? '',
        }));
    }) as any) as { rect: DOMRect; paragraphId: string }[];
}

function clickAt(x: number, y: number): void {
    browser.execute(
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

async function ensureEditMode(): Promise<void> {
    const on = (await browser.execute(
        () => !!(window as any).__readEditorSnapshot?.()?.enabled,
    )) as boolean;
    if (on) return;
    await browser.execute(() => (window as any).toggleAddTextMode?.());
    await browser.pause(1000);
}

describe('List marker stays separate from body on delete', () => {
    before(async () => {
        await markerHelpers.waitForApp();
        await markerHelpers.loadFixturePdf(markerFixturePath);
        await ensureEditMode();
    });

    it('marker is reported separately and survives a body delete + commit', async () => {
        const boxes = await paragraphBoxes();
        if (!boxes.length) throw new Error('no paragraph target boxes found after edit mode on');

        const target = boxes[0];
        const cx = target.rect.left + target.rect.width / 2;
        const cy = target.rect.top + target.rect.height / 2;

        // Find the paragraph box element directly and dispatch events on it.
        // This avoids CSS.escape issues and ensures events reach the handler.
        const openEditor = async () => {
            await browser.execute((pid: string) => {
                const boxes = Array.from(document.querySelectorAll<HTMLElement>('[data-paragraph-id]'));
                const box = boxes.find((b) => b.dataset.paragraphId === pid);
                if (!box) return;
                const rect = box.getBoundingClientRect();
                const init = {
                    bubbles: true,
                    cancelable: true,
                    clientX: rect.left + rect.width / 2,
                    clientY: rect.top + rect.height / 2,
                    button: 0,
                };
                box.dispatchEvent(new PointerEvent('pointerdown', { ...init, pointerType: 'mouse' }));
                box.dispatchEvent(new MouseEvent('mousedown', init));
                box.dispatchEvent(new MouseEvent('mouseup', init));
                box.dispatchEvent(new MouseEvent('click', init));
            }, target.paragraphId);
        };

        await openEditor();
        await browser.pause(800);
        await browser.waitUntil(
            async () => !!(await readSnapshot())?.activeTarget,
            { timeout: 10000, interval: 250, timeoutMsg: 'editor did not open on click' },
        );

        const snapBefore = await readSnapshot();
        if (!snapBefore?.activeTarget) throw new Error('editor did not open on click');
        const markerText = snapBefore.activeTarget.markerText ?? '';
        const bodyText = snapBefore.draftText ?? snapBefore.activeTarget.text;
        if (!markerText) {
            throw new Error(
                `expected marker text before delete, got empty (kind=${snapBefore.activeTarget.markerKind})`,
            );
        }
        if (bodyText.startsWith(markerText)) {
            throw new Error(
                `body must not start with marker before delete: body="${bodyText}" marker="${markerText}"`,
            );
        }

        // Delete the last body char via backspace command.
        const beforeLen = [...bodyText].length;
        if (beforeLen === 0) throw new Error('body text empty, cannot delete');

        const caretUtf16 = (await browser.execute(() => {
            const ta = document.getElementById('pdf-editor-textarea-vector') as HTMLTextAreaElement | null;
            return ta ? ta.value.length : -1;
        })) as number;

        await browser.execute((utf16) => {
            const ta = document.getElementById('pdf-editor-textarea-vector') as HTMLTextAreaElement | null;
            if (!ta) return;
            ta.focus();
            ta.setSelectionRange(utf16, utf16);
            ta.dispatchEvent(
                new KeyboardEvent('keydown', {
                    key: 'Backspace',
                    code: 'Backspace',
                    keyCode: 8,
                    which: 8,
                    bubbles: true,
                    cancelable: true,
                }),
            );
        }, caretUtf16);
        await browser.pause(400);

        const snapAfterDelete = await readSnapshot();
        const markerAfterDelete = snapAfterDelete?.activeTarget?.markerText ?? '';
        if (!markerAfterDelete) {
            throw new Error('marker disappeared after delete');
        }
        if (markerAfterDelete !== markerText) {
            throw new Error(
                `marker text changed after delete: before="${markerText}" after="${markerAfterDelete}"`,
            );
        }
        const bodyAfterDelete = snapAfterDelete?.draftText ?? '';
        if (bodyAfterDelete.startsWith(markerAfterDelete)) {
            throw new Error(
                `marker leaked into body after delete: body="${bodyAfterDelete}" marker="${markerAfterDelete}"`,
            );
        }
        const afterLen = [...bodyAfterDelete].length;
        if (afterLen !== beforeLen - 1) {
            throw new Error(
                `delete did not remove exactly one char: before=${beforeLen} after=${afterLen}`,
            );
        }

        // Commit (exit editor) and reopen the same paragraph, marker must persist.
        await browser.execute(() => (window as any).toggleAddTextMode?.());
        await browser.pause(800);
        await ensureEditMode();
        await browser.pause(500);

        await openEditor();
        await browser.waitUntil(
            async () => !!(await readSnapshot())?.activeTarget,
            { timeout: 5000, interval: 150, timeoutMsg: 'editor did not reopen' },
        );

        const snapReopen = await readSnapshot();
        const markerReopen = snapReopen?.activeTarget?.markerText ?? '';
        if (!markerReopen) {
            throw new Error('marker lost after commit+reopen');
        }
        const bodyReopen = snapReopen?.draftText ?? snapReopen?.activeTarget?.text ?? '';
        if (bodyReopen.startsWith(markerReopen)) {
            throw new Error(
                `marker leaked into body after reopen: body="${bodyReopen}" marker="${markerReopen}"`,
            );
        }
    }).timeout(120000);
});
