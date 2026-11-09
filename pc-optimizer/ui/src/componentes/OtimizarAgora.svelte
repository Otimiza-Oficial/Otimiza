<script lang="ts">
  // "Otimizar agora" (trazido da 3.3, `src/v33/app.ts`, `otimizar`): os ajustes do Windows, do registro e do plano de
  // energia no nível escolhido, conferidos neste PC; cada um entra no histórico e se desfaz sozinho. O Rust cria o
  // ponto de restauração antes (se a preferência estiver ligada) e manda cada passo no evento `optimize:step`. O nível
  // Experimental não aparece aqui: ele vai um grupo por vez, medindo (aplicar tudo de uma vez derrubou o FPS de um
  // cliente).
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { noTauri } from "../lib/ponte";

  type Nivel = { id: string; nome: string; promessa: string; exigencia: string; itens: string[]; acrescenta: string[]; aplica_de_uma_vez: boolean };
  type Passo = { index: number; total: number; name: string; stage: string; message: string; success: boolean };
  type Resultado = { id: string; name: string; success: boolean; applied: boolean; message: string; requires_restart?: boolean };

  const CHAVE = "otimiza.nivel";

  let niveis = $state<Nivel[]>([]);
  let escolhido = $state(ler());
  let rodando = $state(false);
  let passos = $state<{ texto: string; ok: boolean }[]>([]);
  let total = $state(0);
  let fim = $state<string | null>(null);

  function ler() {
    try {
      return localStorage.getItem(CHAVE) ?? "Competitivo";
    } catch {
      return "Competitivo";
    }
  }

  onMount(async () => {
    const todos: Nivel[] = noTauri
      ? await invoke<Nivel[]>("niveis_de_otimizacao").catch(() => [])
      : [
          { id: "Seguro", nome: "Seguro", promessa: "Só o que não muda nada que você veja: menos coisa rodando por trás.", exigencia: "Nenhuma.", itens: Array(14).fill(""), acrescenta: [], aplica_de_uma_vez: true },
          { id: "Competitivo", nome: "Competitivo", promessa: "O Seguro, mais o que tira atraso: rede, mouse, plano de energia.", exigencia: "Pode deixar o PC um pouco mais quente em uso.", itens: Array(26).fill(""), acrescenta: [], aplica_de_uma_vez: true },
          { id: "Experimental", nome: "Experimental", promessa: "Ajustes que dependem de cada PC.", exigencia: "Um grupo por vez, medindo.", itens: Array(9).fill(""), acrescenta: [], aplica_de_uma_vez: false },
        ];
    niveis = todos.filter((n) => n.aplica_de_uma_vez);
    if (!niveis.some((n) => n.id === escolhido)) escolhido = niveis.at(-1)?.id ?? "Competitivo";
  });

  function escolher(id: string) {
    escolhido = id;
    try {
      localStorage.setItem(CHAVE, id);
    } catch {
      // Vale nesta sessão.
    }
  }

  async function otimizar() {
    rodando = true;
    fim = null;
    passos = [];
    total = 0;
    let parar = () => {};
    try {
      if (noTauri) {
        parar = await listen<Passo>("optimize:step", (e) => {
          total = e.payload.total;
          if (e.payload.stage === "finished") passos = [...passos, { texto: e.payload.name, ok: e.payload.success }];
        });
      }
      const r: Resultado[] = noTauri
        ? await invoke<Resultado[]>("optimize_now", { only: null, nivel: escolhido })
        : await new Promise((ok) => {
            const nomes = ["Ponto de restauração", "Plano de energia OTIMIZA", "Atrasos de rede (Nagle)", "Precisão do ponteiro", "Notificações durante o jogo"];
            total = nomes.length;
            nomes.forEach((n, i) => setTimeout(() => (passos = [...passos, { texto: n, ok: true }]), 300 * (i + 1)));
            setTimeout(() => ok(nomes.map((n, i) => ({ id: String(i), name: n, success: true, applied: i > 0, message: "", requires_restart: i === 2 }))), 300 * (nomes.length + 1));
          });
      const aplicados = r.filter((x) => x.success && x.applied);
      const reiniciar = aplicados.filter((x) => x.requires_restart).length;
      const nome = niveis.find((n) => n.id === escolhido)?.nome ?? escolhido;
      fim = aplicados.length
        ? `${aplicados.length} ajuste(s) aplicados no nível ${nome}.${reiniciar ? ` ${reiniciar} só valem depois de reiniciar o PC.` : ""} Jogue alguns minutos: o Otimiza mede o antes e o depois. Tudo se desfaz no Histórico.`
        : "Nada a mudar: o que é compatível já estava certo.";
    } catch (e) {
      fim = String(e);
    } finally {
      parar();
      rodando = false;
    }
  }
</script>

<section class="secao otimizar">
  <h2>Otimizar agora</h2>
  <p class="apoio">Os ajustes do Windows, do registro e do plano de energia, conferidos neste PC. Cada um fica no Histórico com o valor de antes.</p>
  {#if niveis.length > 1}
    <div class="niveis" role="radiogroup" aria-label="Nível">
      {#each niveis as n (n.id)}
        <button role="radio" aria-checked={escolhido === n.id} disabled={rodando} onclick={() => escolher(n.id)}>
          <span class="nome">{n.nome} <span class="apoio pequeno">{n.itens.length} ajustes</span></span>
          <span class="apoio pequeno">{n.promessa}</span>
          {#if n.exigencia && !/^nenhum/i.test(n.exigencia)}<span class="apoio pequeno exige">{n.exigencia}</span>{/if}
        </button>
      {/each}
    </div>
  {/if}
  <div class="acoes">
    <button class="btn primario grande" disabled={rodando || niveis.length === 0} onclick={otimizar}>{rodando ? "Otimizando…" : "Otimizar agora"}</button>
    {#if rodando && total}<span class="apoio">{passos.length} de {total}</span>{/if}
  </div>
  {#if passos.length}
    <ul class="passos">
      {#each passos.slice(-8) as p, i (i)}<li data-ok={p.ok}>{p.ok ? "✓" : "✕"} {p.texto}</li>{/each}
    </ul>
  {/if}
  {#if fim}<p class="fim">{fim}</p>{/if}
</section>

<style>
  .otimizar {
    display: grid;
    gap: 10px;
  }
  p {
    margin: 0;
  }
  .niveis {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 10px;
  }
  .niveis button {
    display: grid;
    gap: 4px;
    text-align: left;
    padding: 12px 14px;
    border-radius: var(--raio-2);
    border: 1px solid var(--linha);
    background: var(--superficie-2);
    color: var(--tinta);
    font: inherit;
    cursor: pointer;
  }
  .niveis button[aria-checked="true"] {
    border-color: var(--acao);
    box-shadow: 0 0 0 1px var(--acao);
  }
  .nome {
    font-weight: 600;
  }
  .exige {
    color: var(--atencao);
  }
  .acoes {
    display: flex;
    gap: 12px;
    align-items: center;
  }
  .grande {
    padding: 12px 22px;
    font-size: 15px;
  }
  .passos {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 4px;
    font-size: 13px;
  }
  .passos li[data-ok="true"] {
    color: var(--ok);
  }
  .passos li[data-ok="false"] {
    color: var(--perigo);
  }
  .pequeno {
    font-size: 12px;
  }
  .fim {
    font-size: 13.5px;
    font-weight: 500;
  }
</style>
