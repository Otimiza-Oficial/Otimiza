<script lang="ts">
  import { como } from "../lib/contrato";
  import { commands } from "../lib/tipos";
  import Esqueleto from "./Esqueleto.svelte";
  // Ativar a licença, em três passos que dizem o caminho inteiro: o código desta máquina vai para o bot, o bot devolve a
  // chave, a chave é colada aqui (`bot/README.md`). O campo entende a chave como ela chega do Discord e avisa quando
  // colaram o código da máquina no lugar dela. Serve à apresentação (`BoasVindas`) e aos Ajustes.
    import { scale } from "svelte/transition";
  import { backOut } from "svelte/easing";
  import { noTauri, ponte, type EstadoDaLicenca } from "../lib/ponte";
  import { lerCampo, limparChave } from "../lib/chave";

  let {
    licenca,
    onativada,
  }: { licenca: EstadoDaLicenca | null; onativada?: (l: EstadoDaLicenca) => void } = $props();

  /** O convite embutido; o Rust devolve o atual quando consegue consultar (o mesmo do pedido de feedback). */
  const CONVITE = "https://discord.gg/ultimus";

  let texto = $state("");
  let ativando = $state(false);
  let recusa = $state<string | null>(null);
  let copiado = $state(false);
  let ativada = $state<EstadoDaLicenca | null>(null);

  const leitura = $derived(lerCampo(texto));
  const atual = $derived(ativada ?? licenca);

  async function copiar() {
    if (!atual) return;
    try {
      await navigator.clipboard.writeText(atual.maquina);
      copiado = true;
      setTimeout(() => (copiado = false), 1800);
    } catch {
      // Sem área de transferência: o código está na tela para copiar à mão.
    }
  }

  async function abrirDiscord() {
    const alvo = noTauri ? await como<string>(commands.conviteDoDiscord(CONVITE)).catch(() => CONVITE) : CONVITE;
    if (noTauri) {
      const { openUrl } = await import("@tauri-apps/plugin-opener");
      await openUrl(alvo).catch(() => {});
    } else window.open(alvo, "_blank", "noreferrer");
  }

  async function ativar(e: SubmitEvent) {
    e.preventDefault();
    if (leitura !== "pronta" || ativando) return;
    ativando = true;
    recusa = null;
    try {
      const l = await ponte.ativarLicenca(limparChave(texto));
      if (l.ativa) {
        ativada = l;
        texto = "";
        onativada?.(l);
      } else recusa = l.motivo ?? "A chave não foi aceita.";
    } catch (erro) {
      recusa = String(erro);
    } finally {
      ativando = false;
    }
  }
</script>

{#if atual?.ativa}
  <div class="ativa" in:scale={{ duration: 520, start: 0.85, easing: backOut }}>
    <span class="selo-ok" aria-hidden="true">
      <svg viewBox="0 0 52 52"><circle cx="26" cy="26" r="24" /><path d="M15 27l7 7 15-16" /></svg>
    </span>
    <div>
      <strong>Licença ativa</strong>
      <p class="apoio">
        {atual.comprador ? `Em nome de ${atual.comprador} · ` : ""}{atual.expira ? `até ${atual.expira}` : "sem vencimento"} · {atual.maquina}
      </p>
    </div>
  </div>
{:else if atual}
  <div class="caixa"><ol class="passos">
    <li>
      <span class="n">1</span>
      <div class="corpo">
        <strong>Copie o código desta máquina</strong>
        <div class="codigo">
          <span class="numero">{atual.maquina}</span>
          <button class="btn" type="button" onclick={copiar}>{copiado ? "Copiado" : "Copiar"}</button>
        </div>
        <span class="apoio">{atual.origem}.</span>
      </div>
    </li>
    <li>
      <span class="n">2</span>
      <div class="corpo">
        <strong>Mande no Discord</strong>
        <span class="apoio">Ao comprar, o bot pede esse código e devolve a sua chave, um texto longo com um ponto no meio.</span>
        <button class="btn" type="button" onclick={abrirDiscord}>Abrir o Discord</button>
      </div>
    </li>
    <li>
      <span class="n">3</span>
      <form class="corpo" onsubmit={ativar}>
        <strong>Cole a chave aqui</strong>
        <textarea
          class="campo"
          bind:value={texto}
          rows="2"
          spellcheck="false"
          autocomplete="off"
          placeholder="Cole a chave que o bot mandou"
          aria-label="Chave da licença"
        ></textarea>
        {#if leitura === "codigo-da-maquina"}
          <span class="dica">Esse é o código da máquina, o do passo 1. Aqui vai a chave que o bot devolveu.</span>
        {:else if leitura === "incompleta"}
          <span class="dica">A chave parece cortada: ela tem duas partes longas separadas por um ponto.</span>
        {/if}
        {#if recusa}<span class="recusa" role="alert">{recusa}</span>{/if}
        <button class="btn primario" disabled={leitura !== "pronta" || ativando}>{ativando ? "Conferindo…" : "Ativar"}</button>
      </form>
    </li>
  </ol></div>
{:else}
  <Esqueleto linhas={2} rotulo="Lendo a licença" />
{/if}

<style>
  .caixa {
    container-type: inline-size;
  }
  .passos {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 14px 20px;
  }
  /* Com espaço, copiar e mandar ficam lado a lado: o campo da chave cabe sem rolar. */
  @container (min-width: 620px) {
    .passos {
      grid-template-columns: 1fr 1fr;
    }
    .passos li:last-child {
      grid-column: span 2;
    }
  }
  .passos li {
    display: grid;
    grid-template-columns: 28px 1fr;
    gap: 12px;
    align-items: start;
  }
  .n {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background: #1d1d1f;
    color: #fff;
    font-size: 13px;
    font-weight: 600;
  }
  .corpo {
    display: grid;
    gap: 6px;
    justify-items: start;
    min-width: 0;
  }
  .corpo strong {
    font-size: 14px;
    font-weight: 600;
  }
  .apoio {
    color: var(--tinta-2, var(--sec, #5d5d63));
    font-size: 12.5px;
    line-height: 1.4;
  }
  .codigo {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 8px 8px 14px;
    border-radius: 12px;
    background: rgb(255 255 255 / 0.6);
    border: 1px solid rgb(0 0 0 / 0.08);
  }
  .codigo .numero {
    white-space: nowrap;
    font-size: 16px;
    font-weight: 600;
    letter-spacing: 0.06em;
    user-select: all;
  }
  .campo {
    width: 100%;
    box-sizing: border-box;
    resize: vertical;
    padding: 10px 12px;
    border-radius: 12px;
    border: 1px solid var(--linha-forte, rgb(0 0 0 / 0.14));
    background: rgb(255 255 255 / 0.75);
    color: inherit;
    font: 12px/1.45 ui-monospace, "Cascadia Mono", Consolas, monospace;
    word-break: break-all;
  }
  .campo:focus-visible {
    outline: 2px solid var(--tinta, var(--texto));
    outline-offset: 1px;
  }
  :global(:root[data-tema="noite"]) .n {
    background: #f5f5f7;
    color: #1d1d1f;
  }
  :global(:root[data-tema="noite"]) .campo {
    background: rgb(255 255 255 / 0.06);
    border-color: rgb(255 255 255 / 0.16);
  }
  :global(:root[data-tema="noite"]) .codigo {
    background: rgb(255 255 255 / 0.06);
    border-color: rgb(255 255 255 / 0.12);
  }
  .dica {
    color: var(--atencao, #b25e00);
    font-size: 12.5px;
  }
  .recusa {
    color: var(--perigo, #c0392b);
    font-size: 12.5px;
  }
  .ativa {
    display: flex;
    align-items: center;
    gap: 14px;
  }
  .ativa strong {
    font-size: 15px;
  }
  .ativa p {
    margin: 2px 0 0;
  }
  .selo-ok svg {
    width: 44px;
    height: 44px;
    fill: none;
    stroke: var(--verde, #34c759);
    stroke-width: 3;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .selo-ok circle {
    stroke-dasharray: 151;
    stroke-dashoffset: 151;
    animation: tracar 600ms ease-out forwards;
  }
  .selo-ok path {
    stroke-dasharray: 36;
    stroke-dashoffset: 36;
    animation: tracar 360ms 480ms ease-out forwards;
  }
  @keyframes tracar {
    to {
      stroke-dashoffset: 0;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .selo-ok circle,
    .selo-ok path {
      animation: none;
      stroke-dashoffset: 0;
    }
  }
</style>
