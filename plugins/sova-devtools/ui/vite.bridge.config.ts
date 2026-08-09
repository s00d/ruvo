import { defineConfig } from "vite";
import { resolve } from "node:path";

/** Host-page bridge only (no Vue). */
export default defineConfig({
  define: {
    "process.env.NODE_ENV": JSON.stringify("production"),
  },
  build: {
    outDir: resolve(__dirname, "../assets"),
    emptyOutDir: false,
    lib: {
      entry: resolve(__dirname, "src/bridge.ts"),
      formats: ["iife"],
      name: "SovaDevToolsBridge",
      fileName: () => "bridge",
    },
    rollupOptions: {
      output: {
        entryFileNames: "bridge.js",
        assetFileNames: "bridge[extname]",
      },
    },
  },
});
