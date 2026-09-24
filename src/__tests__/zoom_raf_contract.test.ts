import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { ZoomTestTraceCollector } from './zoom_test_trace';

const rafSource = readFileSync(
  resolve(process.cwd(), 'crates/pdf-viewer-ui/src/zoom/raf_loop.rs'),
  'utf8',
);
const committedSource = readFileSync(
  resolve(process.cwd(), 'crates/pdf-viewer-ui/src/zoom/raf_committed.rs'),
  'utf8',
);

describe('zoom RAF and SetBox contracts', () => {
  it('keeps RAF start idempotent and stop cleanup complete', () => {
    const trace = new ZoomTestTraceCollector('Z-RAF-001', 'ui', 'raf_lifecycle');
    trace.caseStart({ start: 2, stop: 2 });
    trace.stateBefore({ source: 'raf_loop.rs' });
    const startGuard = /if is_raf_loop_running\(\)\s*\{\s*return;\s*\}/s.test(rafSource);
    const cleanup = ['RAF_HANDLE', 'RAF_CLOSURE', 'clear_dom_cache'].every((name) =>
      rafSource.includes(name),
    );
    trace.decision({ startGuard, cleanup });
    trace.assertion({ startGuard: true, cleanup: true }, { startGuard, cleanup }, startGuard && cleanup ? 'PASS' : 'FAIL');
    trace.caseEnd(startGuard && cleanup ? 'PASS' : 'FAIL');
    expect(startGuard).toBe(true);
    expect(cleanup).toBe(true);
  });

  it('keeps wheel wake-up and settled SetBox ownership explicit', () => {
    const trace = new ZoomTestTraceCollector('Z-RAF-002', 'ui', 'setbox_contract');
    trace.caseStart({ wheel: 'ensure_raf_loop_after_wheel', settled: true });
    const wheelWake = /pub fn ensure_raf_loop_after_wheel\(\)[\s\S]*?start_zoom_raf_loop\(\)/.test(rafSource);
    const scrollAlwaysApplied = /\/\/ Always set scroll position[\s\S]*?set_scroll_left[\s\S]*?set_scroll_top/.test(committedSource);
    const geometryGuarded = /if !wheel_gesture_active && \(settled \|\| !in_gesture\)/.test(committedSource);
    const actual = { wheelWake, scrollAlwaysApplied, geometryGuarded };
    const pass = Object.values(actual).every(Boolean);
    trace.decision(actual);
    trace.assertion({ wheelWake: true, scrollAlwaysApplied: true, geometryGuarded: true }, actual, pass ? 'PASS' : 'FAIL');
    trace.caseEnd(pass ? 'PASS' : 'FAIL');
    expect(actual).toEqual({ wheelWake: true, scrollAlwaysApplied: true, geometryGuarded: true });
  });

  it('scopes the wheel gesture flag to raf_loop with a read-only accessor', () => {
    const trace = new ZoomTestTraceCollector('Z-RAF-003', 'ui', 'gesture_flag_scope');
    const storeSource = readFileSync(
      resolve(process.cwd(), 'crates/pdf-viewer-ui/src/zoom/zoom_store.rs'),
      'utf8',
    );
    // The flag lives in raf_loop (the only writer) as a private thread_local…
    const privatelyOwned =
      /static WHEEL_GESTURE_ACTIVE: RefCell<bool>/.test(rafSource) &&
      !/pub static WHEEL_GESTURE_ACTIVE/.test(rafSource);
    // …is never re-exported from the shared zoom store…
    const absentFromStore = !storeSource.includes('WHEEL_GESTURE_ACTIVE');
    // …and the commit path only ever touches it through the narrow accessor.
    const readOnlyAccess =
      /pub\(super\) fn is_wheel_gesture_active\(\)/.test(rafSource) &&
      /is_wheel_gesture_active\(\)/.test(committedSource) &&
      !committedSource.includes('WHEEL_GESTURE_ACTIVE');
    const actual = { privatelyOwned, absentFromStore, readOnlyAccess };
    const pass = Object.values(actual).every(Boolean);
    trace.decision(actual);
    trace.assertion(
      { privatelyOwned: true, absentFromStore: true, readOnlyAccess: true },
      actual,
      pass ? 'PASS' : 'FAIL',
    );
    trace.caseEnd(pass ? 'PASS' : 'FAIL');
    expect(actual).toEqual({ privatelyOwned: true, absentFromStore: true, readOnlyAccess: true });
  });
});
