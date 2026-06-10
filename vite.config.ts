import { defineConfig } from 'vite';

export default defineConfig({
  base: './',
  server: {
    port: parseInt(process.env.PORT || '5000', 10),
    strictPort: true,
    host: '127.0.0.1',
  },
  // Pre-bundle deps to eliminate dev-mode white-screen caused by lazy dep discovery.
  // Vite's dep optimizer blocks page load when it discovers new deps during a request.
  // `entries` ensures ALL deps are scanned at server start — before the browser connects.
  optimizeDeps: {
    entries: ['./src/main.ts'],
    include: [
      '@tauri-apps/api/core',
      '@tauri-apps/plugin-dialog',
      '@tauri-apps/plugin-fs',
      '@tauri-apps/plugin-shell',
      // Note: CSS files cannot be pre-bundled by esbuild; FontAwesome is now inlined as SVG.
    ],
  },
  build: {
    target: 'esnext',
    outDir: 'dist',
  },
  worker: {
    format: 'es',
  },
});
