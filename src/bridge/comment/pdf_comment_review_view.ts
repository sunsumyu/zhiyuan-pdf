import type {
    PdfCommentReviewCardAction,
    PdfCommentReviewDisplay,
} from './pdf_comment_contracts';

type ReviewViewNodes = {
    meta: HTMLElement | null;
    summary: HTMLElement | null;
    list: HTMLElement | null;
    empty: HTMLElement | null;
};

type ReviewViewHandlers = {
    onSummaryChipClick: (pageIndex: number) => void;
    onCardClick: (commentId: string) => void;
    onActionClick: (action: PdfCommentReviewCardAction, commentId: string) => void;
};

export function clearCommentReviewView(nodes: ReviewViewNodes, panelOpen: boolean): void {
    if (nodes.meta) {
        nodes.meta.textContent = 'No comments loaded';
    }
    if (nodes.summary) {
        nodes.summary.innerHTML = '';
    }
    if (nodes.list) {
        nodes.list.innerHTML = '';
    }
    if (nodes.empty) {
        nodes.empty.style.display = panelOpen ? 'block' : 'none';
    }
}

export function renderCommentReviewView(
    nodes: ReviewViewNodes,
    display: PdfCommentReviewDisplay,
    handlers: ReviewViewHandlers,
): void {
    const { panel, review } = display;
    if (nodes.meta) {
        nodes.meta.textContent = panel.metaText;
    }
    renderSummary(nodes.summary, panel, handlers.onSummaryChipClick);
    if (nodes.empty) {
        nodes.empty.style.display = panel.empty ? 'block' : 'none';
    }
    if (!nodes.list) {
        return;
    }

    nodes.list.innerHTML = '';
    for (const cardModel of panel.cards) {
        const comment = review.comments.find((item) => item.id === cardModel.id);
        if (!comment) {
            continue;
        }
        const card = document.createElement('article');
        card.style.background = cardModel.selected ? 'rgba(137, 180, 250, 0.12)' : 'rgba(24, 24, 37, 0.92)';
        card.style.border = cardModel.selected ? '1px solid rgba(137, 180, 250, 0.9)' : '1px solid #313244';
        card.style.borderRadius = '12px';
        card.style.padding = '12px';
        card.style.display = 'flex';
        card.style.flexDirection = 'column';
        card.style.gap = '10px';

        const header = document.createElement('div');
        header.style.display = 'flex';
        header.style.alignItems = 'center';
        header.style.justifyContent = 'space-between';
        header.style.gap = '10px';

        const pageBadge = document.createElement('span');
        pageBadge.textContent = cardModel.pageLabel;
        pageBadge.style.display = 'inline-flex';
        pageBadge.style.width = 'fit-content';
        pageBadge.style.padding = '4px 8px';
        pageBadge.style.borderRadius = '999px';
        pageBadge.style.background = 'rgba(137, 180, 250, 0.16)';
        pageBadge.style.color = '#89b4fa';
        pageBadge.style.fontSize = '11px';
        pageBadge.style.fontWeight = '700';

        const location = document.createElement('span');
        location.textContent = cardModel.locationLabel;
        location.style.fontSize = '11px';
        location.style.color = '#6c7086';

        header.appendChild(pageBadge);
        header.appendChild(location);

        const body = document.createElement('div');
        body.textContent = comment.contents;
        body.style.color = '#cdd6f4';
        body.style.fontSize = '13px';
        body.style.lineHeight = '1.6';
        body.style.whiteSpace = 'pre-wrap';
        body.style.wordBreak = 'break-word';

        const actions = document.createElement('div');
        actions.style.display = 'flex';
        actions.style.alignItems = 'center';
        actions.style.justifyContent = 'space-between';
        actions.style.gap = '8px';

        const helper = document.createElement('span');
        helper.textContent = cardModel.helperLabel;
        helper.style.fontSize = '11px';
        helper.style.color = '#9399b2';

        const actionGroup = document.createElement('div');
        actionGroup.style.display = 'flex';
        actionGroup.style.gap = '8px';

        for (const action of cardModel.actions) {
            const button = document.createElement('button');
            button.type = 'button';
            button.textContent = action.label;
            applyReviewCardActionStyle(button, action.tone);
            button.addEventListener('click', (event) => {
                event.preventDefault();
                event.stopPropagation();
                handlers.onActionClick(action, comment.id);
            });
            actionGroup.appendChild(button);
        }

        actions.appendChild(helper);
        actions.appendChild(actionGroup);

        card.appendChild(header);
        card.appendChild(body);
        card.appendChild(actions);
        card.addEventListener('click', () => {
            handlers.onCardClick(comment.id);
        });
        nodes.list.appendChild(card);
    }
}

function renderSummary(
    container: HTMLElement | null,
    panel: PdfCommentReviewDisplay['panel'],
    onSummaryChipClick: (pageIndex: number) => void,
): void {
    if (!container) {
        return;
    }
    container.innerHTML = '';
    for (const chip of panel.summaryChips) {
        const button = document.createElement('button');
        button.type = 'button';
        button.textContent = chip.label;
        button.style.background = 'rgba(49, 50, 68, 0.95)';
        button.style.color = '#cdd6f4';
        button.style.border = '1px solid #45475a';
        button.style.borderRadius = '999px';
        button.style.padding = '4px 8px';
        button.style.cursor = 'pointer';
        button.style.fontSize = '11px';
        button.addEventListener('click', () => {
            onSummaryChipClick(chip.pageIndex);
        });
        container.appendChild(button);
    }
}

function applyReviewCardActionStyle(button: HTMLButtonElement, tone: string): void {
    button.style.borderRadius = '8px';
    button.style.padding = '6px 10px';
    button.style.cursor = 'pointer';
    if (tone === 'success') {
        button.style.background = 'rgba(166, 227, 161, 0.14)';
        button.style.color = '#a6e3a1';
        button.style.border = '1px solid rgba(166, 227, 161, 0.28)';
        return;
    }
    if (tone === 'danger') {
        button.style.background = 'rgba(243, 139, 168, 0.14)';
        button.style.color = '#f38ba8';
        button.style.border = '1px solid rgba(243, 139, 168, 0.28)';
        return;
    }
    button.style.background = 'rgba(137, 180, 250, 0.16)';
    button.style.color = '#89b4fa';
    button.style.border = '1px solid rgba(137, 180, 250, 0.32)';
}
