import path from 'path';
import { defineConfig } from 'vite';

export default defineConfig(() => {
  return {
    define: {
      'import.meta.env.VITE_NETWORK_NAME': JSON.stringify(process.env.VITE_NETWORK_NAME || process.env.NETWORK_NAME),
    },
    resolve: {
      alias: {
        '@': path.resolve(__dirname, '.'),
      },
    },
    build: {
      chunkSizeWarningLimit: 1000,
      rollupOptions: {
        output: {
          manualChunks: {
            react: ['react', 'react-dom'],
            charts: ['recharts'],
          },
          assetFileNames: (assetInfo) => {
            if (assetInfo.name?.endsWith('.ttf')) {
              return 'fonts/[name].[hash][extname]';
            }
            return 'assets/[name].[hash][extname]';
          },
        },
      },
    },
    assetsInclude: ['**/*.ttf'],
  };
});
