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

type Medicao = { jogo: string; quando: number; fps: number; low_1pct: number; engasgos_por_minuto: number; confiavel: boolean };

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

type Biblioteca = { jogos: Jogo[]; lacunas: string[]; medicoes_erro: string | null };
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

let raiz: HTMLElement | null = null;
let pedirAdmin: (motivo: string) => void = () => {};

function esc(s: string): string {
  return s.replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]!);
}

function quando(ts: number): string {
  return new Date(ts * 1000).toLocaleString("pt-BR", { dateStyle: "short", timeStyle: "short" });
}

function linhaDoJogo(j: Jogo, i: number): string {
  const med = j.ultima_medicao
    ? `<span class="fg-chip">última partida medida ${quando(j.ultima_medicao.quando)}: ${Math.round(j.ultima_medicao.fps)} FPS${
        j.ultima_medicao.confiavel ? ` · 1% piores ${Math.round(j.ultima_medicao.low_1pct)}` : ""
      }</span>`
    : `<span class="fg-chip">ainda sem medição</span>`;

  let acao = "";
  if (j.ajustador === "unreal" && j.executavel) {
    acao = `
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
      ${linhaDoPortao(j)}
      ${linhaDaDeriva(j)}
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
    if (m.length) setTimeout(() => void carregar(), 2500);
    saida.textContent = m.length
      ? `Pronto: ${m.length} opção(ões) ajustada(s). Jogue uma partida e meça no Mapa de desempenho (Painel) para ver a diferença. Para voltar como era, use Desfazer neste cartão.`
      : "Nada precisou mudar.";
  } catch (e) {
    const msg = String(e);
    if (msg.toLowerCase().includes("administrador")) pedirAdmin(msg);
    saida.textContent = msg;
  }
}

async function carregar() {
  if (!raiz) return;
  raiz.innerHTML = `<p class="fg-nota">Procurando os jogos desta máquina…</p>`;
  try {
    const b = await invoke<Biblioteca>("biblioteca_de_jogos");
    const lacunas = [...b.lacunas, ...(b.medicoes_erro ? [b.medicoes_erro] : [])];
    raiz.innerHTML = `
      <div class="fg-painel">
        ${b.jogos.length ? b.jogos.map(linhaDoJogo).join("") : `<p class="fg-aviso">Nenhum jogo encontrado ainda. Abra qualquer jogo por alguns minutos: o Otimiza reconhece pelo que ele faz na tela e na placa de vídeo, e ele passa a aparecer aqui.</p>`}
        ${lacunas.length ? `<p class="fg-nota">Não deu para ler: ${lacunas.map(esc).join("; ")}</p>` : ""}
        <p class="fg-nota">
          Qualquer jogo que você abrir entra nesta lista sozinho, mesmo fora das lojas conhecidas.
        </p>
      </div>`;
    raiz.querySelectorAll<HTMLButtonElement>("[data-desfazer]").forEach((btn) =>
      btn.addEventListener("click", async () => {
        const j = b.jogos[Number(btn.dataset.desfazer)];
        if (!j.ajuste_aplicado) return;
        btn.disabled = true;
        try {
          await invoke("revert_optimization", { id: j.ajuste_aplicado });
          await carregar();
        } catch (e) {
          btn.disabled = false;
          btn.textContent = String(e);
        }
      }),
    );
    raiz.querySelectorAll<HTMLButtonElement>("[data-orcamento][data-jogo]").forEach((btn) =>
      btn.addEventListener("click", () => {
        const i = Number(btn.dataset.jogo);
        void prever(b.jogos[i], i, btn.dataset.orcamento as Orcamento);
      }),
    );
  } catch (e) {
    raiz.innerHTML = `<p class="fg-erro">${esc(String(e))}</p>`;
  }
}

export function carregarBiblioteca(opcoes: { pedirAdmin: (motivo: string) => void }) {
  if (raiz) return;
  raiz = document.getElementById("biblioteca-jogos");
  if (!raiz) return;
  pedirAdmin = opcoes.pedirAdmin;
  void carregar();
}
