import { VECTOR_CONTAINER_ID } from '../render/vector_host';

export const DEFAULT_PAGE_WIDTH = 595;
export const DEFAULT_PAGE_HEIGHT = 842;
// Host-side safety bounds — must mirror Rust zoom_host::MIN_ZOOM / MAX_ZOOM.
// These are NOT domain rules; they are DOM safety clamps for the host layer.
export const MIN_ZOOM = 0.1;
export const MAX_ZOOM = 30.0;
export const MAX_CANVAS_DIM = 10240;

export type PdfZoomSnapshot = {
    targetZoom: number;
};

export function getWrapper(): HTMLElement | null {
    return document.getElementById('pdf-content-wrapper') as HTMLElement | null;
}

export function getScrollContainer(): HTMLElement | null {
    return document.getElementById('pdf-scroll-container') as HTMLElement | null;
}

export function getVectorContainer(): HTMLElement | null {
    return document.getElementById(VECTOR_CONTAINER_ID) as HTMLElement | null;
}

export function getRasterTarget(): HTMLCanvasElement | null {
    return document.getElementById('pdf-render-target') as HTMLCanvasElement | null;
}

export function getEmptyState(): HTMLElement | null {
    return document.getElementById('pdf-empty-state') as HTMLElement | null;
}

export function getPageIndicator(): HTMLElement | null {
    return document.getElementById('pdf-page-indicator') as HTMLElement | null;
}

export function getDynamicMaxZoom(): number {
    return MAX_ZOOM;
}

export function clampZoom(nextZoom: number): number {
    if (!Number.isFinite(nextZoom)) return 1.0;
    return Math.min(Math.max(nextZoom, MIN_ZOOM), MAX_ZOOM);
}

export function showDocumentWrapper(): void {
    const wrapper = getWrapper();
    const emptyState = getEmptyState();
    if (wrapper) wrapper.style.display = 'block';
    if (emptyState) emptyState.style.display = 'none';
}

export function showEmptyDocumentState(): void {
    const wrapper = getWrapper();
    const emptyState = getEmptyState();
    const pageIndicator = getPageIndicator();
    if (wrapper) {
        wrapper.style.display = 'none';
        wrapper.style.width = '';
        wrapper.style.height = '';
    }
    if (emptyState) emptyState.style.display = 'flex';
    if (pageIndicator) pageIndicator.textContent = 'Page 0 / 0';
}

export function syncZoomSelect(zoomState: PdfZoomSnapshot): void {
    const select = document.getElementById('pdf-zoom-select') as HTMLSelectElement | null;
    if (!select) return;

    const customOptionId = 'pdf-zoom-custom-option';
    const staleCustom = document.getElementById(customOptionId);
    if (staleCustom) staleCustom.remove();

    const exactOption = Array.from(select.options).find((option) => {
        const value = parseFloat(option.value);
        return Number.isFinite(value) && Math.abs(value - zoomState.targetZoom) < 0.0001;
    });

    if (exactOption) {
        select.value = exactOption.value;
    } else {
        const customOption = document.createElement('option');
        customOption.id = customOptionId;
        customOption.value = zoomState.targetZoom.toFixed(3);
        customOption.textContent = `${Math.round(zoomState.targetZoom * 100)}%`;
        select.appendChild(customOption);
        select.value = customOption.value;
    }

    select.title = `${Math.round(zoomState.targetZoom * 100)}%`;
}

export function syncTextEditButton(active: boolean): void {
    const btn = document.getElementById('pdf-add-text-btn') as HTMLElement | null;
    if (!btn) return;
    setToolbarButtonActive(btn, active);
}

export function syncEditorFormatButtons(format: {
    bold: boolean;
    italic: boolean;
    underline: boolean;
    color?: string;
    fontFamily?: string;
    fontSize?: number;
    charSpacing?: number;
    lineHeight?: number;
    paragraphMode?: string;
    alignment?: string;
    listKind?: string;
}): void {
    const boldButton = document.getElementById('pdf-format-bold-btn') as HTMLElement | null;
    const italicButton = document.getElementById('pdf-format-italic-btn') as HTMLElement | null;
    const underlineButton = document.getElementById('pdf-format-underline-btn') as HTMLElement | null;
    const colorInput = document.getElementById('pdf-format-color-input') as HTMLInputElement | null;
    const fontFamilySelect = document.getElementById('pdf-format-font-family') as HTMLSelectElement | null;
    const fontSizeInput = document.getElementById('pdf-format-font-size') as HTMLInputElement | null;
    const lineHeightInput = document.getElementById('pdf-format-line-height') as HTMLInputElement | null;
    const charSpacingInput = document.getElementById('pdf-format-char-spacing') as HTMLInputElement | null;
    const paragraphModeSelect = document.getElementById('pdf-format-paragraph-mode') as HTMLSelectElement | null;
    const alignLeftButton = document.getElementById('pdf-format-align-left-btn') as HTMLElement | null;
    const alignCenterButton = document.getElementById('pdf-format-align-center-btn') as HTMLElement | null;
    const alignRightButton = document.getElementById('pdf-format-align-right-btn') as HTMLElement | null;
    const alignJustifyButton = document.getElementById('pdf-format-align-justify-btn') as HTMLElement | null;
    const bulletButton = document.getElementById('pdf-format-list-bullet-btn') as HTMLElement | null;
    const numberingButton = document.getElementById('pdf-format-list-numbering-btn') as HTMLElement | null;
    const applyState = (button: HTMLElement | null, active: boolean) => {
        if (!button) return;
        setToolbarButtonActive(button, active);
    };
    applyState(boldButton, !!format.bold);
    applyState(italicButton, !!format.italic);
    applyState(underlineButton, !!format.underline);
    const alignment = (format.alignment ?? 'left').toLowerCase();
    const listKind = (format.listKind ?? 'none').toLowerCase();
    applyState(alignLeftButton, alignment === 'left');
    applyState(alignCenterButton, alignment === 'center');
    applyState(alignRightButton, alignment === 'right');
    applyState(alignJustifyButton, alignment === 'justify');
    applyState(bulletButton, listKind === 'bullet');
    applyState(numberingButton, listKind === 'numbering');
    if (colorInput && typeof format.color === 'string' && /^#([0-9a-f]{6})$/i.test(format.color)) {
        colorInput.value = format.color;
    }
    if (fontFamilySelect && typeof format.fontFamily === 'string' && format.fontFamily.trim()) {
        fontFamilySelect.value = format.fontFamily;
    }
    if (fontSizeInput && Number.isFinite(format.fontSize)) {
        fontSizeInput.value = `${Number(format.fontSize).toFixed(1).replace(/\.0$/, '')}`;
    }
    if (lineHeightInput && Number.isFinite(format.lineHeight)) {
        lineHeightInput.value = `${Number(format.lineHeight).toFixed(2)}`;
    }
    if (charSpacingInput && Number.isFinite(format.charSpacing)) {
        charSpacingInput.value = `${Number(format.charSpacing).toFixed(2)}`;
    }
    if (paragraphModeSelect && typeof format.paragraphMode === 'string' && format.paragraphMode) {
        paragraphModeSelect.value = format.paragraphMode;
    }
}

export function bindSaveFocusGuard(): void {
    const saveButton = document.getElementById('pdf-save-btn');
    if (!saveButton || saveButton.dataset.pdfSaveFocusGuard === '1') return;
    saveButton.dataset.pdfSaveFocusGuard = '1';
    const keepEditorFocus = (event: Event) => {
        event.preventDefault();
    };
    saveButton.addEventListener('pointerdown', keepEditorFocus, true);
    saveButton.addEventListener('mousedown', keepEditorFocus, true);
}

export function setToolbarButtonActive(button: HTMLElement | null, active: boolean): void {
    if (!button) return;
    button.classList.toggle('active', active);
    button.dataset.active = active ? 'true' : 'false';
    button.setAttribute('aria-pressed', active ? 'true' : 'false');
}

