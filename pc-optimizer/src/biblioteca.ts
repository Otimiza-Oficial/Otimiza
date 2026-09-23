import { invoke } from "@tauri-apps/api/core";

/*
 * SEUS JOGOS (2.9).
 *
 * A biblioteca desta máquina — Steam, Epic, programas instalados (Riot,
 * Blizzard, EA, Ubisoft, GOG, Rockstar, Roblox, FiveM) e todo jogo que o
 * Otimiza já viu rodando. Para cada um, o que o produto consegue fazer:
 *
 *   A — ajusta a configuração gráfica do jogo (FiveM/GTA V e qualquer jogo em
 *       Unreal Engine), com prévia, cópia do arquivo e desfazer;
 *   C — mede o jogo e ajusta o sistema em volta dele (Mapa de desempenho,
 *       energia, placa de vídeo, gerador de quadros).
 *
 * A única escolha é o orçamento de imagem (quanto visual a pessoa aceita
 * trocar por FPS). Não existe perfil de RP, de PvP ou de competitivo: todo
 * mundo ganha FPS.
 */

type Origem = "Steam" | "Epic" | "Windows" | "Instalado" | "Detectado";
type Orcamento = "MaxFps" | "Equilibrado" | "Qualidade";

type Medicao = {
  jogo: string;
  quando: number;
  fps: number;
  low_1pct: number;
  engasgos_por_minuto: number;
  confiavel: boolean;
  /** O que a placa fez NA MESMA janela medida (2.9). */
  placa?: ResumoGpu | null;
};

type ResumoGpu = {
  amostras: number;
  temperatura_max_c: number | null;
  temperatura_media_c: number | null;
  clock_medio_mhz: number | null;
  potencia_media_w: number | null;
  limite_w: number | null;
  uso_medio_pct: number | null;
  pct_termico: number;
  pct_teto_de_energia: number;
  pct_freio_de_hardware: number;
  motivos_lidos: boolean;
};

/*
 * A PLACA DURANTE A PARTIDA.
 *
 * O número que interessa não é a temperatura: é quanto do tempo o DRIVER disse
 * que estava segurando o clock, e por quê. Temperatura alta com o clock cheio
 * é uma placa trabalhando; 70 °C segurando o clock metade da partida é um
 * problema que nenhum ajuste de Windows resolve.
 *
 * Os limiares moram no Rust (`core::sensores`), e são os mesmos do veredito —
 * a tela não inventa piso próprio.
 */
function linhaDaPlaca(m: Medicao | null): string {
  const p = m?.placa;
  if (!p || !p.amostras) return "";

  const numeros = [
    p.temperatura_max_c !== null ? `máx. ${Math.round(p.temperatura_max_c)} °C` : null,
    p.clock_medio_mhz !== null ? `${Math.round(p.clock_medio_mhz)} MHz` : null,
    p.potencia_media_w !== null
      ? `${Math.round(p.potencia_media_w)} W${p.limite_w !== null ? ` de ${Math.round(p.limite_w)} W` : ""}`
      : null,
  ]
    .filter(Boolean)
    .join(" · ");

  if (!p.motivos_lidos) {
    return `<p class="fg-nota"><strong>Placa na partida:</strong> ${esc(numeros)}. O driver não informou se estava segurando o clock nesta máquina.</p>`;
  }

  const pct = (v: number) => `${v.toFixed(0)}%`;

  if (p.pct_termico >= 5) {
    return `<p class="fg-aviso"><strong>A placa foi limitada por temperatura em ${pct(p.pct_termico)} desta partida</strong> (${esc(numeros)}). Isso é físico: limpe a poeira, confira as ventoinhas e o fluxo de ar do gabinete. Nenhum ajuste de Windows resolve isso.</p>`;
  }
  if (p.pct_freio_de_hardware >= 5) {
    return `<p class="fg-aviso"><strong>A placa acionou o freio de hardware em ${pct(p.pct_freio_de_hardware)} desta partida</strong> (${esc(numeros)}). Costuma ser a fonte, o conector de energia da placa ou a proteção térmica dela.</p>`;
  }
  if (p.pct_teto_de_energia >= 60) {
    return `<p class="fg-nota"><strong>Placa na partida:</strong> ${esc(numeros)}. Ela passou ${pct(p.pct_teto_de_energia)} do tempo no teto de energia — é o funcionamento normal de uma placa em carga máxima, não defeito.</p>`;
  }
  return `<p class="fg-nota"><strong>Placa na partida:</strong> ${esc(numeros)}. Nada segurou o clock além do normal.</p>`;
}

type Jogo = {
  nome: string;
  origem: Origem;
  pasta: string;
  executavel: string | null;
  nivel: "A" | "C";
  ajustador: "fivem" | "unreal" | null;
  ultima_medicao: Medicao | null;
  ajuste_aplicado: string | null;
  em_observacao: boolean;
  decidido: Decidido | null;
  deriva: Deriva | null;
  perfil_nvidia: string | null;
};

type Deriva = {
  mudou: { tipo: "Driver"; de: string; para: string } | { tipo: "Windows"; de: string; para: string } | { tipo: "Nada" };
  fps_antes: number;
  fps_depois: number;
  queda_pct: number;
  partidas_antes: number;
  partidas_depois: number;
};

function linhaDaDeriva(j: Jogo): string {
  const d = j.deriva;
  if (!d) return "";
  const numeros = `${Math.round(d.fps_antes)} → ${Math.round(d.fps_depois)} FPS (−${Math.round(d.queda_pct)}%, ${d.partidas_antes} partidas antes e ${d.partidas_depois} depois)`;
  const causa =
    d.mudou.tipo === "Driver"
      ? `depois que o driver de vídeo mudou de ${esc(d.mudou.de)} para ${esc(d.mudou.para)}. Vale testar o driver anterior ou esperar a próxima versão.`
      : d.mudou.tipo === "Windows"
        ? `depois que o Windows atualizou (${esc(d.mudou.de)} → ${esc(d.mudou.para)}). Confira no Mapa de desempenho o que está limitando agora.`
        : "sem mudança de driver nem de Windows. Poeira e temperatura, um programa novo rodando junto ou uma atualização do jogo são os suspeitos — meça no Mapa de desempenho.";
  return `<p class="fg-aviso"><strong>O desempenho caiu:</strong> ${numeros}, ${causa}</p>`;
}

type Decidido = {
  veredito: "Desfazer" | "Melhorou" | "SemMudanca" | { Aguardando: { antes: number; depois: number } };
  quando: number;
  fps_antes: number | null;
  fps_depois: number | null;
  low_antes: number | null;
  low_depois: number | null;
  erro: string | null;
};

function linhaDoPortao(j: Jogo): string {
  if (j.em_observacao) {
    return `<p class="fg-nota"><strong>Em observação:</strong> o Otimiza está medindo as próximas partidas deste jogo. Se o FPS ou o 1% piores caírem de verdade (além da variação normal e em pelo menos 5%), o ajuste é desfeito sozinho. Precisa da medição automática ligada (aba Sistema, Preferências) e do Otimiza aberto como administrador.</p>`;
  }
  const d = j.decidido;
  if (!d) return "";
  const n = (v: number | null) => (v === null ? "—" : Math.round(v).toString());
  const numeros = `FPS ${n(d.fps_antes)} → ${n(d.fps_depois)}${d.low_antes !== null ? ` · 1% piores ${n(d.low_antes)} → ${n(d.low_depois)}` : ""}`;
  if (d.veredito === "Desfazer") {
    return d.erro
      ? `<p class="fg-erro">O ajuste piorou este jogo (${numeros}) e o Otimiza tentou desfazer, mas falhou: ${esc(d.erro)}. Use o botão Desfazer.</p>`
      : `<p class="fg-aviso"><strong>Desfeito sozinho:</strong> o ajuste piorou este jogo nesta máquina (${numeros}). A configuração voltou ao que era.</p>`;
  }
  if (d.veredito === "Melhorou") return `<p class="fg-nota"><strong>Medido:</strong> melhorou (${numeros}).</p>`;
  if (d.veredito === "SemMudanca") return `<p class="fg-nota"><strong>Medido:</strong> sem diferença além da variação normal entre partidas (${numeros}).</p>`;
  return "";
}

type Biblioteca = { nvidia: boolean; jogos: Jogo[]; lacunas: string[]; medicoes_erro: string | null };
type Mudanca = { chave: string; antes: number; depois: number };
type Previa = { arquivo: string; mudancas: Mudanca[] };

const ORIGEM: Record<Origem, string> = {
  Steam: "Steam",
  Epic: "Epic Games",
  Windows: "visto pelo Windows",
  Instalado: "programas instalados",
  Detectado: "visto rodando pelo Otimiza",
};

const ORCAMENTO: Record<Orcamento, string> = {
  MaxFps: "Máximo de FPS",
  Equilibrado: "Equilibrado",
  Qualidade: "Qualidade",
};

const GRUPO: Record<string, string> = {
  "sg.shadowquality": "Sombras",
  "sg.postprocessquality": "Pós-processamento",
  "sg.effectsquality": "Efeitos",
  "sg.foliagequality": "Folhagem",
  "sg.globalilluminationquality": "Iluminação global",
  "sg.reflectionquality": "Reflexos",
  "sg.shadingquality": "Sombreamento",
  "sg.texturequality": "Texturas",
};
const NIVEL = ["Baixo", "Médio", "Alto", "Épico", "Cinematográfico"];

type ItemDeProntidao =
  | { tipo: "MonitorAbaixo"; hz_atual: number; hz_maximo: number; monitor: string }
  | { tipo: "LimitesEscondidos"; quantos: number }
  | { tipo: "SegundoPlanoPesado"; programas: [string, number][] }
  | { tipo: "MemoriaApertada"; livre_mb: number; commit_pct: number | null; maiores: [string, number][] }
  | { tipo: "ModoJogoDesligado" };

type Prontidao = { pronto: boolean; itens: ItemDeProntidao[]; conferido: string[] };

function fraseDaProntidao(i: ItemDeProntidao): string {
  switch (i.tipo) {
    case "MonitorAbaixo":
      return `<strong>${esc(i.monitor)} está a ${i.hz_atual} Hz e aguenta ${i.hz_maximo} Hz.</strong> É a maior diferença de fluidez que existe — corrija na aba Diagnóstico, em Monitores.`;
    case "LimitesEscondidos":
      return `<strong>${i.quantos} limite(s) de FPS escondido(s).</strong> Veja quais e onde tirar no Mapa de desempenho (Painel).`;
    case "SegundoPlanoPesado":
      return `<strong>Programas usando processador agora:</strong> ${i.programas.map(([n, c]) => `${esc(n)} (${Math.round(c * 100)}% da máquina)`).join(", ")}. Feche antes de jogar, ou ligue o modo jogo.`;
    case "MemoriaApertada":
      return `<strong>Memória já apertada antes do jogo:</strong> ${(i.livre_mb / 1024).toFixed(1)} GB livres${i.commit_pct !== null ? `, ${Math.round(i.commit_pct)}% comprometida` : ""}.${i.maiores.length ? ` Quem mais usa: ${i.maiores.map(([n, mb]) => `${esc(n)} (${(mb / 1024).toFixed(1)} GB)`).join(", ")}.` : ""} Fechar isso antes de abrir o jogo evita travada de paginação.`;
    case "ModoJogoDesligado":
      return `<strong>Modo jogo desligado.</strong> Ligado (aba Sistema, Preferências), ele passa programas que disputam processador para o modo econômico durante a partida.`;
  }
}

async function verificarProntidao(alvo: HTMLElement) {
  alvo.innerHTML = `<p class="fg-nota">Verificando…</p>`;
  try {
    const p = await invoke<Prontidao>("pronto_para_jogar");
    alvo.innerHTML = `
      <p class="fg-aviso"><strong>${p.pronto ? "Pronto para jogar." : "Resolva isto antes de jogar:"}</strong></p>
      ${p.itens.map((i) => `<p class="fg-nota">${fraseDaProntidao(i)}</p>`).join("")}
      ${p.conferido.length ? `<p class="fg-nota">Conferido: ${p.conferido.map(esc).join(" · ")}.</p>` : ""}`;
  } catch (e) {
    alvo.innerHTML = `<p class="fg-erro">${esc(String(e))}</p>`;
  }
}

/*
 * PERFIL NVIDIA DO JOGO (2.9)
 *
 * Vai no perfil do executável, nunca no global: "desempenho máximo" no global
 * deixa a placa acordada até na área de trabalho. V-Sync não entra em nenhum
 * perfil (G-SYNC/VRR), e nenhum perfil põe limite de FPS.
 */
type PerfilDoJogo = "Competitivo" | "BaixaLatencia";
type AjusteDoJogo = { opcao: string; titulo: string; explicacao: string; atual: string; novo: string; igual: boolean };

const PERFIL_NVIDIA: Record<PerfilDoJogo, { nome: string; resumo: string }> = {
  Competitivo: {
    nome: "Competitivo",
    resumo: "A placa não baixa o clock no meio da partida e o processador prepara só um quadro adiantado. A imagem não muda.",
  },
  BaixaLatencia: {
    nome: "Baixa latência",
    resumo: "Tudo do Competitivo, mais o filtro de textura em \"desempenho\" — pode deixar texturas distantes um pouco menos nítidas.",
  },
};

function blocoNvidia(j: Jogo, i: number): string {
  if (!j.executavel) return "";
  if (j.perfil_nvidia) {
    return `<div class="fg-linha"><span class="fg-chip">perfil NVIDIA do Otimiza aplicado neste jogo</span><button class="btn btn-ghost" data-nv-desfazer="${i}">Voltar ao padrão (desfaz o perfil)</button></div>`;
  }
  return `
    <div class="fg-linha">
      <span class="fg-nota"><strong>Perfil NVIDIA deste jogo:</strong></span>
      <div class="fg-segmentos" role="group" aria-label="Perfil NVIDIA">
        ${(Object.keys(PERFIL_NVIDIA) as PerfilDoJogo[])
          .map((p) => `<button class="fg-segmento" data-nv-jogo="${i}" data-nv-perfil="${p}">${PERFIL_NVIDIA[p].nome}</button>`)
          .join("")}
      </div>
    </div>
    <div class="bib-previa" id="bib-nv-${i}"></div>`;
}

async function preverNvidia(j: Jogo, i: number, p: PerfilDoJogo) {
  const alvo = raiz?.querySelector<HTMLElement>(`#bib-nv-${i}`);
  if (!alvo || !j.executavel) return;
  raiz?.querySelectorAll<HTMLElement>(`[data-nv-jogo="${i}"]`).forEach((b) => b.setAttribute("aria-pressed", String(b.dataset.nvPerfil === p)));
  alvo.innerHTML = `<p class="fg-nota">Lendo o driver da NVIDIA…</p>`;
  try {
    const ajustes = await invoke<AjusteDoJogo[]>("nvidia_perfil_prever", { executavel: j.executavel, perfil: p });
    const nada = ajustes.every((a) => a.igual);
    alvo.innerHTML = `
      <p class="fg-nota">${esc(PERFIL_NVIDIA[p].resumo)}</p>
      <table class="fg-tabela">
        <thead><tr><th>Ajuste</th><th>Hoje</th><th>Fica</th></tr></thead>
        <tbody>${ajustes
          .map((a) => `<tr title="${esc(a.explicacao)}"><td>${esc(a.titulo)}</td><td>${esc(a.atual)}</td><td>${a.igual ? "fica como está" : esc(a.novo)}</td></tr>`)
          .join("")}</tbody>
      </table>
      <p class="fg-nota">Só este jogo muda. V-Sync e limite de FPS não são tocados. Ganho não validado nesta máquina: meça antes e depois no Mapa de desempenho.</p>
      ${nada ? `<p class="fg-aviso">Nada a mudar: este jogo já está assim.</p>` : `<div class="fg-linha"><button class="btn btn-primary" data-nv-aplicar="${i}">Aplicar neste jogo</button></div>`}
      <p class="fg-nota" id="bib-nv-resultado-${i}"></p>`;
    alvo.querySelector<HTMLButtonElement>(`[data-nv-aplicar="${i}"]`)?.addEventListener("click", () => void aplicarNvidia(j, i, p));
  } catch (e) {
    alvo.innerHTML = `<p class="fg-erro">${esc(String(e))}</p>`;
  }
}

async function aplicarNvidia(j: Jogo, i: number, p: PerfilDoJogo) {
  const saida = raiz?.querySelector<HTMLElement>(`#bib-nv-resultado-${i}`);
  if (!saida || !j.executavel) return;
  saida.textContent = "Aplicando…";
  try {
    const r = await invoke<{ message: string }>("nvidia_perfil_aplicar", { executavel: j.executavel, perfil: p });
    void seusJogos(true);
    saida.textContent = r.message;
  } catch (e) {
    const msg = String(e);
    if (msg.toLowerCase().includes("administrador")) pedirAdmin(msg);
    saida.textContent = msg;
  }
}

let raiz: HTMLElement | null = null;
let pedirAdmin: (motivo: string) => void = () => {};

function esc(s: string): string {
  return s.replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]!);
}

function quando(ts: number): string {
  return new Date(ts * 1000).toLocaleString("pt-BR", { dateStyle: "short", timeStyle: "short" });
}

/*
 * OTIMIZAR E TESTAR (2.9)
 *
 * Um clique no lugar de "escolha o orçamento, depois escolha o perfil": aplica
 * o ajuste gráfico Equilibrado e o perfil NVIDIA Competitivo — os dois com o
 * estado anterior guardado e desfazer.
 *
 * E não promete nada depois disso. Os dois entram na vigília "nunca menos
 * FPS": quem decide são as próximas partidas medidas, e o que piorar é
 * desfeito sozinho.
 *
 * Não inventa etapa nova: chama os mesmos comandos dos botões ao lado.
 */
async function otimizarETestar(j: Jogo, i: number, nvidia: boolean) {
  const saida = raiz?.querySelector<HTMLElement>(`#bib-otimizar-${i}`);
  if (!saida || !j.executavel) return;
  const feitos: string[] = [];
  const falhas: string[] = [];

  saida.innerHTML = `<p class="fg-nota">Aplicando o ajuste gráfico…</p>`;
  try {
    const m = await invoke<Mudanca[]>("unreal_aplicar", {
      executavel: j.executavel,
      nome: j.nome,
      orcamento: "Equilibrado" as Orcamento,
    });
    feitos.push(
      m.length
        ? `Configuração do jogo: ${m.length} opção(ões) baixada(s).`
        : "Configuração do jogo: já estava nesse nível ou abaixo — nada mudou.",
    );
  } catch (e) {
    const msg = String(e);
    if (msg.toLowerCase().includes("administrador")) pedirAdmin(msg);
    falhas.push(`Configuração do jogo: ${msg}`);
  }

  if (nvidia) {
    saida.innerHTML = `<p class="fg-nota">Aplicando o perfil NVIDIA…</p>`;
    try {
      const r = await invoke<{ message: string }>("nvidia_perfil_aplicar", {
        executavel: j.executavel,
        perfil: "Competitivo" as PerfilDoJogo,
      });
      feitos.push(r.message);
    } catch (e) {
      falhas.push(`Perfil NVIDIA: ${String(e)}`);
    }
  }

  void seusJogos(true);
  saida.innerHTML = `
    ${feitos.map((t) => `<p class="fg-nota">${esc(t)}</p>`).join("")}
    ${falhas.map((t) => `<p class="fg-erro">${esc(t)}</p>`).join("")}
    ${
      feitos.length
        ? `<p class="fg-aviso"><strong>Agora jogue.</strong> O Otimiza mede as próximas
             partidas sozinho e compara com as de antes. Se o FPS ou o 1% piores caírem de
             verdade, ele desfaz o que aplicou e mostra os números. Precisa da medição
             automática ligada (aba Sistema, Preferências) e do Otimiza como administrador.</p>`
        : ""
    }`;
}

function linhaDoJogo(j: Jogo, i: number, nvidia: boolean): string {
  const med = j.ultima_medicao
    ? `<span class="fg-chip">última partida medida ${quando(j.ultima_medicao.quando)}: ${Math.round(j.ultima_medicao.fps)} FPS${
        j.ultima_medicao.confiavel ? ` · 1% piores ${Math.round(j.ultima_medicao.low_1pct)}` : ""
      }</span>`
    : `<span class="fg-chip">ainda sem medição</span>`;

  let acao = "";
  if (j.ajustador === "unreal" && j.executavel) {
    acao = `
      <div class="fg-linha">
        <button class="btn btn-primary" data-otimizar="${i}">Otimizar e testar</button>
        <span class="fg-nota">Aplica o Equilibrado${nvidia ? " e o perfil NVIDIA Competitivo" : ""}, e deixa as próximas partidas decidirem.</span>
      </div>
      <div class="bib-previa" id="bib-otimizar-${i}"></div>
      <p class="fg-nota">Ou escolha o orçamento de imagem você mesmo:</p>
      <div class="fg-linha">
        <div class="fg-segmentos" role="group" aria-label="Orçamento de imagem">
          ${(Object.keys(ORCAMENTO) as Orcamento[])
            .map((o) => `<button class="fg-segmento" data-jogo="${i}" data-orcamento="${o}">${ORCAMENTO[o]}</button>`)
            .join("")}
        </div>
      </div>
      <div class="bib-previa" id="bib-previa-${i}"></div>
      ${j.ajuste_aplicado ? `<div class="fg-linha"><span class="fg-chip">configuração ajustada pelo Otimiza</span><button class="btn btn-ghost" data-desfazer="${i}">Desfazer (volta o arquivo como era)</button></div>` : ""}`;
  } else if (j.ajustador === "fivem") {
    acao = `<p class="fg-nota">A configuração deste jogo é ajustada no painel <strong>Configuração do jogo</strong>, mais abaixo nesta aba.</p>`;
  } else {
    acao = `<p class="fg-nota">
      O Otimiza ainda não escreve a configuração deste jogo. Ele mede as partidas (Mapa de desempenho, no Painel) e
      ajusta o que está em volta: energia, placa de vídeo, programas em segundo plano e o gerador de quadros.
    </p>`;
  }

  return `
    <article class="bib-jogo">
      <header>
        <div>
          <b>${esc(j.nome)}</b>
          <span class="fg-nota">${esc(ORIGEM[j.origem] ?? j.origem)}</span>
        </div>
        <span class="fg-tipo" title="${j.nivel === "A" ? "O Otimiza ajusta a configuração gráfica deste jogo" : "O Otimiza mede e ajusta o sistema em volta deste jogo"}">
          ${j.nivel === "A" ? "AJUSTA O JOGO" : "AJUSTA O SISTEMA"}
        </span>
      </header>
      <div class="fg-linha">${med}</div>
      ${acao}
      ${linhaDaPlaca(j.ultima_medicao)}
      ${linhaDoPortao(j)}
      ${linhaDaDeriva(j)}
      ${nvidia ? blocoNvidia(j, i) : ""}
    </article>`;
}

async function prever(j: Jogo, i: number, orc: Orcamento) {
  const alvo = raiz?.querySelector<HTMLElement>(`#bib-previa-${i}`);
  if (!alvo || !j.executavel) return;
  raiz?.querySelectorAll<HTMLElement>(`[data-jogo="${i}"]`).forEach((b) => b.setAttribute("aria-pressed", String(b.dataset.orcamento === orc)));
  alvo.innerHTML = `<p class="fg-nota">Lendo a configuração…</p>`;
  try {
    const p = await invoke<Previa>("unreal_prever", { executavel: j.executavel, orcamento: orc });
    if (!p.mudancas.length) {
      alvo.innerHTML = `<p class="fg-aviso">Nada a mudar com "${ORCAMENTO[orc]}": o jogo já está nesse nível ou abaixo. O Otimiza nunca sobe qualidade.</p>`;
      return;
    }
    alvo.innerHTML = `
      <table class="fg-tabela">
        <thead><tr><th>Opção</th><th>Hoje</th><th>Fica</th></tr></thead>
        <tbody>${p.mudancas
          .map((m) => `<tr><td>${esc(GRUPO[m.chave.toLowerCase()] ?? m.chave)}</td><td>${NIVEL[m.antes] ?? m.antes}</td><td>${NIVEL[m.depois] ?? m.depois}</td></tr>`)
          .join("")}</tbody>
      </table>
      <p class="fg-nota">Arquivo: <span class="fg-mono">${esc(p.arquivo)}</span>. Resolução, distância de visão e limite de FPS não são tocados. O arquivo inteiro fica guardado para desfazer.</p>
      <div class="fg-linha"><button class="btn btn-primary" data-aplicar="${i}" data-orcamento="${orc}">Aplicar (com o jogo fechado)</button></div>
      <p class="fg-nota" id="bib-resultado-${i}"></p>`;
    alvo.querySelector<HTMLButtonElement>(`[data-aplicar="${i}"]`)?.addEventListener("click", () => void aplicar(j, i, orc));
  } catch (e) {
    alvo.innerHTML = `<p class="fg-erro">${esc(String(e))}</p>`;
  }
}

async function aplicar(j: Jogo, i: number, orc: Orcamento) {
  const saida = raiz?.querySelector<HTMLElement>(`#bib-resultado-${i}`);
  if (!saida || !j.executavel) return;
  saida.textContent = "Aplicando…";
  try {
    const m = await invoke<Mudanca[]>("unreal_aplicar", { executavel: j.executavel, nome: j.nome, orcamento: orc });
    if (m.length) void seusJogos(true);
    saida.textContent = m.length
      ? `Pronto: ${m.length} opção(ões) ajustada(s). Jogue algumas partidas: o Otimiza compara antes e depois e desfaz sozinho se o jogo piorar. Para voltar agora, use Desfazer nesta ficha.`
      : "Nada precisou mudar.";
  } catch (e) {
    const msg = String(e);
    if (msg.toLowerCase().includes("administrador")) pedirAdmin(msg);
    saida.textContent = msg;
  }
}

/*
 * INTEGRAÇÃO COM A GRADE DA 2.8
 *
 * A aba Biblioteca (grade com capas, `main.ts`) é a tela dos jogos. Este
 * arquivo alimenta a FICHA de cada jogo com o que a 2.9 acrescentou: o ajuste
 * gráfico por orçamento de imagem (Unreal), a vigília "nunca menos FPS", a
 * deriva e a última partida medida — e o "Verificar antes de jogar", no topo
 * da aba.
 */

let cache: Promise<Biblioteca> | null = null;

function seusJogos(recarregar = false): Promise<Biblioteca> {
  if (!cache || recarregar) cache = invoke<Biblioteca>("seus_jogos");
  return cache;
}

function mesmoCaminho(a: string | null | undefined, b: string | null | undefined): boolean {
  return !!a && !!b && a.replace(/\//g, "\\").toLowerCase() === b.replace(/\//g, "\\").toLowerCase();
}

/** O jogo da ficha, na lista da 2.9 (pelo executável; senão pela pasta). */
function acharNaLista(lista: Jogo[], executavel: string | null, pasta: string | null): number {
  let i = lista.findIndex((j) => mesmoCaminho(j.executavel, executavel));
  if (i < 0) i = lista.findIndex((j) => mesmoCaminho(j.pasta, pasta));
  return i;
}

/** Preenche a parte 2.9 da ficha de um jogo. */
export async function preencherFichaDoJogo(
  alvo: HTMLElement,
  jogo: { executavel: string | null; pasta: string | null },
  opcoes: { pedirAdmin: (motivo: string) => void },
) {
  pedirAdmin = opcoes.pedirAdmin;
  raiz = alvo;
  alvo.innerHTML = `<p class="fg-nota">Lendo o que o Otimiza sabe deste jogo…</p>`;
  try {
    const lista = await seusJogos();
    const i = acharNaLista(lista.jogos, jogo.executavel, jogo.pasta);
    if (i < 0) {
      alvo.innerHTML = "";
      return;
    }
    alvo.innerHTML = linhaDoJogo(lista.jogos[i], i, lista.nvidia);
    alvo.querySelectorAll<HTMLButtonElement>("[data-nv-perfil]").forEach((btn) =>
      btn.addEventListener("click", () => void preverNvidia(lista.jogos[i], i, btn.dataset.nvPerfil as PerfilDoJogo)),
    );
    alvo.querySelector<HTMLButtonElement>("[data-nv-desfazer]")?.addEventListener("click", async (e) => {
      const btn = e.currentTarget as HTMLButtonElement;
      const id = lista.jogos[i].perfil_nvidia;
      if (!id) return;
      btn.disabled = true;
      try {
        await invoke("revert_optimization", { id });
        await seusJogos(true);
        void preencherFichaDoJogo(alvo, jogo, opcoes);
      } catch (erro) {
        btn.disabled = false;
        btn.textContent = String(erro);
      }
    });
    alvo.querySelector<HTMLButtonElement>("[data-desfazer]")?.addEventListener("click", async (e) => {
      const btn = e.currentTarget as HTMLButtonElement;
      const id = lista.jogos[i].ajuste_aplicado;
      if (!id) return;
      btn.disabled = true;
      try {
        await invoke("revert_optimization", { id });
        await seusJogos(true);
        void preencherFichaDoJogo(alvo, jogo, opcoes);
      } catch (erro) {
        btn.disabled = false;
        btn.textContent = String(erro);
      }
    });
    alvo.querySelector<HTMLButtonElement>("[data-otimizar]")?.addEventListener("click", (e) => {
      (e.currentTarget as HTMLButtonElement).disabled = true;
      void otimizarETestar(lista.jogos[i], i, lista.nvidia);
    });
    alvo.querySelectorAll<HTMLButtonElement>("[data-orcamento][data-jogo]").forEach((btn) =>
      btn.addEventListener("click", () => void prever(lista.jogos[i], i, btn.dataset.orcamento as Orcamento)),
    );
  } catch (e) {
    alvo.innerHTML = `<p class="fg-erro">${esc(String(e))}</p>`;
  }
}

/** O "Verificar antes de jogar", no topo da aba Biblioteca. */
export function ligarProntidao() {
  const botao = document.getElementById("pronto-jogar");
  const saida = document.getElementById("pronto-jogar-resultado");
  if (!botao || !saida || botao.dataset.ligado) return;
  botao.dataset.ligado = "1";
  botao.addEventListener("click", () => void verificarProntidao(saida));
}
