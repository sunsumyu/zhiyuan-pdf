// Tail Chrome DevTools Protocol console events to stdout / file.
// Usage: node tools/cdp-tail.mjs [outputFile]
import { writeFileSync, appendFileSync } from 'node:fs';

const outFile = process.argv[2] || 'console-tail.log';
writeFileSync(outFile, `# CDP tail started ${new Date().toISOString()}\n`);

const targets = await fetch('http://127.0.0.1:9222/json/list').then(r => r.json());
const page = targets.find(t => t.type === 'page' && t.url.startsWith('http://localhost:5000'));
if (!page) {
    console.error('No page target. Targets:', targets.map(t => `${t.type}:${t.url}`));
    process.exit(1);
}
console.error('Connecting to', page.webSocketDebuggerUrl);

const ws = new WebSocket(page.webSocketDebuggerUrl);
let nextId = 1;
const send = (method, params = {}) => {
    ws.send(JSON.stringify({ id: nextId++, method, params }));
};

ws.addEventListener('open', () => {
    send('Runtime.enable');
    send('Log.enable');
    send('Console.enable');
    appendFileSync(outFile, '# Domains enabled, listening...\n');
});

ws.addEventListener('message', (ev) => {
    const msg = JSON.parse(ev.data);
    if (msg.method === 'Runtime.consoleAPICalled') {
        const { type, args, stackTrace } = msg.params;
        const parts = (args || []).map(a => {
            if (a.value !== undefined) return JSON.stringify(a.value);
            if (a.unserializableValue) return a.unserializableValue;
            if (a.preview) return JSON.stringify(a.preview);
            if (a.description) return a.description;
            return JSON.stringify(a);
        });
        const loc = stackTrace?.callFrames?.[0];
        const locStr = loc ? ` @${loc.url.split('/').pop()}:${loc.lineNumber}` : '';
        appendFileSync(outFile, `[${type}]${locStr} ${parts.join(' ')}\n`);
    } else if (msg.method === 'Runtime.exceptionThrown') {
        const e = msg.params.exceptionDetails;
        appendFileSync(outFile, `[exception] ${e.text} ${e.exception?.description || ''}\n`);
    } else if (msg.method === 'Log.entryAdded') {
        const e = msg.params.entry;
        appendFileSync(outFile, `[log:${e.level}:${e.source}] ${e.text}\n`);
    }
});

ws.addEventListener('error', (e) => {
    appendFileSync(outFile, `# WS error: ${e.message || e}\n`);
});
ws.addEventListener('close', () => {
    appendFileSync(outFile, '# WS closed\n');
    process.exit(0);
});

// Keep alive
setInterval(() => {}, 1 << 30);
