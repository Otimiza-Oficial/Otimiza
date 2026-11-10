<script lang="ts">
  // Diagnóstico (trazido da interface antiga, `src/main.ts` e `src/travadas.ts`): "pronto para jogar?" antes da
  // partida, "por que meu FPS está baixo?" com os suspeitos medidos (o Otimiza é o PRIMEIRO suspeito quando houve
  // regressão), a investigação de travadas com o jogo aberto e as quedas de FPS depois de atualizar driver ou Windows.
  // Suspeito ausente não quer dizer máquina limpa: as lacunas aparecem escritas.
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { noTauri } from "../lib/ponte";

  type Suspeito = { id: string; titulo: string; medido: string; porque: string; como_confirmar: string };
  type Investigacao = { suspeitos: Suspeito[]; lacunas: string[] };
  // `programas`: fração do processador inteiro (0 a 1), como o Rust mede (`prontojogo`).
  type Item =
    | { tipo: "MonitorAbaixo"; hz_atual: number; hz_maximo: number; monitor: string }
    | { tipo: "LimitesEscondidos"; quantos: number }
    | { tipo: "SegundoPlanoPesado"; programas: [string, number][] }
    | { tipo: "MemoriaApertada"; livre_mb: number; commit_pct: number | null; maiores: [string, number][] }
    | { tipo: "ModoJogoDesligado" };
  type Prontidao = { pronto: boolean; itens: Item[]; conferido: string[] };
  type Deriva = { jogo: string; mudou: { tipo: "Driver" | "Windows"; de: string; para: string } | { tipo: "Nada" }; fps_antes: number; fps_depois: number; queda_pct: number };
  type Travadas = { jogo: string; investigacao: Investigacao; teste_possivel: string | null; motivo_sem_teste: string | null };

  let pronto = $state<Prontidao | null>(null);
  let quedas = $state<Deriva[]>([]);
  let causas = $state<Investigacao | null>(null);
  let travadas = $state<Travadas | null>(null);
  let ocupado = $state<string | null>(null);
  let erro = $state<string | null>(null);
  let teste = $state<{ processo: string; mantido: boolean; aviso: string | null } | null>(null);

  async function testarCausa() {
    if (!travadas?.teste_possivel) return;
    ocupado = "teste";
    erro = null;
    try {
      teste = noTauri
        ? await invoke("testar_causa_da_travada", { processo: travadas.teste_possivel })
        : { processo: travadas.teste_possivel, mantido: true, aviso: null };
    } catch (e) {
      erro = String(e);
    } finally {
      ocupado = null;
    }
  }

  async function devolver() {
    try {
      const r = noTauri ? await invoke<string | null>("devolver_teste_da_travada") : null;
      teste = null;
      erro = r;
    } catch (e) {
      erro = String(e);
    }
  }

  onMount(async () => {
    if (!noTauri) {
      pronto = { pronto: false, itens: [{ tipo: "MonitorAbaixo", hz_atual: 60, hz_maximo: 165, monitor: "AOC 24G2" }, { tipo: "SegundoPlanoPesado", programas: [["chrome.exe", 0.182]] }], conferido: ["Hz do monitor", "Limites escondidos", "Programas de fundo", "Memória", "Modo jogo"] };
      quedas = [{ jogo: "FiveM", mudou: { tipo: "Driver", de: "560.94", para: "566.03" }, fps_antes: 92, fps_depois: 78, queda_pct: 15.2 }];
      return;
    }
    const [p, q] = await Promise.allSettled([invoke<Prontidao>("pronto_para_jogar"), invoke<Deriva[]>("quedas_de_desempenho")]);
    pronto = p.status === "fulfilled" ? p.value : null;
    quedas = q.status === "fulfilled" ? q.value : [];
  });

  function frase(i: Item): string {
    switch (i.tipo) {
      case "MonitorAbaixo":
        return `O monitor ${i.monitor} está em ${i.hz_atual} Hz e aguenta ${i.hz_maximo}. Ajuste na área Tela.`;
      case "LimitesEscondidos":
        return `${i.quantos} ${i.quantos === 1 ? "limite escondido segurando" : "limites escondidos segurando"} o FPS. Veja na área Energia.`;
      case "SegundoPlanoPesado":
        return `Pesando por trás: ${i.programas.map(([n, p]) => `${n} (${(p * 100).toFixed(0)}% do processador)`).join(", ")}.`;
      case "MemoriaApertada":
        return `Só ${(i.livre_mb / 1024).toFixed(1).replace(".", ",")} GB de memória livre${i.maiores.length ? `; os maiores: ${i.maiores.map(([n]) => n).join(", ")}` : ""}. Veja "Memória para o jogo" no Turbo.`;
      case "ModoJogoDesligado":
        return "O modo jogo está desligado (Energia › Processador no jogo).";
    }
  }

  function mudanca(d: Deriva): string {
    if (d.mudou.tipo === "Nada") return "sem mudança de driver ou Windows registrada";
    return `${d.mudou.tipo === "Driver" ? "driver" : "Windows"} ${d.mudou.de} → ${d.mudou.para}`;
  }

  async function porQue() {
    ocupado = "porque";
    erro = null;
    try {
      causas = noTauri
        ? await invoke<Investigacao>("por_que_o_fps_esta_baixo")
        : { suspeitos: [{ id: "vram", titulo: "Pouca memória de vídeo para servidor de RP", medido: "A placa tem 4 GB e o jogo usou 3,9 GB na última partida.", porque: "Sem memória de vídeo, a textura vem da RAM e o FPS cai conforme o lugar do mapa.", como_confirmar: "Jogue com texturas em Normal e compare a última partida." }], lacunas: ["Sem leitura da temperatura do processador."] };
    } catch (e) {
      erro = String(e);
    } finally {
      ocupado = null;
    }
  }

  async function investigar() {
    ocupado = "travadas";
    erro = null;
    try {
      travadas = noTauri
        ? await invoke<Travadas>("investigar_travadas", { segundos: 30 })
        : await new Promise((r) => setTimeout(() => r({ jogo: "FiveM", investigacao: { suspeitos: [], lacunas: [] }, teste_possivel: null, motivo_sem_teste: null }), 1200));
    } catch (e) {
      erro = String(e);
    } finally {
      ocupado = null;
    }
  }
</script>

<section class="secao diag">
  <h2>Diagnóstico</h2>

  {#if pronto}
    <div class="bloco">
      <span class="sub"><span class="ponto" data-tom={pronto.pronto ? "ok" : "atencao"}></span> {pronto.pronto ? "Pronto para jogar" : "Antes de jogar"}</span>
      {#each pronto.itens as i, k (k)}<p class="apoio">{frase(i)}</p>{/each}
      <p class="apoio pequeno">Conferido: {pronto.conferido.join(", ")}.</p>
    </div>
  {/if}

  {#if quedas.length}
    <div class="bloco">
      <span class="sub">Quedas de FPS medidas</span>
      {#each quedas as d (d.jogo)}
        <p class="aviso">{d.jogo}: de {d.fps_antes.toFixed(0)} para {d.fps_depois.toFixed(0)} FPS (−{d.queda_pct.toFixed(1).replace(".", ",")}%) · {mudanca(d)}.</p>
      {/each}
    </div>
  {/if}

  <div class="bloco">
    <div class="linha">
      <span class="sub">Por que meu FPS está baixo?</span>
      <button class="btn" disabled={ocupado !== null} onclick={porQue}>{ocupado === "porque" ? "Investigando…" : "Investigar"}</button>
    </div>
    {#if causas}
      {#if causas.suspeitos.length === 0}<p class="apoio">Nenhum suspeito com número medido.</p>{/if}
      {#each causas.suspeitos as s (s.id)}
        <div class="suspeito">
          <b>{s.titulo}</b>
          <p class="apoio pequeno"><b>Medido:</b> {s.medido}</p>
          <p class="apoio pequeno"><b>Por quê:</b> {s.porque}</p>
          <p class="apoio pequeno"><b>Como confirmar:</b> {s.como_confirmar}</p>
        </div>
      {/each}
      {#if causas.lacunas.length}<p class="apoio pequeno">Não deu para conferir: {causas.lacunas.join(" ")}</p>{/if}
    {/if}
  </div>

  <div class="bloco">
    <div class="linha">
      <span class="sub">Investigar travadas (30 s, com o jogo aberto)</span>
      <button class="btn" disabled={ocupado !== null} onclick={investigar}>{ocupado === "travadas" ? "Medindo 30 s…" : "Investigar"}</button>
    </div>
    {#if travadas}
      {#if travadas.investigacao.suspeitos.length === 0}
        <p class="apoio">Nada apareceu nos 30 s do {travadas.jogo}. Se a travada é rara, jogue até ela acontecer e investigue logo depois.</p>
      {/if}
      {#each travadas.investigacao.suspeitos as s (s.id)}
        <div class="suspeito">
          <b>{s.titulo}</b>
          <p class="apoio pequeno">{s.medido} {s.porque}</p>
        </div>
      {/each}
      {#if travadas.motivo_sem_teste}<p class="apoio pequeno">{travadas.motivo_sem_teste}</p>{/if}
      {#if travadas.teste_possivel}
        <div class="linha">
          <p class="apoio pequeno">Dá para provar: o Otimiza acalma <b>{travadas.teste_possivel}</b> em janelas alternadas e compara as travadas. Só fica acalmado se for a causa, e até o jogo fechar.</p>
          <button class="btn" disabled={ocupado !== null} onclick={testarCausa}>{ocupado === "teste" ? "Testando…" : "Testar"}</button>
        </div>
      {/if}
      {#if teste}
        <p class="apoio">{teste.mantido ? `${teste.processo} era a causa: fica acalmado até o jogo fechar.` : `${teste.processo} não mudou as travadas: foi devolvido.`}</p>
        {#if teste.aviso}<p class="aviso">{teste.aviso}</p>{/if}
        {#if teste.mantido}<button class="btn quieto" onclick={devolver}>Devolver agora</button>{/if}
      {/if}
    {/if}
  </div>
  {#if erro}<p class="aviso">{erro}</p>{/if}
</section>

<style>
  .diag,
  .bloco {
    display: grid;
    gap: 6px;
  }
  .bloco + .bloco {
    padding-top: 10px;
    border-top: 1px solid var(--linha);
  }
  .sub {
    font-weight: 500;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  p {
    margin: 0;
  }
  .linha {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
  }
  .suspeito {
    display: grid;
    gap: 2px;
    padding: 10px 12px;
    border-radius: var(--raio-2);
    background: var(--superficie-2);
    font-size: 13px;
  }
  .pequeno {
    font-size: 12px;
  }
  .aviso {
    color: var(--atencao);
    font-size: 13px;
  }
</style>
