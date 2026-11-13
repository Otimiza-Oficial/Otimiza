<script lang="ts">
  import { como } from "../lib/contrato";
  import { commands } from "../lib/tipos";
  // VRR (G-Sync / FreeSync) lido do EDID de cada monitor (trazido de `src/main.ts`). O Windows não diz se o VRR está
  // LIGADO no painel do driver: aqui só o que o monitor anuncia, e onde ligar. "Provável" é dito como provável.
  import { onMount } from "svelte";
    import { noTauri } from "../lib/ponte";

  type Monitor = { nome: string | null; min_hz: number | null; max_hz: number | null; vrr: "Anunciado" | "Provavel" | "NaoAnunciado" };
  const FRASE = {
    Anunciado: "faz taxa variável (FreeSync/G-Sync Compatible anunciado)",
    Provavel: "provavelmente faz taxa variável (faixa larga no monitor)",
    NaoAnunciado: "taxa fixa",
  } as const;

  let monitores = $state<Monitor[] | null>(null);

  onMount(async () => {
    monitores = noTauri
      ? await como<Monitor[]>(commands.vrrDosMonitores()).catch(() => null)
      : [{ nome: "AOC 24G2", min_hz: 48, max_hz: 144, vrr: "Anunciado" }];
  });
</script>

{#if monitores && monitores.length}
  <section class="secao">
    <h2>Taxa variável (VRR)</h2>
    {#each monitores as m, i (i)}
      <p>
        <b>{m.nome ?? `Monitor ${i + 1}`}</b>{m.min_hz && m.max_hz ? ` · ${m.min_hz}–${m.max_hz} Hz` : ""}: {FRASE[m.vrr]}.
      </p>
    {/each}
    {#if monitores.some((m) => m.vrr !== "NaoAnunciado")}
      <p class="apoio">
        Com VRR, o monitor espera o quadro do jogo em vez de mostrar o anterior de novo: some o rasgo sem o atraso da
        sincronia vertical. Ligue no painel da NVIDIA (Configurar G-SYNC) ou da AMD (Adrenalin › Tela › FreeSync) e use o
        cabo DisplayPort quando der. O Windows não informa se está ligado lá.
      </p>
    {/if}
  </section>
{/if}

<style>
  p {
    margin: 0;
    font-size: 13px;
  }
  .secao {
    display: grid;
    gap: 8px;
  }
</style>
