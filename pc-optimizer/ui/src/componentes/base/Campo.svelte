<script lang="ts">
  // O campo de texto (1.0.12, Frente 1.1): rótulo sempre visível (o exemplo dentro do campo some ao digitar e não
  // serve de nome), ajuda embaixo e o erro no lugar da ajuda, dizendo o que corrigir. O erro liga `aria-invalid`.
  import type { HTMLInputAttributes } from "svelte/elements";

  let {
    rotulo,
    valor = $bindable(""),
    ajuda = null,
    erro = null,
    largo = false,
    ...resto
  }: {
    rotulo: string;
    valor?: string;
    ajuda?: string | null;
    /** O que está errado e como corrigir ("A chave tem 4 grupos de 5 letras"). */
    erro?: string | null;
    /** Ocupa a linha toda. */
    largo?: boolean;
  } & Omit<HTMLInputAttributes, "value"> = $props();

  const id = `campo-${Math.random().toString(36).slice(2, 8)}`;
</script>

<div class="campo-inteiro" class:largo>
  <label for={id} class="rotulo-campo">{rotulo}</label>
  <input
    {id}
    class="campo"
    class:com-erro={!!erro}
    bind:value={valor}
    aria-invalid={erro ? true : undefined}
    aria-describedby={erro || ajuda ? `${id}-nota` : undefined}
    {...resto}
  />
  {#if erro}
    <span id={`${id}-nota`} class="nota erro" role="alert">{erro}</span>
  {:else if ajuda}
    <span id={`${id}-nota`} class="nota apoio">{ajuda}</span>
  {/if}
</div>

<style>
  .campo-inteiro {
    display: grid;
    gap: 6px;
    min-width: 0;
  }
  .campo-inteiro.largo {
    width: 100%;
  }
  .rotulo-campo {
    font-size: var(--t-linha);
    font-weight: 500;
  }
  .campo {
    width: 100%;
  }
  .campo.com-erro {
    border-color: var(--perigo);
  }
  .nota {
    font-size: var(--t-apoio);
  }
  .nota.erro {
    color: var(--perigo-texto);
  }
</style>
