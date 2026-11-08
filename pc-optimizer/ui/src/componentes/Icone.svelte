<script lang="ts" module>
  export { ICONES, type NomeDoIcone } from "../lib/icones";
</script>

<script lang="ts">
  // Um ícone do conjunto Phosphor (`lib/icones.ts`). O desenho é fixo do conjunto (texto confiável, gerado no código),
  // por isso entra com {@html}.
  import { ICONES, type NomeDoIcone } from "../lib/icones";

  let { nome, tamanho = 20 }: { nome: NomeDoIcone; tamanho?: number; cheio?: boolean } = $props();
</script>

<svg class="icone" viewBox="0 0 256 256" width={tamanho} height={tamanho} fill="currentColor" aria-hidden="true">
  {@html ICONES[nome]}
</svg>

<style>
  .icone {
    flex: none;
  }
</style>
