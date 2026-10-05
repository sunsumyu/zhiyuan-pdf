import { targetInvokeV3 } from './wasm_loader';

type DiagnosticFields = Record<string, unknown>;

type DiagnosticOptions = {
    verboseOnly?: boolean;
    level?: DiagnosticLevel;
    layer?: string;
};

const MAX_FIELD_TEXT = 120;
type DiagnosticLevel = 'TRACE' | 'DEBUG' | 'INFO' | 'WARN' | 'ERROR';

const ANSI_RESET = '\x1b[0m';
const ANSI_DIM = '\x1b[2m';
const ANSI_LEVEL: Record<DiagnosticLevel, string> = {
    TRACE: '\x1b[90m',
    DEBUG: '\x1b[36m',
    INFO: '\x1b[32m',
    WARN: '\x1b[33m',
    ERROR: '\x1b[31m',
};
const ANSI_LAYER = '\x1b[95m';

const CONSOLE_LEVEL_STYLE: Record<DiagnosticLevel, string> = {
    TRACE: 'color:#8b949e',
    DEBUG: 'color:#38bdf8;font-weight:600',
    INFO: 'color:#22c55e;font-weight:600',
    WARN: 'color:#eab308;font-weight:700',
    ERROR: 'color:#ef4444;font-weight:700',
};

function diagnosticsEnabled(): boolean {
    return (window as any).__PDF_DIAGNOSTICS_DISABLED !== true;
}

export function verbosePdfDiagnosticsEnabled(): boolean {
    return (window as any).__PDF_DIAGNOSTICS_VERBOSE === true;
}

function compactString(key: string, value: string): string {
    const normalized = key.toLowerCase().includes('path')
        ? value.split(/[\\/]/).pop() ?? value
        : value.replace(/\s+/g, ' ').trim();
    return normalized.length > MAX_FIELD_TEXT
        ? `${normalized.slice(0, MAX_FIELD_TEXT - 3)}...`
        : normalized;
}

function compactValue(key: string, value: unknown, depth = 0): string {
    if (value == null) return 'null';
    if (typeof value === 'number') {
        return Number.isFinite(value) ? String(Math.round(value * 1000) / 1000) : String(value);
    }
    if (typeof value === 'boolean') return String(value);
    if (typeof value === 'string') return compactString(key, value);
    if (Array.isArray(value)) {
        if (depth > 0) return `[${value.length}]`;
        return `[${value.slice(0, 6).map((item, index) => compactValue(`${key}${index}`, item, depth + 1)).join(',')}${value.length > 6 ? ',...' : ''}]`;
    }
    if (typeof value !== 'object') return compactString(key, String(value));

    const objectValue = value as Record<string, unknown>;
    const preferredKeys = [
        'frameToken',
        'renderReason',
        'displayZoom',
        'renderZoom',
        'baseRenderZoom',
        'cssScale',
        'accepted',
        'page',
        'pageIndex',
        'zoom',
        'width',
        'height',
        'hostWidth',
        'hostHeight',
        'scrollLeft',
        'scrollTop',
        'revision',
        'saved',
        'hadPersistablePatches',
        'errorMessage',
    ];
    const entries = Object.entries(objectValue)
        .filter(([field]) => depth === 0 ? preferredKeys.includes(field) : true)
        .slice(0, depth === 0 ? 10 : 6);
    if (!entries.length) return '{...}';
    return `{${entries.map(([field, fieldValue]) => `${field}:${compactValue(field, fieldValue, depth + 1)}`).join(',')}}`;
}

function nowStamp(): string {
    const now = new Date();
    const h = String(now.getHours()).padStart(2, '0');
    const m = String(now.getMinutes()).padStart(2, '0');
    const s = String(now.getSeconds()).padStart(2, '0');
    const ms = String(now.getMilliseconds()).padStart(3, '0');
    return `${h}:${m}:${s}.${ms}`;
}

function normalizeLayer(channel: string, override?: string): string {
    if (override) return override.toUpperCase().slice(0, 8);
    const normalized = channel.toLowerCase();
    if (normalized === 'prof') return 'PERF';
    if (normalized === 'cache') return 'CACHE';
    if (normalized === 'layout') return 'LAYOUT';
    if (normalized === 'render-flow' || normalized === 'render-chain') return 'RENDER';
    if (normalized === 'render-bundle') return 'ASSET';
    if (normalized === 'present') return 'PRESENT';
    if (normalized === 'canvas-pool') return 'CANVAS';
    if (normalized === 'edit-api') return 'EDIT';
    if (normalized === 'geometry-probe') return 'GEOMETRY';
    return channel.toUpperCase().replace(/[^A-Z0-9]/g, '').slice(0, 8) || 'PDF';
}

function inferLevel(channel: string, event: string, options: DiagnosticOptions = {}): DiagnosticLevel {
    if (options.level) return options.level;
    const text = `${channel}.${event}`.toLowerCase();
    if (text.includes('error') || text.includes('failed') || text.includes('decode-failed')) return 'ERROR';
    if (text.includes('warn') || text.includes('rejected') || text.includes('aborted')) return 'WARN';
    if (options.verboseOnly) return 'DEBUG';
    if (channel.toLowerCase() === 'cache') return 'DEBUG';
    return 'INFO';
}

function formatFields(fields: DiagnosticFields): string {
    const fieldText = Object.entries(fields)
        .filter(([, value]) => value !== undefined)
        .map(([key, value]) => `${key}=${compactValue(key, value)}`)
        .join(' | ');
    return fieldText;
}

function formatLayeredDiagnostic(
    channel: string,
    event: string,
    fields: DiagnosticFields,
    options: DiagnosticOptions = {},
    ansi = false,
): string {
    const timestamp = nowStamp();
    const level = inferLevel(channel, event, options);
    const layer = normalizeLayer(channel, options.layer);
    const fieldText = formatFields(fields);
    const plain = `${timestamp} ${level.padEnd(5)} [${layer.padEnd(8)}] ${event}${fieldText ? ` ${fieldText}` : ''}`;
    if (!ansi) return plain;
    return `${ANSI_DIM}${timestamp}${ANSI_RESET} ${ANSI_LEVEL[level]}${level.padEnd(5)}${ANSI_RESET} ${ANSI_LAYER}[${layer.padEnd(8)}]${ANSI_RESET} ${event}${fieldText ? ` ${fieldText}` : ''}`;
}

export function formatPdfDiagnostic(channel: string, event: string, fields: DiagnosticFields = {}): string {
    return formatLayeredDiagnostic(channel, event, fields);
}

// ── Console-sink rate limit (postmortem 2026-10-05; ADR-0015 前置条件) ──────
// Verbose tracing emits ~260 diagnostics per 100ms during a zoom gesture, and
// the diagnostic flood (console SINK + per-event double formatting +
// allocation) blocks the main thread in 100ms long tasks — the "pipeline
// longtask train" ADR-0026 recorded as its next fix target was this flood,
// not the render pipeline (production, verbose off: zero long tasks on the
// same gesture, 5/5 runs). ADR-0015 already proved that batching the IPC on
// an unbounded-rate channel wedges the page; its retry precondition is
// producer-side rate limiting, applied here to the human-facing console sink
// only:
//   - __PDF_DIAGNOSTICS_HISTORY stays complete (authoritative probe source);
//   - ERROR/WARN never sampled (ADR-0013: the error stream is reserved for
//     real faults and must always surface);
//   - the terminal_log IPC branch is untouched (ADR-0015 reverted batching
//     there — do not route this limiter into it).
// The flood's full composition is NOT fully attributed (postmortem) — this
// limiter bounds the sink; it is not claimed to eliminate the verbose-mode
// train by itself.
export const CONSOLE_SINK_WINDOW_MS = 32;
export const CONSOLE_SINK_MAX_PER_WINDOW = 8;

let consoleSinkWindowStart = 0;
let consoleSinkUsed = 0;
let consoleSinkSuppressed = 0;
let consoleSinkEmitted = 0;

/** Test hook: reset the limiter window and counters. */
export function resetConsoleSinkLimiter(): void {
    consoleSinkWindowStart = 0;
    consoleSinkUsed = 0;
    consoleSinkSuppressed = 0;
    consoleSinkEmitted = 0;
}

function consoleSinkAllows(level: DiagnosticLevel): boolean {
    if (level === 'ERROR' || level === 'WARN') {
        consoleSinkEmitted++;
        return true;
    }
    const now = Date.now();
    if (now - consoleSinkWindowStart >= CONSOLE_SINK_WINDOW_MS) {
        const dropped = consoleSinkSuppressed;
        consoleSinkWindowStart = now;
        consoleSinkUsed = 0;
        consoleSinkSuppressed = 0;
        if (dropped > 0) {
            console.log(
                `[pdf-diagnostics] console sink rate limit: ${dropped} diagnostic(s) suppressed in the last window (in-page history complete)`,
            );
            consoleSinkEmitted++;
        }
    }
    if (consoleSinkUsed >= CONSOLE_SINK_MAX_PER_WINDOW) {
        consoleSinkSuppressed++;
        return false;
    }
    consoleSinkUsed++;
    consoleSinkEmitted++;
    return true;
}

export function emitPdfDiagnostic(
    channel: string,
    event: string,
    fields: DiagnosticFields = {},
    options: DiagnosticOptions = {},
): void {
    if (!diagnosticsEnabled()) return;
    if (options.verboseOnly && !verbosePdfDiagnosticsEnabled()) return;
    const level = inferLevel(channel, event, options);
    const layer = normalizeLayer(channel, options.layer);
    const message = formatLayeredDiagnostic(channel, event, fields, options);
    if (consoleSinkAllows(level)) {
        try {
            const timestamp = nowStamp();
            const fieldText = formatFields(fields);
            const consoleMessage = `%c${timestamp} %c${level.padEnd(5)} %c[${layer.padEnd(8)}]%c ${event}${fieldText ? ` ${fieldText}` : ''}`;
            const logger = level === 'ERROR' ? console.error : level === 'WARN' ? console.warn : console.log;
            logger(
                consoleMessage,
                'color:#8b949e',
                CONSOLE_LEVEL_STYLE[level],
                'color:#c084fc;font-weight:700',
                'color:inherit',
            );
        } catch {
            // Console diagnostics are best-effort only; terminal_log remains the authoritative sink.
        }
    }
    if (typeof window !== 'undefined') {
        try {
            const win = window as any;
            win.__PDF_DIAGNOSTICS_HISTORY = win.__PDF_DIAGNOSTICS_HISTORY || [];
            win.__PDF_DIAGNOSTICS_HISTORY.push({
                timestamp: nowStamp(),
                channel,
                event,
                fields,
                level,
                layer,
                message
            });
            // Cap at 5000 — the original 1000 is easily blown by layout
            // traces during a render iteration, which drops older events
            // (2026-09-28 perf probe showed wheel-event-timing evicted).
            if (win.__PDF_DIAGNOSTICS_HISTORY.length > 5000) {
                win.__PDF_DIAGNOSTICS_HISTORY.shift();
            }
        } catch {
            // ignore
        }
    }
    // Terminal sink is for human-visible events only — DEBUG/TRACE flood the
    // IPC channel during zoom gestures, and the in-page history above remains
    // the authoritative probe source. PROF is a probe channel too
    // (plan-build/render timings): its consumers read the in-page history.
    // NOTE (ADR-0015, reverted): batching these into one IPC wedged the page
    // under verbose tracing (unbounded queue growth between flushes); see the
    // ADR before re-attempting.
    if (level !== 'DEBUG' && level !== 'TRACE' && layer !== 'PROF') {
        // Built lazily: the ANSI variant is only consumed by this branch, and
        // under a verbose flood the per-event format cost is itself measurable.
        const terminalMessage = formatLayeredDiagnostic(channel, event, fields, options, true);
        void targetInvokeV3('terminal_log', {
            message: terminalMessage,
        }).catch(() => undefined);
    }
}
