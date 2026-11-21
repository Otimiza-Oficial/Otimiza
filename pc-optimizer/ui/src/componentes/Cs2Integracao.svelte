<script lang="ts">
  import Cartao from "./base/Cartao.svelte";
  import Esqueleto from "./base/Esqueleto.svelte";
  // A integração oficial do CS2 (Game State Integration) no destaque do jogo: ligar escreve um arquivo na pasta `cfg`
  // do CS2, e o jogo passa a contar ao Otimiza o momento da partida. Com ela, a medição automática só mede rodada
  // valendo com você vivo. Desligar apaga o arquivo.
  import { onDestroy, onMount } from "svelte";
  import { ponte, type PainelDoCs2 } from "../lib/ponte";

  const MOMENTO = {
    menu: "No menu",
    pausa: "Aquecimento, intervalo ou começo de rodada",
    jogando: "Rodada valendo, você vivo",
    assistindo: "Morto ou assistindo",
  } as const;

  let painel = $state<PainelDoCs2 | null>(null);
  let mudando = $state(false);
  let erro = $state<string | null>(null);
  let recemLigado = $state(false);
  let lido = $state(false);
  let relogio: ReturnType<typeof setInterval> | undefined;

  async function ler() {
    painel = await ponte.cs2Painel().catch(() => painel);
    lido = true;
  }

  onMount(() => {
    void ler();
    // Com a janela escondida (o jogo na frente) não pergunta nada.
    relogio = setInterval(() => {
      if (!document.hidden) void ler();
    }, 3000);
    document.addEventListener("visibilitychange", aoVoltar);
  });
  function aoVoltar() {
    if (!document.hidden) void ler();
  }
  onDestroy(() => {
    clearInterval(relogio);
    document.removeEventListener("visibilitychange", aoVoltar);
  });

  async function alternar() {
    if (!painel) return;
    mudando = true;
    erro = null;
    try {
      const ligar = !painel.ligado;
      painel = ligar ? await ponte.cs2Ligar() : await ponte.cs2Desligar();
      recemLigado = ligar;
    } catch (e) {
      erro = String(e);
    } finally {
      mudando = false;
    }
  }
</script>

{#if !lido}
  <Cartao titulo="Integração oficial do CS2"><Esqueleto linhas={2} rotulo="Procurando o CS2" /></Cartao>
{:else if !painel}
  <Cartao titulo="Integração oficial do CS2"><p class="aviso">Não consegui ler a pasta do CS2 agora.</p></Cartao>
{:else if !painel.instalado}
  <Cartao titulo="Integração oficial do CS2">
    <p class="apoio">Não achei o CS2 instalado pela Steam neste PC. Depois de instalar, a integração liga daqui.</p>
  </Cartao>
{:else}
  <Cartao titulo="Integração oficial do CS2">
    {#snippet selo()}
      <button
        class="interruptor"
        role="switch"
        aria-checked={painel?.ligado ?? false}
        aria-label="Integração oficial do CS2"
        disabled={mudando}
        onclick={alternar}
      ></button>
    {/snippet}
      <span class="apoio">
        O próprio CS2 conta ao Otimiza o momento da partida, pelo recurso da Valve que os overlays de torneio usam. Assim
        a medição só conta rodada valendo, com você vivo. Não mexe no jogo além de um arquivo na pasta de configuração.
      </span>
      {#if painel.ligado && painel.estado}
        <span class="estado">
          <span class="ponto" data-tom={painel.estado.momento === "jogando" ? "ok" : "neutro"}></span>
          {MOMENTO[painel.estado.momento]}{painel.estado.mapa ? ` · ${painel.estado.mapa}` : ""}{painel.estado.rodada
            ? ` · rodada ${painel.estado.rodada}`
            : ""}
        </span>
      {:else if painel.ligado}
        <span class="estado">
          <span class="ponto" data-tom="neutro"></span>
          {recemLigado ? "Ligada. Feche e abra o CS2 para valer." : "Ligada. Esperando o CS2 abrir."}
        </span>
      {/if}
      {#if erro}<span class="erro">{erro}</span>{/if}
  </Cartao>
{/if}

<style>
  .estado {
    display: flex;
    gap: 8px;
    align-items: center;
    font-size: var(--t-linha);
  }
  .erro {
    color: var(--perigo);
    font-size: var(--t-linha);
  }
</style>
