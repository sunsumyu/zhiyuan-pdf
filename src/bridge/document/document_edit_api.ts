import type { RenderReason, RustRenderFrame } from '../render/frame_plan';
import { emitPdfDiagnostic } from '../shared/diagnostics';
import {
    createEditorWasmApi,
    type RegionTextReplaceRequest,
    type RegionTextReplaceResult,
    type AcceptReviewChangeResult,
    type RejectReviewChangeResult,
    type ReviewBulkChangeResult,
    type ReviewFeedResult,
} from '../editor/editor_wasm_api';
import { logPdfLayoutTrace } from '../render/layout_trace';

export type PdfEditSource =
    | 'manual-save'
    | 'ai-apply-one'
    | 'ai-apply-all'
    | 'undo'
    | 'redo'
    | 'rollback'
    | 'redo-rollback'
    | 'document-mutation'
    | 'rotate'
    | 'highlight'
    | 'comment'
    | 'find-replace';

export type PdfSaveResult = {
    saved: boolean;
    hadPersistablePatches?: boolean;
    errorMessage?: string;
};

export type PdfRegionTextEdit = {
    id?: string;
    pageIndex: number;
    regionId: string;
    kind: string;
    originalText: string;
    newText: string;
};

export type PdfRegionTextReplace = RegionTextReplaceRequest;

type DocumentEditApiDeps = {
    getWasmApi: () => any;
    getCurrentPath: () => string | null;
    getCurrentPage: () => number;
    getCurrentZoom: () => number;
    buildRenderRequest: (reason?: RenderReason) => Record<string, number | string | boolean>;
    renderScheduledFrame: (frame: RustRenderFrame | null) => Promise<void>;
    invalidateRenderCache: () => void;
    syncViewerState?: () => void;
};

export type DocumentEditApi = {
    applyPatch: (patch: unknown, source: PdfEditSource) => void;
    editRegionText: (edit: PdfRegionTextEdit, source: PdfEditSource) => void;
    replaceRegionTexts: (
        edits: PdfRegionTextReplace[],
        source: PdfEditSource,
    ) => Promise<RegionTextReplaceResult>;
    getReviewFeed: () => ReviewFeedResult | null;
    acceptReviewChange: (patchKey: string) => Promise<AcceptReviewChangeResult | null>;
    rejectReviewChange: (patchKey: string) => Promise<RejectReviewChangeResult | null>;
    acceptAllReviewChanges: () => Promise<ReviewBulkChangeResult | null>;
    rejectAllReviewChanges: () => Promise<ReviewBulkChangeResult | null>;
    saveEdits: (source: PdfEditSource) => Promise<PdfSaveResult>;
    refreshDocument: (source: PdfEditSource) => Promise<void>;
};

export function createDocumentEditApi(deps: DocumentEditApiDeps): DocumentEditApi {
    const editorApi = createEditorWasmApi(deps.getWasmApi);

    function logEditApi(node: string, details: Record<string, unknown>): void {
        emitPdfDiagnostic('edit-api', node, details);
    }

    async function refreshDocument(source: PdfEditSource): Promise<void> {
        const path = deps.getCurrentPath();
        const frameRequest = deps.buildRenderRequest('documentMutation');
        logEditApi('refresh.begin', {
            source,
            path,
            page: deps.getCurrentPage(),
            zoom: deps.getCurrentZoom(),
        });
        logPdfLayoutTrace('document-refresh.begin', {
            source,
            path,
            page: deps.getCurrentPage(),
            zoom: deps.getCurrentZoom(),
        });

        logPdfLayoutTrace('document-refresh.invalidate-cache.before', {
            source,
        });
        deps.invalidateRenderCache();
        logPdfLayoutTrace('document-refresh.invalidate-cache.after', {
            source,
        });
        const refreshResult = editorApi.requestDocumentRefresh(
            source,
            frameRequest,
        );
        const frame = refreshResult?.renderFrame as RustRenderFrame | null | undefined;
        logEditApi('refresh.frame', {
            source,
            revision: refreshResult?.revision ?? null,
            frame,
        });
        logPdfLayoutTrace('document-refresh.note-mutation', {
            source,
            revision: refreshResult?.revision ?? null,
        });
        logPdfLayoutTrace('document-refresh.frame-scheduled', {
            source,
            frame,
        });
        await deps.renderScheduledFrame(frame ?? null);
        deps.syncViewerState?.();
        logEditApi('refresh.done', {
            source,
            page: deps.getCurrentPage(),
            zoom: deps.getCurrentZoom(),
        });
        logPdfLayoutTrace('document-refresh.done', {
            source,
            page: deps.getCurrentPage(),
            zoom: deps.getCurrentZoom(),
        });
    }

    function applyPatch(patch: unknown, source: PdfEditSource): void {
        logEditApi('patch.apply', {
            source,
            page: deps.getCurrentPage(),
            path: deps.getCurrentPath(),
        });
        editorApi.applyDocumentPatch(patch);
    }

    function buildTextPatch(edit: PdfRegionTextEdit, source: PdfEditSource): unknown {
        if (edit.kind !== 'paragraph-region' && edit.kind !== 'list-item-region') {
            throw new Error(`暂不支持通过统一段落编辑 API 应用 ${edit.kind}`);
        }

        logEditApi('region.patch.build', {
            source,
            editId: edit.id,
            kind: edit.kind,
            regionId: edit.regionId,
        });

        const patch = editorApi.buildRegionTextPatch(
            edit.pageIndex,
            edit.regionId,
            edit.kind,
            edit.originalText,
            edit.newText,
        ) as unknown | null;
        if (patch == null) {
            throw new Error(`原生段落补丁构建失败: region=${edit.regionId}`);
        }

        return patch;
    }

    function editRegionText(edit: PdfRegionTextEdit, source: PdfEditSource): void {
        const patch = buildTextPatch(edit, source);
        applyPatch(patch, source);
    }

    async function replaceRegionTexts(
        edits: PdfRegionTextReplace[],
        source: PdfEditSource,
    ): Promise<RegionTextReplaceResult> {
        if (edits.length === 0) {
            return { appliedCount: 0, skippedCount: 0, renderFrame: null };
        }

        logEditApi('replace.begin', {
            source,
            count: edits.length,
            page: deps.getCurrentPage(),
            path: deps.getCurrentPath(),
        });
        deps.invalidateRenderCache();
        const result = editorApi.applyRegionTextReplacements(
            edits,
            deps.buildRenderRequest('documentMutation'),
        ) ?? { appliedCount: 0, skippedCount: edits.length, renderFrame: null };
        logEditApi('replace.result', {
            source,
            result,
        });
        await deps.renderScheduledFrame(result.renderFrame ?? null);
        deps.syncViewerState?.();
        return result;
    }

    async function saveEdits(source: PdfEditSource): Promise<PdfSaveResult> {
        const path = deps.getCurrentPath();
        if (!path) {
            return { saved: false, errorMessage: 'missing-path' };
        }

        logEditApi('save.begin', {
            source,
            path,
            page: deps.getCurrentPage(),
            zoom: deps.getCurrentZoom(),
        });

        const result = await editorApi.saveSession(
            path,
            deps.getCurrentPage(),
        ) as PdfSaveResult | null | undefined;
        logEditApi('save.result', { source, result });

        if (!result?.saved) {
            return {
                saved: false,
                hadPersistablePatches: result?.hadPersistablePatches,
                errorMessage: result?.errorMessage
                    ?? (result?.hadPersistablePatches ? '编辑器保存失败' : '没有可保存的修改'),
            };
        }

        await refreshDocument(source);
        return {
            saved: true,
            hadPersistablePatches: result.hadPersistablePatches,
        };
    }

    function getReviewFeed(): ReviewFeedResult | null {
        return editorApi.getReviewFeed();
    }

    async function acceptReviewChange(
        patchKey: string,
    ): Promise<AcceptReviewChangeResult | null> {
        const result = editorApi.acceptReviewChange(patchKey);
        if (result?.changed) {
            await refreshDocument('document-mutation');
        }
        return result ?? null;
    }

    async function rejectReviewChange(
        patchKey: string,
    ): Promise<RejectReviewChangeResult | null> {
        const result = editorApi.rejectReviewChange(patchKey);
        if (result?.changed) {
            await refreshDocument('document-mutation');
        }
        return result ?? null;
    }

    async function acceptAllReviewChanges(): Promise<ReviewBulkChangeResult | null> {
        const result = editorApi.acceptAllReviewChanges();
        if (result?.changed) {
            await refreshDocument('document-mutation');
        }
        return result ?? null;
    }

    async function rejectAllReviewChanges(): Promise<ReviewBulkChangeResult | null> {
        const result = editorApi.rejectAllReviewChanges();
        if (result?.changed) {
            await refreshDocument('document-mutation');
        }
        return result ?? null;
    }

    return {
        applyPatch,
        editRegionText,
        replaceRegionTexts,
        getReviewFeed,
        acceptReviewChange,
        rejectReviewChange,
        acceptAllReviewChanges,
        rejectAllReviewChanges,
        saveEdits,
        refreshDocument,
    };
}
