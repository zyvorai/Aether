import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  define: {
    __AETHER_DASHBOARD_BUILD__: JSON.stringify(
      process.env.AETHER_DASHBOARD_BUILD ?? 'dev'
    ),
  },
  server: {
    port: 5173,
    proxy: {
      '/api': 'http://localhost:5090',
      '/health': 'http://localhost:5090',
    },
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    // Stable filenames for embedding in the Rust binary (include_str!).
    cssCodeSplit: false,
    rollupOptions: {
      output: {
        inlineDynamicImports: true,
        entryFileNames: 'assets/aether-dashboard.js',
        assetFileNames: 'assets/aether-dashboard[extname]',
      },
    },
  },
});
