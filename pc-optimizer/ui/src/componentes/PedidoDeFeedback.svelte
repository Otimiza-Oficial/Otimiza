<script lang="ts" module>
  /** Os Ajustes pedem para abrir a folha por este evento (a folha mora no App, por cima de tudo). */
  export const ABRIR_FEEDBACK = "otimiza:abrir-feedback";
</script>

<script lang="ts">
  // O pedido de feedback (trazido da interface 3.3/3.6, `src/v33/app.ts`, "feedback"): depois de 10 minutos de uso as
  // correções pausam até chegar o feedback no canal do Discord; medir, ver o diagnóstico e desfazer continuam livres.
  // Sem esta tela, um cliente no estado "Exigido" veria as correções falharem sem saber por quê. O Rust decide a
  // situação (`feedback_estado`, evento `feedback:situacao`); aqui só se mostra e se confere (`feedback_verificar`).
  import { onDestroy, onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { noTauri } from "../lib/ponte";

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
        const novo = await invoke<Estado>("feedback_estado").catch(() => null);
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
    const s = await invoke<Estado>("feedback_estado").catch(() => null);
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
    const alvo = estado?.canal ?? (noTauri ? await invoke<string>("convite_do_discord", { embutido: CONVITE }).catch(() => CONVITE) : CONVITE);
    if (noTauri) {
      const { openUrl } = await import("@tauri-apps/plugin-opener");
      await openUrl(alvo).catch(() => {});
    } else window.open(alvo, "_blank", "noreferrer");
  }

  async function confirmar() {
    conferindo = true;
    recado = { tom: null, texto: "Conferindo com o bot…" };
    try {
      const s = noTauri ? await invoke<Estado>("feedback_verificar") : ({ ...estado!, situacao: "Liberado", verificado_em: "agora" } as Estado);
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
      <p class="apoio pequeno">10 minutos de uso</p>
      <h2 id="feedback-titulo">Conta pra gente como ficou</h2>
      <p class="apoio">
        {estado.situacao === "Exigido" && estado.apagado
          ? "A sua mensagem não está mais no canal. Mande o feedback de novo para liberar as correções."
          : "As correções pausam até chegar o seu feedback. Medir, ver o diagnóstico e desfazer continuam livres."}
      </p>
      <ol class="passos">
        <li>
          <strong>Tire um print com o FPS</strong>
          <span class="apoio">A Prova do Otimiza ou qualquer contador de FPS servem.</span>
        </li>
        <li>
          <strong>Mande no canal de feedbacks</strong>
          <span class="apoio">Pelo menos 30 caracteres e o print. Cole este código na mensagem:</span>
          <span class="codigo">
            <span class="numero">{estado.maquina || "—"}</span>
            <button class="btn quieto" onclick={copiar}>{copiado ? "Copiado" : "Copiar"}</button>
          </span>
        </li>
        <li>
          <strong>Volte e confirme</strong>
          <span class="apoio">O bot leva até um minuto para ver a mensagem.</span>
        </li>
      </ol>
      {#if recado}
        <div class="recado" role="status"><span class="ponto" data-tom={recado.tom}></span><div><span>{recado.texto}</span></div></div>
      {/if}
      <div class="acoes">
        <button class="btn primario" onclick={abrirCanal}>Abrir o canal</button>
        <button class="btn" disabled={conferindo} onclick={confirmar}>{conferindo ? "Conferindo…" : "Já mandei"}</button>
      </div>
      <button class="btn quieto so-medindo" onclick={soMedir}>Continuar só medindo</button>
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
    background: rgb(10 12 20 / 0.42);
  }
  .folha {
    display: grid;
    gap: 10px;
    width: min(460px, 100%);
    padding: 24px;
    border: 1px solid var(--linha-forte);
    border-radius: var(--raio-3);
    background: var(--superficie);
    box-shadow: var(--sombra-3);
    color: var(--tinta);
  }
  :global(:root[data-tema="noite"]) .folha,
  :global(:root[data-tema="noite"]) .lembrete {
    background: rgb(36 36 40 / 0.98);
  }
  .folha h2 {
    font-family: var(--fonte-titulo);
    font-size: var(--t-titulo);
    font-weight: 600;
  }
  .passos {
    display: grid;
    gap: 12px;
    margin: 6px 0;
    padding-left: 20px;
    font-size: 13px;
  }
  /* Os passos têm ordem de verdade: o número fica. */
  .passos li {
    padding-left: 4px;
  }
  .passos li::marker {
    color: var(--tinta-2);
    font-weight: 600;
  }
  .passos li > span,
  .passos li > strong {
    display: block;
  }
  .passos li > .codigo {
    display: flex;
  }
  .codigo {
    display: flex;
    align-items: center;
    gap: 12px;
    width: fit-content;
    margin-top: 4px;
    padding: 6px 10px;
    border-radius: var(--raio-1);
    background: var(--superficie-2);
    user-select: text;
  }
  .acoes {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }
  .so-medindo {
    justify-self: center;
    color: var(--tinta-2);
  }
  p {
    margin: 0;
  }
  .pequeno {
    font-size: 12px;
  }
</style>
