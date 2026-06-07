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
    const terminalMessage = formatLayeredDiagnostic(channel, event, fields, options, true);
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
    void targetInvokeV3('terminal_log', {
        message: terminalMessage,
    }).catch(() => undefined);
}
