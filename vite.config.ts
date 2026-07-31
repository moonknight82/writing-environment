import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    strictPort: true,
    watch: {
      ignored: [
        "**/artifacts/**",
        "**/src-tauri/target/**",
      ],
    },
  },
});
