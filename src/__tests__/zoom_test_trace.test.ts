import { describe, expect, it } from 'vitest';
import { ZoomTestTraceCollector } from './zoom_test_trace';

describe('ZoomTestTraceCollector', () => {
  it('records ordered JSONL lifecycle events with structured evidence', () => {
    const trace = new ZoomTestTraceCollector('Z-TRACE-001', 'test', 'collector');
    trace.caseStart({ delta_y: -80 });
    trace.stateBefore({ target_zoom: 1, visual_zoom: 1, last_rendered_zoom: 1 });
    trace.decision({ action: 'advance', frame_token: 7 });
    trace.stateAfter({ target_zoom: 1.1, visual_zoom: 1.05, last_rendered_zoom: 1 });
    trace.assertion({ settled: false }, { settled: false }, 'PASS');
    trace.caseEnd('PASS');

    const events = trace.snapshot();
    expect(events.map((event) => event.seq)).toEqual([1, 2, 3, 4, 5, 6, 7]);
    expect(events.map((event) => event.event)).toEqual([
      'zoom.test.case-start',
      'zoom.test.input',
      'zoom.test.state-before',
      'zoom.test.decision',
      'zoom.test.state-after',
      'zoom.test.assertion',
      'zoom.test.case-end',
    ]);
    expect(events[3]).toMatchObject({ decision: { frame_token: 7 } });
    expect(events[5]).toMatchObject({ outcome: 'PASS' });
    expect(trace.toJsonLines().split('\n')).toHaveLength(7);
  });

  it('preserves explicit FAIL and NOT_RUN outcomes', () => {
    const trace = new ZoomTestTraceCollector('Z-TRACE-002', 'test', 'outcome');
    trace.assertion({ value: 1 }, { value: 2 }, 'FAIL');
    trace.caseEnd('NOT_RUN');

    expect(trace.snapshot()).toEqual([
      expect.objectContaining({ event: 'zoom.test.assertion', outcome: 'FAIL' }),
      expect.objectContaining({ event: 'zoom.test.case-end', outcome: 'NOT_RUN' }),
    ]);
  });
});
