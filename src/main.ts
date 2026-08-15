// Icons are inlined as SVG in index.html — no FontAwesome dependency needed.

import { plugin } from './bridge';
import { getPdfViewerAPI } from './bridge/viewer/pdf_viewer_api';
import { invoke } from '@tauri-apps/api/core';
// 应用内自验证：挂 `window.verifyEditorBugs()` 到全局，DevTools 控制台可直调。
import './dev/verify_editor_bugs';

function api() {
    return getPdfViewerAPI();
}

async function init() {
    performance.mark('viewer-init-start');
    console.log('Initializing Sovereignty PDF Viewer...');

    const openBtn = document.getElementById('open-btn');
    const openEmptyStateBtn = document.getElementById('open-empty-state-btn');
    // 不阻塞 UI：plugin.initialize() (含 WASM 加载) 在后台并行进行。
    // 用户点击"打开"时，openPdfFile 内部会自己 await ensureWasmInitialized()。
    performance.mark('plugin-init-start');
    const initPromise = plugin.initialize()
        .then(() => {
            performance.mark('plugin-init-end');
            performance.measure('plugin-initialize', 'plugin-init-start', 'plugin-init-end');
        })
        .catch((err) => {
            console.error('[PDF] Plugin initialization failed (buttons will still work):', err);
        });
    void initPromise;

    // --- Navbar Actions ---

    // 1. Open Button
    const pathSpan = document.getElementById('file-path');
    const hiddenInput = document.getElementById('pdf-hidden-file-input') as HTMLInputElement;

    const handleFileOpen = async (filePath: string) => {
        try {
            await api()?.openPdfFile(filePath);
            pathSpan!.textContent = filePath;
        } catch (err) {
            console.error('[PDF] Failed to open file:', filePath, err);
            pathSpan!.textContent = '';
        }
    };

    // 注册高级处理器，替换 index.html 里的 inline fallback
    (window as any).__pdfOpenHandler = async () => {
        if ((window as any).__TAURI_INTERNALS__ || (window as any).__TAURI__) {
            try {
                const selected = await invoke<string | null>('pick_file', {});
                if (selected) {
                    await handleFileOpen(selected);
                }
            } catch (err) {
                console.error('Tauri open failed:', err);
            }
        } else {
            hiddenInput?.click();
        }
        (document.activeElement as HTMLElement | null)?.blur?.();
    };

    openEmptyStateBtn?.addEventListener('click', () => {
        openBtn?.click();
    });

    // 消费 inline script 已经选好的文件，或者通过 URL 参数传入的文件
    const urlParams = new URLSearchParams(window.location.search);
    const fileParam = urlParams.get('file');
    const pendingPath = fileParam || (window as any).__pendingPdfPath;
    if (pendingPath) {
        delete (window as any).__pendingPdfPath;
        handleFileOpen(pendingPath);
    }

    hiddenInput?.addEventListener('change', (e) => {
        const file = (e.target as HTMLInputElement).files?.[0];
        if (file) {
            // In browser, we can't get the full path, but we can use URL.createObjectURL
            const url = URL.createObjectURL(file);
            handleFileOpen(url);
            pathSpan!.textContent = file.name;
        }
    });

    // 2. Save Button
    document.getElementById('pdf-save-btn')?.addEventListener('click', () => {
        api()?.save();
    });

    // 3. Print Button (Placeholder)
    document.getElementById('pdf-print-btn')?.addEventListener('click', () => {
        alert('Print feature coming soon in standalone version.');
    });

    // 4. Undo / Redo
    document.getElementById('pdf-undo-btn')?.addEventListener('click', () => api()?.undo());
    document.getElementById('pdf-redo-btn')?.addEventListener('click', () => api()?.redo());

    // 5. Page Navigation
    document.getElementById('pdf-prev-page-btn')?.addEventListener('click', () => {
        api()?.prevPage();
    });
    document.getElementById('pdf-next-page-btn')?.addEventListener('click', () => {
        api()?.nextPage();
    });

    // 6. Zoom
    const zoomSelect = document.getElementById('pdf-zoom-select') as HTMLSelectElement;
    zoomSelect?.addEventListener('change', (e) => {
        const val = (e.target as HTMLSelectElement).value;
        api()?.setZoom(val);
    });

    document.getElementById('pdf-zoom-in-btn')?.addEventListener('click', () => {
        const currentZoom = parseFloat(zoomSelect.value);
        const nextZoom = (currentZoom + 0.25).toFixed(2);
        api()?.setZoom(nextZoom);
        zoomSelect.value = nextZoom;
    });

    document.getElementById('pdf-zoom-out-btn')?.addEventListener('click', () => {
        const currentZoom = parseFloat(zoomSelect.value);
        const nextZoom = Math.max(0.25, currentZoom - 0.25).toFixed(2);
        api()?.setZoom(nextZoom);
        zoomSelect.value = nextZoom;
    });

    // 7. Search Toggle
    document.getElementById('pdf-search-btn')?.addEventListener('click', () => {
        const findContainer = document.getElementById('pdf-find-container');
        if (findContainer) {
            findContainer.style.display = findContainer.style.display === 'none' ? 'flex' : 'none';
            if (findContainer.style.display === 'flex') {
                document.getElementById('pdf-find-input')?.focus();
            }
        }
    });

    // 8. AI Assistant Toggle
    document.getElementById('pdf-ai-toggle-btn')?.addEventListener('click', () => {
        const aiPanel = document.getElementById('pdf-ai-panel');
        if (aiPanel) {
            aiPanel.style.display = aiPanel.style.display === 'none' ? 'flex' : 'none';
        }
    });

    // --- Sub-Toolbar (Editing Tools) ---

    // Select Mode
    document.getElementById('pdf-select-mode-btn')?.addEventListener('click', (e) => {
        document.querySelectorAll('.tool-btn').forEach(b => b.classList.remove('active'));
        (e.currentTarget as HTMLElement).classList.add('active');
    });

    // Hand Tool
    document.getElementById('pdf-hand-mode-btn')?.addEventListener('click', (e) => {
        document.querySelectorAll('.tool-btn').forEach(b => b.classList.remove('active'));
        (e.currentTarget as HTMLElement).classList.add('active');
    });

    // Add Text Mode
    document.getElementById('pdf-add-text-btn')?.addEventListener('click', (e) => {
        document.querySelectorAll('.tool-btn').forEach(b => b.classList.remove('active'));
        (e.currentTarget as HTMLElement).classList.add('active');
        api()?.toggleTextEditMode();
    });

    // Draw / Highlight (Placeholders)
    document.getElementById('pdf-draw-btn')?.addEventListener('click', (e) => {
        document.querySelectorAll('.tool-btn').forEach(b => b.classList.remove('active'));
        (e.currentTarget as HTMLElement).classList.add('active');
        alert('Drawing tool coming soon.');
    });

    document.getElementById('pdf-highlight-btn')?.addEventListener('click', (e) => {
        document.querySelectorAll('.tool-btn').forEach(b => b.classList.remove('active'));
        (e.currentTarget as HTMLElement).classList.add('active');
        alert('Highlight tool coming soon.');
    });

    // Formatting
    document.getElementById('pdf-format-bold-btn')?.addEventListener('click', () => api()?.toggleBold());
    document.getElementById('pdf-format-italic-btn')?.addEventListener('click', () => api()?.toggleItalic());
    document.getElementById('pdf-format-underline-btn')?.addEventListener('click', () => api()?.toggleUnderline());

    document.getElementById('pdf-format-color-input')?.addEventListener('input', (e) => {
        const color = (e.target as HTMLInputElement).value;
        api()?.setColor(color);
    });

    // --- Sidebar ---
    document.getElementById('pdf-sidebar-toggle-btn')?.addEventListener('click', () => {
        const sidebar = document.getElementById('pdf-sidebar-left');
        const icon = document.querySelector('#pdf-sidebar-toggle-btn i');
        if (sidebar) {
            const isCollapsed = sidebar.style.width === '0px';
            sidebar.style.width = isCollapsed ? 'var(--sidebar-width)' : '0px';
            sidebar.style.minWidth = isCollapsed ? 'var(--sidebar-width)' : '0px';
            icon?.classList.toggle('fa-angle-left', !isCollapsed);
            icon?.classList.toggle('fa-angle-right', isCollapsed);
        }
    });

    void openBtn; // kept for future use; no longer disabled during init
    performance.mark('viewer-init-end');
    performance.measure('viewer-init-total', 'viewer-init-start', 'viewer-init-end');
    console.log('PDF Viewer Logic Wired Up.');
}

// Start the app
init().catch(err => {
    console.error('Failed to initialize PDF viewer:', err);
});
