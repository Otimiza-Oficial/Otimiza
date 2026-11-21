<script lang="ts">
  // A dica (tooltip, 1.0.12, Frente 1.1): meia frase que explica um termo ou um controle sem palavra. Aparece com o
  // mouse e com o foco do teclado, some com Esc, e é ligada ao gatilho por `aria-describedby`. Nunca guarda informação
  // que a pessoa precisa para agir: essa vai no texto da tela.
  import type { Snippet } from "svelte";

  let {
    texto,
    lado = "cima",
    children,
  }: {
    texto: string;
    lado?: "cima" | "baixo";
    /** O gatilho: recebe o id da dica para pôr no `aria-describedby`. */
    children: Snippet<[string]>;
  } = $props();

  const id = `dica-${Math.random().toString(36).slice(2, 8)}`;
  let aberta = $state(false);
  let espera: ReturnType<typeof setTimeout> | undefined;

  function mostrar(atraso: number) {
    clearTimeout(espera);
    espera = setTimeout(() => (aberta = true), atraso);
  }
  function esconder() {
    clearTimeout(espera);
    aberta = false;
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<span
  class="dica-ancora"
  onpointerenter={() => mostrar(350)}
  onpointerleave={esconder}
  onfocusin={() => mostrar(0)}
  onfocusout={esconder}
  onkeydown={(e) => e.key === "Escape" && esconder()}
>
  {@render children(id)}
  <span {id} role="tooltip" class="dica" class:aberta data-lado={lado}>{texto}</span>
</span>

<style>
  .dica-ancora {
    position: relative;
    display: inline-flex;
  }
  .dica {
    position: absolute;
    left: 50%;
    bottom: calc(100% + 8px);
    z-index: 40;
    width: max-content;
    max-width: 260px;
    padding: 6px 10px;
    border: 1px solid var(--linha-forte);
    border-radius: var(--raio-1);
    background: var(--superficie-menu);
    box-shadow: var(--sombra-2);
    color: var(--tinta);
    font-size: var(--t-apoio);
    font-weight: 400;
    line-height: 1.45;
    text-align: left;
    white-space: normal;
    pointer-events: none;
    opacity: 0;
    visibility: hidden;
    transform: translate(-50%, 2px);
    transition:
      opacity var(--mov-rapido) ease,
      transform var(--mov-rapido) var(--curva-saida),
      visibility 0s linear var(--mov-rapido);
  }
  .dica[data-lado="baixo"] {
    top: calc(100% + 8px);
    bottom: auto;
    transform: translate(-50%, -2px);
  }
  .dica.aberta {
    opacity: 1;
    visibility: visible;
    transform: translate(-50%, 0);
    transition:
      opacity var(--mov-rapido) ease,
      transform var(--mov-rapido) var(--curva-saida);
  }
</style>
