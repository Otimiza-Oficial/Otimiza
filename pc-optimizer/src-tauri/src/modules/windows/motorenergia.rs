// ---------------------------------------------------------------------------
// ADAPTIVE POWER ENGINE — o motor de energia adaptativo
//
// O PLANO ANTIGO ERA UMA LISTA FIXA, e é isso que este módulo substitui. Ele
// escrevia os mesmos números em todo PC: EPP 0, estado mínimo 100%, núcleos
// 100% acordados, boost agressivo. Num i3 de desktop isso é quase inofensivo;
// num notebook com Ryzen é temperatura a mais para o mesmo FPS, e num Intel
// híbrido é tirar do Windows a escolha entre núcleo P e núcleo E.
//
// NÃO EXISTE UM PLANO DE ENERGIA IDEAL PARA TODOS OS PCs. O motor faz outra
// coisa:
//
//   1. IDENTIFICA a CPU pela CPUID (família, modelo, híbrida, Speed Shift,
//      EPP, CPPC) e o formato da máquina;
//   2. ENUMERA o que este Windows expõe, lendo `powercfg /qh` — sem lista fixa
//      de GUIDs: ajuste que não aparece não existe, e valor fora da faixa que
//      o próprio Windows publica não é escrito;
//   3. GERA CANDIDATOS específicos da arquitetura, cada número com o motivo;
//   4. MEDE cada candidato: rajada de carga (quanto tempo a CPU leva para
//      entregar desempenho), clock efetivo, limites de firmware, temperatura
//      quando exposta, e o jogo quando estiver aberto;
//   5. ESCOLHE pelo resultado — 1% low, P99 e resposta pesam mais que FPS
//      médio, e regressão térmica conta contra.
//
// O objetivo não é o plano mais agressivo. É o que entrega mais desempenho
// SUSTENTADO com a menor regressão térmica. Se nenhum candidato ganha do
// padrão do Windows com margem, a resposta é o padrão do Windows.
//
// Este arquivo é só regra pura, testada sem máquina. O que lê e escreve no
// Windows mora em `motorenergia_maquina.rs`.
// ---------------------------------------------------------------------------

use serde::{Deserialize, Serialize};

use super::planoenergia::e_guid;

// ============================================================ identificação

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Fabricante {
    Intel,
    Amd,
    Outro,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Arquitetura {
    /// Sem Speed Shift (HWP): o Windows escolhe cada estado de desempenho.
    IntelLegada,
    /// Skylake em diante, com HWP e núcleos todos iguais.
    IntelModerna,
    /// Alder Lake em diante: núcleos P e E, com classes de eficiência.
    IntelHibrida,
    /// Zen e Zen+ (e anteriores ao Ryzen): CPPC limitado ou ausente.
    AmdLegada,
    Zen2,
    Zen3,
    Zen4,
    /// Zen 5 e posteriores (família 1Ah).
    Zen5Mais,
    Desconhecida,
}

/// Quem escolhe o estado de desempenho quando nada é forçado.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Controle {
    /// Intel Speed Shift: o processador escolhe, guiado pelo EPP.
    HardwareHwp,
    /// AMD CPPC: o processador escolhe, guiado pelo EPP.
    HardwareCppc,
    /// P-states clássicos, escolhidos pelo Windows.
    SistemaOperacional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Formato {
    Desktop,
    Notebook,
}

/// O que a instrução CPUID respondeu. Números, não nomes: iguais em qualquer
/// idioma e em qualquer versão do Windows.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Cpuid {
    pub fabricante_bruto: String,
    pub familia: u32,
    pub modelo: u32,
    /// Leaf 7, EDX bit 15.
    pub hibrido: bool,
    /// Leaf 6, EAX bit 7: Hardware P-States (Speed Shift).
    pub hwp: bool,
    /// Leaf 6, EAX bit 10: preferência de energia no HWP.
    pub hwp_epp: bool,
    /// Leaf 6, EAX bit 14: Turbo Boost Max 3.0 (núcleos preferidos Intel).
    pub nucleos_preferidos_intel: bool,
    /// Leaf 80000008h, EBX bit 27: CPPC da AMD.
    pub cppc: bool,
}

/// Família e modelo "de exibição", como a Intel e a AMD documentam.
pub fn familia_e_modelo(eax: u32) -> (u32, u32) {
    let base_familia = (eax >> 8) & 0xF;
    let base_modelo = (eax >> 4) & 0xF;
    let familia = if base_familia == 0xF {
        base_familia + ((eax >> 20) & 0xFF)
    } else {
        base_familia
    };
    let modelo = if base_familia == 0x6 || base_familia == 0xF {
        base_modelo | (((eax >> 16) & 0xF) << 4)
    } else {
        base_modelo
    };
    (familia, modelo)
}

pub fn fabricante_do_cpuid(bruto: &str) -> Fabricante {
    match bruto {
        "GenuineIntel" => Fabricante::Intel,
        "AuthenticAMD" => Fabricante::Amd,
        _ => Fabricante::Outro,
    }
}

pub fn classificar_arquitetura(c: &Cpuid) -> Arquitetura {
    match fabricante_do_cpuid(&c.fabricante_bruto) {
        Fabricante::Intel if c.hibrido => Arquitetura::IntelHibrida,
        Fabricante::Intel if c.hwp => Arquitetura::IntelModerna,
        Fabricante::Intel => Arquitetura::IntelLegada,
        Fabricante::Amd => match (c.familia, c.modelo) {
            (0x17, m) if m >= 0x30 => Arquitetura::Zen2,
            (0x17, _) => Arquitetura::AmdLegada,
            (0x19, m) if (0x10..=0x1F).contains(&m) || (0x60..=0x7F).contains(&m) || (0xA0..=0xAF).contains(&m) => {
                Arquitetura::Zen4
            }
            (0x19, _) => Arquitetura::Zen3,
            (f, _) if f >= 0x1A => Arquitetura::Zen5Mais,
            _ => Arquitetura::AmdLegada,
        },
        Fabricante::Outro => Arquitetura::Desconhecida,
    }
}

pub fn controle(c: &Cpuid, arquitetura: Arquitetura) -> Controle {
    match arquitetura {
        Arquitetura::IntelModerna | Arquitetura::IntelHibrida if c.hwp => Controle::HardwareHwp,
        Arquitetura::Zen2 | Arquitetura::Zen3 | Arquitetura::Zen4 | Arquitetura::Zen5Mais if c.cppc => {
            Controle::HardwareCppc
        }
        _ => Controle::SistemaOperacional,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Topologia {
    pub nucleos_fisicos: u32,
    pub processadores_logicos: u32,
    /// Núcleos físicos por classe de eficiência. Em CPU não híbrida, uma
    /// classe só. Na híbrida, a classe MAIOR é a dos núcleos de desempenho.
    pub classes: Vec<(u8, u32)>,
}

impl Topologia {
    pub fn smt(&self) -> bool {
        self.processadores_logicos > self.nucleos_fisicos && self.nucleos_fisicos > 0
    }

    pub fn nucleos_p_e(&self) -> Option<(u32, u32)> {
        if self.classes.len() < 2 {
            return None;
        }
        let maior = self.classes.iter().map(|(c, _)| *c).max()?;
        let p = self.classes.iter().filter(|(c, _)| *c == maior).map(|(_, n)| n).sum();
        let e = self.classes.iter().filter(|(c, _)| *c != maior).map(|(_, n)| n).sum();
        Some((p, e))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Impressao {
    pub cpu: String,
    pub fabricante: Fabricante,
    pub arquitetura: Arquitetura,
    pub controle: Controle,
    pub cpuid: Cpuid,
    pub topologia: Topologia,
    pub formato: Formato,
    pub na_tomada: Option<bool>,
    pub build_do_windows: u32,
    pub modern_standby: bool,
    pub plano_ativo: Option<String>,
    pub plano_ativo_nome: Option<String>,
}

// =========================================================== enumeração

/// Um ajuste de energia como ESTE Windows o descreve.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Configuracao {
    pub subgrupo: String,
    pub subgrupo_alias: Option<String>,
    pub guid: String,
    pub alias: Option<String>,
    pub minimo: Option<u32>,
    pub maximo: Option<u32>,
    pub incremento: Option<u32>,
    /// A unidade vem traduzida pelo Windows. Evidência, não decide nada.
    pub unidade: Option<String>,
    /// Índice e nome amigável (traduzido) de cada valor possível.
    pub possiveis: Vec<(u32, String)>,
    pub ac: Option<u32>,
    pub dc: Option<u32>,
}

impl Configuracao {
    /// O valor cabe no que o Windows publica para este ajuste?
    pub fn aceita(&self, valor: u32) -> bool {
        if !self.possiveis.is_empty() {
            return self.possiveis.iter().any(|(i, _)| *i == valor);
        }
        match (self.minimo, self.maximo) {
            (Some(min), Some(max)) => (min..=max).contains(&valor),
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Enumeracao {
    pub plano: Option<String>,
    pub configuracoes: Vec<Configuracao>,
}

impl Enumeracao {
    pub fn por_alias(&self, alias: &str) -> Option<&Configuracao> {
        self.configuracoes
            .iter()
            .find(|c| c.alias.as_deref().is_some_and(|a| a.eq_ignore_ascii_case(alias)))
    }
}

fn valor_da_linha(linha: &str) -> &str {
    linha.split_once(':').map(|(_, v)| v.trim()).unwrap_or("")
}

fn hexadecimal(texto: &str) -> Option<u32> {
    let t = texto.split_whitespace().next()?;
    u32::from_str_radix(t.strip_prefix("0x").or_else(|| t.strip_prefix("0X"))?, 16).ok()
}

/// Lê a saída de `powercfg /qh`.
///
/// NADA AQUI DEPENDE DO IDIOMA. Os rótulos vêm traduzidos ("Índice de
/// Configurações de Correntes Alternadas Atuais"), então o parser usa só o que
/// é igual em qualquer Windows: GUIDs, o apelido em maiúsculas, números em
/// hexadecimal, a indentação e a ORDEM em que o `powercfg` escreve — mínimo,
/// máximo, incremento; depois tomada; depois bateria.
pub fn enumerar(saida: &str) -> Enumeracao {
    let mut e = Enumeracao::default();
    let mut subgrupo = String::new();
    let mut subgrupo_alias: Option<String> = None;
    let mut atual: Option<Configuracao> = None;
    let mut hex_do_ajuste: Vec<u32> = Vec::new();
    let mut indice_pendente: Option<u32> = None;

    let fechar = |atual: &mut Option<Configuracao>, hex: &mut Vec<u32>, e: &mut Enumeracao| {
        if let Some(mut c) = atual.take() {
            if c.possiveis.is_empty() {
                c.minimo = hex.first().copied();
                c.maximo = hex.get(1).copied();
                c.incremento = hex.get(2).copied();
            }
            e.configuracoes.push(c);
        }
        hex.clear();
    };

    for bruta in saida.lines() {
        let linha = bruta.trim_end().trim_start_matches('\u{feff}');
        if linha.trim().is_empty() {
            continue;
        }
        let recuo = linha.len() - linha.trim_start().len();
        let texto = linha.trim_start();
        let guid = texto
            .split_whitespace()
            .map(|t| t.trim_matches(|c: char| c == '(' || c == ')' || c == ':'))
            .find(|t| e_guid(t))
            .map(|t| t.to_lowercase());

        if let Some(g) = guid {
            match recuo {
                0 => {
                    fechar(&mut atual, &mut hex_do_ajuste, &mut e);
                    e.plano = Some(g);
                }
                1..=3 => {
                    fechar(&mut atual, &mut hex_do_ajuste, &mut e);
                    subgrupo = g;
                    subgrupo_alias = None;
                }
                _ => {
                    fechar(&mut atual, &mut hex_do_ajuste, &mut e);
                    atual = Some(Configuracao {
                        subgrupo: subgrupo.clone(),
                        subgrupo_alias: subgrupo_alias.clone(),
                        guid: g,
                        ..Default::default()
                    });
                    indice_pendente = None;
                }
            }
            continue;
        }

        let valor = valor_da_linha(texto);
        let e_apelido = texto.to_lowercase().contains("alias")
            && !valor.is_empty()
            && valor.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_');

        if e_apelido {
            match atual.as_mut() {
                Some(c) if recuo > 4 => c.alias = Some(valor.to_string()),
                None => subgrupo_alias = Some(valor.to_string()),
                Some(_) => subgrupo_alias = Some(valor.to_string()),
            }
            continue;
        }

        let Some(c) = atual.as_mut() else { continue };

        if let Some(h) = hexadecimal(valor) {
            if recuo > 4 {
                hex_do_ajuste.push(h);
            } else if c.ac.is_none() {
                c.ac = Some(h);
            } else if c.dc.is_none() {
                c.dc = Some(h);
            }
            continue;
        }

        if recuo > 4 {
            if !valor.is_empty() && valor.len() <= 4 && valor.chars().all(|ch| ch.is_ascii_digit()) {
                indice_pendente = valor.parse().ok();
            } else if let Some(i) = indice_pendente.take() {
                c.possiveis.push((i, valor.to_string()));
            } else if c.unidade.is_none() {
                c.unidade = Some(valor.to_string());
            }
        }
    }

    fechar(&mut atual, &mut hex_do_ajuste, &mut e);
    e
}

// ============================================ os ajustes que o motor conhece

/// Apelidos que o motor sabe interpretar. Todos documentados pela Microsoft em
/// "Processor power management options". Os com sufixo `1` valem para a
/// classe de eficiência 1 — em Intel híbrida, os núcleos de DESEMPENHO.
pub mod apelidos {
    pub const PERFINCTHRESHOLD: &str = "PERFINCTHRESHOLD";
    pub const PERFINCTIME: &str = "PERFINCTIME";
    pub const PERFINCPOL: &str = "PERFINCPOL";
    pub const PERFDECTHRESHOLD: &str = "PERFDECTHRESHOLD";
    pub const PERFDECTIME: &str = "PERFDECTIME";
    pub const PERFDECPOL: &str = "PERFDECPOL";
    pub const PERFEPP: &str = "PERFEPP";
    pub const PERFAUTONOMOUS: &str = "PERFAUTONOMOUS";
    pub const PERFBOOSTMODE: &str = "PERFBOOSTMODE";
    pub const PERFBOOSTPOL: &str = "PERFBOOSTPOL";
    pub const PROCTHROTTLEMIN: &str = "PROCTHROTTLEMIN";
    pub const PROCTHROTTLEMAX: &str = "PROCTHROTTLEMAX";
    pub const PROCFREQMAX: &str = "PROCFREQMAX";
    pub const CPMINCORES: &str = "CPMINCORES";
    pub const CPMAXCORES: &str = "CPMAXCORES";
    pub const CPINCREASETIME: &str = "CPINCREASETIME";
    pub const CPDECREASETIME: &str = "CPDECREASETIME";
    pub const CPCONCURRENCY: &str = "CPCONCURRENCY";
    pub const CPDISTRIBUTION: &str = "CPDISTRIBUTION";
    pub const CPHEADROOM: &str = "CPHEADROOM";
    pub const LATENCYHINTUNPARK: &str = "LATENCYHINTUNPARK";
    pub const LATENCYHINTPERF: &str = "LATENCYHINTPERF";
    pub const LATENCYHINTEPP: &str = "LATENCYHINTEPP";
    pub const SCHEDPOLICY: &str = "SCHEDPOLICY";
    pub const SHORTSCHEDPOLICY: &str = "SHORTSCHEDPOLICY";
    pub const MODULEUNPARKPOLICY: &str = "MODULEUNPARKPOLICY";
    pub const COMPLEXUNPARKPOLICY: &str = "COMPLEXUNPARKPOLICY";
    pub const SMTUNPARKPOLICY: &str = "SMTUNPARKPOLICY";
    pub const ASPM: &str = "ASPM";
    pub const USBSELECTIVESUSPEND: &str = "USBSELECTIVESUSPEND";

    /// Os que a tela avançada mostra, na ordem da resposta da CPU.
    pub const DA_TELA: &[&str] = &[
        PERFEPP, PERFAUTONOMOUS, PERFBOOSTMODE, PERFBOOSTPOL, PROCTHROTTLEMIN, PROCTHROTTLEMAX, PROCFREQMAX,
        PERFINCPOL, PERFINCTHRESHOLD, PERFINCTIME, PERFDECPOL, PERFDECTHRESHOLD, PERFDECTIME,
        LATENCYHINTPERF, LATENCYHINTEPP, LATENCYHINTUNPARK,
        CPMINCORES, CPMAXCORES, CPINCREASETIME, CPDECREASETIME, CPCONCURRENCY, CPDISTRIBUTION, CPHEADROOM,
        MODULEUNPARKPOLICY, COMPLEXUNPARKPOLICY, SMTUNPARKPOLICY, SCHEDPOLICY, SHORTSCHEDPOLICY,
        ASPM, USBSELECTIVESUSPEND,
    ];
}

use apelidos as ap;

/// EPP da escala do processador (0–255) para a do Windows (0–100 %).
pub fn epp_para_percentual(bruto: u32) -> u32 {
    ((bruto.min(255) as f64) * 100.0 / 255.0).round() as u32
}

// ============================================================ candidatos

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Estacionamento {
    /// O que o plano base traz.
    Windows,
    /// Uma reserva de núcleos acordada, o resto acorda rápido e dorme devagar.
    Adaptativo,
    /// Todos acordados. Testado, nunca presumido.
    TodosAcordados,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Boost {
    Windows,
    Agressivo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Autonomia {
    Windows,
    /// Deixar o processador escolher os estados (HWP / CPPC).
    Hardware,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Resposta {
    Windows,
    /// Sobe cedo e direto; desce em degraus, sem pressa.
    Rapida,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PoliticaDeDispositivo {
    Windows,
    /// Economia desligada NA TOMADA. Só entra como variação testada.
    Desligada,
}

/// Os parâmetros de comportamento de um candidato. Os números concretos saem
/// de `gerar`, que conhece a arquitetura e o que o Windows expõe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Parametros {
    /// EPP na escala do processador (0–255). `None` mantém o do plano base.
    pub epp_bruto: Option<u32>,
    pub estacionamento: Estacionamento,
    pub boost: Boost,
    pub autonomia: Autonomia,
    pub resposta: Resposta,
    /// Estado mínimo do processador. Só oferecido em CPU sem controle por
    /// hardware, onde ele governa de verdade.
    pub minimo: Option<u32>,
    pub pcie: PoliticaDeDispositivo,
    pub usb: PoliticaDeDispositivo,
}

impl Parametros {
    pub const WINDOWS: Parametros = Parametros {
        epp_bruto: None,
        estacionamento: Estacionamento::Windows,
        boost: Boost::Windows,
        autonomia: Autonomia::Windows,
        resposta: Resposta::Windows,
        minimo: None,
        pcie: PoliticaDeDispositivo::Windows,
        usb: PoliticaDeDispositivo::Windows,
    };
}

/// Por que cada número foi escolhido. A tela escolhe a frase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Motivo {
    EppDoCandidato,
    EppNucleosDeEficienciaPreservado,
    AutonomiaPeloHardware,
    BoostAgressivoEmTeste,
    SobeMaisCedo,
    SobeSemEsperarJanela,
    SobeDireto,
    DesceEmDegraus,
    DesceSemPressa,
    ReservaDeNucleosAcordada,
    NucleosAcordamRapido,
    NucleosDormemDevagar,
    TodosOsNucleosEmTeste,
    DicaDeLatenciaAcordaNucleos,
    DicaDeLatenciaDesempenho,
    PreferirNucleosDeDesempenho,
    MinimoEmCpuSemControleDeHardware,
    TetoNaoPodeLimitar,
    PcieEconomiaDesligadaEmTeste,
    UsbSuspensaoDesligadaEmTeste,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MotivoIgnorado {
    /// Este Windows não expõe o ajuste.
    NaoExiste,
    /// O valor não está na faixa que o Windows publica para ele.
    ForaDaFaixa,
    /// A CPU não tem o recurso (EPP sem HWP/CPPC, autonomia sem hardware).
    CpuNaoSuporta,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mudanca {
    pub alias: String,
    pub subgrupo: String,
    pub guid: String,
    /// `None` = não mexer neste lado. No notebook, a bateria é sempre `None`.
    pub ac: Option<u32>,
    pub dc: Option<u32>,
    pub motivo: Motivo,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ignorado {
    pub alias: String,
    pub motivo: MotivoIgnorado,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Candidato {
    pub id: String,
    pub papel: Papel,
    pub parametros: Parametros,
    pub mudancas: Vec<Mudanca>,
    pub ignorados: Vec<Ignorado>,
}

/// O papel do candidato na bateria de testes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Papel {
    PadraoWindows,
    /// No notebook: "PLUGGED COMPETITIVE".
    A,
    /// No notebook: "PLUGGED BALANCED".
    B,
    C,
    /// "Resposta máxima": o estilo dos planos de concorrente — tudo no máximo.
    /// Só em desktop, e só como candidato MEDIDO: se ganhar nesta máquina, é
    /// escolhido; se não, não.
    D,
    /// Vizinho do vencedor, testado na segunda etapa do autoajuste.
    Refino,
    Epp,
    Dispositivo,
}

struct Gerador<'a> {
    impressao: &'a Impressao,
    base: &'a Enumeracao,
    mudancas: Vec<Mudanca>,
    ignorados: Vec<Ignorado>,
}

impl Gerador<'_> {
    fn por(&mut self, alias: &str, valor: u32, motivo: Motivo) {
        let Some(c) = self.base.por_alias(alias) else {
            self.ignorados.push(Ignorado { alias: alias.to_string(), motivo: MotivoIgnorado::NaoExiste });
            return;
        };
        if !c.aceita(valor) {
            self.ignorados.push(Ignorado { alias: alias.to_string(), motivo: MotivoIgnorado::ForaDaFaixa });
            return;
        }
        // NUNCA APLICAR PERFIL DE DESKTOP NA BATERIA. No notebook, o lado da
        // bateria fica exatamente como o plano base (BATTERY SAFE). No desktop
        // os dois lados recebem o valor: com nobreak, o Windows usa o da bateria.
        let dc = match self.impressao.formato {
            Formato::Desktop => Some(valor),
            Formato::Notebook => None,
        };
        self.mudancas.retain(|m| !m.alias.eq_ignore_ascii_case(alias));
        self.mudancas.push(Mudanca {
            alias: c.alias.clone().unwrap_or_else(|| alias.to_string()),
            subgrupo: c.subgrupo.clone(),
            guid: c.guid.clone(),
            ac: Some(valor),
            dc,
            motivo,
        });
    }

    /// Em CPU híbrida, o ajuste com sufixo `1` governa os núcleos P.
    fn nos_nucleos_de_desempenho(&mut self, alias: &str, valor: u32, motivo: Motivo) {
        let com_classe = format!("{}1", alias);
        if self.impressao.arquitetura == Arquitetura::IntelHibrida && self.base.por_alias(&com_classe).is_some() {
            self.por(&com_classe, valor, motivo);
        } else {
            self.por(alias, valor, motivo);
        }
    }

    fn cpu_nao_suporta(&mut self, alias: &str) {
        self.ignorados.push(Ignorado { alias: alias.to_string(), motivo: MotivoIgnorado::CpuNaoSuporta });
    }
}

fn controle_por_hardware(i: &Impressao) -> bool {
    matches!(i.controle, Controle::HardwareHwp | Controle::HardwareCppc)
}

/// Traduz parâmetros em números para ESTA máquina.
pub fn gerar(impressao: &Impressao, base: &Enumeracao, id: &str, papel: Papel, p: Parametros) -> Candidato {
    let mut g = Gerador { impressao, base, mudancas: Vec::new(), ignorados: Vec::new() };
    let notebook = impressao.formato == Formato::Notebook;

    // O teto nunca pode estar limitando por acidente. Só entra se o plano
    // base estiver diferente do "sem limite".
    if base.por_alias(ap::PROCTHROTTLEMAX).is_some_and(|c| c.ac != Some(100)) {
        g.por(ap::PROCTHROTTLEMAX, 100, Motivo::TetoNaoPodeLimitar);
    }
    if base.por_alias(ap::PROCFREQMAX).is_some_and(|c| c.ac.is_some_and(|v| v != 0)) {
        g.por(ap::PROCFREQMAX, 0, Motivo::TetoNaoPodeLimitar);
    }

    if let Some(bruto) = p.epp_bruto {
        if controle_por_hardware(impressao) {
            let pct = epp_para_percentual(bruto);
            g.nos_nucleos_de_desempenho(ap::PERFEPP, pct, Motivo::EppDoCandidato);
            if impressao.arquitetura == Arquitetura::IntelHibrida
                && base.por_alias("PERFEPP1").is_some()
            {
                // Os núcleos E ficam com o EPP do plano base: é trabalho de
                // fundo, e forçá-los a desempenho é calor sem FPS.
                if let Some(v) = base.por_alias(ap::PERFEPP).and_then(|c| c.ac) {
                    g.por(ap::PERFEPP, v, Motivo::EppNucleosDeEficienciaPreservado);
                }
            }
        } else {
            g.cpu_nao_suporta(ap::PERFEPP);
        }
    }

    if p.autonomia == Autonomia::Hardware {
        if controle_por_hardware(impressao) {
            g.por(ap::PERFAUTONOMOUS, 1, Motivo::AutonomiaPeloHardware);
        } else {
            g.cpu_nao_suporta(ap::PERFAUTONOMOUS);
        }
    }

    if p.boost == Boost::Agressivo {
        g.por(ap::PERFBOOSTMODE, 2, Motivo::BoostAgressivoEmTeste);
    }

    if p.resposta == Resposta::Rapida {
        g.nos_nucleos_de_desempenho(ap::PERFINCTHRESHOLD, 30, Motivo::SobeMaisCedo);
        g.nos_nucleos_de_desempenho(ap::PERFINCTIME, 1, Motivo::SobeSemEsperarJanela);
        g.nos_nucleos_de_desempenho(ap::PERFINCPOL, 2, Motivo::SobeDireto);
        g.nos_nucleos_de_desempenho(ap::PERFDECPOL, 1, Motivo::DesceEmDegraus);
        g.nos_nucleos_de_desempenho(ap::PERFDECTIME, 3, Motivo::DesceSemPressa);
        g.por(ap::LATENCYHINTPERF, 100, Motivo::DicaDeLatenciaDesempenho);
    }

    match p.estacionamento {
        Estacionamento::Windows => {}
        Estacionamento::Adaptativo => {
            let reserva = if notebook { 25 } else { 50 };
            g.nos_nucleos_de_desempenho(ap::CPMINCORES, reserva, Motivo::ReservaDeNucleosAcordada);
            g.por(ap::CPINCREASETIME, 1, Motivo::NucleosAcordamRapido);
            g.por(ap::CPDECREASETIME, 20, Motivo::NucleosDormemDevagar);
            g.por(ap::LATENCYHINTUNPARK, 100, Motivo::DicaDeLatenciaAcordaNucleos);
        }
        Estacionamento::TodosAcordados => {
            g.nos_nucleos_de_desempenho(ap::CPMINCORES, 100, Motivo::TodosOsNucleosEmTeste);
        }
    }

    if impressao.arquitetura == Arquitetura::IntelHibrida && p.resposta == Resposta::Rapida {
        // Preferir P, sem proibir E: o Windows ainda usa os E quando os P
        // estão ocupados. Desligar núcleo E nunca é oferecido.
        g.por(ap::SCHEDPOLICY, 2, Motivo::PreferirNucleosDeDesempenho);
    }

    if let Some(minimo) = p.minimo {
        if controle_por_hardware(impressao) {
            g.cpu_nao_suporta(ap::PROCTHROTTLEMIN);
        } else {
            g.por(ap::PROCTHROTTLEMIN, minimo, Motivo::MinimoEmCpuSemControleDeHardware);
        }
    }

    if p.pcie == PoliticaDeDispositivo::Desligada {
        g.por(ap::ASPM, 0, Motivo::PcieEconomiaDesligadaEmTeste);
    }
    if p.usb == PoliticaDeDispositivo::Desligada {
        g.por(ap::USBSELECTIVESUSPEND, 0, Motivo::UsbSuspensaoDesligadaEmTeste);
    }

    Candidato { id: id.to_string(), papel, parametros: p, mudancas: g.mudancas, ignorados: g.ignorados }
}

/// A bateria de autoajuste: padrão do Windows e três candidatos, escolhidos
/// pela arquitetura. Nenhum valor é universal.
pub fn candidatos_de_autoajuste(impressao: &Impressao, base: &Enumeracao) -> Vec<Candidato> {
    let hw = controle_por_hardware(impressao);
    let notebook = impressao.formato == Formato::Notebook;

    let (a, b, c) = if hw {
        (
            Parametros { epp_bruto: Some(0), estacionamento: Estacionamento::TodosAcordados, resposta: Resposta::Rapida, ..Parametros::WINDOWS },
            Parametros { epp_bruto: Some(32), estacionamento: Estacionamento::Adaptativo, resposta: Resposta::Rapida, ..Parametros::WINDOWS },
            Parametros { epp_bruto: Some(64), estacionamento: Estacionamento::Adaptativo, autonomia: Autonomia::Hardware, ..Parametros::WINDOWS },
        )
    } else {
        (
            Parametros {
                estacionamento: Estacionamento::TodosAcordados,
                resposta: Resposta::Rapida,
                // Mínimo alto só em desktop, e só como candidato medido.
                minimo: (!notebook).then_some(100),
                ..Parametros::WINDOWS
            },
            Parametros { estacionamento: Estacionamento::Adaptativo, resposta: Resposta::Rapida, ..Parametros::WINDOWS },
            Parametros { estacionamento: Estacionamento::Adaptativo, resposta: Resposta::Rapida, boost: Boost::Agressivo, ..Parametros::WINDOWS },
        )
    };

    let mut v = vec![
        gerar(impressao, base, "windows", Papel::PadraoWindows, Parametros::WINDOWS),
        gerar(impressao, base, "a", Papel::A, a),
        gerar(impressao, base, "b", Papel::B, b),
        gerar(impressao, base, "c", Papel::C, c),
    ];
    if !notebook {
        let d = Parametros {
            epp_bruto: hw.then_some(0),
            estacionamento: Estacionamento::TodosAcordados,
            boost: Boost::Agressivo,
            resposta: Resposta::Rapida,
            minimo: (!hw).then_some(100),
            ..Parametros::WINDOWS
        };
        v.push(gerar(impressao, base, "d", Papel::D, d));
    }
    v
}

/// O REFINO: vizinhos do vencedor da primeira etapa, para achar o ponto
/// ótimo em volta dele em vez de ficar com o melhor de quatro palpites.
///
/// EPP 16 acima e 16 abaixo (quando o processador usa EPP) e o estacionamento
/// trocado entre adaptativo e todos acordados. Nada que o vencedor já tinha.
pub fn vizinhos_do_vencedor(p: Parametros, impressao: &Impressao) -> Vec<Parametros> {
    let mut v = Vec::new();
    if let Some(epp) = p.epp_bruto {
        if controle_por_hardware(impressao) {
            for novo in [epp.saturating_sub(16), (epp + 16).min(255)] {
                if novo != epp {
                    v.push(Parametros { epp_bruto: Some(novo), ..p });
                }
            }
        }
    }
    let outro = match p.estacionamento {
        Estacionamento::TodosAcordados => Some(Estacionamento::Adaptativo),
        Estacionamento::Adaptativo if impressao.formato == Formato::Desktop => Some(Estacionamento::TodosAcordados),
        Estacionamento::Adaptativo => None,
        Estacionamento::Windows => Some(Estacionamento::Adaptativo),
    };
    if let Some(e) = outro {
        v.push(Parametros { estacionamento: e, ..p });
    }
    v.dedup();
    v
}

/// O laboratório de EPP: mesma resposta e estacionamento, só o EPP varia.
pub fn escada_de_epp(impressao: &Impressao, base: &Enumeracao) -> Vec<Candidato> {
    if !controle_por_hardware(impressao) || base.por_alias(ap::PERFEPP).is_none() {
        return Vec::new();
    }
    [0u32, 16, 32, 64, 128]
        .into_iter()
        .map(|bruto| {
            gerar(
                impressao,
                base,
                &format!("epp-{}", bruto),
                Papel::Epp,
                Parametros { epp_bruto: Some(bruto), estacionamento: Estacionamento::Adaptativo, ..Parametros::WINDOWS },
            )
        })
        .collect()
}

/// Variações de dispositivo sobre um candidato: ASPM e suspensão do USB. Nunca
/// entram sozinhas; só medidas contra o mesmo candidato sem elas.
pub fn variacoes_de_dispositivo(impressao: &Impressao, base: &Enumeracao, sobre: Parametros) -> Vec<Candidato> {
    vec![
        gerar(impressao, base, "pcie", Papel::Dispositivo, Parametros { pcie: PoliticaDeDispositivo::Desligada, ..sobre }),
        gerar(impressao, base, "usb", Papel::Dispositivo, Parametros { usb: PoliticaDeDispositivo::Desligada, ..sobre }),
    ]
}

/// Todo apelido que algum candidato pode tocar. Antes de aplicar um candidato,
/// esses voltam ao plano base — senão o candidato B herdaria o que o A deixou.
pub fn apelidos_controlados(candidatos: &[Candidato]) -> Vec<String> {
    let mut v: Vec<String> = candidatos.iter().flat_map(|c| c.mudancas.iter().map(|m| m.alias.clone())).collect();
    v.sort();
    v.dedup();
    v
}

/// O que precisa ser escrito para o plano ficar no candidato: controlados
/// voltam ao base, depois o candidato por cima. Só o que difere do atual.
pub fn plano_de_escrita(
    candidato: &Candidato,
    controlados: &[String],
    base: &Enumeracao,
    atual: &Enumeracao,
) -> Vec<Mudanca> {
    let mut alvo: Vec<Mudanca> = Vec::new();

    for alias in controlados {
        let Some(b) = base.por_alias(alias) else { continue };
        let do_candidato = candidato.mudancas.iter().find(|m| m.alias.eq_ignore_ascii_case(alias));
        let (ac, dc) = match do_candidato {
            Some(m) => (m.ac.or(b.ac), m.dc.or(b.dc)),
            None => (b.ac, b.dc),
        };
        let agora = atual.por_alias(alias);
        let ac_escrever = ac.filter(|v| agora.and_then(|c| c.ac) != Some(*v));
        let dc_escrever = dc.filter(|v| agora.and_then(|c| c.dc) != Some(*v));
        if ac_escrever.is_none() && dc_escrever.is_none() {
            continue;
        }
        alvo.push(Mudanca {
            alias: alias.clone(),
            subgrupo: b.subgrupo.clone(),
            guid: b.guid.clone(),
            ac: ac_escrever,
            dc: dc_escrever,
            motivo: do_candidato.map(|m| m.motivo).unwrap_or(Motivo::TetoNaoPodeLimitar),
        });
    }
    alvo
}

/// Conferência depois de escrever: quais apelidos não ficaram no alvo.
pub fn divergencias(pedidas: &[Mudanca], relido: &Enumeracao) -> Vec<String> {
    pedidas
        .iter()
        .filter(|m| {
            let c = relido.por_alias(&m.alias);
            let ac_ok = m.ac.map_or(true, |v| c.and_then(|c| c.ac) == Some(v));
            let dc_ok = m.dc.map_or(true, |v| c.and_then(|c| c.dc) == Some(v));
            !(ac_ok && dc_ok)
        })
        .map(|m| m.alias.clone())
        .collect()
}

// ================================================== teste de resposta (rajada)

/// Medida da rajada: ociosa → carga curta → ociosa → carga.
///
/// A CPU é medida PELO TRABALHO QUE ELA ENTREGA, fatia por fatia: nenhum
/// contador de frequência precisa ser confiável para isto funcionar. Se o
/// plano demora a subir o desempenho, as primeiras fatias de cada rajada
/// fazem menos trabalho que as do fim — e isso é exatamente a resposta que
/// importa para um jogo.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RespostaMedida {
    /// Tempo até a CPU entregar 90% do trabalho sustentado (mediana das rajadas).
    pub ate_90_ms: f64,
    /// Quanto do sustentado a primeira fatia entrega, em %.
    pub primeira_fatia_pct: f64,
    /// Trabalho sustentado por fatia (unidade arbitrária, só comparável aqui).
    pub sustentado: f64,
    /// Variação do sustentado entre rajadas, em %.
    pub dispersao_pct: f64,
    pub rajadas: usize,
}

fn mediana(v: &[f64]) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    let mut o = v.to_vec();
    o.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    o[o.len() / 2]
}

fn arred(x: f64, casas: i32) -> f64 {
    let f = 10f64.powi(casas);
    (x * f).round() / f
}

pub fn analisar_rajadas(rajadas: &[Vec<f64>], fatia_ms: f64) -> Option<RespostaMedida> {
    let validas: Vec<&Vec<f64>> = rajadas.iter().filter(|r| r.len() >= 10).collect();
    if validas.is_empty() || fatia_ms <= 0.0 {
        return None;
    }

    let mut tempos = Vec::new();
    let mut primeiras = Vec::new();
    let mut sustentados = Vec::new();

    for r in &validas {
        let cauda = &r[r.len() * 7 / 10..];
        let sustentado = mediana(cauda);
        if sustentado <= 0.0 {
            continue;
        }
        let indice = r.iter().position(|v| *v >= sustentado * 0.9).unwrap_or(r.len() - 1);
        tempos.push((indice as f64 + 1.0) * fatia_ms);
        primeiras.push((r[0] / sustentado * 100.0).min(150.0));
        sustentados.push(sustentado);
    }

    if sustentados.is_empty() {
        return None;
    }

    let media = sustentados.iter().sum::<f64>() / sustentados.len() as f64;
    let desvio = (sustentados.iter().map(|s| (s - media).powi(2)).sum::<f64>() / sustentados.len() as f64).sqrt();

    Some(RespostaMedida {
        ate_90_ms: arred(mediana(&tempos), 1),
        primeira_fatia_pct: arred(mediana(&primeiras), 0),
        sustentado: arred(media, 1),
        dispersao_pct: arred(if media > 0.0 { desvio / media * 100.0 } else { 0.0 }, 1),
        rajadas: sustentados.len(),
    })
}

// ======================================================= amostras da CPU

/// Uma amostra dos contadores de desempenho do Windows (nomes em inglês pelo
/// PDH, então iguais em qualquer idioma).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct AmostraCpu {
    /// `% Processor Performance`: desempenho em relação à frequência nominal.
    pub desempenho_pct: Option<f64>,
    /// `Processor Frequency`: frequência nominal, em MHz.
    pub frequencia_mhz: Option<f64>,
    /// `% Processor Time`: fração do tempo em que a CPU trabalhou.
    pub uso_pct: Option<f64>,
    /// `% Performance Limit`: 100 = sem limite.
    ///
    /// `None` quando o contador não respondeu. Isso NÃO é 100: um contador que
    /// falhou e um firmware que não limita davam a mesma resposta antes desta
    /// mudança, e o motor concluía "não há limite" a partir de uma leitura que
    /// nunca aconteceu.
    pub limite_pct: Option<f64>,
    /// `Performance Limit Flags`: bit 0 térmico, bit 1 elétrico. `None` quando
    /// o contador não respondeu — pela mesma razão de `limite_pct`.
    pub flags: Option<u64>,
    pub nucleos_acordados: Option<u32>,
    pub nucleos_total: Option<u32>,
    /// Zona térmica ACPI. Pode não ser o sensor do processador.
    pub temperatura_c: Option<f64>,
}

impl AmostraCpu {
    /// Clock reportado desta amostra: nominal × desempenho.
    ///
    /// `None` quando falta qualquer uma das duas metades — meia conta não é
    /// meia resposta, é nenhuma.
    pub fn clock_reportado_mhz(&self) -> Option<f64> {
        Some(self.frequencia_mhz? * self.desempenho_pct? / 100.0)
    }

    /// Clock efetivo: o reportado descontado o tempo em que a CPU não estava
    /// trabalhando. É o número que cai quando o firmware "sobe o clock" e a
    /// máquina não entrega nada com isso.
    pub fn clock_efetivo_mhz(&self) -> Option<f64> {
        Some(self.clock_reportado_mhz()? * self.uso_pct? / 100.0)
    }

    /// Esta amostra serve para a conta de clock.
    fn tem_clock(&self) -> bool {
        self.clock_efetivo_mhz().is_some()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct ResumoCpu {
    /// Quantas amostras entraram na conta de clock.
    pub amostras: usize,
    /// Quantas foram DESCARTADAS por terem vindo incompletas do PDH. Existe
    /// para aparecer no relatório: um resumo tirado de três amostras boas e
    /// trinta perdidas não vale o mesmo que um tirado de trinta e três.
    #[serde(default)]
    pub amostras_descartadas: usize,
    /// Frequência nominal × desempenho: o que o Windows REPORTA.
    pub clock_reportado_mhz: f64,
    /// Reportado × uso: trabalho realmente entregue, que cai quando a CPU
    /// "sobe o clock" mas passa o tempo em espera ou limitada.
    pub clock_efetivo_mhz: f64,
    /// Uso médio do processador na janela, em %.
    ///
    /// É o `% Processor Time` das amostras que serviram. Sai separado do clock
    /// efetivo porque quem quer saber "o quanto a CPU trabalhou" não deve ter
    /// de desfazer a multiplicação por frequência para chegar lá.
    #[serde(default)]
    pub uso_medio_pct: Option<f64>,
    /// `None` quando o contador de limite não respondeu em nenhuma amostra.
    pub limite_medio_pct: Option<f64>,
    /// Fração das amostras com o firmware limitando. `None` quando não houve
    /// amostra com os contadores de limite.
    pub tempo_limitado_pct: Option<f64>,
    /// `None` quando o contador de flags não respondeu: não observar a flag é
    /// diferente de observar que ela está baixa.
    pub limite_termico: Option<bool>,
    pub limite_eletrico: Option<bool>,
    pub temperatura_max_c: Option<f64>,
    pub temperatura_media_c: Option<f64>,
    pub nucleos_acordados_medio: Option<f64>,
    pub nucleos_total: Option<u32>,
}

/// Resume uma série de amostras do PDH.
///
/// `None` quando nenhuma amostra trouxe as três leituras de que a conta de
/// clock precisa. Antes isto devolvia um `ResumoCpu` ZERADO, e um resumo com
/// clock efetivo de 0 MHz e "o firmware não limita" seguia para a pontuação
/// como se fosse medição.
pub fn resumir_cpu(amostras: &[AmostraCpu]) -> Option<ResumoCpu> {
    let uteis: Vec<&AmostraCpu> = amostras.iter().filter(|a| a.tem_clock()).collect();
    let n = uteis.len();
    if n == 0 {
        return None;
    }
    let media = |f: &dyn Fn(&AmostraCpu) -> f64| uteis.iter().map(|a| f(a)).sum::<f64>() / n as f64;
    // Limite e flags são contadores SEPARADOS dos de clock: podem faltar numa
    // amostra que serve para o resto. Só entram na conta as amostras em que
    // eles vieram, e se não vier nenhuma o campo fica desconhecido.
    let com_limite = uteis.iter().filter(|a| a.limite_pct.is_some() || a.flags.is_some()).count();
    let limitadas = uteis
        .iter()
        .filter(|a| a.flags.is_some_and(|f| f != 0) || a.limite_pct.is_some_and(|l| l < 95.0))
        .count();
    let limites_pct: Vec<f64> = uteis.iter().filter_map(|a| a.limite_pct).collect();
    let com_flags = uteis.iter().filter(|a| a.flags.is_some()).count();
    let mut temps: Vec<f64> = uteis.iter().filter_map(|a| a.temperatura_c).collect();
    // SENSOR PARADO NÃO É SENSOR. Muitas placas publicam uma zona térmica ACPI
    // fixa (28 °C o dia inteiro, com a CPU a 100%). Com carga e sem variação
    // nenhuma, a leitura é descartada: folga térmica inventada é pior que
    // "não sei".
    if temps.len() >= 8 {
        let (min, max) = temps.iter().fold((f64::MAX, f64::MIN), |(a, b), t| (a.min(*t), b.max(*t)));
        let com_carga = uteis.iter().any(|a| a.uso_pct.is_some_and(|u| u >= 10.0));
        if com_carga && max - min < 0.5 {
            temps.clear();
        }
    }
    let acordados: Vec<f64> = uteis.iter().filter_map(|a| a.nucleos_acordados.map(|v| v as f64)).collect();

    Some(ResumoCpu {
        amostras: n,
        amostras_descartadas: amostras.len() - n,
        clock_reportado_mhz: arred(media(&|a| a.clock_reportado_mhz().unwrap_or_default()), 0),
        clock_efetivo_mhz: arred(media(&|a| a.clock_efetivo_mhz().unwrap_or_default()), 0),
        uso_medio_pct: Some(arred(media(&|a| a.uso_pct.unwrap_or_default()), 1)),
        limite_medio_pct: (!limites_pct.is_empty()).then(|| arred(limites_pct.iter().sum::<f64>() / limites_pct.len() as f64, 1)),
        tempo_limitado_pct: (com_limite > 0).then(|| arred(limitadas as f64 / com_limite as f64 * 100.0, 0)),
        limite_termico: (com_flags > 0).then(|| uteis.iter().filter(|a| a.flags.is_some_and(|f| f & 1 != 0)).count() * 5 >= com_flags),
        limite_eletrico: (com_flags > 0).then(|| uteis.iter().filter(|a| a.flags.is_some_and(|f| f & 2 != 0)).count() * 5 >= com_flags),
        temperatura_max_c: temps.iter().copied().reduce(f64::max).map(|t| arred(t, 0)),
        temperatura_media_c: (!temps.is_empty()).then(|| arred(temps.iter().sum::<f64>() / temps.len() as f64, 0)),
        nucleos_acordados_medio: (!acordados.is_empty()).then(|| arred(acordados.iter().sum::<f64>() / acordados.len() as f64, 1)),
        nucleos_total: uteis.iter().find_map(|a| a.nucleos_total),
    })
}

/// THERMAL HEADROOM SCORE, 0–100. `None` quando não há leitura de temperatura
/// e nenhum limite térmico foi visto — não se inventa folga.
///
/// Note o `Some(true)`: flag não observada não conta como "sem limite". Com o
/// contador mudo e sem temperatura, a resposta continua sendo "não sei".
pub fn folga_termica(r: &ResumoCpu) -> Option<f64> {
    const TETO: f64 = 95.0;
    const CONFORTO: f64 = 45.0;
    let por_temperatura = r.temperatura_max_c.map(|t| ((TETO - t) / (TETO - CONFORTO) * 100.0).clamp(0.0, 100.0));
    match (por_temperatura, r.limite_termico) {
        (Some(v), Some(true)) => Some(v.min(20.0)),
        (Some(v), _) => Some(arred(v, 0)),
        (None, Some(true)) => Some(10.0),
        (None, _) => None,
    }
}

// ================================================================ jogo

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Quadros {
    pub fps: f64,
    pub low_1: f64,
    pub low_01: f64,
    pub p99_ms: f64,
}

pub fn resumir_quadros(fps: f64, intervalos_ms: &[f64]) -> Option<Quadros> {
    let mut o: Vec<f64> = intervalos_ms.iter().copied().filter(|v| v.is_finite() && *v > 0.0).collect();
    if o.len() < 100 {
        return None;
    }
    o.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = o.len();
    let media_dos_piores = |fracao: usize| {
        let q = (n / fracao).max(1);
        o[n - q..].iter().sum::<f64>() / q as f64
    };
    let posto = ((0.99 * n as f64).ceil() as usize).clamp(1, n) - 1;
    Some(Quadros {
        fps: arred(fps, 1),
        low_1: arred(1000.0 / media_dos_piores(100), 1),
        low_01: arred(1000.0 / media_dos_piores(1000), 1),
        p99_ms: arred(o[posto], 2),
    })
}

// ========================================================= resultado e nota

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResultadoDoCandidato {
    pub candidato: String,
    pub resposta: Option<RespostaMedida>,
    /// `None` quando o PDH não entregou nenhuma amostra utilizável. Resultado
    /// antigo gravado em disco tem um objeto aqui e continua desserializando
    /// como `Some` — o campo só passou a admitir ausência.
    #[serde(default)]
    pub cpu: Option<ResumoCpu>,
    pub quadros: Option<Quadros>,
    /// FPS de cada repetição, para a confiança.
    pub fps_repeticoes: Vec<f64>,
    pub gpu_pct: Option<f64>,
    pub uso_cpu_pct: Option<f64>,
    /// Os quadros vieram do teste de quadros do Otimiza, e não de um jogo.
    #[serde(default)]
    pub quadros_sinteticos: bool,
}

impl ResultadoDoCandidato {
    /// Temperatura máxima medida, se houve resumo e se houve sensor.
    fn temp_max(&self) -> Option<f64> {
        self.cpu.as_ref()?.temperatura_max_c
    }

    fn clock_efetivo(&self) -> Option<f64> {
        self.cpu.as_ref().map(|c| c.clock_efetivo_mhz)
    }

    fn clock_reportado(&self) -> Option<f64> {
        self.cpu.as_ref().map(|c| c.clock_reportado_mhz)
    }

    /// O clock reportado subiu e o efetivo caiu: o firmware anuncia mais
    /// frequência e a máquina entrega menos trabalho. `None` quando falta
    /// medição dos dois lados — e aí não se acusa nada.
    fn clock_piorou_contra(&self, base: &Self) -> Option<bool> {
        let (r_rep, b_rep) = (self.clock_reportado()?, base.clock_reportado()?);
        let (r_ef, b_ef) = (self.clock_efetivo()?, base.clock_efetivo()?);
        Some(r_rep > b_rep * 1.02 && r_ef < b_ef * 0.98)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Nota {
    /// 100 = igual ao padrão do Windows.
    pub total: f64,
    pub low_1: Option<f64>,
    pub p99: Option<f64>,
    pub fps: Option<f64>,
    pub resposta: Option<f64>,
    pub clock: Option<f64>,
    pub termica: Option<f64>,
    pub estabilidade: Option<f64>,
}

fn razao(melhor_se_maior: bool, valor: f64, base: f64) -> Option<f64> {
    if base <= 0.0 || valor <= 0.0 {
        return None;
    }
    let r = if melhor_se_maior { valor / base } else { base / valor };
    Some((r * 100.0).clamp(50.0, 150.0))
}

/// POWER PERFORMANCE SCORE, relativo ao padrão do Windows.
///
/// FPS médio pesa 15%. 1% low, P99 e resposta juntos pesam 55%, e a folga
/// térmica e a estabilidade sob limite de firmware entram contra quem ganha
/// FPS esquentando.
pub fn pontuar(r: &ResultadoDoCandidato, base: &ResultadoDoCandidato) -> Nota {
    let low_1 = r.quadros.zip(base.quadros).and_then(|(a, b)| razao(true, a.low_1, b.low_1));
    let p99 = r.quadros.zip(base.quadros).and_then(|(a, b)| razao(false, a.p99_ms, b.p99_ms));
    let fps = r.quadros.zip(base.quadros).and_then(|(a, b)| razao(true, a.fps, b.fps));
    let resposta = r.resposta.zip(base.resposta).and_then(|(a, b)| razao(false, a.ate_90_ms, b.ate_90_ms));
    let clock = r.clock_efetivo().zip(base.clock_efetivo()).and_then(|(a, b)| razao(true, a, b));
    let folga = |x: &ResultadoDoCandidato| x.cpu.as_ref().and_then(folga_termica);
    let termica = match (folga(r), folga(base)) {
        (Some(a), Some(b)) => Some((100.0 + (a - b)).clamp(50.0, 150.0)),
        _ => None,
    };
    // Sem contador de limite dos dois lados não há estabilidade a comparar.
    // Antes o campo era sempre preenchido, e um par de leituras que não
    // aconteceu virava nota 100 — um décimo do peso saindo do nada.
    let limitado = |x: &ResultadoDoCandidato| x.cpu.as_ref().and_then(|c| c.tempo_limitado_pct);
    let estabilidade = limitado(r).zip(limitado(base)).map(|(a, b)| (100.0 - (a - b)).clamp(50.0, 150.0));

    let partes = [
        (low_1, 0.25),
        (p99, 0.15),
        (fps, 0.15),
        (resposta, 0.15),
        (clock, 0.10),
        (termica, 0.10),
        (estabilidade, 0.10),
    ];
    let peso: f64 = partes.iter().filter(|(v, _)| v.is_some()).map(|(_, p)| p).sum();
    let soma: f64 = partes.iter().filter_map(|(v, p)| v.map(|v| v * p)).sum();

    Nota {
        total: arred(if peso > 0.0 { soma / peso } else { 100.0 }, 1),
        low_1: low_1.map(|v| arred(v, 1)),
        p99: p99.map(|v| arred(v, 1)),
        fps: fps.map(|v| arred(v, 1)),
        resposta: resposta.map(|v| arred(v, 1)),
        clock: clock.map(|v| arred(v, 1)),
        termica: termica.map(|v| arred(v, 1)),
        estabilidade: estabilidade.map(|v| arred(v, 1)),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Achado {
    /// O firmware limita a CPU (térmico) — nenhum plano resolve isso.
    FirmwareLimitaPorTemperatura,
    /// O firmware limita a CPU (energia/EDP/PL) — nenhum plano resolve isso.
    FirmwareLimitaPorEnergia,
    /// O clock reportado subiu e o efetivo caiu: não é melhora.
    ClockReportadoSubiuEfetivoCaiu,
    /// Sem jogo medido: a escolha se apoia só na rajada e nos contadores.
    SemJogoMedido,
    /// Temperatura não exposta pelo Windows nesta máquina.
    TemperaturaIndisponivel,
    /// Os contadores de limite de firmware não responderam. Não dá para dizer
    /// que a CPU está livre nem que está limitada.
    LimitesDeFirmwareIndisponiveis,
    /// A amostragem do PDH falhou em mais da metade das leituras. O resumo
    /// vale menos do que o número de amostras sugere.
    AmostragemFalha,
    /// Os candidatos ficaram dentro da margem: fica o padrão do Windows.
    NenhumGanhouComMargem,
    /// O plano que a pessoa já usa não perdeu para nenhum candidato: fica ele.
    PlanoAtualJaEOMelhor,
    /// Um candidato foi descartado por ter MENOS FPS ou pior 1% low.
    CandidatoTirariaFps,
    /// Sem jogo aberto, os quadros vieram do teste de quadros do Otimiza.
    QuadrosSinteticos,
    /// Empate de desempenho; ganhou o que esquenta menos.
    EmpateDesfeitoPelaTemperatura,
    PoucasRepeticoes,
    MedicaoInstavel,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Escolha {
    pub notas: Vec<(String, Nota)>,
    /// `None` = fica o padrão do Windows.
    pub vencedor: Option<String>,
    pub confianca_pct: f64,
    pub achados: Vec<Achado>,
    /// Ganhos MEDIDOS do vencedor contra o padrão, em %. `None` quando não
    /// houve medição daquele número.
    pub fps_pct: Option<f64>,
    pub low_1_pct: Option<f64>,
    pub p99_pct: Option<f64>,
    pub resposta_pct: Option<f64>,
    pub temperatura_delta_c: Option<f64>,
}

const MARGEM: f64 = 2.0;

fn variacao(a: f64, b: f64) -> Option<f64> {
    (b > 0.0).then(|| arred((a - b) / b * 100.0, 1))
}

/// Escolhe o candidato. `base` é o id do padrão do Windows.
pub fn escolher(resultados: &[ResultadoDoCandidato], base: &str) -> Option<Escolha> {
    let b = resultados.iter().find(|r| r.candidato == base)?;
    let mut achados = Vec::new();

    // `== Some(true)`: o achado só é anunciado quando a flag foi LIDA e estava
    // alta. Contador mudo não vira "o firmware limita" nem o seu contrário.
    let termico = b.cpu.as_ref().and_then(|c| c.limite_termico);
    let eletrico = b.cpu.as_ref().and_then(|c| c.limite_eletrico);

    if termico == Some(true) {
        achados.push(Achado::FirmwareLimitaPorTemperatura);
    }
    if eletrico == Some(true) {
        achados.push(Achado::FirmwareLimitaPorEnergia);
    }
    if termico.is_none() && eletrico.is_none() {
        achados.push(Achado::LimitesDeFirmwareIndisponiveis);
    }
    if b.quadros.is_none() {
        achados.push(Achado::SemJogoMedido);
    }
    if b.temp_max().is_none() {
        achados.push(Achado::TemperaturaIndisponivel);
    }
    // Sem resumo nenhum a amostragem falhou inteira, que é o caso mais grave e
    // não pode ser o mais silencioso.
    if b.cpu.as_ref().is_none_or(|c| c.amostras_descartadas > c.amostras) {
        achados.push(Achado::AmostragemFalha);
    }

    let notas: Vec<(String, Nota)> = resultados.iter().map(|r| (r.candidato.clone(), pontuar(r, b))).collect();

    let sem_regressao = |r: &ResultadoDoCandidato| {
        // NUNCA MENOS FPS. Um plano que tira FPS médio ou 1% low não entra,
        // não importa o que mais ele melhore.
        let quadros_ok = match (r.quadros, b.quadros) {
            (Some(a), Some(q)) => a.fps >= q.fps * 0.99 && a.low_1 >= q.low_1 * 0.99 && a.p99_ms <= q.p99_ms * 1.03,
            _ => true,
        };
        let termica_ok = match (r.temp_max(), b.temp_max(), r.quadros, b.quadros) {
            (Some(t), Some(tb), Some(a), Some(q)) => t <= tb + 5.0 || a.low_1 >= q.low_1 * 1.05,
            (Some(t), Some(tb), _, _) => t <= tb + 5.0,
            _ => true,
        };
        // Sem medição dos dois lados o veto não se aplica: não há do que
        // acusar o candidato.
        let clock_ok = r.clock_piorou_contra(b) != Some(true);
        quadros_ok && termica_ok && clock_ok
    };

    if resultados.iter().any(|r| r.candidato != base && r.clock_piorou_contra(b) == Some(true)) {
        achados.push(Achado::ClockReportadoSubiuEfetivoCaiu);
    }

    let mut elegiveis: Vec<(&ResultadoDoCandidato, Nota)> = resultados
        .iter()
        .zip(notas.iter())
        .filter(|(r, (_, n))| r.candidato != base && n.total >= 100.0 + MARGEM && sem_regressao(r))
        .map(|(r, (_, n))| (r, *n))
        .collect();

    elegiveis.sort_by(|(_, a), (_, b)| b.total.partial_cmp(&a.total).unwrap_or(std::cmp::Ordering::Equal));

    // PERFORMANCE PER WATT COMO DESEMPATE: dentro da margem do líder, fica o
    // que esquenta menos.
    let vencedor = match elegiveis.as_slice() {
        [] => None,
        [(unico, _)] => Some(*unico),
        [(lider, nl), resto @ ..] => {
            let empatados: Vec<&ResultadoDoCandidato> = std::iter::once(*lider)
                .chain(resto.iter().filter(|(_, n)| nl.total - n.total < MARGEM).map(|(r, _)| *r))
                .collect();
            let mais_fresco = empatados
                .iter()
                .copied()
                .filter(|r| r.temp_max().is_some())
                .min_by(|a, b| a.temp_max().partial_cmp(&b.temp_max()).unwrap_or(std::cmp::Ordering::Equal));
            match mais_fresco {
                Some(f) if empatados.len() > 1 && f.candidato != lider.candidato => {
                    achados.push(Achado::EmpateDesfeitoPelaTemperatura);
                    Some(f)
                }
                _ => Some(*lider),
            }
        }
    };

    if resultados.iter().any(|r| {
        r.candidato != base
            && matches!((r.quadros, b.quadros), (Some(a), Some(q)) if a.fps < q.fps * 0.99 || a.low_1 < q.low_1 * 0.99)
    }) {
        achados.push(Achado::CandidatoTirariaFps);
    }
    if b.quadros_sinteticos {
        achados.push(Achado::QuadrosSinteticos);
    }

    // O PLANO ATUAL É O PISO. Trocar o plano que a pessoa já usa por um que dá
    // menos FPS — mesmo que ganhe do padrão do Windows — é tirar FPS dela.
    let atual = resultados.iter().find(|r| r.candidato == "atual");
    let vencedor = match (vencedor, atual) {
        (Some(v), Some(_)) if v.candidato == "atual" => {
            achados.push(Achado::PlanoAtualJaEOMelhor);
            Some(v)
        }
        (Some(v), Some(a)) if v.candidato != "atual" => {
            let perde_para_o_atual = matches!((v.quadros, a.quadros), (Some(q), Some(qa)) if q.fps < qa.fps * 0.99 || q.low_1 < qa.low_1 * 0.99);
            if perde_para_o_atual {
                achados.push(Achado::PlanoAtualJaEOMelhor);
                Some(a)
            } else {
                Some(v)
            }
        }
        (None, Some(a)) if matches!((a.quadros, b.quadros), (Some(qa), Some(q)) if qa.fps >= q.fps * 0.99 && qa.low_1 >= q.low_1 * 0.99) => {
            achados.push(Achado::PlanoAtualJaEOMelhor);
            Some(a)
        }
        (v, _) => v,
    };

    if vencedor.is_none() {
        achados.push(Achado::NenhumGanhouComMargem);
    }

    // CONFIANÇA: repetições, estabilidade das repetições, jogo medido e a
    // margem sobre o segundo colocado.
    let mut confianca: f64 = 95.0;
    let reps = resultados.iter().map(|r| r.fps_repeticoes.len()).min().unwrap_or(0);
    if b.quadros.is_some() && reps < 2 {
        confianca -= 20.0;
        achados.push(Achado::PoucasRepeticoes);
    }
    if b.quadros.is_none() {
        confianca -= 25.0;
    }
    let cv = resultados
        .iter()
        .filter(|r| r.fps_repeticoes.len() >= 2)
        .map(|r| {
            let m = r.fps_repeticoes.iter().sum::<f64>() / r.fps_repeticoes.len() as f64;
            let d = (r.fps_repeticoes.iter().map(|f| (f - m).powi(2)).sum::<f64>() / r.fps_repeticoes.len() as f64).sqrt();
            if m > 0.0 { d / m * 100.0 } else { 0.0 }
        })
        .fold(0.0, f64::max);
    confianca -= (cv * 4.0).min(40.0);
    if cv > 5.0 {
        achados.push(Achado::MedicaoInstavel);
    }
    if let Some(r) = resultados.iter().filter_map(|r| r.resposta).map(|r| r.dispersao_pct).reduce(f64::max) {
        confianca -= (r * 1.5).min(15.0);
    }
    if let (Some(v), [_, segundo, ..]) = (vencedor, elegiveis.as_slice()) {
        let nv = notas.iter().find(|(id, _)| *id == v.candidato).map(|(_, n)| n.total).unwrap_or(0.0);
        if nv - segundo.1.total < 3.0 {
            confianca -= 10.0;
        }
    }
    let confianca_pct = arred(confianca.clamp(5.0, 99.0), 0);

    let (fps_pct, low_1_pct, p99_pct, resposta_pct, temperatura_delta_c) = match vencedor {
        Some(v) => (
            v.quadros.zip(b.quadros).and_then(|(a, q)| variacao(a.fps, q.fps)),
            v.quadros.zip(b.quadros).and_then(|(a, q)| variacao(a.low_1, q.low_1)),
            v.quadros.zip(b.quadros).and_then(|(a, q)| variacao(a.p99_ms, q.p99_ms)),
            v.resposta.zip(b.resposta).and_then(|(a, q)| variacao(a.ate_90_ms, q.ate_90_ms)),
            v.temp_max().zip(b.temp_max()).map(|(a, q)| arred(a - q, 0)),
        ),
        None => (None, None, None, None, None),
    };

    Some(Escolha {
        notas,
        vencedor: vencedor.map(|v| v.candidato.clone()),
        confianca_pct,
        achados,
        fps_pct,
        low_1_pct,
        p99_pct,
        resposta_pct,
        temperatura_delta_c,
    })
}

// ================================================== modo dinâmico de jogo

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EstadoDinamico {
    Normal,
    Jogo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Transicao {
    Nenhuma,
    EntrarNoPerfilDeJogo,
    VoltarAoNormal,
}

/// NORMAL → JOGO ABRIU → PERFIL DE BAIXA LATÊNCIA → JOGO FECHOU → NORMAL.
///
/// Para sair do perfil, o jogo precisa ficar fechado por duas olhadas: uma
/// tela de carregamento que troca de processo não pode derrubar o perfil e
/// subi-lo de novo três segundos depois.
pub fn transicao(estado: EstadoDinamico, jogo_aberto: bool, olhadas_sem_jogo: u32) -> Transicao {
    match (estado, jogo_aberto) {
        (EstadoDinamico::Normal, true) => Transicao::EntrarNoPerfilDeJogo,
        (EstadoDinamico::Jogo, false) if olhadas_sem_jogo >= 2 => Transicao::VoltarAoNormal,
        _ => Transicao::Nenhuma,
    }
}

// ================================================================== testes

#[cfg(test)]
mod tests {
    use super::*;

    const QH: &str = "GUID do Esquema de Energia: 381b4222-f694-41f0-9685-ff5bb260df2e  (Equilibrado)
  GUID de Subgrupos: 54533251-82be-4824-96c1-47b60b740d00  (Gerenciamento de energia do processador)
    Alias GUID: SUB_PROCESSOR
    GUID de Configuração de Energia: 06cadf0e-64ed-448a-8927-ce7bf90eb35d  (Limite de aumento)
      Alias GUID: PERFINCTHRESHOLD
      Configuração Mínima Possível: 0x00000000
      Configuração Máxima Possível: 0x00000064
      Incremento de Configurações Possíveis: 0x00000001
      Unidades de Configurações Possíveis: %
    Índice de Configurações de Correntes Alternadas Atuais: 0x0000003c
    Índice de Configurações de Correntes Contínuas Atuais: 0x0000003c

    GUID de Configuração de Energia: be337238-0d82-4146-a960-4f3749d470c7  (Modo de aumento)
      Alias GUID: PERFBOOSTMODE
      Índice de Configurações Possíveis: 000
      Nome Amigável de Configuração Possível: Desabilitado
      Índice de Configurações Possíveis: 001
      Nome Amigável de Configuração Possível: Habilitado
      Índice de Configurações Possíveis: 002
      Nome Amigável de Configuração Possível: Agressivo
    Índice de Configurações de Correntes Alternadas Atuais: 0x00000001
    Índice de Configurações de Correntes Contínuas Atuais: 0x00000002

    GUID de Configuração de Energia: 36687f9e-e3a5-4dbf-b1dc-15eb381c6863  (EPP)
      Alias GUID: PERFEPP
      Configuração Mínima Possível: 0x00000000
      Configuração Máxima Possível: 0x00000064
      Incremento de Configurações Possíveis: 0x00000001
      Unidades de Configurações Possíveis: %
    Índice de Configurações de Correntes Alternadas Atuais: 0x00000021
    Índice de Configurações de Correntes Contínuas Atuais: 0x00000032

    GUID de Configuração de Energia: 0cc5b647-c1df-4637-891a-dec35c318583  (Núcleos mínimos)
      Alias GUID: CPMINCORES
      Configuração Mínima Possível: 0x00000000
      Configuração Máxima Possível: 0x00000064
      Incremento de Configurações Possíveis: 0x00000001
      Unidades de Configurações Possíveis: %
    Índice de Configurações de Correntes Alternadas Atuais: 0x0000000a
    Índice de Configurações de Correntes Contínuas Atuais: 0x0000000a

    GUID de Configuração de Energia: 8baa4a8a-14c6-4451-8e8b-14bdbd197537  (Autônomo)
      Alias GUID: PERFAUTONOMOUS
      Índice de Configurações Possíveis: 000
      Nome Amigável de Configuração Possível: Desabilitado
      Índice de Configurações Possíveis: 001
      Nome Amigável de Configuração Possível: Habilitado
    Índice de Configurações de Correntes Alternadas Atuais: 0x00000000
    Índice de Configurações de Correntes Contínuas Atuais: 0x00000000

    GUID de Configuração de Energia: bc5038f7-23e0-4960-96da-33abaf5935ec  (Estado máximo)
      Alias GUID: PROCTHROTTLEMAX
      Configuração Mínima Possível: 0x00000000
      Configuração Máxima Possível: 0x00000064
      Incremento de Configurações Possíveis: 0x00000001
      Unidades de Configurações Possíveis: %
    Índice de Configurações de Correntes Alternadas Atuais: 0x00000064
    Índice de Configurações de Correntes Contínuas Atuais: 0x00000064

  GUID de Subgrupos: 501a4d13-42af-4429-9fd1-a8218c268e20  (PCI Express)
    Alias GUID: SUB_PCIEXPRESS
    GUID de Configuração de Energia: ee12f906-d277-404b-b6da-e5fa1a576df5  (Gerenciamento de Energia do Estado de Link)
      Alias GUID: ASPM
      Índice de Configurações Possíveis: 000
      Nome Amigável de Configuração Possível: Desativado
      Índice de Configurações Possíveis: 001
      Nome Amigável de Configuração Possível: Economia de energia moderada
      Índice de Configurações Possíveis: 002
      Nome Amigável de Configuração Possível: Economia de energia máxima
    Índice de Configurações de Correntes Alternadas Atuais: 0x00000001
    Índice de Configurações de Correntes Contínuas Atuais: 0x00000002
";

    fn intel_moderna(formato: Formato) -> Impressao {
        let cpuid = Cpuid { fabricante_bruto: "GenuineIntel".into(), familia: 6, modelo: 0xA5, hwp: true, hwp_epp: true, ..Default::default() };
        Impressao {
            cpu: "Intel(R) Core(TM) i3-10100F".into(),
            fabricante: Fabricante::Intel,
            arquitetura: classificar_arquitetura(&cpuid),
            controle: Controle::HardwareHwp,
            cpuid,
            topologia: Topologia { nucleos_fisicos: 4, processadores_logicos: 8, classes: vec![(0, 4)] },
            formato,
            na_tomada: Some(true),
            build_do_windows: 19045,
            modern_standby: false,
            plano_ativo: None,
            plano_ativo_nome: None,
        }
    }

    fn legada() -> Impressao {
        let cpuid = Cpuid { fabricante_bruto: "GenuineIntel".into(), familia: 6, modelo: 0x3C, ..Default::default() };
        Impressao { arquitetura: classificar_arquitetura(&cpuid), controle: Controle::SistemaOperacional, cpuid, ..intel_moderna(Formato::Desktop) }
    }

    // ---- identificação

    #[test]
    fn familia_e_modelo_de_exibicao() {
        // i3-10100F: "Intel64 Family 6 Model 165" (0xA5) → EAX 0x000A0653.
        assert_eq!(familia_e_modelo(0x000A0653), (6, 0xA5));
        // Ryzen 7 5800X (Zen 3): família 0x19, modelo 0x21 → EAX 0x00A20F10.
        assert_eq!(familia_e_modelo(0x00A20F10), (0x19, 0x21));
        // Ryzen 5 3600 (Zen 2): família 0x17, modelo 0x71 → EAX 0x00870F10.
        assert_eq!(familia_e_modelo(0x00870F10), (0x17, 0x71));
    }

    #[test]
    fn arquiteturas() {
        let amd = |familia, modelo| Cpuid { fabricante_bruto: "AuthenticAMD".into(), familia, modelo, cppc: true, ..Default::default() };
        assert_eq!(classificar_arquitetura(&amd(0x17, 0x08)), Arquitetura::AmdLegada);
        assert_eq!(classificar_arquitetura(&amd(0x17, 0x71)), Arquitetura::Zen2);
        assert_eq!(classificar_arquitetura(&amd(0x19, 0x21)), Arquitetura::Zen3);
        assert_eq!(classificar_arquitetura(&amd(0x19, 0x61)), Arquitetura::Zen4);
        assert_eq!(classificar_arquitetura(&amd(0x19, 0x74)), Arquitetura::Zen4);
        assert_eq!(classificar_arquitetura(&amd(0x1A, 0x44)), Arquitetura::Zen5Mais);
        assert_eq!(classificar_arquitetura(&amd(0x15, 0x02)), Arquitetura::AmdLegada);
        let intel = |hibrido, hwp| Cpuid { fabricante_bruto: "GenuineIntel".into(), familia: 6, modelo: 0x97, hibrido, hwp, ..Default::default() };
        assert_eq!(classificar_arquitetura(&intel(true, true)), Arquitetura::IntelHibrida);
        assert_eq!(classificar_arquitetura(&intel(false, true)), Arquitetura::IntelModerna);
        assert_eq!(classificar_arquitetura(&intel(false, false)), Arquitetura::IntelLegada);
        assert_eq!(controle(&amd(0x19, 0x21), Arquitetura::Zen3), Controle::HardwareCppc);
        assert_eq!(controle(&Cpuid { cppc: false, ..amd(0x19, 0x21) }, Arquitetura::Zen3), Controle::SistemaOperacional);
    }

    #[test]
    fn topologia_hibrida() {
        let t = Topologia { nucleos_fisicos: 14, processadores_logicos: 20, classes: vec![(0, 8), (1, 6)] };
        assert_eq!(t.nucleos_p_e(), Some((6, 8)));
        assert!(t.smt());
        assert_eq!(Topologia { nucleos_fisicos: 4, processadores_logicos: 4, classes: vec![(0, 4)] }.nucleos_p_e(), None);
    }

    // ---- enumeração

    #[test]
    fn le_o_powercfg_em_portugues_sem_depender_dos_rotulos() {
        let e = enumerar(QH);
        assert_eq!(e.plano.as_deref(), Some("381b4222-f694-41f0-9685-ff5bb260df2e"));
        let inc = e.por_alias("PERFINCTHRESHOLD").unwrap();
        assert_eq!((inc.minimo, inc.maximo, inc.incremento, inc.ac, inc.dc), (Some(0), Some(100), Some(1), Some(60), Some(60)));
        assert_eq!(inc.subgrupo, "54533251-82be-4824-96c1-47b60b740d00");
        assert_eq!(inc.subgrupo_alias.as_deref(), Some("SUB_PROCESSOR"));
        let boost = e.por_alias("PERFBOOSTMODE").unwrap();
        assert_eq!(boost.possiveis.len(), 3);
        assert_eq!(boost.possiveis[2], (2, "Agressivo".to_string()));
        assert_eq!((boost.ac, boost.dc), (Some(1), Some(2)));
        assert!(boost.minimo.is_none());
        let aspm = e.por_alias("ASPM").unwrap();
        assert_eq!(aspm.subgrupo, "501a4d13-42af-4429-9fd1-a8218c268e20");
        assert_eq!(e.configuracoes.len(), 7);
    }

    #[test]
    fn le_o_powercfg_em_ingles_igual() {
        let ingles = "Power Scheme GUID: 381b4222-f694-41f0-9685-ff5bb260df2e  (Balanced)
  Subgroup GUID: 54533251-82be-4824-96c1-47b60b740d00  (Processor power management)
    GUID Alias: SUB_PROCESSOR
    Power Setting GUID: be337238-0d82-4146-a960-4f3749d470c7  (Processor performance boost mode)
      GUID Alias: PERFBOOSTMODE
      Possible Setting Index: 000
      Possible Setting Friendly Name: Disabled
      Possible Setting Index: 001
      Possible Setting Friendly Name: Enabled
    Current AC Power Setting Index: 0x00000001
    Current DC Power Setting Index: 0x00000001
";
        let e = enumerar(ingles);
        let b = e.por_alias("perfboostmode").unwrap();
        assert_eq!(b.possiveis, vec![(0, "Disabled".into()), (1, "Enabled".into())]);
        assert_eq!(b.ac, Some(1));
    }

    #[test]
    fn aceita_so_o_que_o_windows_publica() {
        let e = enumerar(QH);
        assert!(e.por_alias("PERFBOOSTMODE").unwrap().aceita(2));
        assert!(!e.por_alias("PERFBOOSTMODE").unwrap().aceita(5));
        assert!(e.por_alias("PERFEPP").unwrap().aceita(100));
        assert!(!e.por_alias("PERFEPP").unwrap().aceita(101));
    }

    // ---- candidatos

    #[test]
    fn nenhum_candidato_e_universal() {
        let base = enumerar(QH);
        let moderna = candidatos_de_autoajuste(&intel_moderna(Formato::Desktop), &base);
        let antiga = candidatos_de_autoajuste(&legada(), &base);
        assert_ne!(moderna[2].parametros, antiga[2].parametros);
        // EPP só em CPU com controle por hardware.
        assert!(moderna[2].mudancas.iter().any(|m| m.alias == "PERFEPP"));
        assert!(!antiga.iter().any(|c| c.mudancas.iter().any(|m| m.alias == "PERFEPP")));
    }

    #[test]
    fn padrao_do_windows_nao_muda_nada() {
        let base = enumerar(QH);
        let c = &candidatos_de_autoajuste(&intel_moderna(Formato::Desktop), &base)[0];
        assert_eq!(c.papel, Papel::PadraoWindows);
        assert!(c.mudancas.is_empty());
    }

    #[test]
    fn epp_vai_para_a_escala_do_windows() {
        assert_eq!(epp_para_percentual(0), 0);
        assert_eq!(epp_para_percentual(32), 13);
        assert_eq!(epp_para_percentual(64), 25);
        assert_eq!(epp_para_percentual(128), 50);
        assert_eq!(epp_para_percentual(255), 100);
        let base = enumerar(QH);
        let escada = escada_de_epp(&intel_moderna(Formato::Desktop), &base);
        assert_eq!(escada.len(), 5);
        assert_eq!(escada[3].mudancas.iter().find(|m| m.alias == "PERFEPP").unwrap().ac, Some(25));
        assert!(escada_de_epp(&legada(), &base).is_empty());
    }

    #[test]
    fn notebook_nunca_mexe_na_bateria() {
        let base = enumerar(QH);
        for c in candidatos_de_autoajuste(&intel_moderna(Formato::Notebook), &base) {
            assert!(c.mudancas.iter().all(|m| m.dc.is_none()), "{} tocou a bateria", c.id);
        }
        for c in candidatos_de_autoajuste(&intel_moderna(Formato::Desktop), &base) {
            assert!(c.mudancas.iter().all(|m| m.dc == m.ac));
        }
    }

    #[test]
    fn minimo_alto_so_em_cpu_sem_controle_de_hardware_e_desktop() {
        let base = enumerar(QH);
        let hw = candidatos_de_autoajuste(&intel_moderna(Formato::Desktop), &base);
        assert!(hw.iter().all(|c| c.parametros.minimo.is_none()));
        let so = candidatos_de_autoajuste(&legada(), &base);
        assert_eq!(so[1].parametros.minimo, Some(100));
        let note = candidatos_de_autoajuste(&Impressao { formato: Formato::Notebook, ..legada() }, &base);
        assert!(note.iter().all(|c| c.parametros.minimo.is_none()));
    }

    #[test]
    fn ajuste_ausente_vira_ignorado_e_nao_erro() {
        let base = enumerar(QH);
        let c = gerar(&intel_moderna(Formato::Desktop), &base, "x", Papel::B, Parametros { resposta: Resposta::Rapida, ..Parametros::WINDOWS });
        assert!(c.ignorados.iter().any(|i| i.alias == "PERFINCTIME" && i.motivo == MotivoIgnorado::NaoExiste));
        assert!(c.mudancas.iter().any(|m| m.alias == "PERFINCTHRESHOLD" && m.ac == Some(30)));
    }

    #[test]
    fn autonomia_so_com_hardware() {
        let base = enumerar(QH);
        let p = Parametros { autonomia: Autonomia::Hardware, ..Parametros::WINDOWS };
        assert!(gerar(&intel_moderna(Formato::Desktop), &base, "x", Papel::C, p).mudancas.iter().any(|m| m.alias == "PERFAUTONOMOUS"));
        let l = gerar(&legada(), &base, "x", Papel::C, p);
        assert!(l.ignorados.iter().any(|i| i.alias == "PERFAUTONOMOUS" && i.motivo == MotivoIgnorado::CpuNaoSuporta));
    }

    #[test]
    fn dispositivos_so_como_variacao() {
        let base = enumerar(QH);
        let i = intel_moderna(Formato::Desktop);
        assert!(candidatos_de_autoajuste(&i, &base).iter().all(|c| !c.mudancas.iter().any(|m| m.alias == "ASPM")));
        let v = variacoes_de_dispositivo(&i, &base, Parametros::WINDOWS);
        assert!(v[0].mudancas.iter().any(|m| m.alias == "ASPM" && m.ac == Some(0)));
    }

    #[test]
    fn plano_de_escrita_volta_ao_base_e_so_escreve_o_diferente() {
        let base = enumerar(QH);
        let i = intel_moderna(Formato::Desktop);
        let todos = candidatos_de_autoajuste(&i, &base);
        let controlados = apelidos_controlados(&todos);
        // Plano atual está no candidato A; aplicar o padrão precisa desfazer A.
        let mut atual = base.clone();
        for m in &todos[1].mudancas {
            if let Some(c) = atual.configuracoes.iter_mut().find(|c| c.alias.as_deref() == Some(m.alias.as_str())) {
                c.ac = m.ac.or(c.ac);
                c.dc = m.dc.or(c.dc);
            }
        }
        let escrita = plano_de_escrita(&todos[0], &controlados, &base, &atual);
        assert!(escrita.iter().any(|m| m.alias == "PERFEPP" && m.ac == Some(33)));
        assert!(escrita.iter().any(|m| m.alias == "CPMINCORES" && m.ac == Some(10)));
        // Nada a escrever quando já está no alvo.
        assert!(plano_de_escrita(&todos[0], &controlados, &base, &base).is_empty());
    }

    #[test]
    fn divergencia_depois_de_reler() {
        let base = enumerar(QH);
        let pedida = vec![Mudanca { alias: "PERFEPP".into(), subgrupo: String::new(), guid: String::new(), ac: Some(0), dc: None, motivo: Motivo::EppDoCandidato }];
        assert_eq!(divergencias(&pedida, &base), vec!["PERFEPP".to_string()]);
    }

    // ---- rajada

    #[test]
    fn rajada_que_sobe_devagar_demora_mais() {
        let rapida: Vec<f64> = (0..40).map(|i| if i < 2 { 80.0 } else { 100.0 }).collect();
        let lenta: Vec<f64> = (0..40).map(|i| (40.0 + i as f64 * 5.0).min(100.0)).collect();
        let r = analisar_rajadas(&[rapida.clone(), rapida], 5.0).unwrap();
        let l = analisar_rajadas(&[lenta.clone(), lenta], 5.0).unwrap();
        assert_eq!(r.ate_90_ms, 15.0);
        assert!(l.ate_90_ms > r.ate_90_ms);
        assert!(l.primeira_fatia_pct < r.primeira_fatia_pct);
        assert_eq!(r.rajadas, 2);
        assert!(analisar_rajadas(&[vec![1.0; 3]], 5.0).is_none());
    }

    // ---- cpu

    /// Uma amostra completa, como o PDH entrega quando tudo responde.
    fn amostra_cheia() -> AmostraCpu {
        AmostraCpu {
            desempenho_pct: Some(120.0),
            frequencia_mhz: Some(3600.0),
            uso_pct: Some(50.0),
            limite_pct: Some(100.0),
            flags: Some(0),
            temperatura_c: Some(70.0),
            ..Default::default()
        }
    }

    #[test]
    fn clock_efetivo_e_limites() {
        let a = amostra_cheia();
        let b = AmostraCpu { flags: Some(1), limite_pct: Some(80.0), temperatura_c: Some(96.0), ..a };
        let r = resumir_cpu(&[a, a, b]).expect("três amostras completas");
        assert_eq!(r.clock_reportado_mhz, 4320.0);
        assert_eq!(r.clock_efetivo_mhz, 2160.0);
        assert_eq!(r.limite_termico, Some(true));
        assert_eq!(r.limite_eletrico, Some(false));
        assert_eq!(r.temperatura_max_c, Some(96.0));
        assert_eq!(r.amostras_descartadas, 0);
        assert_eq!(folga_termica(&r), Some(0.0));

        let sem_sensor = resumir_cpu(&[AmostraCpu { temperatura_c: None, ..a }]).expect("completa");
        assert_eq!(folga_termica(&sem_sensor), None);
    }

    #[test]
    fn contador_mudo_nao_vira_leitura() {
        // O defeito: `unwrap_or(0.0)` e `unwrap_or(100.0)` no amostrador
        // transformavam PDH calado em "CPU a 0%" e "firmware sem limite".
        let a = amostra_cheia();

        // Sem os contadores de clock a amostra não serve e é descartada — não
        // vira média puxada para baixo.
        let cega = AmostraCpu { desempenho_pct: None, ..a };
        let r = resumir_cpu(&[a, cega, a]).expect("duas amostras boas");
        assert_eq!(r.amostras, 2);
        assert_eq!(r.amostras_descartadas, 1);
        assert_eq!(r.clock_efetivo_mhz, 2160.0, "a amostra cega não entrou na média");

        // Sem flags nem limite, o motor não afirma que a CPU está livre.
        let sem_limite = AmostraCpu { flags: None, limite_pct: None, ..a };
        let r = resumir_cpu(&[sem_limite; 4]).expect("clock presente");
        assert_eq!(r.limite_termico, None);
        assert_eq!(r.limite_eletrico, None);
        assert_eq!(r.tempo_limitado_pct, None);
        assert_eq!(r.limite_medio_pct, None);

        // Nenhuma amostra utilizável: resumo nenhum, e não um resumo zerado.
        assert_eq!(resumir_cpu(&[]), None);
        assert_eq!(resumir_cpu(&[AmostraCpu::default(); 5]), None);
    }

    #[test]
    fn flag_nao_lida_nao_vira_folga_termica() {
        let muda = AmostraCpu { flags: None, temperatura_c: None, ..amostra_cheia() };
        let r = resumir_cpu(&[muda; 3]).expect("clock presente");
        assert_eq!(folga_termica(&r), None, "sem sensor e sem flag, a resposta é não sei");

        let com_temp = resumir_cpu(&[AmostraCpu { temperatura_c: Some(90.0), ..muda }; 3]).expect("clock");
        assert_eq!(folga_termica(&com_temp), Some(10.0), "90 °C dá folga baixa, não folga inventada");
    }

    #[test]
    fn zona_termica_parada_sob_carga_e_descartada() {
        let a = AmostraCpu {
            desempenho_pct: Some(100.0),
            frequencia_mhz: Some(3600.0),
            uso_pct: Some(90.0),
            limite_pct: Some(100.0),
            temperatura_c: Some(27.85),
            ..Default::default()
        };
        assert_eq!(resumir_cpu(&[a; 10]).unwrap().temperatura_max_c, None);
        let ociosa = AmostraCpu { uso_pct: Some(3.0), ..a };
        assert_eq!(resumir_cpu(&[ociosa; 10]).unwrap().temperatura_max_c, Some(28.0));
    }

    #[test]
    fn quadros_com_01_low() {
        let mut v = vec![10.0; 2000];
        v[0] = 100.0;
        v[1] = 100.0;
        let q = resumir_quadros(100.0, &v).unwrap();
        assert_eq!(q.low_01, 10.0);
        assert!(q.low_1 < 100.0);
        assert!(resumir_quadros(60.0, &[16.0; 10]).is_none());
    }

    // ---- escolha

    fn resultado(id: &str, fps: f64, low: f64, p99: f64, ate90: f64, temp: f64, clock: (f64, f64)) -> ResultadoDoCandidato {
        ResultadoDoCandidato {
            candidato: id.into(),
            resposta: Some(RespostaMedida { ate_90_ms: ate90, primeira_fatia_pct: 80.0, sustentado: 100.0, dispersao_pct: 1.0, rajadas: 5 }),
            cpu: Some(ResumoCpu { amostras: 20, clock_reportado_mhz: clock.0, clock_efetivo_mhz: clock.1, limite_medio_pct: Some(100.0), tempo_limitado_pct: Some(0.0), limite_termico: Some(false), limite_eletrico: Some(false), temperatura_max_c: Some(temp), ..Default::default() }),
            quadros: Some(Quadros { fps, low_1: low, low_01: low * 0.8, p99_ms: p99 }),
            fps_repeticoes: vec![fps, fps],
            gpu_pct: None,
            uso_cpu_pct: None,
            quadros_sinteticos: false,
        }
    }

    #[test]
    fn candidato_que_tira_fps_nunca_ganha() {
        let base = resultado("windows", 100.0, 60.0, 20.0, 40.0, 70.0, (3600.0, 1800.0));
        // Resposta e 1% low melhores, mas 3% menos FPS médio: fora.
        let a = resultado("a", 97.0, 66.0, 17.0, 15.0, 70.0, (3700.0, 1900.0));
        let e = escolher(&[base, a], "windows").unwrap();
        assert_eq!(e.vencedor, None);
        assert!(e.achados.contains(&Achado::CandidatoTirariaFps));
    }

    #[test]
    fn plano_atual_e_o_piso() {
        let base = resultado("windows", 90.0, 50.0, 22.0, 40.0, 70.0, (3600.0, 1800.0));
        let atual = resultado("atual", 110.0, 70.0, 16.0, 8.0, 72.0, (4000.0, 2000.0));
        // B ganha do Windows com folga, mas perde do plano que a pessoa já usa.
        let b = resultado("b", 104.0, 64.0, 17.0, 10.0, 71.0, (3900.0, 1950.0));
        let e = escolher(&[base, atual, b], "windows").unwrap();
        assert_eq!(e.vencedor.as_deref(), Some("atual"));
        assert!(e.achados.contains(&Achado::PlanoAtualJaEOMelhor));
    }

    #[test]
    fn vizinhos_exploram_em_volta_do_vencedor() {
        let p = Parametros { epp_bruto: Some(32), estacionamento: Estacionamento::Adaptativo, resposta: Resposta::Rapida, ..Parametros::WINDOWS };
        let v = vizinhos_do_vencedor(p, &intel_moderna(Formato::Desktop));
        assert!(v.iter().any(|x| x.epp_bruto == Some(16)));
        assert!(v.iter().any(|x| x.epp_bruto == Some(48)));
        assert!(v.iter().any(|x| x.estacionamento == Estacionamento::TodosAcordados && x.epp_bruto == Some(32)));
        assert!(!v.contains(&p));
        // EPP 0 não desce abaixo de zero, e notebook não ganha "todos acordados".
        let zero = Parametros { epp_bruto: Some(0), ..p };
        let note = vizinhos_do_vencedor(zero, &intel_moderna(Formato::Notebook));
        assert_eq!(note, vec![Parametros { epp_bruto: Some(16), ..zero }]);
        // CPU sem EPP: só o estacionamento varia.
        assert!(vizinhos_do_vencedor(p, &legada()).iter().all(|x| x.epp_bruto == Some(32)));
    }

    #[test]
    fn candidato_d_so_em_desktop() {
        let base = enumerar(QH);
        assert!(candidatos_de_autoajuste(&intel_moderna(Formato::Desktop), &base).iter().any(|c| c.papel == Papel::D));
        assert!(!candidatos_de_autoajuste(&intel_moderna(Formato::Notebook), &base).iter().any(|c| c.papel == Papel::D));
        let d = candidatos_de_autoajuste(&intel_moderna(Formato::Desktop), &base).into_iter().find(|c| c.papel == Papel::D).unwrap();
        assert!(d.mudancas.iter().any(|m| m.alias == "PERFBOOSTMODE" && m.ac == Some(2)));
        assert!(d.mudancas.iter().any(|m| m.alias == "PERFEPP" && m.ac == Some(0)));
    }

    #[test]
    fn fps_medio_sozinho_nao_ganha() {
        let base = resultado("windows", 100.0, 60.0, 20.0, 40.0, 70.0, (3600.0, 1800.0));
        // Mais FPS médio, mas 1% low pior: regressão.
        let a = resultado("a", 115.0, 50.0, 24.0, 30.0, 72.0, (3800.0, 1900.0));
        let e = escolher(&[base, a], "windows").unwrap();
        assert_eq!(e.vencedor, None);
        assert!(e.achados.contains(&Achado::NenhumGanhouComMargem));
    }

    #[test]
    fn ganha_o_que_melhora_low_e_resposta() {
        let base = resultado("windows", 100.0, 60.0, 20.0, 40.0, 70.0, (3600.0, 1800.0));
        let b = resultado("b", 104.0, 68.0, 17.0, 20.0, 71.0, (3700.0, 1900.0));
        let e = escolher(&[base, b], "windows").unwrap();
        assert_eq!(e.vencedor.as_deref(), Some("b"));
        assert!(e.low_1_pct.unwrap() > 10.0);
        assert!(e.p99_pct.unwrap() < 0.0);
        assert!(e.confianca_pct > 70.0);
    }

    #[test]
    fn regressao_termica_sem_ganho_de_low_perde() {
        let base = resultado("windows", 100.0, 60.0, 20.0, 40.0, 70.0, (3600.0, 1800.0));
        let quente = resultado("a", 106.0, 61.0, 19.0, 25.0, 88.0, (4000.0, 2000.0));
        assert_eq!(escolher(&[base, quente], "windows").unwrap().vencedor, None);
    }

    #[test]
    fn clock_reportado_maior_com_efetivo_menor_nao_e_melhora() {
        let base = resultado("windows", 100.0, 60.0, 20.0, 40.0, 70.0, (3600.0, 1800.0));
        let enganoso = resultado("a", 104.0, 66.0, 18.0, 30.0, 70.0, (4200.0, 1600.0));
        let e = escolher(&[base, enganoso], "windows").unwrap();
        assert_eq!(e.vencedor, None);
        assert!(e.achados.contains(&Achado::ClockReportadoSubiuEfetivoCaiu));
    }

    #[test]
    fn empate_vai_para_o_mais_fresco() {
        let base = resultado("windows", 100.0, 60.0, 20.0, 40.0, 70.0, (3600.0, 1800.0));
        let a = resultado("a", 105.0, 70.0, 17.0, 20.0, 78.0, (3700.0, 1900.0));
        let c = resultado("c", 105.0, 66.5, 17.0, 21.0, 71.0, (3700.0, 1880.0));
        let e = escolher(&[base, a, c], "windows").unwrap();
        assert_eq!(e.vencedor.as_deref(), Some("c"));
        assert!(e.achados.contains(&Achado::EmpateDesfeitoPelaTemperatura));
    }

    #[test]
    fn firmware_limitado_e_dito() {
        let mut base = resultado("windows", 100.0, 60.0, 20.0, 40.0, 70.0, (3600.0, 1800.0));
        base.cpu.as_mut().expect("resultado de teste tem resumo").limite_eletrico = Some(true);
        let e = escolher(&[base], "windows").unwrap();
        assert!(e.achados.contains(&Achado::FirmwareLimitaPorEnergia));
    }

    #[test]
    fn medicao_instavel_derruba_confianca() {
        let base = resultado("windows", 100.0, 60.0, 20.0, 40.0, 70.0, (3600.0, 1800.0));
        let mut b = resultado("b", 104.0, 68.0, 17.0, 20.0, 71.0, (3700.0, 1900.0));
        b.fps_repeticoes = vec![90.0, 118.0];
        let e = escolher(&[base, b], "windows").unwrap();
        assert!(e.achados.contains(&Achado::MedicaoInstavel));
        assert!(e.confianca_pct < 60.0);
    }

    // ---- dinâmico

    #[test]
    fn transicoes_do_modo_dinamico() {
        assert_eq!(transicao(EstadoDinamico::Normal, true, 0), Transicao::EntrarNoPerfilDeJogo);
        assert_eq!(transicao(EstadoDinamico::Jogo, true, 0), Transicao::Nenhuma);
        assert_eq!(transicao(EstadoDinamico::Jogo, false, 1), Transicao::Nenhuma);
        assert_eq!(transicao(EstadoDinamico::Jogo, false, 2), Transicao::VoltarAoNormal);
        assert_eq!(transicao(EstadoDinamico::Normal, false, 9), Transicao::Nenhuma);
    }

    #[test]
    fn nunca_desliga_nucleo_e_nem_usa_tempo_real() {
        let fonte = include_str!("motorenergia.rs");
        let codigo = fonte.split("#[cfg(test)]").next().unwrap();
        for proibido in ["REALTIME_PRIORITY", "SetProcessAffinityMask", "HETEROPOLICY, 0", "CPMAXCORES, 0"] {
            assert!(!codigo.contains(proibido), "{}", proibido);
        }
    }
}
