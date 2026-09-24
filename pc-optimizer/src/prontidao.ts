import { invoke } from "@tauri-apps/api/core";

/*
 * "Pronto para jogar?" — o Painel olha a MÁQUINA antes da partida: monitor
 * abaixo da taxa, limites escondidos, memória e programas pesando.
 */

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

function esc(s: string): string {
  return s.replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]!);
}

/** Liga o botão "Verificar antes de jogar" do Painel. */
export function ligarProntidao() {
  const botao = document.getElementById("pronto-jogar");
  const saida = document.getElementById("pronto-jogar-resultado");
  if (!botao || !saida || botao.dataset.ligado) return;
  botao.dataset.ligado = "1";
  botao.addEventListener("click", () => void verificarProntidao(saida));
}
