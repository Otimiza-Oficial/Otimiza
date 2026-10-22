// A interface da 3.3. Monta em #v33, ao lado do console antigo: o console continua inteiro e volta pelo "Modo
// clássico" (Ajustes). Cada número que aparece aqui vem do backend com a origem dele; o que ainda não foi medido
// aparece como "medindo", nunca como um número inventado.

import "./estilo.css";
import marcaSvg from "../assets/otimiza-marca.svg?raw";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  ponte,
  ouvir,
  motivo,
  type CaminhoDasCores,
  type Compatibilidade,
  type EstadoDoFeedback,
  type EstadoDoGerador,
  type Hardware,
  type JogoAberto,
  type CantoDoOverlay,
  type ModoDoOverlay,
  type PainelFrameBoost,
  type NivelDeEscala,
  type Gargalo,
  type PainelTurbo,
  type ReceitaDoTurbo,
  type PassoDaProvaDoTurbo,
  type PlanoJogoLiso,
  type FovFivem,
  type MemoriaParaOJogo,
  type EnergiaDeJogo,
  type JogoDaLoja,
  type ItemDoCatalogo,
  type PassoDoLote,
  type RecursoAvaliado,
  type ResumoDoJogo,
  type ResultadoDoLab,
  type SituacaoDoFeedback,
  type Suporte,
} from "./ponte";
import { capa } from "./capas";
import { CORES, ESTILOS, desenharMira, gravarMira, lerMira, cmPorVolta, sensValorant, treinar, type Mira } from "./mira";
import { NORMAL, PREDEFINICOES, previa, type Ajuste } from "./cores";



type Destino = "inicio" | "jogos" | "boost" | "mira" | "cores" | "lab" | "ajustes";

const CONVITE = "https://discord.gg/ultimus";

const estado = {
  hw: null as Hardware | null,
  compat: null as Compatibilidade | null,
  jogo: null as JogoAberto | null,
  biblioteca: [] as ResumoDoJogo[],
  gerador: null as EstadoDoGerador | null,
  feedback: null as EstadoDoFeedback | null,
  otimizando: false,
  destino: "inicio" as Destino,
};

let enviarMira = 0;
let logoD = "";
let logoViewBox = "0 0 512 512";

// ---------------------------------------------------------------- utilidades

const $ = <T extends HTMLElement = HTMLElement>(sel: string, raiz: ParentNode = document) => raiz.querySelector(sel) as T;
const $$ = <T extends HTMLElement = HTMLElement>(sel: string, raiz: ParentNode = document) => [...raiz.querySelectorAll(sel)] as T[];

function esc(v: unknown): string {
  return String(v ?? "").replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]!);
}

function numero(v: number | null | undefined, casas = 0): string {
  if (v === null || v === undefined || !Number.isFinite(v)) return "—";
  return v.toLocaleString("pt-BR", { minimumFractionDigits: casas, maximumFractionDigits: casas });
}

function haQuanto(segundos: number): string {
  const d = Math.max(0, Date.now() / 1000 - segundos);
  if (d < 90) return "agora há pouco";
  if (d < 3600) return `há ${Math.round(d / 60)} min`;
  if (d < 86400) return `há ${Math.round(d / 3600)} h`;
  return `há ${Math.round(d / 86400)} dias`;
}

function saudacao(): string {
  const h = new Date().getHours();
  return h < 12 ? "Bom dia" : h < 18 ? "Boa tarde" : "Boa noite";
}

function contar(el: HTMLElement, ate: number, ms = 1000) {
  const de = Number(el.dataset.valor ?? ate);
  el.dataset.valor = String(ate);
  if (matchMedia("(prefers-reduced-motion: reduce)").matches || de === ate) {
    el.textContent = numero(ate);
    return;
  }
  const t0 = performance.now();
  const passo = (t: number) => {
    const p = Math.min(1, (t - t0) / ms);
    const e = 1 - Math.pow(1 - p, 3);
    el.textContent = numero(de + (ate - de) * e);
    if (p < 1) requestAnimationFrame(passo);
    else el.textContent = numero(ate); // termina sempre no valor medido
  };
  requestAnimationFrame(passo);
}

function selo(s: Suporte): string {
  switch (s.estado) {
    case "Suportado":
      return `<span class="selo medido">funciona aqui</span>`;
    case "Parcial":
      return `<span class="selo estimado">com condição</span>`;
    case "PrecisaAtualizar":
      return `<span class="selo aviso">atualize o driver</span>`;
    default:
      return `<span class="selo">indisponível</span>`;
  }
}

function idDoJogo(j: JogoAberto): string {
  return j.no_catalogo ? j.perfil.id : j.executavel.toLowerCase();
}

function resumoDoJogoAberto(): ResumoDoJogo | undefined {
  const j = estado.jogo;
  return j ? estado.biblioteca.find((r) => r.id === idDoJogo(j)) : undefined;
}

function frameBoost(): RecursoAvaliado | null {
  return estado.jogo?.frame_boost_auto ?? estado.compat?.frame_boost_auto ?? null;
}

const ANTICHEAT: Record<string, string> = {
  Nenhum: "sem anticheat",
  CfxRe: "anticheat da Cfx.re",
  Vac: "VAC",
  EasyAntiCheat: "Easy Anti-Cheat",
  BattlEye: "BattlEye",
  Vanguard: "Vanguard",
  Ricochet: "Ricochet",
  Hyperion: "Hyperion",
  Desconhecido: "anticheat não identificado",
};
const API: Record<string, string> = { DirectX12: "DirectX 12", DirectX11: "DirectX 11", DirectX9: "DirectX 9", Vulkan: "Vulkan", OpenGl: "OpenGL", Desconhecida: "API não identificada" };

// ---------------------------------------------------------------- ícones

const ICONE: Record<Destino, string> = {
  inicio: `<path class="preenche" d="M4.5 10.3 12 4.2l7.5 6.1v8.4a1.8 1.8 0 0 1-1.8 1.8H6.3a1.8 1.8 0 0 1-1.8-1.8z"/><path d="M4.5 10.3 12 4.2l7.5 6.1v8.4a1.8 1.8 0 0 1-1.8 1.8H6.3a1.8 1.8 0 0 1-1.8-1.8z"/><path d="M9.6 20.5v-4.6a2.4 2.4 0 0 1 4.8 0v4.6"/>`,
  jogos: `<path class="preenche" d="M8.2 6.6h7.6a5.6 5.6 0 0 1 5.5 6.7l-.7 3.4a2.6 2.6 0 0 1-4.4 1.3L14.4 16H9.6l-1.8 2a2.6 2.6 0 0 1-4.4-1.3l-.7-3.4a5.6 5.6 0 0 1 5.5-6.7z"/><path d="M8.2 6.6h7.6a5.6 5.6 0 0 1 5.5 6.7l-.7 3.4a2.6 2.6 0 0 1-4.4 1.3L14.4 16H9.6l-1.8 2a2.6 2.6 0 0 1-4.4-1.3l-.7-3.4a5.6 5.6 0 0 1 5.5-6.7z"/><path d="M8.4 9.7v3.4M6.7 11.4h3.4"/><circle cx="15.7" cy="10.3" r="1" class="ponto"/><circle cx="17.6" cy="12.5" r="1" class="ponto"/>`,
  boost: `<path class="preenche" d="M3.6 16.8a8.6 8.6 0 1 1 16.8 0z"/><path d="M3.6 16.8a8.6 8.6 0 1 1 16.8 0"/><path d="M12 6.2v1.6M6.2 9.1l1.1 1.1M17.8 9.1l-1.1 1.1"/><path d="m12.4 14.3 3.6-4.4"/><circle cx="12" cy="14.8" r="1.5" class="ponto"/><path d="M3.6 16.8h16.8"/>`,
  mira: `<circle class="preenche" cx="12" cy="12" r="7.4"/><circle cx="12" cy="12" r="7.4"/><path d="M12 2.6v4.6M12 16.8v4.6M2.6 12h4.6M16.8 12h4.6"/><circle cx="12" cy="12" r="1.3" class="ponto"/>`,
  cores: `<rect class="preenche" x="3.2" y="4.4" width="17.6" height="12.4" rx="2.2"/><rect x="3.2" y="4.4" width="17.6" height="12.4" rx="2.2"/><path d="M9 20.4h6M12 16.8v3.6"/><circle cx="12" cy="10.6" r="3.2"/><path class="ponto" d="M12 7.4a3.2 3.2 0 0 1 0 6.4z"/>`,
  lab: `<path class="preenche" d="M6.6 14.4h10.8l1.6 2.7a2.2 2.2 0 0 1-1.9 3.4H6.9A2.2 2.2 0 0 1 5 17.1z"/><path d="M9.4 3.6h5.2M10.4 3.6v5.5L5 17.1a2.2 2.2 0 0 0 1.9 3.4h10.2a2.2 2.2 0 0 0 1.9-3.4l-5.4-8V3.6"/><path d="M6.6 14.4h10.8"/>`,
  ajustes: `<path d="M6 3.8v5.6M6 13.6v6.6M12 3.8v2.6M12 10.6v9.6M18 3.8v8.6M18 16.6v3.6"/><circle class="preenche" cx="6" cy="11.5" r="2.1"/><circle cx="6" cy="11.5" r="2.1"/><circle class="preenche" cx="12" cy="8.5" r="2.1"/><circle cx="12" cy="8.5" r="2.1"/><circle class="preenche" cx="18" cy="14.5" r="2.1"/><circle cx="18" cy="14.5" r="2.1"/>`,
};
const NOMES: Record<Destino, string> = { inicio: "Início", jogos: "Jogos", boost: "Boost", mira: "Mira", cores: "Cores", lab: "Lab", ajustes: "Ajustes" };

function botaoDestino(d: Destino) {
  return `<button class="ir" data-ir="${d}" aria-selected="${d === "inicio"}"><svg viewBox="0 0 24 24" aria-hidden="true">${ICONE[d]}</svg><span class="nome">${NOMES[d]}</span></button>`;
}

const svgDoca = {
  otimizar: `<path d="M13 2 4.5 13.5H11L10 22l8.5-11.5H12z"/>`,
  boost: `<rect x="7" y="7" width="13" height="10" rx="2"/><path d="M4 14V6a2 2 0 0 1 2-2h10"/>`,
  lab: `<path d="M4 19h16M6 16l3.5-5 3 3L17 7"/>`,
  desfazer: `<path d="M9 7 5 11l4 4"/><path d="M5 11h9a5 5 0 0 1 0 10h-2"/>`,
};

// ---------------------------------------------------------------- estrutura

function estrutura(): string {
  return `
  <nav class="trilho" aria-label="Navegação">
    <div class="logo" aria-label="Otimiza"><svg viewBox="${logoViewBox}" aria-hidden="true"><path fill="currentColor" fill-rule="evenodd" d="${logoD}"/></svg></div>
    ${(["inicio", "jogos", "boost", "mira", "cores", "lab"] as Destino[]).map(botaoDestino).join("")}
    <div class="fim"><span class="sep-trilho"></span>${botaoDestino("ajustes")}</div>
  </nav>

  <aside class="painel">
    <button class="busca" id="v-busca"><svg viewBox="0 0 24 24"><circle cx="11" cy="11" r="7"/><path d="m20 20-3.5-3.5"/></svg>Pesquisar<kbd>Ctrl K</kbd></button>
    <div class="rotulo">Seus jogos</div>
    <div class="arvore" id="v-arvore"><div class="vazio-painel">Abra um jogo: ele aparece aqui com o FPS medido.</div></div>
    <div class="cartao-pc" id="v-pc"><div class="rotulo">Este PC</div><div class="vazio-painel">Lendo o hardware…</div></div>
  </aside>

  <main class="v-palco">
    <section class="tela" id="t-inicio">
      <div class="cabeca">
        <div><h1><span class="barras" aria-hidden="true"></span><span id="i-titulo">${saudacao()}</span></h1><p id="i-sub">Lendo este PC…</p></div>
        <span class="pilula parado" id="i-pilula"><span class="v-vivo"></span><span id="i-pilula-t">Nenhum jogo aberto</span></span>
      </div>
      <div class="i-grade">
        <div class="i-topo" id="i-topo">
          <div class="i-arte" id="arte-a"></div><div class="i-arte" id="arte-b"></div>
          <div class="i-imagem">
            <button id="f-por" title="Pôr imagens suas no topo, até 8, que se alternam (ficam só neste PC)"><svg viewBox="0 0 24 24" aria-hidden="true"><rect x="3.5" y="5" width="17" height="14" rx="2.5"/><circle cx="9" cy="10" r="1.6"/><path d="m4.5 17 5-4.5 3.5 3 2.5-2 4 3.5"/></svg><span>Pôr imagens</span></button>
            <button id="f-limpar" hidden title="Voltar à arte do jogo"><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M9 7 5 11l4 4"/><path d="M5 11h9a5 5 0 0 1 0 10h-2"/></svg></button>
            <input type="file" id="f-arquivo" accept="image/jpeg,image/png,image/webp" multiple hidden>
          </div>
          <div class="i-topo-c">
            <div class="rotulo" id="h-rot">Último jogo</div>
            <h2 id="h-nome">—</h2>
            <div class="i-numeros"><span class="num" id="r-num">—</span><small id="r-un"></small><span class="etiquetas" id="r-selos"></span></div>
            <p class="nota" id="r-d">Lendo…</p>
            <div class="i-acoes">
              <button class="botao prim" id="d-turbo" hidden title="Mede o jogo, liga só o que ajuda nele e lembra"><svg viewBox="0 0 24 24">${svgDoca.boost}</svg><span id="d-turbo-t">Turbo</span></button>
              <button class="botao prim" id="d-otimizar"><svg viewBox="0 0 24 24">${svgDoca.otimizar}</svg><span id="d-otimizar-t">Otimizar agora</span></button>
              <button class="botao" id="d-boost" hidden><svg viewBox="0 0 24 24">${svgDoca.boost}</svg><span>Frame Boost</span></button>
              <button class="botao" id="d-lab"><svg viewBox="0 0 24 24">${svgDoca.lab}</svg><span>Testar FPS</span></button>
              <button class="botao fantasma" id="d-desfazer"><svg viewBox="0 0 24 24">${svgDoca.desfazer}</svg><span>Desfazer</span></button>
            </div>
          </div>
        </div>

        <div class="i-cartao i-status">
          <div class="rotulo">Status do PC</div>
          <div class="i-status-topo"><span class="i-ponto" id="s-ponto"></span><b id="s-titulo">Conferindo…</b></div>
          <p class="nota" id="s-sub">Lendo este PC.</p>
          <div class="i-status-corpo">
            <svg class="i-anel" viewBox="0 0 120 120" aria-hidden="true"><defs><linearGradient id="g-acento" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#ffffff"/><stop offset="1" stop-color="#9a9a9a"/></linearGradient></defs><circle cx="60" cy="60" r="50" class="trilha"/><circle cx="60" cy="60" r="50" class="valor" id="s-anel"/></svg>
            <span class="i-anel-num num" id="s-conta">—</span>
            <div id="i-cuidados" class="i-checks"></div>
          </div>
        </div>

        <div class="i-cartao i-desempenho">
          <div class="titulo-sec"><div class="rotulo">Desempenho agora</div><span class="nota" id="p-quando">ao vivo</span></div>
          <div class="i-graf-topo"><span class="num" id="p-cpu-grande">—</span><small>% de processador</small></div>
          <svg class="i-graf" viewBox="0 0 300 80" preserveAspectRatio="none" aria-hidden="true"><defs><linearGradient id="g-area" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#ffffff" stop-opacity="0.22"/><stop offset="1" stop-color="#ffffff" stop-opacity="0"/></linearGradient></defs><path id="p-area" fill="url(#g-area)"/><path id="p-linha" fill="none" stroke="url(#g-acento)" stroke-width="2" vector-effect="non-scaling-stroke"/></svg>
          <div class="i-medidores">
            <div><span class="rotulo">CPU</span><b class="num" id="p-cpu">—</b><i><u id="b-cpu"></u></i></div>
            <div><span class="rotulo">Placa</span><b class="num" id="p-gpu">—</b><i><u id="b-gpu"></u></i></div>
            <div><span class="rotulo">RAM</span><b class="num" id="p-ram">—</b><i><u id="b-ram"></u></i></div>
          </div>
        </div>

        <div class="i-cartao i-atalhos">
          <div class="rotulo">Ações rápidas</div>
          <div class="i-blocos" id="i-blocos"></div>
        </div>
      </div>
      <div class="passos" id="i-passos" hidden></div>
    </section>

    <section class="tela" id="t-jogos" hidden></section>
    <section class="tela" id="t-boost" hidden></section>
    <section class="tela" id="t-mira" hidden></section>
    <section class="tela" id="t-cores" hidden></section>
    <section class="tela" id="t-lab" hidden></section>
    <section class="tela" id="t-ajustes" hidden></section>

  </main>

  <div class="lembrete" id="v-lembrete" hidden>
    <div style="display:flex;justify-content:space-between;align-items:center"><b style="font-weight:500" id="l-titulo">Faltam 2 minutos</b><button id="l-fechar" aria-label="Fechar" style="color:var(--texto-3);font-size:18px">×</button></div>
    <p class="nota">Depois dos 10 minutos as correções pausam até chegar seu feedback no Discord. Medir e desfazer continuam livres.</p>
    <button class="botao" id="l-abrir">Deixar feedback agora</button>
  </div>

  <div class="veu" id="v-veu" hidden>
    <div class="folha" role="dialog" aria-modal="true" aria-labelledby="f-titulo">
      <div><div class="rotulo">10 minutos de uso</div><h2 id="f-titulo" style="margin-top:8px">Conta pra gente como ficou</h2><p class="nota" style="margin-top:6px" id="f-sub">As correções pausam até chegar o seu feedback. Medir, ver o diagnóstico e desfazer continuam livres.</p></div>
      <ol>
        <li><div><h3>Tire um print com o FPS</h3><p>O Lab do Otimiza ou qualquer contador de FPS servem.</p></div></li>
        <li><div><h3>Mande no canal de feedbacks</h3><p>Pelo menos 30 caracteres e o print. Cole este código na mensagem:</p><div class="copiar"><span id="f-codigo">—</span><button id="f-copiar">Copiar</button></div></div></li>
        <li><div><h3>Volte e confirme</h3><p>O bot leva até um minuto para ver a mensagem.</p></div></li>
      </ol>
      <div class="estado" id="f-estado" hidden></div>
      <div class="acoes"><button class="botao prim" id="f-canal" style="flex:1">Abrir o canal</button><button class="botao" id="f-confirmar" style="flex:1">Já mandei</button></div>
      <button class="nota" id="f-depois" style="text-align:center">Continuar só medindo</button>
    </div>
  </div>`;
}

// ---------------------------------------------------------------- navegação

function ir(d: Destino) {
  estado.destino = d;
  $$(".ir", $("#v33")).forEach((b) => b.setAttribute("aria-selected", String(b.dataset.ir === d)));
  $$(".tela", $("#v33")).forEach((t) => (t.hidden = t.id !== `t-${d}`));
  if (d === "jogos") void telaJogos();
  if (d === "boost") void telaBoost();
  if (d === "mira") void telaMira();
  if (d === "cores") void telaCores();
  if (d === "lab") void telaLab();
  if (d === "ajustes") void telaAjustes();
  if (d === "inicio") {
    void cuidados();
    void amostrar();
  }
}

// ---------------------------------------------------------------- painel lateral

async function painel() {
  const arvore = $("#v-arvore");
  const j = estado.jogo;
  const lista = estado.biblioteca.slice(0, 8);
  if (!j && lista.length === 0) {
    arvore.innerHTML = `<div class="vazio-painel">Abra um jogo e jogue alguns minutos: ele aparece aqui com o FPS medido.</div>`;
  } else {
    const linhas: { id: string; nome: string; appid: number | null; dir: string; ativo: boolean }[] = [];
    if (j) linhas.push({ id: idDoJogo(j), nome: j.nome, appid: j.perfil.steam_appid, dir: `<span class="v-vivo"></span>`, ativo: true });
    for (const r of lista) {
      if (j && r.id === idDoJogo(j)) continue;
      linhas.push({ id: r.id, nome: r.nome, appid: null, dir: `<span class="cont">${numero(r.ultima?.fps_nativo)}</span>`, ativo: false });
    }
    arvore.innerHTML = linhas
      .map((l) => `<button class="item ${l.ativo ? "ativo" : ""}" data-jogo="${esc(l.id)}"><img alt="" data-capa="${esc(l.id)}|${esc(l.nome)}|${l.appid ?? ""}"><span>${esc(l.nome)}</span>${l.dir}</button>`)
      .join("");
    $$("img[data-capa]", arvore).forEach(async (img) => {
      const [id, nome, appid] = (img.dataset.capa ?? "").split("|");
      (img as HTMLImageElement).src = await capa(id, nome, appid ? Number(appid) : null);
    });
    $$(".item", arvore).forEach((b) => b.addEventListener("click", () => ir("jogos")));
  }

  const hw = estado.hw;
  if (hw) {
    const tela = hw.telas.find((t) => t.principal) ?? hw.telas[0];
    $("#v-pc").innerHTML = `<div class="rotulo">Este PC</div>
      <div class="linha-pc">Processador<b title="${esc(hw.processador)}">${esc(hw.processador.replace(/\(R\)|\(TM\)|CPU|@.*$/g, "").trim())}</b></div>
      <div class="linha-pc">Placa<b title="${esc(hw.placa.nome)}">${esc(hw.placa.nome.replace(/NVIDIA |AMD |GeForce /g, ""))}</b></div>
      <div class="linha-pc">Memória<b>${numero(hw.memoria_gb, 1)} GB</b></div>
      ${tela ? `<div class="linha-pc">Monitor<b>${tela.hz_atual} Hz${tela.hz_maximo > tela.hz_atual + 5 ? ` de ${tela.hz_maximo}` : ""}</b></div>` : ""}
      <div class="linha-pc">Disco do sistema<b>${esc(hw.disco_do_sistema)}</b></div>`;
  }
}

// ---------------------------------------------------------------- início

/** O jogo mais recente com partida medida: é o que o topo mostra sem jogo aberto. */
function ultimaPartida(): ResumoDoJogo | undefined {
  return estado.biblioteca
    .filter((r) => r.ultima)
    .sort((a, b) => (b.ultima?.quando ?? 0) - (a.ultima?.quando ?? 0))[0];
}

/** A arte da Steam deste PC para o topo. FiveM e RedM rodam sobre o GTA V e o RDR2: a arte é a deles. */
function appidDaArte(id: string, appid: number | null): number | null {
  if (appid) return appid;
  const i = id.toLowerCase();
  if (i.includes("fivem") || i.includes("gta")) return 271590;
  if (i.includes("redm") || i.includes("rdr")) return 1174180;
  return null;
}

const artes = new Map<string, Promise<string | null>>();
let fundos: string[] | null = null;
let arteDoJogo: string | null = null;
let arteMostrada = "";
let camada = 0;
let indiceDoFundo = Math.floor(Math.random() * 8);

/** Troca a arte com fade entre as duas camadas: sem piscar o topo. */
function mostrarArte(url: string | null) {
  const chave = url ?? "";
  if (chave === arteMostrada) return;
  arteMostrada = chave;
  const [a, b] = [$("#arte-a"), $("#arte-b")];
  const entra = camada ? a : b;
  const sai = camada ? b : a;
  camada = 1 - camada;
  if (url) entra.style.backgroundImage = `url("${url}")`;
  else entra.style.removeProperty("background-image");
  entra.classList.add("vis");
  sai.classList.remove("vis");
}

function proximaArte() {
  if (fundos && fundos.length) mostrarArte(fundos[indiceDoFundo++ % fundos.length]);
  else mostrarArte(arteDoJogo);
}

async function carregarFundos() {
  fundos = await ponte.fundosListar().catch(() => []);
  $("#f-limpar").hidden = !fundos.length;
  arteMostrada = "-";
  proximaArte();
}

async function arteDoTopo(id: string | null, nome: string | null, appid: number | null) {
  if (fundos === null) await carregarFundos();
  let url: string | null = null;
  if (id && nome) {
    const a = appidDaArte(id, appid);
    const chave = `${id}:${a ?? ""}`;
    let p = artes.get(chave);
    if (!p) {
      p = (async () => (a ? await ponte.capaLarga(a).catch(() => null) : null) ?? (a ? await capa(id, nome, a) : null))();
      artes.set(chave, p);
    }
    url = await p;
  }
  arteDoJogo = url;
  if (!fundos?.length) mostrarArte(url);
}

function ligarImagens() {
  const arquivo = $<HTMLInputElement>("#f-arquivo");
  $("#f-por").addEventListener("click", () => arquivo.click());
  // Várias de uma vez (até 8 no total). O erro vai para o quadro de mensagens: no texto do topo ele sumia na
  // atualização seguinte, e a imagem recusada parecia "não ter passado" sem motivo.
  const ler = (f: File) =>
    new Promise<string>((ok, falha) => {
      const leitor = new FileReader();
      leitor.onload = () => ok(String(leitor.result));
      leitor.onerror = () => falha(new Error("não consegui ler o arquivo"));
      leitor.readAsDataURL(f);
    });
  arquivo.addEventListener("change", async () => {
    const escolhidos = [...(arquivo.files ?? [])];
    arquivo.value = "";
    if (!escolhidos.length) return;
    const recusadas: { texto: string; classe?: string }[] = [];
    let entraram = 0;
    for (const f of escolhidos) {
      try {
        await ponte.fundosAdicionar(await ler(f));
        entraram += 1;
      } catch (erro) {
        recusadas.push({ texto: `${f.name}: ${String(erro)}`, classe: "falhou" });
      }
    }
    await carregarFundos();
    if (entraram) {
      indiceDoFundo = Math.max(0, (fundos?.length ?? 1) - entraram);
      proximaArte();
    }
    mostrarPassos(
      recusadas.length
        ? [{ texto: `${entraram} de ${escolhidos.length} imagem(ns) no topo.`, classe: entraram ? "feito" : "" }, ...recusadas]
        : [],
    );
  });
  $("#f-limpar").addEventListener("click", async () => {
    await ponte.fundosLimpar().catch(() => {});
    await carregarFundos();
  });
  // Com mais de uma imagem, elas se alternam devagar, só com o Início na frente.
  window.setInterval(() => {
    if (estado.destino === "inicio" && !document.hidden && (fundos?.length ?? 0) > 1) proximaArte();
  }, 14000);
}

const ICONE_OK = `<svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="9"/><path d="m8 12.5 2.6 2.6L16 9.6"/></svg>`;
const ICONE_PENDENTE = `<svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="9"/><path d="M12 8v4.5M12 16h.01"/></svg>`;

const BLOCOS: { id: string; nome: string; icone: string; ir: Destino }[] = [
  { id: "energia", nome: "Energia de jogo", icone: `<path class="preenche" d="M13.2 3 5.8 13.2h5.4L10.4 21l7.6-10.4h-5.5z"/><path d="M13.2 3 5.8 13.2h5.4L10.4 21l7.6-10.4h-5.5z"/>`, ir: "boost" },
  { id: "memoria", nome: "Liberar memória", icone: `<rect class="preenche" x="3" y="7.2" width="18" height="8.6" rx="1.6"/><rect x="3" y="7.2" width="18" height="8.6" rx="1.6"/><path d="M6.6 10h2.2v3H6.6zM10.9 10h2.2v3h-2.2zM15.2 10h2.2v3h-2.2z"/><path d="M5.5 15.8v2.4M8.5 15.8v2.4M11.5 15.8v2.4M14.5 15.8v2.4M17.5 15.8v2.4"/>`, ir: "boost" },
  { id: "liso", nome: "Jogo liso", icone: `<path d="M3 13.5 5 8.5l2.2 7 2.2-5.5 1.9 3.5H21"/><circle cx="21" cy="13.5" r="1.3" class="ponto"/>`, ir: "boost" },
  { id: "boost", nome: "Frame Boost", icone: `<rect class="preenche" x="7.4" y="8" width="13" height="9.6" rx="1.8"/><rect x="7.4" y="8" width="13" height="9.6" rx="1.8"/><path d="M4.4 14.6V7.3A1.8 1.8 0 0 1 6.2 5.5h10.6"/><path d="m12.6 10.6 3 2.2-3 2.2z" class="ponto"/>`, ir: "boost" },
  { id: "driver", nome: "Driver da placa", icone: `<rect class="preenche" x="2.8" y="6.4" width="18.4" height="10.4" rx="1.8"/><rect x="2.8" y="6.4" width="18.4" height="10.4" rx="1.8"/><circle cx="9" cy="11.6" r="3"/><circle cx="9" cy="11.6" r=".9" class="ponto"/><path d="M14.4 9.6h3.8M14.4 12h3.8M14.4 14.4h2.2M5.2 16.8v2.2M8.2 16.8v2.2M11.2 16.8v2.2"/>`, ir: "boost" },
  { id: "lab", nome: "Testar FPS", icone: `<path d="M4 20h16"/><rect class="preenche" x="5.5" y="12" width="3" height="8" rx=".8"/><rect x="5.5" y="12" width="3" height="8" rx=".8"/><rect class="preenche" x="10.5" y="8" width="3" height="12" rx=".8"/><rect x="10.5" y="8" width="3" height="12" rx=".8"/><rect class="preenche" x="15.5" y="4.5" width="3" height="15.5" rx=".8"/><rect x="15.5" y="4.5" width="3" height="15.5" rx=".8"/>`, ir: "lab" },
];

/** O status do PC e as ações rápidas, cada linha lida neste PC. Separado do `inicio()`: a energia leva segundos para
 * ler, e o `inicio()` roda a cada evento. O anel conta só o que foi conferido aqui — nunca um "100%" de enfeite. */
async function cuidados() {
  const caixa = document.querySelector<HTMLElement>("#i-cuidados");
  if (!caixa) return;
  const [energia, memoria] = await Promise.all([
    ponte.energiaDeJogo().catch(() => null),
    ponte.memoriaParaOJogo().catch(() => null),
  ]);
  let liso: { trancos: number; quando: number } | null = null;
  try {
    liso = JSON.parse(localStorage.getItem("otimiza-liso-ultima") ?? "null");
  } catch {
    liso = null;
  }
  const t = estado.hw?.telas.find((x) => x.principal) ?? estado.hw?.telas[0];
  const apertada = !!memoria && memoria.livre_gb < 2.5 && memoria.total_gb <= 16;

  const itens: { nome: string; ok: boolean | null; valor: string }[] = [
    { nome: "Energia de jogo", ok: energia ? energia.ligado : null, valor: !energia ? "não lida" : energia.agindo_em ? `agindo em ${energia.agindo_em}` : energia.ligado ? "ligada" : "desligada" },
    { nome: "Memória livre", ok: memoria ? !apertada : null, valor: memoria ? `${numero(memoria.livre_gb, 1)} de ${numero(memoria.total_gb, 1)} GB` : "não lida" },
    { nome: "Ritmo do jogo", ok: liso ? liso.trancos < 2 : null, valor: liso ? `${numero(liso.trancos)} trancos/min` : "não medido" },
    { nome: "Monitor", ok: t ? !(t.hz_maximo > t.hz_atual + 5) : null, valor: t ? `${t.hz_atual} Hz` : "não lido" },
  ];
  caixa.innerHTML = itens
    .map((i) => `<button class="i-check ${i.ok === true ? "ok" : i.ok === false ? "aviso" : ""}" data-ir="boost">${i.ok === true ? ICONE_OK : ICONE_PENDENTE}<span>${esc(i.nome)}</span><b>${esc(i.valor)}</b></button>`)
    .join("");

  const conferidos = itens.filter((i) => i.ok !== null);
  const certos = itens.filter((i) => i.ok === true).length;
  const anel = document.querySelector<SVGCircleElement>("#s-anel");
  const volta = 2 * Math.PI * 50;
  if (anel) {
    anel.style.strokeDasharray = `${volta}`;
    anel.style.strokeDashoffset = `${volta * (1 - certos / itens.length)}`;
  }
  $("#s-conta").textContent = `${certos}/${itens.length}`;
  const pendentes = conferidos.length - certos;
  const tudo = pendentes === 0 && conferidos.length === itens.length;
  $("#s-ponto").className = `i-ponto ${tudo ? "ok" : pendentes > 0 ? "aviso" : ""}`;
  $("#s-titulo").textContent = tudo ? "Pronto para jogar" : pendentes > 0 ? `${pendentes} ${pendentes === 1 ? "ponto pede" : "pontos pedem"} atenção` : "Falta medir o jogo";
  $("#s-sub").textContent = tudo
    ? "Tudo conferido neste PC agora."
    : apertada && memoria?.pesados[0]
      ? `${memoria.pesados[0].nome} ocupa ${numero(memoria.pesados[0].mb / 1024, 1)} GB da memória.`
      : !liso
        ? "Meça o jogo no Boost para saber se ele trava."
        : "Toque num item para ver o que fazer.";

  // Ações rápidas: o estado ao lado de cada uma é o lido agora.
  const estadoDe: Record<string, boolean> = {
    energia: !!energia?.ligado,
    memoria: !!memoria && !apertada,
    liso: !!liso && liso.trancos < 2,
    boost: estado.gerador?.situacao === "Gerando",
  };
  $("#i-blocos").innerHTML = BLOCOS.map(
    (b) => `<button class="i-bloco ${estadoDe[b.id] ? "ligado" : ""}" data-ir="${b.ir}">${estadoDe[b.id] ? `<span class="i-marca">${ICONE_OK}</span>` : ""}<svg viewBox="0 0 24 24" aria-hidden="true">${b.icone}</svg><span>${b.nome}</span></button>`,
  ).join("");

  document.querySelectorAll<HTMLButtonElement>("#t-inicio [data-ir]").forEach((b) => {
    b.onclick = () => ir(b.dataset.ir as Destino);
  });
}

// ---------------------------------------------------------------- desempenho ao vivo

const amostrasCpu: number[] = [];

function desenharGrafico() {
  const n = 30;
  const pts = amostrasCpu.slice(-n);
  if (pts.length < 2) return;
  const passo = 300 / (n - 1);
  const x0 = 300 - (pts.length - 1) * passo;
  const y = (v: number) => 76 - (Math.min(100, Math.max(0, v)) / 100) * 70;
  const linha = pts.map((v, i) => `${i ? "L" : "M"}${(x0 + i * passo).toFixed(1)} ${y(v).toFixed(1)}`).join(" ");
  $("#p-linha").setAttribute("d", linha);
  $("#p-area").setAttribute("d", `${linha} L300 80 L${x0.toFixed(1)} 80 Z`);
}

function medidor(id: string, barra: string, pct: number | null, texto: string) {
  $(id).textContent = texto;
  $(barra).style.width = pct === null ? "0" : `${Math.min(100, Math.max(0, pct))}%`;
}

let amostrando = false;
async function amostrar() {
  if (amostrando || estado.destino !== "inicio" || document.hidden) return;
  amostrando = true;
  try {
    const m = await ponte.metricas();
    const cpu = m.cpu.overall;
    const gpu = m.telemetry?.metrics?.["gpu.usage"]?.value ?? null;
    const ram = m.ram.total_gb > 0 ? (m.ram.used_gb / m.ram.total_gb) * 100 : null;
    if (cpu !== null) {
      amostrasCpu.push(cpu);
      if (amostrasCpu.length > 60) amostrasCpu.shift();
    }
    $("#p-cpu-grande").textContent = cpu === null ? "—" : numero(cpu);
    medidor("#p-cpu", "#b-cpu", cpu, cpu === null ? "—" : `${numero(cpu)}%`);
    medidor("#p-gpu", "#b-gpu", gpu, gpu === null ? "—" : `${numero(gpu)}%`);
    medidor("#p-ram", "#b-ram", ram, `${numero(m.ram.used_gb, 1)}/${numero(m.ram.total_gb, 0)} GB`);
    $("#p-quando").textContent = "ao vivo · últimos 60 s";
    desenharGrafico();
  } catch {
    $("#p-quando").textContent = "sem leitura agora";
  } finally {
    amostrando = false;
  }
}

function inicio() {
  const j = estado.jogo;
  const gerando = estado.gerador?.situacao === "Gerando";
  const fb = frameBoost();

  $("#i-titulo").textContent = saudacao();
  $("#i-sub").textContent = j
    ? `${j.nome} aberto agora · ${API[j.perfil.api] ?? j.perfil.api} · ${ANTICHEAT[j.perfil.anticheat] ?? j.perfil.anticheat}`
    : "Seu PC está pronto para jogar. Abra um jogo e o Otimiza mede sozinho.";
  $("#i-pilula").classList.toggle("parado", !j);
  $("#i-pilula-t").textContent = gerando ? "Frame Boost ativo" : j ? "Jogo aberto" : "Nenhum jogo aberto";

  // O topo: o jogo aberto, ou o último medido. O número é sempre um medido.
  const r = j ? resumoDoJogoAberto() : ultimaPartida();
  const nome = j?.nome ?? r?.nome ?? null;
  $("#h-rot").textContent = j ? "Aberto agora" : r ? "Último jogo" : "Comece por aqui";
  $("#h-nome").textContent = nome ?? "Abra um jogo";
  void arteDoTopo(j ? idDoJogo(j) : r?.id ?? null, nome, j?.perfil.steam_appid ?? null);

  const num = $("#r-num");
  const selos: string[] = [];
  if (gerando && estado.gerador) {
    // A mesma regra do painel do Boost: medido é o que chegou à tela (PresentMon); o apresentado é só estimativa.
    const exibidos = estado.gerador.fps_exibidos ?? null;
    contar(num, exibidos !== null && exibidos > 0 ? exibidos : estado.gerador.fps_apresentados, 1300);
    $("#r-un").textContent = "FPS na tela";
    const modo = estado.gerador.automatico
      ? "Auto"
      : estado.gerador.escala && estado.gerador.multiplicador <= 1
        ? "só escala"
        : `${estado.gerador.multiplicador}×`;
    $("#r-d").textContent = `Frame Boost ${modo}${estado.gerador.escala && modo !== "só escala" ? " + escala" : ""} · desligue com Ctrl+Alt+G`;
    selos.push(
      exibidos !== null && exibidos > 0
        ? `<span class="selo medido">medido na tela</span>`
        : `<span class="selo estimado" title="Quadros apresentados pelo gerador. Abra o Otimiza como administrador para medir o que chega à tela.">estimado</span>`
    );
  } else if (r?.ultima) {
    contar(num, r.ultima.fps_nativo);
    $("#r-un").textContent = "FPS";
    $("#r-d").textContent = `1% low ${numero(r.ultima.low_1pct)}${r.ultima.frametime_ms ? ` · ${numero(r.ultima.frametime_ms, 1)} ms por quadro` : ""} · ${haQuanto(r.ultima.quando)}`;
    selos.push(`<span class="selo medido">medido</span>`);
    if (r.ganho_fps.estado === "Medido") selos.push(`<span class="delta">▲ ${numero(r.ganho_fps.pct)}%</span>`);
  } else {
    num.textContent = "—";
    delete num.dataset.valor;
    $("#r-un").textContent = "";
    $("#r-d").textContent = j ? "Jogue um minuto em partida: o Otimiza mede o FPS e os trancos sozinho." : "Abra um jogo e jogue um minuto em partida: o Otimiza mede o FPS e os trancos sozinho.";
  }
  $("#r-selos").innerHTML = selos.join("");
  // Sem número medido, nada de "—" solto no topo: só o texto que diz como medir.
  $(".i-numeros").hidden = !(gerando || r?.ultima);

  const bBoost = $<HTMLButtonElement>("#d-boost");
  // Some sem jogo em vez de ficar cinza: botão que não faz nada é ruído.
  bBoost.hidden = !j || (!fb && !gerando);
  bBoost.classList.toggle("ligado", gerando);
  // Com o jogo aberto, o Turbo é o botão principal da tela; o "Otimizar agora" fica ao lado.
  $<HTMLButtonElement>("#d-turbo").hidden = !j;
  $("#d-otimizar").classList.toggle("prim", !j);
  $<HTMLButtonElement>("#d-otimizar").disabled = estado.otimizando;
}

/** O Turbo da tela do jogo: com a receita lembrada (e não reprovada), um clique; sem ela, mede 10 s. Depois, o cartão
 *  do Turbo no Boost mostra o que foi ligado e a prova. Já ligado: vai direto para o cartão. */
async function turboDaTelaDoJogo() {
  const botao = $<HTMLButtonElement>("#d-turbo");
  const rotulo = $("#d-turbo-t");
  botao.disabled = true;
  try {
    const p = await ponte.turbo();
    if (!p.ligado && !p.provando) {
      const lembrado = !!p.perfil && p.perfil.prova !== "Piorou";
      rotulo.textContent = lembrado ? "Ligando…" : "Medindo 10 s… jogue normalmente";
      await ponte.turboLigar(lembrado);
      estado.gerador = await ponte.geradorEstado().catch(() => estado.gerador);
    }
    ir("boost");
  } catch (e) {
    mostrarPassos([{ texto: String(e) }]);
  } finally {
    rotulo.textContent = "Turbo";
    botao.disabled = false;
    inicio();
  }
}

function mostrarPassos(linhas: { texto: string; classe?: string }[]) {
  const caixa = $("#i-passos");
  caixa.hidden = linhas.length === 0;
  caixa.innerHTML = linhas.map((l) => `<div class="${l.classe ?? ""}">${esc(l.texto)}</div>`).join("");
  caixa.scrollTop = caixa.scrollHeight;
}

async function otimizar() {
  if (estado.otimizando) return;
  estado.otimizando = true;
  $("#d-otimizar-t").textContent = "Otimizando…";
  inicio();
  const linhas: { texto: string; classe?: string }[] = [{ texto: "Conferindo este PC e o que é compatível…" }];
  mostrarPassos(linhas);
  const parar = await ouvir<PassoDoLote>("optimize:step", (p) => {
    if (p.stage !== "finished") return;
    linhas.push({ texto: p.name, classe: p.success ? "feito" : "falhou" });
    mostrarPassos(linhas.slice(-9));
  });
  try {
    const r = await ponte.otimizar();
    const aplicados = r.filter((x) => x.success && x.applied).length;
    linhas.push({ texto: aplicados ? `${aplicados} ajuste(s) aplicados. Jogue alguns minutos: o Otimiza mede o antes e o depois.` : "Nada a mudar: o que é compatível já estava certo.", classe: "feito" });
  } catch (e) {
    linhas.push({ texto: String(e), classe: "falhou" });
  } finally {
    parar();
    mostrarPassos(linhas.slice(-9));
    estado.otimizando = false;
    $("#d-otimizar-t").textContent = "Otimizar agora";
    void cuidados();
    inicio();
    setTimeout(() => mostrarPassos([]), 12000);
  }
}

let vigiaDoGerador = 0;
/** O botão Frame Boost da doca: liga o que o automático escolheu (gerador do Otimiza, AFMF ou Smooth Motion) ou desliga. */
async function alternarBoost() {
  if (!estado.jogo) return;
  try {
    const p = await ponte.frameBoost();
    if (p.ativo) {
      await ponte.frameBoostDesligar();
    } else {
      const escala = escalaPedida();
      const novo = await ponte.frameBoostLigar(
        multiplicadorQueCabe(multiplicadoresOferecidos(p.multiplicadores), escala !== null),
        semLimite(),
        escala
      );
      if (novo.ativo?.pelo_driver) {
        mostrarPassos([{ texto: `${novo.recomendado?.nome ?? "Frame Boost"} ligado no driver. Se o jogo já estava aberto, feche e abra de novo. O FPS na tela aparece medido no Boost.`, classe: "feito" }]);
      } else {
        clearInterval(vigiaDoGerador);
        vigiaDoGerador = window.setInterval(async () => {
          estado.gerador = await ponte.geradorEstado();
          // Procurando a janela é o gerador se refazendo (janela redimensionada, minimizada): continua ligado.
          if (estado.gerador.situacao !== "Gerando" && estado.gerador.situacao !== "ProcurandoJanela") clearInterval(vigiaDoGerador);
          inicio();
        }, 1000);
      }
    }
    estado.gerador = await ponte.geradorEstado();
  } catch (e) {
    mostrarPassos([{ texto: String(e), classe: "falhou" }]);
  }
  inicio();
}

// ---------------------------------------------------------------- jogos

async function telaJogos() {
  const t = $("#t-jogos");
  try {
    estado.biblioteca = await ponte.biblioteca();
  } catch {
    // Fica a lista que já havia.
  }
  const j = estado.jogo;
  const destaque = j ?? null;
  const ultimo = ultimaPartida();
  // A prateleira: os jogos medidos e os instalados (Steam, Epic, instaladores), um cartão por jogo.
  const instalados = await ponte.jogosInstalados().catch(() => [] as JogoDaLoja[]);
  const simples = (n: string) => n.toLowerCase().replace(/[^a-z0-9]/g, "");
  const mesmo = (a: string, b: string) => simples(a) === simples(b) || (simples(a).length > 3 && simples(b).startsWith(simples(a))) || (simples(b).length > 3 && simples(a).startsWith(simples(b)));
  const cartoes: { id: string; nome: string; loja: string; appid: number | null; r: ResumoDoJogo | null }[] = estado.biblioteca.map((r) => ({ id: r.id, nome: r.nome, loja: "", appid: null, r }));
  for (const i of instalados) {
    const c = cartoes.find((x) => mesmo(x.nome, i.nome));
    if (c) {
      c.loja = i.loja;
      c.appid = c.appid ?? i.appid;
    } else cartoes.push({ id: simples(i.nome), nome: i.nome, loja: i.loja, appid: i.appid, r: null });
  }
  const ganho = (r: ResumoDoJogo) => (r.ganho_fps.estado === "Medido" ? `<span class="delta">▲ ${numero(r.ganho_fps.pct)}%</span>` : r.ganho_fps.estado === "SemDiferenca" ? `<span class="nota">sem diferença</span>` : `<span class="nota">medindo</span>`);

  t.innerHTML = `
    <div class="cabeca"><div><h1><span class="barras"></span>Meus jogos</h1><p>Cada jogo com o próprio histórico. O ganho só aparece quando a medição separa o antes do depois.</p></div>
    <span class="pilula ${j ? "" : "parado"}"><span class="v-vivo"></span>${estado.biblioteca.length} jogo(s) medido(s)</span></div>
    ${!destaque && ultimo?.ultima ? `<div class="destaque"><img id="j-dest" alt=""><div class="dest-c"><div><span class="pilula parado"><span class="v-vivo"></span>Último jogo</span><h2>${esc(ultimo.nome)}</h2><p class="nota" style="margin-top:4px">${haQuanto(ultimo.ultima.quando)} · ${ultimo.partidas} partida(s) medida(s)</p></div>
      <div class="metricas"><div><div class="rotulo">FPS nativo</div><div class="v">${numero(ultimo.ultima.fps_nativo)}<small>FPS</small></div></div><div><div class="rotulo">1% low</div><div class="v">${numero(ultimo.ultima.low_1pct)}<small>FPS</small></div></div><div><div class="rotulo">Melhor</div><div class="v">${numero(ultimo.melhor_fps)}<small>FPS</small></div></div></div></div></div>` : ""}
    ${destaque ? `<div class="destaque"><img id="j-dest" alt=""><div class="dest-c"><div><span class="pilula"><span class="v-vivo"></span>Jogando agora</span><h2>${esc(destaque.nome)}</h2><p class="nota" style="margin-top:4px">${esc(ANTICHEAT[destaque.perfil.anticheat] ?? "")}${destaque.perfil.nota ? " · " + esc(destaque.perfil.nota) : ""}</p></div>
      <div class="metricas">${(() => {
        const r = resumoDoJogoAberto();
        return r?.ultima
          ? `<div><div class="rotulo">FPS nativo</div><div class="v">${numero(r.ultima.fps_nativo)}<small>FPS</small></div></div><div><div class="rotulo">1% low</div><div class="v">${numero(r.ultima.low_1pct)}<small>FPS</small></div></div><div><div class="rotulo">Partidas</div><div class="v">${r.partidas}</div></div>`
          : `<div class="nota">Ainda sem medição deste jogo.</div>`;
      })()}</div></div></div>` : ""}
    ${cartoes.length === 0
      ? `<div class="v-vazio" style="margin-top:24px">Nenhum jogo achado na Steam, na Epic ou nos instaladores deste PC. Abra um jogo e jogue alguns minutos com o Otimiza aberto como administrador: ele aparece aqui com o FPS de cada partida.</div>`
      : `<div class="titulo-sec" style="margin-top:28px"><div class="rotulo">Neste PC</div><span class="nota">${cartoes.length} jogo(s)${instalados.length ? " · Steam, Epic e instaladores" : ""}</span></div>
        <div class="grade g-jogos" style="margin-top:0">${cartoes
          .map((c) => `<button class="jogo-card" data-id="${esc(c.id)}"><img alt="Capa de ${esc(c.nome)}" data-capa="${esc(c.id)}|${esc(c.nome)}|${c.appid ?? ""}"><div class="topo">${j && c.r && idDoJogo(j) === c.r.id ? `<span class="pilula"><span class="v-vivo"></span>ao vivo</span>` : c.loja && c.loja !== "Instalado" ? `<span class="loja">${esc(c.loja)}</span>` : ""}</div>
            <div class="sobre"><div class="nome-jogo">${esc(c.nome)}</div>${c.r?.ultima
              ? `<div style="display:flex;justify-content:space-between;align-items:baseline"><span class="fps">${numero(c.r.ultima.fps_nativo)}<small style="font-size:11px;color:var(--texto-2)"> FPS</small></span>${ganho(c.r)}</div><div class="nota" style="font-size:11.5px">${haQuanto(c.r.ultima.quando)} · ${c.r.partidas} partida(s)</div>`
              : `<div class="nota" style="font-size:11.5px">Ainda não medido · abra e jogue um minuto</div>`}</div></button>`)
          .join("")}</div>`}`;

  const dest = document.querySelector<HTMLImageElement>("#j-dest");
  const d = destaque ? { id: idDoJogo(destaque), nome: destaque.nome, appid: destaque.perfil.steam_appid } : ultimo ? { id: ultimo.id, nome: ultimo.nome, appid: null } : null;
  if (dest && d) {
    const a = appidDaArte(d.id, d.appid);
    dest.src = (a ? await ponte.capaLarga(a).catch(() => null) : null) ?? (await capa(d.id, d.nome, a));
  }
  const catalogo = await ponte.catalogo().catch(() => []);
  for (const img of $$<HTMLImageElement>("img[data-capa]", t)) {
    const [id, nome, appid] = (img.dataset.capa ?? "").split("|");
    const doCatalogo = catalogo.find((p) => p.id === id)?.steam_appid ?? null;
    img.src = await capa(id, nome, appidDaArte(`${id} ${nome}`, appid ? Number(appid) : doCatalogo));
  }
}

// ---------------------------------------------------------------- boost

async function telaBoost() {
  const t = $("#t-boost");
  const fab = estado.hw?.placa.fabricante ?? "Desconhecido";

  t.innerHTML = `
    <div class="cabeca"><div><h1><span class="barras"></span>Boost</h1><p>Só entra o que é compatível com o seu PC${estado.jogo ? " e com o " + esc(estado.jogo.nome) : ""}. Tudo se desfaz em um clique.</p></div>
      <span class="pilula">${esc(estado.hw?.placa.nome ?? "Placa de vídeo")}</span></div>
    <div class="vidro" id="b-turbo" style="margin-bottom:16px"><div class="rotulo">Turbo</div><p class="nota" style="margin-top:8px">Lendo…</p></div>
    <div class="modos">
      <div class="modo" aria-pressed="true"><svg viewBox="0 0 24 24"><path d="M13.2 3 5.8 13.2h5.4L10.4 21l7.6-10.4h-5.5z"/></svg><b>Máximo FPS</b><span>Os ajustes do Windows e do driver que têm evidência de ganho, verificados neste PC.</span><button class="botao prim" id="b-otimizar">Otimizar agora</button></div>
      <div class="modo"><svg viewBox="0 0 24 24"><path d="M4 7.5h9M17.5 7.5H20M4 16.5h2.5M11 16.5h9"/><circle cx="15.2" cy="7.5" r="2.3"/><circle cx="8.8" cy="16.5" r="2.3"/></svg><b>Personalizado</b><span>Cada ajuste, um por um, com o que ele muda e o valor de antes.</span><button class="botao" id="b-classico">Abrir</button></div>
    </div>
    <div class="grade g-2-iguais">
      <div class="vidro" id="b-energia"><div class="rotulo">Energia de jogo</div><p class="nota" style="margin-top:8px">Lendo…</p></div>
      <div class="vidro" id="b-liso"><div class="rotulo">Jogo liso</div><p class="nota" style="margin-top:8px">Abra o jogo para medir.</p></div>
    </div>
    <div class="vidro" id="b-mem" style="margin-top:16px"><div class="rotulo">Memória para o jogo</div><p class="nota" style="margin-top:8px">Lendo…</p></div>
    <div class="grade g-2-iguais">
      <div class="vidro" id="b-entrada"><div class="rotulo">Mouse e teclado</div><p class="nota" style="margin-top:8px">Lendo…</p></div>
      <div class="vidro" id="b-leve"><div class="rotulo">Windows mais leve</div><p class="nota" style="margin-top:8px">Lendo…</p></div>
    </div>
    <div class="vidro" id="b-lixo" style="margin-top:16px"><div class="rotulo">Remoção de lixo</div><p class="nota" style="margin-top:8px">Medindo…</p></div>
    <div class="grade g-2">
      <div class="vidro"><div class="titulo-sec"><h2>Driver da placa de vídeo</h2><span class="nota" id="b-drv-nota"></span></div><div id="b-drv"><p class="nota">Lendo o driver…</p></div></div>
      <div class="vidro" id="b-fb"><div class="rotulo">Frame Boost</div><p class="nota" style="margin-top:8px">Lendo…</p></div>
    </div>
    <div id="b-comparar"></div>
    <div class="vidro" id="b-config" style="margin-top:16px"><div class="rotulo">Configuração do jogo</div><p class="nota" style="margin-top:8px">Procurando…</p></div>`;

  $("#b-otimizar").addEventListener("click", () => {
    ir("inicio");
    void otimizar();
  });
  $("#b-classico").addEventListener("click", () => modoClassico(true));
  void cartaoMemoria();
  void cartaoEnergia();
  void cartaoEntrada();
  void cartaoLeve();
  void cartaoLixo();
  void cartaoJogoLiso();
  void cartaoTurbo();
  void painelFrameBoost();
  void cartaoConfigDosJogos();
  await driver(fab);
}

/** O perfil leve no arquivo de configuração de cada jogo achado: o que custa caro e quase não se vê. */
async function cartaoConfigDosJogos() {
  const caixa = document.querySelector<HTMLElement>("#b-config");
  if (!caixa) return;
  const jogos = await ponte.configDosJogos().catch(() => null);
  const titulo = `<div class="rotulo">Configuração do jogo</div>`;
  if (!jogos) {
    caixa.innerHTML = `${titulo}<p class="nota" style="margin-top:8px">Não consegui ler a configuração dos jogos.</p>`;
    return;
  }
  const achados = jogos.filter((j) => j.arquivo);
  const linhas = achados.map((j) => {
    const mudancas = j.previsao.map((m) => `<p>${esc(m.por_que)} <span class="nota">(${esc(m.antes)} → ${esc(m.depois)})</span></p>`).join("");
    const acao = j.aplicado
      ? `<button class="botao" data-desfazer="${esc(j.historico)}">Desfazer</button>`
      : j.previsao.length
        ? `<button class="botao" data-aplicar="${esc(j.id)}" ${j.aberto ? "disabled" : ""}>Aplicar</button>`
        : "";
    const estado = j.aplicado
      ? "Perfil leve aplicado pelo Otimiza. O desfazer devolve o arquivo exatamente como estava."
      : j.previsao.length
        ? j.aberto
          ? "Feche o jogo para aplicar: ele reescreve a configuração ao sair."
          : ""
        : "Já está leve: nada a mudar.";
    return `<div class="linha"><div>${esc(j.nome)}${estado ? `<p>${esc(estado)}</p>` : ""}${j.aplicado ? "" : mudancas}</div>${acao}</div>`;
  });
  caixa.innerHTML = `${titulo}
    <p class="nota" style="margin-top:8px">O que custa caro e quase não se vê, no arquivo que o próprio jogo grava. Só muda o que já existe nele, nunca com o jogo aberto, e tudo se desfaz byte a byte.</p>
    ${linhas.join("") || `<p class="nota" style="margin-top:8px">Nenhum dos jogos cobertos (Fortnite, CS2, Apex) foi achado neste PC.</p>`}
    <p class="nota" style="margin-top:8px">Valorant e Warzone: em estudo. A configuração deles ainda não é escrita.</p>
    <p class="estado" id="cf-estado" hidden></p>`;
  const erro = (e: unknown) => {
    const s = $("#cf-estado");
    s.hidden = false;
    s.textContent = String(e);
  };
  $$<HTMLButtonElement>("#b-config [data-aplicar]").forEach((b) =>
    b.addEventListener("click", async () => {
      b.disabled = true;
      try {
        await ponte.configAplicar(b.dataset.aplicar ?? "");
        await cartaoConfigDosJogos();
      } catch (e) {
        b.disabled = false;
        erro(e);
      }
    })
  );
  $$<HTMLButtonElement>("#b-config [data-desfazer]").forEach((b) =>
    b.addEventListener("click", async () => {
      b.disabled = true;
      try {
        await ponte.desfazer(b.dataset.desfazer ?? "");
        await cartaoConfigDosJogos();
      } catch (e) {
        b.disabled = false;
        erro(e);
      }
    })
  );
}

const NOMES_DO_GARGALO: Record<Gargalo, string> = {
  Placa: "placa de vídeo no limite",
  Processador: "processador no limite",
  Memoria: "memória acabando",
  Nenhum: "nada no limite",
  SemCarga: "jogo parado na medida",
};

/** Em palavras curtas, o que a receita liga. */
function resumoDaReceita(r: ReceitaDoTurbo): string {
  const partes: string[] = [];
  if (r.escala) partes.push(`escala ${NOMES_DA_ESCALA[r.escala]}`);
  if (r.multiplicador !== null && r.multiplicador !== 1) partes.push(r.multiplicador === 0 ? "geração Auto" : `geração ${r.multiplicador}×`);
  if (r.energia) partes.push("energia de jogo");
  if (r.memoria) partes.push("memória");
  return partes.join(" + ") || "nada";
}

/** O botão que mede o jogo, escolhe pelo gargalo e aplica. */
async function cartaoTurbo(dado?: PainelTurbo) {
  const caixa = document.querySelector<HTMLElement>("#b-turbo");
  if (!caixa) return;
  const p = dado ?? (await ponte.turbo().catch(() => null));
  if (!p) {
    caixa.innerHTML = `<div class="rotulo">Turbo</div><p class="nota" style="margin-top:8px">Não consegui ler o Turbo.</p>`;
    return;
  }
  const lembrado = p.perfil;
  const reprovada = lembrado?.prova === "Piorou";
  const minutos = Math.round((p.duracao_da_prova_s ?? 318) / 60);
  const corpo = !p.jogo
    ? `<p class="nota" style="margin-top:8px">Abra um jogo. O Turbo mede o jogo aberto por 10 segundos, vê o que está no limite (placa, processador ou memória) e liga só o que ajuda nele.</p>`
    : p.provando
      ? `<p class="nota" style="margin-top:8px" id="tb-progresso">Provando…</p>
         <p class="nota" style="margin-top:4px">Jogue normalmente: o Turbo liga e desliga sozinho em 8 rodadas e mede o FPS do jogo em cada uma.</p>`
      : p.ligado
        ? `${p.passos.map((t, i) => `<p class="nota" style="margin-top:${i ? 4 : 8}px">${i ? "· " : ""}${esc(t)}</p>`).join("")}
           <div class="acoes" style="margin-top:14px"><button class="botao prim" id="tb-provar" title="8 rodadas alternadas, com e sem o Turbo, com o jogo aberto">Provar (~${minutos} min)</button><button class="botao" id="tb-desligar">Desligar Turbo</button></div>`
        : `<p class="nota" style="margin-top:8px">${
            lembrado
              ? `Da última vez no ${esc(p.jogo)}: ${esc(NOMES_DO_GARGALO[lembrado.receita.gargalo])}${lembrado.fps_antes ? ` a ${numero(lembrado.fps_antes)} fps` : ""} → ${esc(resumoDaReceita(lembrado.receita))}.${reprovada ? " Na prova, esta receita deixou o jogo pior neste PC: o Turbo não a liga de novo." : ""}`
              : `Mede o ${esc(p.jogo)} por 10 segundos (jogue normalmente enquanto isso), vê o que está no limite e liga só o que ajuda nele.`
          }${p.competitivo ? " Jogo competitivo: sem geração de quadros, para não acrescentar atraso." : ""}</p>
           <div class="acoes" style="margin-top:14px">${
             lembrado
               ? `${reprovada ? "" : `<button class="botao prim" id="tb-lembrado">Turbo</button>`}<button class="botao ${reprovada ? "prim" : ""}" id="tb-medir">Medir de novo</button><button class="botao" id="tb-esquecer">Esquecer</button>`
               : `<button class="botao prim" id="tb-medir">Turbo</button>`
           }</div>`;
  // A prova A-B: o único número do Turbo que vale como prova (margem de erro, rodadas alternadas) e que se compartilha.
  const pv = !p.provando ? p.prova : null;
  const lado = (r: { media: number; margem: number | null } | null) =>
    r ? `${numero(r.media)}${r.margem !== null ? ` ± ${numero(Math.ceil(r.margem))}` : ""}` : "—";
  const pct = pv?.diferenca_fps?.Real?.pct ?? null;
  const selosDaProva: Record<string, string> = {
    Ganhou: `<span class="selo medido">provado${pct !== null ? `: ${pct > 0 ? "+" : ""}${numero(pct, 1)}%` : ""}</span>`,
    Piorou: `<span class="selo">piorou: Turbo desligado</span>`,
    Indistinguivel: `<span class="selo">dentro do ruído</span>`,
    SemComparacaoJusta: `<span class="selo">sem comparação justa</span>`,
  };
  const provaHtml = pv
    ? `<div class="rotulo" style="margin-top:16px">Prova do Turbo · ${pv.rodadas.length} rodadas alternadas</div>
       <div class="grande" style="font-size:40px">${lado(pv.fps_sem)} → ${lado(pv.fps_com)}<small style="font-size:16px;color:var(--texto-2)"> fps no jogo</small></div>
       <div style="display:flex;gap:6px;flex-wrap:wrap;margin-top:8px">${selosDaProva[pv.desfecho] ?? ""}</div>
       <p class="nota" style="margin-top:8px">${esc(pv.leitura)}</p>
       ${pv.desfecho !== "SemComparacaoJusta" ? `<div class="acoes" style="margin-top:10px"><button class="botao" id="tb-copiar">Copiar resultado</button></div>` : ""}`
    : "";
  // O antes × depois da última vez: o número que o cliente sente, com a ressalva de que não é prova A-B. Com prova,
  // ela fala por ele.
  const r = !p.ligado && !pv ? p.resultado : null;
  const resultado = r
    ? `<div class="rotulo" style="margin-top:16px">Da última vez com o Turbo</div>
       <div class="grande" style="font-size:44px">${numero(r.fps_antes)} → ${numero(r.fps_do_jogo)}<small style="font-size:16px;color:var(--texto-2)"> fps no jogo</small></div>
       <div style="display:flex;gap:6px;flex-wrap:wrap;margin-top:8px"><span class="selo ${r.ganho_pct > 0 ? "medido" : ""}">${r.ganho_pct > 0 ? "+" : ""}${numero(r.ganho_pct, 1)}%</span>${r.fps_na_tela ? `<span class="selo ${r.na_tela_medido ? "medido" : "estimado"}">${numero(r.fps_na_tela)} na tela</span>` : ""}${r.escala ? `<span class="selo">escala ${esc(NOMES_DA_ESCALA[r.escala])}</span>` : ""}</div>
       <p class="nota" style="margin-top:8px">Comparação simples: o antes são os 10 s da medida; o depois, a média de ${numero(r.minutos, 1)} min de jogo. Cenas diferentes, então não é prova A-B.</p>`
    : "";
  caixa.innerHTML = `<div class="titulo-sec"><div class="rotulo">Turbo</div>${p.provando ? `<span class="pilula"><span class="v-vivo"></span>provando</span>` : p.ligado ? `<span class="pilula"><span class="v-vivo"></span>ligado</span>` : ""}</div>
    ${provaHtml}${resultado}${corpo}<p class="estado" id="tb-estado" hidden></p>`;

  const agir = async (botao: HTMLButtonElement, acao: () => Promise<PainelTurbo>, medindo = false) => {
    $$<HTMLButtonElement>("#b-turbo button").forEach((b) => (b.disabled = true));
    if (medindo) botao.textContent = "Medindo 10 s… jogue normalmente";
    try {
      const novo = await acao();
      estado.gerador = await ponte.geradorEstado().catch(() => estado.gerador);
      await cartaoTurbo(novo);
      void painelFrameBoost();
    } catch (e) {
      // Redesenha primeiro (os botões voltam) e só então mostra o erro, senão o redesenho o apagaria.
      await cartaoTurbo();
      const s = $("#tb-estado");
      s.hidden = false;
      s.textContent = String(e);
    }
  };
  document.querySelector<HTMLButtonElement>("#tb-medir")?.addEventListener("click", (e) => void agir(e.currentTarget as HTMLButtonElement, () => ponte.turboLigar(false), true));
  document.querySelector<HTMLButtonElement>("#tb-lembrado")?.addEventListener("click", (e) => void agir(e.currentTarget as HTMLButtonElement, () => ponte.turboLigar(true)));
  document.querySelector<HTMLButtonElement>("#tb-esquecer")?.addEventListener("click", (e) => void agir(e.currentTarget as HTMLButtonElement, () => ponte.turboEsquecer()));
  document.querySelector<HTMLButtonElement>("#tb-desligar")?.addEventListener("click", (e) => void agir(e.currentTarget as HTMLButtonElement, () => ponte.turboDesligar()));
  document.querySelector<HTMLButtonElement>("#tb-provar")?.addEventListener("click", () => void provarTurbo(p));
  document.querySelector<HTMLButtonElement>("#tb-copiar")?.addEventListener("click", async (e) => {
    const botao = e.currentTarget as HTMLButtonElement;
    try {
      await navigator.clipboard.writeText(await ponte.turboCompartilhar());
      botao.textContent = "Copiado";
    } catch (erro) {
      const s = $("#tb-estado");
      s.hidden = false;
      s.textContent = String(erro);
    }
  });
}

/** A prova A-B do Turbo: o cartão mostra a rodada em que está, e o resultado no fim. */
async function provarTurbo(antes: PainelTurbo) {
  await cartaoTurbo({ ...antes, provando: true });
  const progresso = (texto: string) => {
    const el = document.querySelector<HTMLElement>("#tb-progresso");
    if (el) el.textContent = texto;
  };
  const parar = await ouvir<PassoDaProvaDoTurbo>("turbo:prova", (passo) =>
    progresso(
      passo.indice === 0
        ? `Aquecendo ${passo.segundos} s…`
        : `Rodada ${passo.indice} de ${passo.total} · ${passo.com_turbo ? "com o Turbo" : "sem o Turbo"} · ${passo.segundos} s`
    )
  ).catch(() => () => {});
  try {
    await ponte.turboProvar();
    estado.gerador = await ponte.geradorEstado().catch(() => estado.gerador);
    await cartaoTurbo();
    void painelFrameBoost();
  } catch (e) {
    await cartaoTurbo();
    const s = $("#tb-estado");
    s.hidden = false;
    s.textContent = String(e);
  } finally {
    parar();
  }
}

async function driver(fab: string) {
  const caixa = $("#b-drv");
  const linha = (titulo: string, texto: string, ligado: boolean, acao: string, desabilitada = false, extra = "") =>
    `<div class="linha"><div>${esc(titulo)}<p>${esc(texto)}${extra}</p></div><button class="chave" role="switch" aria-checked="${ligado}" aria-label="${esc(titulo)}" data-acao="${esc(acao)}" ${desabilitada ? "disabled" : ""}></button></div>`;

  try {
    if (fab === "Nvidia") {
      const p = await ponte.driverNvidia();
      $("#b-drv-nota").textContent = estado.hw?.placa.driver ? `driver ${estado.hw.placa.driver}` : "";
      caixa.innerHTML = p.estado !== "Disponivel" ? `<p class="nota">${esc(p.nota)}</p>` : p.ajustes.map((a) => linha(a.titulo, a.explicacao, a.aplicado, a.aplicado ? `desfazer:${a.historico}` : `nvidia:${a.id}`)).join("");
    } else if (fab === "Amd") {
      const p = await ponte.driverAmd();
      if (p.estado !== "Disponivel" || !p.ajustes) {
        caixa.innerHTML = `<p class="nota">${esc(p.nota)}</p>`;
      } else {
        const meu = (id: string) => p.aplicados.includes(id);
        const itens: string[] = [];
        if (p.ajustes.anti_lag !== null) {
          const porVoce = p.ajustes.anti_lag && !meu("amd-antilag");
          itens.push(linha("Radeon Anti-Lag", "Menos atraso quando a placa está no limite.", !!p.ajustes.anti_lag, meu("amd-antilag") ? "desfazer:amd-antilag" : "amd:amd-antilag", porVoce, porVoce ? " Ligado por você no Adrenalin." : ""));
        }
        if (p.ajustes.afmf !== null) {
          const porVoce = p.ajustes.afmf && !meu("amd-afmf");
          itens.push(linha("AMD Fluid Motion Frames", "Geração de quadros do driver. Pede tela cheia ou sem bordas.", !!p.ajustes.afmf, meu("amd-afmf") ? "desfazer:amd-afmf" : "amd:amd-afmf", porVoce, porVoce ? " Ligado por você no Adrenalin." : ""));
        }
        if (p.ajustes.rsr !== null && p.ajustes.rsr !== undefined) {
          const porVoce = p.ajustes.rsr && !meu("amd-rsr");
          itens.push(linha("Escala do driver (Radeon Super Resolution)", "Para jogos em tela cheia: escolha no jogo uma resolução menor que a do monitor e o driver amplia com nitidez. Em janela sem bordas, use a escala do Frame Boost.", !!p.ajustes.rsr, meu("amd-rsr") ? "desfazer:amd-rsr" : "amd:amd-rsr", porVoce, porVoce ? " Ligado por você no Adrenalin." : ""));
        }
        caixa.innerHTML = itens.join("") || `<p class="nota">Este driver não ofereceu ajustes ao Otimiza.</p>`;
      }
    } else if (fab === "Intel") {
      const p = await ponte.driverIntel();
      if (p.estado !== "Disponivel" || !p.ajustes) {
        caixa.innerHTML = `<p class="nota">${esc(p.nota)}</p>`;
      } else {
        const meu = p.aplicados.includes("intel-latencia");
        const ligado = (p.ajustes.baixa_latencia ?? 0) > 0;
        caixa.innerHTML = linha("Baixa latência", "Menos atraso entre o clique e a tela.", ligado, meu ? "desfazer:intel-latencia" : "intel:intel-latencia", ligado && !meu, ligado && !meu ? " Ligado por você no painel da Intel." : "");
      }
    } else {
      caixa.innerHTML = `<p class="nota">Não identifiquei a placa de vídeo. Os ajustes do Windows continuam valendo.</p>`;
    }
  } catch (e) {
    caixa.innerHTML = `<p class="nota">${esc(String(e))}</p>`;
  }

  $$<HTMLButtonElement>(".chave[data-acao]", caixa).forEach((b) =>
    b.addEventListener("click", async () => {
      const [tipo, id] = (b.dataset.acao ?? "").split(/:(.*)/s);
      b.disabled = true;
      try {
        if (tipo === "desfazer") await ponte.desfazer(id);
        else if (tipo === "nvidia") await ponte.ajusteNvidia(id);
        else if (tipo === "amd") await ponte.ajusteAmd(id);
        else if (tipo === "intel") await ponte.ajusteIntel(id);
      } catch (e) {
        $("#b-drv-nota").textContent = String(e);
      }
      await driver(fab);
    })
  );
}

// ---------------------------------------------------------------- frame boost

const CHAVE_MULT = "otimiza-fb-mult";

/** 0 = Auto: enche o monitor. É o padrão de quem nunca escolheu. */
function multiplicadorPedido(): number {
  try {
    const v = localStorage.getItem(CHAVE_MULT);
    return v === null ? 0 : Number(v) || 0;
  } catch {
    return 0;
  }
}

function gravarMultiplicador(m: number) {
  try {
    localStorage.setItem(CHAVE_MULT, String(m));
  } catch {
    // Vale nesta sessão.
  }
}

/** Passar do limite do monitor: os Hz não cortam o multiplicador (até 6×) e o Auto mira o dobro deles. */
function semLimite(): boolean {
  // Tirado da tela em 08/10/2026: acima dos Hz do monitor a imagem fica menos lisa e a placa não dá conta. O valor salvo
  // por versões anteriores é ignorado.
  return false;
}

/** Os multiplicadores oferecidos: os que cabem no monitor, ou de 2× a 6× passando do limite. */
function multiplicadoresOferecidos(cabem: number[]): number[] {
  return semLimite() ? [2, 3, 4, 5, 6] : cabem;
}

/** O pedido, se couber no monitor; senão o maior que cabe abaixo dele. 1 = só escala, e só vale com a escala ligada. */
function multiplicadorQueCabe(cabem: number[], comEscala = false): number {
  const pedido = multiplicadorPedido();
  if (pedido === 0 || cabem.includes(pedido) || (pedido === 1 && comEscala)) return pedido;
  return cabem.filter((m) => m <= pedido).pop() ?? cabem[0] ?? 2;
}

const CHAVE_ESCALA = "otimiza-fb-escala";

const NOMES_DA_ESCALA: Record<NivelDeEscala, string> = {
  UltraQualidade: "Ultra qualidade",
  Qualidade: "Qualidade",
  Equilibrado: "Equilibrado",
  Desempenho: "Desempenho",
};

/** A escala escolhida (FSR 1), ou nenhuma. Desligada é o padrão: ligar muda o tamanho da janela do jogo. */
function escalaPedida(): NivelDeEscala | null {
  try {
    const v = localStorage.getItem(CHAVE_ESCALA);
    return v !== null && v in NOMES_DA_ESCALA ? (v as NivelDeEscala) : null;
  } catch {
    return null;
  }
}

function gravarEscala(n: NivelDeEscala | null) {
  try {
    if (n) localStorage.setItem(CHAVE_ESCALA, n);
    else localStorage.removeItem(CHAVE_ESCALA);
  } catch {
    // Vale nesta sessão.
  }
}

let vigiaDoPainel = 0;

/** O Frame Boost no Boost. Ligado, o número é SÓ o FPS que chegou à tela, medido; antes de ligar, só faixa estimada. */
/** Uma linha por item do catálogo, com o estado lido neste PC e o botão que cabe: aplicar, desfazer ou nada. */
function linhaDeItem(i: ItemDoCatalogo | undefined, nome: string, efeito: string): string {
  if (!i) return "";
  const lado =
    i.state === "Applied" ? `<button class="botao" data-desfazer="${esc(i.id)}">Desfazer</button>`
    : i.state === "Available" ? `<button class="botao prim" data-aplicar="${esc(i.id)}">Aplicar</button>`
    : i.state === "AlreadyOptimal" ? `<span class="selo medido">já está assim</span>`
    : `<span class="selo">${i.state === "Unknown" ? "não deu para ler" : "não se aplica"}</span>`;
  return `<div class="linha"><div>${esc(nome)}<p>${esc(efeito)}</p></div>${lado}</div>`;
}

function ligarItens(caixa: HTMLElement, recarregar: () => Promise<void>) {
  const estadoEl = caixa.querySelector<HTMLElement>(".estado");
  const agir = async (b: HTMLButtonElement, acao: () => Promise<unknown>) => {
    b.disabled = true;
    try {
      await acao();
      await recarregar();
    } catch (erro) {
      b.disabled = false;
      if (estadoEl) {
        estadoEl.hidden = false;
        estadoEl.textContent = String(erro);
      }
    }
  };
  caixa.querySelectorAll<HTMLButtonElement>("[data-aplicar]").forEach((b) => b.addEventListener("click", () => agir(b, () => ponte.aplicarItem(b.dataset.aplicar!))));
  caixa.querySelectorAll<HTMLButtonElement>("[data-desfazer]").forEach((b) => b.addEventListener("click", () => agir(b, () => ponte.desfazer(b.dataset.desfazer!))));
}

function tamanho(bytes: number): string {
  if (bytes >= 1024 ** 3) return `${numero(bytes / 1024 ** 3, 1)} GB`;
  if (bytes >= 1024 ** 2) return `${numero(bytes / 1024 ** 2, 0)} MB`;
  return `${numero(bytes / 1024, 0)} KB`;
}

/** Remoção de lixo: o tamanho medido antes, e só temporário e relatório de erro com mais de 2 dias. Libera disco, não
 * dá FPS — e o cartão diz isso. */
async function cartaoLixo(ultimo?: string) {
  const caixa = document.querySelector<HTMLElement>("#b-lixo");
  if (!caixa) return;
  const itens = await ponte.limpezaMedir().catch(() => null);
  if (!itens) {
    caixa.innerHTML = `<div class="rotulo">Remoção de lixo</div><p class="nota" style="margin-top:8px">Não consegui medir as pastas temporárias.</p>`;
    return;
  }
  const total = itens.reduce((s, i) => s + i.bytes, 0);
  caixa.innerHTML = `<div class="titulo-sec"><div class="rotulo">Remoção de lixo</div><span class="selo medido">${tamanho(total)} para liberar</span></div>
    <p class="nota">Só o que o Windows e os programas jogam fora e esquecem, com mais de 2 dias. Nunca a Lixeira, Downloads ou arquivos seus. Libera disco; não muda o FPS.</p>
    ${itens.map((i) => `<div class="linha"><div>${esc(i.nome)} · ${tamanho(i.bytes)}<p>${esc(i.explica)}${i.precisa_admin ? " Precisa do Otimiza como administrador." : ""}</p></div>${i.bytes > 0 && !i.precisa_admin ? `<button class="botao" data-limpar="${esc(i.id)}">Limpar</button>` : `<span class="selo">${i.bytes > 0 ? "sem administrador" : "limpo"}</span>`}</div>`).join("")}
    <p class="estado ${ultimo ? "ok" : ""}" ${ultimo ? "" : "hidden"}>${ultimo ? esc(ultimo) : ""}</p>`;
  caixa.querySelectorAll<HTMLButtonElement>("[data-limpar]").forEach((b) =>
    b.addEventListener("click", async () => {
      b.disabled = true;
      b.textContent = "Limpando…";
      try {
        const r = await ponte.limpezaExecutar(b.dataset.limpar!);
        await cartaoLixo(`${tamanho(r.bytes_liberados)} liberados (${r.apagados} arquivos).${r.ficaram ? ` ${r.ficaram} em uso ficaram.` : ""}`);
      } catch (erro) {
        b.disabled = false;
        b.textContent = "Limpar";
        const s = caixa.querySelector<HTMLElement>(".estado");
        if (s) {
          s.hidden = false;
          s.textContent = String(erro);
        }
      }
    }),
  );
}

/** Mouse e teclado: a correção de precisão (sem aceleração e ponteiro 1:1) e o teclado respondendo na hora. Nada
 * aqui mira por ninguém: é o Windows parando de mexer no que a mão faz. */
async function cartaoEntrada() {
  const caixa = document.querySelector<HTMLElement>("#b-entrada");
  if (!caixa) return;
  const itens = await ponte.itens().catch(() => [] as ItemDoCatalogo[]);
  const de = (id: string) => itens.find((i) => i.id === id);
  caixa.innerHTML = `<div class="titulo-sec"><div class="rotulo">Mouse e teclado</div></div>
    <p class="nota">A correção de precisão: o mesmo movimento da mão leva a mira sempre à mesma distância.</p>
    ${linhaDeItem(de("mouse_precision_off"), "Sem aceleração do mouse", "O mesmo movimento da mão vai sempre à mesma distância, rápido ou devagar.")}
    ${linhaDeItem(de("mouse_pointer_1to1"), "Ponteiro 1:1", "Cada contagem do sensor vira um pixel: sem pular nem perder movimento.")}
    ${linhaDeItem(de("keyboard_fast_repeat"), "Teclado sem demora de repetição", "A tecla segurada repete na hora, sem a pausa de meio segundo.")}
    ${linhaDeItem(de("accessibility_keys_off"), "Teclas de acessibilidade sem querer", "Segurar o Shift no jogo não liga mais a Filtragem de Teclas, que atrasa o teclado.")}
    <p class="estado" hidden></p>`;
  ligarItens(caixa, cartaoEntrada);
}

/** Windows mais leve: os efeitos visuais e o que abre com o Windows. Não dá FPS (diz isso); tira peso da área de
 * trabalho, do alt-tab e da memória que sobra para o jogo. */
async function cartaoLeve() {
  const caixa = document.querySelector<HTMLElement>("#b-leve");
  if (!caixa) return;
  const [itens, inicio] = await Promise.all([
    ponte.itens().catch(() => [] as ItemDoCatalogo[]),
    ponte.inicializacao().catch(() => null),
  ]);
  const abrem = (inicio ?? []).filter((e) => e.classe !== "Essencial");
  caixa.innerHTML = `<div class="titulo-sec"><div class="rotulo">Windows mais leve</div></div>
    <p class="nota">Não muda o FPS: deixa a área de trabalho e o alt-tab leves, e sobra memória para o jogo.</p>
    ${linhaDeItem(itens.find((i) => i.id === "visual_effects_performance"), "Efeitos visuais no mínimo", "Sem animações e sombras: menus e alt-tab abrem na hora.")}
    <div class="rotulo" style="margin-top:14px">Abrem com o Windows</div>
    ${inicio === null ? `<p class="nota" style="margin-top:6px">Não consegui ler a inicialização.</p>`
      : abrem.length === 0 ? `<p class="nota" style="margin-top:6px">Nada além do essencial abre com o Windows.</p>`
      : abrem.slice(0, 8).map((e) => `<div class="linha"><div>${esc(e.name)}<p>${e.enabled ? "Abre com o Windows" : "Não abre mais com o Windows"}</p></div><button class="chave" role="switch" aria-checked="${e.enabled}" aria-label="Abrir ${esc(e.name)} com o Windows" data-hive="${esc(e.hive)}" data-nome="${esc(e.name)}"></button></div>`).join("")}
    <p class="estado" hidden></p>`;
  ligarItens(caixa, cartaoLeve);
  caixa.querySelectorAll<HTMLButtonElement>(".chave[data-nome]").forEach((b) =>
    b.addEventListener("click", async () => {
      b.disabled = true;
      try {
        await ponte.inicializacaoDefinir(b.dataset.hive!, b.dataset.nome!, b.getAttribute("aria-checked") !== "true");
        await cartaoLeve();
      } catch (erro) {
        b.disabled = false;
        const s = caixa.querySelector<HTMLElement>(".estado");
        if (s) {
          s.hidden = false;
          s.textContent = String(erro);
        }
      }
    }),
  );
}

/** Energia de jogo: o processador em desempenho total só com o jogo aberto, e de volta quando ele fecha. Mostra o número
 * lido no plano ativo (ex.: "33 → 0"), não uma promessa. */
async function cartaoEnergia(dado?: EnergiaDeJogo | null) {
  const caixa = document.querySelector<HTMLElement>("#b-energia");
  if (!caixa) return;
  const e = dado ?? (await ponte.energiaDeJogo().catch(() => null));
  if (!e) {
    caixa.innerHTML = `<div class="rotulo">Energia de jogo</div><p class="nota" style="margin-top:8px">Não consegui ler a energia do processador.</p>`;
    return;
  }
  const muda = e.o_que_muda
    ? `<p class="nota" style="margin-top:8px">${esc(e.o_que_muda)}</p>`
    : `<p class="nota" style="margin-top:8px">O processador já está em desempenho total: nada a mudar aqui.</p>`;
  const agindo = e.agindo_em ? ` <span class="selo medido">agindo em ${esc(e.agindo_em)}</span>` : "";
  caixa.innerHTML = `<div class="titulo-sec"><div class="rotulo">Energia de jogo</div>${agindo}</div>
    ${muda}
    ${e.impedimento ? `<p class="nota" style="margin-top:8px">${esc(e.impedimento)}</p>` : ""}
    <div class="acoes" style="margin-top:12px"><button class="botao ${e.ligado ? "" : "prim"}" id="energia-alternar" ${e.impedimento && !e.ligado ? "disabled" : ""}>${e.ligado ? "Desligar" : "Ligar"}</button></div>
    <p class="nota" style="margin-top:6px">Liga sozinho quando o jogo abre e devolve o de antes quando ele fecha.</p>
    <p class="estado" id="energia-estado" hidden></p>`;
  caixa.querySelector<HTMLButtonElement>("#energia-alternar")?.addEventListener("click", async (ev) => {
    const b = ev.currentTarget as HTMLButtonElement;
    b.disabled = true;
    try {
      await cartaoEnergia(await ponte.energiaDeJogoDefinir(!e.ligado));
    } catch (erro) {
      b.disabled = false;
      const s = caixa.querySelector<HTMLElement>("#energia-estado")!;
      s.hidden = false;
      s.textContent = String(erro);
    }
  });
}

/** Memória para o jogo: quem ocupa a RAM agora, medido, e fechar só o que a pessoa escolher. Num PC de 8 GB, o navegador e
 * o Discord abertos fazem o jogo buscar memória no disco, e é aí que ele trava no giro rápido de câmera. */
async function cartaoMemoria(dado?: MemoriaParaOJogo | null) {
  const caixa = document.querySelector<HTMLElement>("#b-mem");
  if (!caixa) return;
  const m = dado ?? (await ponte.memoriaParaOJogo().catch(() => null));
  if (!m) {
    caixa.innerHTML = `<div class="rotulo">Memória para o jogo</div><p class="nota" style="margin-top:8px">Não consegui ler a memória.</p>`;
    return;
  }
  const apertada = m.livre_gb < 2.5 && m.total_gb <= 16;
  const somaMb = m.pesados.reduce((s, p) => s + p.mb, 0);
  caixa.innerHTML = `<div class="titulo-sec"><div class="rotulo">Memória para o jogo</div><span class="selo medido">medido</span></div>
    <p class="nota" style="margin-top:8px"><b>${numero(m.livre_gb, 1)} GB livres</b> de ${numero(m.total_gb, 1)} GB.${apertada ? " Pouco para jogar: sem memória, o jogo busca no disco e trava quando carrega cenário novo (o tranco ao virar a câmera rápido)." : ""}</p>
    ${m.pesados.length ? `<p class="nota" style="margin-top:8px">Feche antes de jogar para liberar até <b>${numero(somaMb / 1024, 1)} GB</b>:</p>
    ${m.pesados.map((p) => `<div class="linha"><div>${esc(p.nome)} · ${numero(p.mb / 1024, 1)} GB${p.processos > 1 ? ` (${p.processos} processos)` : ""}${p.aviso ? `<p>${esc(p.aviso)}</p>` : ""}</div><button class="botao" data-fechar="${esc(p.nome)}">Fechar</button></div>`).join("")}` : `<p class="nota" style="margin-top:8px">Nada pesado aberto além do jogo.</p>`}
    <p class="estado" id="mem-estado" hidden></p>`;
  caixa.querySelectorAll<HTMLButtonElement>("[data-fechar]").forEach((b) =>
    b.addEventListener("click", async () => {
      b.disabled = true;
      b.textContent = "Fechando…";
      try {
        await cartaoMemoria(await ponte.memoriaFechar(b.dataset.fechar!));
      } catch (erro) {
        b.disabled = false;
        b.textContent = "Fechar";
        const s = caixa.querySelector<HTMLElement>("#mem-estado")!;
        s.hidden = false;
        s.textContent = String(erro);
      }
    })
  );
}

/** Jogo liso: mede o ritmo do jogo e trava o FPS num valor estável (o que o cliente SENTE, não o número). */
async function cartaoJogoLiso(plano?: PlanoJogoLiso | null) {
  const caixa = document.querySelector<HTMLElement>("#b-liso");
  if (!caixa) return;
  const p = plano ?? null;
  if (!p) {
    caixa.innerHTML = `<div class="rotulo">Jogo liso</div>
      <p class="nota" style="margin-top:8px">Trava o FPS do jogo num ritmo constante — é o que o olho sente como liso, mais que o número. Abra o jogo e meça.</p>
      <div class="acoes" style="margin-top:12px"><button class="botao prim" id="liso-medir">Medir o jogo</button></div>`;
    document.querySelector<HTMLButtonElement>("#liso-medir")?.addEventListener("click", async (e) => {
      const b = e.currentTarget as HTMLButtonElement;
      b.disabled = true;
      b.textContent = "Medindo 20 s…";
      try {
        await cartaoJogoLiso(await ponte.jogoLisoMedir());
      } catch (erro) {
        b.disabled = false;
        b.textContent = "Medir o jogo";
        caixa.querySelector(".nota")!.textContent = String(erro);
      }
    });
    return;
  }
  const temTeto = p.teto !== null;
  // A medição anterior, para o antes × agora (só neste PC; o número de verdade é o medido).
  let antes: { trancos: number; fps: number; quando: number } | null = null;
  try {
    antes = JSON.parse(localStorage.getItem("otimiza-liso-ultima") ?? "null");
    localStorage.setItem("otimiza-liso-ultima", JSON.stringify({ trancos: p.trancos_por_minuto, fps: p.fps_agora, quando: Date.now() }));
  } catch {
    antes = null;
  }
  const comparacao = antes
    ? `<p class="nota" style="margin-top:8px">Medição anterior: <b>${numero(antes.trancos)}×/min</b> a ${numero(antes.fps)} fps → agora <b>${numero(p.trancos_por_minuto)}×/min</b> a ${numero(p.fps_agora)} fps.</p>`
    : "";
  caixa.innerHTML = `<div class="rotulo">Jogo liso</div>
    <p class="nota" style="margin-top:8px">O jogo entrega <b>${numero(p.fps_agora)} fps</b> e trava <b>${numero(p.trancos_por_minuto)}×/min</b> · monitor de ${p.hz} Hz. <span class="selo medido">medido</span></p>${comparacao}
    <p class="nota" style="margin-top:8px">${esc(p.motivo)}</p>
    ${p.aviso_do_teto ? `<p class="estado aviso">${esc(p.aviso_do_teto)}</p>` : p.teto_no_driver ? `<p class="nota" style="margin-top:8px">Travado no driver em <b>${p.teto_no_driver} fps</b> neste jogo.</p>` : ""}
    ${p.tela ? `<p class="nota" style="margin-top:8px">Chegada à tela: <b>${p.tela.apresentacao === "Direta" ? "direto ao monitor" : p.tela.apresentacao === "Copiada" ? "copiado pelo Windows" : p.tela.apresentacao === "Composta" ? "composto pelo Windows" : "não identificado"}</b> <span class="selo medido">medido</span></p>${p.tela.aviso ? `<p class="estado aviso">${esc(p.tela.aviso)}</p>` : ""}` : ""}
    ${temTeto ? `<div class="acoes" style="margin-top:12px"><button class="botao prim" id="liso-aplicar">Travar em ${p.teto} fps</button></div>
    <p class="nota" style="margin-top:6px">Vale na próxima vez que você abrir o jogo. Desfaz em Ajustes → Desfazer tudo.</p>` : ""}`;
  document.querySelector<HTMLButtonElement>("#liso-aplicar")?.addEventListener("click", async (e) => {
    const b = e.currentTarget as HTMLButtonElement;
    b.disabled = true;
    b.textContent = "Aplicando…";
    try {
      await ponte.jogoLisoAplicar();
      b.textContent = `Travado em ${p.teto} fps — reabra o jogo`;
    } catch (erro) {
      b.disabled = false;
      b.textContent = `Travar em ${p.teto} fps`;
      caixa.querySelectorAll(".nota")[1]!.textContent = String(erro);
    }
  });
}

async function painelFrameBoost(dado?: PainelFrameBoost | null) {
  const caixa = document.querySelector<HTMLElement>("#b-fb");
  if (!caixa) return;
  const p = dado ?? (await ponte.frameBoost().catch(() => null));
  if (!p) {
    caixa.innerHTML = `<div class="rotulo">Frame Boost</div><p class="nota" style="margin-top:8px">Não consegui ler o Frame Boost.</p>`;
    return;
  }
  const a = p.ativo;
  const rec = p.recomendado;
  const doOtimiza = rec?.recurso === "FrameBoostOtimiza";
  // O que o Otimiza liga de verdade: o gerador dele, o AFMF e, com o interruptor experimental, o Smooth Motion.
  const experimental = rec?.recurso === "FrameBoostSmoothMotion" && (motivo(rec.suporte) ?? "").startsWith("Experimental");
  const liga = doOtimiza || rec?.recurso === "FrameBoostAfmf" || experimental;
  const livre = semLimite();
  const oferecidos = multiplicadoresOferecidos(p.multiplicadores);
  // A escala é do gerador do Otimiza: o do jogo e o do driver não ampliam por fora.
  const escala = doOtimiza ? escalaPedida() : null;
  const mult = multiplicadorQueCabe(oferecidos, escala !== null);
  const soEscala = mult === 1;
  const opcaoDaEscala = escala ? p.escalas?.find((o) => o.nivel === escala) : undefined;
  const dicaDaEscala = !escala
    ? "O jogo renderiza numa resolução menor e o Otimiza amplia para o monitor (FSR 1): é FPS real a mais, não só quadro gerado."
    : opcaoDaEscala && p.monitor
      ? `O Otimiza põe o jogo em ${opcaoDaEscala.largura}×${opcaoDaEscala.altura} e amplia para ${p.monitor[0]}×${p.monitor[1]}, com o cursor no lugar; ao desligar, a janela volta como estava. O jogo precisa estar em janela sem bordas.`
      : "O Otimiza põe o jogo numa janela menor e amplia para a tela inteira; ao desligar, a janela volta como estava. O jogo precisa estar em janela sem bordas.";
  // Auto: o nativo vezes até 6, nunca acima do monitor (a faixa sai da estimativa de 2×).
  const dois = p.estimativas.find((e) => e.multiplicador === 2);
  // Passando do limite: o teto do Auto é o dobro dos Hz, e o multiplicador fixo não tem teto.
  const teto = p.hz ? (livre ? p.hz * 2 : p.hz) : Infinity;
  const vezes = (m: number) =>
    dois ? { multiplicador: m, faixa: { ...dois.faixa, minimo: Math.min(teto, (dois.faixa.minimo / 2) * m), maximo: Math.min(teto, (dois.faixa.maximo / 2) * m) } } : undefined;
  // Só escala não tem estimativa: o ganho depende de quanto a placa limita o jogo, e aparece medido ao ligar.
  const est = soEscala ? undefined : mult === 0 ? (p.hz ? vezes(6) : undefined) : livre ? vezes(mult) : p.estimativas.find((e) => e.multiplicador === mult);
  // Medido: o que o PresentMon viu chegar à tela. Estimado: o que o gerador apresentou (sem PresentMon, sem administrador).
  const medido = a && (a.fps_na_tela.origem === "Medido" || a.fps_na_tela.origem === "Estimado") ? a.fps_na_tela.valor : null;
  const naTelaEstimado = a?.fps_na_tela.origem === "Estimado";

  // Ligado só com escala: não gera quadros, então não há o que a guarda comparar.
  const soEscalaLigada = !!a?.escala && !a.automatico && a.multiplicador <= 1;

  let numero_ = "—";
  let rotulo = "FPS na tela";
  let selos = "";
  let linha = "";
  if (a) {
    if (medido !== null && a.hz > 0 && medido > a.hz * 1.03) rotulo = "FPS";
    selos =
      medido === null
        ? `<span class="selo">${a.pelo_driver ? "falta medir" : "medindo"}</span>`
        : naTelaEstimado
          ? `<span class="selo estimado" title="Quadros apresentados pelo gerador. Abra o Otimiza como administrador para medir o que chega à tela.">estimado</span>`
          : `<span class="selo medido" title="Quadros que chegaram à tela, pelo PresentMon">medido na tela</span>`;
    if (a.hdr) selos += `<span class="selo">HDR</span>`;
    if (a.leve) selos += `<span class="selo" title="A placa estava sem folga, ou o jogo perdeu FPS: o gerador calcula o movimento numa resolução menor para não tirar quadros do jogo.">modo leve</span>`;
    if (a.escala) selos += `<span class="selo" title="O jogo renderiza menor e o Otimiza amplia para o monitor (FSR 1)">escala ${esc(NOMES_DA_ESCALA[a.escala])}</span>`;
    const naTela = a.escala && a.jogo_px && a.tela_px ? `jogo ${a.jogo_px[0]}×${a.jogo_px[1]} → tela ${a.tela_px[0]}×${a.tela_px[1]} · ` : "";
    const atraso =
      (a.atraso_ms.valor !== null ? ` · atraso +${numero(a.atraso_ms.valor, 1)} ms (estimado)` : "") +
      (a.placa_pct !== null && a.placa_pct !== undefined ? ` · usa ${numero(a.placa_pct, 1)}% da placa (medido)` : "");
    linha = a.pelo_driver
      ? a.nota ?? ""
      : a.situacao === "ProcurandoJanela"
        ? `Ajustando à janela do jogo…${a.refeitas ? ` Ele se refaz sozinho quando a janela muda (${a.refeitas}×).` : ""}`
        : a.situacao === "SemQuadros"
          ? "O jogo parou de desenhar: a sobreposição some até ele voltar."
          : `${naTela}${soEscalaLigada ?"só escala, sem gerar quadros" : a.automatico ? `Auto (agora ${a.multiplicador}×)` : `${a.multiplicador}×`} · monitor de ${a.hz} Hz${a.sem_limite ? ` · passando do limite: o monitor mostra até ${a.hz} por segundo` : ""}${atraso}${!a.sem_limite && !a.automatico && a.multiplicador < multiplicadorPedido() ? `. ${multiplicadorPedido()}× passaria do monitor agora: o gerador nunca mostra mais quadros que os ${a.hz} Hz.` : ""}`;
  } else if (doOtimiza && soEscala) {
    linha = "Só escala: o jogo renderiza menor e o Otimiza amplia. O FPS a mais é do próprio jogo, sem o atraso da geração. O número aparece medido ao ligar.";
  } else if (est) {
    numero_ = `${numero(est.faixa.minimo)}–${numero(est.faixa.maximo)}`;
    rotulo = livre ? (mult === 0 ? "FPS no Auto" : `FPS com ${mult}×`) : mult === 0 ? "FPS na tela no Auto" : `FPS na tela com ${mult}×`;
    selos = `<span class="selo estimado">estimado</span>`;
    linha = "Pelo último FPS nativo medido deste jogo. O número de verdade aparece medido quando ligar.";
  } else if (rec && doOtimiza) {
    linha = "Jogue um minuto para o Otimiza medir o FPS nativo e estimar o ganho. Dá para ligar mesmo assim.";
  }
  // A prova do custo: o FPS do próprio jogo com o gerador ligado e quanto ele manteve (a guarda compara com e sem).
  const prova =
    a && !a.pelo_driver && a.situacao === "Gerando" && a.fps_do_jogo !== null
      ? soEscalaLigada
        ? `Jogo a ${numero(a.fps_do_jogo)} fps`
        : `Jogo a ${numero(a.fps_do_jogo)} fps · ${
          a.manteve_pct !== null
            ? `manteve ${numero(a.manteve_pct, 1)}% do FPS com o Frame Boost (medido em ${a.comparacoes} ${a.comparacoes === 1 ? "comparação" : "comparações"})`
            : "conferindo se o jogo perde FPS com o Frame Boost…"
        }`
      : "";
  // Desligado: como foi a última vez (só com 30 s ou mais gerando). Sem estimativa para mostrar, ela vira o número grande.
  const u = !a ? p.ultima_sessao : null;
  const ultimaNoTopo = !!u && !!rec && !est && u.fps_na_tela.valor !== null;
  if (u && ultimaNoTopo) {
    numero_ = numero(u.fps_na_tela.valor ?? 0);
    rotulo = "FPS na tela da última vez";
    selos = u.fps_na_tela.origem === "Medido" ? `<span class="selo medido">medido na tela</span>` : `<span class="selo estimado">estimado</span>`;
    linha = `${numero(u.minutos, 1)} min com o jogo a ${numero(u.fps_do_jogo)} fps${u.manteve_pct !== null ? ` · o jogo manteve ${numero(u.manteve_pct, 1)}% do FPS com o Frame Boost` : ""}${u.desligou_por_perda ? " · desligou sozinho: o jogo perdia FPS" : ""}.`;
  }
  const ultima = u && !ultimaNoTopo
    ? `<div class="linha" style="margin-top:12px"><div>Da última vez · ${numero(u.minutos, 1)} min<p>Jogo a ${numero(u.fps_do_jogo)} fps → ${u.fps_na_tela.valor !== null ? numero(u.fps_na_tela.valor) : "—"} na tela${u.manteve_pct !== null ? ` · o jogo manteve ${numero(u.manteve_pct, 1)}% do FPS` : ""}${u.desligou_por_perda ? " · desligou sozinho: o jogo perdia FPS" : ""}</p></div><span class="selo ${u.fps_na_tela.origem === "Medido" ? "medido" : "estimado"}">${u.fps_na_tela.origem === "Medido" ? "medido" : "estimado"}</span></div>`
    : "";

  const avisos: string[] = [];
  if (p.aviso) avisos.push(p.aviso);
  if (a?.integrada) avisos.push("Este notebook liga o monitor na placa integrada, e o gerador roda nela. Se o jogo perder FPS, ele desliga sozinho.");
  if (p.hz !== null && p.hz <= 75 && doOtimiza) avisos.push(`Monitor de ${p.hz} Hz: o Frame Boost nunca mostra mais quadros que o monitor. Se ele aceita mais Hz, ajuste nas configurações de vídeo do Windows primeiro.`);

  caixa.innerHTML = `
    <div class="titulo-sec"><div class="rotulo">Frame Boost · automático</div>${a ? `<span class="pilula"><span class="v-vivo"></span>ligado</span>` : ""}</div>
    <h2 style="margin-top:8px">${rec ? esc(rec.nome) : p.jogo ? "Nenhum gerador funciona aqui" : "Abra um jogo"}</h2>
    <p class="nota" style="margin-top:4px">${
      !p.jogo
        ? "Com o jogo aberto, o Otimiza escolhe o melhor gerador: o do jogo, depois o do driver, depois o dele."
        : rec
          ? esc(motivo(rec.suporte) ?? (doOtimiza ? "Funciona por fora do jogo, sem injetar nada. Acrescenta um pouco de atraso." : "Funciona neste jogo e nesta placa."))
          : "Nem o jogo, nem o driver, nem o gerador do Otimiza servem neste jogo com esta placa."
    }</p>
    ${rec ? `<div class="rotulo" style="margin-top:16px">${esc(rotulo)}</div>
    <div class="grande" style="font-size:64px"><span id="fb-num">${a ? (medido !== null ? "" : "—") : numero_}</span><small style="font-size:18px;color:var(--texto-2)">FPS</small></div>
    <div style="display:flex;gap:6px;flex-wrap:wrap;margin-top:8px">${selos}</div>
    ${linha ? `<p class="nota" style="margin-top:8px">${esc(linha)}</p>` : ""}
    ${prova ? `<p class="nota" style="margin-top:4px">${esc(prova)}</p>` : ""}` : ""}
    ${doOtimiza && !a ? `<div class="rotulo" style="margin-top:16px">Escala</div><div class="seg" id="fb-escala" style="margin-top:8px"><button aria-pressed="${escala === null}" data-e="">Desligada</button>${(Object.keys(NOMES_DA_ESCALA) as NivelDeEscala[]).map((n) => `<button aria-pressed="${escala === n}" data-e="${n}">${NOMES_DA_ESCALA[n]}</button>`).join("")}</div><p class="nota" style="margin-top:6px">${esc(dicaDaEscala)}</p>` : ""}
    ${doOtimiza && !a ? `<div class="seg" id="fb-mult" style="margin-top:14px">${escala ? `<button aria-pressed="${mult === 1}" data-m="1" title="Sem gerar quadros: só o FPS real a mais, sem atraso">Só escala</button>` : ""}<button aria-pressed="${mult === 0}" data-m="0" title="Enche o monitor: quantos quadros couberem">Auto</button>${oferecidos.map((m) => `<button aria-pressed="${m === mult}" data-m="${m}">${m}×</button>`).join("")}</div>${mult === 0 && !livre ? `<p class="nota" style="margin-top:6px">Auto enche o monitor: com o jogo a 60 fps num de 180 Hz, a tela recebe 180. Mais liso, mas segura o quadro real por cerca de um quadro do jogo (o 2× segura metade): para jogo competitivo, o 2× tem menos atraso.</p>` : ""}` : ""}
    ${rec ? `<div class="acoes" style="margin-top:14px"><button class="botao ${a ? "" : "prim"}" id="fb-ligar">${a ? "Desligar Frame Boost" : experimental ? "Ligar (experimental)" : liga ? "Ligar Frame Boost" : "Como ligar"}</button>${a?.pelo_driver ? `<button class="botao" id="fb-medir">Medir agora</button>` : ""}${a?.escala && !a.pelo_driver ? `<button class="botao" id="fb-comparar">Comparar a imagem</button>` : ""}${doOtimiza ? `<span class="nota" style="align-self:center">Ctrl+Alt+G desliga de dentro do jogo</span>` : ""}</div>` : ""}
    ${doOtimiza && (p.base || p.nota_da_base) ? `<div class="linha" style="margin-top:12px"><div>FPS base${p.base ? ` <b>${p.base.fps}</b> · ${p.base.multiplicador}× fecham o monitor <span class="selo ${p.base.valendo ? "medido" : ""}">${p.base.valendo ? "valendo" : "vale ao reabrir o jogo"}</span>` : ""}<p>${esc(p.nota_da_base ?? "Base travada numa fração do monitor: cada quadro real ganha sempre o mesmo número de gerados.")}</p></div>${p.base ? `<button class="botao" id="fb-destravar">Destravar</button>` : ""}</div>` : ""}
    ${doOtimiza && p.base_solta && !p.base ? `<div class="linha" style="margin-top:12px"><div>FPS base solto<p>Você destravou: o Frame Boost não trava a base deste jogo sozinho.${a && !a.pelo_driver ? "" : " Para travar de novo, ligue o Frame Boost."}</p></div>${a && !a.pelo_driver ? `<button class="botao" id="fb-travar">Travar de novo</button>` : ""}</div>` : ""}
    ${ultima}
    ${avisos.map((t) => `<p class="estado aviso">${esc(t)}</p>`).join("")}
    <p class="estado" id="fb-estado" hidden></p>
    ${p.opcoes.length ? `<div style="margin-top:14px">${p.opcoes.map((r) => `<div class="linha"><div>${esc(r.nome)}${motivo(r.suporte) ? `<p>${esc(motivo(r.suporte))}</p>` : ""}</div>${selo(r.suporte)}</div>`).join("")}</div>` : ""}`;

  if (medido !== null) {
    const el = $("#fb-num");
    el.dataset.valor = caixa.dataset.ultimo ?? String(medido);
    contar(el, medido, 900);
    caixa.dataset.ultimo = String(medido);
  } else {
    delete caixa.dataset.ultimo;
  }

  $$<HTMLButtonElement>("#fb-mult button").forEach((b) =>
    b.addEventListener("click", () => {
      gravarMultiplicador(Number(b.dataset.m));
      void painelFrameBoost(p);
    })
  );
  $$<HTMLButtonElement>("#fb-escala button").forEach((b) =>
    b.addEventListener("click", () => {
      gravarEscala(b.dataset.e ? (b.dataset.e as NivelDeEscala) : null);
      void painelFrameBoost(p);
    })
  );
  document.querySelector<HTMLButtonElement>("#fb-ligar")?.addEventListener("click", async (e) => {
    const botao = e.currentTarget as HTMLButtonElement;
    botao.disabled = true;
    try {
      const novo = a ? await ponte.frameBoostDesligar() : await ponte.frameBoostLigar(mult, livre, escala);
      estado.gerador = await ponte.geradorEstado().catch(() => estado.gerador);
      await painelFrameBoost(novo);
    } catch (erro) {
      botao.disabled = false;
      const s = $("#fb-estado");
      s.hidden = false;
      s.textContent = String(erro);
    }
  });
  document.querySelector<HTMLButtonElement>("#fb-destravar")?.addEventListener("click", async (e) => {
    const botao = e.currentTarget as HTMLButtonElement;
    botao.disabled = true;
    try {
      await painelFrameBoost(await ponte.frameBoostDestravarBase());
    } catch (erro) {
      botao.disabled = false;
      const s = $("#fb-estado");
      s.hidden = false;
      s.textContent = String(erro);
    }
  });
  document.querySelector<HTMLButtonElement>("#fb-travar")?.addEventListener("click", async (e) => {
    const botao = e.currentTarget as HTMLButtonElement;
    botao.disabled = true;
    try {
      await painelFrameBoost(await ponte.frameBoostTravarBase());
    } catch (erro) {
      botao.disabled = false;
      const s = $("#fb-estado");
      s.hidden = false;
      s.textContent = String(erro);
    }
  });
  document.querySelector<HTMLButtonElement>("#fb-medir")?.addEventListener("click", async (e) => {
    const botao = e.currentTarget as HTMLButtonElement;
    botao.disabled = true;
    botao.textContent = "Medindo 6 s…";
    try {
      await painelFrameBoost(await ponte.frameBoostMedir());
    } catch (erro) {
      botao.disabled = false;
      botao.textContent = "Medir agora";
      const s = $("#fb-estado");
      s.hidden = false;
      s.textContent = String(erro);
    }
  });
  document.querySelector<HTMLButtonElement>("#fb-comparar")?.addEventListener("click", () => void comparadorDaEscala());
  // O número do gerador do Otimiza muda a cada segundo; o do driver só quando alguém mede.
  vigiarPainel(a !== null && !a.pelo_driver);
}

let vigiaDoComparador = 0;

/** O mesmo pedaço do jogo, ampliado do jogo comum (filtro do navegador, como um monitor faria) e pelo FSR do Otimiza,
 *  com um divisor arrastável. Fora do `#b-fb`: o painel do Frame Boost se redesenha a cada segundo. */
async function comparadorDaEscala() {
  const caixa = document.querySelector<HTMLElement>("#b-comparar");
  if (!caixa) return;
  clearInterval(vigiaDoComparador);
  const moldura = (corpo: string) =>
    `<div class="vidro" style="margin-top:16px"><div class="titulo-sec"><div class="rotulo">Comparador da escala</div><button class="botao" id="cp-fechar">Fechar</button></div>${corpo}</div>`;
  const fechar = () =>
    document.querySelector<HTMLButtonElement>("#cp-fechar")?.addEventListener("click", () => {
      clearInterval(vigiaDoComparador);
      caixa.innerHTML = "";
    });
  try {
    await ponte.escalaPedirComparacao();
  } catch (e) {
    caixa.innerHTML = moldura(`<p class="nota" style="margin-top:8px">${esc(String(e))}</p>`);
    fechar();
    return;
  }
  caixa.innerHTML = moldura(`<p class="nota" style="margin-top:8px">Volte ao jogo por um instante: a comparação é tirada no próximo quadro dele, com ele na frente. Depois volte aqui.</p>`);
  fechar();
  caixa.scrollIntoView({ behavior: "smooth", block: "center" });
  let tentativas = 0;
  vigiaDoComparador = window.setInterval(async () => {
    tentativas += 1;
    const c = await ponte.escalaComparacao().catch(() => null);
    if (!c) {
      if (tentativas >= 120) {
        clearInterval(vigiaDoComparador);
        caixa.innerHTML = moldura(`<p class="nota" style="margin-top:8px">O jogo não voltou para a frente em 2 minutos. Toque em Comparar a imagem de novo.</p>`);
        fechar();
      }
      return;
    }
    clearInterval(vigiaDoComparador);
    caixa.innerHTML = moldura(`
      <div style="position:relative;margin-top:12px;aspect-ratio:${c.tela_px[0]}/${c.tela_px[1]};overflow:hidden;border-radius:12px">
        <img src="${c.jogo}" alt="O jogo ampliado do jeito comum" style="position:absolute;inset:0;width:100%;height:100%">
        <img src="${c.tela}" alt="O mesmo pedaço ampliado pelo FSR do Otimiza" id="cp-tela" style="position:absolute;inset:0;width:100%;height:100%;clip-path:inset(0 0 0 50%)">
        <div id="cp-linha" style="position:absolute;top:0;bottom:0;left:50%;width:2px;background:#fff;box-shadow:0 0 6px rgba(0,0,0,.6)"></div>
      </div>
      <input type="range" id="cp-divisor" min="0" max="100" value="50" aria-label="Divisor entre as duas imagens" style="width:100%;margin-top:10px">
      <p class="nota" style="margin-top:6px">O centro da tela, de perto. À esquerda, o jogo em ${c.jogo_px[0]}×${c.jogo_px[1]} ampliado do jeito comum, como um monitor faria. À direita, o mesmo pedaço ampliado pelo FSR do Otimiza para ${c.tela_px[0]}×${c.tela_px[1]}.</p>`);
    fechar();
    const divisor = $<HTMLInputElement>("#cp-divisor");
    divisor.addEventListener("input", () => {
      const v = Number(divisor.value);
      $("#cp-tela").style.clipPath = `inset(0 0 0 ${v}%)`;
      $("#cp-linha").style.left = `${v}%`;
    });
  }, 1000);
}

/** Enquanto o Frame Boost está ligado e o Boost aberto, o número se atualiza a cada segundo. */
function vigiarPainel(ligado = true) {
  clearInterval(vigiaDoPainel);
  if (!ligado) return;
  vigiaDoPainel = window.setInterval(async () => {
    if (estado.destino !== "boost") {
      clearInterval(vigiaDoPainel);
      return;
    }
    const p = await ponte.frameBoost().catch(() => null);
    estado.gerador = await ponte.geradorEstado().catch(() => estado.gerador);
    await painelFrameBoost(p);
  }, 1000);
}

// ---------------------------------------------------------------- mira

/** FOV de primeira pessoa do FiveM: a mesma opção do menu do jogo, gravada no arquivo dele com o jogo fechado. */
async function cartaoFov(dado?: FovFivem | null) {
  const caixa = document.querySelector<HTMLElement>("#m-fov");
  if (!caixa) return;
  const f = dado ?? (await ponte.fovFivem().catch(() => null));
  if (!f) {
    caixa.innerHTML = `<div class="titulo-sec"><h2>Campo de visão · FiveM</h2></div><p class="nota">Não consegui ler a configuração do FiveM.</p>`;
    return;
  }
  const valor = f.pedido ?? f.atual ?? 5;
  const situacao = f.pendente
    ? `Grava ${f.pedido === null ? "a volta do de antes" : `a posição ${f.pedido}`} quando o FiveM fechar: com ele aberto, o jogo regrava o arquivo e a troca se perderia.`
    : f.atual === null
      ? "O FiveM ainda não gravou essa opção neste PC: abra o jogo uma vez."
      : `No arquivo do FiveM agora: posição <b>${f.atual}</b> de ${f.maximo}.`;
  caixa.innerHTML = `<div class="titulo-sec"><h2>Campo de visão · FiveM</h2><span class="nota">primeira pessoa</span></div>
    <p class="nota">A mesma opção do menu do jogo (Configurações → Câmera), sem abrir o jogo. Mais aberto mostra mais da cena; servidor que fixa o FOV vence.</p>
    <div class="controle" style="margin-top:10px"><div class="topo-c">Posição no menu<b id="m-fov-v">${valor}</b></div><input type="range" id="m-fov-r" min="0" max="${f.maximo}" step="1" value="${valor}"></div>
    <p class="nota" style="margin-top:8px">${situacao}</p>
    <div class="acoes" style="margin-top:10px"><button class="botao prim" id="m-fov-gravar">Gravar no FiveM</button>${f.alterado ? `<button class="botao" id="m-fov-devolver">Devolver o de antes</button>` : ""}</div>
    <p class="estado" id="m-fov-estado" hidden></p>`;
  const r = $<HTMLInputElement>("#m-fov-r");
  const pintar = () => r.style.setProperty("--p", `${(Number(r.value) / f.maximo) * 100}%`);
  pintar();
  r.addEventListener("input", () => { $("#m-fov-v").textContent = r.value; pintar(); });
  const agir = async (b: HTMLButtonElement, fazer: () => Promise<FovFivem>) => {
    b.disabled = true;
    try {
      await cartaoFov(await fazer());
    } catch (e) {
      b.disabled = false;
      const s = $("#m-fov-estado");
      s.hidden = false;
      s.className = "estado aviso";
      s.textContent = String(e);
    }
  };
  $<HTMLButtonElement>("#m-fov-gravar").addEventListener("click", (e) => void agir(e.currentTarget as HTMLButtonElement, () => ponte.fovFivemDefinir(Number(r.value))));
  document.querySelector<HTMLButtonElement>("#m-fov-devolver")?.addEventListener("click", (e) => void agir(e.currentTarget as HTMLButtonElement, () => ponte.fovFivemDevolver()));
}

async function telaMira() {
  const t = $("#t-mira");
  const m = lerMira();
  const j = estado.jogo;
  const recurso = (j?.recursos ?? estado.compat?.recursos ?? []).find((r) => r.recurso === "MiraNaTela");

  t.innerHTML = `
    <div class="cabeca"><div><h1><span class="barras"></span>Mira</h1><p>Mira na tela, sensibilidade e treino. Nada aqui mexe no jogo nem mira por você.</p></div>
      ${recurso ? `<span class="pilula">${esc(j?.nome ?? "Sem jogo aberto")} · ${selo(recurso.suporte)}</span>` : ""}</div>
    <div class="vidro linha-aviso"><div><b>Correção de precisão do mouse</b><p class="nota">Sem aceleração e ponteiro 1:1: o mesmo movimento da mão leva a mira sempre ao mesmo lugar.</p></div><button class="botao" id="m-precisao">Abrir</button></div>
    <div class="grade g-2">
      <div class="vidro" style="padding:14px">
        <div class="previa"><img id="m-fundo" alt=""><svg class="mira" id="m-svg" width="120" height="120" viewBox="-60 -60 120 120"></svg></div>
        <p class="nota" style="margin:12px 4px 2px">${esc(recurso ? motivo(recurso.suporte) ?? "A mira fica numa janela por cima do jogo, sem tocar nele." : "A mira fica numa janela por cima do jogo, sem tocar nele.")}</p>
        <div class="acoes" style="margin-top:12px"><button class="botao prim" id="m-ligar">Mostrar no jogo</button><button class="botao" id="m-cs2">Copiar para o console do CS2</button></div>
        <div class="estado" id="m-estado" hidden></div>
        <p class="nota" style="margin-top:10px">Aparece com o jogo em janela ou sem bordas. Em tela cheia exclusiva o Windows não deixa nada por cima.</p>
      </div>
      <div class="vidro">
        <div class="titulo-sec"><h2>Estilo</h2></div>
        <div class="estilos" id="m-estilos">${ESTILOS.map(([id, nome]) => `<button aria-pressed="${id === m.estilo}" data-e="${id}" aria-label="${nome}" title="${nome}"><svg viewBox="-14 -14 28 28">${desenharMira({ ...m, estilo: id, tamanho: 6, espessura: 1.6, abertura: 2.5, contorno: false, opacidade: 1, cor: "#fff" })}</svg></button>`).join("")}</div>
        <div class="titulo-sec" style="margin-top:14px"><span class="rotulo">Cor</span></div>
        <div class="cores-mira" id="m-cores">${CORES.map((c) => `<button aria-pressed="${c === m.cor}" data-c="${c}" style="background:${c}" aria-label="Cor ${c}"></button>`).join("")}</div>
        ${[["tamanho", "Tamanho", 2, 30, 1], ["espessura", "Espessura", 1, 6, 1], ["abertura", "Abertura do centro", 0, 14, 1], ["opacidade", "Opacidade", 0.2, 1, 0.05]]
          .map(([k, n, a, b, p]) => `<div class="controle"><div class="topo-c">${n}<b id="m-v-${k}"></b></div><input type="range" id="m-${k}" min="${a}" max="${b}" step="${p}" value="${m[k as keyof Mira]}"></div>`)
          .join("")}
        <div class="linha">Contorno preto<button class="chave" role="switch" aria-checked="${m.contorno}" id="m-contorno" aria-label="Contorno"></button></div>
        <div class="linha">Esconder ao mirar com o botão direito<button class="chave" role="switch" aria-checked="${!!m.esconder_ao_mirar}" id="m-esconder" aria-label="Esconder ao mirar"></button></div>
      </div>
    </div>
    <div class="grade g-2">
      <div class="vidro"><div class="titulo-sec"><h2>Sensibilidade</h2></div>
        <div class="linha">DPI do mouse<input class="campo" id="m-dpi" type="number" min="100" max="32000" value="800"></div>
        <div class="linha">Sensibilidade no CS2<input class="campo" id="m-sens" type="number" min="0.05" step="0.05" value="1.2"></div>
        <div class="linha">Volta completa<b class="num" id="m-cm">—</b></div>
        <div class="linha">A mesma mira no Valorant<b class="num" id="m-val">—</b></div>
        <p class="nota" style="margin-top:8px">CS2, Apex e Valorant giram 0,022° por contagem do mouse. Mesma volta em centímetros, mesma memória muscular.</p>
      </div>
      <div class="vidro"><div class="titulo-sec"><h2>Treino de tracking</h2><span class="nota" id="m-rec"></span></div>
        <div class="treino"><canvas id="m-treino"></canvas><div class="sobre-treino" id="m-treino-sobre"><button class="botao prim" id="m-treino-ir">Começar · 15 s</button></div></div>
        <p class="nota" style="margin-top:10px" id="m-treino-res">Mantenha o cursor sobre o alvo. Mede quanto tempo você ficou nele.</p>
      </div>
    </div>
    <div class="vidro" id="m-fov"></div>`;

  $("#m-precisao").addEventListener("click", () => {
    ir("boost");
    setTimeout(() => document.querySelector("#b-entrada")?.scrollIntoView({ behavior: "smooth", block: "center" }), 300);
  });
  const fundo = $<HTMLImageElement>("#m-fundo");
  fundo.src = j ? await capa(idDoJogo(j), j.nome, j.perfil.steam_appid) : await capa("fivem", "FiveM", null);

  const atual = { ...m };
  const pintar = (r: HTMLInputElement) => r.style.setProperty("--p", `${((Number(r.value) - Number(r.min)) / (Number(r.max) - Number(r.min))) * 100}%`);
  const atualizar = () => {
    $("#m-svg").innerHTML = desenharMira(atual, 1.6);
    $("#m-v-tamanho").textContent = String(atual.tamanho);
    $("#m-v-espessura").textContent = String(atual.espessura);
    $("#m-v-abertura").textContent = String(atual.abertura);
    $("#m-v-opacidade").textContent = `${Math.round(atual.opacidade * 100)}%`;
    gravarMira(atual);
    clearTimeout(enviarMira);
    enviarMira = window.setTimeout(() => void ponte.miraAtualizar(atual).catch(() => {}), 120);
  };
  $$<HTMLButtonElement>("#m-estilos button").forEach((b) => b.addEventListener("click", () => { atual.estilo = b.dataset.e as Mira["estilo"]; $$("#m-estilos button").forEach((x) => x.setAttribute("aria-pressed", String(x === b))); atualizar(); }));
  $$<HTMLButtonElement>("#m-cores button").forEach((b) => b.addEventListener("click", () => { atual.cor = b.dataset.c!; $$("#m-cores button").forEach((x) => x.setAttribute("aria-pressed", String(x === b))); atualizar(); }));
  for (const k of ["tamanho", "espessura", "abertura", "opacidade"] as const) {
    const r = $<HTMLInputElement>(`#m-${k}`);
    pintar(r);
    r.addEventListener("input", () => { atual[k] = Number(r.value); pintar(r); atualizar(); });
  }
  $("#m-contorno").addEventListener("click", (e) => { atual.contorno = !atual.contorno; (e.currentTarget as HTMLElement).setAttribute("aria-checked", String(atual.contorno)); atualizar(); });
  $("#m-esconder").addEventListener("click", (e) => { atual.esconder_ao_mirar = !atual.esconder_ao_mirar; (e.currentTarget as HTMLElement).setAttribute("aria-checked", String(atual.esconder_ao_mirar)); atualizar(); });
  atualizar();

  const aviso = (texto: string, classe = "estado") => {
    const s = $("#m-estado");
    s.hidden = false;
    s.className = classe;
    s.innerHTML = texto;
  };
  const botao = $<HTMLButtonElement>("#m-ligar");
  const pintarBotao = (ligada: boolean) => {
    botao.textContent = ligada ? "Tirar do jogo" : "Mostrar no jogo";
    botao.classList.toggle("prim", !ligada);
  };
  pintarBotao((await ponte.miraEstado().catch(() => null))?.ligada ?? false);
  const ligar = async (confirmado: boolean) => {
    try {
      await ponte.miraLigar(atual, confirmado);
      pintarBotao(true);
      aviso(`Mira ligada no ${esc(estado.jogo?.nome ?? "jogo")}. Ela some quando o jogo sai da frente.`, "estado ok");
    } catch (e) {
      const texto = String(e);
      if (texto.startsWith("CONFIRMAR:")) {
        aviso(`${esc(texto.slice(10))} <button class="botao" id="m-confirmar" style="margin-left:8px">Ligar mesmo assim</button>`, "estado aviso");
        $("#m-confirmar").addEventListener("click", () => void ligar(true));
      } else {
        aviso(esc(texto), "estado aviso");
      }
    }
  };
  botao.addEventListener("click", async () => {
    if (botao.textContent === "Tirar do jogo") {
      await ponte.miraDesligar().catch(() => {});
      pintarBotao(false);
      $("#m-estado").hidden = true;
    } else {
      await ligar(false);
    }
  });
  $("#m-cs2").addEventListener("click", async () => {
    try {
      const comandos = await ponte.miraCs2(atual);
      await navigator.clipboard.writeText(comandos);
      aviso("Copiado. No CS2, abra o console do desenvolvedor e cole: a mira fica no próprio jogo, sem janela por cima.", "estado ok");
    } catch (e) {
      aviso(esc(String(e)), "estado aviso");
    }
  });

  await cartaoFov();

  const converter = () => {
    const dpi = Number($<HTMLInputElement>("#m-dpi").value);
    const s = Number($<HTMLInputElement>("#m-sens").value);
    const cm = cmPorVolta(dpi, s);
    $("#m-cm").textContent = cm ? `${numero(cm, 1)} cm` : "—";
    $("#m-val").textContent = numero(sensValorant(s), 3);
  };
  $("#m-dpi").addEventListener("input", converter);
  $("#m-sens").addEventListener("input", converter);
  converter();

  let recorde = Number(localStorage.getItem("otimiza-recorde-tracking") ?? 0);
  if (recorde) $("#m-rec").textContent = `recorde ${recorde}%`;
  $("#m-treino-ir").addEventListener("click", () => {
    $("#m-treino-sobre").hidden = true;
    treinar($<HTMLCanvasElement>("#m-treino"), 15, (pct) => {
      if (pct > recorde) {
        recorde = pct;
        try { localStorage.setItem("otimiza-recorde-tracking", String(pct)); } catch { /* sem armazenamento */ }
        $("#m-rec").textContent = `recorde ${pct}%`;
      }
      $("#m-treino-res").textContent = `No alvo ${pct}% do tempo. Treine com a sensibilidade que você usa no jogo.`;
      $("#m-treino-sobre").hidden = false;
      $("#m-treino-ir").textContent = "De novo";
    });
  });
}

// ---------------------------------------------------------------- cores

const CAMINHO: Record<CaminhoDasCores, string> = { Amd: "driver AMD", Nvidia: "driver NVIDIA", Gama: "Windows" };

async function telaCores() {
  const t = $("#t-cores");
  const recurso = (estado.jogo?.recursos ?? estado.compat?.recursos ?? []).find((r) => r.recurso === "CoresDoMonitor");
  const st = await ponte.coresEstado().catch(() => null);
  const nomeSat = st?.caminho === "Nvidia" ? "Vibração digital" : "Saturação";
  const jogo = estado.jogo;
  const id = jogo ? idDoJogo(jogo) : null;
  const salvo = id ? st?.perfis.por_jogo[id] : undefined;
  // Começa do que está salvo para o jogo, ou do valor real do driver: quem já usa 100% no painel da placa não é
  // jogado para 50 sem pedir.
  const a: Ajuste = salvo ? { ...salvo } : { ...NORMAL, saturacao: st?.saturacao_atual ?? NORMAL.saturacao };
  const igual = (x: Ajuste, y: Ajuste) => (Object.keys(x) as (keyof Ajuste)[]).every((k) => x[k] === y[k]);
  const vale = (k: keyof Ajuste) => st?.suporta[k] ?? false;
  let soComJogo = st?.perfis.so_com_jogo ?? false;
  let aoVivo = st?.aplicado ?? false;
  const controles: [keyof Ajuste, string, number, number][] = [
    ["saturacao", nomeSat, 0, 100],
    ["brilho", "Brilho", -40, 40],
    ["contraste", "Contraste", -40, 40],
    ["gama", "Gama", 70, 150],
    ["matiz", "Matiz", -30, 30],
  ];

  t.innerHTML = `
    <div class="cabeca"><div><h1><span class="barras"></span>Cores</h1><p>Saturação, brilho e contraste do monitor, por jogo. Com “só com o jogo aberto”, tudo volta ao normal quando o jogo fecha.</p></div>
      ${recurso ? `<span class="pilula">${esc(recurso.nome)} · ${selo(recurso.suporte)}</span>` : ""}</div>
    <div class="grade g-2">
      <div class="vidro" style="padding:14px">
        <div class="previa"><canvas id="c-cv" style="position:absolute;inset:0;width:100%;height:100%"></canvas></div>
        <input type="range" id="c-corte" min="0" max="100" value="50" aria-label="Comparar antes e depois" style="margin-top:10px">
        <p class="nota" style="margin:6px 4px 2px">À esquerda como está, à direita com o ajuste. Arraste para comparar.</p>
      </div>
      <div class="vidro">
        <div class="seg" id="c-pre">${Object.keys(PREDEFINICOES).map((k) => `<button aria-pressed="${igual(a, PREDEFINICOES[k])}" data-p="${k}">${k[0].toUpperCase() + k.slice(1)}</button>`).join("")}</div>
        ${controles
          .map(([k, n, mi, ma]) => `<div class="controle" ${vale(k) ? "" : `title="Pelo ${esc(CAMINHO[st?.caminho ?? "Gama"])} isso não muda" style="opacity:.4"`}><div class="topo-c">${n}<b id="c-v-${k}"></b></div><input type="range" id="c-${k}" min="${mi}" max="${ma}" value="${a[k]}" ${vale(k) ? "" : "disabled"}></div>`)
          .join("")}
        <div class="linha" style="margin-top:6px"><div>Só com o jogo aberto<p>${jogo ? `Liga quando o ${esc(jogo.nome)} abre e devolve quando ele fecha.` : "Abra um jogo para salvar o ajuste dele."}</p></div><button class="chave" role="switch" aria-checked="${soComJogo}" id="c-so" aria-label="Só com o jogo aberto" ${jogo ? "" : "disabled"}></button></div>
        <div class="acoes" style="margin-top:12px"><button class="botao prim" id="c-aplicar" ${st ? "" : "disabled"}>Aplicar no monitor</button><button class="botao" id="c-normal">Voltar ao normal</button></div>
        <p class="nota" style="margin-top:10px">${esc(st ? st.nota : "O Otimiza não conseguiu ler as cores deste monitor.")}</p>
        <p id="c-estado" hidden></p>
      </div>
    </div>`;

  const cv = $<HTMLCanvasElement>("#c-cv");
  const corte = $<HTMLInputElement>("#c-corte");
  const pintar = (r: HTMLInputElement) => r.style.setProperty("--p", `${((Number(r.value) - Number(r.min)) / (Number(r.max) - Number(r.min))) * 100}%`);
  const aviso = (texto: string, classe = "estado") => {
    const s = $("#c-estado");
    s.hidden = false;
    s.className = classe;
    s.innerHTML = texto;
  };
  const desenhar = () => {
    previa(cv, a, Number(corte.value) / 100);
    $("#c-v-saturacao").textContent = `${a.saturacao}%`;
    $("#c-v-brilho").textContent = `${a.brilho > 0 ? "+" : ""}${a.brilho}`;
    $("#c-v-contraste").textContent = `${a.contraste > 0 ? "+" : ""}${a.contraste}`;
    $("#c-v-gama").textContent = numero(a.gama / 100, 2);
    $("#c-v-matiz").textContent = `${a.matiz}°`;
  };
  const sincronizar = () => (Object.keys(a) as (keyof Ajuste)[]).forEach((k) => { const r = $<HTMLInputElement>(`#c-${k}`); r.value = String(a[k]); pintar(r); });
  const salvar = () => (id ? ponte.coresSalvarPerfil(id, a, soComJogo).then(() => {}).catch(() => {}) : Promise.resolve());

  const aplicar = async () => {
    try {
      const c = await ponte.coresAplicar(a);
      aoVivo = true;
      await salvar();
      aviso(`Aplicado pelo ${esc(CAMINHO[c])}.${soComJogo ? " Volta ao normal quando o jogo fechar." : " Fica até você voltar ao normal."}`, "estado ok");
    } catch (e) {
      aviso(esc(String(e)), "estado aviso");
    }
  };
  // Depois do primeiro "Aplicar", arrastar já muda o monitor, como no painel do driver.
  let espera = 0;
  const mudou = () => {
    desenhar();
    if (!aoVivo) return;
    clearTimeout(espera);
    espera = window.setTimeout(() => void aplicar(), 150);
  };

  for (const k of Object.keys(a) as (keyof Ajuste)[]) {
    const r = $<HTMLInputElement>(`#c-${k}`);
    pintar(r);
    r.addEventListener("input", () => { a[k] = Number(r.value); pintar(r); $$("#c-pre button").forEach((b) => b.setAttribute("aria-pressed", "false")); mudou(); });
  }
  corte.addEventListener("input", () => { pintar(corte); desenhar(); });
  pintar(corte);
  $$<HTMLButtonElement>("#c-pre button").forEach((b) => b.addEventListener("click", () => {
    Object.assign(a, PREDEFINICOES[b.dataset.p!]);
    // O que o caminho não muda fica no normal, para a prévia não prometer o que o monitor não vai mostrar.
    (Object.keys(a) as (keyof Ajuste)[]).forEach((k) => { if (!vale(k)) a[k] = NORMAL[k]; });
    $$("#c-pre button").forEach((x) => x.setAttribute("aria-pressed", String(x === b)));
    sincronizar();
    mudou();
  }));
  $("#c-aplicar").addEventListener("click", () => void aplicar());
  $("#c-normal").addEventListener("click", async () => {
    clearTimeout(espera);
    aoVivo = false;
    try {
      await ponte.coresRestaurar();
      const depois = await ponte.coresEstado().catch(() => null);
      Object.assign(a, NORMAL, { saturacao: depois?.saturacao_atual ?? NORMAL.saturacao });
      if (id) await ponte.coresSalvarPerfil(id, null, soComJogo).catch(() => {});
      $$("#c-pre button").forEach((x) => x.setAttribute("aria-pressed", String(igual(a, PREDEFINICOES[x.dataset.p!]))));
      sincronizar();
      desenhar();
      aviso("As cores voltaram ao que eram antes do Otimiza.", "estado ok");
    } catch (e) {
      aviso(esc(String(e)), "estado aviso");
    }
  });
  $("#c-so").addEventListener("click", async (e) => {
    soComJogo = !soComJogo;
    (e.currentTarget as HTMLElement).setAttribute("aria-checked", String(soComJogo));
    await salvar();
    aviso(soComJogo ? `Salvo para o ${esc(jogo?.nome ?? "jogo")}: liga sozinho quando ele abrir.` : "Desligado: o ajuste fica só quando você aplicar.", "estado ok");
  });
  desenhar();
}

// ---------------------------------------------------------------- lab

const CONFIANCA: Record<string, string> = { Alta: "confiança alta", Media: "confiança média", SemDiferenca: "sem diferença", Insuficiente: "faltam rodadas" };

async function telaLab(resultado?: ResultadoDoLab) {
  const t = $("#t-lab");
  const r = resultado ?? (await ponte.labEstado().catch(() => null));
  const j = estado.jogo;
  const barra = (n: number, min: number) => `<div class="rodadas">${Array.from({ length: Math.max(min, n) }, (_, i) => `<i class="${i < n ? "on" : ""}"></i>`).join("")}</div>`;

  t.innerHTML = `
    <div class="cabeca"><div><h1><span class="barras"></span>Lab</h1><p>Rodadas no mesmo trecho antes e depois de otimizar, medidas com o PresentMon. O resultado diz quanto dá para confiar.</p></div>
      <span class="pilula ${j ? "" : "parado"}"><span class="v-vivo"></span>${j ? esc(j.nome) : "Abra um jogo para testar"}</span></div>
    <div class="grade g-2">
      <div class="vidro">
        <div class="titulo-sec"><h2>${r?.nome ? esc(r.nome) : "Testar performance"}</h2><button class="botao" id="lab-zerar">Recomeçar</button></div>
        ${r && r.linhas.some((l) => l.antes !== null)
          ? `<div style="overflow-x:auto"><table class="tabela"><thead><tr><th>Métrica</th><th>Antes</th><th>Depois</th><th>Diferença</th></tr></thead><tbody>${r.linhas
              .map((l) => `<tr><td>${esc(l.metrica)}</td><td>${numero(l.antes, l.metrica.includes("tempo") ? 1 : 0)}</td><td>${numero(l.depois, l.metrica.includes("tempo") ? 1 : 0)}</td><td>${l.diferenca_pct !== null ? `${l.diferenca_pct > 0 ? "+" : ""}${numero(l.diferenca_pct)}%` : `<span class="nota">${CONFIANCA[l.confianca]}</span>`}</td></tr>`)
              .join("")}</tbody></table></div>
             ${r.descartadas ? `<p class="nota" style="margin-top:10px">${r.descartadas} rodada(s) com geração de quadros ficaram de fora: com gerador ligado o FPS do jogo cai e a comparação mentiria.</p>` : ""}`
          : `<div class="v-vazio">Entre na partida, fique num lugar que dá para repetir e meça três rodadas antes. Depois otimize e meça três rodadas no mesmo lugar.</div>`}
      </div>
      <div class="vidro">
        <div class="rotulo">Antes de otimizar</div>${barra(r?.rodadas_antes ?? 0, r?.minimo_por_lado ?? 3)}
        <button class="botao" id="lab-antes" ${j ? "" : "disabled"} style="width:100%">Medir rodada antes · 30 s</button>
        <div class="rotulo" style="margin-top:18px">Depois de otimizar</div>${barra(r?.rodadas_depois ?? 0, r?.minimo_por_lado ?? 3)}
        <button class="botao" id="lab-depois" ${j && (r?.rodadas_antes ?? 0) > 0 ? "" : "disabled"} style="width:100%">Medir rodada depois · 30 s</button>
        <div class="estado" id="lab-estado" hidden></div>
        <p class="nota" style="margin-top:12px">Três rodadas por lado é o mínimo para haver margem de erro. Com menos, o Lab mostra os números, mas não diz que houve ganho.</p>
      </div>
    </div>`;

  const medir = async (lado: "Antes" | "Depois") => {
    if (!estado.jogo) return;
    const s = $("#lab-estado");
    s.hidden = false;
    s.className = "estado";
    s.textContent = "Medindo 30 segundos… jogue normalmente.";
    $$<HTMLButtonElement>("#lab-antes, #lab-depois").forEach((b) => (b.disabled = true));
    try {
      await telaLab(await ponte.labMedir(estado.jogo.executavel, lado, 30));
    } catch (e) {
      s.className = "estado aviso";
      s.textContent = String(e);
      $$<HTMLButtonElement>("#lab-antes, #lab-depois").forEach((b) => (b.disabled = false));
    }
  };
  $("#lab-antes").addEventListener("click", () => void medir("Antes"));
  $("#lab-depois").addEventListener("click", () => void medir("Depois"));
  $("#lab-zerar").addEventListener("click", async () => {
    await ponte.labZerar();
    await telaLab();
  });
}

// ---------------------------------------------------------------- ajustes

const MODOS: [ModoDoOverlay, string][] = [["Desligado", "Desligado"], ["Minimo", "Mínimo"], ["Detalhado", "Detalhado"]];
const CANTOS: [CantoDoOverlay, string][] = [["SuperiorEsquerdo", "↖"], ["SuperiorDireito", "↗"], ["InferiorEsquerdo", "↙"], ["InferiorDireito", "↘"]];

/** O FPS por cima do jogo, em Ajustes: modo, canto e "mostrar agora", com a mesma confirmação da Mira. */
async function secaoOverlay() {
  const caixa = document.querySelector<HTMLElement>("#a-overlay");
  if (!caixa) return;
  const e = await ponte.overlayEstado().catch(() => null);
  if (!e) {
    caixa.innerHTML = `<div class="rotulo">FPS por cima do jogo</div><p class="nota" style="margin-top:8px">Não consegui ler o overlay.</p>`;
    return;
  }
  const { modo, canto } = e.config;
  const cf = await ponte.contadorFivem().catch(() => null);
  caixa.innerHTML = `
    <div class="titulo-sec"><div class="rotulo">FPS por cima do jogo</div>${e.ligado ? `<span class="pilula"><span class="v-vivo"></span>no ${esc(e.jogo ?? "jogo")}</span>` : ""}</div>
    <p class="nota" style="margin-top:6px">Só número medido pelo PresentMon (precisa do Otimiza como administrador). Com o Frame Boost, mostra o FPS do próprio jogo; o que chega à tela fica no painel do Frame Boost. Aparece nos prints e gravações (como o cl_drawfps), não pega clique e fica fora de jogos com anticheat que proíbe.</p>
    <div class="seg" id="o-modo" style="margin-top:12px">${MODOS.map(([m, t]) => `<button aria-pressed="${m === modo}" data-m="${m}">${t}</button>`).join("")}</div>
    <div class="seg" id="o-canto" style="margin-top:8px" aria-label="Canto">${CANTOS.map(([c, t]) => `<button aria-pressed="${c === canto}" data-c="${c}" aria-label="${c}">${t}</button>`).join("")}</div>
    <div class="linha" style="margin-top:8px"><div>No lugar do contador do FiveM<p>${cf?.pendente ? "Vale quando o FiveM fechar: o Otimiza troca na hora certa e você não precisa digitar nada no F8." : "Desliga o cl_drawFPS no arquivo de configuração do FiveM (sem tocar no jogo) e deixa o FPS do Otimiza sempre no mesmo canto. Desligar devolve como estava."}</p></div><button class="chave" role="switch" aria-checked="${!!cf?.ligado}" id="o-fivem" aria-label="No lugar do contador do FiveM"></button></div>
    <div class="linha" style="margin-top:8px"><div>Mostrar com o Frame Boost<p>O contador do próprio jogo nunca vê os quadros gerados. Com isto, o FPS do jogo aparece no canto, igual ao cl_drawfps, enquanto o Frame Boost estiver ligado; quantos quadros chegam à tela fica no painel do Frame Boost. No FiveM, desligue o dele no F8: cl_drawfps false.</p></div><button class="chave" role="switch" aria-checked="${e.config.com_frame_boost}" id="o-fb" aria-label="Mostrar com o Frame Boost"></button></div>
    <div class="acoes" style="margin-top:12px"><button class="botao" id="o-ligar" ${estado.jogo ? "" : "disabled title=\"Abra um jogo\""}>${e.ligado ? "Já está no jogo" : "Mostrar agora no jogo"}</button></div>
    <p id="o-estado" hidden></p>`;

  const aviso = (texto: string, classe = "estado") => {
    const s = $("#o-estado");
    s.hidden = false;
    s.className = classe;
    s.innerHTML = texto;
  };
  const gravar = async (m: ModoDoOverlay, c: CantoDoOverlay) => {
    try {
      await ponte.overlayGravar(m, c);
    } catch (erro) {
      aviso(esc(String(erro)), "estado aviso");
    }
    await secaoOverlay();
  };
  $$<HTMLButtonElement>("#o-modo button").forEach((b) => b.addEventListener("click", () => void gravar(b.dataset.m as ModoDoOverlay, canto)));
  $$<HTMLButtonElement>("#o-canto button").forEach((b) => b.addEventListener("click", () => void gravar(modo, b.dataset.c as CantoDoOverlay)));
  const ligar = async (confirmado: boolean) => {
    try {
      await ponte.overlayLigar(confirmado);
      await secaoOverlay();
    } catch (erro) {
      const texto = String(erro);
      if (texto.startsWith("CONFIRMAR:")) {
        aviso(`${esc(texto.slice(10))} <button class="botao" id="o-confirmar" style="margin-left:8px">Mostrar mesmo assim</button>`, "estado aviso");
        $("#o-confirmar").addEventListener("click", () => void ligar(true));
      } else {
        aviso(esc(texto), "estado aviso");
      }
    }
  };
  $("#o-ligar").addEventListener("click", () => void ligar(false));
  $("#o-fivem").addEventListener("click", async () => {
    try {
      await (cf?.ligado ? ponte.contadorFivemDesligar() : ponte.contadorFivemLigar());
    } catch (erro) {
      aviso(esc(String(erro)), "estado aviso");
      return;
    }
    await secaoOverlay();
  });
  $("#o-fb").addEventListener("click", async () => {
    await ponte.overlayGravar(modo, canto, !e.config.com_frame_boost).catch(() => {});
    await secaoOverlay();
  });
}

async function telaAjustes() {
  const t = $("#t-ajustes");
  const lic = await ponte.licenca().catch(() => null);
  const fb = estado.feedback;
  const textoFb = !fb ? "" : fb.situacao === "Liberado" ? "Feedback recebido. Obrigado." : fb.situacao === "Exigido" ? "As correções estão pausadas até chegar o seu feedback." : fb.situacao === "Contando" ? `O pedido de feedback aparece em ${Math.ceil(fb.restante_segundos / 60)} min.` : "";

  t.innerHTML = `
    <div class="cabeca"><div><h1><span class="barras"></span>Ajustes</h1><p>Licença, feedback e o botão que devolve o PC como estava.</p></div></div>
    <div class="grade g-2">
      <div class="vidro">
        <div class="linha"><div>Licença<p class="num">${esc(lic?.maquina ?? "")}</p></div><span class="selo ${lic?.ativa ? "medido" : "aviso"}">${lic?.ativa ? (lic.expira ? `até ${esc(lic.expira)}` : "vitalícia") : "não ativada"}</span></div>
        ${textoFb ? `<div class="linha"><div>Feedback<p>${esc(textoFb)}</p></div>${fb?.situacao === "Exigido" ? `<button class="botao" id="a-fb">Enviar feedback</button>` : ""}</div>` : ""}
        ${estado.hw?.windows_modificado ? `<div class="linha"><div>Windows deste PC<p>${esc(estado.hw.windows_modificado.nome)}${estado.hw.windows_modificado.autor ? `, feito por ${esc(estado.hw.windows_modificado.autor)}` : ""}: uma versão modificada do Windows. Essas versões tiram ou desligam peças do sistema para ficar mais leves, e alguns recursos podem não existir nela. Se algo do Otimiza não funcionar aqui, a causa pode ser o sistema.</p></div><span class="selo aviso">modificado</span></div>` : ""}
        <div class="linha"><div>Modo clássico<p>As telas da 3.2, com cada ajuste do Windows, energia, núcleos e BIOS um por um.</p></div><button class="botao" id="a-classico">Abrir</button></div>
        ${estado.hw?.placa.fabricante === "Nvidia" ? `<div class="linha"><div>Smooth Motion pelo Otimiza (experimental)<p>Liga o Smooth Motion no perfil do jogo no driver, com um ajuste que a NVIDIA não documenta. Só RTX 40 e 50; se desfaz em um clique.</p></div><button class="chave" role="switch" aria-checked="false" id="a-sm" aria-label="Smooth Motion pelo Otimiza"></button></div>` : ""}
        <div class="linha"><div>Interface nova<p>Desligar volta para a 3.2 também na próxima vez que o Otimiza abrir.</p></div><button class="chave" role="switch" aria-checked="true" id="a-shell" aria-label="Interface nova"></button></div>
      </div>
      <div class="vidro">
        <div class="rotulo">Segurança</div>
        <h2 style="margin-top:8px">Restaurar meu PC</h2>
        <p class="nota" style="margin-top:4px">Desfaz tudo o que o Otimiza mudou, na ordem inversa, conferindo cada valor com o que estava antes.</p>
        <div class="acoes" style="margin-top:14px"><button class="botao" id="a-restaurar">Restaurar tudo</button><button class="botao" id="a-restaurar-sim" hidden>Sim, restaurar agora</button></div>
        <div class="estado" id="a-estado" hidden></div>
      </div>
      <div class="vidro" id="a-overlay"><div class="rotulo">FPS por cima do jogo</div><p class="nota" style="margin-top:8px">Lendo…</p></div>
    </div>`;

  $("#a-classico").addEventListener("click", () => modoClassico(true));
  void secaoOverlay();
  const chaveSm = document.querySelector<HTMLButtonElement>("#a-sm");
  if (chaveSm) {
    const f = await ponte.flags().catch(() => null);
    chaveSm.setAttribute("aria-checked", String(!!f?.smooth_motion_drs));
    chaveSm.addEventListener("click", async () => {
      const atual = await ponte.flags();
      const ligado = !atual.smooth_motion_drs;
      await ponte.gravarFlags({ ...atual, smooth_motion_drs: ligado });
      chaveSm.setAttribute("aria-checked", String(ligado));
    });
  }
  $("#a-fb")?.addEventListener("click", () => abrirPedido());
  $("#a-shell").addEventListener("click", async () => {
    const f = await ponte.flags();
    await ponte.gravarFlags({ ...f, novo_shell: false });
    modoClassico(true);
  });
  $("#a-restaurar").addEventListener("click", () => {
    $("#a-restaurar-sim").hidden = false;
    const s = $("#a-estado");
    s.hidden = false;
    s.className = "estado aviso";
    s.textContent = "Isto desfaz todas as mudanças do Otimiza. Confirme para continuar.";
  });
  $("#a-restaurar-sim").addEventListener("click", async () => {
    const s = $("#a-estado");
    s.className = "estado";
    s.textContent = "Restaurando…";
    $("#a-restaurar-sim").hidden = true;
    try {
      await ponte.restaurarTudo();
      s.className = "estado ok";
      s.textContent = "Pronto: o PC voltou ao que era antes do Otimiza.";
    } catch (e) {
      s.className = "estado aviso";
      s.textContent = String(e);
    }
  });
}

// ---------------------------------------------------------------- modo clássico

function modoClassico(ligar: boolean) {
  document.documentElement.classList.remove("v33-cedo");
  document.body.classList.toggle("v33-ativo", !ligar);
  let volta = document.getElementById("v33-voltar");
  if (ligar && !volta) {
    volta = document.createElement("button");
    volta.id = "v33-voltar";
    volta.textContent = "Voltar para a interface nova";
    Object.assign(volta.style, { position: "fixed", right: "16px", bottom: "16px", zIndex: "80", padding: "10px 16px", borderRadius: "14px", border: "1px solid rgba(255,255,255,.2)", background: "#0b0b0b", color: "#f4f4f4", font: "500 13px Geist, sans-serif", cursor: "pointer" });
    volta.addEventListener("click", async () => {
      const f = await ponte.flags().catch(() => null);
      if (f && !f.novo_shell) await ponte.gravarFlags({ ...f, novo_shell: true });
      modoClassico(false);
    });
    document.body.append(volta);
  }
  if (volta) volta.hidden = !ligar;
}

// ---------------------------------------------------------------- feedback

let fecharPedido = false;
let vigiaFeedback = 0;

function abrirPedido(aviso?: string) {
  const fb = estado.feedback;
  $("#v-veu").hidden = false;
  $("#v-lembrete").hidden = true;
  $("#f-codigo").textContent = fb?.maquina ?? "—";
  if (fb?.situacao === "Exigido" && fb.apagado) $("#f-sub").textContent = "A sua mensagem não está mais no canal. Mande o feedback de novo para liberar as correções.";
  const s = $("#f-estado");
  s.hidden = !aviso;
  if (aviso) {
    s.className = "estado aviso";
    s.textContent = aviso;
  }
}

function aplicarFeedback(fb: SituacaoDoFeedback & Partial<EstadoDoFeedback>) {
  estado.feedback = { ...(estado.feedback ?? { maquina: "", canal: null, canal_id: "", aviso: null }), ...fb } as EstadoDoFeedback;
  const s = estado.feedback;
  if (s.situacao === "Exigido" && !fecharPedido) abrirPedido();
  if (s.situacao === "Liberado") $("#v-veu").hidden = true;
  const perto = s.situacao === "Contando" && s.restante_segundos <= 120;
  $("#v-lembrete").hidden = !perto;
  if (perto) $("#l-titulo").textContent = `Faltam ${Math.max(1, Math.ceil(s.restante_segundos / 60))} minuto(s)`;

  // Contando: confere a cada 30 s, para o lembrete aparecer na hora certa (o evento só vem na troca de situação).
  clearInterval(vigiaFeedback);
  if (s.situacao === "Contando") {
    vigiaFeedback = window.setInterval(async () => aplicarFeedback(await ponte.feedback()), 30000);
  }
}

function ligarFeedback() {
  $("#l-fechar").addEventListener("click", () => ($("#v-lembrete").hidden = true));
  $("#l-abrir").addEventListener("click", () => abrirPedido());
  $("#f-depois").addEventListener("click", () => {
    fecharPedido = true;
    $("#v-veu").hidden = true;
  });
  $("#f-copiar").addEventListener("click", async (e) => {
    const b = e.currentTarget as HTMLButtonElement;
    try {
      await navigator.clipboard.writeText(estado.feedback?.maquina ?? "");
      b.textContent = "Copiado";
    } catch {
      b.textContent = "Selecione e copie";
    }
  });
  $("#f-canal").addEventListener("click", async () => {
    const alvo = estado.feedback?.canal ?? (await ponte.convite(CONVITE).catch(() => CONVITE));
    await openUrl(alvo).catch(() => {});
  });
  $("#f-confirmar").addEventListener("click", async () => {
    const s = $("#f-estado");
    s.hidden = false;
    s.className = "estado";
    s.textContent = "Conferindo com o bot…";
    try {
      const fb = await ponte.feedbackVerificar();
      aplicarFeedback(fb);
      if (fb.situacao === "Liberado") {
        s.className = "estado ok";
        s.textContent = "Feedback encontrado. As correções estão liberadas.";
        setTimeout(() => ($("#v-veu").hidden = true), 1400);
      } else {
        s.className = "estado aviso";
        s.textContent = fb.aviso ?? "Ainda não achamos a sua mensagem. Tente de novo em um minuto.";
      }
    } catch (e) {
      s.className = "estado aviso";
      s.textContent = String(e);
    }
  });
}

// ---------------------------------------------------------------- início do app

async function carregar() {
  // O último retrato gravado primeiro: a tela aparece na hora, e a leitura nova chega depois.
  estado.hw = await ponte.hardwareUltimo().catch(() => null);
  inicio();
  void painel();

  const [hw, compat, bib, fb] = await Promise.all([
    ponte.hardware().catch(() => estado.hw),
    ponte.compatibilidade().catch(() => null),
    ponte.biblioteca().catch(() => []),
    ponte.feedback().catch(() => null),
  ]);
  estado.hw = hw;
  estado.compat = compat;
  estado.biblioteca = bib;
  // Perguntado DEPOIS da leitura lenta do hardware: perguntado junto, a resposta velha ("nenhum jogo") chegava depois
  // do aviso `jogo:sessao` e apagava o jogo que tinha acabado de abrir.
  await trocarJogo(await ponte.jogoAberto().catch(() => estado.jogo));
  if (fb) aplicarFeedback(fb);
  estado.gerador = await ponte.geradorEstado().catch(() => null);
  inicio();
  void painel();
}

async function trocarJogo(jogo: JogoAberto | null) {
  estado.jogo = jogo;
  try {
    await trocarJogoNaTela();
  } catch (erro) {
    void ponte.anotarErro(`trocar jogo: ${erro instanceof Error ? erro.stack ?? erro.message : String(erro)}`).catch(() => {});
    inicio();
  }
}

async function trocarJogoNaTela() {
  inicio();
  void cuidados();
  void painel();
  if (estado.destino !== "inicio") ir(estado.destino);
}

function vigiarErros() {
  const anotar = (texto: string) => void ponte.anotarErro(texto).catch(() => {});
  window.addEventListener("error", (e) => anotar(`${e.message} em ${e.filename}:${e.lineno}`));
  window.addEventListener("unhandledrejection", (e) => anotar(`promessa: ${e.reason instanceof Error ? e.reason.stack ?? e.reason.message : String(e.reason)}`));
}

export async function iniciarV33() {
  vigiarErros();
  const raiz = document.getElementById("v33");
  if (!raiz) {
    document.documentElement.classList.remove("v33-cedo");
    return;
  }
  const flags = await ponte.flags().catch(() => null);
  if (flags && !flags.novo_shell) {
    modoClassico(true);
    return;
  }

  logoD = marcaSvg.match(/ d="([^"]+)"/)?.[1] ?? "";
  // Do próprio arquivo: com a logo nova (11/10/2026) o desenho mudou de tamanho, e um número fixo aqui a deslocava.
  logoViewBox = marcaSvg.match(/viewBox="([^"]+)"/)?.[1] ?? logoViewBox;

  raiz.innerHTML = estrutura();
  document.body.classList.add("v33-ativo");
  raiz.classList.add("sem-painel");
  document.documentElement.classList.remove("v33-cedo");

  $$<HTMLButtonElement>(".ir", raiz).forEach((b) => b.addEventListener("click", () => ir(b.dataset.ir as Destino)));
  $("#v-busca").addEventListener("click", () => document.getElementById("abrir-comandos-topo")?.click());
  $("#d-otimizar").addEventListener("click", () => void otimizar());
  $("#d-boost").addEventListener("click", () => void alternarBoost());
  $("#d-turbo").addEventListener("click", () => void turboDaTelaDoJogo());
  $("#d-lab").addEventListener("click", () => ir("lab"));
  $("#d-desfazer").addEventListener("click", () => ir("ajustes"));
  ligarFeedback();


  await ouvir<JogoAberto | null>("jogo:sessao", (j) => void trocarJogo(j));
  await ouvir<SituacaoDoFeedback>("feedback:situacao", (s) => aplicarFeedback(s));
  await ouvir("prova:automatica", async () => {
    estado.biblioteca = await ponte.biblioteca().catch(() => estado.biblioteca);
    inicio();
    void painel();
  });

  ligarImagens();
  await carregar();
  void ponte.iniciarMonitor().catch(() => {});
  window.setInterval(() => void amostrar(), 2000);
  void amostrar();

}

