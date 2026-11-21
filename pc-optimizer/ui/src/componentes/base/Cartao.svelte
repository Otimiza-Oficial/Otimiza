<script lang="ts">
  // O cartão de dentro de uma área (o spec premium, `docs/planos/superpowers/specs/2026-11-13-ui-premium-design.md`):
  // cabeçalho com o título e, à direita, um selo ou estado; o corpo; as ações embaixo; e, quando há, os detalhes
  // recolhidos. Um formato só para a página do jogo, no lugar dos quatro que conviviam ali.
  import type { Snippet } from "svelte";

  let {
    titulo,
    apoio = null,
    marca = false,
    selo,
    children,
    acoes,
    detalhes,
    rotuloDosDetalhes = "Ver detalhes",
  }: {
    titulo: string;
    /** Uma frase sob o título: o que este cartão resolve. */
    apoio?: string | null;
    /** O cartão principal da página (o Turbo do jogo): a borda mais firme. Sem cor: a marca v2 é monocromática. */
    marca?: boolean;
    selo?: Snippet;
    children?: Snippet;
    acoes?: Snippet;
    detalhes?: Snippet;
    rotuloDosDetalhes?: string;
  } = $props();
</script>

<div class="cartao" class:marca>
  <div class="cabeca">
    <h3>{titulo}</h3>
    {#if selo}<span class="selo-do-cartao">{@render selo()}</span>{/if}
  </div>
  {#if apoio}<p class="apoio">{apoio}</p>{/if}
  {@render children?.()}
  {#if acoes}<div class="acoes">{@render acoes()}</div>{/if}
  {#if detalhes}
    <details>
      <summary class="apoio">{rotuloDosDetalhes}</summary>
      <div class="detalhes">{@render detalhes()}</div>
    </details>
  {/if}
</div>

<style>
  .cartao {
    display: grid;
    align-content: start;
    gap: 10px;
    min-width: 0;
    padding: var(--e-4) 18px;
    border: 1px solid var(--linha);
    border-radius: var(--raio-3);
    background: var(--superficie-2);
  }
  .cartao.marca {
    border-color: var(--linha-forte);
    box-shadow: var(--fio);
  }
  .cabeca {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    min-height: 24px;
  }
  h3 {
    margin: 0;
    font-family: var(--fonte-titulo);
    font-size: var(--t-corpo);
    font-weight: 600;
    letter-spacing: var(--rastreio-subtitulo);
  }
  .selo-do-cartao {
    display: inline-flex;
    flex: none;
    align-items: center;
    gap: 6px;
  }
  .cartao :global(p) {
    margin: 0;
  }
  .acoes {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    padding-top: 2px;
  }
  .detalhes {
    display: grid;
    gap: 8px;
    padding-top: 10px;
  }
</style>
