import { emitPdfDiagnostic } from '../shared/diagnostics';

type ReadEditorDiagnostics = () => unknown;

export type EditorHostDiagnostics = {
    logNode: (node: string, details: Record<string, unknown>) => void;
    logRustDiagnostics: (reason: string) => void;
};

export function createEditorHostDiagnostics(
    readEditorDiagnostics: ReadEditorDiagnostics,
): EditorHostDiagnostics {
    let lastTerminalTraceSeq = -1;
    const rustTerminalAllowList = new Map<string, Set<string>>([
        ['activation.client', new Set(['resolved-open-point', 'target-hit', 'target-hit-nearest', 'target-hit-missing', 'target-hit-empty', 'missing-shell-bbox'])],
        ['open.runtime', new Set(['target-built'])],
        ['document-plan.source-runs', new Set(['resolved', 'missing', 'invalid-anchor'])],
        ['document-plan.open-caret', new Set(['resolved'])],
        ['overlay.source-indices', new Set(['persisted'])],
        ['visual.paint', new Set(['style-flags', 'shell-render-plan'])],
        ['paint.overlay', new Set(['active-shell-occlusion', 'render-plan'])],
        ['effective-plan', new Set(['overlay-path-summary', 'overlay-compact', 'overlay-min'])],
        ['canvas.draw', new Set(['vector-path', 'underline-stroke', 'draw-command-line', 'draw-command-fill-rect', 'draw-command-stroke-rect'])],
    ]);

    function enqueueTerminalLog(lines: string[]): void {
        if (!lines.length) return;
        for (const line of lines) {
            emitPdfDiagnostic('editor', 'trace', { line });
        }
    }

    function stringifyTerminalValue(value: unknown): string {
        if (value == null) return 'null';
        if (typeof value === 'string') return value.length > 90 ? `${value.slice(0, 87)}...` : value;
        if (typeof value === 'number' || typeof value === 'boolean') return String(value);
        try {
            return JSON.stringify(value);
        } catch {
            return String(value);
        }
    }

    function formatEditorTextForLog(text: unknown): string {
        if (typeof text !== 'string') return stringifyTerminalValue(text);
        const visible = text.replace(/\n/g, '\\n').replace(/ /g, '·');
        return visible.length > 80 ? `${visible.slice(0, 77)}...` : visible;
    }

    function formatTerminalTraceDetails(details: unknown): string {
        if (!Array.isArray(details)) return '';
        const allowed = new Set([
            'baseParagraphId',
            'paragraphId',
            'targetId',
            'summary',
            'objectId',
            'objectIndex',
            'objectType',
            'owner',
            'entryKind',
            'intersectsShell',
            'overlayCount',
            'commandType',
            'command',
            'storedCaretIndex',
            'effectiveCaretIndex',
            'caretBefore',
            'caretAfter',
            'removeIndex',
            'removedText',
            'beforeText',
            'afterText',
            'requestedText',
            'normalizedCaretIndex',
            'textChanged',
            'sceneChanged',
            'draftText',
            'bodyText',
            'sourceText',
            'fullSourceText',
            'bodySourceText',
            'sourceLen',
            'draftLen',
            'prefixLen',
            'suffixLen',
            'preservedRunCount',
            'measuredRunCount',
            'lostOriginRunCount',
            'lineSummary',
            'runCount',
            'lineRunCount',
            'markerTextOverride',
            'visualLineCount',
            'caretStopCount',
            'targetColor',
            'targetText',
            'targetTextDecoration',
            'sourceColor',
            'sourceTextDecoration',
            'sourceUnderline',
            'sourceUnderlineRunCount',
            'underlineRunCount',
            'liveColor',
            'liveTextDecoration',
            'liveUnderline',
            'targetWidth',
            'targetHeight',
            'bodyObjectIdCount',
            'originalObjectIdCount',
            'bodyObjectIds',
            'originalObjectIds',
            'targetClientLeft',
            'targetClientTop',
            'shellWidth',
            'shellHeight',
            'shellLeft',
            'shellTop',
            'projectionZoom',
            'objectIntersectCount',
            'textIntersectCount',
            'pathIntersectCount',
            'imageIntersectCount',
            'thinHorizontalPathCount',
            'suppressedPathCount',
            'suppressedTextObjectCount',
            'suppressedTextRunCount',
            'sourceObjectIdCount',
            'sourceObjectIndexCount',
            'sourceObjectIndices',
            'resolvedCount',
            'resolvedIndices',
            'firstPathSummary',
            'objectSummary1',
            'objectSummary2',
            'objectSummary3',
            'source',
            'originalText',
            'newText',
            'hasNewRuns',
            'sourceAlignment',
            'activeAlignment',
            'sourceMarker',
            'newMarker',
            'hasMarker',
            'bodyCharStart',
            'fullCaret',
            'bodyCaret',
            'clickPageX',
            'clickPageY',
            'bodyAnchor',
            'shellBBox',
            'bodyBBox',
            'sourceBBox',
            'textClearBBox',
            'pathSuppressionBBox',
            'occlusionBBox',
            'bbox',
            'shellLeft',
            'shellTop',
            'shellRight',
            'shellBottom',
            'bodyLeft',
            'bodyTop',
            'bodyRight',
            'bodyBottom',
            'strokeColor',
            'fillColor',
            'strokeWidth',
            'pathWidth',
            'pathHeight',
            'x1',
            'y1',
            'x2',
            'y2',
            'runText',
            'width',
            'height',
            'occlusionWidth',
            'occlusionHeight',
            'color',
        ]);
        const priority = [
            'summary',
            'paragraphId',
            'owner',
            'replacesSource',
            'suppressedPathCount',
            'thinHorizontalPathCount',
            'pathIntersectCount',
            'imageIntersectCount',
            'suppressedTextObjectCount',
            'suppressedTextRunCount',
            'pathSuppressionBBox',
            'firstPathSummary',
            'objectSummary1',
            'objectSummary2',
            'objectSummary3',
            'caretTop',
            'caretHeight',
            'bodyTopOffset',
        ];
        const priorityIndex = new Map(priority.map((key, index) => [key, index]));
        return details
            .filter((field) => allowed.has(typeof field?.key === 'string' ? field.key : ''))
            .sort((left, right) => {
                const leftKey = typeof left?.key === 'string' ? left.key : '';
                const rightKey = typeof right?.key === 'string' ? right.key : '';
                return (priorityIndex.get(leftKey) ?? 1000) - (priorityIndex.get(rightKey) ?? 1000);
            })
            .slice(0, 22)
            .map((field) => {
                const key = typeof field?.key === 'string' ? field.key : 'unknown';
                const value = key.toLowerCase().includes('text') || key === 'lineSummary'
                    ? formatEditorTextForLog(field?.value)
                    : stringifyTerminalValue(field?.value);
                return `${key}=${value}`;
            })
            .join(' | ');
    }

    function flushRustTrace(diagnostics?: any): void {
        const activeDiagnostics = diagnostics ?? readEditorDiagnostics();
        const trace = Array.isArray(activeDiagnostics?.debugTrace) ? activeDiagnostics.debugTrace : [];
        if (!trace.length) {
            return;
        }
        const maxSeq = trace.reduce((value: number, event: any) => {
            const seq = typeof event?.seq === 'number' ? event.seq : value;
            return Math.max(value, seq);
        }, -1);
        if (maxSeq < lastTerminalTraceSeq) {
            lastTerminalTraceSeq = -1;
        }
        const freshEvents = trace.filter((event: any) => {
            if (typeof event?.seq !== 'number' || event.seq <= lastTerminalTraceSeq) {
                return false;
            }
            const node = typeof event?.node === 'string' ? event.node : '';
            const action = typeof event?.action === 'string' ? event.action : '';
            const allowedActions = rustTerminalAllowList.get(node);
            return !!allowedActions && allowedActions.has(action);
        });
        if (!freshEvents.length) {
            return;
        }
        lastTerminalTraceSeq = freshEvents.reduce(
            (value: number, event: any) => Math.max(value, typeof event?.seq === 'number' ? event.seq : value),
            lastTerminalTraceSeq,
        );
        const lines = freshEvents
            .map((event: any) => {
                const seq = typeof event?.seq === 'number' ? event.seq : '?';
                const node = typeof event?.node === 'string' ? event.node : 'unknown';
                const action = typeof event?.action === 'string' ? event.action : 'unknown';
                const details = formatTerminalTraceDetails(event?.details);
                return `[pdf-editor] rust.${node}.${action} | #${seq}${details ? ` | ${details}` : ''}`;
            });
        enqueueTerminalLog(lines);
    }

    function logRustDiagnostics(reason: string): void {
        try {
            const diagnostics = readEditorDiagnostics();
            if (diagnostics) {
                (window as any).__pdfEditorDiagnostics = diagnostics;
                flushRustTrace(diagnostics);
            }
        } catch (error) {
            emitPdfDiagnostic('editor', 'diagnostics-error', { reason, error }, { verboseOnly: true });
        }
    }

    function logNode(node: string, details: Record<string, unknown>): void {
        const interestingNodes = new Set([
            'ts.mode.set',
            'ts.sync-targets.result',
            'ts.target.pointerdown',
            'ts.root.pointerdown',
            'ts.target-layer.rendered',
            'ts.open.input',
            'ts.open.result',
            'ts.open.root.input',
            'ts.open.root.result',
            'ts.shell.positioned',
            'ts.render-active-editor',
            'ts.blue-scan',
            // === Commit/blur/input diagnostics (added for marker-restore bug) ===
            'ts.commit',
            'ts.blur.commit-requested',
            'ts.blur.commit-suppressed',
            'ts.beforeinput',
            'ts.shell-mousedown.input',
            'ts.shell-mousedown.result',
            'ts.open.focus-stabilized',
            'ts.close',
        ]);
        if (!interestingNodes.has(node)) return;
        const parts = Object.entries(details)
            .flatMap(([key, value]) => {
                if (key === 'result' || key === 'commitResult') {
                    const result = value as Record<string, unknown> | null | undefined;
                    if (!result || typeof result !== 'object') return [];
                    return [
                        `changed=${stringifyTerminalValue(result.changed)}`,
                        `textChanged=${stringifyTerminalValue(result.textChanged)}`,
                        `caretChanged=${stringifyTerminalValue(result.caretChanged)}`,
                        `sceneChanged=${stringifyTerminalValue(result.sceneChanged)}`,
                        `caret=${stringifyTerminalValue(result.caretIndex)}`,
                        `hasFrame=${stringifyTerminalValue(!!result.renderFrame)}`,
                    ];
                }
                if (key.toLowerCase().includes('text') || key === 'textareaValue' || key === 'rustText') {
                    return [`${key}="${formatEditorTextForLog(value)}"`];
                }
                if (['command', 'inputType', 'caretIndex', 'rustCaretIndex', 'snapshotCaretIndex', 'lastRustCaretIndex', 'selectionStart', 'selectionEnd', 'displayZoom', 'committed', 'changed', 'targetCount', 'childCount', 'suppressForSave', 'suppressForOpen', 'sessionDirty', 'activeElementIsTextarea'].includes(key)) {
                    return [`${key}=${stringifyTerminalValue(value)}`];
                }
                if ([
                    'reason',
                    'enabled',
                    'snapshotEnabled',
                    'active',
                    'paragraphId',
                    'targetId',
                    'initialCaretIndex',
                    'bodyCharCount',
                    'slotCount',
                    'lineCount',
                    'targetWidth',
                    'targetHeight',
                    'targetClientLeft',
                    'targetClientTop',
                    'shellWidth',
                    'shellHeight',
                    'shellLeft',
                    'shellTop',
                    'targetLeft',
                    'targetTop',
                    'underline',
                    'targetUnderline',
                    'sourceUnderline',
                    'liveUnderline',
                    'opened',
                    'displayed',
                ].includes(key)) {
                    return [`${key}=${stringifyTerminalValue(value)}`];
                }
                if (['targetColor', 'sourceColor', 'liveColor', 'textDecoration', 'targetTextDecoration', 'sourceTextDecoration', 'liveTextDecoration'].includes(key)) {
                    return [`${key}=${stringifyTerminalValue(value)}`];
                }
                if (key === 'overlays') {
                    return [`overlays=${stringifyTerminalValue(value)}`];
                }
                if (['main', 'detail', 'editor'].includes(key)) {
                    const scan = value as Record<string, unknown> | null | undefined;
                    if (!scan || typeof scan !== 'object') return [`${key}=null`];
                    return [
                        `${key}BlueWidth=${stringifyTerminalValue(scan.runCssWidth)}`,
                        `${key}BlueHit=${stringifyTerminalValue(scan.thresholdHit)}`,
                        `${key}BlueTop=${stringifyTerminalValue(scan.rowCssTop)}`,
                    ];
                }
                return [];
            })
            .filter((part) => !part.includes('=undefined'));
        enqueueTerminalLog([`[pdf-editor] host.${node} | ${parts.join(' | ')}`]);
    }

    return {
        logNode,
        logRustDiagnostics,
    };
}

