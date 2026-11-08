<script lang="ts">
  import { onMount } from "svelte";
  import Ceu from "./componentes/Ceu.svelte";
  import Historico from "./componentes/Historico.svelte";
  import Latencia from "./componentes/Latencia.svelte";
  import Rede from "./componentes/Rede.svelte";
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

  /** "Intel(R) Core(TM) i3-10100F CPU @ 3.60GHz" vira "Intel Core i3-10100F": a ficha cabe numa linha. */
  function nomeCurto(nome: string) {
    return nome
      .replace(/\((R|TM)\)/gi, "")
      .replace(/\s+CPU\s*@.*$/i, "")
      .replace(/^NVIDIA\s+/i, "")
      .replace(/\s+/g, " ")
      .trim();
  }

  /** Sem capa: as iniciais do nome ("Counter-Strike 2" → "CS"), não uma letra solta. */
  function iniciais(nome: string) {
    const palavras = nome.split(/[\s:-]+/).filter((p) => /^[A-Za-zÀ-ú]/.test(p));
    return (palavras.length > 1 ? palavras[0][0] + palavras[1][0] : nome.slice(0, 2)).toUpperCase();
  }

  function inteiro(v: number | null) {
    return v == null ? "—" : String(Math.round(v));
  }

  /** O retrato de hardware é de quando foi lido: depois de uma troca na área Tela, o widget pega o Hz novo dela. */
  function trocouHz(dispositivo: string, hz: number) {
    if (!hardware) return;
    hardware = {
      ...hardware,
      telas: hardware.telas.map((t) => (t.dispositivo.toLowerCase() === dispositivo.toLowerCase() ? { ...t, hz_atual: hz } : t)),
    };
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
      <Mascote estado={jogo ? "focado" : "atento"} tamanho={26} />
      <strong>Otimiza</strong>
      {#if versao}<span class="versao">{versao.exibida}</span>{/if}
    </div>

    <span class="onde" data-tauri-drag-region>{aberta === "inicio" ? "" : area.nome}</span>

    <div class="situacao" data-tauri-drag-region>
      {#if !noTauri}
        <span class="demo">Dados de demonstração</span>
      {/if}
      {#if jogo}
        <span class="jogo"><span class="ponto" data-tom="ok"></span>{jogo.nome} aberto</span>
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
      <button class="capa" title={j.nome} aria-label={j.nome} onclick={() => (aberta = "jogos")}>
        {#if capa}
          <img src={capa} alt="" />
        {:else}
          <span>{iniciais(j.nome)}</span>
        {/if}
      </button>
    {/each}
  </nav>

  <main class="mesa">
    {#if aberta === "inicio"}
      <section class="inicio">
        <div class="saudacao">
          <Mascote {estado} tamanho={176} />
          <div class="texto">
            <h1>{fraseDoMascote(clima, jogo?.nome ?? null, lista)}</h1>
            <p class="apoio-grande">
              {#if jogo}
                Com o jogo aberto eu só confiro, a cada 5 segundos, se ele continua lá.
              {:else if lista.length === 0}
                Olhei a tela e a memória. Rede, temperatura e disco entram nas próximas ondas.
              {:else}
                Cada item abaixo veio de um número lido agora neste PC.
              {/if}
            </p>
          </div>
        </div>

        {#if lista.length > 0}
          <ul class="lista avisos">
            {#each lista as a (a.id)}
              <li>
                <span class="ponto" data-tom={a.gravidade === "serio" ? "perigo" : "atencao"}></span>
                <div class="aviso-texto">
                  <strong>{a.titulo}</strong>
                  <span class="apoio">{a.detalhe}</span>
                </div>
                {#if a.id.startsWith("hz-")}
                  <button class="btn" onclick={() => (aberta = "tela")}>Corrigir</button>
                {/if}
              </li>
            {/each}
          </ul>
        {/if}

        <div class="acoes">
          <button class="btn primario" onclick={() => (aberta = "turbo")}>Ligar o Turbo</button>
          <button class="btn" onclick={() => (aberta = "historico")}>Ver o que foi mudado</button>
        </div>
      </section>
    {:else}
      <section class="janela" aria-labelledby="titulo-da-area">
        <header class="janela-topo">
          <span class="icone" aria-hidden="true">
            <svg viewBox="0 0 24 24" width="20" height="20"><path d={area.traco} /></svg>
          </span>
          <div>
            <h1 id="titulo-da-area">{area.nome}</h1>
            {#if area.pergunta}<p class="apoio">{area.pergunta}</p>{/if}
          </div>
          <button class="fechar-janela" aria-label="Fechar e voltar ao início" title="Voltar ao início" onclick={() => (aberta = "inicio")}>
            <svg viewBox="0 0 12 12" width="12" height="12"><path d="M3 3l6 6M9 3l-6 6" /></svg>
          </button>
        </header>
        <div class="janela-corpo">
          {#if aberta === "historico"}
            <Historico />
          {:else if aberta === "rede"}
            <Rede />
          {:else if aberta === "latencia"}
            <Latencia />
          {:else if aberta === "tela"}
            <Tela aoMudarHz={trocouHz} />
          {:else}
            <div class="em-breve">
              <Mascote estado="descansando" tamanho={96} />
              <p>{area.nome} chega na Onda {area.onda}. Por enquanto ela continua na interface atual do Otimiza.</p>
              {#if area.vai_ter.length > 0}
                <ul class="lista">
                  {#each area.vai_ter as item (item)}
                    <li>{item}</li>
                  {/each}
                </ul>
              {/if}
            </div>
          {/if}
        </div>
      </section>
    {/if}
  </main>

  <aside class="lateral" aria-label="Agora">
    <div class="temas" role="radiogroup" aria-label="Céu">
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

    <section class="painel">
      <h2>Agora</h2>
      {#if jogo}
        <p class="apoio">Pausado enquanto {jogo.nome} está aberto, para não pesar no jogo.</p>
      {:else}
        <div class="medidor">
          <div class="medidor-linha"><span>Processador</span><span class="numero">{pct(metricas?.cpu.overall)}</span></div>
          <div class="barra"><div style:width={`${Math.min(100, metricas?.cpu.overall ?? 0)}%`}></div></div>
        </div>
        <div class="medidor">
          <div class="medidor-linha">
            <span>Memória</span>
            <span class="numero">{memoriaPct == null ? "—" : pct(memoriaPct * 100)}</span>
          </div>
          <div class="barra"><div style:width={`${Math.min(100, (memoriaPct ?? 0) * 100)}%`}></div></div>
        </div>
      {/if}
    </section>

    <section class="painel">
      <h2>Último jogo</h2>
      {#if ultimaPartida?.ultima}
        <p class="jogo-nome">{ultimaPartida.nome}</p>
        <div class="dois">
          <div><span class="numero grande">{inteiro(ultimaPartida.ultima.fps_nativo)}</span><span class="apoio">FPS</span></div>
          <div><span class="numero grande">{inteiro(ultimaPartida.ultima.low_1pct)}</span><span class="apoio">1% pior</span></div>
        </div>
        {#if ultimaPartida.ganho_low.estado === "Medido" && ultimaPartida.ganho_low.pct != null}
          <p class="ganho">
            {ultimaPartida.ganho_low.pct >= 0 ? "+" : ""}{decimal(ultimaPartida.ganho_low.pct)}% no 1% pior
            <span class="selo-medido">Medido</span>
          </p>
        {:else if ultimaPartida.ganho_low.estado === "SemDiferenca"}
          <p class="apoio">Sem diferença medida no 1% pior.</p>
        {/if}
      {:else}
        <p class="apoio">Jogue uma partida com o Otimiza aberto e o resultado aparece aqui.</p>
      {/if}
    </section>

    {#if hardware}
      <section class="painel">
        <h2>Este PC</h2>
        <dl class="ficha">
          <dt>Processador</dt>
          <dd>{nomeCurto(hardware.processador)}</dd>
          <dt>Placa</dt>
          <dd>{nomeCurto(hardware.placa.nome)}</dd>
          <dt>Memória</dt>
          <dd>{decimal(hardware.memoria_gb)} GB</dd>
          {#if telaPrincipal}
            <dt>Tela</dt>
            <dd>
              {telaPrincipal.hz_atual} Hz{#if telaPrincipal.hz_maximo > telaPrincipal.hz_atual}
                <span class="apoio">&nbsp;· aguenta {telaPrincipal.hz_maximo}</span>{/if}
            </dd>
          {/if}
        </dl>
      </section>
    {/if}
  </aside>

  <nav class="dock" aria-label="Áreas">
    {#each areas as a (a.id)}
      <button class="area" class:ativa={aberta === a.id} aria-label={a.nome} title={a.nome} onclick={() => (aberta = a.id)}>
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
    grid-template-columns: 64px minmax(0, 1fr) 288px;
    grid-template-rows: 40px minmax(0, 1fr) 80px;
    grid-template-areas:
      "menu menu menu"
      "trilho mesa lateral"
      "trilho dock lateral";
  }

  /* Barra de menu */
  .menu {
    grid-area: menu;
    display: flex;
    align-items: center;
    gap: 16px;
    padding-left: 12px;
    background: var(--superficie-menu);
    backdrop-filter: var(--desfoque);
    border-bottom: 1px solid var(--linha);
    font-size: 13px;
  }
  .marca {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .marca strong {
    font-family: var(--fonte-titulo);
    font-weight: 600;
  }
  .versao,
  .onde {
    color: var(--tinta-2);
  }
  .situacao {
    flex: 1;
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 16px;
    height: 100%;
    color: var(--tinta-2);
  }
  .demo {
    color: var(--ambar);
  }
  .jogo {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--tinta);
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
  .controles svg,
  .fechar-janela svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 1.3;
  }
  .controles button:hover {
    background: var(--linha);
  }
  .controles .fechar:hover {
    background: #c42b1c;
    color: #fff;
  }

  /* Trilho de jogos */
  .trilho {
    grid-area: trilho;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 16px 0;
  }
  .capa {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    padding: 0;
    border: 0;
    border-radius: var(--raio-2);
    background: var(--superficie);
    box-shadow: var(--sombra-2);
    overflow: hidden;
    cursor: pointer;
    font-family: var(--fonte-titulo);
    font-size: var(--t-apoio);
    font-weight: 600;
    color: var(--tinta-2);
  }
  .capa img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .capa:hover {
    outline: 2px solid var(--superficie);
  }

  /* Mesa */
  .mesa {
    grid-area: mesa;
    display: grid;
    place-items: center;
    padding: 24px 32px 8px;
    min-height: 0;
  }

  .inicio {
    width: min(640px, 100%);
    display: grid;
    gap: 24px;
  }
  .saudacao {
    display: flex;
    align-items: center;
    gap: 24px;
  }
  .saudacao .texto {
    flex: 1;
    display: grid;
    gap: 8px;
  }
  .saudacao h1 {
    font-family: var(--fonte-titulo);
    font-size: 26px;
    font-weight: 600;
    line-height: 1.2;
    letter-spacing: -0.015em;
    text-wrap: balance;
  }
  .apoio-grande {
    color: var(--tinta-2);
    text-wrap: pretty;
  }
  .avisos {
    padding: 4px 16px;
    border-radius: var(--raio-3);
    background: var(--superficie);
    box-shadow: var(--sombra-2);
  }
  .avisos .ponto {
    align-self: flex-start;
    margin-top: 7px;
  }
  .aviso-texto {
    flex: 1;
    display: grid;
    gap: 2px;
  }
  .aviso-texto strong {
    font-weight: 500;
  }
  .acoes {
    display: flex;
    gap: 8px;
  }

  /* Janela de uma área */
  .janela {
    width: min(720px, 100%);
    max-height: 100%;
    display: flex;
    flex-direction: column;
    border-radius: var(--raio-3);
    background: var(--superficie);
    backdrop-filter: var(--desfoque);
    box-shadow: var(--sombra-3);
    overflow: hidden;
  }
  .janela-topo {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 16px 16px 16px 24px;
    border-bottom: 1px solid var(--linha);
  }
  .janela-topo > div {
    flex: 1;
  }
  .janela-topo h1 {
    font-family: var(--fonte-titulo);
    font-size: var(--t-titulo);
    font-weight: 600;
    line-height: 1.2;
  }
  .icone {
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    border-radius: var(--raio-2);
    background: var(--acao-suave);
  }
  .icone svg,
  .area svg {
    fill: none;
    stroke-width: 1.6;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .icone svg {
    stroke: var(--acao);
  }
  .fechar-janela {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    border: 0;
    border-radius: var(--raio-1);
    background: transparent;
    color: var(--tinta-2);
    cursor: pointer;
  }
  .fechar-janela:hover {
    background: var(--linha);
    color: var(--tinta);
  }
  .janela-corpo {
    min-height: 0;
    overflow: auto;
    padding: 8px 24px 24px;
  }
  .em-breve {
    display: grid;
    justify-items: start;
    gap: 16px;
    padding: 16px 0 4px;
  }
  .em-breve p {
    max-width: 52ch;
  }
  .em-breve .lista {
    width: 100%;
    color: var(--tinta-2);
  }

  /* Lateral */
  .lateral {
    grid-area: lateral;
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px 16px 16px 0;
    overflow: auto;
  }
  .temas {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
    padding: 0 4px 4px;
  }
  .bolinha {
    width: 18px;
    height: 18px;
    padding: 0;
    border-radius: 50%;
    border: 2px solid rgb(255 255 255 / 0.9);
    cursor: pointer;
    box-shadow: var(--sombra-1);
  }
  .bolinha[aria-checked="true"] {
    outline: 2px solid var(--acao);
    outline-offset: 2px;
  }
  .bolinha[data-cor="auto"] {
    background: linear-gradient(180deg, #6d9ae4, #dce8fa);
  }
  .bolinha[data-cor="por-do-sol"] {
    background: linear-gradient(180deg, #9a9ad6, #f8d5b4);
  }
  .bolinha[data-cor="lavanda"] {
    background: #c4cdf4;
  }
  .bolinha[data-cor="noite"] {
    background: #1a2a5e;
  }
  .painel {
    display: grid;
    gap: 12px;
    padding: 16px;
    border-radius: var(--raio-3);
    background: var(--superficie);
    backdrop-filter: var(--desfoque);
    box-shadow: var(--sombra-2);
  }
  .painel h2 {
    font-size: var(--t-apoio);
    font-weight: 500;
    color: var(--tinta-2);
  }
  .medidor {
    display: grid;
    gap: 6px;
  }
  .medidor-linha {
    display: flex;
    justify-content: space-between;
  }
  .barra {
    height: 4px;
    border-radius: 2px;
    background: var(--linha);
    overflow: hidden;
  }
  .barra div {
    height: 100%;
    border-radius: 2px;
    background: var(--acao);
    transition: width 600ms ease;
  }
  .jogo-nome {
    font-weight: 500;
    margin-bottom: -4px;
  }
  .dois {
    display: flex;
    gap: 24px;
  }
  .dois div {
    display: flex;
    align-items: baseline;
    gap: 6px;
  }
  .grande {
    font-size: var(--t-numero);
    font-weight: 600;
    line-height: 1;
  }
  .ganho {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--ok);
    font-weight: 500;
  }
  .ficha {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 6px 12px;
    margin: 0;
  }
  .ficha dt {
    color: var(--tinta-2);
  }
  .ficha dd {
    margin: 0;
    text-align: right;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Dock */
  .dock {
    grid-area: dock;
    justify-self: center;
    align-self: center;
    display: flex;
    gap: 4px;
    padding: 6px;
    border-radius: var(--raio-3);
    background: var(--superficie-menu);
    backdrop-filter: var(--desfoque);
    box-shadow: var(--sombra-2);
    border: 1px solid rgb(255 255 255 / 0.5);
  }
  :global(:root[data-tema="noite"]) .dock {
    border-color: var(--linha);
  }
  .area {
    position: relative;
    display: grid;
    place-items: center;
    width: 44px;
    height: 44px;
    border: 0;
    border-radius: var(--raio-2);
    background: transparent;
    cursor: pointer;
  }
  .area svg {
    stroke: var(--tinta-2);
  }
  .area:hover {
    background: var(--superficie);
  }
  .area:hover svg {
    stroke: var(--tinta);
  }
  .area.ativa {
    background: var(--superficie);
    box-shadow: var(--sombra-1);
  }
  .area.ativa svg {
    stroke: var(--acao);
  }
  .area.ativa::after {
    content: "";
    position: absolute;
    bottom: 3px;
    width: 4px;
    height: 4px;
    border-radius: 50%;
    background: var(--acao);
  }

  @media (max-width: 1080px) {
    .casca {
      grid-template-columns: 56px minmax(0, 1fr);
      grid-template-rows: 40px minmax(0, 1fr) auto 80px;
      grid-template-areas:
        "menu menu"
        "trilho mesa"
        "trilho lateral"
        "dock dock";
    }
    .lateral {
      padding: 0 16px 8px 0;
      flex-direction: row;
      flex-wrap: wrap;
    }
    .lateral .painel {
      flex: 1 1 200px;
    }
    .temas {
      display: none;
    }
  }
</style>
