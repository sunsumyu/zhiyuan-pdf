export type ViewerSessionSnapshot = {
    path: string | null;
    currentPage: number;
};

export type PdfPageAnnotationTarget = {
    id: string;
    kind: string;
    pageIndex: number;
    label: string;
};

export type PdfCommentTargetOverlayMarker = {
    id: string;
    kind: string;
    pageIndex: number;
    label: string;
    title: string;
    frame: {
        leftPercent: number;
        topPercent: number;
        widthPercent: number;
        heightPercent: number;
    };
};

export type PdfCommentTargetOverlayDisplay = {
    targets: PdfCommentTargetOverlayMarker[];
};

export type PdfPageCommentItem = {
    id: string;
    pageIndex: number;
    pageWidth: number;
    pageHeight: number;
    color: [number, number, number];
    contents: string;
    boxRect: {
        left: number;
        top: number;
        width: number;
        height: number;
    };
};

export type PdfCommentOverlayMarker = {
    id: string;
    title: string;
    frame: {
        leftPercent: number;
        topPercent: number;
        widthPercent: number;
        heightPercent: number;
    };
    selected: boolean;
};

export type PdfCommentOverlayDisplay = {
    comments: PdfCommentOverlayMarker[];
};

export type PdfCommentReviewPageSummary = {
    pageIndex: number;
    totalComments: number;
    filteredComments: number;
};

export type PdfCommentReviewResult = {
    totalComments: number;
    filteredComments: number;
    pagesWithComments: number;
    summaries: PdfCommentReviewPageSummary[];
    comments: PdfPageCommentItem[];
};

export type PdfCommentReviewSummaryChip = {
    pageIndex: number;
    label: string;
};

export type PdfCommentReviewCardAction = {
    id: string;
    label: string;
    tone: 'primary' | 'success' | 'danger' | string;
};

export type PdfCommentReviewCard = {
    id: string;
    pageIndex: number;
    contents: string;
    pageLabel: string;
    locationLabel: string;
    helperLabel: string;
    selected: boolean;
    actions: PdfCommentReviewCardAction[];
};

export type PdfCommentReviewPanel = {
    metaText: string;
    empty: boolean;
    summaryChips: PdfCommentReviewSummaryChip[];
    cards: PdfCommentReviewCard[];
};

export type CommentReviewScope = 'page' | 'document';

export type CommentReviewSession = {
    panelOpen: boolean;
    scope: CommentReviewScope;
    query: string;
    selectedCommentId?: string | null;
};

export type PdfCommentReviewDisplay = {
    session: CommentReviewSession;
    review: PdfCommentReviewResult;
    panel: PdfCommentReviewPanel;
    overlay: PdfCommentOverlayDisplay;
};

export const EMPTY_REVIEW_RESULT: PdfCommentReviewResult = {
    totalComments: 0,
    filteredComments: 0,
    pagesWithComments: 0,
    summaries: [],
    comments: [],
};

export const EMPTY_REVIEW_PANEL: PdfCommentReviewPanel = {
    metaText: 'No comments loaded',
    empty: true,
    summaryChips: [],
    cards: [],
};

export const EMPTY_OVERLAY_DISPLAY: PdfCommentOverlayDisplay = {
    comments: [],
};

export const EMPTY_TARGET_OVERLAY_DISPLAY: PdfCommentTargetOverlayDisplay = {
    targets: [],
};

export function normalizeReviewSession(raw: CommentReviewSession | null | undefined): CommentReviewSession {
    return {
        panelOpen: !!raw?.panelOpen,
        scope: raw?.scope === 'document' ? 'document' : 'page',
        query: raw?.query ?? '',
        selectedCommentId: raw?.selectedCommentId ?? null,
    };
}

export function normalizeOverlayDisplay(raw: PdfCommentOverlayDisplay | null | undefined): PdfCommentOverlayDisplay {
    return raw ?? EMPTY_OVERLAY_DISPLAY;
}

export function normalizeTargetOverlayDisplay(
    raw: PdfCommentTargetOverlayDisplay | null | undefined,
): PdfCommentTargetOverlayDisplay {
    return raw ?? EMPTY_TARGET_OVERLAY_DISPLAY;
}

export function normalizeReviewDisplay(
    raw:
        | {
            session?: CommentReviewSession | null;
            review?: PdfCommentReviewResult | null;
            panel?: PdfCommentReviewPanel | null;
            overlay?: PdfCommentOverlayDisplay | null;
        }
        | null
        | undefined,
): PdfCommentReviewDisplay {
    return {
        session: normalizeReviewSession(raw?.session),
        review: raw?.review ?? EMPTY_REVIEW_RESULT,
        panel: raw?.panel ?? EMPTY_REVIEW_PANEL,
        overlay: normalizeOverlayDisplay(raw?.overlay),
    };
}
