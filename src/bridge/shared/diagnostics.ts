import { targetInvokeV3 } from './wasm_loader';

type DiagnosticFields = Record<string, unknown>;

type DiagnosticOptions = {
    verboseOnly?: boolean;
};

const MAX_FIELD_TEXT = 120;

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

export function formatPdfDiagnostic(channel: string, event: string, fields: DiagnosticFields = {}): string {
    const fieldText = Object.entries(fields)
        .filter(([, value]) => value !== undefined)
        .map(([key, value]) => `${key}=${compactValue(key, value)}`)
        .join(' | ');
    return [`[pdf.${channel}]`, event, fieldText].filter(Boolean).join(' ');
}

export function emitPdfDiagnostic(
    channel: string,
    event: string,
    fields: DiagnosticFields = {},
    options: DiagnosticOptions = {},
): void {
    if (!diagnosticsEnabled()) return;
    if (options.verboseOnly && !verbosePdfDiagnosticsEnabled()) return;
    const message = formatPdfDiagnostic(channel, event, fields);
    try {
        // Use console.log so messages appear at default DevTools log level
        // (console.debug is hidden unless "Verbose" filter is enabled).
        console.log(message);
    } catch {
        // Console diagnostics are best-effort only; terminal_log remains the authoritative sink.
    }
    void targetInvokeV3('terminal_log', {
        message,
    }).catch(() => undefined);
}
