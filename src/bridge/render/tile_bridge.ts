// ─────────────────────────────────────────────────────────────────────────────
// Tile rendering bridge — typed access to the Rust TileManager facade exports.
//
// All calls go through getWasmApi() (the wasm module object), NOT window —
// wasm_bindgen js_name exports live on the module, not on globalThis.
// Every entry is optional-chained so the bridge is safe before WASM init and
// in unit tests.
// ─────────────────────────────────────────────────────────────────────────────

import { getWasmApi } from '../shared/wasm_loader';

export type TileRenderRequest = {
    tile_key: { page: number; zoom: number; dpr: number; x: number; y: number };
    priority: number; // 0=viewport, 1=near, 2=far
    frame_token: number;
};

export type TileManagerStats = {
    cache: {
        total: number;
        pending: number;
        rendering: number;
        ready: number;
        failed: number;
        max_size: number;
    };
    queue_size: number;
    current_frame_token: number;
    is_animating: boolean;
};

function api(): any {
    try {
        return getWasmApi() as any;
    } catch {
        return null;
    }
}

export const tileFacade = {
    updateViewport(
        page: number,
        zoom: number,
        dpr: number,
        viewportX: number,
        viewportY: number,
        viewportWidth: number,
        viewportHeight: number,
        frameToken: number,
    ): void {
        api()?.renderFacadeUpdateViewport?.(
            page, zoom, dpr, viewportX, viewportY, viewportWidth, viewportHeight, frameToken,
        );
    },

    startAnimation(targetZoom: number): void {
        api()?.renderFacadeStartTileAnimation?.(targetZoom);
    },

    updateAnimation(visualZoom: number, frameToken: number): void {
        api()?.renderFacadeUpdateTileAnimation?.(visualZoom, frameToken);
    },

    endAnimation(frameToken: number): void {
        api()?.renderFacadeEndTileAnimation?.(frameToken);
    },

    nextRequest(): TileRenderRequest | null {
        const req = api()?.renderFacadeNextTileRequest?.();
        return req ?? null;
    },

    markRendering(page: number, zoom: number, dpr: number, x: number, y: number): void {
        api()?.renderFacadeMarkTileRendering?.(page, zoom, dpr, x, y);
    },

    markReady(page: number, zoom: number, dpr: number, x: number, y: number): void {
        api()?.renderFacadeMarkTileReady?.(page, zoom, dpr, x, y);
    },

    /** Flip a Rendering tile back to Pending (dropped render / render error). */
    resetTile(page: number, zoom: number, dpr: number, x: number, y: number): void {
        api()?.renderFacadeResetTile?.(page, zoom, dpr, x, y);
    },

    isReady(page: number, zoom: number, dpr: number, x: number, y: number): boolean {
        return !!api()?.renderFacadeIsTileReady?.(page, zoom, dpr, x, y);
    },

    clearPage(page: number): void {
        api()?.renderFacadeClearTileCache?.(page);
    },

    stats(): TileManagerStats | null {
        const stats = api()?.renderFacadeTileStats?.();
        return stats ?? null;
    },
};
