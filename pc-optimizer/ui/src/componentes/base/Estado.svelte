<script lang="ts">
  // O estado de um bloco quando ainda não há o conteúdo (1.0.12, Frente 1.1 e 1.7). Quatro casos, sempre desenhados e
  // sempre com o que fazer:
  // - lendo: barras no formato do que vem (o `Esqueleto`), nunca "Lendo…" solto;
  // - vazio: não há nada ainda, e qual ação faz aparecer ("Abra o jogo uma vez e feche");
  // - erro: o que aconteceu, em palavras do jogador, e o que fazer; o detalhe técnico fica recolhido;
  // - sem-admin: o que precisa de administrador e como abrir assim.
  // "Não consegui ler" é erro, não vazio: quem lê "nada aqui" acha que está tudo certo.
  import type { Snippet } from "svelte";
  import Esqueleto from "./Esqueleto.svelte";
  import Oti from "../Oti.svelte";

  let {
    tipo,
    titulo = null,
    fazer = null,
    detalhe = null,
    linhas = 3,
    oti = false,
    compacto = false,
    acao,
  }: {
    tipo: "lendo" | "vazio" | "erro" | "sem-admin";
    /** O que aconteceu (vazio, erro, sem-admin) ou o que está sendo lido (lendo, só para o leitor de tela). */
    titulo?: string | null;
    /** O próximo passo, em uma frase. Obrigatório fora do "lendo": todo estado diz o que fazer. */
    fazer?: string | null;
    /** A mensagem técnica (o erro do Windows): recolhida, para quem for mandar ao suporte. */
    detalhe?: string | null;
    linhas?: number;
    /** O Oti ao lado do estado vazio: só nos vazios de primeira vez, nunca em erro. */
    oti?: boolean;
    /** Sem a margem de cima e de baixo, para dentro de uma linha de lista. */
    compacto?: boolean;
    /** O botão do próximo passo (um `Botao`). */
    acao?: Snippet;
  } = $props();

  const TITULO_PADRAO = {
    lendo: "Carregando",
    vazio: "Nada aqui ainda",
    erro: "Não consegui ler isto agora",
    "sem-admin": "Precisa do Otimiza como administrador",
  } as const;
  const FAZER_PADRAO = {
    lendo: null,
    vazio: null,
    erro: "Tente de novo em alguns segundos. Se repetir, mande o arquivo de registro (Ajustes › Suporte).",
    "sem-admin": "Feche o Otimiza e abra de novo com o botão direito › Executar como administrador.",
  } as const;
</script>

{#if tipo === "lendo"}
  <Esqueleto {linhas} rotulo={titulo ?? TITULO_PADRAO.lendo} />
{:else}
  <div class="estado" class:compacto data-tipo={tipo} role={tipo === "erro" ? "alert" : "status"}>
    {#if oti && tipo === "vazio"}<span class="oti-do-estado"><Oti tamanho={40} rotulo="" /></span>{/if}
    <div class="textos">
      <p class="titulo-estado">
        {#if tipo !== "vazio"}<span class="ponto" data-tom={tipo === "erro" ? "perigo" : "atencao"} aria-hidden="true"></span>{/if}
        {titulo ?? TITULO_PADRAO[tipo]}
      </p>
      {#if fazer ?? FAZER_PADRAO[tipo]}<p class="apoio fazer">{fazer ?? FAZER_PADRAO[tipo]}</p>{/if}
      {#if acao}<div class="acao-estado">{@render acao()}</div>{/if}
      {#if detalhe}
        <details>
          <summary class="apoio">Detalhe para o suporte</summary>
          <code class="detalhe">{detalhe}</code>
        </details>
      {/if}
    </div>
  </div>
{/if}

<style>
  .estado {
    display: flex;
    align-items: flex-start;
    gap: var(--e-3);
    padding: var(--e-4) 0;
  }
  .estado.compacto {
    padding: 0;
  }
  .oti-do-estado {
    flex: none;
    padding-top: 2px;
  }
  .textos {
    display: grid;
    gap: var(--e-1);
    justify-items: start;
    min-width: 0;
  }
  .titulo-estado {
    display: flex;
    align-items: center;
    gap: var(--e-2);
    font-size: var(--t-corpo);
    font-weight: 600;
  }
  .fazer {
    font-size: var(--t-linha);
    line-height: 1.5;
    max-width: 60ch;
  }
  .acao-estado {
    margin-top: var(--e-2);
  }
  details {
    margin-top: var(--e-1);
  }
  .detalhe {
    display: block;
    margin-top: var(--e-1);
    padding: var(--e-2) var(--e-3);
    border-radius: var(--raio-1);
    background: var(--superficie-2);
    color: var(--tinta-2);
    font-size: var(--t-apoio);
    white-space: pre-wrap;
    word-break: break-word;
    user-select: text;
  }
</style>
