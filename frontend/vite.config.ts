import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import path from 'path'

// https://vitejs.dev/config/
export default defineConfig({
  clearScreen: false,
  plugins: [react()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  build: {
    chunkSizeWarningLimit: 2000,
    rollupOptions: {
      output: {
        manualChunks: {
          'react-vendor': ['react', 'react-dom'],
          'react-router': ['react-router-dom'],
          'valtio': ['valtio'],
          'axios': ['axios'],
          'tanstack': ['@tanstack/react-query', '@tanstack/react-table'],
          'radix': ['@radix-ui/react-dialog', '@radix-ui/react-dropdown-menu', '@radix-ui/react-select'],
          'echarts': ['echarts', 'echarts-for-react'],
          'utils': ['clsx', 'date-fns', 'class-variance-authority']
        },
      },
    },
  },
  server: {
    port: 7690,
    host: '0.0.0.0',
    proxy: {
      '/api/ws': {
        target: 'ws://127.0.0.1:7680',
        ws: true,
        changeOrigin: true,
      },
      '/api': {
        target: 'http://127.0.0.1:7680',
        changeOrigin: true,
        secure: false,
        configure: (proxy, _options) => {
          proxy.on('error', (err, _req, _res) => {
          });
          proxy.on('proxyReq', (proxyReq, req, _res) => {
          });
          proxy.on('proxyRes', (proxyRes, req, _res) => {
          });
        },
      },
    },
  },
})