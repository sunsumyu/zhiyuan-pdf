import net from 'node:net';
import { spawn } from 'node:child_process';
import { writeFileSync, unlinkSync, existsSync } from 'node:fs';
import { join } from 'node:path';

const defaultPort = parseInt(process.env.PORT || '5000', 10);
const overrideFilePath = join('src-tauri', 'tauri.port-override.json');

function checkPort(port) {
  return new Promise((resolve) => {
    const server = net.createServer();
    server.listen(port, '127.0.0.1', () => {
      server.once('close', () => resolve(true));
      server.close();
    });
    server.on('error', () => {
      resolve(false);
    });
  });
}

async function findFreePort(startPort) {
  let port = startPort;
  while (true) {
    console.log(`[Port Probe] Testing port ${port}...`);
    const available = await checkPort(port);
    if (available) {
      console.log(`[Port Probe] Port ${port} is available!`);
      return port;
    }
    console.log(`[Port Probe] Port ${port} is occupied or excluded. Trying next port...`);
    port++;
  }
}

async function run() {
  const isViteOnly = process.argv.includes('--vite-only');
  const port = await findFreePort(defaultPort);
  
  process.env.PORT = port.toString();

  let cleanupDone = false;
  const cleanup = () => {
    if (cleanupDone) return;
    cleanupDone = true;
    if (existsSync(overrideFilePath)) {
      try {
        unlinkSync(overrideFilePath);
        console.log(`\n[Dev Launcher] Cleaned up temporary config: ${overrideFilePath}`);
      } catch (err) {
        console.error(`[Dev Launcher] Failed to clean up ${overrideFilePath}:`, err.message);
      }
    }
  };

  // Register cleanup listeners
  process.on('exit', cleanup);
  process.on('SIGINT', () => {
    cleanup();
    process.exit(0);
  });
  process.on('SIGTERM', () => {
    cleanup();
    process.exit(0);
  });
  process.on('uncaughtException', (err) => {
    console.error('[Dev Launcher] Uncaught Exception:', err);
    cleanup();
    process.exit(1);
  });

  if (isViteOnly) {
    console.log(`[Dev Launcher] Starting Vite on port ${port}...`);
    const viteProcess = spawn('npx', ['vite', '--strictPort'], {
      stdio: 'inherit',
      shell: true,
      env: { ...process.env, PORT: port.toString() }
    });

    viteProcess.on('exit', (code) => {
      cleanup();
      process.exit(code || 0);
    });
  } else {
    // Generate the tauri configuration override
    console.log(`[Dev Launcher] Writing temporary Tauri override to ${overrideFilePath}...`);
    const overrideConfig = {
      build: {
        devUrl: `http://localhost:${port}`,
        beforeDevCommand: '' // Disable so Tauri dev doesn't spawn Vite again
      }
    };
    writeFileSync(overrideFilePath, JSON.stringify(overrideConfig, null, 2), 'utf8');

    console.log(`[Dev Launcher] Spawning Vite dev server on port ${port}...`);
    const viteProcess = spawn('npx', ['vite', '--strictPort'], {
      stdio: 'inherit',
      shell: true,
      env: { ...process.env, PORT: port.toString() }
    });

    // Wait 1.5 seconds for Vite to warm up before launching Tauri dev
    await new Promise((resolve) => setTimeout(resolve, 1500));

    console.log(`[Dev Launcher] Spawning Tauri dev client...`);
    const tauriProcess = spawn('npx', ['tauri', 'dev', '-c', overrideFilePath], {
      stdio: 'inherit',
      shell: true,
      env: { ...process.env, PORT: port.toString() }
    });

    // Handle process exits
    let exited = false;
    const handleExit = (code, source) => {
      if (exited) return;
      exited = true;
      console.log(`[Dev Launcher] ${source} exited. Shutting down...`);
      try {
        viteProcess.kill();
      } catch {}
      try {
        tauriProcess.kill();
      } catch {}
      cleanup();
      process.exit(code || 0);
    };

    viteProcess.on('exit', (code) => handleExit(code, 'Vite'));
    tauriProcess.on('exit', (code) => handleExit(code, 'Tauri'));
  }
}

const isShim = process.argv.includes('--shim');
if (isShim) {
  const subCommand = process.argv.find(arg => ['dev', 'build', 'info', 'icon', 'init', 'signer'].includes(arg));
  if (subCommand !== 'dev') {
    // If it's not dev (e.g. build, info, etc.), just forward directly to tauri CLI
    const args = process.argv.slice(2).filter(arg => arg !== '--shim');
    console.log(`[Tauri Shim] Forwarding command to Tauri CLI: npx tauri ${args.join(' ')}`);
    const tauriProcess = spawn('npx', ['tauri', ...args], { stdio: 'inherit', shell: true });
    tauriProcess.on('exit', (code) => process.exit(code || 0));
  } else {
    // It's a dev command, run our dynamic master dev launcher
    run().catch((err) => {
      console.error('[Dev Launcher] Critical Error:', err);
      process.exit(1);
    });
  }
} else {
  // Direct entry (e.g. npm run tauri:dev or npm run dev)
  run().catch((err) => {
    console.error('[Dev Launcher] Critical Error:', err);
    process.exit(1);
  });
}
