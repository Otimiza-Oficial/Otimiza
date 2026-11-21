<script lang="ts">
  // O ornamento da época ao lado da saudação (1.0.13): um desenho de traço, no mesmo estilo dos ícones do app (quadro
  // 24 × 24, traço 1,75, pontas redondas), na cor de texto da época (`--epoca-texto`, com contraste AA medido).
  import type { IdDaTemporada } from "../../lib/temporadas";

  let { epoca, tamanho = 16 }: { epoca: IdDaTemporada; tamanho?: number } = $props();
</script>

<svg class="ornamento" width={tamanho} height={tamanho} viewBox="0 0 24 24" aria-hidden="true">
  {#if epoca === "natal"}
    <!-- floco de neve -->
    <path d="M12 3v18M4.2 7.5l15.6 9M4.2 16.5l15.6-9M9.5 4.5 12 7l2.5-2.5M9.5 19.5 12 17l2.5 2.5" />
  {:else if epoca === "ano-novo"}
    <!-- estouro de fogos -->
    <path d="M12 4v4M12 16v4M4 12h4M16 12h4M6.3 6.3l2.8 2.8M14.9 14.9l2.8 2.8M6.3 17.7l2.8-2.8M14.9 9.1l2.8-2.8" />
  {:else if epoca === "carnaval"}
    <!-- máscara -->
    <path d="M3 9c3-1.5 6-1.5 9 0 3-1.5 6-1.5 9 0-.3 4.5-2.6 7-5 7-1.8 0-3-1.2-4-3-1 1.8-2.2 3-4 3-2.4 0-4.7-2.5-5-7Z" />
    <path d="M7 11.5h2M15 11.5h2" />
  {:else if epoca === "pascoa"}
    <!-- ovo pintado -->
    <path d="M12 3c-3.6 0-6.5 5.2-6.5 10a6.5 6.5 0 0 0 13 0C18.5 8.2 15.6 3 12 3Z" />
    <path d="M6 12.5l2-1.5 2 1.5 2-1.5 2 1.5 2-1.5 2 1.5" />
  {:else if epoca === "festa-junina"}
    <!-- bandeirinhas -->
    <path d="M3 6c6 2 12 2 18 0" />
    <path d="M6 6.8v6l2-1.6 2 1.6V7.3M14 7.3v6l2-1.6 2 1.6v-6.5" />
  {:else if epoca === "dia-das-criancas"}
    <!-- balão -->
    <path d="M12 3a5.5 5.5 0 0 0-5.5 5.8C6.5 12.6 9.5 15 12 15s5.5-2.4 5.5-6.2A5.5 5.5 0 0 0 12 3Z" />
    <path d="M11 15.5h2l-1 1.5ZM12 17c0 1.5 1.5 2 0 4" />
  {:else}
    <!-- abóbora -->
    <path d="M12 7c-1.5-1-4-1.2-5.5.3C4.3 9.5 4.3 15 6.5 17.5c1.6 1.8 4 1.8 5.5.8 1.5 1 3.9 1 5.5-.8 2.2-2.5 2.2-8 0-10.2C16 5.8 13.5 6 12 7Z" />
    <path d="M12 7c0-1.5.6-2.8 2-3.5M9.5 12l1-1 1 1M12.5 12l1-1 1 1M9.5 15c1.5.8 3.5.8 5 0" />
  {/if}
</svg>

<style>
  .ornamento {
    display: inline-block;
    flex: none;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.75;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
</style>
