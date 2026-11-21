<script lang="ts">
  // Os botões do trilho da esquerda: as 4 abas (a intenção). O vidro e a posição são do contêiner, na casca
  // (`App.svelte`, `nav.trilho`). Saiu do `App.svelte` na 1.0.12 (Frente 1.2), sem mudar o visual.
  import Icone, { type NomeDoIcone } from "./Icone.svelte";
  import { grupos, type Grupo, type IdDoGrupo } from "../lib/areas";

  let { ativo, onabrir }: { ativo: IdDoGrupo; onabrir: (g: Grupo) => void } = $props();

  const ICONE_DO_GRUPO: Record<IdDoGrupo, NomeDoIcone> = {
    jogar: "controle",
    meu_pc: "monitor",
    prova: "relogio",
    ajustes: "ajustes",
  };
</script>

{#each grupos as g (g.id)}
  <button
    class="trilho-botao"
    class:ativo={ativo === g.id}
    aria-label={g.nome}
    aria-current={ativo === g.id ? "page" : undefined}
    title={g.nome}
    onclick={() => onabrir(g)}
  >
    <Icone nome={ICONE_DO_GRUPO[g.id]} tamanho={20} />
    <span class="trilho-nome">{g.nome}</span>
  </button>
{/each}

<style>
  .trilho-botao {
    display: grid;
    place-items: center;
    align-content: center;
    gap: 3px;
    width: 56px;
    height: 54px;
    border: 0;
    border-radius: 18px;
    background: transparent;
    color: var(--texto);
    cursor: pointer;
    transition:
      background var(--mov-medio) ease,
      transform var(--mov-medio) var(--curva-mola);
  }
  .trilho-botao:hover {
    background: var(--bloco-2);
  }
  /* O lugar onde você está: cheio e escuro, para achar de relance sobre o vidro claro. */
  .trilho-botao.ativo {
    background: var(--texto);
    color: var(--contra, #fff);
    box-shadow: 0 2px 6px rgb(0 0 0 / 0.18);
  }
  .trilho-botao:focus-visible {
    outline-offset: 0;
  }
  /* O nome da aba sob o ícone: quatro abas, e quem abre pela primeira vez lê onde está o "Jogar". */
  .trilho-nome {
    font-size: 10.5px;
    font-weight: 500;
    line-height: 1;
    white-space: nowrap;
  }
</style>
