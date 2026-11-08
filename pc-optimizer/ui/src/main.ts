/// <reference types="vite/client" />
import { mount } from "svelte";
import "./tokens.css";
import App from "./App.svelte";

const alvo = document.getElementById("app");
if (!alvo) throw new Error("index.html sem #app");

// `?galeria` (só em desenvolvimento): o mascote em todos os humores, para conferir o desenho.
async function iniciar(destino: HTMLElement) {
  const busca = new URLSearchParams(location.search);
  // `?icone`: o Oti sozinho, em fundo transparente, para gerar o ícone do app (`npm run tauri icon`).
  if (import.meta.env.DEV && busca.has("icone")) {
    const { default: Oti } = await import("./componentes/Oti.svelte");
    document.body.style.background = "transparent";
    destino.classList.add("so-o-icone");
    return mount(Oti, { target: destino, props: { tamanho: 1024, segueCursor: false } });
  }
  if (import.meta.env.DEV && busca.has("galeria")) {
    const { default: Galeria } = await import("./componentes/Galeria.svelte");
    return mount(Galeria, { target: destino });
  }
  return mount(App, { target: destino });
}

export default iniciar(alvo);
