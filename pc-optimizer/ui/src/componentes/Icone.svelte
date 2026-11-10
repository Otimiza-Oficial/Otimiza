<script lang="ts" module>
  export { ICONES, type NomeDoIcone } from "../lib/icones";
</script>

<script lang="ts">
  // Um ícone do conjunto do Otimiza (`lib/icones.ts`). O desenho é fixo (texto confiável, escrito no código), por
  // isso entra com {@html}. O traço não engorda com o tamanho: em ícone grande ele afina para manter a família.
  import { ICONES, type NomeDoIcone } from "../lib/icones";

  let { nome, tamanho = 20 }: { nome: NomeDoIcone; tamanho?: number } = $props();

  const traco = $derived(tamanho >= 28 ? 1.5 : tamanho <= 14 ? 2 : 1.75);
</script>

<svg
  class="icone"
  viewBox="0 0 24 24"
  width={tamanho}
  height={tamanho}
  fill="none"
  stroke="currentColor"
  stroke-width={traco}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {@html ICONES[nome]}
</svg>

<style>
  .icone {
    flex: none;
  }
</style>
