import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";
import { resolve } from "node:path";

/** Local playground only — root base so /playground.html resolves. */
export default defineConfig({
  plugins: [vue(), tailwindcss()],
  base: "/",
  root: resolve(__dirname),
  server: {
    open: "/playground.html",
  },
});
