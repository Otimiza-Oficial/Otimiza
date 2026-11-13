<script lang="ts">
  import { como } from "../lib/contrato";
  import { commands } from "../lib/tipos";
  // O comparador da escala (trazido da interface 3.3/3.6, `comparadorDaEscala`): o mesmo pedaço do centro do jogo,
  // ampliado do jeito comum (como um monitor faria) e pelo FSR do Otimiza, com um divisor arrastável. A captura só sai
  // com o jogo na frente, então o pedido espera o próximo quadro dele (`escala_pedir_comparacao`) e a tela confere a
  // cada segundo se já chegou (`escala_comparacao`), por até 2 minutos.
  import { onDestroy } from "svelte";
    import { noTauri } from "../lib/ponte";

  type Comparacao = { jogo: string; tela: string; jogo_px: [number, number]; tela_px: [number, number] };

  let etapa = $state<"parado" | "esperando" | "pronto">("parado");
  let comparacao = $state<Comparacao | null>(null);
  let recado = $state<string | null>(null);
  let divisor = $state(50);
  let vigia: ReturnType<typeof setInterval> | undefined;

  onDestroy(() => clearInterval(vigia));

  async function pedir() {
    clearInterval(vigia);
    recado = null;
    comparacao = null;
    if (!noTauri) {
      recado = "No app, a comparação é tirada do próximo quadro do jogo.";
      return;
    }
    try {
      await commands.escalaPedirComparacao();
    } catch (e) {
      recado = String(e);
      return;
    }
    etapa = "esperando";
    let tentativas = 0;
    vigia = setInterval(async () => {
      tentativas += 1;
      const c = await como<Comparacao | null>(commands.escalaComparacao()).catch(() => null);
      if (c) {
        clearInterval(vigia);
        comparacao = c;
        divisor = 50;
        etapa = "pronto";
      } else if (tentativas >= 120) {
        clearInterval(vigia);
        etapa = "parado";
        recado = "O jogo não voltou para a frente em 2 minutos. Peça a comparação de novo.";
      }
    }, 1000);
  }

  function fechar() {
    clearInterval(vigia);
    etapa = "parado";
    comparacao = null;
    recado = null;
  }
</script>

<div class="comparador">
  <div class="cabeca">
    <div>
      <span>Comparar a imagem</span>
      <p class="apoio">O centro da tela, de perto: o jogo ampliado do jeito comum ao lado do FSR do Otimiza.</p>
    </div>
    {#if etapa === "parado"}
      <button class="btn" onclick={pedir}>Comparar</button>
    {:else}
      <button class="btn quieto" onclick={fechar}>Fechar</button>
    {/if}
  </div>

  {#if etapa === "esperando"}
    <p class="apoio">Volte ao jogo por um instante: a comparação é tirada no próximo quadro dele. Depois volte aqui.</p>
  {:else if etapa === "pronto" && comparacao}
    {@const c = comparacao}
    <div class="quadro" style:aspect-ratio="{c.tela_px[0]} / {c.tela_px[1]}">
      <img src={c.jogo} alt="O jogo ampliado do jeito comum" />
      <img src={c.tela} alt="O mesmo pedaço ampliado pelo FSR do Otimiza" style:clip-path="inset(0 0 0 {divisor}%)" />
      <div class="risco" style:left="{divisor}%"></div>
    </div>
    <input type="range" min="0" max="100" bind:value={divisor} aria-label="Divisor entre as duas imagens" />
    <p class="apoio pequeno">
      À esquerda, o jogo em {c.jogo_px[0]}×{c.jogo_px[1]} ampliado do jeito comum. À direita, o mesmo pedaço ampliado
      pelo FSR do Otimiza para {c.tela_px[0]}×{c.tela_px[1]}.
    </p>
  {/if}
  {#if recado}<p class="apoio pequeno">{recado}</p>{/if}
</div>

<style>
  .comparador {
    display: grid;
    gap: 8px;
    padding-top: 10px;
    border-top: 1px solid var(--linha);
  }
  .cabeca {
    display: flex;
    gap: 12px;
    align-items: center;
    justify-content: space-between;
  }
  .quadro {
    position: relative;
    overflow: hidden;
    border-radius: var(--raio-1);
    background: #000;
  }
  .quadro img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    image-rendering: auto;
  }
  .risco {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 2px;
    margin-left: -1px;
    background: #fff;
    box-shadow: 0 0 6px rgb(0 0 0 / 0.6);
    pointer-events: none;
  }
  input[type="range"] {
    width: 100%;
  }
  p {
    margin: 0;
  }
  .pequeno {
    font-size: 12px;
  }
</style>
