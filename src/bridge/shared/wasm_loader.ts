import init, * as wasmExports from '../../../crates/pdf-viewer-ui/pkg/pdf_viewer_ui';

/**
 * [Sovereignty V3] WASM 权威桥接加载 (精简)
 * 职责：仅负责 WASM 二进制文件的物理加载
 * 所有具体业务函数通过统一 Rust WASM API 直接按需获取
 */

let wasmInitialized = false;
let initPromise: Promise<any> | null = null;
let currentWasm: WasmModule | null = null;
const PDF_VIEWER_RUNTIME_FINGERPRINT = 'pdf-viewer-rust-single-chain-20260429';

function installTargetInvokeBridge(): void {
    const host = globalThis as any;
    
    // Core Bridge Mappings
    host.targetInvokeV3 = targetInvokeV3;
    host.__targetInvokeV3 = targetInvokeV3;
    host.targetInvoke = targetInvokeV3; // Legacy/Compat mapping for Rust runtime.rs
    
    // Debug Trace Mappings
    host.onDebug = (kind: string, msg: string) => {
        if (kind === 'ERROR') {
            console.error(`[WASM-${kind}]`, msg);
        } else {
            console.log(`[WASM-${kind}]`, msg);
        }
    };

    // UI Callback Stubs
    host.onInput = () => {};
    host.onOpen = () => {};
    host.onCommit = (_text: string) => {};
    host.onCancel = () => {};
    
    if (typeof window !== 'undefined') {
        (window as any).targetInvokeV3 = targetInvokeV3;
        (window as any).__targetInvokeV3 = targetInvokeV3;
        (window as any).targetInvoke = targetInvokeV3;
        (window as any).onDebug = host.onDebug;
        (window as any).onInput = host.onInput;
        (window as any).onOpen = host.onOpen;
        (window as any).onCommit = host.onCommit;
        (window as any).onCancel = host.onCancel;
    }
}

export async function ensureWasmInitialized() {
    if (wasmInitialized) return currentWasm;
    if (initPromise) return initPromise;

    initPromise = (async () => {
        try {
            console.log('[V3-Sovereign] Starting WASM Kernel Initialization...');
            console.log(`[PDF-RUNTIME] ${PDF_VIEWER_RUNTIME_FINGERPRINT}`);
            installTargetInvokeBridge();
            await init();
            wasmInitialized = true;
            currentWasm = wasmExports;
            const host = globalThis as any;
            host.wasmv3 = wasmExports;
            host.__pdfViewerRuntimeFingerprint = PDF_VIEWER_RUNTIME_FINGERPRINT;
            installTargetInvokeBridge();
            console.log('[V3-Sovereign] Rust Kernel Active');
            return wasmExports;
        } catch (err) {
            initPromise = null;
            console.error('[V3-Sovereign] WASM Kernel Critical Failure:', err);
            throw err;
        }
    })();

    return initPromise;
}

export type WasmModule = typeof wasmExports;

/**
 * [Sovereignty] 获取 WASM 原始导出对象
 * 注意：必须先调用 ensureWasmInitialized
 */
export function getWasmApi(): WasmModule {
    if (!wasmInitialized) throw new Error('[V3-Sovereign] WASM not initialized yet.');
    return currentWasm!;
}

// 代理 Tauri 侧的指令隧道
export async function targetInvokeV3(cmd: string, args: any) : Promise<any> {
    if (typeof window !== 'undefined') {
        const { invoke } = await import('@tauri-apps/api/core');
        return invoke(cmd, args);
    }
    throw new Error('targetInvokeV3 cannot be used in a Web Worker');
}
