import type {
    PdfCommentOverlayDisplay,
    PdfCommentTargetOverlayDisplay,
} from './pdf_comment_contracts';

export function renderCommentOverlay(
    container: HTMLElement,
    overlay: PdfCommentOverlayDisplay,
    onCommentClick: (commentId: string) => void,
): void {
    container.innerHTML = '';

    for (const comment of overlay.comments) {
        const node = document.createElement('button');
        node.type = 'button';
        node.dataset.commentId = comment.id;
        node.title = comment.title;
        node.textContent = '💬';
        node.style.position = 'absolute';
        node.style.left = `${comment.frame.leftPercent}%`;
        node.style.top = `${comment.frame.topPercent}%`;
        node.style.width = `${comment.frame.widthPercent}%`;
        node.style.height = `${comment.frame.heightPercent}%`;
        node.style.minWidth = '18px';
        node.style.minHeight = '18px';
        node.style.borderRadius = '999px';
        node.style.border = comment.selected
            ? '2px solid rgba(249, 226, 175, 0.98)'
            : '1px solid rgba(137, 180, 250, 0.75)';
        node.style.background = comment.selected
            ? 'rgba(249, 226, 175, 0.28)'
            : 'rgba(137, 180, 250, 0.22)';
        node.style.color = '#cdd6f4';
        node.style.pointerEvents = 'auto';
        node.style.cursor = 'pointer';
        node.style.boxSizing = 'border-box';
        node.style.boxShadow = comment.selected
            ? '0 0 0 1px rgba(249, 226, 175, 0.3), 0 0 16px rgba(249, 226, 175, 0.28)'
            : 'none';
        node.addEventListener('click', (event) => {
            event.preventDefault();
            event.stopPropagation();
            onCommentClick(comment.id);
        });
        container.appendChild(node);
    }
}

export function renderCommentTargetOverlay(
    container: HTMLElement,
    enabled: boolean,
    targetDisplay: PdfCommentTargetOverlayDisplay,
    onTargetClick: (targetId: string) => void,
): void {
    container.innerHTML = '';
    container.style.pointerEvents = enabled ? 'auto' : 'none';
    if (!enabled) {
        return;
    }

    for (const target of targetDisplay.targets) {
        const node = document.createElement('div');
        node.dataset.commentTargetId = target.id;
        node.style.position = 'absolute';
        node.style.left = `${target.frame.leftPercent}%`;
        node.style.top = `${target.frame.topPercent}%`;
        node.style.width = `${target.frame.widthPercent}%`;
        node.style.height = `${target.frame.heightPercent}%`;
        node.style.borderRadius = '6px';
        node.style.background = 'rgba(137, 180, 250, 0.08)';
        node.style.border = '1px dashed rgba(137, 180, 250, 0.7)';
        node.style.boxSizing = 'border-box';
        node.style.cursor = 'pointer';
        node.title = target.title;
        node.addEventListener('click', (event) => {
            event.preventDefault();
            event.stopPropagation();
            onTargetClick(target.id);
        });
        container.appendChild(node);
    }
}
