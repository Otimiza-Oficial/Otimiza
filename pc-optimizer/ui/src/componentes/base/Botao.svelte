<script lang="ts">
  // O botão do app (1.0.12, Frente 1.1). Três jeitos e um de perigo, sempre com o verbo do que acontece:
  // - principal: cheio na cor do texto, letra do avesso. Um por bloco.
  // - contorno: a ação de apoio.
  // - link: leva a outro lugar; sublinhado de leve para não parecer título.
  // - perigo: apaga ou desfaz; vem depois de uma confirmação dentro do app (`lib/confirmar.ts`).
  // "Trabalhando" troca o texto pelo que está acontecendo e trava o clique, sem mudar a largura de repente.
  import type { Snippet } from "svelte";
  import type { HTMLButtonAttributes } from "svelte/elements";

  let {
    variante = "contorno",
    pequeno = false,
    trabalhando = false,
    textoTrabalhando = "Aguarde…",
    disabled = false,
    type = "button",
    children,
    ...resto
  }: {
    variante?: "principal" | "contorno" | "link" | "perigo";
    pequeno?: boolean;
    /** A ação está rodando: o botão trava e diz o que acontece. */
    trabalhando?: boolean;
    textoTrabalhando?: string;
    children: Snippet;
  } & Omit<HTMLButtonAttributes, "children"> = $props();
</script>

<button
  {type}
  class="btn"
  class:primario={variante === "principal"}
  class:perigo={variante === "perigo"}
  class:quieto={variante === "link"}
  class:pequeno
  disabled={disabled || trabalhando}
  aria-busy={trabalhando || undefined}
  {...resto}
>
  {#if trabalhando}
    <span class="giro" aria-hidden="true"></span>{textoTrabalhando}
  {:else}
    {@render children()}
  {/if}
</button>

<style>
  /* O sinal de "rodando": um arco fino na cor da letra. Para com o jogo aberto e com "reduzir movimento" (regras
     globais do `tokens.css`), e aí fica um arco parado, que ainda diz "ocupado". */
  .giro {
    flex: none;
    width: 12px;
    height: 12px;
    border: 1.5px solid currentColor;
    border-right-color: transparent;
    border-radius: 50%;
    animation: girar 800ms linear infinite;
  }
  @keyframes girar {
    to {
      transform: rotate(1turn);
    }
  }
</style>
