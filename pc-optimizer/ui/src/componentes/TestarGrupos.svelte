<script lang="ts">
  // Testar um grupo de cada vez (trazido da interface 3.6, `carregarProtocolo`): vinte ajustes juntos dão um número e
  // nenhuma informação; seis grupos em sequência dão seis respostas, que valem para sempre nesta máquina. Só mostra
  // o estado de cada grupo (`protocolo_de_grupos`); aplicar continua no Otimizar e desfazer no Histórico. "Não mudou
  // nada" é neutro de propósito: saber que não rende é o que deixa parar de mexer.
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { noTauri } from "../lib/ponte";

  type Lado = { fps: number; low_1pct: number; amostras: number };
  type Decisao =
    | { decisao: "Esperar"; falta: string }
    | { decisao: "Manter"; ganho_pct: number }
    | { decisao: "NaoMudouNada" }
    | { decisao: "ReverterSozinho"; queda_pct: number }
    | { decisao: "PiorouMasNaoReverto"; queda_pct: number; porque: string };
  type Experimento = {
    grupo: string;
    letra: string;
    nome: string;
    descricao: string;
    itens: string[];
    exige_reinicio: boolean;
    decisao: Decisao;
    veredito: { antes: Lado | null; depois: Lado | null; variacao_fps_pct: number | null } | null;
  };

  const ROTULO: Record<Decisao["decisao"], { texto: string; tom: "ok" | "atencao" | null }> = {
    Esperar: { texto: "ainda medindo", tom: null },
    Manter: { texto: "rendeu aqui", tom: "ok" },
    NaoMudouNada: { texto: "não mudou nada aqui", tom: null },
    ReverterSozinho: { texto: "piorou, o Otimiza desfaz", tom: "atencao" },
    PiorouMasNaoReverto: { texto: "piorou", tom: "atencao" },
  };

  let grupos = $state<Experimento[] | null>(null);
  let recusa = $state<string | null>(null);

  onMount(async () => {
    if (!noTauri) {
      grupos = DEMO;
      return;
    }
    try {
      grupos = await invoke<Experimento[]>("protocolo_de_grupos", { jogo: null });
    } catch (e) {
      // A recusa explica o que fazer: é a primeira instrução do protocolo.
      recusa = String(e);
    }
  });

  const int = (v: number) => v.toLocaleString("pt-BR", { maximumFractionDigits: 0 });

  function frase(d: Decisao): string {
    switch (d.decisao) {
      case "Esperar":
        return d.falta;
      case "Manter":
        return `Rendeu ${int(d.ganho_pct)}% nesta máquina. Mantenha.`;
      case "NaoMudouNada":
        return "Medido dos dois lados, a diferença ficou dentro da variação normal entre duas partidas. Não rende aqui.";
      case "ReverterSozinho":
        return `Caiu ${int(Math.abs(d.queda_pct))}% depois deste grupo. Como ele não exige reiniciar, a comparação é limpa e o Otimiza desfaz.`;
      case "PiorouMasNaoReverto":
        return `Caiu ${int(Math.abs(d.queda_pct))}% depois deste grupo. ${d.porque}`;
    }
  }

  const DEMO: Experimento[] = [
    {
      grupo: "energia",
      letra: "A",
      nome: "Energia",
      descricao: "Plano de energia e o processador sem dormir no jogo.",
      itens: ["plano", "estacionamento", "boost"],
      exige_reinicio: false,
      decisao: { decisao: "Manter", ganho_pct: 6 },
      veredito: { antes: { fps: 79, low_1pct: 38, amostras: 4 }, depois: { fps: 84, low_1pct: 44, amostras: 4 }, variacao_fps_pct: 6.3 },
    },
    {
      grupo: "rede",
      letra: "B",
      nome: "Rede",
      descricao: "Pacotes pequenos na hora e a fila da placa de rede.",
      itens: ["nagle", "throttling"],
      exige_reinicio: true,
      decisao: { decisao: "Esperar", falta: "Faltam 2 partidas medidas depois de aplicar." },
      veredito: null,
    },
  ];
</script>

<section class="secao">
  <h2>Testar um grupo de cada vez</h2>
  <p class="apoio texto">
    Vinte ajustes juntos dão um número e nenhuma informação. Grupos testados em sequência dão uma resposta cada, e ela
    vale para sempre nesta máquina. Aplique pelo Otimizar; desfazer fica no Histórico.
  </p>
  {#if recusa}
    <p class="apoio">{recusa}</p>
  {:else if grupos === null}
    <p class="apoio">Lendo…</p>
  {:else}
    <ul class="grupos">
      {#each grupos as g (g.grupo)}
        {@const r = ROTULO[g.decisao.decisao]}
        {@const v = g.veredito}
        <li>
          <div class="cabeca">
            <strong>{g.letra} · {g.nome}</strong>
            <span class="estado">{#if r.tom}<span class="ponto" data-tom={r.tom}></span>{/if}{r.texto}</span>
          </div>
          <p class="apoio">{g.descricao} {g.itens.length === 1 ? "1 ajuste" : `${g.itens.length} ajustes`}, {g.exige_reinicio ? "exige reiniciar" : "sem reiniciar"}.</p>
          {#if v?.antes && v.depois}
            <p class="numeros">
              Antes {int(v.antes.fps)} FPS, 1% pior {int(v.antes.low_1pct)} <span class="apoio">({v.antes.amostras} medições)</span> · depois
              {int(v.depois.fps)} FPS, 1% pior {int(v.depois.low_1pct)} <span class="apoio">({v.depois.amostras} medições)</span>{#if v.variacao_fps_pct !== null}<span class="numero">{` · ${v.variacao_fps_pct > 0 ? "+" : ""}${int(v.variacao_fps_pct)}%`}</span>{/if}
            </p>
          {/if}
          <p class="pequeno">{frase(g.decisao)}</p>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .texto {
    max-width: 64ch;
  }
  .grupos {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
  }
  .grupos li {
    display: grid;
    gap: 3px;
    padding: 10px 0;
    border-top: 1px solid var(--linha);
  }
  .grupos li:first-child {
    border-top: 0;
  }
  .cabeca {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 12px;
  }
  .estado {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--tinta-2);
    font-size: 12px;
    white-space: nowrap;
  }
  .numeros {
    font-size: 13px;
  }
  p {
    margin: 0;
  }
  .pequeno {
    font-size: 12px;
  }
</style>
