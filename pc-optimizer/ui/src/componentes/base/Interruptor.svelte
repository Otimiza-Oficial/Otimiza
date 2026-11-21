<script lang="ts">
  // O interruptor (1.0.12, Frente 1.1): liga uma coisa que existe de verdade, nunca só navega. `role="switch"` com o
  // nome do que liga; quando não pode ligar, o motivo vai no texto de apoio (não só num `title` que o teclado não lê).
  let {
    ligado,
    rotulo,
    apoio = null,
    impedimento = null,
    disabled = false,
    onalternar,
  }: {
    ligado: boolean;
    /** O que liga, em palavras: vira o nome do controle para o leitor de tela. */
    rotulo: string;
    /** Uma linha sob o rótulo: o que muda quando liga. */
    apoio?: string | null;
    /** Por que não dá para ligar agora, e o que fazer ("Abra o Otimiza como administrador"). */
    impedimento?: string | null;
    disabled?: boolean;
    onalternar: (ligar: boolean) => void;
  } = $props();

  const id = `interruptor-${Math.random().toString(36).slice(2, 8)}`;
  const travado = $derived(disabled || (!ligado && !!impedimento));
</script>

<div class="linha-interruptor">
  <div class="textos">
    <span id={`${id}-rotulo`} class="rotulo-interruptor">{rotulo}</span>
    {#if impedimento && !ligado}
      <span id={`${id}-apoio`} class="apoio impedido">{impedimento}</span>
    {:else if apoio}
      <span id={`${id}-apoio`} class="apoio">{apoio}</span>
    {/if}
  </div>
  <button
    class="interruptor"
    role="switch"
    aria-checked={ligado}
    aria-labelledby={`${id}-rotulo`}
    aria-describedby={apoio || impedimento ? `${id}-apoio` : undefined}
    disabled={travado}
    onclick={() => onalternar(!ligado)}
  ></button>
</div>

<style>
  .linha-interruptor {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--e-3);
    min-width: 0;
  }
  .textos {
    display: grid;
    gap: 2px;
    min-width: 0;
  }
  .rotulo-interruptor {
    font-size: var(--t-linha);
    font-weight: 500;
  }
  .impedido {
    color: var(--atencao-texto);
  }
</style>
