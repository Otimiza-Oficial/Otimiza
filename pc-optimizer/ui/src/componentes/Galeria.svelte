<script lang="ts">
  // Só em desenvolvimento (`?galeria`): o Oti em todos os humores e gestos, para conferir o desenho de uma vez.
  import Oti, { type Gesto, type Humor } from "./Oti.svelte";

  const humores: Humor[] = ["atento", "falando", "feliz", "pensando", "preocupado", "surpreso", "jogando", "dormindo"];
  const gestos: Gesto[] = ["acenar", "apontar", "comemorar"];
</script>

<main>
  {#each humores as h (h)}
    <figure><Oti humor={h} tamanho={180} segueCursor={false} /><figcaption>{h}</figcaption></figure>
  {/each}
  {#each gestos as g (g)}
    <figure><Oti gesto={g} humor="feliz" tamanho={180} segueCursor={false} /><figcaption>{g}</figcaption></figure>
  {/each}
  <figure class="claro"><Oti tamanho={180} segueCursor={false} /><figcaption>fundo claro</figcaption></figure>
</main>

<style>
  main {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 24px;
    padding: 32px;
    height: 100vh;
    overflow: auto;
    background: radial-gradient(circle at 50% 30%, #2a2466, #0b0b22);
    color: #fff;
  }
  figure {
    margin: 0;
    display: grid;
    justify-items: center;
    gap: 8px;
  }
  .claro {
    background: #eef1fb;
    color: #222;
    border-radius: 16px;
  }
</style>
