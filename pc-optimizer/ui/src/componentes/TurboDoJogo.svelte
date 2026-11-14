<script lang="ts">
  // O Turbo neste jogo (página do jogo, aba Jogos): a receita que o Turbo guardou para ele, o que ela liga, se a prova
  // A-B confirmou e qual escolha de atraso vale. Só lê: ligar e medir de novo continuam no Turbo, com o jogo aberto.
  import { onMount } from "svelte";
  import { chaveDoJogo } from "../lib/capas";
  import { ponte, type ReceitaDoTurboPorJogo } from "../lib/ponte";

  let { jogo }: { jogo: string } = $props();

  let receitas = $state<ReceitaDoTurboPorJogo[] | null>(null);
  onMount(async () => {
    receitas = await ponte.turboReceitas().catch(() => []);
  });

  const r = $derived(receitas?.find((x) => x.id === chaveDoJogo(jogo) || chaveDoJogo(x.nome) === chaveDoJogo(jogo)) ?? null);

  const GARGALO: Record<string, string> = {
    Placa: "a placa de vídeo",
    Processador: "o processador",
    Memoria: "a memória",
  };
  const PROVA: Record<string, { texto: string; tom: "ok" | "atencao" | "perigo" }> = {
    Ganhou: { texto: "Provado: ganhou", tom: "ok" },
    Piorou: { texto: "Reprovado: não liga mais", tom: "perigo" },
    Indistinguivel: { texto: "Prova sem diferença", tom: "atencao" },
    SemComparacaoJusta: { texto: "Prova sem comparação justa", tom: "atencao" },
  };

  /** O que a receita liga, em etiquetas curtas. */
  const pecas = $derived.by(() => {
    if (!r) return [];
    const x = r.perfil.receita;
    const l: string[] = [];
    if (x.escala) l.push(`Escala ${x.escala}`);
    if (x.multiplicador != null && x.multiplicador !== 1) l.push(x.multiplicador === 0 ? "Frame Boost Auto" : `Frame Boost ${x.multiplicador}×`);
    if (x.energia) l.push("Energia de jogo");
    if (x.memoria) l.push("Memória livre para o jogo");
    return l;
  });

  const data = (s: number) => new Date(s * 1000).toLocaleDateString("pt-BR", { day: "2-digit", month: "2-digit" });
</script>

{#if r}
  {@const prova = r.perfil.prova ? PROVA[r.perfil.prova] : null}
  <div class="turbo-do-jogo">
    <div class="cabeca">
      <strong>Turbo neste jogo</strong>
      {#if prova}<span class="selo" data-tom={prova.tom}>{prova.texto}</span>{:else}<span class="selo estimado">ainda sem prova</span>{/if}
    </div>
    <p class="segura">
      Quem segura aqui é <b>{GARGALO[r.perfil.receita.gargalo] ?? "o jogo"}</b>{r.perfil.fps_antes != null ? ` · ${Math.round(r.perfil.fps_antes)} FPS antes` : ""} · medido em {data(r.perfil.medido_em)}
    </p>
    {#if pecas.length}
      <ul class="pecas">
        {#each pecas as p (p)}<li>{p}</li>{/each}
      </ul>
    {/if}
    <p class="apoio">{r.perfil.receita.explicacao}</p>
    <p class="apoio modo">
      {r.menor_atraso ? "Menor atraso" : "Mais FPS"}{r.competitivo && r.menor_atraso ? " (jogo competitivo: sem geração de quadros)" : ""}. No próximo
      "Jogar", o Turbo liga esta receita sozinho; para medir de novo, abra o jogo e use o Turbo.
    </p>
  </div>
{/if}

<style>
  .turbo-do-jogo {
    display: grid;
    gap: 8px;
    margin-top: 6px;
    padding: 14px 16px;
    border-radius: 16px;
    background: linear-gradient(180deg, var(--marca-suave), transparent 80%), var(--bloco, var(--superficie-2));
    border: 1px solid var(--linha);
  }
  .cabeca {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }
  .cabeca strong {
    font-family: var(--fonte-titulo);
    font-size: 15px;
  }
  p {
    margin: 0;
  }
  .segura {
    font-size: 13px;
  }
  .pecas {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .pecas li {
    padding: 4px 10px;
    border-radius: 999px;
    border: 1px solid var(--linha-forte);
    font-size: 12px;
    font-weight: 500;
  }
  .selo[data-tom="perigo"] {
    color: var(--perigo);
  }
  .modo {
    font-size: 12px;
  }
</style>
