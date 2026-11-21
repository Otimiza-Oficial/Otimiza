<script lang="ts">
  // As áreas da aba ativa, embaixo (o lugar), com a pílula que desliza até a área aberta. O vidro e a posição são do
  // contêiner, na casca (`App.svelte`, `nav.abas`). Saiu do `App.svelte` na 1.0.12 (Frente 1.2), sem mudar o visual.
  import { onMount, tick } from "svelte";
  import type { Grupo, IdDaArea } from "../lib/areas";
  import { nomeDaAba } from "../estado/navegacao.svelte";

  let { grupo, aberta, onabrir }: { grupo: Grupo; aberta: IdDaArea; onabrir: (id: IdDaArea) => void } = $props();

  // A pílula branca desliza até a aba ativa. A medida é dentro do contêiner da casca, que é o pai da pílula.
  let pilulaEl: HTMLElement | undefined = $state();
  let pilula = $state({ x: 0, w: 0 });
  async function medirPilula() {
    await tick();
    const barra = pilulaEl?.parentElement;
    const alvo = barra?.querySelector<HTMLElement>(".aba.ativa");
    if (!alvo || !barra) {
      pilula = { x: pilula.x, w: 0 };
      return;
    }
    pilula = { x: alvo.offsetLeft, w: alvo.offsetWidth };
  }
  $effect(() => {
    void aberta;
    void medirPilula();
  });

  onMount(() => {
    const ajustar = () => void medirPilula();
    window.addEventListener("resize", ajustar);
    return () => window.removeEventListener("resize", ajustar);
  });
</script>

<span class="pilula-ativa" bind:this={pilulaEl} style:transform={`translateX(${pilula.x}px)`} style:width={`${pilula.w}px`} style:opacity={pilula.w ? 1 : 0}></span>
{#each grupo.areas as id (id)}
  <button class="aba" class:ativa={aberta === id} onclick={() => onabrir(id)}>{nomeDaAba(id)}</button>
{/each}

<style>
  .pilula-ativa {
    position: absolute;
    left: 0;
    top: 6px;
    height: 38px;
    border-radius: 19px;
    background: var(--ativo);
    box-shadow: 0 4px 14px rgb(0 0 0 / 0.14);
    transition:
      transform var(--mov-entrar) var(--curva-mola),
      width var(--mov-entrar) var(--curva-mola),
      opacity var(--mov-medio) ease;
  }
  .aba {
    position: relative;
    height: 38px;
    padding: 0 18px;
    border: 0;
    border-radius: 19px;
    background: transparent;
    color: var(--texto);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: color var(--mov-medio) ease;
  }
  .aba.ativa {
    color: var(--ativo-texto);
    font-weight: 600;
  }
  /* O pontinho da marca embaixo da aba ativa: o único vermelho fixo da tela. */
  .aba::after {
    content: "";
    position: absolute;
    left: 50%;
    bottom: 5px;
    width: 4px;
    height: 4px;
    margin-left: -2px;
    border-radius: 50%;
    background: var(--marca);
    opacity: 0;
    transform: scale(0.4);
    transition: opacity var(--mov-medio) ease, transform var(--mov-medio) var(--curva-mola);
  }
  .aba.ativa::after {
    opacity: 1;
    transform: scale(1);
  }
</style>
