<script lang="ts">
  // O diálogo de confirmação do app (pedido por `confirmar()` de `lib/confirmar.ts`). O foco começa em "Cancelar": um
  // Enter distraído nunca apaga nada. Esc e clique fora também cancelam.
  import { tick } from "svelte";
  import { PEDIR_CONFIRMACAO, type PedidoDeConfirmacao } from "../../lib/confirmar";

  let pedido = $state<PedidoDeConfirmacao | null>(null);
  let cancelar = $state<HTMLButtonElement | null>(null);
  let sim = $state<HTMLButtonElement | null>(null);
  /** Quem pediu (o botão "Limpar"): o foco volta para ele ao fechar. */
  let antes: HTMLElement | null = null;

  async function receber(e: Event) {
    const novo = (e as CustomEvent<PedidoDeConfirmacao>).detail;
    e.preventDefault();
    pedido?.responder(false);
    if (!pedido) antes = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    pedido = novo;
    await tick();
    cancelar?.focus();
  }

  function responder(sim: boolean) {
    const p = pedido;
    pedido = null;
    p?.responder(sim);
    antes?.focus();
    antes = null;
  }

  function teclado(e: KeyboardEvent) {
    if (!pedido) return;
    if (e.key === "Escape") responder(false);
    // O Tab fica entre os dois botões: a página borrada atrás não recebe foco.
    if (e.key === "Tab") {
      e.preventDefault();
      (document.activeElement === cancelar ? sim : cancelar)?.focus();
    }
  }

  $effect(() => {
    window.addEventListener(PEDIR_CONFIRMACAO, receber);
    return () => window.removeEventListener(PEDIR_CONFIRMACAO, receber);
  });
</script>

<svelte:window onkeydown={teclado} />

{#if pedido}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="veu" onclick={(e) => e.target === e.currentTarget && responder(false)}>
    <div class="folha" role="alertdialog" aria-modal="true" aria-labelledby="confirmar-titulo" aria-describedby="confirmar-texto">
      <h2 id="confirmar-titulo">{pedido.titulo}</h2>
      <p id="confirmar-texto" class="apoio">{pedido.texto}</p>
      <div class="acoes">
        <button class="btn" bind:this={cancelar} onclick={() => responder(false)}>Cancelar</button>
        <button class="btn" class:perigo={pedido.perigo} class:primario={!pedido.perigo} bind:this={sim} onclick={() => responder(true)}>{pedido.sim}</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .veu {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: grid;
    place-items: center;
    padding: 24px;
    background: var(--veu);
    backdrop-filter: var(--desfoque);
    animation: aparece var(--mov-medio) var(--curva-saida);
  }
  .folha {
    display: grid;
    gap: 10px;
    width: min(420px, 100%);
    padding: 22px 22px 18px;
    border: 1px solid var(--linha-forte);
    border-radius: var(--raio-3);
    background: var(--superficie);
    box-shadow: var(--sombra-3);
    color: var(--tinta);
    animation: sobe var(--mov-entrar) var(--curva-saida);
  }
  h2 {
    font-family: var(--fonte-titulo);
    font-size: var(--t-titulo);
    letter-spacing: -0.02em;
  }
  .folha .apoio {
    font-size: var(--t-corpo);
    line-height: 1.5;
  }
  .acoes {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 8px;
  }
  @keyframes aparece {
    from {
      opacity: 0;
    }
  }
  @keyframes sobe {
    from {
      opacity: 0;
      transform: translateY(12px) scale(0.98);
    }
  }
</style>
