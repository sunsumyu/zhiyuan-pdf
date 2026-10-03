// CPU profile the zoom gesture via CDP (ADR-0012 follow-up: locate the
// remaining ~100ms zoom-independent per-wheel-step main-thread block).
//
// Usage: node tools/cdp-profile.mjs [outPrefix]
// Requires the app running with WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS
// containing --remote-debugging-port=9222.
import { writeFileSync } from 'node:fs';

const prefix = process.argv[2] || 'scratch/cpu_profile';
const CDP_PORT = process.env.CDP_PORT || '9333';
const FIXTURE = 'F:/chain/pdf-viewer-standalone/tests/e2e/fixtures/multipage.pdf';

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

const targets = await fetch(`http://127.0.0.1:${CDP_PORT}/json/list`).then((r) => r.json());
const page = targets.find((t) => t.type === 'page' && !t.url.startsWith('devtools'));
if (!page) {
    console.error('No page target. Targets:', targets.map((t) => `${t.type}:${t.url}`));
    process.exit(1);
}
console.error('Connecting to', page.url);

const ws = new WebSocket(page.webSocketDebuggerUrl);
let nextId = 1;
const pending = new Map();
const events = [];
const send = (method, params = {}) =>
    new Promise((resolve, reject) => {
        const id = nextId++;
        pending.set(id, { resolve, reject });
        ws.send(JSON.stringify({ id, method, params }));
    });

ws.addEventListener('message', (ev) => {
    const msg = JSON.parse(ev.data);
    if (msg.id && pending.has(msg.id)) {
        const { resolve, reject } = pending.get(msg.id);
        pending.delete(msg.id);
        msg.error ? reject(new Error(msg.error.message)) : resolve(msg.result);
        return;
    }
    if (msg.method) events.push(msg);
});

await new Promise((resolve, reject) => {
    ws.addEventListener('open', resolve, { once: true });
    ws.addEventListener('error', reject, { once: true });
});

const evaluate = async (expression, awaitPromise = false) => {
    const r = await send('Runtime.evaluate', {
        expression,
        awaitPromise,
        returnByValue: true,
    });
    if (r.exceptionDetails) {
        throw new Error('eval failed: ' + JSON.stringify(r.exceptionDetails).slice(0, 400));
    }
    return r.result?.value;
};

await send('Runtime.enable');
await send('Page.enable');

// ── 1. wait for the app shell, then load the fixture ──
for (let i = 0; i < 60; i++) {
    const ready = await evaluate(`!!document.getElementById('pdf-viewer-root')`).catch(() => false);
    if (ready) break;
    await sleep(500);
}
await evaluate(`window.__PDF_DIAGNOSTICS_VERBOSE = true; window.__PDF_LAYOUT_TRACE_VERBOSE = true;`);
await evaluate(`window.openPdfFile(${JSON.stringify(FIXTURE)})`, true);
await sleep(2500);

// ── 2. install the rAF-gap sampler ──
await evaluate(`(() => {
    const w = window;
    w.__prof = { frames: [], done: false };
    const t0 = performance.now();
    const sample = () => {
        if (w.__prof.done) return;
        const zs = w.wasmv3?.readZoomState?.();
        w.__prof.frames.push({
            ms: Math.round(performance.now() - t0),
            v: zs ? +zs.visualZoom.toFixed(3) : -1,
        });
        requestAnimationFrame(sample);
    };
    requestAnimationFrame(sample);
    return true;
})()`);

// ── 3. profile the gesture ──
await send('Profiler.enable');
await send('Profiler.setSamplingInterval', { interval: 200 }); // µs — fine grain
await send('Profiler.start');

await evaluate(`(() => {
    const scroller = document.getElementById('pdf-scroll-container');
    const r = scroller.getBoundingClientRect();
    const fire = (dy) => scroller.dispatchEvent(new WheelEvent('wheel', {
        bubbles: true, cancelable: true, ctrlKey: true,
        deltaY: dy, clientX: r.left + r.width / 2, clientY: r.top + r.height / 2,
    }));
    let i = 0;
    const timer = setInterval(() => { fire(-120); i += 1; if (i >= 16) clearInterval(timer); }, 120);
    return true;
})()`);

await sleep(16 * 120 + 3000);
await evaluate(`window.__prof.done = true;`);
const frames = await evaluate(`window.__prof.frames`);
const { profile } = await send('Profiler.stop');

writeFileSync(`${prefix}.cpuprofile`, JSON.stringify(profile));
writeFileSync(`${prefix}.frames.json`, JSON.stringify(frames));
console.error(`profile written: ${prefix}.cpuprofile (${profile.nodes.length} nodes, ${profile.samples.length} samples), ${frames.length} frames`);
ws.close();
process.exit(0);
