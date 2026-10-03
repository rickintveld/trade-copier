import { defineConfig } from "vite";
import react from "@vitejs/plugin-react-swc";
import path from "path";

// https://vitejs.dev/config/
export default defineConfig(({ mode }) => ({
  // Tauri expects a fixed port in dev mode
  server: {
    host: "localhost",
    port: 1420,
    strictPort: true,
  },
  // Tauri uses a different base path for production
  base: "./",
  plugins: [react()],
  resolve: {
    alias: {
      "@": path.resolve(import.meta.dirname, "./frontend"),
    },
  },
  // Prevent vite from obscuring rust errors
  clearScreen: false,
  // Enable env variables with TAURI_ prefix
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    // Tauri uses Chromium on Windows and WebKit on macOS and Linux
    target: process.env.TAURI_ENV_PLATFORM == "windows" ? "chrome105" : "safari13",
    // Don't minify for debug builds
    minify: !process.env.TAURI_ENV_DEBUG,
    // Produce sourcemaps for debug builds
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (!id.includes("node_modules")) return;
          if (/[\\/]node_modules[\\/](react|react-dom|react-router|scheduler)[\\/]/.test(id)) return "vendor-react";
          if (id.includes("@radix-ui")) return "vendor-radix";
          if (/[\\/]node_modules[\\/](recharts|d3-[^\\/]+|victory-vendor)[\\/]/.test(id)) return "vendor-charts";
          if (/[\\/]node_modules[\\/](date-fns|@tanstack)[\\/]/.test(id)) return "vendor-utils";
        },
      },
    },
  },
}));
