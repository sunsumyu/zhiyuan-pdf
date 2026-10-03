// Count console volume during a wheel gesture (ADR-0013 diagnosis).
const CDP_PORT = process.env.CDP_PORT || '9333';
const FIXTURE = 'F:/chain/pdf-viewer-standalone/tests/e2e/fixtures/multipage.pdf';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const targets = await fetch(`http://127.0.0.1:${CDP_PORT}/json/list`).then(r => r.json());
const page = targets.find(t => t.type === 'page' && !t.url.startsWith('devtools'));
const ws = new WebSocket(page.webSocketDebuggerUrl);
let nextId = 1; const pending = new Map(); let logCount = 0; const byPrefix = new Map();
const send = (m, p = {}) => new Promise((res, rej) => { const id = nextId++; pending.set(id, { res, rej }); ws.send(JSON.stringify({ id, method: m, params: p })); });
ws.addEventListener('message', ev => {
    const msg = JSON.parse(ev.data);
    if (msg.id && pending.has(msg.id)) { const { res, rej } = pending.get(msg.id); pending.delete(msg.id); msg.error ? rej(new Error(msg.error.message)) : res(msg.result); return; }
    if (msg.method === 'Runtime.consoleAPICalled') {
        logCount++;
        const text = (msg.params.args || []).map(a => a.value ?? a.description ?? '').join(' ');
        const pre = (text.match(/^\[[A-Za-z-]+\]/) || ['(none)'])[0];
        byPrefix.set(pre, (byPrefix.get(pre) || 0) + 1);
    }
});
await new Promise(r => ws.addEventListener('open', r, { once: true }));
const evaluate = async (expression, awaitPromise = false) => {
    const r = await send('Runtime.evaluate', { expression, awaitPromise, returnByValue: true });
    if (r.exceptionDetails) throw new Error(JSON.stringify(r.exceptionDetails).slice(0, 300));
    return r.result?.value;
};
await send('Runtime.enable');
for (let i = 0; i < 40; i++) { if (await evaluate(`!!document.getElementById('pdf-viewer-root')`).catch(() => false)) break; await sleep(500); }
await evaluate(`window.openPdfFile(${JSON.stringify(FIXTURE)})`, true);
await sleep(2500);
logCount = 0; byPrefix.clear();
await evaluate(`(() => {
    const scroller = document.getElementById('pdf-scroll-container');
    const r = scroller.getBoundingClientRect();
    let i = 0;
    const timer = setInterval(() => {
        scroller.dispatchEvent(new WheelEvent('wheel', { bubbles: true, cancelable: true, ctrlKey: true, deltaY: -120, clientX: r.left + r.width / 2, clientY: r.top + r.height / 2 }));
        if (++i >= 16) clearInterval(timer);
    }, 120);
})()`);
await sleep(16 * 120 + 2500);
console.log(`console messages during gesture: ${logCount}`);
for (const [k, v] of [...byPrefix.entries()].sort((a, b) => b[1] - a[1]).slice(0, 12)) console.log(`  ${String(v).padStart(6)}  ${k}`);
ws.close(); process.exit(0);
