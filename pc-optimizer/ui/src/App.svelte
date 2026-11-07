<script lang="ts">
  import { onMount } from "svelte";
  import Ceu from "./componentes/Ceu.svelte";
  import Historico from "./componentes/Historico.svelte";
  import Tela from "./componentes/Tela.svelte";
  import Mascote from "./componentes/Mascote.svelte";
  import { areas, type IdDaArea } from "./lib/areas";
  import { avisos as lerAvisos, climaDe, fraseDoMascote, horaDe, type EstadoDoMascote, type Tema } from "./lib/clima";
  import {
    noTauri,
    ponte,
    type Hardware,
    type JogoAberto,
    type JogoDaLoja,
    type MetricasAoVivo,
    type ResumoDoJogo,
    type VersaoDoApp,
  } from "./lib/ponte";

  const CHAVE_DO_TEMA = "otimiza.tema";

  let versao = $state<VersaoDoApp | null>(null);
  let hardware = $state<Hardware | null>(null);
  let metricas = $state<MetricasAoVivo | null>(null);
  let jogo = $state<JogoAberto | null>(null);
  let biblioteca = $state<ResumoDoJogo[]>([]);
  let trilho = $state<{ jogo: JogoDaLoja; capa: string | null }[]>([]);
  let agora = $state(new Date());
  let tema = $state<Tema>(lerTema());
  let aberta = $state<IdDaArea>("inicio");

  const lista = $derived(lerAvisos(hardware, metricas, jogo !== null));
  const clima = $derived(climaDe(lista));
  const hora = $derived(horaDe(agora));
  const noturno = $derived(tema === "noite" || (tema === "auto" && hora === "noite"));
  const estado = $derived<EstadoDoMascote>(
    jogo ? "focado" : lista.some((a) => a.gravidade === "serio") ? "preocupado" : "atento"
  );
  const ultimaPartida = $derived(
    biblioteca
      .filter((j) => j.ultima)
      .sort((a, b) => (b.ultima?.quando ?? 0) - (a.ultima?.quando ?? 0))[0] ?? null
  );
  const area = $derived(areas.find((a) => a.id === aberta) ?? areas[0]);
  const telaPrincipal = $derived(hardware?.telas.find((t) => t.principal) ?? hardware?.telas[0] ?? null);
  const memoriaPct = $derived.by(() => {
    const total = metricas?.ram.total_gb ?? null;
    const usada = metricas?.ram.used_gb ?? null;
    return total != null && usada != null && total > 0 ? usada / total : null;
  });

  function lerTema(): Tema {
    try {
      const salvo = localStorage.getItem(CHAVE_DO_TEMA);
      if (salvo === "auto" || salvo === "por-do-sol" || salvo === "lavanda" || salvo === "noite") return salvo;
    } catch {
      // Sem armazenamento (janela privada, prévia): fica o padrão.
    }
    return "auto";
  }

  function escolherTema(t: Tema) {
    tema = t;
    try {
      localStorage.setItem(CHAVE_DO_TEMA, t);
    } catch {
      // Só não lembra na próxima abertura.
    }
  }

  $effect(() => {
    document.documentElement.classList.toggle("jogo-aberto", jogo !== null);
    document.documentElement.dataset.tema = noturno ? "noite" : "dia";
  });

  function pct(v: number | null | undefined) {
    return v == null ? "—" : `${Math.round(v)}%`;
  }

  /** `null` é "não medido": aparece como travessão, nunca como zero. */
  function decimal(v: number | null) {
    return v == null ? "—" : v.toFixed(1).replace(".", ",");
  }

  function inteiro(v: number | null) {
    return v == null ? "—" : String(Math.round(v));
  }

  async function janela(acao: "minimizar" | "maximizar" | "fechar") {
    if (!noTauri) return;
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const w = getCurrentWindow();
    if (acao === "minimizar") await w.minimize();
    else if (acao === "maximizar") await w.toggleMaximize();
    else await w.close();
  }

  onMount(() => {
    let vivo = true;
    let espera: ReturnType<typeof setTimeout> | undefined;

    void (async () => {
      versao = await ponte.versao().catch(() => null);
      hardware = await ponte.hardwareUltimo().catch(() => null);
      ponte.hardware().then((h) => (hardware = h)).catch(() => {});
      ponte.biblioteca().then((b) => (biblioteca = b)).catch(() => {});
      ponte
        .instalados()
        .then(async (lista) => {
          const recentes = [...lista].sort((a, b) => b.ultima_vez - a.ultima_vez).slice(0, 6);
          trilho = recentes.map((j) => ({ jogo: j, capa: null }));
          for (const [i, j] of recentes.entries()) {
            if (j.appid == null) continue;
            const capa = await ponte.capa(j.appid).catch(() => null);
            if (vivo && capa) trilho[i] = { jogo: j, capa };
          }
        })
        .catch(() => {});
      await ponte.iniciarMonitor().catch(() => {});
    })();

    // Um laço só. Com o jogo aberto ele só confere se o jogo continua lá, devagar; com a janela escondida, nem isso
    // além do jogo. A interface não pode ser o motivo de um quadro perdido.
    const laco = async () => {
      if (!vivo) return;
      jogo = await ponte.jogoAberto().catch(() => null);
      if (!jogo && !document.hidden) {
        metricas = await ponte.metricas().catch(() => metricas);
        agora = new Date();
      }
      espera = setTimeout(laco, jogo ? 5000 : document.hidden ? 4000 : 2000);
    };
    void laco();

    return () => {
      vivo = false;
      clearTimeout(espera);
    };
  });
</script>

<Ceu {clima} {hora} {tema} />

<div class="casca">
  <header class="menu" data-tauri-drag-region>
    <div class="marca" data-tauri-drag-region>
      <img src="/mascote-logo.png" alt="" width="22" height="22" />
      <strong>Otimiza</strong>
      {#if versao}<span class="versao">{versao.exibida}</span>{/if}
    </div>

    <div class="situacao" data-tauri-drag-region>
      {#if !noTauri}
        <span class="pilula demo">Dados de demonstração</span>
      {/if}
      {#if jogo}
        <span class="pilula jogo">{jogo.nome} aberto</span>
      {/if}
      <span class="hora numero">{agora.toLocaleTimeString("pt-BR", { hour: "2-digit", minute: "2-digit" })}</span>
    </div>

    <div class="controles">
      <button aria-label="Minimizar" onclick={() => janela("minimizar")}>
        <svg viewBox="0 0 12 12" width="12" height="12"><path d="M2 6h8" /></svg>
      </button>
      <button aria-label="Maximizar" onclick={() => janela("maximizar")}>
        <svg viewBox="0 0 12 12" width="12" height="12"><rect x="2" y="2" width="8" height="8" rx="1.5" /></svg>
      </button>
      <button aria-label="Fechar" class="fechar" onclick={() => janela("fechar")}>
        <svg viewBox="0 0 12 12" width="12" height="12"><path d="M3 3l6 6M9 3l-6 6" /></svg>
      </button>
    </div>
  </header>

  <nav class="trilho" aria-label="Seus jogos">
    {#each trilho as { jogo: j, capa } (j.nome)}
      <button class="espaco" title={j.nome} aria-label={j.nome} onclick={() => (aberta = "jogos")}>
        {#if capa}
          <img src={capa} alt="" />
        {:else}
          <span>{j.nome.slice(0, 1)}</span>
        {/if}
      </button>
    {/each}
  </nav>

  <main class="mesa">
    {#if aberta === "inicio"}
      <section class="saudacao">
        <Mascote {estado} tamanho={200} />
        <p class="frase">{fraseDoMascote(clima, jogo?.nome ?? null, lista)}</p>

        {#if lista.length > 0}
          <ul class="avisos">
            {#each lista as a (a.id)}
              <li data-gravidade={a.gravidade}>
                <strong>{a.titulo}</strong>
                <span>{a.detalhe}</span>
              </li>
            {/each}
          </ul>
        {/if}

        <button class="turbo" onclick={() => (aberta = "turbo")}>
          <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true"><path d="M13 2 5 14h6l-1 8 8-12h-6z" /></svg>
          Ligar o Turbo
        </button>
      </section>
    {:else}
      <section class="janela" aria-labelledby="titulo-da-area">
        <header>
          <span class="objeto grande" aria-hidden="true">
            <svg viewBox="0 0 24 24" width="30" height="30"><path d={area.traco} /></svg>
          </span>
          <div>
            <h1 id="titulo-da-area">{area.nome}</h1>
            {#if area.pergunta}<p>{area.pergunta}</p>{/if}
          </div>
          <button class="voltar" onclick={() => (aberta = "inicio")}>Voltar ao início</button>
        </header>
        {#if aberta === "historico"}
          <Historico />
        {:else if aberta === "tela"}
          <Tela />
        {:else}
          <p class="chega">Esta área chega na Onda {area.onda} da Nova Geração. Vai ter:</p>
          <ul>
            {#each area.vai_ter as item (item)}
              <li>{item}</li>
            {/each}
          </ul>
        {/if}
      </section>
    {/if}
  </main>

  <aside class="widgets" aria-label="Agora">
    <div class="temas" role="radiogroup" aria-label="Tema">
      {#each [["auto", "Céu da hora"], ["por-do-sol", "Pôr do sol"], ["lavanda", "Lavanda"], ["noite", "Noite"]] as [id, nome] (id)}
        <button
          role="radio"
          aria-checked={tema === id}
          aria-label={nome}
          title={nome}
          class="bolinha"
          data-cor={id}
          onclick={() => escolherTema(id as Tema)}
        ></button>
      {/each}
    </div>

    <div class="dupla">
      <article class="widget">
        <h2>Processador</h2>
        <p class="valor numero">{jogo ? "—" : pct(metricas?.cpu.overall)}</p>
        <div class="bolha-medidor" style:--nivel={`${Math.min(100, metricas?.cpu.overall ?? 0)}%`}></div>
      </article>
      <article class="widget">
        <h2>Memória</h2>
        <p class="valor numero">{jogo || memoriaPct == null ? "—" : pct(memoriaPct * 100)}</p>
        <div class="bolha-medidor" style:--nivel={`${Math.min(100, (memoriaPct ?? 0) * 100)}%`}></div>
      </article>
    </div>

    <article class="widget">
      <h2>Último jogo</h2>
      {#if ultimaPartida?.ultima}
        <p class="jogo-nome">{ultimaPartida.nome}</p>
        <div class="tres">
          <div><span class="numero grande">{inteiro(ultimaPartida.ultima.fps_nativo)}</span><small>FPS</small></div>
          <div><span class="numero grande">{inteiro(ultimaPartida.ultima.low_1pct)}</span><small>1% pior</small></div>
        </div>
        {#if ultimaPartida.ganho_low.estado === "Medido" && ultimaPartida.ganho_low.pct != null}
          <p class="ganho">
            {ultimaPartida.ganho_low.pct >= 0 ? "+" : ""}{decimal(ultimaPartida.ganho_low.pct)}% no 1% pior
            <span class="selo">Medido</span>
          </p>
        {:else if ultimaPartida.ganho_low.estado === "SemDiferenca"}
          <p class="ganho neutro">Sem diferença medida <span class="selo">Medido</span></p>
        {/if}
      {:else}
        <p class="vazio">Jogue uma partida com o Otimiza aberto e o resultado aparece aqui.</p>
      {/if}
    </article>

    {#if telaPrincipal}
      <article class="widget">
        <h2>Tela</h2>
        <p class="valor numero">{telaPrincipal.hz_atual} Hz</p>
        <p class="detalhe">
          {telaPrincipal.hz_maximo > telaPrincipal.hz_atual
            ? `aguenta ${telaPrincipal.hz_maximo} Hz`
            : "no máximo do monitor"}
        </p>
      </article>
    {/if}

    {#if hardware}
      <article class="widget maquina">
        <h2>Este PC</h2>
        <p>{hardware.processador}</p>
        <p>{hardware.placa.nome}</p>
        <p>{decimal(hardware.memoria_gb)} GB de memória</p>
      </article>
    {/if}
  </aside>

  <nav class="dock" aria-label="Áreas">
    {#each areas as a (a.id)}
      <button class="objeto" class:ativo={aberta === a.id} aria-label={a.nome} title={a.nome} onclick={() => (aberta = a.id)}>
        <svg viewBox="0 0 24 24" width="22" height="22" aria-hidden="true"><path d={a.traco} /></svg>
      </button>
    {/each}
  </nav>
</div>

<style>
  .casca {
    position: relative;
    z-index: 1;
    height: 100vh;
    display: grid;
    grid-template-columns: 72px 1fr 300px;
    grid-template-rows: 44px 1fr 92px;
    grid-template-areas:
      "menu menu menu"
      "trilho mesa widgets"
      "trilho dock widgets";
  }

  /* Barra de menu */
  .menu {
    grid-area: menu;
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 0 0 0 16px;
    background: var(--vidro);
    backdrop-filter: var(--desfoque);
    border-bottom: 1px solid var(--vidro-borda);
  }
  .marca {
    display: flex;
    align-items: center;
    gap: 8px;
    font-family: var(--fonte-titulo);
  }
  .marca img {
    border-radius: 50%;
  }
  .versao {
    color: var(--tinta-suave);
    font-size: 12px;
  }
  .situacao {
    flex: 1;
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 10px;
    height: 100%;
  }
  .pilula {
    padding: 3px 10px;
    border-radius: var(--raio-botao);
    font-size: 12px;
    background: var(--vidro-denso);
  }
  .pilula.demo {
    color: var(--ambar-700);
  }
  :global(:root[data-tema="noite"]) .pilula.demo {
    color: #ffd2a0;
  }
  .pilula.jogo {
    background: var(--ceu-500);
    color: white;
  }
  .hora {
    color: var(--tinta-suave);
    font-size: 13px;
  }
  .controles {
    display: flex;
    height: 100%;
  }
  .controles button {
    width: 46px;
    border: 0;
    background: transparent;
    cursor: pointer;
  }
  .controles svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 1.3;
  }
  .controles button:hover {
    background: rgb(255 255 255 / 0.4);
  }
  .controles .fechar:hover {
    background: var(--perigo);
    color: white;
  }

  /* Trilho de jogos */
  .trilho {
    grid-area: trilho;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 18px 0;
  }
  .espaco {
    width: 44px;
    height: 44px;
    padding: 0;
    border-radius: 50%;
    border: 2px solid var(--vidro-borda);
    background: var(--vidro);
    backdrop-filter: var(--desfoque);
    overflow: hidden;
    cursor: pointer;
    box-shadow: var(--vidro-sombra);
    font-family: var(--fonte-titulo);
    font-weight: 700;
  }
  .espaco img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  /* Mesa */
  .mesa {
    grid-area: mesa;
    display: grid;
    place-items: center;
    padding: 24px;
    min-height: 0;
    overflow: auto;
  }
  .saudacao {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 18px;
    max-width: 520px;
    text-align: center;
  }
  .frase {
    margin: 0;
    font-family: var(--fonte-titulo);
    font-size: 26px;
    font-weight: 600;
    line-height: 1.2;
    letter-spacing: -0.01em;
    text-wrap: balance;
  }
  .avisos {
    list-style: none;
    margin: 0;
    padding: 0;
    width: 100%;
    display: grid;
    gap: 8px;
    text-align: left;
  }
  .avisos li {
    display: grid;
    gap: 2px;
    padding: 12px 16px 12px 18px;
    border-radius: 16px;
    background: var(--vidro-denso);
    box-shadow: var(--vidro-sombra);
    border-left: 4px solid var(--atencao);
  }
  .avisos li[data-gravidade="serio"] {
    border-left-color: var(--perigo);
  }
  .avisos span {
    color: var(--tinta-suave);
    font-size: 13px;
  }
  .turbo {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    padding: 14px 30px;
    border: 0;
    border-radius: var(--raio-botao);
    font-family: var(--fonte-titulo);
    font-size: 17px;
    font-weight: 700;
    color: white;
    cursor: pointer;
    background: linear-gradient(180deg, #6f97ec, var(--ceu-700));
    box-shadow:
      0 1px 0 rgb(255 255 255 / 0.45) inset,
      0 0 0 3px rgb(255 230 204 / 0.55),
      0 14px 30px rgb(47 95 168 / 0.4);
  }
  .turbo svg {
    fill: var(--pessego-200);
  }
  .turbo:active {
    transform: translateY(1px);
  }

  /* Janela de uma área */
  .janela {
    width: min(620px, 100%);
    padding: 26px 30px 30px;
    border-radius: var(--raio-bolha);
    background: var(--vidro-denso);
    border: 1px solid var(--vidro-borda);
    box-shadow: var(--vidro-sombra);
  }
  .janela header {
    display: flex;
    align-items: center;
    gap: 16px;
  }
  .janela h1 {
    margin: 0;
    font-family: var(--fonte-titulo);
    font-size: 26px;
  }
  .janela header p {
    margin: 2px 0 0;
    color: var(--tinta-suave);
  }
  .voltar {
    margin-left: auto;
    padding: 8px 14px;
    border-radius: var(--raio-botao);
    border: 1px solid var(--vidro-borda);
    background: var(--vidro);
    cursor: pointer;
  }
  .chega {
    margin: 22px 0 8px;
    color: var(--tinta-suave);
  }
  .janela ul {
    margin: 0;
    padding-left: 20px;
    display: grid;
    gap: 6px;
  }

  /* Widgets */
  .widgets {
    grid-area: widgets;
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 18px 18px 18px 0;
    overflow: auto;
  }
  .temas {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
    padding: 6px 4px;
  }
  .bolinha {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    border: 2px solid white;
    cursor: pointer;
    padding: 0;
    box-shadow: 0 2px 6px rgb(30 60 120 / 0.25);
  }
  .bolinha[aria-checked="true"] {
    outline: 2px solid var(--ceu-700);
    outline-offset: 2px;
  }
  .bolinha[data-cor="auto"] {
    background: linear-gradient(135deg, #ffe6cc, #79a2e6);
  }
  .bolinha[data-cor="por-do-sol"] {
    background: linear-gradient(135deg, #b59bd8, #f3b7a0);
  }
  .bolinha[data-cor="lavanda"] {
    background: #b4c3f3;
  }
  .bolinha[data-cor="noite"] {
    background: #141d45;
  }
  .dupla {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }
  .widget {
    position: relative;
    padding: 14px 16px;
    border-radius: var(--raio-cartao);
    background: var(--vidro);
    backdrop-filter: var(--desfoque);
    border: 1px solid var(--vidro-borda);
    box-shadow: var(--vidro-sombra);
    overflow: hidden;
  }
  .widget h2 {
    margin: 0 0 4px;
    font-size: 12px;
    font-weight: 500;
    color: var(--tinta-suave);
  }
  .valor {
    margin: 0;
    font-size: 28px;
    font-weight: 700;
  }
  .detalhe,
  .vazio,
  .maquina p {
    margin: 0;
    color: var(--tinta-suave);
    font-size: 13px;
  }
  .maquina p:first-of-type {
    color: var(--tinta);
  }
  .bolha-medidor {
    position: absolute;
    inset: auto 0 0 0;
    height: var(--nivel);
    background: linear-gradient(180deg, rgb(91 134 229 / 0.18), rgb(91 134 229 / 0.32));
    transition: height 600ms ease;
    pointer-events: none;
  }
  .jogo-nome {
    margin: 0 0 6px;
    font-family: var(--fonte-titulo);
    font-weight: 600;
  }
  .tres {
    display: flex;
    gap: 22px;
  }
  .tres div {
    display: grid;
  }
  .tres .grande {
    font-size: 30px;
    font-weight: 700;
    line-height: 1;
  }
  .tres small {
    color: var(--tinta-suave);
  }
  .ganho {
    margin: 10px 0 0;
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 600;
    color: var(--sucesso);
  }
  .ganho.neutro {
    color: var(--tinta-suave);
  }
  .selo {
    padding: 1px 8px;
    border-radius: var(--raio-botao);
    font-size: 11px;
    font-weight: 600;
    color: var(--ambar-700);
    background: rgb(255 230 204 / 0.85);
  }

  /* Dock */
  .dock {
    grid-area: dock;
    justify-self: center;
    align-self: center;
    display: flex;
    gap: 6px;
    padding: 8px 10px;
    border-radius: 24px;
    background: var(--vidro);
    backdrop-filter: var(--desfoque);
    border: 1px solid var(--vidro-borda);
    box-shadow: var(--vidro-sombra);
  }
  .objeto {
    display: grid;
    place-items: center;
    width: 46px;
    height: 46px;
    border: 0;
    border-radius: 14px;
    cursor: pointer;
    background: linear-gradient(180deg, rgb(255 255 255 / 0.75), rgb(201 220 248 / 0.6));
    box-shadow:
      0 1px 0 white inset,
      0 4px 10px rgb(30 60 120 / 0.15);
    transition: transform 160ms ease;
  }
  .objeto svg {
    fill: none;
    stroke: var(--ceu-700);
    stroke-width: 1.7;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .objeto:hover {
    transform: translateY(-3px);
  }
  .objeto.ativo {
    background: linear-gradient(180deg, #8fb0f0, var(--ceu-500));
  }
  .objeto.ativo svg {
    stroke: white;
  }
  .objeto.grande {
    width: 58px;
    height: 58px;
    border-radius: 18px;
  }
  :global(:root[data-tema="noite"]) .objeto:not(.ativo) {
    background: linear-gradient(180deg, rgb(80 96 160 / 0.7), rgb(40 52 100 / 0.7));
  }
  :global(:root[data-tema="noite"]) .objeto:not(.ativo) svg {
    stroke: #c9d6fb;
  }

  :global(.jogo-aberto) .objeto,
  :global(.jogo-aberto) .bolha-medidor {
    transition: none;
  }

  @media (max-width: 980px) {
    .casca {
      grid-template-columns: 64px 1fr;
      grid-template-rows: 44px 1fr auto 92px;
      grid-template-areas:
        "menu menu"
        "trilho mesa"
        "trilho widgets"
        "dock dock";
    }
    .widgets {
      padding: 0 18px 12px 0;
      flex-direction: row;
      flex-wrap: wrap;
    }
  }
</style>
