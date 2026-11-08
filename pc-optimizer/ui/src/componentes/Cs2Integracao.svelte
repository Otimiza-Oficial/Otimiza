<script lang="ts">
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
  let relogio: ReturnType<typeof setInterval> | undefined;

  async function ler() {
    painel = await ponte.cs2Painel().catch(() => painel);
  }

  onMount(() => {
    void ler();
    relogio = setInterval(ler, 3000);
  });
  onDestroy(() => clearInterval(relogio));

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

{#if painel?.instalado}
  <div class="cs2">
    <div class="texto">
      <span class="titulo" id="cs2-gsi">Integração oficial do CS2</span>
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
    </div>
    <button
      class="interruptor"
      role="switch"
      aria-checked={painel.ligado}
      aria-labelledby="cs2-gsi"
      disabled={mudando}
      onclick={alternar}
    ></button>
  </div>
{/if}

<style>
  .cs2 {
    display: flex;
    gap: 20px;
    align-items: flex-start;
    padding: 14px 16px;
    border-radius: var(--raio-2);
    background: var(--superficie-2);
  }
  .texto {
    flex: 1;
    display: grid;
    gap: 4px;
  }
  .titulo {
    font-weight: 600;
  }
  .apoio {
    max-width: 62ch;
  }
  .estado {
    display: flex;
    gap: 8px;
    align-items: center;
    font-size: 13px;
  }
  .erro {
    color: var(--perigo);
    font-size: 13px;
  }
</style>
