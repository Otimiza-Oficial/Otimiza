<script lang="ts" module>
  // A lista de todos os jogos vem numa chamada só (`o_que_mudou`): fica guardada enquanto a janela estiver aberta, e
  // trocar de jogo na área Jogos não pede de novo.
  let todas: Promise<LinhaDoJogo[] | null> | null = null;

  type CampoDaTroca = "Driver" | "Windows" | "Placa" | "Bios" | "Otimiza";
  type Marco =
    | { tipo: "Partida"; quando: number; fps: number; low_1pct: number; fonte: "PresentMon" | "CanalAntigo"; comparavel: boolean }
    | { tipo: "Troca"; quando: number; troca: { campo: CampoDaTroca; de: string; para: string } }
    | { tipo: "Otimiza"; quando: number; nome: string };
  export type LinhaDoJogo = {
    jogo: string;
    marcos: Marco[];
    julgamento:
      | {
          estado: "Caiu";
          queda: {
            mudou: { tipo: "Driver" | "Windows"; de: string; para: string } | { tipo: "Nada" };
            fps_antes: number;
            fps_depois: number;
            queda_pct: number;
            partidas_antes: number;
            partidas_depois: number;
          };
        }
      | { estado: "NaoCaiu" }
      | { estado: "PoucasPartidas"; comparaveis: number; precisa: number };
    partidas: number;
    otimiza_antes_da_primeira: number;
  };
</script>

<script lang="ts">
  // O que mudou neste jogo (trazido da interface 3.6, `desenharLinhaDoJogo`): as partidas medidas, o que trocou em volta
  // da máquina (driver, Windows, placa, BIOS, versão do Otimiza) e o que o Otimiza mudou no meio. Estar no mesmo período
  // não prova que foi a causa: a queda só é dita quando as partidas de antes e de depois passam da margem de erro.
  import { invoke } from "@tauri-apps/api/core";
  import { noTauri } from "../lib/ponte";

  let { jogo }: { jogo: string } = $props();

  const NOME_DO_CAMPO: Record<CampoDaTroca, string> = {
    Driver: "Driver de vídeo",
    Windows: "Windows",
    Placa: "Placa de vídeo",
    Bios: "BIOS",
    Otimiza: "Versão do Otimiza",
  };
  const NA_TELA = 15;

  let linha = $state<LinhaDoJogo | null | undefined>(undefined);
  let erro = $state<string | null>(null);

  $effect(() => {
    const nome = jogo;
    linha = undefined;
    todas ??= noTauri
      ? invoke<LinhaDoJogo[]>("o_que_mudou").catch((e) => {
          erro = String(e);
          todas = null;
          return null;
        })
      : Promise.resolve(DEMO);
    void todas.then((l) => {
      if (nome === jogo) linha = l?.find((x) => x.jogo.toLowerCase() === nome.toLowerCase()) ?? null;
    });
  });

  const n = (v: number) => v.toLocaleString("pt-BR", { maximumFractionDigits: 0 });
  const data = (q: number) => new Date(q * 1000).toLocaleString("pt-BR", { dateStyle: "short", timeStyle: "short" });

  function veredito(l: LinhaDoJogo): string {
    const j = l.julgamento;
    if (j.estado === "Caiu") {
      const q = j.queda;
      const onde =
        q.mudou.tipo === "Nada"
          ? "nas últimas partidas contra as anteriores"
          : `antes e depois da troca de ${q.mudou.tipo === "Driver" ? "driver de vídeo" : "Windows"} (${q.mudou.de} → ${q.mudou.para})`;
      return `O FPS médio caiu ${n(q.queda_pct)}%, de ${n(q.fps_antes)} para ${n(q.fps_depois)}, ${onde}: ${q.partidas_antes} partidas antes, ${q.partidas_depois} depois.`;
    }
    if (j.estado === "NaoCaiu") return "O FPS médio das últimas partidas não caiu além da margem de erro. O 1% pior não entra nesta conta.";
    return `${j.comparaveis === 1 ? "1 partida comparável" : `${j.comparaveis} partidas comparáveis`} de ${j.precisa} necessárias: ainda não dá para dizer se caiu. Partidas com gerador de quadros não contam.`;
  }

  const DEMO: LinhaDoJogo[] = [
    {
      jogo: "FiveM",
      partidas: 12,
      otimiza_antes_da_primeira: 1,
      julgamento: {
        estado: "Caiu",
        queda: { mudou: { tipo: "Driver", de: "560.94", para: "566.03" }, fps_antes: 92, fps_depois: 78, queda_pct: 15.2, partidas_antes: 6, partidas_depois: 4 },
      },
      marcos: [
        { tipo: "Partida", quando: Date.now() / 1000 - 3600, fps: 78, low_1pct: 39, fonte: "PresentMon", comparavel: true },
        { tipo: "Troca", quando: Date.now() / 1000 - 86400 * 2, troca: { campo: "Driver", de: "560.94", para: "566.03" } },
        { tipo: "Partida", quando: Date.now() / 1000 - 86400 * 3, fps: 93, low_1pct: 47, fonte: "PresentMon", comparavel: true },
        { tipo: "Otimiza", quando: Date.now() / 1000 - 86400 * 4, nome: "Plano de energia OTIMIZA" },
      ],
    },
  ];
</script>

{#if erro}
  <p class="apoio pequeno">Não consegui montar a linha do tempo: {erro}</p>
{:else if linha}
  <details class="linha-do-tempo">
    <summary>
      O que mudou neste jogo
      <span class="apoio">
        {linha.julgamento.estado === "Caiu" ? "· caiu" : linha.julgamento.estado === "NaoCaiu" ? "· não caiu" : "· poucas partidas"}
      </span>
    </summary>
    <div class="corpo">
      <p class:atencao={linha.julgamento.estado === "Caiu"}>{veredito(linha)}</p>
      <ol class="marcos">
        {#each linha.marcos.slice(0, NA_TELA) as m, i (i)}
          <li>
            <span class="quando">{data(m.quando)}</span>
            <span>
              {#if m.tipo === "Partida"}
                Partida: {n(m.fps)} FPS, 1% pior {n(m.low_1pct)}<span class="apoio">{m.fonte === "PresentMon" ? "" : " · canal antigo"}{m.comparavel ? "" : " · com gerador de quadros, fora da comparação"}</span>
              {:else if m.tipo === "Troca"}
                <strong>{NOME_DO_CAMPO[m.troca.campo]}:</strong> {m.troca.de} → {m.troca.para}
              {:else}
                <strong>Otimiza aplicou:</strong> {m.nome}
              {/if}
            </span>
          </li>
        {/each}
      </ol>
      {#if linha.marcos.length > NA_TELA}
        <p class="apoio pequeno">E mais {linha.marcos.length - NA_TELA === 1 ? "1 registro mais antigo" : `${linha.marcos.length - NA_TELA} registros mais antigos`}.</p>
      {/if}
      {#if linha.otimiza_antes_da_primeira > 0}
        <p class="apoio pequeno">
          {linha.otimiza_antes_da_primeira === 1 ? "1 mudança do Otimiza já estava aplicada" : `${linha.otimiza_antes_da_primeira} mudanças do Otimiza já estavam aplicadas`} desde a primeira partida.
        </p>
      {/if}
      <p class="apoio pequeno">Das mudanças do Otimiza, só aparecem as que continuam aplicadas.</p>
    </div>
  </details>
{/if}

<style>
  .linha-do-tempo {
    padding-top: 12px;
    border-top: 1px solid var(--linha);
    font-size: 13px;
  }
  summary {
    font-weight: 600;
  }
  .corpo {
    display: grid;
    gap: 8px;
    padding-top: 10px;
  }
  .atencao {
    color: var(--atencao);
  }
  .marcos {
    list-style: none;
    margin: 0;
    padding: 0 0 0 12px;
    display: grid;
    gap: 6px;
    border-left: 1px solid var(--linha-forte);
  }
  .marcos li {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 10px;
  }
  .quando {
    white-space: nowrap;
    color: var(--tinta-2);
    font-variant-numeric: tabular-nums;
  }
  p {
    margin: 0;
  }
  .pequeno {
    font-size: 12px;
  }
</style>
