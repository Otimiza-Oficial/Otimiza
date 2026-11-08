import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { fileURLToPath } from "node:url";

// A interface da Nova Geração (Otimiza 1.0). Mora em `ui/` e convive com a atual (`index.html` + `src/`) até a
// migração terminar (Onda 2 do `docs/planos/PLANO-1.0-NOVA-ERA.md`). Mesma porta: o Tauri só conhece uma.

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  root: fileURLToPath(new URL("./ui", import.meta.url)),
  plugins: [svelte()],
  build: {
    outDir: fileURLToPath(new URL("./dist-nova", import.meta.url)),
    emptyOutDir: true,
    target: "es2022",
    // As páginas: a janela do app, a do FPS por cima do jogo (`plataforma::overlay`) e a da mira (`plataforma::mira`).
    rollupOptions: {
      input: {
        index: fileURLToPath(new URL("./ui/index.html", import.meta.url)),
        overlay: fileURLToPath(new URL("./ui/overlay.html", import.meta.url)),
        mira: fileURLToPath(new URL("./ui/mira.html", import.meta.url)),
      },
    },
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
});
