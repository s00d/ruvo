import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";
import { resolve } from "node:path";

/** Full DevTools SPA (dock iframe + pop-out window). */
export default defineConfig({
  plugins: [vue(), tailwindcss()],
  base: "/_devtools/assets/",
  define: {
    "process.env.NODE_ENV": JSON.stringify("production"),
  },
  build: {
    outDir: resolve(__dirname, "../assets"),
    emptyOutDir: true,
    cssCodeSplit: false,
    assetsInlineLimit: 200_000,
    rollupOptions: {
      input: resolve(__dirname, "index.html"),
      output: {
        inlineDynamicImports: true,
        entryFileNames: "app.js",
        assetFileNames: "app[extname]",
      },
    },
  },
});
