import { defineConfig } from "vite";
import { resolve } from "node:path";

// Vite config for the Tauri frontend.
// See https://v2.tauri.app/start/frontend/vite/
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  root: "src",
  publicDir: false,
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? { protocol: "ws", host, port: 1421 }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**", "**/sidecars/**"]
    }
  },
  build: {
    outDir: "../dist",
    emptyOutDir: true,
    target: ["es2022", "chrome110", "safari16"],
    minify: "esbuild",
    sourcemap: !!process.env.TAURI_DEBUG
  },
  resolve: {
    alias: {
      "@": resolve(__dirname, "src")
    }
  }
});
