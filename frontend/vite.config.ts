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
    rollupOptions: {
      output: {
        manualChunks(id) {
          // Handle node_modules
          if (id.includes('node_modules')) {
            // React router first (more specific)
            if (id.includes('react-router')) {
              return 'react-router';
            }
            // React core - keep react and react-dom together
            if (id.includes('react')) {
              return 'react';
            }
            
            // Charts
            if (id.includes('echarts') && !id.includes('echarts-for-react')) {
              return 'echarts-core';
            }
            if (id.includes('echarts-for-react')) {
              return 'echarts-react';
            }
            
            // UI libraries
            if (id.includes('@radix-ui/react-dialog') || id.includes('@radix-ui/react-dropdown') || id.includes('@radix-ui/react-popover')) {
              return 'radix-base';
            }
            if (id.includes('@radix-ui/react-select') || id.includes('@radix-ui/react-checkbox') || id.includes('@radix-ui/react-label') || id.includes('@radix-ui/react-switch')) {
              return 'radix-forms';
            }
            if (id.includes('@radix-ui')) {
              return 'radix-layout';
            }
            
            // Other vendors
            if (id.includes('valtio')) return 'state';
            if (id.includes('lucide-react') || id.includes('@radix-ui/react-icons')) return 'icons';
            if (id.includes('clsx') || id.includes('date-fns') || id.includes('class-variance-authority')) return 'utils';
            if (id.includes('@tanstack')) return 'tanstack';
            if (id.includes('axios')) return 'http';
            if (id.includes('react-hook-form')) return 'forms';
            if (id.includes('highlight.js') || id.includes('github-markdown-css')) return 'styling';
            
            return 'vendor';
          }
          
          // Split app code by feature to prevent large chunks
          if (id.includes('src/store/chat') || id.includes('src/lib/services/chat')) {
            return 'chat-core';
          }
          if (id.includes('src/hooks/chat') || id.includes('src/hooks/use-chat')) {
            return 'chat-hooks';
          }
          if (id.includes('src/components/chat/sidebar')) {
            return 'chat-sidebar';
          }
          if (id.includes('src/components/chat/display/message')) {
            return 'chat-messages';
          }
          if (id.includes('src/components/chat/display')) {
            return 'chat-display';  
          }
          if (id.includes('src/components/chat/input')) {
            return 'chat-input';
          }
          if (id.includes('src/components/data-table')) {
            return 'data-table';
          }
          if (id.includes('src/components/setup')) {
            return 'setup';
          }
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
            console.log('proxy error', err);
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