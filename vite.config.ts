import { defineConfig } from 'vite';

export default defineConfig({
  server: {
    port: 3000,
    strictPort: true,
  },
  // Pre-bundle internal bridge modules to eliminate dev-mode module waterfall.
  // Without this, Vite serves each .ts file as a separate HTTP request, causing
  // 50+ sequential round-trips on Windows localhost (~10-100ms each = seconds).
  optimizeDeps: {
    include: [
      '@tauri-apps/api/core',
      '@tauri-apps/plugin-dialog',
      '@tauri-apps/plugin-fs',
      '@tauri-apps/plugin-shell',
      '@fortawesome/fontawesome-free/css/all.min.css',
    ],
  },
  build: {
    target: 'esnext',
    outDir: 'dist',
  },
});
