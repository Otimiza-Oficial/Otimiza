<script lang="ts">
  // As 4 abas do trilho da esquerda (a intenção). Abas de verdade (1.0.12, Frente 1.3): `tablist` vertical, setas de
  // cima e de baixo, Home e End, só a escolhida no Tab. A posição e a superfície são do contêiner, na casca.
  import { tick } from "svelte";
  import Icone, { type NomeDoIcone } from "./Icone.svelte";
  import { grupos, type Grupo, type IdDoGrupo } from "../lib/areas";
  import { indiceDeTab, proximaAba } from "../lib/abas";

  let { ativo, onabrir }: { ativo: IdDoGrupo; onabrir: (g: Grupo) => void } = $props();

  const ICONE_DO_GRUPO: Record<IdDoGrupo, NomeDoIcone> = {
    jogar: "controle",
    meu_pc: "monitor",
    prova: "relogio",
    ajustes: "ajustes",
  };

  let lista: HTMLElement | undefined = $state();

  async function teclado(e: KeyboardEvent) {
    const proximo = proximaAba(
      grupos.findIndex((g) => g.id === ativo),
      e.key,
      grupos.length,
      "vertical",
    );
    if (proximo === null) return;
    e.preventDefault();
    onabrir(grupos[proximo]);
    await tick();
    lista?.querySelector<HTMLElement>(`#trilho-${grupos[proximo].id}`)?.focus();
  }
</script>

<div bind:this={lista} class="trilho-lista" role="tablist" aria-label="Seções" aria-orientation="vertical" tabindex="-1" onkeydown={teclado}>
  {#each grupos as g (g.id)}
    <button
      id={`trilho-${g.id}`}
      class="trilho-botao"
      class:ativo={ativo === g.id}
      role="tab"
      aria-selected={ativo === g.id}
      aria-controls="painel-da-area"
      aria-label={g.nome}
      tabindex={indiceDeTab(ativo === g.id)}
      onclick={() => onabrir(g)}
    >
      <Icone nome={ICONE_DO_GRUPO[g.id]} tamanho={20} />
      <span class="trilho-nome" aria-hidden="true">{g.nome}</span>
    </button>
  {/each}
</div>

<style>
  .trilho-lista {
    display: grid;
    gap: var(--e-1);
  }
  .trilho-lista:focus {
    outline: none;
  }
  .trilho-botao {
    display: grid;
    place-items: center;
    align-content: center;
    gap: 5px;
    width: 64px;
    height: 60px;
    border: 0;
    border-radius: var(--raio-3);
    background: transparent;
    color: var(--tinta-2);
    cursor: pointer;
    transition:
      background var(--mov-rapido) ease,
      color var(--mov-rapido) ease;
  }
  .trilho-botao:hover {
    background: var(--acao-suave);
    color: var(--tinta);
  }
  /* O lugar onde você está: a superfície de cima e o texto cheio. Sem cor: a marca é monocromática. */
  .trilho-botao.ativo {
    background: var(--superficie-3);
    color: var(--tinta);
    box-shadow: var(--fio);
  }
  .trilho-botao:focus-visible {
    outline-offset: -2px;
  }
  /* O nome da aba sob o ícone: quatro abas, e quem abre pela primeira vez lê onde está o "Jogar". */
  .trilho-nome {
    font-size: var(--t-apoio);
    font-weight: 500;
    line-height: 1;
    white-space: nowrap;
  }
</style>
