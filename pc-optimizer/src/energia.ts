import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

/*
 * Aba do motor de energia adaptativo (`motorenergia.rs`): a tela só traduz estados em frases e cores, sem
 * comparar texto do Rust. Ganho sem medição aparece como "não medido", nunca um número.
 */

type Arquitetura =
  | "IntelLegada" | "IntelModerna" | "IntelHibrida" | "AmdLegada" | "Zen2" | "Zen3" | "Zen4" | "Zen5Mais" | "Desconhecida";
type Controle = "HardwareHwp" | "HardwareCppc" | "SistemaOperacional";
type Formato = "Desktop" | "Notebook";
type Estacionamento = "Windows" | "Adaptativo" | "TodosAcordados";
type Boost = "Windows" | "Agressivo";
type Autonomia = "Windows" | "Hardware";
type RespostaPol = "Windows" | "Rapida";
type Dispositivo = "Windows" | "Desligada";
type Papel = "PadraoWindows" | "A" | "B" | "C" | "D" | "Refino" | "Epp" | "Dispositivo";
type Motivo =
  | "EppDoCandidato" | "EppNucleosDeEficienciaPreservado" | "AutonomiaPeloHardware" | "BoostAgressivoEmTeste"
  | "SobeMaisCedo" | "SobeSemEsperarJanela" | "SobeDireto" | "DesceEmDegraus" | "DesceSemPressa"
  | "ReservaDeNucleosAcordada" | "NucleosAcordamRapido" | "NucleosDormemDevagar" | "TodosOsNucleosEmTeste"
  | "DicaDeLatenciaAcordaNucleos" | "DicaDeLatenciaDesempenho" | "PreferirNucleosDeDesempenho"
  | "MinimoEmCpuSemControleDeHardware" | "TetoNaoPodeLimitar" | "PcieEconomiaDesligadaEmTeste" | "UsbSuspensaoDesligadaEmTeste";
type MotivoIgnorado = "NaoExiste" | "ForaDaFaixa" | "CpuNaoSuporta";
type Achado =
  | "FirmwareLimitaPorTemperatura" | "FirmwareLimitaPorEnergia" | "ClockReportadoSubiuEfetivoCaiu" | "SemJogoMedido"
  | "TemperaturaIndisponivel" | "NenhumGanhouComMargem" | "EmpateDesfeitoPelaTemperatura" | "PoucasRepeticoes" | "MedicaoInstavel"
  | "PlanoAtualJaEOMelhor" | "CandidatoTirariaFps" | "QuadrosSinteticos";

type Parametros = {
  epp_bruto: number | null;
  estacionamento: Estacionamento;
  boost: Boost;
  autonomia: Autonomia;
  resposta: RespostaPol;
  minimo: number | null;
  pcie: Dispositivo;
  usb: Dispositivo;
};

type Mudanca = { alias: string; subgrupo: string; guid: string; ac: number | null; dc: number | null; motivo: Motivo };
type Candidato = {
  id: string;
  papel: Papel;
  parametros: Parametros;
  mudancas: Mudanca[];
  ignorados: { alias: string; motivo: MotivoIgnorado }[];
};

type Configuracao = {
  subgrupo: string;
  guid: string;
  alias: string | null;
  minimo: number | null;
  maximo: number | null;
  unidade: string | null;
  possiveis: [number, string][];
  ac: number | null;
  dc: number | null;
};

type Impressao = {
  cpu: string;
  arquitetura: Arquitetura;
  controle: Controle;
  cpuid: { familia: number; modelo: number; hibrido: boolean; hwp: boolean; hwp_epp: boolean; nucleos_preferidos_intel: boolean; cppc: boolean };
  topologia: { nucleos_fisicos: number; processadores_logicos: number; classes: [number, number][] };
  formato: Formato;
  na_tomada: boolean | null;
  build_do_windows: number;
  modern_standby: boolean;
  plano_ativo: string | null;
  plano_ativo_nome: string | null;
};

type PerfilDeJogo = { executavel: string; parametros: Parametros; escolha: Escolha | null; quando: number };

type Painel = {
  impressao: Impressao;
  bateria: { autoajuste: Candidato[]; escada_de_epp: Candidato[]; dispositivos: Candidato[]; controlados: string[] };
  ppm_atual: Configuracao[];
  plano_otimiza_ativo: boolean;
  backup: { plano_anterior: string; nome_anterior: string | null; capturado_em: number } | null;
  perfis_de_jogo: PerfilDeJogo[];
  dinamico: boolean;
  elevado: boolean;
  teste_interrompido: boolean;
};

type RespostaMedida = { ate_90_ms: number; primeira_fatia_pct: number; sustentado: number; dispersao_pct: number; rajadas: number };
type ResumoCpu = {
  amostras: number;
  clock_reportado_mhz: number;
  clock_efetivo_mhz: number;
  limite_medio_pct: number;
  tempo_limitado_pct: number;
  limite_termico: boolean;
  limite_eletrico: boolean;
  temperatura_max_c: number | null;
  temperatura_media_c: number | null;
  nucleos_acordados_medio: number | null;
  nucleos_total: number | null;
};
type Quadros = { fps: number; low_1: number; low_01: number; p99_ms: number };
type Resultado = {
  candidato: string;
  resposta: RespostaMedida | null;
  cpu: ResumoCpu;
  quadros: Quadros | null;
  fps_repeticoes: number[];
  gpu_pct: number | null;
  uso_cpu_pct: number | null;
  quadros_sinteticos: boolean;
};
type Aplicacao = { plano: string; candidato: string; escritos: number; divergentes: string[]; recusados: string[]; ativo: boolean };
type Medicao = { resultado: Resultado; curva_pct: number[]; aplicacao: Aplicacao | null };
type Nota = {
  total: number;
  low_1: number | null;
  p99: number | null;
  fps: number | null;
  resposta: number | null;
  clock: number | null;
  termica: number | null;
  estabilidade: number | null;
};
type Escolha = {
  notas: [string, Nota][];
  vencedor: string | null;
  confianca_pct: number;
  achados: Achado[];
  fps_pct: number | null;
  low_1_pct: number | null;
  p99_pct: number | null;
  resposta_pct: number | null;
  temperatura_delta_c: number | null;
};
type Restauracao = { plano_ativo: string; divergentes: string[]; plano_otimiza_apagado: boolean };

type Tom = "ok" | "aviso" | "erro" | "neutro";

const ARQUITETURA: Record<Arquitetura, string> = {
  IntelLegada: "Intel sem Speed Shift",
  IntelModerna: "Intel com Speed Shift",
  IntelHibrida: "Intel híbrida (núcleos P e E)",
  AmdLegada: "AMD Zen / Zen+ ou anterior",
  Zen2: "AMD Zen 2",
  Zen3: "AMD Zen 3",
  Zen4: "AMD Zen 4",
  Zen5Mais: "AMD Zen 5 ou mais novo",
  Desconhecida: "não identificada",
};

const CONTROLE: Record<Controle, string> = {
  HardwareHwp: "o processador escolhe (Intel Speed Shift / HWP)",
  HardwareCppc: "o processador escolhe (AMD CPPC)",
  SistemaOperacional: "o Windows escolhe cada estado",
};

const ESTACIONAMENTO: Record<Estacionamento, string> = {
  Windows: "como o Windows deixa",
  Adaptativo: "adaptativo — reserva acordada, o resto acorda rápido",
  TodosAcordados: "todos os núcleos acordados",
};

const MOTIVO: Record<Motivo, string> = {
  EppDoCandidato: "EPP que este candidato testa",
  EppNucleosDeEficienciaPreservado: "núcleos E mantêm o EPP do Windows",
  AutonomiaPeloHardware: "deixa o processador gerenciar os P-states",
  BoostAgressivoEmTeste: "boost agressivo, só como candidato medido",
  SobeMaisCedo: "sobe o desempenho com menos carga acumulada",
  SobeSemEsperarJanela: "sobe sem esperar várias janelas de medição",
  SobeDireto: "sobe direto ao alvo, não em degraus",
  DesceEmDegraus: "desce em degraus",
  DesceSemPressa: "espera mais antes de descer",
  ReservaDeNucleosAcordada: "reserva de núcleos acordada",
  NucleosAcordamRapido: "núcleos acordam na primeira checagem",
  NucleosDormemDevagar: "núcleos demoram mais para dormir",
  TodosOsNucleosEmTeste: "todos acordados, só como candidato medido",
  DicaDeLatenciaAcordaNucleos: "em clique/abertura, acorda os núcleos",
  DicaDeLatenciaDesempenho: "em clique/abertura, desempenho total por instantes",
  PreferirNucleosDeDesempenho: "prefere núcleos P sem proibir os E",
  MinimoEmCpuSemControleDeHardware: "estado mínimo, onde ele de fato governa",
  TetoNaoPodeLimitar: "remove um teto de frequência que estava limitando",
  PcieEconomiaDesligadaEmTeste: "ASPM desligado na tomada, só em teste",
  UsbSuspensaoDesligadaEmTeste: "suspensão do USB desligada na tomada, só em teste",
};

const IGNORADO: Record<MotivoIgnorado, string> = {
  NaoExiste: "este Windows não expõe",
  ForaDaFaixa: "fora da faixa que o Windows publica",
  CpuNaoSuporta: "a CPU não tem o recurso",
};

const ACHADO: Record<Achado, { texto: string; tom: Tom }> = {
  FirmwareLimitaPorTemperatura: {
    texto: "O firmware limitou a CPU por temperatura durante a medição. Nenhum plano de energia resolve isso — é resfriamento.",
    tom: "erro",
  },
  FirmwareLimitaPorEnergia: {
    texto: "O firmware limitou a CPU por energia (limite de potência/corrente do fabricante). O plano não consegue passar desse teto e não finge que consegue.",
    tom: "erro",
  },
  ClockReportadoSubiuEfetivoCaiu: {
    texto: "Um candidato subiu o clock reportado, mas o clock efetivo caiu. Isso não foi contado como melhora.",
    tom: "aviso",
  },
  SemJogoMedido: { texto: "Sem jogo aberto, a escolha se apoia só na rajada e nos contadores. Meça com o jogo para decidir de verdade.", tom: "aviso" },
  TemperaturaIndisponivel: {
    texto: "Este Windows não expõe temperatura real do processador (ou só expõe uma zona ACPI parada). A folga térmica ficou fora da nota.",
    tom: "neutro",
  },
  NenhumGanhouComMargem: { texto: "Nenhum candidato ganhou do padrão do Windows com margem. A recomendação é o padrão do Windows.", tom: "neutro" },
  EmpateDesfeitoPelaTemperatura: { texto: "Houve empate de desempenho; ganhou o que esquenta menos.", tom: "ok" },
  PoucasRepeticoes: { texto: "Só uma medição por candidato. Com duas ou mais, a confiança sobe.", tom: "aviso" },
  MedicaoInstavel: { texto: "O FPS variou muito entre repetições. Repita numa cena mais estável.", tom: "aviso" },
  PlanoAtualJaEOMelhor: {
    texto: "O plano que você já usa não perdeu para nenhum candidato em FPS nem em 1% low. Ele fica — trocar seria tirar FPS.",
    tom: "ok",
  },
  CandidatoTirariaFps: {
    texto: "Pelo menos um candidato foi descartado por dar menos FPS ou pior 1% low. Nenhum plano que tire FPS é recomendado.",
    tom: "neutro",
  },
  QuadrosSinteticos: {
    texto: "Sem jogo aberto, o FPS veio do teste de quadros do Otimiza (trabalho de processador com esperas curtas, como a thread principal de um jogo). Para a palavra final, meça com o jogo.",
    tom: "neutro",
  },
};

const ROTULO_DO_PAPEL = (p: Papel, notebook: boolean): string =>
  ({
    PadraoWindows: "Padrão do Windows",
    A: notebook ? "A · Tomada competitivo" : "A · Resposta máxima",
    B: notebook ? "B · Tomada equilibrado" : "B · Adaptativo",
    C: "C · Autonomia do hardware",
    D: "D · Resposta máxima (tudo no máximo)",
    Refino: "Refino do vencedor",
    Epp: "Laboratório de EPP",
    Dispositivo: "Política de dispositivo",
  })[p];

const esc = (s: string) =>
  s.replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]!);
const num = (n: number, casas = 0) =>
  n.toLocaleString("pt-BR", { minimumFractionDigits: casas, maximumFractionDigits: casas });
const pct = (n: number | null, invertido = false) => {
  if (n === null) return `<span class="fg-desconhecido">NÃO MEDIDO</span>`;
  const bom = invertido ? n < 0 : n > 0;
  return `<b class="en-delta" data-bom="${bom}">${n > 0 ? "+" : ""}${num(n, 1)}%</b>`;
};
const chip = (texto: string, tom: Tom) => `<span class="fg-chip" data-tom="${tom}">${esc(texto)}</span>`;

function painel(titulo: string, tag: string, corpo: string) {
  return `<section class="panel fg-painel">
    <div class="panel-head"><h2>${titulo}</h2>${tag ? `<span class="panel-tag">${tag}</span>` : ""}</div>
    ${corpo}
  </section>`;
}

function descreverParametros(p: Parametros): string {
  const partes: string[] = [];
  partes.push(p.epp_bruto === null ? "EPP do Windows" : `EPP ${p.epp_bruto} (${Math.round((p.epp_bruto * 100) / 255)}%)`);
  partes.push(`núcleos: ${ESTACIONAMENTO[p.estacionamento]}`);
  if (p.resposta === "Rapida") partes.push("resposta rápida");
  if (p.autonomia === "Hardware") partes.push("autonomia do hardware");
  if (p.boost === "Agressivo") partes.push("boost agressivo");
  if (p.minimo !== null) partes.push(`mínimo ${p.minimo}%`);
  if (p.pcie === "Desligada") partes.push("ASPM desligado");
  if (p.usb === "Desligada") partes.push("suspensão USB desligada");
  return partes.join(" · ");
}

const estado = {
  painel: null as Painel | null,
  erro: "",
  aviso: "",
  processo: "",
  segundos: 20,
  repeticoes: 2,
  rodando: "" as string,
  progresso: "",
  resultados: {} as Record<string, Medicao>,
  escolha: null as Escolha | null,
  ultimaMedicao: null as Medicao | null,
  bateriaTestada: "autoajuste" as "autoajuste" | "epp" | "dispositivos",
  /** Candidatos do refino (segunda etapa), gerados a partir do vencedor. */
  refino: [] as Candidato[],
};

let raiz: HTMLElement;
let pedirAdmin: (motivo: string) => void = () => {};

const notebook = () => estado.painel?.impressao.formato === "Notebook";

function candidatosDaBateria(): Candidato[] {
  const b = estado.painel?.bateria;
  if (!b) return [];
  if (estado.bateriaTestada === "epp") return [b.autoajuste[0], ...b.escada_de_epp].filter(Boolean);
  if (estado.bateriaTestada === "dispositivos") return [b.autoajuste[2], ...b.dispositivos].filter(Boolean);
  return b.autoajuste;
}

function todosOsCandidatos(): Candidato[] {
  const b = estado.painel?.bateria;
  return b ? [...b.autoajuste, ...b.escada_de_epp, ...b.dispositivos, ...estado.refino] : [];
}

async function carregar() {
  try {
    estado.painel = await invoke<Painel>("energia_painel");
    estado.erro = "";
  } catch (e) {
    estado.erro = String(e);
  }
  desenhar();
}

async function medirAgora() {
  estado.rodando = "medir";
  estado.progresso = "Medindo a resposta da CPU: 5 rajadas curtas com a máquina ociosa entre elas…";
  desenhar();
  try {
    estado.ultimaMedicao = await invoke<Medicao>("energia_medir_atual", { processo: null, segundos: 10, repeticoes: 1 });
  } catch (e) {
    estado.erro = String(e);
  }
  estado.rodando = "";
  desenhar();
}

async function testarBateria() {
  if (!estado.painel?.elevado) {
    pedirAdmin("Testar planos de energia exige permissão de administrador. Podemos reabrir o Otimiza com essa permissão?");
    return;
  }
  const lista = candidatosDaBateria();
  const base = lista[0];
  if (!base) return;
  estado.resultados = {};
  estado.escolha = null;
  estado.refino = [];
  estado.erro = "";
  estado.rodando = "bateria";

  // CURRENT PLAN: o plano que a pessoa usa hoje entra na comparação, medido
  // antes de qualquer escrita.
  estado.progresso = `Medindo o plano atual (${esc(estado.painel?.impressao.plano_ativo_nome ?? "")}) antes de mudar qualquer coisa…`;
  desenhar();
  try {
    const atual = await invoke<Medicao>("energia_medir_atual", {
      processo: estado.processo || null,
      segundos: estado.segundos,
      repeticoes: estado.repeticoes,
    });
    atual.resultado.candidato = "atual";
    estado.resultados.atual = atual;
  } catch (e) {
    estado.erro = String(e);
    estado.rodando = "";
    desenhar();
    return;
  }

  for (const [i, c] of lista.entries()) {
    estado.progresso = `Candidato ${i + 1} de ${lista.length}: ${esc(ROTULO_DO_PAPEL(c.papel, notebook()))}${
      c.papel === "Epp" || c.papel === "Dispositivo" ? ` (${esc(descreverParametros(c.parametros))})` : ""
    }. ${estado.processo ? "Continue jogando na mesma cena." : "Deixe o PC parado."}`;
    desenhar();
    try {
      const m = await invoke<Medicao>("energia_testar_candidato", {
        candidato: c,
        processo: estado.processo || null,
        segundos: estado.segundos,
        repeticoes: estado.repeticoes,
      });
      estado.resultados[c.id] = m;
      estado.ultimaMedicao = m;
    } catch (e) {
      estado.erro = `Parou no candidato ${c.id}: ${e}`;
      break;
    }
  }

  const resultados = Object.values(estado.resultados).map((m) => m.resultado);
  if (resultados.length) {
    estado.escolha = await invoke<Escolha | null>("energia_escolher", { resultados, base: base.id });
  }

  // ETAPA 2 — REFINO: os vizinhos do vencedor, medidos na mesma cena.
  estado.refino = [];
  const primeiro = vencedor();
  if (!estado.erro && primeiro && estado.bateriaTestada === "autoajuste") {
    try {
      estado.refino = await invoke<Candidato[]>("energia_vizinhos", { parametros: primeiro.parametros });
    } catch {
      estado.refino = [];
    }
    for (const [i, c] of estado.refino.entries()) {
      estado.progresso = `Refino ${i + 1} de ${estado.refino.length} em volta do vencedor: ${esc(descreverParametros(c.parametros))}. ${estado.processo ? "Continue na mesma cena." : "Deixe o PC parado."}`;
      desenhar();
      try {
        const m = await invoke<Medicao>("energia_testar_candidato", {
          candidato: c,
          processo: estado.processo || null,
          segundos: estado.segundos,
          repeticoes: estado.repeticoes,
        });
        estado.resultados[c.id] = m;
        estado.ultimaMedicao = m;
      } catch (e) {
        estado.erro = `Parou no refino ${c.id}: ${e}`;
        break;
      }
    }
    const todos = Object.values(estado.resultados).map((m) => m.resultado);
    estado.escolha = await invoke<Escolha | null>("energia_escolher", { resultados: todos, base: base.id });
  }

  // A bateria não deixa a máquina no último candidato testado: volta ao plano
  // de antes, e o vencedor só entra quando a pessoa aplicar.
  estado.progresso = "Voltando ao plano de antes do teste…";
  desenhar();
  try {
    await invoke<Restauracao>("energia_restaurar_anterior");
  } catch (e) {
    estado.aviso = `Não consegui voltar ao plano anterior sozinho: ${e}`;
  }
  estado.rodando = "";
  await carregar();
}

function vencedor(): Candidato | null {
  const id = estado.escolha?.vencedor;
  return id ? (todosOsCandidatos().find((c) => c.id === id) ?? null) : null;
}

async function aplicarVencedor() {
  const v = vencedor();
  if (!v) return;
  estado.rodando = "aplicar";
  desenhar();
  try {
    const a = await invoke<Aplicacao>("energia_aplicar", { parametros: v.parametros });
    estado.aviso = a.ativo
      ? `Perfil aplicado no plano OTIMIZA e conferido relendo do Windows${a.divergentes.length ? ` — exceto: ${a.divergentes.join(", ")}` : ""}.`
      : "Os valores foram escritos, mas o plano OTIMIZA não ficou ativo.";
  } catch (e) {
    estado.erro = String(e);
  }
  estado.rodando = "";
  await carregar();
}

async function salvarParaOJogo() {
  const v = vencedor();
  const exe = estado.processo.trim();
  if (!v || !exe) return;
  try {
    await invoke("energia_salvar_perfil_de_jogo", { executavel: exe, parametros: v.parametros, escolha: estado.escolha });
    estado.aviso = `Perfil guardado para "${exe}".`;
  } catch (e) {
    estado.erro = String(e);
  }
  await carregar();
}

async function removerPerfil(exe: string) {
  await invoke("energia_remover_perfil_de_jogo", { executavel: exe }).catch((e) => (estado.erro = String(e)));
  await carregar();
}

async function alternarDinamico(ligado: boolean) {
  try {
    await invoke<boolean>("energia_modo_dinamico", { ligado });
  } catch (e) {
    estado.erro = String(e);
  }
  await carregar();
}

async function restaurar(qual: "anterior" | "windows") {
  estado.rodando = "restaurar";
  desenhar();
  try {
    const r = await invoke<Restauracao>(qual === "anterior" ? "energia_restaurar_anterior" : "energia_restaurar_windows");
    estado.aviso =
      qual === "anterior"
        ? r.divergentes.length
          ? `Plano anterior ativo, mas ${r.divergentes.length} valor(es) não batem com o backup: ${r.divergentes.join(", ")}.`
          : "Plano anterior ativo, e cada valor relido bate com o backup."
        : "Plano Equilibrado do Windows ativo, e o plano OTIMIZA foi apagado.";
  } catch (e) {
    estado.erro = String(e);
  }
  estado.rodando = "";
  await carregar();
}

function desenharPrincipio() {
  return painel(
    "Motor de energia adaptativo",
    "mede antes de escolher",
    `<p class="fg-lead">Não existe plano de energia ideal para todos os PCs. O motor lê esta CPU, enumera o que este Windows expõe e testa candidatos feitos para esta arquitetura. Ganha o plano que entrega <strong>mais desempenho sustentado, melhor 1% low, melhor resposta e menos regressão térmica</strong> — não o mais agressivo.</p>
    <p class="fg-nota">Nada é universal: EPP 0, mínimo 100%, núcleos todos acordados e boost agressivo só aparecem como candidatos medidos, e só onde fazem sentido. O plano do cliente nunca é escrito; o motor trabalha dentro do plano OTIMIZA e guarda um backup exato do anterior.</p>`,
  );
}

function desenharImpressao() {
  const p = estado.painel;
  if (!p) return painel("Hardware fingerprint", "", `<p class="fg-nota">Lendo a máquina…</p>`);
  const i = p.impressao;
  const classes = i.topologia.classes;
  const pe =
    classes.length > 1
      ? (() => {
          const maior = Math.max(...classes.map(([c]) => c));
          const P = classes.filter(([c]) => c === maior).reduce((s, [, n]) => s + n, 0);
          return `${P} P + ${i.topologia.nucleos_fisicos - P} E`;
        })()
      : "sem classes (todos iguais)";
  const epp = p.ppm_atual.find((c) => c.alias === "PERFEPP");
  const autonomo = p.ppm_atual.find((c) => c.alias === "PERFAUTONOMOUS");
  const itens: [string, string][] = [
    ["CPU", esc(i.cpu)],
    ["Arquitetura", `${ARQUITETURA[i.arquitetura]}<small>família ${i.cpuid.familia.toString(16).toUpperCase()}h · modelo ${i.cpuid.modelo.toString(16).toUpperCase()}h</small>`],
    ["Quem escolhe a frequência", CONTROLE[i.controle]],
    ["Núcleos", `${i.topologia.nucleos_fisicos} físicos · ${i.topologia.processadores_logicos} lógicos<small>${pe} · SMT ${i.topologia.processadores_logicos > i.topologia.nucleos_fisicos ? "ligado" : "desligado"}</small>`],
    ["Recursos da CPU", [i.cpuid.hwp ? "HWP" : "", i.cpuid.hwp_epp ? "EPP" : "", i.cpuid.cppc ? "CPPC" : "", i.cpuid.nucleos_preferidos_intel ? "núcleos preferidos" : ""].filter(Boolean).join(" · ") || "nenhum dos recursos de controle por hardware"],
    ["EPP no plano ativo", epp ? `${epp.ac ?? "—"}% na tomada · ${epp.dc ?? "—"}% na bateria` : "não exposto"],
    ["Autonomia no plano ativo", autonomo ? (autonomo.ac === 1 ? "ligada" : "desligada") : "não exposta"],
    ["Formato", `${i.formato === "Notebook" ? "notebook" : "desktop"}<small>${i.na_tomada === null ? "alimentação desconhecida" : i.na_tomada ? "na tomada" : "na bateria"}${i.modern_standby ? " · Modern Standby" : ""}</small>`],
    ["Windows", `build ${i.build_do_windows}`],
    ["Plano ativo", `${esc(i.plano_ativo_nome ?? "desconhecido")}${p.plano_otimiza_ativo ? " " + chip("do motor", "ok") : ""}`],
  ];
  return painel(
    "Hardware fingerprint",
    ARQUITETURA[i.arquitetura],
    `<dl class="fg-grade">${itens.map(([k, v]) => `<div><dt>${k}</dt><dd>${v}</dd></div>`).join("")}</dl>
    ${i.formato === "Notebook" ? `<div class="fg-aviso"><b>Notebook.</b> Os candidatos mexem só no lado da tomada. A bateria fica exatamente como o Windows deixou (BATTERY SAFE) — perfil de desktop nunca vai para a bateria.</div>` : ""}
    ${p.elevado ? "" : `<p class="fg-nota" data-tom="aviso">Sem permissão de administrador: dá para ler, mas não para testar nem aplicar.</p>`}`,
  );
}

function curva(pontos: number[]) {
  if (pontos.length < 2) return "";
  const w = 600;
  const h = 110;
  const max = Math.max(110, ...pontos);
  const linha = pontos
    .map((v, i) => `${((i / (pontos.length - 1)) * w).toFixed(1)},${(h - (Math.min(v, max) / max) * h).toFixed(1)}`)
    .join(" ");
  const y90 = h - (90 / max) * h;
  return `<figure class="fg-grafico">
    <svg viewBox="0 0 ${w} ${h}" preserveAspectRatio="none" aria-label="Trabalho entregue ao longo da rajada">
      <line x1="0" x2="${w}" y1="${y90.toFixed(1)}" y2="${y90.toFixed(1)}" class="fg-grafico-media" />
      <polyline points="${linha}" class="fg-grafico-linha" />
    </svg>
    <figcaption>Trabalho entregue em cada fatia de 2 ms, em % do sustentado · linha tracejada = 90%</figcaption>
  </figure>`;
}

function desenharMapaDeResposta() {
  const m = estado.ultimaMedicao;
  const r = m?.resultado;
  const epp = estado.painel?.ppm_atual.find((c) => c.alias === "PERFEPP");
  const folga = (() => {
    if (!r) return null;
    const t = r.cpu.temperatura_max_c;
    if (t === null) return r.cpu.limite_termico ? 10 : null;
    const v = Math.max(0, Math.min(100, ((95 - t) / 50) * 100));
    return r.cpu.limite_termico ? Math.min(v, 20) : Math.round(v);
  })();

  const metrica = (rotulo: string, valor: string, detalhe = "") =>
    `<div class="en-metrica"><span>${rotulo}</span><b>${valor}</b>${detalhe ? `<small>${detalhe}</small>` : ""}</div>`;

  const etapas = [
    ["IDLE", "ocioso entre rajadas"],
    ["LOAD DETECTED", "a carga começa"],
    ["BOOST", r?.resposta ? `90% em ${num(r.resposta.ate_90_ms, 0)} ms` : "—"],
    ["SUSTAINED", r ? `${num(r.cpu.clock_efetivo_mhz)} MHz efetivos` : "—"],
    ["RECOVERY", "volta a ociosar"],
  ];

  return painel(
    "CPU response map",
    r ? "medido agora" : "",
    `<div class="en-mapa">${etapas.map(([t, d], i) => `<div class="en-etapa" data-i="${i}"><b>${t}</b><span>${d}</span></div>`).join('<i aria-hidden="true">→</i>')}</div>
    <div class="en-metricas">
      ${metrica("BOOST RESPONSE", r?.resposta ? `${num(r.resposta.ate_90_ms, 0)} ms` : "—", r?.resposta ? `1ª fatia entrega ${num(r.resposta.primeira_fatia_pct)}% · ${r.resposta.rajadas} rajadas` : "")}
      ${metrica("THERMAL HEADROOM", folga === null ? "DESCONHECIDO" : `${folga}/100`, r?.cpu.temperatura_max_c != null ? `máx. ${num(r.cpu.temperatura_max_c)} °C (zona térmica do Windows)` : "sem sensor utilizável")}
      ${metrica("POWER LIMIT", r ? (r.cpu.limite_termico || r.cpu.limite_eletrico ? "LIMITADO" : "livre") : "—", r ? `limitado em ${num(r.cpu.tempo_limitado_pct)}% das amostras${r.cpu.limite_termico ? " · térmico" : ""}${r.cpu.limite_eletrico ? " · energia" : ""}` : "")}
      ${metrica("EPP", epp?.ac != null ? `${epp.ac}%` : "não exposto", epp?.ac != null ? `≈ ${Math.round((epp.ac * 255) / 100)} na escala da CPU` : "")}
      ${metrica("CORE ACTIVITY", r?.cpu.nucleos_acordados_medio != null ? `${num(r.cpu.nucleos_acordados_medio, 1)} de ${r.cpu.nucleos_total}` : "—", "processadores lógicos acordados, em média")}
      ${metrica("EFFECTIVE CLOCK", r ? `${num(r.cpu.clock_efetivo_mhz)} MHz` : "—", r ? `reportado ${num(r.cpu.clock_reportado_mhz)} MHz` : "")}
    </div>
    ${m ? curva(m.curva_pct) : ""}
    <button class="btn" data-acao="medir" ${estado.rodando ? "disabled" : ""}>Medir a resposta do plano ativo</button>`,
  );
}

function desenharAutoajuste() {
  const p = estado.painel;
  if (!p) return "";
  const lista = candidatosDaBateria();
  const cards = lista
    .map((c) => {
      const medido = estado.resultados[c.id];
      return `<div class="en-candidato" data-vencedor="${estado.escolha?.vencedor === c.id}">
        <div class="en-candidato-topo"><b>${ROTULO_DO_PAPEL(c.papel, notebook())}</b>${medido ? chip("medido", "ok") : ""}</div>
        <p>${esc(descreverParametros(c.parametros))}</p>
        <details><summary>${c.mudancas.length} ajuste(s)${c.ignorados.length ? ` · ${c.ignorados.length} ignorado(s)` : ""}</summary>
          <ul>${c.mudancas.map((m) => `<li><code>${esc(m.alias)}</code> → ${m.ac ?? "—"}${m.dc === null ? " (bateria intacta)" : ""} <small>${MOTIVO[m.motivo]}</small></li>`).join("")}
          ${c.ignorados.map((g) => `<li class="en-ignorado"><code>${esc(g.alias)}</code> <small>${IGNORADO[g.motivo]}</small></li>`).join("")}</ul>
        </details>
      </div>`;
    })
    .join("");

  const baterias = (
    [
      ["autoajuste", "Autoajuste (A · B · C)"],
      ["epp", "Laboratório de EPP"],
      ["dispositivos", "PCIe e USB"],
    ] as const
  )
    .filter(([id]) => id !== "epp" || p.bateria.escada_de_epp.length)
    .map(([id, rotulo]) => `<button class="fg-segmento" data-bateria="${id}" aria-pressed="${estado.bateriaTestada === id}">${rotulo}</button>`)
    .join("");

  const duracoes = [10, 20, 30, 60].map((s) => `<option value="${s}" ${estado.segundos === s ? "selected" : ""}>${s} s</option>`).join("");
  const reps = [1, 2, 3].map((n) => `<option value="${n}" ${estado.repeticoes === n ? "selected" : ""}>${n}×</option>`).join("");
  // + 1 do plano atual, + até 3 do refino.
  const minutos = Math.ceil(((lista.length + 4) * (15 + estado.segundos * estado.repeticoes)) / 60);

  const explicacao =
    estado.bateriaTestada === "epp"
      ? "Mesma resposta e mesmo estacionamento; só o EPP varia (0, 16, 32, 64, 128 na escala da CPU). Mede FPS, 1% low, tempo de quadro, clock efetivo, resposta e temperatura quando exposta."
      : estado.bateriaTestada === "dispositivos"
        ? "ASPM (PCI Express) e suspensão seletiva do USB, cada um desligado sozinho sobre o candidato B. Nunca entram por padrão: só ficam se a medição mostrar ganho. A política de USB por dispositivo, ligada ao AimTracking, ainda não existe — aqui é o controlador inteiro."
        : "Seu plano atual, o padrão do Windows e candidatos feitos para esta arquitetura. Cada um é aplicado e medido com rajadas de carga e com quadros — do jogo, se ele estiver aberto, ou do teste de quadros do Otimiza. Nenhum plano que dê menos FPS ou pior 1% low que o seu plano atual é recomendado.";

  return painel(
    "Autoajuste",
    `≈ ${minutos} min`,
    `<div class="fg-segmentos">${baterias}</div>
    <p class="fg-nota">${explicacao}</p>
    <div class="fg-linha">
      <label class="fg-campo">Jogo aberto (opcional)<input id="en-processo" value="${esc(estado.processo)}" placeholder="ex.: fivem" spellcheck="false" /></label>
      <label class="fg-campo">Por medição<select id="en-segundos">${duracoes}</select></label>
      <label class="fg-campo">Repetições<select id="en-reps">${reps}</select></label>
      <button class="btn btn-primary" data-acao="testar" ${estado.rodando ? "disabled" : ""}>Testar ${lista.length} candidatos</button>
    </div>
    ${estado.rodando ? `<div class="fg-progresso"><div class="fg-progresso-trilho"><div class="fg-progresso-barra en-indeterminada"></div></div><span class="fg-progresso-texto">${estado.progresso}</span></div>` : ""}
    <div class="en-candidatos">${cards}</div>
    <p class="fg-nota">Ao terminar, a máquina volta ao plano de antes do teste. O vencedor só entra quando você aplicar.</p>`,
  );
}

function desenharResultados() {
  const lista = [...candidatosDaBateria(), ...estado.refino].filter((c) => estado.resultados[c.id]);
  if (!lista.length) return "";
  const notas = new Map(estado.escolha?.notas ?? []);
  const atual: Candidato | null = estado.resultados.atual
    ? { id: "atual", papel: "PadraoWindows", parametros: { epp_bruto: null, estacionamento: "Windows", boost: "Windows", autonomia: "Windows", resposta: "Windows", minimo: null, pcie: "Windows", usb: "Windows" }, mudancas: [], ignorados: [] }
    : null;
  const linhas = [...(atual ? [atual] : []), ...lista]
    .map((c) => {
      const r = estado.resultados[c.id].resultado;
      const q = r.quadros;
      const n = notas.get(c.id);
      return `<tr ${estado.escolha?.vencedor === c.id ? 'data-selecionada="true"' : ""}>
        <td>${c.id === "atual" ? `Plano atual<small>${esc(estado.painel?.impressao.plano_ativo_nome ?? "")} · sem mudança nenhuma</small>` : `${ROTULO_DO_PAPEL(c.papel, notebook())}<small>${esc(descreverParametros(c.parametros))}</small>`}</td>
        <td class="fg-mono">${n ? num(n.total, 1) : "—"}</td>
        <td class="fg-mono">${q ? num(q.fps, 1) : "—"}${q && r.quadros_sinteticos ? "<small>teste de quadros</small>" : ""}</td>
        <td class="fg-mono">${q ? num(q.low_1, 1) : "—"}</td>
        <td class="fg-mono">${q ? num(q.low_01, 1) : "—"}</td>
        <td class="fg-mono">${q ? `${num(q.p99_ms, 1)} ms` : "—"}</td>
        <td class="fg-mono">${r.resposta ? `${num(r.resposta.ate_90_ms, 0)} ms` : "—"}</td>
        <td class="fg-mono">${num(r.cpu.clock_efetivo_mhz)}<small>rep. ${num(r.cpu.clock_reportado_mhz)}</small></td>
        <td class="fg-mono">${r.cpu.temperatura_max_c === null ? "—" : `${num(r.cpu.temperatura_max_c)} °C`}</td>
        <td class="fg-mono">${num(r.cpu.tempo_limitado_pct)}%</td>
        <td class="fg-mono">${r.gpu_pct === null ? "—" : `${num(r.gpu_pct)}%`}</td>
      </tr>`;
    })
    .join("");
  return painel(
    "Power benchmark",
    "Windows · atual · adaptativo",
    `<div class="en-rolagem"><table class="fg-tabela"><thead><tr><th>Candidato</th><th>Nota</th><th>FPS</th><th>1% low</th><th>0,1% low</th><th>P99</th><th>Resposta</th><th>Clock efetivo</th><th>Temp.</th><th>Limitado</th><th>GPU</th></tr></thead><tbody>${linhas}</tbody></table></div>
    <p class="fg-nota">Nota 100 = igual ao padrão do Windows. 1% low pesa 25%, P99 15%, resposta 15%, FPS médio só 15%; clock efetivo, folga térmica e tempo sob limite de firmware completam. "Energia do pacote" e PL1/PL2 não aparecem: o Windows não expõe sem driver de terceiros, e o Otimiza não instala um.</p>`,
  );
}

function desenharPerfil() {
  const e = estado.escolha;
  if (!e) return "";
  const v = vencedor();
  const i = estado.painel?.impressao;
  const achados = e.achados.map((a) => `<p class="fg-nota" data-tom="${ACHADO[a].tom === "ok" ? "" : ACHADO[a].tom}">${ACHADO[a].texto}</p>`).join("");

  if (e.vencedor === "atual") {
    return painel(
      "Adaptive power profile",
      `confiança ${num(e.confianca_pct)}%`,
      `<div class="fg-decisao" data-tom="ok"><div><span>RESULTADO</span><b>Seu plano atual</b></div><strong>JÁ É O MELHOR AQUI</strong></div>
      <p class="fg-nota">O plano que você já usa ganhou dos candidatos nesta medição. Nada a aplicar.</p>${achados}`,
    );
  }

  if (!v) {
    return painel(
      "Adaptive power profile",
      `confiança ${num(e.confianca_pct)}%`,
      `<div class="fg-decisao" data-tom="neutro"><div><span>RESULTADO</span><b>Padrão do Windows</b></div><strong>NENHUM CANDIDATO GANHOU</strong></div>${achados}`,
    );
  }

  const epp = v.parametros.epp_bruto;
  const linhas: [string, string][] = [
    ["CPU", esc(i?.cpu ?? "")],
    ["EPP", epp === null ? "do Windows" : `${epp} (${Math.round((epp * 100) / 255)}%)`],
    ["Core parking", ESTACIONAMENTO[v.parametros.estacionamento]],
    ["Boost policy", v.parametros.boost === "Agressivo" ? "agressivo (medido)" : "do Windows"],
    ["Autonomous mode", v.parametros.autonomia === "Hardware" ? "ligado" : "do Windows"],
    ["Resposta", v.parametros.resposta === "Rapida" ? "subida rápida, descida em degraus" : "do Windows"],
  ];
  return painel(
    "Adaptive power profile",
    ROTULO_DO_PAPEL(v.papel, notebook()),
    `<div class="fg-decisao" data-tom="ok"><div><span>CONFIDENCE</span><b>${num(e.confianca_pct)}%</b></div><strong>${ROTULO_DO_PAPEL(v.papel, notebook()).toUpperCase()}</strong></div>
    <dl class="fg-grade">${linhas.map(([k, val]) => `<div><dt>${k}</dt><dd>${val}</dd></div>`).join("")}</dl>
    <div class="fg-antes-depois">
      <div><span>FPS médio</span><b>${pct(e.fps_pct)}</b></div>
      <div><span>1% low</span><b>${pct(e.low_1_pct)}</b></div>
      <div><span>P99 (tempo de quadro)</span><b>${pct(e.p99_pct, true)}</b></div>
      <div><span>Tempo de resposta</span><b>${pct(e.resposta_pct, true)}</b></div>
      <div><span>Temperatura</span><b>${e.temperatura_delta_c === null ? `<span class="fg-desconhecido">NÃO MEDIDO</span>` : `${e.temperatura_delta_c > 0 ? "+" : ""}${num(e.temperatura_delta_c)} °C`}</b></div>
    </div>
    ${achados}
    <div class="fg-linha">
      <button class="btn btn-primary" data-acao="aplicar" ${estado.rodando ? "disabled" : ""}>Aplicar este perfil</button>
      <button class="btn" data-acao="salvar-jogo" ${estado.processo ? "" : "disabled"}>Guardar para ${estado.processo ? esc(estado.processo) : "o jogo"}</button>
    </div>`,
  );
}

function desenharDinamico() {
  const p = estado.painel;
  if (!p) return "";
  const perfis = p.perfis_de_jogo
    .map(
      (g) => `<tr><td><b>${esc(g.executavel)}</b><small>${esc(descreverParametros(g.parametros))}</small></td>
      <td class="fg-mono">${g.escolha ? `${num(g.escolha.confianca_pct)}%` : "—"}</td>
      <td class="fg-mono">${g.escolha?.low_1_pct != null ? `${g.escolha.low_1_pct > 0 ? "+" : ""}${num(g.escolha.low_1_pct, 1)}%` : "—"}</td>
      <td><button class="btn btn-ghost" data-remover="${esc(g.executavel)}">remover</button></td></tr>`,
    )
    .join("");
  return painel(
    "Modo dinâmico de jogo",
    p.dinamico ? "ligado" : "desligado",
    `<div class="en-fluxo"><span>NORMAL</span>→<span>JOGO ABRIU</span>→<span data-forte="true">PERFIL DE BAIXA LATÊNCIA</span>→<span>JOGO FECHOU</span>→<span>NORMAL</span></div>
    <p class="fg-nota">Com o modo ligado, o Otimiza olha a cada 3 segundos só os executáveis que têm perfil guardado. Quando um abre, aplica o perfil daquele jogo; quando fecha (duas olhadas seguidas), volta ao plano normal. A máquina não fica em modo agressivo o dia inteiro.</p>
    ${perfis ? `<table class="fg-tabela"><thead><tr><th>Jogo</th><th>Confiança</th><th>1% low</th><th></th></tr></thead><tbody>${perfis}</tbody></table>` : `<p class="fg-nota">Nenhum jogo com perfil ainda. Rode o autoajuste com o jogo aberto e use "Guardar para o jogo".</p>`}
    <button class="btn" data-acao="dinamico" data-ligar="${!p.dinamico}" ${p.perfis_de_jogo.length || p.dinamico ? "" : "disabled"}>${p.dinamico ? "Desligar o modo dinâmico" : "Ligar o modo dinâmico"}</button>`,
  );
}

function valorLegivel(c: Configuracao | undefined, v: number | null | undefined) {
  if (v === null || v === undefined) return "—";
  const nome = c?.possiveis.find(([i]) => i === v)?.[1];
  return nome ? `${v} <small>${esc(nome)}</small>` : `${v}${c?.unidade && c.unidade.length <= 3 ? esc(c.unidade) : ""}`;
}

function desenharPpm() {
  const p = estado.painel;
  if (!p) return "";
  const v = vencedor() ?? p.bateria.autoajuste[2];
  const nota = v && estado.escolha ? new Map(estado.escolha.notas).get(v.id) : undefined;
  const linhas = p.ppm_atual
    .map((c) => {
      const m = v?.mudancas.find((x) => x.alias === c.alias);
      const recomendado = m ? valorLegivel(c, m.ac) : `<span class="fg-nota">mantém</span>`;
      const testado = m && nota ? `nota ${num(nota.total, 1)}` : "—";
      return `<tr><td><code>${esc(c.alias ?? c.guid)}</code><small>${c.minimo !== null ? `${c.minimo}–${c.maximo}` : `${c.possiveis.length} valores`}</small></td>
        <td class="fg-mono">${valorLegivel(c, c.ac)}</td><td class="fg-mono">${valorLegivel(c, c.dc)}</td>
        <td class="fg-mono">${recomendado}</td><td class="fg-mono">${testado}</td></tr>`;
    })
    .join("");
  return `<section class="panel fg-painel"><details class="en-avancado"><summary><b>PPM settings</b> <span class="fg-nota">${p.ppm_atual.length} ajustes do plano ativo · atual, recomendado e resultado testado</span></summary>
    <div class="en-rolagem"><table class="fg-tabela"><thead><tr><th>Ajuste</th><th>Atual (tomada)</th><th>Atual (bateria)</th><th>Recomendado</th><th>Testado</th></tr></thead><tbody>${linhas}</tbody></table></div>
    <p class="fg-nota">"Recomendado" é o vencedor medido; sem teste, é o candidato B, marcado como não testado. Valores fora desta lista não são tocados.</p>
  </details></section>`;
}

function desenharVolta() {
  const p = estado.painel;
  if (!p) return "";
  return painel(
    "Voltar",
    "",
    `<p class="fg-nota">${
      p.backup
        ? `Backup do plano anterior: <b>${esc(p.backup.nome_anterior ?? p.backup.plano_anterior)}</b>, com todos os valores guardados. Restaurar reativa esse plano e confere valor por valor.`
        : "Não há backup: o motor ainda não mudou o plano desta máquina."
    }</p>
    <div class="fg-linha">
      <button class="btn" data-acao="restaurar-anterior" ${p.backup && !estado.rodando ? "" : "disabled"}>Restaurar plano anterior</button>
      <button class="btn" data-acao="restaurar-windows" ${estado.rodando ? "disabled" : ""}>Restaurar padrão do Windows</button>
    </div>`,
  );
}

function desenhar() {
  if (!raiz) return;
  raiz.innerHTML = [
    estado.erro ? `<div class="fg-erro" role="alert">${esc(estado.erro)}</div>` : "",
    estado.aviso ? `<div class="fg-aviso" role="status">${esc(estado.aviso)}</div>` : "",
    estado.painel?.teste_interrompido && !estado.rodando
      ? `<div class="fg-erro" role="alert">Um autoajuste foi interrompido no meio e o computador pode ter ficado num plano de teste. Abra o Otimiza como administrador ou clique em "Restaurar plano anterior" lá embaixo.</div>`
      : "",
    desenharPrincipio(),
    desenharImpressao(),
    desenharMapaDeResposta(),
    desenharAutoajuste(),
    desenharResultados(),
    desenharPerfil(),
    desenharDinamico(),
    desenharPpm(),
    desenharVolta(),
  ].join("");
}

function ligarEventos() {
  raiz.addEventListener("click", async (ev) => {
    const b = (ev.target as HTMLElement).closest<HTMLButtonElement>("button");
    if (!b) return;
    const acao = b.dataset.acao;
    if (acao === "medir") await medirAgora();
    else if (acao === "testar") await testarBateria();
    else if (acao === "aplicar") await aplicarVencedor();
    else if (acao === "salvar-jogo") await salvarParaOJogo();
    else if (acao === "dinamico") await alternarDinamico(b.dataset.ligar === "true");
    else if (acao === "restaurar-anterior") await restaurar("anterior");
    else if (acao === "restaurar-windows") await restaurar("windows");
    else if (b.dataset.remover) await removerPerfil(b.dataset.remover);
    else if (b.dataset.bateria) {
      estado.bateriaTestada = b.dataset.bateria as typeof estado.bateriaTestada;
      estado.resultados = {};
      estado.escolha = null;
      desenhar();
    }
  });

  raiz.addEventListener("change", (ev) => {
    const alvo = ev.target as HTMLInputElement | HTMLSelectElement;
    if (alvo.id === "en-processo") estado.processo = alvo.value.trim();
    else if (alvo.id === "en-segundos") estado.segundos = Number(alvo.value);
    else if (alvo.id === "en-reps") estado.repeticoes = Number(alvo.value);
    desenhar();
  });
}

let carregado = false;

/** Uma vez só, ao abrir a aba: a leitura passa pelo PowerShell e pelo `powercfg`. */
export async function carregarMotorDeEnergia(opcoes: { pedirAdmin: (motivo: string) => void }) {
  if (carregado) return;
  carregado = true;
  pedirAdmin = opcoes.pedirAdmin;
  raiz = document.getElementById("energia-motor")!;
  if (!raiz) return;
  ligarEventos();
  desenhar();
  void listen<{ transicao: string; jogo: string | null }>("energia:dinamico", () => void carregar()).catch(() => {});
  await carregar();
}
