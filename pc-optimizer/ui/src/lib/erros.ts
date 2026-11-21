// O erro mostrado ao cliente (1.0.12, Frente 1.7): todo erro diz o que aconteceu e o que fazer, em português. O Rust já
// devolve frases para gente na maioria dos casos ("abra o jogo para guardar o perfil dele"); quando a frase só diz o
// que aconteceu, ou quando vem crua do Windows ("Access is denied. (os error 5)"), aqui entra o próximo passo.
// Puro, para o teste (`testes/erros.test.ts`).

/** Onde o cliente acha o botão que gera o relatório para o suporte. */
export const ONDE_O_SUPORTE = "Ajustes › Versão e suporte";

const FAZER_ADMIN = "Abra o Otimiza como administrador (botão direito no atalho › Executar como administrador) e tente de novo.";
const FAZER_PADRAO = `Tente de novo em alguns segundos. Se repetir, gere o relatório em ${ONDE_O_SUPORTE} e mande ao suporte.`;

/** Começos de frase que já são uma instrução: a mensagem já diz o que fazer. */
const JA_DIZ_O_QUE_FAZER =
  /\b(abra|feche|tente|ative|reinicie|atualize|conecte|instale|ligue|desligue|jogue|volte|use|espere|aguarde|calibre|escolha|mande|confira|rode|entre|saia|apague|clique|toque|deixe|troque|suba|baixe|libere|selecione|ajuste|desfaça|refaça|abre|fecha)\b/i;

/** O que o Windows diz em inglês e o que isso quer dizer para o jogador. */
const TRADUCOES: [RegExp, string, string][] = [
  [/access (is )?denied|os error 5\b|acesso negado|requires elevation|privil[eé]gio/i, "O Windows não deixou: precisa de administrador.", FAZER_ADMIN],
  [/os error 32\b|being used by another process|sendo usado por outro processo/i, "O arquivo está aberto em outro programa.", "Feche o jogo (e o launcher dele) e tente de novo."],
  [/os error 2\b|cannot find the (file|path)|não foi possível encontrar/i, "Não achei o arquivo ou a pasta.", "Abra o jogo uma vez e feche: ele cria o que falta. Depois tente de novo."],
  [/timed? ?out|tempo esgotado|demorou demais/i, "O Windows demorou demais para responder.", "Tente de novo em alguns segundos. Se o PC estiver ocupado (atualização, antivírus), espere terminar."],
  [/no space|espaço insuficiente|disk full|os error 112\b/i, "O disco está cheio.", "Libere espaço no disco C: (a área Sistema › Limpeza ajuda) e tente de novo."],
  [/network|rede indispon|dns|could not connect|failed to fetch|conexão/i, "Sem conexão com a internet agora.", "Confira a internet e tente de novo."],
  [/licen[çc]a/i, "", "Ative a licença pela barra de cima (Ativar licença)."],
];

/** A mensagem crua de um erro qualquer (Error, texto, objeto do Tauri). */
export function mensagemDe(e: unknown): string {
  if (e == null) return "";
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  if (typeof e === "object" && "message" in e && typeof (e as { message: unknown }).message === "string") return (e as { message: string }).message;
  return String(e);
}

function frase(t: string) {
  const limpo = t.replace(/^(error|erro)\s*:\s*/i, "").trim();
  if (!limpo) return "";
  const comMaiuscula = limpo[0].toUpperCase() + limpo.slice(1);
  return /[.!?…]$/.test(comMaiuscula) ? comMaiuscula : `${comMaiuscula}.`;
}

/** O erro para a tela: o que aconteceu + o que fazer, numa ou duas frases. */
export function explicarErro(e: unknown): string {
  const cru = mensagemDe(e).trim();
  for (const [padrao, aconteceu, fazer] of TRADUCOES) {
    if (!padrao.test(cru)) continue;
    // Frase em inglês do Windows: troca pela explicação. Frase já em português: mantém e soma o que fazer, se faltar.
    const emIngles =
      !/[ãõçéêáíóúâô]/i.test(cru) && /\b(the|is|was|not|cannot|could|failed|denied|timed|out|error|access|file|operation)\b/i.test(cru);
    const base = emIngles || !cru ? aconteceu : frase(cru);
    return JA_DIZ_O_QUE_FAZER.test(base) ? base : `${base} ${fazer}`.trim();
  }
  const base = frase(cru);
  if (!base) return `Algo deu errado. ${FAZER_PADRAO}`;
  return JA_DIZ_O_QUE_FAZER.test(base) ? base : `${base} ${FAZER_PADRAO}`;
}
