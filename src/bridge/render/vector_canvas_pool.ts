import { emitPdfDiagnostic } from '../shared/diagnostics';

/**
 * Production-grade Canvas Object Pool to eliminate DOM creation overhead
 * and micro-stutters from Garbage Collection (GC thrashing).
 */
export class CanvasPool {
    private static pool: HTMLCanvasElement[] = [];
    private static readonly MAX_POOL_SIZE = 12;

    /**
     * Rents a canvas with the specified dimensions from the pool, or creates one if empty.
     */
    public static rent(width: number, height: number): HTMLCanvasElement {
        const targetWidth = Math.max(1, Math.round(width));
        const targetHeight = Math.max(1, Math.round(height));

        let canvas: HTMLCanvasElement;
        if (this.pool.length > 0) {
            canvas = this.pool.pop()!;
            // Only adjust size if it has changed to minimize internal browser GPU buffer re-allocations
            if (canvas.width !== targetWidth) canvas.width = targetWidth;
            if (canvas.height !== targetHeight) canvas.height = targetHeight;
        } else {
            canvas = document.createElement('canvas');
            canvas.width = targetWidth;
            canvas.height = targetHeight;
        }

        // Clean slate transform state
        const ctx = canvas.getContext('2d', { alpha: false });
        if (ctx) {
            ctx.setTransform(1, 0, 0, 1, 0, 0);
        }

        emitPdfDiagnostic('canvas-pool', 'rent', {
            poolSize: this.pool.length,
            rentedWidth: targetWidth,
            rentedHeight: targetHeight,
        }, { verboseOnly: true });

        return canvas;
    }

    /**
     * Resets, clears, and returns a canvas back to the pool for recycling.
     */
    public static recycle(canvas: HTMLCanvasElement): void {
        if (!canvas) return;

        // Reset transform and clear buffer to release GPU memory representation
        const ctx = canvas.getContext('2d', { alpha: false });
        if (ctx) {
            ctx.setTransform(1, 0, 0, 1, 0, 0);
            ctx.clearRect(0, 0, canvas.width, canvas.height);
        }

        if (this.pool.length < this.MAX_POOL_SIZE) {
            this.pool.push(canvas);
        } else {
            // Buffer full: drop canvas to let browser dereference and clean up
            canvas.width = 1;
            canvas.height = 1;
        }

        emitPdfDiagnostic('canvas-pool', 'recycle', {
            poolSize: this.pool.length,
        }, { verboseOnly: true });
    }

    /**
     * Empties the pool and releases all retained canvases.
     */
    public static clear(): void {
        for (const canvas of this.pool) {
            canvas.width = 1;
            canvas.height = 1;
        }
        this.pool = [];
        emitPdfDiagnostic('canvas-pool', 'clear', {}, { verboseOnly: true });
    }
}
