<script lang="ts" generics="T extends string">
  // As abas acessíveis (1.0.12, Frentes 1.1 e 1.3): `role="tablist"` com `tab` e `aria-selected`, só a aba escolhida
  // no Tab, setas para andar (dão a volta), Home e End. A aba focada já abre (`lib/abas.ts`). Cada aba aponta o painel
  // que ela mostra (`idDoPainel`); o painel leva `role="tabpanel"` e `aria-labelledby={idDaAba(...)}`.
  //
  // Dois desenhos: "segmento" (o seletor dentro de uma área) e "barra" (as áreas embaixo, com o marcador que desliza
  // até a aba escolhida).
  import { onMount, tick } from "svelte";
  import { indiceDeTab, proximaAba, type Orientacao } from "../../lib/abas";

  let {
    abas,
    ativa,
    rotulo,
    prefixo,
    idDoPainel = null,
    variante = "segmento",
    orientacao = "horizontal",
    onmudar,
  }: {
    abas: { id: T; rotulo: string; desligada?: boolean }[];
    ativa: T;
    /** O nome do conjunto para o leitor de tela ("Partes do Sistema"). */
    rotulo: string;
    /** Prefixo único dos ids (`idDaAba`). */
    prefixo: string;
    /** O id do painel que as abas controlam (um painel só, que troca de conteúdo). */
    idDoPainel?: string | null;
    variante?: "segmento" | "barra";
    orientacao?: Orientacao;
    onmudar: (id: T) => void;
  } = $props();

  let lista: HTMLElement | undefined = $state();
  let marcador = $state({ x: 0, w: 0 });

  const habilitadas = $derived(abas.filter((a) => !a.desligada));

  function idDe(id: T) {
    return `${prefixo}-aba-${id}`;
  }

  async function teclado(e: KeyboardEvent) {
    const atual = habilitadas.findIndex((a) => a.id === ativa);
    const proximo = proximaAba(atual, e.key, habilitadas.length, orientacao);
    if (proximo === null) return;
    e.preventDefault();
    const alvo = habilitadas[proximo];
    onmudar(alvo.id);
    await tick();
    lista?.querySelector<HTMLElement>(`#${CSS.escape(idDe(alvo.id))}`)?.focus();
  }

  async function medir() {
    await tick();
    const alvo = lista?.querySelector<HTMLElement>('[aria-selected="true"]');
    marcador = alvo ? { x: alvo.offsetLeft, w: alvo.offsetWidth } : { x: marcador.x, w: 0 };
  }
  $effect(() => {
    void ativa;
    void abas.length;
    if (variante === "barra") void medir();
  });
  onMount(() => {
    if (variante !== "barra") return;
    const observador = new ResizeObserver(() => void medir());
    if (lista) observador.observe(lista);
    return () => observador.disconnect();
  });
</script>

<div
  bind:this={lista}
  class="abas-acessiveis"
  class:segmento={variante === "segmento"}
  class:barra={variante === "barra"}
  role="tablist"
  aria-label={rotulo}
  aria-orientation={orientacao}
  tabindex="-1"
  onkeydown={teclado}
>
  {#if variante === "barra"}
    <span
      class="marcador"
      aria-hidden="true"
      style:transform={`translateX(${marcador.x}px)`}
      style:width={`${marcador.w}px`}
      style:opacity={marcador.w ? 1 : 0}
    ></span>
  {/if}
  {#each abas as a (a.id)}
    <button
      role="tab"
      id={idDe(a.id)}
      class="aba"
      aria-selected={a.id === ativa}
      aria-controls={idDoPainel ?? undefined}
      tabindex={indiceDeTab(a.id === ativa)}
      disabled={a.desligada}
      onclick={() => onmudar(a.id)}
    >
      {a.rotulo}
    </button>
  {/each}
</div>

<style>
  .abas-acessiveis:focus {
    outline: none;
  }
  /* "barra": as áreas da aba do trilho, embaixo. A escolhida ganha o marcador cheio que desliza até ela. */
  .barra {
    position: relative;
    display: flex;
    align-items: center;
    gap: 2px;
  }
  .barra .marcador {
    position: absolute;
    left: 0;
    top: 0;
    height: 100%;
    border-radius: var(--raio-pilula);
    background: var(--acao);
    transition:
      transform var(--mov-entrar) var(--curva-padrao),
      width var(--mov-entrar) var(--curva-padrao),
      opacity var(--mov-medio) ease;
  }
  .barra .aba {
    position: relative;
    height: 36px;
    padding: 0 var(--e-4);
    border: 0;
    border-radius: var(--raio-pilula);
    background: transparent;
    color: var(--tinta-2);
    font-size: var(--t-linha);
    font-weight: 500;
    white-space: nowrap;
    cursor: pointer;
    transition: color var(--mov-medio) ease;
  }
  .barra .aba:hover:not([aria-selected="true"]) {
    color: var(--tinta);
  }
  .barra .aba[aria-selected="true"] {
    color: var(--contra);
    font-weight: 600;
  }
  .barra .aba:focus-visible {
    outline-offset: 3px;
  }
</style>
