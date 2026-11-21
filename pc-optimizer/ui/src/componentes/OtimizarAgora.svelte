<script lang="ts">
  import { como } from "../lib/contrato";
  import { commands, type Nivel as NivelDoContrato } from "../lib/tipos";
  // "Otimizar agora" (trazido da 3.3, `src/v33/app.ts`, `otimizar`): os ajustes do Windows, do registro e do plano de
  // energia no nível escolhido, conferidos neste PC; cada um entra no histórico e se desfaz sozinho. O Rust cria o
  // ponto de restauração antes (se a preferência estiver ligada) e manda cada passo no evento `optimize:step`. O nível
  // Experimental não aparece aqui: ele vai um grupo por vez, medindo (aplicar tudo de uma vez derrubou o FPS de um
  // cliente). Redesenho da 1.0.12 (Frente 1.4): níveis como rádio de verdade (setas), estados desenhados e o próximo
  // passo no fim (medir na Prova).
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { noTauri } from "../lib/ponte";
  import { abrirArea } from "../lib/areas";
  import { proximaAba } from "../lib/abas";
  import Botao from "./base/Botao.svelte";
  import Estado from "./base/Estado.svelte";

  type Nivel = { id: string; nome: string; promessa: string; exigencia: string; itens: string[]; acrescenta: string[]; aplica_de_uma_vez: boolean };
  type Passo = { index: number; total: number; name: string; stage: string; message: string; success: boolean };
  type Resultado = { id: string; name: string; success: boolean; applied: boolean; message: string; requires_restart?: boolean };

  const CHAVE = "otimiza.nivel";

  /** Quem embute (a primeira execução) sabe quando a rodada terminou bem, para marcar o passo como feito. */
  let { onterminou }: { onterminou?: () => void } = $props();

  let niveis = $state<Nivel[]>([]);
  let escolhido = $state(ler());
  let rodando = $state(false);
  let passos = $state<{ texto: string; ok: boolean }[]>([]);
  let total = $state(0);
  /** O fim da última rodada: o que mudou (ok) ou o que deu errado e o que fazer (erro). */
  let fim = $state<{ tom: "ok" | "erro"; texto: string; detalhe?: string } | null>(null);
  /** A lista de níveis não veio (o Rust não respondeu): erro desenhado, não um botão morto. */
  let semNiveis = $state(false);

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
    semNiveis = niveis.length === 0;
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

  /** Setas entre os níveis (grupo de rádio): escolhem e levam o foco junto. */
  function teclaNoNivel(e: KeyboardEvent) {
    const i = niveis.findIndex((n) => n.id === escolhido);
    const tecla = e.key === "ArrowDown" ? "ArrowRight" : e.key === "ArrowUp" ? "ArrowLeft" : e.key;
    const j = proximaAba(i, tecla, niveis.length);
    if (j === null || rodando) return;
    e.preventDefault();
    escolher(niveis[j].id);
    (e.currentTarget as HTMLElement).querySelectorAll<HTMLElement>("[role=radio]")[j]?.focus();
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
      const falharam = r.filter((x) => !x.success).length;
      const reiniciar = aplicados.filter((x) => x.requires_restart).length;
      const nome = niveis.find((n) => n.id === escolhido)?.nome ?? escolhido;
      fim = {
        tom: "ok",
        texto: aplicados.length
          ? `${aplicados.length} ${aplicados.length === 1 ? "ajuste aplicado" : "ajustes aplicados"} no nível ${nome}.` +
            (reiniciar ? ` ${reiniciar} ${reiniciar === 1 ? "só vale" : "só valem"} depois de reiniciar o PC.` : "") +
            (falharam ? ` ${falharam} não ${falharam === 1 ? "pôde ser aplicado e ficou" : "puderam ser aplicados e ficaram"} como estava.` : "") +
            " Agora jogue uns minutos e veja na Prova se melhorou. Tudo se desfaz no Histórico."
          : "Nada a mudar: o que é compatível com este PC já estava certo.",
      };
      onterminou?.();
    } catch (e) {
      fim = {
        tom: "erro",
        texto: "Não consegui terminar de otimizar. O que já foi aplicado está no Histórico e se desfaz de lá.",
        detalhe: String(e),
      };
    } finally {
      parar();
      rodando = false;
    }
  }
</script>

<section class="secao otimizar" aria-labelledby="otimizar-titulo">
  <h2 id="otimizar-titulo">Otimizar agora</h2>
  <p class="apoio">Os ajustes do Windows, do registro e do plano de energia, conferidos neste PC. Cada um fica no Histórico com o valor de antes.</p>
  {#if semNiveis}
    <Estado tipo="erro" titulo="Não consegui ler os níveis de otimização" fazer="Feche e abra o Otimiza. Se continuar, mande o arquivo de registro ao suporte (Ajustes)." />
  {:else if niveis.length === 0}
    <Estado tipo="lendo" linhas={2} titulo="Lendo os ajustes compatíveis" />
  {:else}
    {#if niveis.length > 1}
      <!-- svelte-ignore a11y_interactive_supports_focus -->
      <div class="niveis" role="radiogroup" aria-label="Nível de otimização" onkeydown={teclaNoNivel}>
        {#each niveis as n (n.id)}
          <button role="radio" aria-checked={escolhido === n.id} tabindex={escolhido === n.id ? 0 : -1} disabled={rodando} onclick={() => escolher(n.id)}>
            <span class="radio" aria-hidden="true"></span>
            <span class="nome">{n.nome}</span>
            <span class="conta"><strong class="numero">{n.itens.length}</strong> ajustes</span>
            <span class="apoio">{n.promessa}</span>
            {#if n.exigencia && !/^nenhum/i.test(n.exigencia)}<span class="exige"><span class="ponto" data-tom="atencao" aria-hidden="true"></span>{n.exigencia}</span>{/if}
          </button>
        {/each}
      </div>
    {/if}
    <div class="acoes">
      <Botao variante="principal" trabalhando={rodando} textoTrabalhando={total ? `Otimizando · ${passos.length} de ${total}` : "Otimizando…"} onclick={otimizar}>Otimizar agora</Botao>
      {#if !rodando}<span class="apoio">Cria um ponto de restauração antes, se estiver ligado nos Ajustes.</span>{/if}
    </div>
    {#if (rodando || passos.length) && total}
      <div class="progresso" role="progressbar" aria-label="Ajustes aplicados" aria-valuemin={0} aria-valuemax={total} aria-valuenow={passos.length}>
        <div style:width={`${Math.min(100, (passos.length / total) * 100)}%`}></div>
      </div>
    {/if}
    {#if passos.length}
      <ul class="passos" aria-live="polite">
        {#each passos.slice(-6) as p, i (i)}
          <li><span class="ponto" data-tom={p.ok ? "ok" : "perigo"} aria-hidden="true"></span>{p.texto}<span class="so-leitor">{p.ok ? ": aplicado" : ": não aplicado"}</span></li>
        {/each}
      </ul>
    {/if}
    {#if fim?.tom === "ok"}
      <div class="recado" role="status">
        <span class="ponto" data-tom="ok"></span>
        <div>
          <span>{fim.texto}</span>
          <span class="links-do-fim">
            <Botao variante="link" pequeno onclick={() => abrirArea("prova")}>Medir na Prova</Botao>
            <Botao variante="link" pequeno onclick={() => abrirArea("historico")}>Ver o que mudou</Botao>
          </span>
        </div>
      </div>
    {:else if fim?.tom === "erro"}
      <Estado tipo="erro" titulo={fim.texto} fazer="Tente de novo. Se repetir, abra o Otimiza como administrador (botão direito › Executar como administrador)." detalhe={fim.detalhe} />
    {/if}
  {/if}
</section>

<style>
  .otimizar {
    display: grid;
    gap: var(--e-3);
  }
  p {
    margin: 0;
  }
  .niveis {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: var(--e-3);
  }
  /* Cada nível é um cartão escolhível: o rádio no canto, o nome, o número de ajustes grande. O escolhido ganha a borda
     na cor do texto; sem brilho nem cor de marca. */
  .niveis button {
    position: relative;
    display: grid;
    gap: 6px;
    align-content: start;
    text-align: left;
    padding: 18px var(--e-4);
    border-radius: var(--raio-3);
    border: 1px solid var(--linha);
    background: var(--superficie-2);
    color: var(--tinta);
    cursor: pointer;
    transition:
      border-color var(--mov-rapido) ease,
      background var(--mov-rapido) ease;
  }
  .niveis button:hover:not(:disabled) {
    border-color: var(--linha-forte);
  }
  .niveis button[aria-checked="true"] {
    border-color: var(--tinta);
    box-shadow: inset 0 0 0 1px var(--tinta);
  }
  .niveis button:disabled {
    cursor: default;
  }
  .radio {
    position: absolute;
    top: 18px;
    right: 18px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: 1.5px solid var(--linha-forte);
    transition: border-color var(--mov-rapido) ease;
  }
  .niveis button[aria-checked="true"] .radio {
    border-color: var(--tinta);
    background: radial-gradient(circle, var(--tinta) 0 4px, transparent 4.5px);
  }
  .nome {
    font-family: var(--fonte-titulo);
    font-size: var(--t-subtitulo);
    font-weight: 600;
    letter-spacing: var(--rastreio-subtitulo);
  }
  .conta {
    color: var(--tinta-2);
    font-size: var(--t-linha);
  }
  .conta .numero {
    margin-right: 4px;
    color: var(--tinta);
    font-size: var(--t-numero);
    letter-spacing: var(--rastreio-titulo);
  }
  .exige {
    display: flex;
    align-items: center;
    gap: var(--e-2);
    color: var(--tinta-2);
    font-size: var(--t-apoio);
  }
  .acoes {
    display: flex;
    gap: var(--e-3);
    align-items: center;
  }
  .acoes :global(.btn.primario) {
    min-height: 40px;
    padding: 0 var(--e-5);
  }
  .progresso {
    height: 3px;
    border-radius: 2px;
    background: var(--superficie-3);
    overflow: hidden;
  }
  .progresso div {
    height: 100%;
    background: var(--tinta);
    transition: width var(--mov-medio) var(--curva-padrao);
  }
  .passos {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 6px;
    font-size: var(--t-linha);
  }
  .passos li {
    display: flex;
    align-items: center;
    gap: var(--e-2);
  }
  .links-do-fim {
    display: flex;
    gap: var(--e-4);
  }
  .so-leitor {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
</style>
