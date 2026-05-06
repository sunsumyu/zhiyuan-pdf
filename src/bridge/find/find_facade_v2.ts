// Find facade v2 — frozen v1 TS bindings against `crate::find::facade`.
// (named *_v2.ts to avoid clashing with existing bridge/find_facade.ts)
// See docs/api-contract.md.

import { getWasmApi } from '../shared/wasm_loader';

export type StubResult = { implemented: boolean; error: string };

function call<T>(name: string, ...args: unknown[]): T | null {
    const api = getWasmApi();
    const fn = (api as any)[name];
    if (typeof fn !== 'function') return null;
    try { return args.length ? fn(...args) : fn(); } catch { return null; }
}

// Stable
export function facadeFindClearSession(): void { call('findFacadeClearSession'); }
export function facadeFindReadSession(): unknown { return call('findFacadeReadSession'); }
export function facadeFindSetSession(
    query: string,
    scope: 'page' | 'document',
    matchPages: number[],
    preferredActivePage?: number,
): unknown {
    return call('findFacadeSetSession', query, scope, matchPages, preferredActivePage ?? null);
}
export function facadeFindMoveMatch(step: number): unknown { return call('findFacadeMoveMatch', step); }

// Stubs
export function facadeFindSetOptions(caseSensitive: boolean, wholeWord: boolean, regex: boolean): StubResult | null {
    return call('findFacadeSetOptions', caseSensitive, wholeWord, regex);
}
export function facadeFindReplaceCurrent(replacement: string): StubResult | null {
    return call('findFacadeReplaceCurrent', replacement);
}
export function facadeFindReplaceAll(replacement: string): StubResult | null {
    return call('findFacadeReplaceAll', replacement);
}
export function facadeFindHighlightAll(enabled: boolean): StubResult | null {
    return call('findFacadeHighlightAll', enabled);
}

