/// <reference types="vite/client" />
import { mount } from "svelte";
import { invoke } from "@tauri-apps/api/core";
import "./tokens.css";
import App from "./App.svelte";

// Erro de JavaScript vai para o `otimiza.log` (`anotar_erro_da_tela`): é o arquivo que o cliente manda ao suporte, e
// sem isso uma tela quebrada no PC dele não deixa rastro nenhum.
if ("__TAURI_INTERNALS__" in window) {
  const anotar = (texto: string) => void invoke("anotar_erro_da_tela", { texto }).catch(() => {});
  window.addEventListener("error", (e) => anotar(`${e.message} (${e.filename}:${e.lineno}:${e.colno})`));
  window.addEventListener("unhandledrejection", (e) => anotar(`promessa: ${e.reason instanceof Error ? (e.reason.stack ?? e.reason.message) : String(e.reason)}`));
}

const alvo = document.getElementById("app");
if (!alvo) throw new Error("index.html sem #app");

// `?galeria` (só em desenvolvimento): o mascote em todos os humores, para conferir o desenho.
async function iniciar(destino: HTMLElement) {
  const busca = new URLSearchParams(location.search);
  // `?icone`: o Oti sozinho, em fundo transparente, para gerar o ícone do app (`npm run tauri icon`) e os da bandeja.
  if (import.meta.env.DEV && busca.has("icone")) {
    const { default: Oti } = await import("./componentes/Oti.svelte");
    document.body.style.background = "transparent";
    destino.classList.add("so-o-icone");
    // `&humor=dormindo&tamanho=64`: os ícones da bandeja, um por humor (`src-tauri/icons/bandeja`).
    const humor = (busca.get("humor") ?? "atento") as import("./componentes/Oti.svelte").Humor;
    const tamanho = Number(busca.get("tamanho")) || 1024;
    return mount(Oti, { target: destino, props: { tamanho, humor, segueCursor: false } });
  }
  if (import.meta.env.DEV && busca.has("galeria")) {
    const { default: Galeria } = await import("./componentes/Galeria.svelte");
    return mount(Galeria, { target: destino });
  }
  return mount(App, { target: destino });
}

const app = iniciar(alvo);

// Quanto a página levou até o primeiro desenho (`abertura_pronta`): a meta do plano é abrir em menos de 1,5 s num i3.
// O Rust soma o tempo do WebView2 nascendo; a divisão diz onde cortar.
if ("__TAURI_INTERNALS__" in window)
  void app.then(() =>
    requestAnimationFrame(() =>
      setTimeout(() => void invoke("abertura_pronta", { paginaMs: performance.now() }).catch(() => {}), 0)
    )
  );

export default app;
