<script lang="ts">
  import { como } from "../lib/contrato";
  import { commands } from "../lib/tipos";
  import Esqueleto from "./base/Esqueleto.svelte";
  import Botao from "./base/Botao.svelte";
  // Ativar a licença, em três passos que dizem o caminho inteiro: o código desta máquina vai para o bot, o bot devolve a
  // chave, a chave é colada aqui (`bot/README.md`). O campo entende a chave como ela chega do Discord e avisa quando
  // colaram o código da máquina no lugar dela. Serve à primeira execução (`BoasVindas`) e aos Ajustes. Marca v2
  // (1.0.12, Frente 1.4): números na cor do texto, código num campo de leitura, recusa que diz o que fazer.
  import { fade } from "svelte/transition";
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
  const parado = typeof window !== "undefined" && window.matchMedia("(prefers-reduced-motion: reduce)").matches;

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
  <div class="ativa" role="status" in:fade={{ duration: parado ? 0 : 240 }}>
    <span class="selo-ok" aria-hidden="true">
      <svg viewBox="0 0 24 24"><path d="M6 12.5l4 4 8-9" /></svg>
    </span>
    <div>
      <strong>Licença ativa</strong>
      <p class="apoio">
        {atual.comprador ? `Em nome de ${atual.comprador} · ` : ""}{atual.expira ? `até ${atual.expira}` : "sem vencimento"} · {atual.maquina}
      </p>
    </div>
  </div>
{:else if atual}
  <div class="caixa">
    <ol class="passos">
      <li>
        <span class="n" aria-hidden="true">1</span>
        <div class="corpo">
          <strong>Copie o código desta máquina</strong>
          <div class="codigo">
            <span class="numero" aria-label="Código da máquina">{atual.maquina}</span>
            <Botao pequeno onclick={copiar}>{copiado ? "Copiado" : "Copiar"}</Botao>
          </div>
          <span class="apoio">{atual.origem}.</span>
        </div>
      </li>
      <li>
        <span class="n" aria-hidden="true">2</span>
        <div class="corpo">
          <strong>Mande no Discord</strong>
          <span class="apoio">Ao comprar, o bot pede esse código e devolve a sua chave: um texto longo com um ponto no meio.</span>
          <Botao pequeno onclick={abrirDiscord}>Abrir o Discord</Botao>
        </div>
      </li>
      <li>
        <span class="n" aria-hidden="true">3</span>
        <form class="corpo" onsubmit={ativar}>
          <label for="chave-da-licenca"><strong>Cole a chave aqui</strong></label>
          <textarea
            id="chave-da-licenca"
            class="campo chave"
            class:com-erro={!!recusa}
            bind:value={texto}
            rows="2"
            spellcheck="false"
            autocomplete="off"
            placeholder="A chave que o bot mandou"
            aria-invalid={recusa ? true : undefined}
            aria-describedby="chave-nota"
          ></textarea>
          <span id="chave-nota" class="nota">
            {#if recusa}
              <span class="recusa" role="alert">{recusa} Confira se copiou a chave inteira (do começo ao fim) e se ela é desta máquina; se o problema seguir, mande o código do passo 1 no Discord.</span>
            {:else if leitura === "codigo-da-maquina"}
              <span class="dica">Esse é o código da máquina, o do passo 1. Aqui vai a chave que o bot devolveu.</span>
            {:else if leitura === "incompleta"}
              <span class="dica">A chave parece cortada: ela tem duas partes longas separadas por um ponto. Copie de novo no Discord.</span>
            {/if}
          </span>
          <Botao type="submit" variante="principal" disabled={leitura !== "pronta"} trabalhando={ativando} textoTrabalhando="Conferindo…">Ativar</Botao>
        </form>
      </li>
    </ol>
  </div>
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
    gap: var(--e-4) var(--e-5);
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
    gap: var(--e-3);
    align-items: start;
  }
  .n {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border-radius: 50%;
    border: 1px solid var(--linha-forte);
    color: var(--tinta);
    font-size: var(--t-apoio);
    font-weight: 600;
  }
  .corpo {
    display: grid;
    gap: var(--e-2);
    justify-items: start;
    min-width: 0;
  }
  .corpo strong {
    font-size: var(--t-corpo);
    font-weight: 600;
  }
  .apoio {
    line-height: 1.45;
  }
  .codigo {
    display: flex;
    align-items: center;
    gap: var(--e-3);
    padding: 6px 6px 6px var(--e-3);
    border-radius: var(--raio-2);
    border: 1px solid var(--linha);
    background: var(--superficie-2);
  }
  .codigo .numero {
    white-space: nowrap;
    font-size: var(--t-subtitulo);
    letter-spacing: 0.06em;
    user-select: all;
  }
  .chave {
    width: 100%;
    min-height: 64px;
    resize: vertical;
    padding: 10px var(--e-3);
    font-family: ui-monospace, "Cascadia Mono", Consolas, monospace;
    font-size: var(--t-apoio);
    line-height: 1.45;
    word-break: break-all;
  }
  .chave.com-erro {
    border-color: var(--perigo);
  }
  .nota {
    font-size: var(--t-apoio);
    line-height: 1.45;
  }
  .dica {
    color: var(--atencao-texto);
  }
  .recusa {
    color: var(--perigo-texto);
  }
  .ativa {
    display: flex;
    align-items: center;
    gap: var(--e-3);
  }
  .ativa strong {
    font-size: var(--t-subtitulo);
  }
  .ativa p {
    margin: 2px 0 0;
  }
  /* O sinal de ativa: o traço na cor de "ok" dentro de um aro fino. É estado, por isso tem cor. */
  .selo-ok {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    border: 1.5px solid var(--ok);
    border-radius: 50%;
  }
  .selo-ok svg {
    width: 22px;
    height: 22px;
    fill: none;
    stroke: var(--ok);
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
</style>
