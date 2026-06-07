import { targetInvokeV3 } from '../shared/wasm_loader';
import type { RenderReason } from '../render/frame_plan';
import type { RenderScheduler } from '../render/render_scheduler';
import type { PdfEditSource } from '../document/document_edit_api';
import type { EditorFormatAction } from '../editor/types';
import type { PageTurnDecision } from './page_presentation_runtime';


export type PdfViewerApiDeps = {
    ensureWasmInitialized: () => Promise<unknown>;
    getWasmApi: () => any;
    readPath: () => string | null;
    readCurrentPage: () => number;
    readPageCount: () => number;
    requestPageTurn: (targetPage: number, reason: 'next' | 'prev' | 'jump') => PageTurnDecision;
    setCurrentPage: (pageIndex: number) => void;
    refreshDocument: (reason: PdfEditSource) => Promise<unknown>;
    resetPdfViewerState: () => void;
    renderScheduler: RenderScheduler;
    renderCurrentPage: (reason?: RenderReason) => Promise<void>;
    clampZoom: (zoom: number) => number;
    syncZoomSelect: () => void;
    syncTextEditButton: () => void;
    readTargetZoom: () => number;
    editorHost: {
        clear: () => void;
        isTextEditEnabled: () => boolean;
        commitActiveEditor: () => Promise<unknown>;
        saveEdits: () => Promise<{
            saved?: boolean;
            hadPersistablePatches?: boolean;
            errorMessage?: string | null;
        }>;
        applyFormatAction: (action: EditorFormatAction) => Promise<void>;
        setTextEditEnabled: (enabled: boolean) => void;
        syncTargets: (displayZoom: number) => void;
    };
    annotationController: { toggle: () => Promise<void> };
    commentController: { toggle: () => Promise<void>; togglePanel: () => Promise<void> };
    reviewController: { togglePanel: () => Promise<void> };
    findController: {
        toggle: () => void;
        open: () => void;
        close: () => void;
        next: () => Promise<void>;
        prev: () => Promise<void>;
    };
    resumeAiController: {
        togglePanel: () => void;
        applyAllSuggestions: () => Promise<unknown>;
    };
    defaultPageWidth: number;
    defaultPageHeight: number;
    openTextPdfFlow: (path: string) => Promise<void>;
    clearVectorHost: () => void;
    geometryProbe: unknown;
};

export class PdfViewerAPI {
    private deps: PdfViewerApiDeps;

    constructor(deps: PdfViewerApiDeps) {
        this.deps = deps;
    }

    // === Document Lifecycle ===

    async openPdfFile(path?: string): Promise<void> {
        await this.deps.ensureWasmInitialized();
        if (path) {
            await this.deps.openTextPdfFlow(path);
            return;
        }
        const wasm = this.deps.getWasmApi();
        const openResult = await wasm.pick_document_pipeline({
            initialZoom: 1.0,
            defaultPageWidth: this.deps.defaultPageWidth,
            defaultPageHeight: this.deps.defaultPageHeight,
        });
        if (openResult?.opened && openResult?.path) {
            this.deps.clearVectorHost();
            this.deps.editorHost.clear();
            this.deps.syncZoomSelect();
            this.deps.syncTextEditButton();
            await this.deps.renderCurrentPage();
        }
    }

    closePdf(): void {
        this.deps.resetPdfViewerState();
    }

    // === Page Navigation ===

    async prevPage(): Promise<void> {
        const current = this.deps.readCurrentPage();
        const decision = this.deps.requestPageTurn(Math.max(0, current - 1), 'prev');
        if (!decision.accepted) return;
        this.deps.setCurrentPage(decision.targetPage);
        await this.deps.renderScheduler.requestRender('navigation', 'navigation', {
            pageTurnId: decision.pageTurnId,
            targetPage: decision.targetPage,
        });
    }

    async nextPage(): Promise<void> {
        const current = this.deps.readCurrentPage();
        const decision = this.deps.requestPageTurn(current + 1, 'next');
        if (!decision.accepted) return;
        this.deps.setCurrentPage(decision.targetPage);
        await this.deps.renderScheduler.requestRender('navigation', 'navigation', {
            pageTurnId: decision.pageTurnId,
            targetPage: decision.targetPage,
        });
    }

    // === Zoom ===

    async setZoom(val: string): Promise<void> {
        let zoom = parseFloat(val);
        if (val.includes('%')) zoom = zoom / 100;
        const nextZoom = this.deps.clampZoom(zoom);
        const result = this.deps.getWasmApi().apply_zoom_selection?.(nextZoom);
        this.deps.syncZoomSelect();
        if (result?.changed) {
            await this.deps.renderCurrentPage();
        }
    }

    // === Edit Operations ===

    async undo(): Promise<void> {
        if (this.deps.editorHost.isTextEditEnabled()) {
            await this.deps.editorHost.commitActiveEditor();
        }
        const wasm = this.deps.getWasmApi();
        const result = wasm.undo_document_pipeline() as { changed?: boolean } | null;
        if (result?.changed) {
            await this.deps.refreshDocument('undo');
            return;
        }
        await this.undoSavedEdit();
    }

    async redo(): Promise<void> {
        if (this.deps.editorHost.isTextEditEnabled()) {
            await this.deps.editorHost.commitActiveEditor();
        }
        const wasm = this.deps.getWasmApi();
        const result = wasm.redo_document_pipeline() as { changed?: boolean } | null;
        if (result?.changed) {
            await this.deps.refreshDocument('redo');
            return;
        }
        await this.redoSavedEdit();
    }

    async rotate(delta = 90): Promise<void> {
        const result = await this.deps.getWasmApi().rotate_document_pipeline(delta);
        if (!result?.rotated) return;
        await this.deps.refreshDocument('rotate');
    }

    async save(): Promise<void> {
        const saveResult = await this.deps.editorHost.saveEdits();
        if (saveResult.saved || saveResult.hadPersistablePatches) {
            if (!saveResult.saved) {
                console.warn('[PDF][save] failed', saveResult.errorMessage ?? 'unknown');
            }
            return;
        }
        await this.deps.resumeAiController.applyAllSuggestions();
    }

    // === Text Edit Mode ===

    async toggleTextEditMode(): Promise<void> {
        const nextEnabled = !this.deps.editorHost.isTextEditEnabled();
        if (!nextEnabled) {
            await this.deps.editorHost.commitActiveEditor();
        }
        this.deps.editorHost.setTextEditEnabled(nextEnabled);
        this.deps.syncTextEditButton();
        if (this.deps.readPath()) {
            this.deps.editorHost.syncTargets(this.deps.readTargetZoom());
        }
    }

    // === Annotation ===

    async toggleAnnotation(): Promise<void> {
        return this.deps.annotationController.toggle();
    }

    // === Comment ===

    async toggleComment(): Promise<void> {
        return this.deps.commentController.toggle();
    }

    async toggleCommentPanel(): Promise<void> {
        return this.deps.commentController.togglePanel();
    }

    // === Review ===

    async toggleReviewPanel(): Promise<void> {
        return this.deps.reviewController.togglePanel();
    }

    // === Format ===

    async toggleBold(): Promise<void> {
        return this.deps.editorHost.applyFormatAction({ type: 'toggleBold' });
    }

    async toggleItalic(): Promise<void> {
        return this.deps.editorHost.applyFormatAction({ type: 'toggleItalic' });
    }

    async toggleUnderline(): Promise<void> {
        return this.deps.editorHost.applyFormatAction({ type: 'toggleUnderline' });
    }

    async setColor(color: string): Promise<void> {
        return this.deps.editorHost.applyFormatAction({ type: 'setColor', color });
    }

    async setFontFamily(fontFamily: string): Promise<void> {
        return this.deps.editorHost.applyFormatAction({ type: 'setFontFamily', fontFamily });
    }

    async setFontSize(fontSize: number): Promise<void> {
        return this.deps.editorHost.applyFormatAction({ type: 'setFontSize', fontSize });
    }

    async increaseFontSize(): Promise<void> {
        return this.deps.editorHost.applyFormatAction({ type: 'increaseFontSize' });
    }

    async decreaseFontSize(): Promise<void> {
        return this.deps.editorHost.applyFormatAction({ type: 'decreaseFontSize' });
    }

    async setCharSpacing(charSpacing: number): Promise<void> {
        return this.deps.editorHost.applyFormatAction({ type: 'setCharSpacing', charSpacing });
    }

    async setLineHeight(lineHeight: number): Promise<void> {
        return this.deps.editorHost.applyFormatAction({ type: 'setLineHeight', lineHeight });
    }

    async setParagraphMode(mode: string): Promise<void> {
        return this.deps.editorHost.applyFormatAction({ type: 'setParagraphMode', mode });
    }

    async setAlignment(alignment: 'left' | 'center' | 'right' | 'justify'): Promise<void> {
        return this.deps.editorHost.applyFormatAction({ type: 'setAlignment', alignment });
    }

    async setListKind(listKind: 'none' | 'bullet' | 'numbering'): Promise<void> {
        return this.deps.editorHost.applyFormatAction({ type: 'setListKind', listKind });
    }

    // === AI ===

    toggleAiPanel(): void {
        this.deps.resumeAiController.togglePanel();
    }

    // === Find ===

    toggleFind(): void {
        this.deps.findController.toggle();
    }

    openFind(): void {
        this.deps.findController.open();
    }

    closeFind(): void {
        this.deps.findController.close();
    }

    async findNext(): Promise<void> {
        return this.deps.findController.next();
    }

    async findPrev(): Promise<void> {
        return this.deps.findController.prev();
    }

    // === Helpers ===

    private async undoSavedEdit(): Promise<boolean> {
        const path = this.deps.readPath();
        if (!path) return false;
        try {
            await targetInvokeV3('undo', { path });
            await this.deps.refreshDocument('rollback');
            return true;
        } catch (error) {
            console.warn('[PDF][undo][rollback] failed', error);
            return false;
        }
    }

    private async redoSavedEdit(): Promise<boolean> {
        const path = this.deps.readPath();
        if (!path) return false;
        try {
            await targetInvokeV3('redo', { path });
            await this.deps.refreshDocument('redo-rollback');
            return true;
        } catch (error) {
            console.warn('[PDF][redo][rollback] failed', error);
            return false;
        }
    }
}

// Global instance (legacy compatibility during migration)
let globalApi: PdfViewerAPI | null = null;

export function registerPdfViewerAPI(deps: PdfViewerApiDeps): PdfViewerAPI {
    globalApi = new PdfViewerAPI(deps);

    // V3 callbacks (required by WASM bridge)
    const w = window as any;
    w.onDebugV3 = (label: string, payload: string) => {
        console.log(`[PDF-RUST][${label}] ${payload}`);
    };
    w.onOpenV3 = () => {};
    w.onCloseV3 = () => {};
    w.onInputV3 = () => {
        window.requestAnimationFrame(() => {
            void deps.renderCurrentPage();
        });
    };
    w.onCommitV3 = () => {
        window.requestAnimationFrame(() => {
            void deps.renderCurrentPage();
        });
    };
    w.onCancelV3 = () => {
        deps.editorHost.clear();
        deps.syncTextEditButton();
    };
    w.__pdfViewerGeometryProbe = deps.geometryProbe;
    w.__getCurrentPage = () => deps.readCurrentPage();

    // Bind legacy window.* functions for backward compatibility
    w.openPdfFile = (path?: string) => globalApi!.openPdfFile(path);
    w.pdfPrevPage = () => globalApi!.prevPage();
    w.pdfNextPage = () => globalApi!.nextPage();
    w.pdfZoomChange = (val: string) => globalApi!.setZoom(val);
    w.closePdf = () => globalApi!.closePdf();
    w.pdfUndo = () => globalApi!.undo();
    w.pdfRedo = () => globalApi!.redo();
    w.pdfRotate = (delta = 90) => globalApi!.rotate(delta);
    w.pdfSave = () => globalApi!.save();
    w.toggleAddTextMode = () => globalApi!.toggleTextEditMode();
    w.toggleHighlightMode = () => globalApi!.toggleAnnotation();
    w.toggleCommentMode = () => globalApi!.toggleComment();
    w.pdfToggleCommentReview = () => globalApi!.toggleCommentPanel();
    w.pdfToggleReview = () => globalApi!.toggleReviewPanel();
    w.pdfToggleBold = () => globalApi!.toggleBold();
    w.pdfToggleItalic = () => globalApi!.toggleItalic();
    w.pdfToggleUnderline = () => globalApi!.toggleUnderline();
    w.pdfSetColor = (color: string) => globalApi!.setColor(color);
    w.pdfSetFontFamily = (fontFamily: string) => globalApi!.setFontFamily(fontFamily);
    w.pdfSetFontSize = (fontSize: number) => globalApi!.setFontSize(fontSize);
    w.pdfIncreaseFontSize = () => globalApi!.increaseFontSize();
    w.pdfDecreaseFontSize = () => globalApi!.decreaseFontSize();
    w.pdfSetCharSpacing = (charSpacing: number) => globalApi!.setCharSpacing(charSpacing);
    w.pdfSetLineHeight = (lineHeight: number) => globalApi!.setLineHeight(lineHeight);
    w.pdfSetParagraphMode = (mode: string) => globalApi!.setParagraphMode(mode);
    w.pdfSetAlignment = (alignment: 'left' | 'center' | 'right' | 'justify') => globalApi!.setAlignment(alignment);
    w.pdfSetListKind = (listKind: 'none' | 'bullet' | 'numbering') => globalApi!.setListKind(listKind);
    w.pdfSummarize = () => globalApi!.toggleAiPanel();
    w.pdfToggleFind = () => globalApi!.toggleFind();
    w.pdfFindNext = () => globalApi!.findNext();
    w.pdfFindPrev = () => globalApi!.findPrev();
    w.pdfCloseFind = () => globalApi!.closeFind();

    return globalApi;
}

export function getPdfViewerAPI(): PdfViewerAPI | null {
    return globalApi;
}
