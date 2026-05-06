export type ResumeRegionKind = 'paragraph-region' | 'list-item-region';

export type ResumeChatRole = 'user' | 'assistant';

export type ResumeAiScope = 'current-page' | 'whole-document';

export interface ResumeChatTurn {
  role: ResumeChatRole;
  text: string;
}

export interface PdfPersistableRegionPatch {
  patchKey: string;
  pageIndex: number;
  regionId: string;
  originalText: string;
  newText: string;
  source: ResumeRegionKind;
  snapshot: Record<string, unknown>;
  targetIndices: number[];
  fullTargetIndices?: number[];
  wrapWidth?: number;
  kind?: string;
  markerText?: string;
  newMarkerText?: string;
}

export interface ResumeAiEditDraft {
  pageIndex: number;
  regionId: string;
  summary: string;
  replacement: string;
  confidence?: number;
}

export interface ResumeAiPlan {
  reply: string;
  edits: ResumeAiEditDraft[];
}

export interface ResumeAiPlanResult {
  reply: string;
  suggestions: ResumeAiSuggestion[];
  warnings?: string[];
}

export interface ResumeAiThreadView {
  path?: string;
  currentPage?: number;
  scope: ResumeAiScope;
  turns: ResumeChatTurn[];
  suggestions: ResumeAiSuggestion[];
  phase: 'idle' | 'planning' | 'applying-one' | 'applying-all' | 'failed';
  busy: boolean;
  notice?: string;
}

export interface ResumeAiSuggestion {
  id: string;
  path: string;
  pageIndex: number;
  regionId: string;
  kind: ResumeRegionKind;
  summary: string;
  originalText: string;
  suggestedText: string;
  confidence: number;
  patch: PdfPersistableRegionPatch;
  state: 'pending' | 'applied' | 'failed';
  errorMessage?: string;
}

export interface RawParagraphRegionLine {
  text?: string;
  renderedText?: string;
  objectIndices?: number[];
  [key: string]: unknown;
}

export interface RawParagraphRegion {
  id: string;
  text: string;
  wrapWidth?: number;
  objectIndices?: number[];
  lines?: RawParagraphRegionLine[];
  [key: string]: unknown;
}

export interface RawListItemRegion {
  id: string;
  text: string;
  bodyText?: string;
  markerText?: string;
  wrapWidth?: number;
  objectIndices?: number[];
  [key: string]: unknown;
}

export interface RawPageRegionContext {
  sceneHint?: string;
  paragraphRegions?: RawParagraphRegion[];
  listItemRegions?: RawListItemRegion[];
}

export interface ResumeEditableRegion {
  id: string;
  kind: ResumeRegionKind;
  pageIndex: number;
  text: string;
  summary: string;
  objectIndices: number[];
  wrapWidth?: number;
  snapshot: Record<string, unknown>;
  markerText?: string;
  bodyText?: string;
  originalLineTexts?: string[];
}

export interface ResumePageContext {
  path: string;
  pageIndex: number;
  sceneHint: string;
  editableRegions: ResumeEditableRegion[];
}

export interface ResumeDocumentContext {
  path: string;
  scope: ResumeAiScope;
  pageContexts: ResumePageContext[];
  editableRegions: ResumeEditableRegion[];
  sceneHint: string;
}
