<script lang="ts">
  // Teste de reflexo (Plano 1.1, Onda 3b): a área fica verde depois de um tempo sorteado, e a pessoa clica o mais rápido
  // que puder. Cinco rodadas, vale a mediana (uma rodada distraída não decide). O número junta o reflexo da pessoa com o
  // atraso do PC (mouse, sistema, tela): por isso serve para comparar ANTES e DEPOIS no mesmo PC, nunca entre PCs.
  // O resultado anterior fica guardado só neste computador, para a comparação.

  const RODADAS = 5;
  const CHAVE = "otimiza.reflexo";

  type Fase = "parado" | "esperando" | "agora" | "cedo" | "pronto";
  type Guardado = { ms: number; quando: number };

  let fase = $state<Fase>("parado");
  let tempos = $state<number[]>([]);
  let anterior = $state<Guardado | null>(ler());
  let ultimo = $state<Guardado | null>(null);
  let verdeEm = 0;
  let espera: ReturnType<typeof setTimeout> | undefined;

  function ler(): Guardado | null {
    try {
      const v = JSON.parse(localStorage.getItem(CHAVE) ?? "null");
      return v && typeof v.ms === "number" ? v : null;
    } catch {
      return null;
    }
  }

  function guardar(g: Guardado) {
    try {
      localStorage.setItem(CHAVE, JSON.stringify(g));
    } catch {
      // Sem armazenamento: só não compara da próxima vez.
    }
  }

  function mediana(v: number[]) {
    const o = [...v].sort((a, b) => a - b);
    return o[Math.floor(o.length / 2)];
  }

  function rodada() {
    fase = "esperando";
    clearTimeout(espera);
    espera = setTimeout(
      () => {
        fase = "agora";
        // O instante em que o verde foi pintado, não o do timer: o atraso até a tela entra na conta, como no jogo.
        requestAnimationFrame(() => (verdeEm = performance.now()));
      },
      1200 + Math.random() * 2600,
    );
  }

  function comecar() {
    tempos = [];
    ultimo = null;
    anterior = ler();
    rodada();
  }

  function apertar(e: PointerEvent) {
    if (e.button !== 0) return;
    if (fase === "esperando") {
      clearTimeout(espera);
      fase = "cedo";
      return;
    }
    if (fase === "agora" && verdeEm > 0) {
      tempos = [...tempos, Math.max(0, e.timeStamp - verdeEm)];
      verdeEm = 0;
      if (tempos.length >= RODADAS) {
        const g = { ms: Math.round(mediana(tempos)), quando: Date.now() };
        ultimo = g;
        guardar(g);
        fase = "pronto";
      } else {
        rodada();
      }
      return;
    }
    if (fase === "cedo") rodada();
    else if (fase === "parado" || fase === "pronto") comecar();
  }

  const diferenca = $derived(ultimo && anterior ? ultimo.ms - anterior.ms : null);

  function data(t: number) {
    return new Date(t).toLocaleDateString("pt-BR", { day: "numeric", month: "short" });
  }
</script>

<div class="reflexo">
  <button class="alvo" data-fase={fase} onpointerdown={apertar} aria-live="polite">
    {#if fase === "parado"}
      <strong>Clique para começar</strong>
      <span>Quando ficar verde, clique o mais rápido que puder. {RODADAS} rodadas.</span>
    {:else if fase === "esperando"}
      <strong>Espere o verde…</strong>
      <span>Rodada {tempos.length + 1} de {RODADAS}</span>
    {:else if fase === "agora"}
      <strong>Agora!</strong>
    {:else if fase === "cedo"}
      <strong>Cedo demais</strong>
      <span>Clique para repetir esta rodada.</span>
    {:else if ultimo}
      <strong class="numero">{ultimo.ms} ms</strong>
      <span>Mediana de {RODADAS} rodadas · clique para medir de novo</span>
    {/if}
  </button>

  {#if tempos.length > 0 && fase !== "pronto"}
    <p class="apoio">{tempos.map((t) => `${Math.round(t)} ms`).join(" · ")}</p>
  {/if}

  {#if fase === "pronto" && ultimo}
    <p class="apoio">
      {#if anterior && diferenca !== null}
        Antes ({data(anterior.quando)}): {anterior.ms} ms.
        {#if Math.abs(diferenca) < 8}
          Praticamente igual: a diferença está dentro do que varia de uma tentativa para outra.
        {:else if diferenca < 0}
          {-diferenca} ms mais rápido agora.
        {:else}
          {diferenca} ms mais lento agora (cansaço conta: repita descansado antes de concluir).
        {/if}
      {:else}
        Guardado. Meça de novo depois de otimizar para ver a diferença. O número junta o seu reflexo e o atraso do PC:
        compare só neste PC.
      {/if}
    </p>
  {/if}
</div>

<style>
  .reflexo {
    display: grid;
    gap: 10px;
  }
  .alvo {
    display: grid;
    place-content: center;
    gap: 6px;
    min-height: 168px;
    border: 0;
    border-radius: var(--raio-3);
    background: var(--superficie-2);
    color: var(--tinta);
    font: inherit;
    text-align: center;
    cursor: pointer;
    user-select: none;
    touch-action: manipulation;
  }
  .alvo strong {
    font-size: 20px;
    font-weight: 600;
  }
  .alvo .numero {
    font-size: var(--t-numero);
  }
  .alvo span {
    color: var(--tinta-2);
    font-size: 13px;
  }
  .alvo[data-fase="esperando"] {
    background: color-mix(in srgb, var(--perigo) 16%, var(--superficie-2));
  }
  .alvo[data-fase="agora"] {
    background: var(--ok);
    color: #fff;
  }
  .alvo[data-fase="cedo"] {
    background: color-mix(in srgb, var(--atencao) 22%, var(--superficie-2));
  }
  .apoio {
    margin: 0;
  }
</style>
