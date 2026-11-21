<script lang="ts">
  // O diálogo de confirmação do app (pedido por `confirmar()` de `lib/confirmar.ts`). É o `<dialog>` do navegador
  // aberto com `showModal()`: ele já prende o foco, deixa a página atrás inerte e fecha com Esc, sem `div` clicável
  // (1.0.12, Frente 1.3). O foco começa em "Cancelar": um Enter distraído nunca apaga nada. Fechar de qualquer jeito
  // que não seja o botão da ação é "não".
  import { tick } from "svelte";
  import { PEDIR_CONFIRMACAO, type PedidoDeConfirmacao } from "../../lib/confirmar";

  let pedido = $state<PedidoDeConfirmacao | null>(null);
  let dialogo = $state<HTMLDialogElement | null>(null);
  let cancelar = $state<HTMLButtonElement | null>(null);
  /** Quem pediu (o botão "Limpar"): o foco volta para ele ao fechar. */
  let antes: HTMLElement | null = null;

  async function receber(e: Event) {
    const novo = (e as CustomEvent<PedidoDeConfirmacao>).detail;
    e.preventDefault();
    pedido?.responder(false);
    if (!pedido) antes = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    pedido = novo;
    await tick();
    if (dialogo && !dialogo.open) dialogo.showModal();
    cancelar?.focus();
  }

  function responder(sim: boolean) {
    const p = pedido;
    pedido = null;
    if (dialogo?.open) dialogo.close();
    p?.responder(sim);
    antes?.focus();
    antes = null;
  }

  $effect(() => {
    window.addEventListener(PEDIR_CONFIRMACAO, receber);
    return () => window.removeEventListener(PEDIR_CONFIRMACAO, receber);
  });
</script>

<dialog
  bind:this={dialogo}
  class="folha"
  role="alertdialog"
  aria-labelledby="confirmar-titulo"
  aria-describedby="confirmar-texto"
  oncancel={(e) => {
    e.preventDefault();
    responder(false);
  }}
>
  {#if pedido}
    <h2 id="confirmar-titulo">{pedido.titulo}</h2>
    <p id="confirmar-texto" class="apoio">{pedido.texto}</p>
    <div class="acoes">
      <button class="btn" bind:this={cancelar} onclick={() => responder(false)}>Cancelar</button>
      <button class="btn" class:perigo={pedido.perigo} class:primario={!pedido.perigo} onclick={() => responder(true)}>{pedido.sim}</button>
    </div>
  {/if}
</dialog>

<style>
  .folha {
    display: grid;
    gap: 10px;
    width: min(420px, calc(100% - 48px));
    margin: auto;
    padding: 22px 22px 18px;
    border: 1px solid var(--linha-forte);
    border-radius: var(--raio-3);
    background: var(--superficie);
    box-shadow: var(--sombra-3), var(--fio);
    color: var(--tinta);
  }
  .folha:not([open]) {
    display: none;
  }
  .folha[open] {
    animation: sobe var(--mov-entrar) var(--curva-saida);
  }
  .folha::backdrop {
    background: var(--veu);
    backdrop-filter: var(--desfoque);
    animation: aparece var(--mov-medio) var(--curva-saida);
  }
  h2 {
    font-family: var(--fonte-titulo);
    font-size: var(--t-titulo);
    letter-spacing: var(--rastreio-subtitulo);
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
      transform: translateY(8px);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .folha[open],
    .folha::backdrop {
      animation: none;
    }
  }
</style>
