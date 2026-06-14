import * as os from 'node:os';
import * as path from 'node:path';
import { existsSync } from 'node:fs';
import { spawn, spawnSync } from 'node:child_process';
import type { Capabilities as Caps, Options } from '@wdio/types';

// 本目录的 package.json 设了 "type": "commonjs" 覆盖根项目的 ESM，
// 因此这里用 CommonJS 的 __dirname 即可，不需要从 import.meta.url 推导。

/**
 * Tauri v2 + WebdriverIO v7 + tauri-driver 2.0.6 配置。
 *
 * 链路：wdio (4444) → tauri-driver → msedgedriver → tauri 应用的 WebView2
 *
 * 注意 wdio v8/v9 与 tauri-driver 不兼容。
 * 参考社区可工作示例：https://github.com/Haprog/tauri-wdio-win-test
 *
 * 前置条件：
 *   1. `cargo install tauri-driver --locked`
 *   2. `tools/msedgedriver/msedgedriver.exe` 与本机 Edge 主版本号匹配
 *   3. 已经 `npm run e2e:build`，即在 `target/debug/pdf-viewer-standalone.exe` 有产物
 */

const repoRoot = path.resolve(__dirname, '..', '..');
const releaseBinary = path.join(repoRoot, 'target', 'release', 'pdf-viewer-standalone.exe');
const debugBinary = path.join(repoRoot, 'target', 'debug', 'pdf-viewer-standalone.exe');
// `npm run e2e:build` 生成 debug 产物，默认用它，避免本地旧 release 干扰回归测试。
const appBinary = existsSync(debugBinary) ? debugBinary : releaseBinary;
const msedgedriverPath = path.join(repoRoot, 'tools', 'msedgedriver', 'msedgedriver.exe');
// 4444/4445 and several nearby ranges can fall inside Windows excluded TCP ranges on some machines.
const tauriDriverPort = 5210;
const nativeDriverPort = 5211;

if (!existsSync(appBinary)) {
    throw new Error(
        `Tauri binary not found:\n  ${releaseBinary}\n  ${debugBinary}\n请先运行 \`npm run e2e:build\`。`,
    );
}
if (!existsSync(msedgedriverPath)) {
    throw new Error(`msedgedriver not found at ${msedgedriverPath}`);
}

// 让 wdio 接受 tauri-driver 期待的非标准 capability shape。
interface CustomCaps {
    'tauri:options': {
        application?: string;
        webviewOptions?: Record<string, unknown>;
    };
}
type CapItem = (Caps.DesiredCapabilities & CustomCaps) | (Caps.W3CCapabilities & CustomCaps);
type CustomConfig = Omit<Options.Testrunner, 'capabilities'> & {
    capabilities: CapItem[];
};

let tauriDriverProc: ReturnType<typeof spawn> | null = null;

export const config: CustomConfig = {
    runner: 'local',
    autoCompileOpts: {
        autoCompile: true,
        tsNodeOpts: {
            transpileOnly: true,
            project: path.join(__dirname, 'tsconfig.json'),
        },
    },
    specs: [path.join(__dirname, 'specs', 'load_pdf.spec.ts')],
    maxInstances: 1,
    capabilities: [
        {
            maxInstances: 1,
            'tauri:options': {
                application: appBinary,
                webviewOptions: {},
            },
        },
    ],
    logLevel: 'info',
    bail: 0,
    waitforTimeout: 10_000,
    connectionRetryTimeout: 120_000,
    connectionRetryCount: 3,
    hostname: '127.0.0.1',
    port: tauriDriverPort,
    framework: 'mocha',
    reporters: ['spec'],
    mochaOpts: {
        ui: 'bdd',
        timeout: 60_000,
    },

    // 在 session 创建之前启动 tauri-driver。它会把 capabilities[0]['tauri:options'].application
    // 拿出来翻译给 msedgedriver 启动 WebView2 应用。
    beforeSession(): void {
        const tauriDriverBin = path.resolve(os.homedir(), '.cargo', 'bin', 'tauri-driver.exe');
        if (!existsSync(tauriDriverBin)) {
            throw new Error(
                `tauri-driver not found at ${tauriDriverBin}. 请运行 \`cargo install tauri-driver --locked\`。`,
            );
        }
        tauriDriverProc = spawn(
            tauriDriverBin,
            [
                '--port',
                String(tauriDriverPort),
                '--native-port',
                String(nativeDriverPort),
                '--native-driver',
                msedgedriverPath,
            ],
            { stdio: ['ignore', 'inherit', 'inherit'] },
        );
        tauriDriverProc.on('error', (err) => {
            console.error('[wdio] tauri-driver spawn error:', err);
        });
    },

    afterSession(): void {
        if (tauriDriverProc && !tauriDriverProc.killed) {
            tauriDriverProc.kill();
            tauriDriverProc = null;
        }
        // Best-effort：杀掉孤儿进程。
        try {
            spawnSync('taskkill', ['/F', '/IM', 'tauri-driver.exe'], { stdio: 'ignore' });
            spawnSync('taskkill', ['/F', '/IM', 'msedgedriver.exe'], { stdio: 'ignore' });
            spawnSync('taskkill', ['/F', '/IM', 'pdf-viewer-standalone.exe'], { stdio: 'ignore' });
        } catch {
            // ignore
        }
    },
};
