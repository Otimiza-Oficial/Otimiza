import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

/*
 * Por que o jogo trava (docs/planos/PIP-ENGENHARIA.md, seção 10). A tela só traduz: o motor (`causadatravada`)
 * compara cada suspeito com a taxa de base dele, diz a evidência a favor e contra, ou se abstém; o teste ativo
 * (`testedatravada`) prova ou derruba a suspeita com o jogo aberto. Nenhum número aqui é calculado na tela.
 */

type Confianca = "NaoMedido" | "Inconclusiva" | "SemEvidencia" | "Moderada" | "Forte";
type Hipotese =
  | { tipo: "Processo"; nome: string }
  | { tipo: "Disco" }
  | { tipo: "Paginacao" }
  | { tipo: "Vram" }
  | { tipo: "NucleoSaturado" }
  | { tipo: "ProcessadorCheio" };
type Teste =
  | { tipo: "AcalmarProcesso"; processo: string }
  | { tipo: "Recomendacao"; texto: string }
  | { tipo: "Nenhum" };

interface Avaliacao {
  hipotese: Hipotese;
  confianca: Confianca;
  episodios_com_sinal: number;
  episodios_cobertos: number;
  taxa_de_base: number;
  lift: number | null;
  p_corrigido: number | null;
  a_favor: string[];
  contra: string[];
  teste: Teste;
  ja_testado: string | null;
}

interface Investigacao {
  segundos: number;
  quadros: number;
  fps_mediano: number | null;
  episodios: unknown[];
  por_minuto: number;
  severidades: { leve: number; moderada: number; severa: number; extrema: number };
  p99_ms: number | null;
  p999_ms: number | null;
  lado_processador: number;
  lado_placa: number;
  avaliacoes: Avaliacao[];
  sem_causa: number;
  abstencao: string | null;
  conclusao: string;
}

interface ResultadoDaInvestigacao {
  jogo: string;
  investigacao: Investigacao;
  custo: {
    otimiza_pct_de_um_nucleo: number | null;
    presentmon_pct_de_um_nucleo: number | null;
    amostras: number;
    analise_ms: number;
  };
  teste_possivel: string | null;
  motivo_sem_teste: string | null;
}

interface ResultadoDoTeste {
  processo: string;
  julgamento: {
    decisao: "Manter" | "SemMelhoriaQueImporta" | "SemMelhoriaConfiavel" | "PiorouAsTravadas" | "PiorouOFps" | "Invalido" | "Continuar";
    frase: string;
    janelas: { lado: "Normal" | "Acalmado"; travadas: number; minutos: number; fps: number | null; valida: boolean; motivo: string | null }[];
  };
  mantido: boolean;
  aviso: string | null;
}

const ROTULO: Record<Confianca, string> = {
  Forte: "Associação forte",
  Moderada: "Associação moderada",
  SemEvidencia: "Sem evidência",
  Inconclusiva: "Inconclusivo",
  NaoMedido: "Não medido",
};

function el(id: string): HTMLElement {
  const e = document.getElementById(id);
  if (!e) throw new Error(`#${id} não existe`);
  return e;
}

function esc(s: string): string {
  const d = document.createElement("div");
  d.textContent = s;
  return d.innerHTML;
}

function status(msg: string, kind: "ok" | "warn" | "error" | "progress") {
  const s = el("travadas-status");
  s.textContent = msg;
  s.className = `status ${kind}`;
}

function numero(x: number, casas = 0): string {
  return x.toLocaleString("pt-BR", { minimumFractionDigits: casas, maximumFractionDigits: casas });
}

function nomeDa(h: Hipotese): string {
  switch (h.tipo) {
    case "Processo":
      return h.nome;
    case "Disco":
      return "Disco lento";
    case "Paginacao":
      return "Falta de memória (paginação)";
    case "Vram":
      return "Memória de vídeo cheia";
    case "NucleoSaturado":
      return "Um núcleo no limite (o próprio jogo)";
    case "ProcessadorCheio":
      return "Processador inteiro ocupado";
  }
}

function cartao(a: Avaliacao, i: number): string {
  const severidade = a.confianca === "Forte" ? "Important" : a.confianca === "Moderada" ? "Info" : "Neutral";
  const conta =
    a.lift !== null && a.p_corrigido !== null
      ? `<p class="hint">Conta: ${a.episodios_com_sinal} de ${a.episodios_cobertos} travadas com o sinal · ${numero(a.taxa_de_base * a.episodios_cobertos, 1)} esperadas ao acaso · ${numero(a.lift, 1)}× o esperado · chance de ser acaso ${a.p_corrigido < 0.001 ? "< 0,1%" : `${numero(a.p_corrigido * 100, 1)}%`} (já corrigida pelo número de suspeitos).</p>`
      : "";
  const contra = a.contra.length
    ? `<p class="finding-advice"><strong>Contra:</strong> ${a.contra.map(esc).join(" ")}</p>`
    : "";
  const teste =
    a.teste.tipo === "Recomendacao"
      ? `<p class="finding-advice"><strong>Para testar:</strong> ${esc(a.teste.texto)}</p>`
      : "";
  const antes = a.ja_testado ? `<p class="hint">${esc(a.ja_testado)}</p>` : "";
  return `
    <article class="finding" data-severity="${severidade}" style="--i:${i}">
      <div class="finding-top">
        <h3>${esc(nomeDa(a.hipotese))}</h3>
        <span class="finding-size">${ROTULO[a.confianca]}</span>
      </div>
      ${a.a_favor.map((f) => `<p class="finding-measured">${esc(f)}</p>`).join("")}
      ${contra}${conta}${teste}${antes}
    </article>`;
}

let ultima: ResultadoDaInvestigacao | null = null;

function desenhar(r: ResultadoDaInvestigacao) {
  const inv = r.investigacao;
  const fortes = inv.avaliacoes.filter((a) => a.confianca === "Forte" || a.confianca === "Moderada");
  const outras = inv.avaliacoes.filter((a) => !fortes.includes(a));
  const s = inv.severidades;
  const caudas = [
    inv.p99_ms !== null ? `P99 ${numero(inv.p99_ms, 1)} ms` : null,
    inv.p999_ms !== null ? `P99,9 ${numero(inv.p999_ms, 1)} ms` : null,
  ].filter(Boolean);
  const custo = [
    r.custo.otimiza_pct_de_um_nucleo !== null ? `Otimiza ${numero(r.custo.otimiza_pct_de_um_nucleo, 1)}%` : "Otimiza não medido",
    r.custo.presentmon_pct_de_um_nucleo !== null ? `PresentMon ${numero(r.custo.presentmon_pct_de_um_nucleo, 1)}%` : "PresentMon não medido",
  ].join(" · ");

  el("travadas-tag").textContent = `${numero(inv.por_minuto, 1)} travadas/min`;
  el("travadas-resultado").innerHTML = `
    <p class="lead">${esc(inv.conclusao)}</p>
    <p class="hint">${esc(r.jogo)} · ${numero(inv.segundos)} s · ${numero(inv.quadros)} quadros${inv.fps_mediano !== null ? ` · ${numero(inv.fps_mediano)} FPS medianos` : ""} · ${inv.episodios.length} travadas (${s.leve} leves, ${s.moderada} moderadas, ${s.severa} severas, ${s.extrema} extremas)${caudas.length ? ` · ${caudas.join(" · ")}` : ""} · lado do processador ${inv.lado_processador}, da placa ${inv.lado_placa}.</p>
    ${fortes.map(cartao).join("")}
    ${
      outras.length
        ? `<details><summary>O que mais foi verificado (${outras.length})</summary>${outras.map((a, i) => cartao(a, i + fortes.length)).join("")}</details>`
        : ""
    }
    ${r.motivo_sem_teste ? `<p class="finding-advice">${esc(r.motivo_sem_teste)}</p>` : ""}
    <p class="hint">Custo desta investigação, em % de um núcleo: ${custo}.</p>`;

  const bloco = el("travadas-teste");
  if (!r.teste_possivel) {
    bloco.hidden = true;
    return;
  }
  const p = esc(r.teste_possivel);
  bloco.hidden = false;
  bloco.innerHTML = `
    <h3>Testar se o ${p} é a causa</h3>
    <ul class="linha-do-tempo">
      <li><strong>O que muda:</strong> o ${p} passa para prioridade baixa e modo econômico em janelas de 30 s, alternando com o normal. Nada é fechado; o jogo não é tocado.</li>
      <li><strong>Por quê:</strong> associação não é causa. Se acalmar o ${p} diminuir as travadas, ele é a causa; se não, a suspeita cai.</li>
      <li><strong>Duração:</strong> até 12 minutos jogando normalmente, no mesmo tipo de lugar. Para antes se a resposta vier.</li>
      <li><strong>Risco:</strong> o ${p} fica um pouco mais lento enquanto está acalmado.</li>
      <li><strong>Volta:</strong> ao fim de cada etapa, no erro, e na próxima abertura do Otimiza se ele fechar no meio. Só fica acalmado se as travadas caírem 30% ou mais e o FPS médio, medido em janelas vizinhas, não cair mais de 5%; e só até o jogo fechar.</li>
      <li><strong>Medido:</strong> travadas por minuto e FPS em cada janela.</li>
    </ul>
    <div class="row-actions">
      <button id="travadas-testar" class="btn btn-primary" data-custo="até 12 min">Começar o teste</button>
    </div>
    <div id="travadas-teste-resultado" class="resultado" data-vazio=""></div>`;
  el("travadas-testar").addEventListener("click", () => void testar(r.teste_possivel as string));
}

async function investigar() {
  const botao = el("travadas-investigar") as HTMLButtonElement;
  botao.disabled = true;
  el("travadas-teste").hidden = true;
  status("Medindo 60 s. Continue jogando, fora do menu…", "progress");
  try {
    ultima = await invoke<ResultadoDaInvestigacao>("investigar_travadas", { segundos: 60 });
    desenhar(ultima);
    status(ultima.investigacao.abstencao ? "Investigação feita: nada para apontar." : "Investigação feita.", "ok");
  } catch (e) {
    status(String(e), "error");
  } finally {
    botao.disabled = false;
  }
}

async function testar(processo: string) {
  const botao = el("travadas-testar") as HTMLButtonElement;
  const investigarBotao = el("travadas-investigar") as HTMLButtonElement;
  botao.disabled = true;
  investigarBotao.disabled = true;
  status(`Testando o ${processo}. Continue jogando…`, "progress");
  const parar = await listen<string>("travada:progresso", (e) => status(e.payload, "progress"));
  try {
    const r = await invoke<ResultadoDoTeste>("testar_causa_da_travada", { processo });
    const j = r.julgamento;
    const validas = j.janelas.filter((x) => x.valida).length;
    el("travadas-teste-resultado").innerHTML = `
      <p class="lead">${esc(j.frase)}</p>
      <p class="hint">${validas} de ${j.janelas.length} janelas valeram.</p>
      ${r.aviso ? `<p class="finding-advice"><strong>Atenção:</strong> ${esc(r.aviso)}</p>` : ""}
      ${
        r.mantido
          ? `<p class="finding-advice">O ${esc(r.processo)} fica acalmado até o jogo fechar. <button id="travadas-devolver" class="btn" type="button">Devolver agora</button></p>`
          : r.aviso
            ? ""
            : `<p class="hint">O ${esc(r.processo)} voltou ao normal.</p>`
      }`;
    if (r.mantido) {
      el("travadas-devolver").addEventListener("click", () => void devolver());
    }
    // "Nada ficou mudado" só quando a devolução deu certo: com aviso, algo pode ter ficado para trás.
    if (r.aviso) status("Teste concluído, com um aviso: leia abaixo.", "warn");
    else if (r.mantido) status("Causa demonstrada.", "ok");
    else status("Teste concluído: tudo voltou ao normal.", "warn");
  } catch (e) {
    status(String(e), "error");
  } finally {
    parar();
    botao.disabled = false;
    investigarBotao.disabled = false;
  }
}

async function devolver() {
  try {
    const frase = await invoke<string | null>("devolver_teste_da_travada");
    status(frase ?? "Nada estava acalmado.", "ok");
    const b = document.getElementById("travadas-devolver");
    if (b) b.remove();
  } catch (e) {
    status(String(e), "error");
  }
}

function ligar() {
  const botao = document.getElementById("travadas-investigar");
  if (!botao) return;
  botao.addEventListener("click", () => void investigar());
}

if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", ligar);
} else {
  ligar();
}
