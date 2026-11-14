<script lang="ts">
  import { como } from "../lib/contrato";
  import { commands, type Nivel as NivelDoContrato } from "../lib/tipos";
  // "Otimizar agora" (trazido da 3.3, `src/v33/app.ts`, `otimizar`): os ajustes do Windows, do registro e do plano de
  // energia no nível escolhido, conferidos neste PC; cada um entra no histórico e se desfaz sozinho. O Rust cria o
  // ponto de restauração antes (se a preferência estiver ligada) e manda cada passo no evento `optimize:step`. O nível
  // Experimental não aparece aqui: ele vai um grupo por vez, medindo (aplicar tudo de uma vez derrubou o FPS de um
  // cliente).
  import { onMount } from "svelte";
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
      ? await como<Nivel[]>(commands.niveisDeOtimizacao()).catch(() => [])
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
        ? await como<Resultado[]>(commands.optimizeNow(null, escolhido as NivelDoContrato))
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
        ? `${aplicados.length} ${aplicados.length === 1 ? "ajuste aplicado" : "ajustes aplicados"} no nível ${nome}.${reiniciar ? ` ${reiniciar} só valem depois de reiniciar o PC.` : ""} Jogue alguns minutos: o Otimiza mede o antes e o depois. Tudo se desfaz no Histórico.`
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
          <span class="radio" aria-hidden="true"></span>
          <span class="nome">{n.nome}</span>
          <span class="conta"><strong class="numero">{n.itens.length}</strong> ajustes</span>
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
  {#if (rodando || passos.length) && total}
    <div class="progresso" aria-hidden="true"><div style:width={`${Math.min(100, (passos.length / total) * 100)}%`}></div></div>
  {/if}
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
  /* Cada nível é um cartão escolhível: a bolinha de rádio no canto, o nome, o número de ajustes grande. */
  .niveis button {
    position: relative;
    display: grid;
    gap: 4px;
    align-content: start;
    text-align: left;
    padding: 16px 18px;
    border-radius: 16px;
    border: 1px solid var(--linha);
    background: var(--bloco, var(--superficie-2));
    color: var(--tinta);
    font: inherit;
    cursor: pointer;
    transition: border-color var(--mov-rapido) ease, transform var(--mov-rapido) ease, background var(--mov-rapido) ease;
  }
  .niveis button:hover:not(:disabled) {
    border-color: var(--linha-forte);
  }
  .niveis button[aria-checked="true"] {
    border-color: var(--marca);
    background: linear-gradient(180deg, var(--marca-suave), transparent 70%), var(--bloco, var(--superficie-2));
    box-shadow: 0 0 0 1px var(--marca), 0 12px 30px -14px var(--marca);
  }
  .radio {
    position: absolute;
    top: 16px;
    right: 16px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: 2px solid var(--linha-forte);
    transition: border-color var(--mov-rapido) ease, box-shadow var(--mov-rapido) ease;
  }
  .niveis button[aria-checked="true"] .radio {
    border-color: var(--marca);
    box-shadow: inset 0 0 0 4px var(--superficie, #fff), inset 0 0 0 9px var(--marca);
  }
  :global(:root[data-tema="noite"]) .niveis button[aria-checked="true"] .radio {
    box-shadow: inset 0 0 0 4px #1e1e22, inset 0 0 0 9px var(--marca);
  }
  .nome {
    font-family: var(--fonte-titulo);
    font-size: 16px;
    font-weight: 700;
  }
  .conta {
    color: var(--tinta-2);
    font-size: 13px;
  }
  .conta .numero {
    font-size: 30px;
    font-weight: 700;
    color: var(--tinta);
    letter-spacing: -0.03em;
    margin-right: 2px;
  }
  .progresso {
    height: 6px;
    border-radius: 999px;
    background: var(--linha-forte);
    overflow: hidden;
  }
  .progresso div {
    height: 100%;
    border-radius: 999px;
    background: var(--marca);
    transition: width var(--mov-medio) ease;
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
    min-height: 40px;
    padding: 0 20px;
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
    font-size: 13px;
    font-weight: 500;
  }
</style>
