import path from "path";
import { defineConfig, loadEnv } from "vite";

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, ".", "");
  return {
    define: {
      "process.env.API_KEY": JSON.stringify(env.API_KEY),
    },
    resolve: {
      alias: {
        "@": path.resolve(__dirname, "."),
      },
    },
    build: {
      chunkSizeWarningLimit: 1000,
      rollupOptions: {
        output: {
          manualChunks: {
            react: ["react", "react-dom"],
            charts: ["recharts"],
          },
        },
      },
    },
  };
});
