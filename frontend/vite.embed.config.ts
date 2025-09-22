import { defineConfig } from 'vite';
import { resolve } from 'path';

export default defineConfig({
  build: {
    lib: {
      entry: resolve(__dirname, 'src/lib/embed-sdk/index.ts'),
      name: 'ClayStudio',
      fileName: 'embed',
      formats: ['iife', 'es', 'cjs']
    },
    outDir: 'dist-embed',
    rollupOptions: {
      // Make sure to externalize deps that shouldn't be bundled
      external: [],
      output: {
        // Provide global variables for the IIFE build
        globals: {}
      }
    },
    minify: 'terser',
    sourcemap: true
  },
  define: {
    'process.env.NODE_ENV': JSON.stringify('production')
  }
});