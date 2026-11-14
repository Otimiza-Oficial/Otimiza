<script lang="ts" module>
  /** Os Ajustes pedem para abrir a folha por este evento (a folha mora no App, por cima de tudo). */
  export const ABRIR_FEEDBACK = "otimiza:abrir-feedback";
</script>

<script lang="ts">
  import { como } from "../lib/contrato";
  import { commands } from "../lib/tipos";
  // O pedido de feedback (trazido da interface 3.3/3.6, `src/v33/app.ts`, "feedback"): depois de 10 minutos de uso as
  // correções pausam até chegar o feedback no canal do Discord; medir, ver o diagnóstico e desfazer continuam livres.
  // Sem esta tela, um cliente no estado "Exigido" veria as correções falharem sem saber por quê. O Rust decide a
  // situação (`feedback_estado`, evento `feedback:situacao`); aqui só se mostra e se confere (`feedback_verificar`).
  import { onDestroy, onMount } from "svelte";
    import { listen } from "@tauri-apps/api/event";
  import { noTauri } from "../lib/ponte";
  import Oti from "./Oti.svelte";

  type Situacao =
    | { situacao: "NaoSeAplica" }
    | { situacao: "Contando"; restante_segundos: number }
    | { situacao: "Liberado"; verificado_em: string }
    | { situacao: "Exigido"; apagado: boolean };
  type Estado = Situacao & { maquina: string; canal: string | null; canal_id: string; aviso: string | null };

  /** O convite embutido; o Rust devolve o atual quando consegue consultar. */
  const CONVITE = "https://discord.gg/ultimus";

  let estado = $state<Estado | null>(null);
  let folhaAberta = $state(false);
  let soMedindo = $state(false);
  let lembreteFechado = $state(false);
  let conferindo = $state(false);
  let recado = $state<{ tom: "ok" | "atencao" | null; texto: string } | null>(null);
  let copiado = $state(false);

  const perto = $derived(estado?.situacao === "Contando" && estado.restante_segundos <= 120);
  const minutos = $derived(estado?.situacao === "Contando" ? Math.max(1, Math.ceil(estado.restante_segundos / 60)) : 0);

  let vigia: ReturnType<typeof setInterval> | undefined;
  let deixar: (() => void) | undefined;

  function aplicar(s: Partial<Estado> & Situacao) {
    estado = { ...(estado ?? { maquina: "", canal: null, canal_id: "", aviso: null }), ...s } as Estado;
    if (estado.situacao === "Exigido" && !soMedindo) folhaAberta = true;
    if (estado.situacao === "Liberado") folhaAberta = false;
    // Contando: confere a cada 30 s, para o lembrete aparecer na hora certa (o evento só vem na troca de situação).
    clearInterval(vigia);
    if (estado.situacao === "Contando" && noTauri) {
      vigia = setInterval(async () => {
        const novo = await como<Estado>(commands.feedbackEstado()).catch(() => null);
        if (novo) aplicar(novo);
      }, 30000);
    }
  }

  function abrir() {
    folhaAberta = true;
    recado = null;
  }

  onMount(async () => {
    window.addEventListener(ABRIR_FEEDBACK, abrir);
    if (!noTauri) {
      // `?feedback` mostra a folha em demonstração, para conferir o desenho.
      if (new URLSearchParams(location.search).has("feedback")) aplicar({ situacao: "Exigido", apagado: false, maquina: "OTZ-7F3A-91C2", canal: null, canal_id: "", aviso: null });
      return;
    }
    const s = await como<Estado>(commands.feedbackEstado()).catch(() => null);
    if (s) aplicar(s);
    deixar = await listen<Situacao>("feedback:situacao", (e) => aplicar(e.payload));
  });

  onDestroy(() => {
    window.removeEventListener(ABRIR_FEEDBACK, abrir);
    clearInterval(vigia);
    deixar?.();
  });

  async function copiar() {
    try {
      await navigator.clipboard.writeText(estado?.maquina ?? "");
      copiado = true;
      setTimeout(() => (copiado = false), 1600);
    } catch {
      recado = { tom: "atencao", texto: "Não deu para copiar. Selecione o código e copie à mão." };
    }
  }

  async function abrirCanal() {
    const alvo = estado?.canal ?? (noTauri ? await como<string>(commands.conviteDoDiscord(CONVITE)).catch(() => CONVITE) : CONVITE);
    if (noTauri) {
      const { openUrl } = await import("@tauri-apps/plugin-opener");
      await openUrl(alvo).catch(() => {});
    } else window.open(alvo, "_blank", "noreferrer");
  }

  async function confirmar() {
    conferindo = true;
    recado = { tom: null, texto: "Conferindo com o bot…" };
    try {
      const s = noTauri ? await como<Estado>(commands.feedbackVerificar()) : ({ ...estado!, situacao: "Liberado", verificado_em: "agora" } as Estado);
      aplicar(s);
      if (s.situacao === "Liberado") {
        recado = { tom: "ok", texto: "Feedback encontrado. As correções estão liberadas." };
        setTimeout(() => (folhaAberta = false), 1400);
      } else recado = { tom: "atencao", texto: s.aviso ?? "Ainda não achamos a sua mensagem. Tente de novo em um minuto." };
    } catch (e) {
      recado = { tom: "atencao", texto: String(e) };
    } finally {
      conferindo = false;
    }
  }

  function soMedir() {
    soMedindo = true;
    folhaAberta = false;
  }

  function teclado(e: KeyboardEvent) {
    if (folhaAberta && e.key === "Escape") soMedir();
  }
</script>

<svelte:window onkeydown={teclado} />

{#if perto && !lembreteFechado && !folhaAberta}
  <div class="lembrete" role="status">
    <div class="lembrete-topo">
      <strong>{minutos === 1 ? "Falta 1 minuto" : `Faltam ${minutos} minutos`}</strong>
      <button class="fechar" aria-label="Fechar" onclick={() => (lembreteFechado = true)}>×</button>
    </div>
    <p class="apoio">Depois dos 10 minutos as correções pausam até chegar o seu feedback no Discord. Medir e desfazer continuam livres.</p>
    <button class="btn" onclick={abrir}>Deixar feedback agora</button>
  </div>
{/if}

{#if folhaAberta && estado}
  <div class="veu">
    <div class="folha" role="dialog" aria-modal="true" aria-labelledby="feedback-titulo">
      <div class="cabeca">
        <span class="mascote"><Oti humor="feliz" tamanho={58} segueCursor={false} /></span>
        <div>
          <span class="etiqueta">10 minutos de uso</span>
          <h2 id="feedback-titulo">Conta pra gente como ficou</h2>
        </div>
      </div>
      <p class="apoio">
        {estado.situacao === "Exigido" && estado.apagado
          ? "A sua mensagem não está mais no canal. Mande o feedback de novo para liberar as correções."
          : "As correções pausam até chegar o seu feedback. Medir, ver o diagnóstico e desfazer continuam livres."}
      </p>
      <ol class="passos">
        <li>
          <span class="bola">1</span>
          <div>
            <strong>Tire um print com o FPS</strong>
            <span class="apoio">A Prova do Otimiza ou qualquer contador de FPS servem.</span>
          </div>
        </li>
        <li>
          <span class="bola">2</span>
          <div>
            <strong>Mande no canal de feedbacks</strong>
            <span class="apoio">Pelo menos 30 caracteres e o print, com este código na mensagem:</span>
            <span class="codigo" class:copiado>
              <span class="valor">{estado.maquina || "—"}</span>
              <button onclick={copiar} aria-label="Copiar o código">{copiado ? "✓ Copiado" : "Copiar"}</button>
            </span>
          </div>
        </li>
        <li>
          <span class="bola">3</span>
          <div>
            <strong>Volte e confirme</strong>
            <span class="apoio">O bot leva até um minuto para ver a mensagem.</span>
          </div>
        </li>
      </ol>
      {#if recado}
        <div class="recado" role="status"><span class="ponto" data-tom={recado.tom}></span><div><span>{recado.texto}</span></div></div>
      {/if}
      <div class="acoes">
        <button class="botao-discord" onclick={abrirCanal}>
          <svg viewBox="0 0 24 24" aria-hidden="true"><path fill="currentColor" d="M20.3 4.4A19.8 19.8 0 0 0 15.4 3l-.6 1.3a18.4 18.4 0 0 0-5.6 0L8.6 3a19.7 19.7 0 0 0-4.9 1.5C.6 9.1-.3 13.6.1 18.1a19.9 19.9 0 0 0 6 3l1.3-2a12.8 12.8 0 0 1-2-1l.5-.4a14.2 14.2 0 0 0 12.2 0l.5.4c-.6.4-1.3.7-2 1l1.3 2a19.8 19.8 0 0 0 6-3c.5-5.2-.9-9.7-3.6-13.7ZM8 15.3c-1.2 0-2.2-1.1-2.2-2.4S6.8 10.5 8 10.5s2.2 1.1 2.2 2.4S9.2 15.3 8 15.3Zm8 0c-1.2 0-2.2-1.1-2.2-2.4s1-2.4 2.2-2.4 2.2 1.1 2.2 2.4-1 2.4-2.2 2.4Z"/></svg>
          Abrir o canal
        </button>
        <button class="botao-ja" disabled={conferindo} onclick={confirmar}>{conferindo ? "Conferindo…" : "Já mandei"}</button>
      </div>
      <button class="so-medindo" onclick={soMedir}>Continuar só medindo</button>
    </div>
  </div>
{/if}

<style>
  .lembrete {
    position: fixed;
    right: 24px;
    bottom: 96px;
    z-index: 30;
    display: grid;
    gap: 8px;
    width: 300px;
    padding: 14px 16px;
    border: 1px solid var(--linha-forte);
    border-radius: var(--raio-3);
    background: var(--superficie);
    box-shadow: var(--sombra-3);
    backdrop-filter: var(--desfoque);
    font-size: 13px;
  }
  .lembrete-topo {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .fechar {
    border: 0;
    background: none;
    color: var(--tinta-2);
    font-size: 18px;
    line-height: 1;
    cursor: pointer;
  }
  .lembrete .btn {
    justify-self: start;
  }
  .veu {
    position: fixed;
    inset: 0;
    z-index: 40;
    display: grid;
    place-items: center;
    padding: 24px;
    background: rgb(6 6 10 / 0.55);
    backdrop-filter: blur(6px);
    animation: aparece var(--mov-entrar) var(--curva-saida);
  }
  .folha {
    position: relative;
    display: grid;
    gap: 14px;
    width: min(480px, 100%);
    padding: 26px 26px 18px;
    border: 1px solid var(--linha-forte);
    border-radius: 26px;
    background:
      radial-gradient(120% 60% at 0% 0%, var(--marca-suave), transparent 60%),
      var(--superficie);
    box-shadow: 0 40px 100px -20px rgb(0 0 0 / 0.6), inset 0 1px 0 rgb(255 255 255 / 0.08);
    color: var(--tinta);
    animation: sobe var(--mov-entrar) var(--curva-saida);
  }
  :global(:root[data-tema="noite"]) .folha,
  :global(:root[data-tema="noite"]) .lembrete {
    background-color: rgb(30 30 34 / 0.98);
  }
  .cabeca {
    display: flex;
    align-items: center;
    gap: 14px;
  }
  .mascote {
    flex: none;
    filter: drop-shadow(0 8px 18px color-mix(in srgb, var(--marca) 35%, transparent));
  }
  .etiqueta {
    display: inline-block;
    padding: 3px 10px;
    border-radius: 999px;
    background: var(--marca-suave);
    color: var(--marca);
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }
  .folha h2 {
    margin-top: 6px;
    font-family: var(--fonte-titulo);
    font-size: 24px;
    font-weight: 700;
    letter-spacing: -0.025em;
    line-height: 1.15;
  }
  /* Os passos numa linha do tempo: bolinha numerada e o fio entre elas. */
  .passos {
    list-style: none;
    display: grid;
    gap: 14px;
    margin: 2px 0;
    padding: 0;
    font-size: 13px;
  }
  .passos li {
    position: relative;
    display: flex;
    gap: 14px;
  }
  .passos li:not(:last-child)::before {
    content: "";
    position: absolute;
    left: 13px;
    top: 30px;
    bottom: -12px;
    width: 2px;
    background: var(--linha-forte);
  }
  .bola {
    flex: none;
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background: var(--marca);
    color: #fff;
    font-weight: 800;
    font-size: 13px;
    box-shadow: 0 0 0 4px var(--marca-suave);
  }
  .passos strong,
  .passos .apoio {
    display: block;
  }
  .passos strong {
    font-size: 14px;
    margin-bottom: 2px;
  }
  .codigo {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    margin-top: 8px;
    padding: 4px 4px 4px 12px;
    border-radius: 12px;
    border: 1px dashed var(--linha-forte);
    background: var(--superficie-2);
    user-select: text;
  }
  .codigo .valor {
    font-family: ui-monospace, "Cascadia Code", Consolas, monospace;
    font-size: 14px;
    font-weight: 700;
    letter-spacing: 0.06em;
  }
  .codigo button {
    margin-left: 8px;
    height: 30px;
    padding: 0 12px;
    border: 0;
    border-radius: 9px;
    background: var(--tinta);
    color: var(--contra, #fff);
    font: inherit;
    font-size: 12px;
    font-weight: 700;
    cursor: pointer;
  }
  .codigo.copiado {
    border-color: var(--ok);
  }
  .codigo.copiado button {
    background: var(--ok);
    color: #fff;
  }
  .acoes {
    display: grid;
    grid-template-columns: 1.3fr 1fr;
    gap: 10px;
    margin-top: 4px;
  }
  .botao-discord,
  .botao-ja {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    height: 46px;
    border-radius: 14px;
    font: inherit;
    font-size: 14px;
    font-weight: 700;
    cursor: pointer;
    transition: transform var(--mov-rapido) var(--curva-mola), filter var(--mov-rapido) ease;
  }
  .botao-discord {
    border: 0;
    background: #5865f2;
    color: #fff;
    box-shadow: 0 10px 24px -8px rgb(88 101 242 / 0.7);
  }
  .botao-discord svg {
    width: 20px;
    height: 20px;
  }
  .botao-discord:hover {
    filter: brightness(1.08);
    transform: translateY(-1px);
  }
  .botao-ja {
    border: 1px solid var(--linha-forte);
    background: var(--superficie-2);
    color: var(--tinta);
  }
  .botao-ja:hover:not(:disabled) {
    transform: translateY(-1px);
  }
  .botao-ja:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .so-medindo {
    justify-self: center;
    border: 0;
    background: none;
    color: var(--tinta-2);
    font: inherit;
    font-size: 13px;
    cursor: pointer;
  }
  .so-medindo:hover {
    color: var(--tinta);
    text-decoration: underline;
    text-underline-offset: 3px;
  }
  @keyframes aparece {
    from {
      opacity: 0;
    }
  }
  @keyframes sobe {
    from {
      opacity: 0;
      transform: translateY(16px) scale(0.98);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .veu,
    .folha {
      animation: none;
    }
  }
  p {
    margin: 0;
  }
</style>
