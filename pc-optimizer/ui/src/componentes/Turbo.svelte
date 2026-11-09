<script lang="ts">
  // A área Turbo: mede o jogo aberto por 10 s, monta a receita pelo gargalo (escala, Frame Boost, energia), liga e
  // lembra. A prova A-B (uns 5 minutos) é o que diz se ganhou de verdade; o antes × depois de uma sessão NÃO é prova e
  // a tela diz isso. Receita reprovada na prova não volta a ligar neste jogo.
  import { onDestroy, onMount } from "svelte";
  import DriverDaPlaca from "./DriverDaPlaca.svelte";
  import FrameBoost from "./FrameBoost.svelte";
  import Icone from "./Icone.svelte";
  import NoJogo from "./NoJogo.svelte";
  import OtimizarAgora from "./OtimizarAgora.svelte";
  import { ponte, type PainelDoTurbo, type ProvaDoTurbo } from "../lib/ponte";

  const GARGALO: Record<string, string> = {
    Placa: "a placa de vídeo",
    Processador: "o processador",
    Memoria: "a memória",
    Nenhum: "nada no limite",
    SemCarga: "o jogo estava parado na medida",
    Jogo: "o próprio jogo (limite dele ou do servidor)",
  };
  const ESCALA: Record<string, string> = {
    UltraQualidade: "Ultra qualidade",
    Qualidade: "Qualidade",
    Equilibrado: "Equilibrado",
    Desempenho: "Desempenho",
  };
  const DESFECHO: Record<string, { texto: string; tom: "ok" | "perigo" | "atencao" | "neutro" }> = {
    Ganhou: { texto: "Ganhou", tom: "ok" },
    Piorou: { texto: "Piorou: o Turbo foi desfeito", tom: "perigo" },
    Indistinguivel: { texto: "Sem diferença que dê para separar", tom: "neutro" },
    SemComparacaoJusta: { texto: "Sem comparação justa", tom: "atencao" },
  };

  /** O jogo aberto, vindo da casca: quando muda, o painel é relido (abrir o jogo com o Turbo na tela). */
  let { jogo = null }: { jogo?: string | null } = $props();

  let painel = $state<PainelDoTurbo | null>(null);
  let erro = $state<string | null>(null);
  let ocupado = $state<"" | "medindo" | "ligando" | "desligando" | "provando">("");
  let passou = $state(0);
  let copiado = $state(false);
  let relogio: ReturnType<typeof setInterval> | undefined;
  let laco: ReturnType<typeof setTimeout> | undefined;

  const prova = $derived<ProvaDoTurbo | null>(painel?.prova ?? null);
  const receita = $derived(painel?.perfil?.receita ?? null);
  const reprovada = $derived(painel?.perfil?.prova === "Piorou");

  async function ler() {
    try {
      painel = await ponte.turbo();
      erro = null;
    } catch (e) {
      erro = String(e);
    }
  }

  // Enquanto a prova roda (inclusive a automática, que começa sozinha), relê o painel devagar.
  function vigiar() {
    clearTimeout(laco);
    laco = setTimeout(async () => {
      await ler();
      if (painel?.provando) vigiar();
    }, 3000);
  }

  onMount(async () => {
    await ler();
    if (painel?.provando) vigiar();
  });

  let jogoLido: string | null | undefined = undefined;
  $effect(() => {
    const agora = jogo;
    if (jogoLido === undefined) {
      jogoLido = agora;
      return;
    }
    if (agora !== jogoLido) {
      jogoLido = agora;
      void ler();
    }
  });
  onDestroy(() => {
    clearInterval(relogio);
    clearTimeout(laco);
  });

  function contar() {
    passou = 0;
    clearInterval(relogio);
    relogio = setInterval(() => (passou += 1), 1000);
  }

  async function ligar(lembrado: boolean) {
    ocupado = lembrado ? "ligando" : "medindo";
    erro = null;
    contar();
    try {
      painel = await ponte.ligarTurbo(lembrado);
      if (painel.provando) vigiar();
    } catch (e) {
      erro = String(e);
    } finally {
      clearInterval(relogio);
      ocupado = "";
    }
  }

  async function desligar() {
    ocupado = "desligando";
    erro = null;
    try {
      painel = await ponte.desligarTurbo();
    } catch (e) {
      erro = String(e);
    } finally {
      ocupado = "";
    }
  }

  async function provar() {
    ocupado = "provando";
    erro = null;
    contar();
    vigiar();
    try {
      await ponte.provarTurbo();
    } catch (e) {
      erro = String(e);
    } finally {
      clearInterval(relogio);
      ocupado = "";
      await ler();
    }
  }

  async function atraso(menor: boolean) {
    try {
      painel = await ponte.atrasoDoTurbo(menor);
    } catch (e) {
      erro = String(e);
    }
  }

  async function esquecer() {
    try {
      painel = await ponte.esquecerTurbo();
    } catch (e) {
      erro = String(e);
    }
  }

  async function compartilhar() {
    try {
      await navigator.clipboard.writeText(await ponte.compartilharTurbo());
      copiado = true;
      setTimeout(() => (copiado = false), 2500);
    } catch (e) {
      erro = String(e);
    }
  }

  function num(v: number | null | undefined, casas = 0) {
    return v == null ? "—" : v.toFixed(casas).replace(".", ",");
  }

  function diferenca(d: ProvaDoTurbo["diferenca_fps"]): string {
    if (!d) return "—";
    if (d.Real) return `${(d.Real.pct ?? 0) >= 0 ? "+" : ""}${num(d.Real.pct, 1)}%`;
    if (d.Indistinguivel) return "sem diferença";
    return "faltou medir";
  }
</script>

<div class="turbo">
  <OtimizarAgora />
  {#if erro}
    <div class="recado" role="alert">
      <span class="ponto" data-tom="perigo"></span>
      <div><span>{erro}</span></div>
    </div>
  {/if}

  {#if painel === null}
    <p class="vazio">Lendo o Turbo…</p>
  {:else if !painel.jogo}
    <section class="secao sem-jogo">
      <span class="grande-icone"><Icone nome="raio" tamanho={30} /></span>
      <h2>Abra o jogo e volte aqui</h2>
      <p class="apoio texto">
        O Turbo mede o jogo aberto por 10 segundos, descobre quem está segurando o FPS (placa, processador ou memória) e
        monta a receita para este PC. Da segunda vez, ele lembra e liga num toque.
      </p>
    </section>
  {:else}
    <!-- Cabeça: o jogo, o estado e o botão principal -->
    <section class="secao cabeca">
      <div class="estado">
        <span class="lampada" class:ligado={painel.ligado}><Icone nome="raio" tamanho={22} /></span>
        <div>
          <h2>{painel.jogo}</h2>
          <span class="apoio">
            {#if ocupado === "medindo"}
              Medindo o jogo… {passou} de 10 s. Jogue normalmente.
            {:else if ocupado === "provando" || painel.provando}
              Provando{painel.prova_sozinha ? " (automática)" : ""}… {ocupado === "provando" ? `${Math.floor(passou / 60)}:${String(passou % 60).padStart(2, "0")} de ${Math.round(painel.duracao_da_prova_s / 60)} min` : "jogue normalmente"}
            {:else if painel.ligado}
              Turbo ligado
            {:else if reprovada}
              A receita lembrada piorou na prova e não liga de novo
            {:else if receita}
              Receita lembrada para este jogo
            {:else}
              Ainda sem receita para este jogo
            {/if}
          </span>
        </div>
      </div>
      <div class="acoes">
        {#if painel.ligado}
          <button class="btn" disabled={ocupado !== "" || painel.provando} onclick={desligar}>
            {ocupado === "desligando" ? "Desligando…" : "Desligar"}
          </button>
        {:else if receita && !reprovada}
          <button class="btn" disabled={ocupado !== ""} onclick={() => ligar(false)}>Medir de novo</button>
          <button class="btn primario" disabled={ocupado !== ""} onclick={() => ligar(true)}>
            {ocupado === "ligando" ? "Ligando…" : "Ligar o Turbo"}
          </button>
        {:else}
          <button class="btn primario" disabled={ocupado !== ""} onclick={() => ligar(false)}>
            {ocupado === "medindo" ? "Medindo…" : "Medir e ligar"}
          </button>
        {/if}
      </div>
    </section>

    {#if ocupado === "medindo" || ocupado === "provando" || painel.provando}
      <div class="progresso" aria-hidden="true">
        <div style:width={`${Math.min(100, ocupado === "medindo" ? (passou / 10) * 100 : (passou / Math.max(1, painel.duracao_da_prova_s)) * 100)}%`}></div>
      </div>
    {/if}

    <!-- Preferência -->
    <section class="secao">
      <div class="linha-opcao">
        <div>
          <h2>O que importa mais</h2>
          <span class="apoio">Com menor atraso, o Turbo não liga geração de quadros.</span>
        </div>
        <div class="segmento" role="radiogroup" aria-label="O que importa mais">
          <button role="radio" aria-checked={!painel.menor_atraso} onclick={() => atraso(false)}>Mais FPS</button>
          <button role="radio" aria-checked={painel.menor_atraso} onclick={() => atraso(true)}>Menor atraso</button>
        </div>
      </div>
    </section>

    <!-- Receita -->
    {#if receita}
      <section class="secao">
        <h2>A receita</h2>
        <p class="texto">{receita.explicacao}</p>
        <ul class="lista">
          <li><span class="ponto"></span><span class="rot">Quem segura o FPS</span><span>{GARGALO[receita.gargalo] ?? receita.gargalo}</span></li>
          <li>
            <span class="ponto" data-tom={receita.escala ? "ok" : "neutro"}></span><span class="rot">Escala na janela</span>
            <span>{receita.escala ? ESCALA[receita.escala] : "não liga"}</span>
          </li>
          <li>
            <span class="ponto" data-tom={receita.multiplicador != null && receita.multiplicador !== 1 ? "ok" : "neutro"}></span>
            <span class="rot">Frame Boost</span>
            <span>
              {receita.multiplicador == null ? "não liga" : receita.multiplicador === 0 ? "automático" : receita.multiplicador === 1 ? "só escala" : `${receita.multiplicador}×`}
            </span>
          </li>
          <li><span class="ponto" data-tom={receita.energia ? "ok" : "neutro"}></span><span class="rot">Plano de energia</span><span>{receita.energia ? "liga" : "não mexe"}</span></li>
          {#if receita.memoria}
            <li><span class="ponto" data-tom="atencao"></span><span class="rot">Memória</span><span>sugere fechar o que ocupa (nunca fecha sozinho)</span></li>
          {/if}
          {#if painel.perfil?.fps_antes != null}
            <li><span class="ponto"></span><span class="rot">FPS na medida</span><span class="numero">{num(painel.perfil.fps_antes)}</span></li>
          {/if}
        </ul>
        {#if !painel.ligado}
          <button class="btn quieto esquecer" onclick={esquecer}>Esquecer a receita deste jogo</button>
        {/if}
      </section>
    {/if}

    <!-- O que fez -->
    {#if painel.ligado && painel.passos.length > 0}
      <section class="secao">
        <h2>O que o Turbo fez</h2>
        <ul class="lista">
          {#each painel.passos as p, i (i)}
            <li><span class="ponto" data-tom="ok"></span><span>{p}</span></li>
          {/each}
        </ul>
      </section>
    {/if}

    <!-- Prova -->
    <section class="secao">
      <div class="linha-opcao">
        <div>
          <h2>Funcionou mesmo?</h2>
          <span class="apoio">
            A prova alterna com e sem o Turbo por uns {Math.round(painel.duracao_da_prova_s / 60)} minutos no mesmo jogo. Se
            piorar, o Turbo é desfeito.
          </span>
        </div>
        <button class="btn" disabled={!painel.ligado || ocupado !== "" || painel.provando} onclick={provar}>
          {ocupado === "provando" || painel.provando ? "Provando…" : prova ? "Provar de novo" : "Provar"}
        </button>
      </div>

      {#if prova}
        {@const d = DESFECHO[prova.desfecho]}
        <div class="resultado">
          <div class="veredito">
            <span class="ponto" data-tom={d.tom}></span>
            <strong>{d.texto}</strong>
            <span class="selo-medido">Medido</span>
          </div>
          <p class="texto">{prova.leitura}</p>
          <div class="numeros">
            <div>
              <span class="numero valor">{num(prova.fps_sem?.media)} → {num(prova.fps_com?.media)}</span>
              <span class="apoio">FPS sem → com · {diferenca(prova.diferenca_fps)}</span>
            </div>
            <div>
              <span class="numero valor">{num(prova.low_sem?.media)} → {num(prova.low_com?.media)}</span>
              <span class="apoio">1% pior sem → com · {diferenca(prova.diferenca_low)}</span>
            </div>
          </div>
          {#if prova.falha_ao_desfazer}
            <p class="erro">Não consegui desfazer depois de piorar: {prova.falha_ao_desfazer}</p>
          {/if}
          {#if prova.desfecho === "Ganhou"}
            <button class="btn" onclick={compartilhar}>{copiado ? "Copiado!" : "Copiar para o Discord"}</button>
          {/if}
        </div>
      {/if}
    </section>

    {#if painel.ritmo?.trancos_por_minuto}
      <section class="secao">
        <div class="recado">
          <span class="ponto" data-tom="atencao"></span>
          <div>
            <span>
              O {painel.jogo} deu {num(painel.ritmo.trancos_por_minuto)} trancos por minuto na última medida. Um teto
              estável em {painel.ritmo.teto} FPS tira o tranco (o Jogo liso chega aqui na próxima etapa).
            </span>
          </div>
        </div>
      </section>
    {/if}
  {/if}

  <!-- O Frame Boost à mão (a receita do Turbo também liga ele sozinho). -->
  <FrameBoost />
  <DriverDaPlaca />
  <NoJogo />
</div>

<style>
  .turbo {
    display: grid;
    gap: 4px;
    padding-top: 8px;
  }
  .vazio {
    padding: 24px 0;
    color: var(--tinta-2);
  }
  .texto {
    max-width: 64ch;
    color: var(--tinta-2);
  }
  .sem-jogo {
    justify-items: start;
    padding: 28px 0;
  }
  .sem-jogo h2 {
    font-family: var(--fonte-titulo);
    font-size: 20px;
  }
  .grande-icone,
  .lampada {
    display: grid;
    place-items: center;
    width: 52px;
    height: 52px;
    border-radius: 50%;
    background: var(--superficie-2);
    color: var(--tinta-2);
  }
  .cabeca {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    border-top: 0;
  }
  .estado {
    display: flex;
    align-items: center;
    gap: 14px;
  }
  .estado h2 {
    font-family: var(--fonte-titulo);
    font-size: 18px;
  }
  .lampada.ligado {
    background: #ffcc00;
    color: #1d1d1f;
    box-shadow: 0 0 0 6px rgb(255 204 0 / 0.25);
    animation: pulsar 2.4s ease-in-out infinite;
  }
  @keyframes pulsar {
    50% {
      box-shadow: 0 0 0 12px rgb(255 204 0 / 0.08);
    }
  }
  .acoes {
    display: flex;
    gap: 8px;
  }
  .progresso {
    height: 4px;
    margin: -8px 0 8px;
    border-radius: 2px;
    background: var(--linha);
    overflow: hidden;
  }
  .progresso div {
    height: 100%;
    background: #ffcc00;
    transition: width 900ms linear;
  }
  .linha-opcao {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }
  .linha-opcao > div:first-child {
    display: grid;
    gap: 2px;
  }
  .segmento {
    display: flex;
    flex: none;
    padding: 3px;
    border-radius: 18px;
    background: var(--superficie-2);
  }
  .segmento button {
    height: 30px;
    padding: 0 12px;
    border: 0;
    border-radius: 15px;
    background: transparent;
    color: var(--tinta-2);
    font-size: 13px;
    cursor: pointer;
  }
  .segmento button[aria-checked="true"] {
    background: var(--tinta);
    color: #fff;
  }
  :global(:root[data-tema="noite"]) .segmento button[aria-checked="true"] {
    color: #1d1d1f;
  }
  .lista .rot {
    flex: 1;
  }
  .esquecer {
    justify-self: start;
  }
  .resultado {
    display: grid;
    gap: 10px;
    justify-items: start;
    padding: 14px;
    border-radius: 14px;
    background: var(--superficie-2);
  }
  .veredito {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .numeros {
    display: flex;
    flex-wrap: wrap;
    gap: 28px;
  }
  .numeros div {
    display: grid;
  }
  .valor {
    font-size: 22px;
    font-weight: 600;
  }
  .erro {
    color: var(--perigo);
  }
</style>
