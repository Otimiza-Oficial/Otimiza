<script lang="ts">
  // O jogo está esperando o disco? (trazido da interface 3.6, `analyzeStreaming`): cruza os trancos da última partida
  // medida com o instante em que o disco estava ocupado e separa as três causas do mesmo sintoma (troca de memória,
  // jogo num disco lento, outra coisa no disco). Só lê (`laboratorio_de_streaming`). Os números que sustentaram vão
  // junto, mesmo com algum faltando.
  import { invoke } from "@tauri-apps/api/core";
  import { noTauri } from "../lib/ponte";

  type Veredito = "SemMedicao" | "SemEngasgos" | "NaoEODisco" | "TrocaDeMemoria" | "AssetsDeMidiaLenta" | "OutraCoisaNoDisco";
  type Laboratorio = {
    analise: {
      veredito: Veredito;
      engasgos_por_minuto: number | null;
      coincidencia_pct: number | null;
      latencia_ms: number | null;
      memoria_pct: number | null;
      midia: "Ssd" | "Mecanico" | "NaoDeuParaLer" | null;
      falta: string[];
      explicacao: string;
      proximo_passo: string | null;
    };
    jogo: { jogo: string; caminho: string; unidade: string | null } | null;
    lacunas: string[];
  };

  /** Só acende quando há o que fazer. */
  const ROTULO: Record<Veredito, { texto: string; tom: "ok" | "atencao" | null }> = {
    SemMedicao: { texto: "Falta medir", tom: null },
    SemEngasgos: { texto: "Partida lisa", tom: "ok" },
    NaoEODisco: { texto: "Não é o disco", tom: "ok" },
    TrocaDeMemoria: { texto: "É a memória", tom: "atencao" },
    AssetsDeMidiaLenta: { texto: "É o disco", tom: "atencao" },
    OutraCoisaNoDisco: { texto: "Outra coisa no disco", tom: null },
  };

  let r = $state<Laboratorio | null>(null);
  let lendo = $state(false);
  let erro = $state<string | null>(null);

  async function ler() {
    lendo = true;
    erro = null;
    try {
      r = noTauri ? await invoke<Laboratorio>("laboratorio_de_streaming") : DEMO;
    } catch (e) {
      erro = String(e);
    } finally {
      lendo = false;
    }
  }

  const num = (v: number | null, sufixo: string) => (v === null ? "—" : `${v.toLocaleString("pt-BR", { maximumFractionDigits: 1 })}${sufixo}`);

  const DEMO: Laboratorio = {
    analise: {
      veredito: "AssetsDeMidiaLenta",
      engasgos_por_minuto: 4.2,
      coincidencia_pct: 71,
      latencia_ms: 38,
      memoria_pct: 64,
      midia: "Mecanico",
      falta: [],
      explicacao: "7 em cada 10 trancos caíram com o disco do jogo ocupado, e ele é um HD mecânico respondendo em 38 ms. A memória não estava cheia.",
      proximo_passo: "Mova o jogo para um SSD pelo iniciador (na Steam: Propriedades › Arquivos instalados › Mover pasta).",
    },
    jogo: { jogo: "Counter-Strike 2", caminho: "D:\\SteamLibrary\\steamapps\\common\\Counter-Strike Global Offensive", unidade: "D" },
    lacunas: [],
  };
</script>

<section class="secao">
  <div class="topo">
    <h2>O jogo está esperando o disco?</h2>
    <button class="btn" disabled={lendo} onclick={ler}>{lendo ? "Lendo os discos…" : r ? "Ler de novo" : "Conferir"}</button>
  </div>
  {#if !r && !erro}
    <p class="apoio texto">
      Usa a medição de quadros da última partida para ver se os trancos caem no instante em que o disco está ocupado. Vale
      mais numa partida andando pelo mapa: é andando que o jogo pede o que ainda não está na memória. Nada é alterado.
    </p>
  {/if}
  {#if erro}
    <div class="recado" role="alert"><span class="ponto" data-tom="perigo"></span><div><span>{erro}</span></div></div>
  {/if}
  {#if r}
    {@const rot = ROTULO[r.analise.veredito]}
    <p class="titulo">{#if rot.tom}<span class="ponto" data-tom={rot.tom}></span>{/if}{rot.texto}</p>
    <p class="apoio texto">{r.analise.explicacao}</p>
    <dl class="leituras">
      <div><dt>Trancos por minuto</dt><dd class="numero">{num(r.analise.engasgos_por_minuto, "")}</dd></div>
      <div><dt>Com o disco ocupado</dt><dd class="numero">{num(r.analise.coincidencia_pct, "%")}</dd></div>
      <div><dt>Resposta do disco</dt><dd class="numero">{num(r.analise.latencia_ms, " ms")}</dd></div>
      <div><dt>Memória do sistema</dt><dd class="numero">{num(r.analise.memoria_pct, "%")}</dd></div>
    </dl>
    {#if r.analise.proximo_passo}<p class="pequeno">{r.analise.proximo_passo}</p>{/if}
    {#if r.jogo}<p class="apoio pequeno">O jogo que entrou na conta: {r.jogo.jogo}, em {r.jogo.caminho}.</p>{/if}
    {#if r.analise.falta.length}<p class="apoio pequeno">Não entrou na conta, por falta de medida: {r.analise.falta.join("; ")}.</p>{/if}
    {#if r.lacunas.length}<p class="apoio pequeno">A leitura dos discos não conseguiu: {r.lacunas.join("; ")}.</p>{/if}
  {/if}
</section>

<style>
  .topo {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
  }
  .texto {
    max-width: 64ch;
  }
  .titulo {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 600;
  }
  .leituras {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 16px;
    margin: 0;
  }
  .leituras > div {
    display: grid;
    gap: 2px;
  }
  .leituras dt {
    color: var(--tinta-2);
    font-size: 12px;
  }
  .leituras dd {
    margin: 0;
    font-size: 22px;
    font-weight: 600;
    line-height: 1.1;
  }
  p {
    margin: 0;
  }
  .pequeno {
    font-size: 12px;
  }
  @media (max-width: 900px) {
    .leituras {
      grid-template-columns: 1fr 1fr;
    }
  }
</style>
