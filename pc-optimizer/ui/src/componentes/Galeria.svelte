<script lang="ts">
  // Só em desenvolvimento (`?galeria`): o Oti da marca v2 em cada olhar e nos dois tons, para conferir o desenho.
  import Oti, { type Humor } from "./Oti.svelte";

  const humores: Humor[] = ["atento", "pensando", "jogando", "preocupado", "dormindo"];
</script>

<main>
  {#each humores as h (h)}
    <figure><Oti humor={h} tamanho={180} tom="branco" /><figcaption>{h}</figcaption></figure>
  {/each}
  <figure class="claro"><Oti tamanho={180} tom="negativo" /><figcaption>fundo claro (negativo)</figcaption></figure>
  <figure><Oti tamanho={32} tom="branco" /><figcaption>32 px</figcaption></figure>
  <figure class="claro"><Oti tamanho={24} tom="negativo" /><figcaption>24 px</figcaption></figure>
</main>

<style>
  main {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 24px;
    padding: 32px;
    height: 100vh;
    overflow: auto;
    background: var(--preto);
    color: #fff;
  }
  figure {
    margin: 0;
    display: grid;
    justify-items: center;
    gap: 8px;
  }
  .claro {
    background: var(--papel);
    color: var(--carvao);
    border-radius: 16px;
  }
</style>
