<script lang="ts">
  // A saúde do que foi aplicado (trazida da interface antiga, `src/main.ts`): o que o Windows (ou outro programa)
  // desfez por fora — com "Refazer" —, os ajustes que brigam entre si e por quê, e o Otimiza conferindo o próprio
  // trabalho: o FPS de cada jogo antes e depois. "Sem amostra" é dito como "ainda não dá para dizer", nunca "tudo bem".
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { noTauri } from "../lib/ponte";

  type Estado = "Intacto" | "MudouPorFora" | "NaoLeu" | "NaoSeAplicaMais";
  type Vistoria = { itens: { id: string; nome: string; estado: Estado; retirado: boolean }[]; fora_da_vistoria: string[]; com_politica_de_grupo: boolean };
  type Conflito = { id: string; um: string; outro: string; tipo: string; mecanismo: string; conselho: string };
  type Desfecho = "SemAmostra" | "Igual" | "Melhorou" | "Piorou" | "PiorouMuito";
  type Veredito = { jogo: string; desfecho: Desfecho; variacao_fps_pct: number | null; variacao_low_pct: number | null };

  const DESFECHO: Record<Desfecho, { texto: string; tom: string }> = {
    SemAmostra: { texto: "ainda não dá para dizer", tom: "neutro" },
    Igual: { texto: "igual", tom: "neutro" },
    Melhorou: { texto: "melhorou", tom: "ok" },
    Piorou: { texto: "piorou", tom: "atencao" },
    PiorouMuito: { texto: "piorou muito", tom: "perigo" },
  };

  let vistoria = $state<Vistoria | null>(null);
  let conflitos = $state<Conflito[]>([]);
  let vereditos = $state<Veredito[]>([]);
  let ocupado = $state<string | null>(null);
  let recado = $state<string | null>(null);

  async function carregar() {
    if (!noTauri) {
      vistoria = { itens: [{ id: "nagle", nome: "Rede de baixa latência", estado: "MudouPorFora", retirado: false }, { id: "plano", nome: "Plano de energia OTIMIZA", estado: "Intacto", retirado: false }], fora_da_vistoria: [], com_politica_de_grupo: false };
      conflitos = [];
      vereditos = [{ jogo: "FiveM", desfecho: "Melhorou", variacao_fps_pct: 6.2, variacao_low_pct: 11.4 }];
      return;
    }
    const [v, c, r] = await Promise.allSettled([
      invoke<Vistoria>("vistoriar_mudancas"),
      invoke<Conflito[]>("conflitos_entre_ajustes"),
      invoke<Veredito[]>("conferir_o_proprio_trabalho"),
    ]);
    vistoria = v.status === "fulfilled" ? v.value : null;
    conflitos = c.status === "fulfilled" ? c.value : [];
    vereditos = r.status === "fulfilled" ? r.value : [];
  }

  onMount(carregar);

  async function refazer(id: string) {
    ocupado = id;
    recado = null;
    try {
      const r = noTauri ? await invoke<{ message: string }>("refazer_mudanca", { id }) : { message: "Demonstração: refeito." };
      recado = r.message;
      await carregar();
    } catch (e) {
      recado = String(e);
    } finally {
      ocupado = null;
    }
  }

  const mudou = $derived((vistoria?.itens ?? []).filter((i) => i.estado === "MudouPorFora"));
  const pct = (v: number | null) => (v == null ? "—" : `${v > 0 ? "+" : ""}${v.toFixed(1).replace(".", ",")}%`);
</script>

{#if vistoria || vereditos.length || conflitos.length}
  <section class="secao saude">
    <h2>O que foi aplicado continua valendo?</h2>

    {#if vistoria}
      {#if mudou.length === 0}
        <p class="apoio"><span class="ponto" data-tom="ok"></span> Tudo o que o Otimiza aplicou continua como ele deixou ({vistoria.itens.filter((i) => i.estado === "Intacto").length} conferidos).</p>
      {:else}
        <p class="aviso">{mudou.length === 1 ? "1 ajuste foi mudado" : `${mudou.length} ajustes foram mudados`} por fora — por uma atualização do Windows ou outro programa.{vistoria.com_politica_de_grupo ? " Esta máquina tem política de grupo: ela pode desfazer de novo." : ""}</p>
        {#each mudou as i (i.id)}
          <div class="linha">
            <span>{i.nome}</span>
            {#if !i.retirado}
              <button class="btn" disabled={ocupado !== null} onclick={() => refazer(i.id)}>{ocupado === i.id ? "Refazendo…" : "Refazer"}</button>
            {:else}
              <span class="apoio pequeno">saiu do catálogo: desfaça no histórico</span>
            {/if}
          </div>
        {/each}
      {/if}
      {#if vistoria.itens.some((i) => i.estado === "NaoLeu")}<p class="apoio pequeno">Alguns não deram para ler (permissão ou política) — isso não quer dizer que mudaram.</p>{/if}
    {/if}

    {#each conflitos as c (c.id)}
      <div class="conflito">
        <b>{c.um} × {c.outro}</b>
        <p class="apoio pequeno">{c.mecanismo}</p>
        <p class="apoio pequeno"><b>O que fazer:</b> {c.conselho}</p>
      </div>
    {/each}

    {#if vereditos.length}
      <div class="bloco">
        <span class="sub">FPS de cada jogo antes × depois do Otimiza</span>
        {#each vereditos as v (v.jogo)}
          <div class="linha">
            <span>{v.jogo}</span>
            <span data-tom={DESFECHO[v.desfecho].tom} class="desfecho">
              {DESFECHO[v.desfecho].texto}{v.desfecho !== "SemAmostra" ? ` · FPS ${pct(v.variacao_fps_pct)} · 1% pior ${pct(v.variacao_low_pct)}` : ""}
            </span>
          </div>
        {/each}
      </div>
    {/if}
    {#if recado}<p class="recado">{recado}</p>{/if}
  </section>
{/if}

<style>
  .saude,
  .bloco {
    display: grid;
    gap: 8px;
  }
  p {
    margin: 0;
  }
  .linha {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    font-size: 13px;
  }
  .sub {
    font-weight: 500;
  }
  .conflito {
    padding: 10px 12px;
    border-radius: var(--raio-2);
    background: var(--superficie-2);
    font-size: 13px;
  }
  .desfecho[data-tom="ok"] {
    color: var(--ok);
  }
  .desfecho[data-tom="atencao"] {
    color: var(--atencao);
  }
  .desfecho[data-tom="perigo"] {
    color: var(--perigo);
  }
  .aviso {
    color: var(--atencao);
    font-size: 13px;
  }
  .pequeno {
    font-size: 12px;
  }
  .recado {
    font-size: 13px;
  }
</style>
