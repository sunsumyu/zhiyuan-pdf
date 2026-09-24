export type ZoomTestOutcome = 'PASS' | 'FAIL' | 'NOT_RUN';

export interface ZoomTestEvent {
  seq: number;
  event: string;
  case_id: string;
  module?: string;
  method?: string;
  frame_token?: number;
  [key: string]: unknown;
}

export class ZoomTestTraceCollector {
  private sequence = 0;
  private readonly events: ZoomTestEvent[] = [];

  constructor(
    private readonly caseId: string,
    private readonly module: string,
    private readonly method: string,
  ) {}

  emit(event: string, details: Record<string, unknown> = {}): ZoomTestEvent {
    const entry: ZoomTestEvent = {
      seq: ++this.sequence,
      event,
      case_id: this.caseId,
      module: this.module,
      method: this.method,
      ...details,
    };
    this.events.push(entry);
    return entry;
  }

  caseStart(input: unknown): void {
    this.emit('zoom.test.case-start');
    this.emit('zoom.test.input', { input });
  }

  stateBefore(state: unknown): void {
    this.emit('zoom.test.state-before', { state });
  }

  decision(decision: unknown): void {
    this.emit('zoom.test.decision', { decision });
  }

  stateAfter(state: unknown): void {
    this.emit('zoom.test.state-after', { state });
  }

  assertion(expected: unknown, actual: unknown, outcome: ZoomTestOutcome): void {
    this.emit('zoom.test.assertion', { expected, actual, outcome });
  }

  caseEnd(outcome: ZoomTestOutcome): void {
    this.emit('zoom.test.case-end', { outcome });
  }

  snapshot(): readonly ZoomTestEvent[] {
    return this.events.map((event) => ({ ...event }));
  }

  toJsonLines(): string {
    return this.events.map((event) => JSON.stringify(event)).join('\n');
  }
}
