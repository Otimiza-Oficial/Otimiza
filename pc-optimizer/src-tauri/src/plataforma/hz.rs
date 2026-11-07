// "Seu monitor aguenta 165 Hz e está em 60" (Otimiza Tela, PLANO-1.0 7.4). Por monitor (`\\.\DISPLAYn`), nunca só o
// principal: o gerador de quadros já pagou por usar o Hz do principal num segundo monitor.
//
// Duas fontes, e cada uma responde uma pergunta diferente:
// - Os modos que o Windows lista (`EnumDisplaySettingsExW`, todos os modos): o que a PLACA, o CABO e a PORTA
//   entregam agora. É a mesma função que aplica: se lista, acontece. É daqui que sai o "um clique".
// - A EDID do monitor (`Enum\DISPLAY\<modelo>\<instância>\Device Parameters\EDID`): o que o MONITOR anuncia. Só
//   as temporizações detalhadas (DTD da base e do CTA-861, e as tipo I/VII do DisplayID) contam, porque cada uma
//   traz resolução E taxa. O descritor de faixa (0xFD) vai junto só como informação: ele vale para o monitor
//   inteiro, e em monitor de dois modos (1440p 165 / 1080p 240) a faixa diz 240 sem que 1440p aceite 240.
//   Acusar cabo com ela seria afirmar sem fonte.
//
// O monitor anuncia mais do que a ligação entrega nesta resolução: o gargalo está no meio (cabo, porta, versão do
// HDMI). Não é "um clique": o conserto é físico, e a tela diz qual.
//
// A troca é como a do próprio Windows ("Manter estas configurações?"): aplica sem gravar, espera a pessoa
// confirmar e só então grava no registro e no histórico. Sem confirmação em 15 s, volta sozinha. A volta mora aqui
// no Rust, com relógio próprio: tela preta é justamente o caso em que ninguém consegue clicar em "voltar".

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// O mesmo corte do diagnóstico antigo (`windows::display::DIFERENCA_QUE_IMPORTA`, que só existe no Windows): 165 →
/// 180 é um degrau, 165 → 170 (modo "overclock" de alguns monitores) não paga uma troca de modo que pisca a tela.
pub const GANHO_MINIMO_HZ: u32 = 15;

/// Taxas fracionárias: 59,94 sai do Windows como 59 e da EDID como 60; 143,98 como 143 e 144. É o mesmo degrau.
pub const TOLERANCIA_DO_DEGRAU_HZ: u32 = 1;

/// O prazo do "Manter estas configurações?" do Windows é 15 s. Mais curto, quem procura o botão não acha a tempo.
pub const PRAZO_PARA_CONFIRMAR: Duration = Duration::from_secs(15);

/// Prefixo do id no histórico. O antigo `monitor:` (`set_max_refresh_rate`) guarda só a taxa; este guarda o modo.
pub const PREFIXO_DO_ID: &str = "tela:";

pub fn id_no_historico(dispositivo: &str) -> String {
    format!("{}{}", PREFIXO_DO_ID, dispositivo.to_lowercase())
}

// ---------------------------------------------------------------------------------------------------------------
// Tipos

/// Os campos do `DEVMODEW` que descrevem o modo de um monitor na área de trabalho. É o que o desfazer devolve,
/// LIDO antes da troca, nunca montado de cabeça.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct ModoDeVideo {
    pub largura: u32,
    pub altura: u32,
    pub bits: u32,
    pub hz: u32,
    /// `dmDisplayFlags` (entrelaçado etc.).
    pub flags: u32,
    /// `dmDisplayOrientation`: 0, 1 (90°), 2 (180°), 3 (270°).
    pub orientacao: u32,
    pub x: i32,
    pub y: i32,
}

/// Um modo da lista do Windows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct Modo {
    pub largura: u32,
    pub altura: u32,
    pub bits: u32,
    pub hz: u32,
    pub entrelacado: bool,
}

/// Uma temporização detalhada da EDID, com a taxa em milésimos de Hz (59 940 = 59,94 Hz).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct Temporizacao {
    pub largura: u32,
    pub altura: u32,
    pub mhz: u32,
}

impl Temporizacao {
    pub fn hz(&self) -> u32 {
        (self.mhz + 500) / 1000
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct Edid {
    /// Nome gravado no monitor (descritor 0xFC).
    pub nome: Option<String>,
    /// A primeira temporização da base: na EDID 1.3+ é a preferida, a resolução nativa do painel.
    pub preferida: Option<Temporizacao>,
    /// Todas as temporizações progressivas (base, CTA-861, DisplayID).
    pub temporizacoes: Vec<Temporizacao>,
    /// Máximo vertical do descritor de faixa (0xFD). Só informação: vale para o monitor inteiro, não por resolução.
    pub faixa_max_hz: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(specta::Type))]
#[serde(tag = "estado")]
pub enum LeituraEdid {
    Lida(Edid),
    /// O Windows não ligou este monitor a uma instância com EDID (adaptador virtual, monitor sem EDID).
    NaoAchada,
    /// Achou e não conseguiu ler, ou o que leu não é EDID.
    Ilegivel { motivo: String },
}

/// O que se lê de um monitor, sem decidir nada.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LeituraDaTela {
    pub dispositivo: String,
    pub nome: String,
    pub principal: bool,
    /// `None`: o Windows não devolveu o modo atual.
    pub atual: Option<ModoDeVideo>,
    pub modos: Vec<Modo>,
    pub edid: LeituraEdid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(specta::Type))]
pub enum FonteDaNativa {
    /// Temporização preferida da EDID.
    Edid,
    /// Sem EDID: a maior resolução que o Windows lista. Quase sempre é a nativa, mas é dedução.
    MaiorModoListado,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct Nativa {
    pub largura: u32,
    pub altura: u32,
    pub fonte: FonteDaNativa,
}

/// O cabo ou a porta segurando a taxa. Só se afirma com temporização da EDID na MESMA resolução.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(specta::Type))]
#[cfg_attr(test, serde(rename = "CaboDaTela"))]
#[serde(tag = "estado")]
pub enum Cabo {
    /// O monitor anuncia `hz_anunciado` em `largura`x`altura`, e o Windows só oferece `hz_oferecido` ali.
    Limitando {
        largura: u32,
        altura: u32,
        hz_oferecido: u32,
        hz_anunciado_mhz: u32,
    },
    /// A EDID tem temporização nesta resolução, e o Windows oferece o mesmo degrau (ou mais).
    NaoLimita,
    /// A EDID não traz temporização na resolução atual nem na nativa: não há como comparar.
    SemTemporizacaoNaResolucao,
    EdidNaoAchada,
    EdidIlegivel { motivo: String },
}

/// A resposta de cada monitor, um estado nomeado (nada de "sem achado" quando a leitura falhou).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(specta::Type))]
#[cfg_attr(test, serde(rename = "VereditoDaTela"))]
#[serde(tag = "estado")]
pub enum Veredito {
    /// Já está no maior degrau desta resolução.
    NoMaximo { hz_atual: u32 },
    /// (a) Taxa maior na MESMA resolução e profundidade de cor: um clique, reversível.
    UmClique { hz_atual: u32, hz_alvo: u32 },
    /// (c) O monitor anuncia mais do que a ligação entrega: troca de cabo ou de porta, não um clique.
    CaboOuPortaLimitando {
        hz_atual: u32,
        hz_oferecido: u32,
        hz_anunciado: u32,
    },
    /// (b) Taxa maior só em OUTRA resolução: explica a troca, não oferece sozinho.
    SoEmOutraResolucao {
        hz_atual: u32,
        largura: u32,
        altura: u32,
        hz: u32,
    },
    /// O Windows não devolveu o modo atual, ou a lista de modos veio sem o modo atual.
    ModosIlegiveis,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct DiagnosticoDaTela {
    pub dispositivo: String,
    pub nome: String,
    pub principal: bool,
    pub atual: Option<ModoDeVideo>,
    /// Maior taxa que o Windows oferece na resolução e profundidade atuais.
    pub hz_maximo_aqui: Option<u32>,
    pub nativa: Option<Nativa>,
    pub hz_maximo_na_nativa: Option<u32>,
    pub cabo: Cabo,
    pub faixa_max_hz: Option<u32>,
    pub veredito: Veredito,
    /// A frase da tela, já com a ressalva de FPS onde cabe.
    pub texto: String,
}

// ---------------------------------------------------------------------------------------------------------------
// Puras: degraus e EDID

/// Pura. 59 e 60, 143 e 144, 239 e 240 são o mesmo degrau.
pub fn mesmo_degrau(a_hz: u32, b_hz: u32) -> bool {
    a_hz.abs_diff(b_hz) <= TOLERANCIA_DO_DEGRAU_HZ
}

/// Pura. Vale a troca: degrau diferente e ganho de pelo menos [`GANHO_MINIMO_HZ`].
pub fn ganho_que_importa(de_hz: u32, para_hz: u32) -> bool {
    !mesmo_degrau(de_hz, para_hz) && para_hz >= de_hz + GANHO_MINIMO_HZ
}

/// Pura: taxa em mHz de uma temporização. `None` com total zerado (descritor que não é temporização).
fn taxa_mhz(pixel_hz: u64, h_total: u32, v_total: u32) -> Option<u32> {
    let total = h_total as u64 * v_total as u64;
    if pixel_hz == 0 || total == 0 {
        return None;
    }
    u32::try_from((pixel_hz * 1000 + total / 2) / total).ok()
}

/// Pura: um descritor de 18 bytes (base e CTA-861). `None` se não é temporização (relógio zero) ou é entrelaçada
/// (a taxa calculada seria a de campos, não a de quadros, e jogo não usa entrelaçado).
pub fn ler_dtd(d: &[u8]) -> Option<Temporizacao> {
    if d.len() < 18 {
        return None;
    }
    let relogio = u16::from_le_bytes([d[0], d[1]]) as u64 * 10_000;
    if relogio == 0 || d[17] & 0x80 != 0 {
        return None;
    }
    let largura = d[2] as u32 | ((d[4] as u32 & 0xF0) << 4);
    let h_branco = d[3] as u32 | ((d[4] as u32 & 0x0F) << 8);
    let altura = d[5] as u32 | ((d[7] as u32 & 0xF0) << 4);
    let v_branco = d[6] as u32 | ((d[7] as u32 & 0x0F) << 8);
    if largura == 0 || altura == 0 {
        return None;
    }
    Some(Temporizacao {
        largura,
        altura,
        mhz: taxa_mhz(relogio, largura + h_branco, altura + v_branco)?,
    })
}

/// Pura: temporização detalhada do DisplayID (tipo I do 1.x, em 10 kHz; tipo VII do 2.0, em 1 kHz). 20 bytes,
/// cada campo gravado como valor menos 1.
fn ler_displayid(d: &[u8], unidade_hz: u64) -> Option<Temporizacao> {
    if d.len() < 20 {
        return None;
    }
    let campo = |i: usize| u16::from_le_bytes([d[i], d[i + 1]]) as u32 + 1;
    let relogio = (u32::from_le_bytes([d[0], d[1], d[2], 0]) as u64 + 1) * unidade_hz;
    // Bit 4 do byte de opções: entrelaçado.
    if d[3] & 0x10 != 0 {
        return None;
    }
    let (largura, h_branco, altura, v_branco) = (campo(4), campo(6), campo(12), campo(14));
    Some(Temporizacao {
        largura,
        altura,
        mhz: taxa_mhz(relogio, largura + h_branco, altura + v_branco)?,
    })
}

/// Pura: os blocos de dados de uma extensão DisplayID (tag 0x70 na EDID).
fn temporizacoes_displayid(ext: &[u8], saida: &mut Vec<Temporizacao>) {
    // ext[0] = 0x70; a seção começa em ext[1]: versão, tamanho, tipo de produto, contagem de extensões.
    let tamanho = ext.get(2).copied().unwrap_or(0) as usize;
    let fim = (6 + tamanho).min(ext.len().saturating_sub(1));
    let mut i = 5;
    while i + 3 <= fim {
        let (tag, n) = (ext[i], ext[i + 2] as usize);
        let carga = match ext.get(i + 3..(i + 3 + n).min(fim)) {
            Some(c) => c,
            None => break,
        };
        let unidade = match tag {
            0x03 => Some(10_000),
            0x22 => Some(1_000),
            _ => None,
        };
        if let Some(u) = unidade {
            saida.extend(carga.chunks_exact(20).filter_map(|t| ler_displayid(t, u)));
        }
        if n == 0 && tag == 0 {
            break;
        }
        i += 3 + n;
    }
}

/// Pura. `None` se não for EDID.
pub fn ler_edid(b: &[u8]) -> Option<Edid> {
    if b.len() < 128 || b[0..8] != [0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00] {
        return None;
    }

    let mut nome = None;
    let mut faixa_max_hz = None;
    let mut preferida = None;
    let mut temporizacoes = Vec::new();

    for (n, inicio) in [54usize, 72, 90, 108].into_iter().enumerate() {
        let d = &b[inicio..inicio + 18];
        if d[0] != 0 || d[1] != 0 {
            if let Some(t) = ler_dtd(d) {
                if n == 0 {
                    preferida = Some(t);
                }
                temporizacoes.push(t);
            }
            continue;
        }
        match d[3] {
            0xFC => {
                let texto: String = d[5..18]
                    .iter()
                    .take_while(|c| **c != 0x0A && **c != 0)
                    .map(|c| *c as char)
                    .collect::<String>()
                    .trim()
                    .to_string();
                if !texto.is_empty() {
                    nome = Some(texto);
                }
            }
            0xFD => {
                // EDID 1.4: o bit 1 do byte 4 soma 255 ao máximo vertical.
                faixa_max_hz = Some(d[6] as u32 + if d[4] & 0x02 != 0 { 255 } else { 0 });
            }
            _ => {}
        }
    }

    for i in 1..=b[126] as usize {
        let Some(ext) = b.get(i * 128..i * 128 + 128) else {
            break;
        };
        match ext[0] {
            0x02 => {
                // CTA-861: as DTDs começam no deslocamento do byte 2 e vão até um relógio zerado.
                let mut j = ext[2] as usize;
                while j >= 4 && j + 18 <= 127 {
                    match ler_dtd(&ext[j..j + 18]) {
                        Some(t) => temporizacoes.push(t),
                        None if ext[j] == 0 && ext[j + 1] == 0 => break,
                        None => {}
                    }
                    j += 18;
                }
            }
            0x70 => temporizacoes_displayid(ext, &mut temporizacoes),
            _ => {}
        }
    }

    Some(Edid {
        nome,
        preferida,
        temporizacoes,
        faixa_max_hz,
    })
}

/// Pura: o caminho de interface do monitor (`EDD_GET_DEVICE_INTERFACE_NAME`) aponta a instância EXATA no registro:
/// `\\?\DISPLAY#AOCB401#5&1a2b3c&0&UID4352#{e6f07b5f-...}` → (`AOCB401`, `5&1a2b3c&0&UID4352`). Sem isto, dois
/// monitores do mesmo modelo (ou o mesmo em duas portas) dividiriam uma EDID só.
pub fn instancia_do_caminho(caminho: &str) -> Option<(String, String)> {
    let sem_prefixo = caminho
        .strip_prefix(r"\\?\")
        .unwrap_or(caminho);
    let mut partes = sem_prefixo.split('#');
    partes
        .next()
        .filter(|p| p.eq_ignore_ascii_case("DISPLAY"))?;
    let modelo = partes.next().filter(|m| !m.is_empty())?;
    let instancia = partes.next().filter(|i| !i.is_empty())?;
    Some((modelo.to_string(), instancia.to_string()))
}

// ---------------------------------------------------------------------------------------------------------------
// Pura: o diagnóstico

fn maior_hz<'a>(modos: impl Iterator<Item = &'a Modo>) -> Option<u32> {
    modos.filter(|m| !m.entrelacado && m.hz > 1).map(|m| m.hz).max()
}

fn anunciado_em(edid: &Edid, largura: u32, altura: u32) -> Option<u32> {
    edid.temporizacoes
        .iter()
        .filter(|t| t.largura == largura && t.altura == altura)
        .map(|t| t.mhz)
        .max()
}

/// Pura. Uma tela, um veredito. Nada aqui lê a máquina.
pub fn diagnosticar(t: &LeituraDaTela) -> DiagnosticoDaTela {
    let edid = match &t.edid {
        LeituraEdid::Lida(e) => Some(e),
        _ => None,
    };
    let nome = edid
        .and_then(|e| e.nome.clone())
        .unwrap_or_else(|| t.nome.clone());

    let base = |veredito: Veredito,
                hz_maximo_aqui,
                nativa,
                hz_maximo_na_nativa,
                cabo: Cabo| {
        let texto = texto_do_veredito(&nome, &veredito, &cabo);
        DiagnosticoDaTela {
            dispositivo: t.dispositivo.clone(),
            nome: nome.clone(),
            principal: t.principal,
            atual: t.atual,
            hz_maximo_aqui,
            nativa,
            hz_maximo_na_nativa,
            cabo,
            faixa_max_hz: edid.and_then(|e| e.faixa_max_hz),
            veredito,
            texto,
        }
    };

    let cabo_sem_edid = || match &t.edid {
        LeituraEdid::NaoAchada => Cabo::EdidNaoAchada,
        LeituraEdid::Ilegivel { motivo } => Cabo::EdidIlegivel {
            motivo: motivo.clone(),
        },
        LeituraEdid::Lida(_) => Cabo::SemTemporizacaoNaResolucao,
    };

    let Some(atual) = t.atual.filter(|a| a.hz > 1) else {
        return base(Veredito::ModosIlegiveis, None, None, None, cabo_sem_edid());
    };

    let aqui = |m: &&Modo| {
        m.largura == atual.largura && m.altura == atual.altura && m.bits == atual.bits
    };
    // A lista sem o modo em uso é lista errada (filtro, driver), não "não há modos".
    let hz_maximo_aqui = match maior_hz(t.modos.iter().filter(aqui)) {
        Some(h) if t.modos.iter().filter(aqui).any(|m| mesmo_degrau(m.hz, atual.hz)) => {
            h.max(atual.hz)
        }
        _ => return base(Veredito::ModosIlegiveis, None, None, None, cabo_sem_edid()),
    };

    let nativa = match edid.and_then(|e| e.preferida) {
        Some(p) => Some(Nativa {
            largura: p.largura,
            altura: p.altura,
            fonte: FonteDaNativa::Edid,
        }),
        None => t
            .modos
            .iter()
            .filter(|m| m.bits == atual.bits)
            .max_by_key(|m| (m.largura as u64 * m.altura as u64, m.hz))
            .map(|m| Nativa {
                largura: m.largura,
                altura: m.altura,
                fonte: FonteDaNativa::MaiorModoListado,
            }),
    };
    let hz_maximo_na_nativa = nativa.and_then(|n| {
        maior_hz(
            t.modos
                .iter()
                .filter(|m| m.largura == n.largura && m.altura == n.altura && m.bits == atual.bits),
        )
    });

    // (c) Só com temporização da EDID na resolução comparada. Primeiro a atual, depois a nativa.
    let cabo = match edid {
        None => cabo_sem_edid(),
        Some(e) => {
            let mut candidatos = vec![(atual.largura, atual.altura, Some(hz_maximo_aqui))];
            if let Some(n) = nativa {
                if (n.largura, n.altura) != (atual.largura, atual.altura) {
                    candidatos.push((n.largura, n.altura, hz_maximo_na_nativa));
                }
            }
            let mut achou_temporizacao = false;
            let mut limitando = None;
            for (l, a, oferecido) in candidatos {
                let Some(anunciado) = anunciado_em(e, l, a) else {
                    continue;
                };
                achou_temporizacao = true;
                // Sem nenhum modo nessa resolução o Windows não entrega nada ali: não é comparável.
                let Some(oferecido) = oferecido else {
                    continue;
                };
                if ganho_que_importa(oferecido, (anunciado + 500) / 1000) {
                    limitando = Some(Cabo::Limitando {
                        largura: l,
                        altura: a,
                        hz_oferecido: oferecido,
                        hz_anunciado_mhz: anunciado,
                    });
                    break;
                }
            }
            match limitando {
                Some(c) => c,
                None if achou_temporizacao => Cabo::NaoLimita,
                None => Cabo::SemTemporizacaoNaResolucao,
            }
        }
    };

    // (b) A maior taxa em outra resolução, mesma profundidade de cor. Empate: a maior resolução.
    let outra = t
        .modos
        .iter()
        .filter(|m| {
            !m.entrelacado
                && m.bits == atual.bits
                && (m.largura, m.altura) != (atual.largura, atual.altura)
        })
        .max_by_key(|m| (m.hz, m.largura as u64 * m.altura as u64))
        .filter(|m| ganho_que_importa(hz_maximo_aqui, m.hz));

    let veredito = if ganho_que_importa(atual.hz, hz_maximo_aqui) {
        Veredito::UmClique {
            hz_atual: atual.hz,
            hz_alvo: hz_maximo_aqui,
        }
    } else if let Cabo::Limitando {
        hz_oferecido,
        hz_anunciado_mhz,
        ..
    } = &cabo
    {
        Veredito::CaboOuPortaLimitando {
            hz_atual: atual.hz,
            hz_oferecido: *hz_oferecido,
            hz_anunciado: (hz_anunciado_mhz + 500) / 1000,
        }
    } else if let Some(m) = outra {
        Veredito::SoEmOutraResolucao {
            hz_atual: atual.hz,
            largura: m.largura,
            altura: m.altura,
            hz: m.hz,
        }
    } else {
        Veredito::NoMaximo { hz_atual: atual.hz }
    };

    base(
        veredito,
        Some(hz_maximo_aqui),
        nativa,
        hz_maximo_na_nativa,
        cabo,
    )
}

/// Subir a taxa não sobe o FPS, sobe o TETO: prometer FPS aqui seria mentira fácil de vender.
const RESSALVA_DE_FPS: &str = "O jogo fica mais suave, e o contador de FPS continua onde estava: a taxa do monitor \
                               não cria quadros, ela deixa de segurar os que a placa já entrega.";

/// Pura: a frase da tela.
pub fn texto_do_veredito(nome: &str, v: &Veredito, cabo: &Cabo) -> String {
    let cabo_tambem = match cabo {
        Cabo::Limitando {
            largura,
            altura,
            hz_anunciado_mhz,
            ..
        } => format!(
            " E o monitor anuncia {} Hz em {}x{}: o cabo ou a porta está segurando o resto. Use DisplayPort, ou \
             HDMI 2.0 ou mais novo, ligado na placa de vídeo.",
            (hz_anunciado_mhz + 500) / 1000,
            largura,
            altura
        ),
        _ => String::new(),
    };
    match v {
        Veredito::UmClique { hz_atual, hz_alvo } => format!(
            "{} aguenta {} Hz e está em {}. Corrigir leva um clique e se desfaz pelo histórico. {}{}",
            nome, hz_alvo, hz_atual, RESSALVA_DE_FPS, cabo_tambem
        ),
        Veredito::CaboOuPortaLimitando {
            hz_atual,
            hz_oferecido,
            hz_anunciado,
        } => format!(
            "{} anuncia {} Hz, mas a ligação só entrega {} Hz (está em {}). Cabo ou porta limitando: use \
             DisplayPort, ou HDMI 2.0 ou mais novo, ligado na placa de vídeo. Isso não se corrige por programa.",
            nome, hz_anunciado, hz_oferecido, hz_atual
        ),
        Veredito::SoEmOutraResolucao {
            hz_atual,
            largura,
            altura,
            hz,
        } => format!(
            "{} já está na maior taxa desta resolução ({} Hz). Ele chega a {} Hz só em {}x{}, com a imagem menos \
             nítida: é uma troca, e o Otimiza não faz sozinho.",
            nome, hz_atual, hz, largura, altura
        ),
        Veredito::NoMaximo { hz_atual } => {
            format!("{} já está na maior taxa desta resolução ({} Hz).", nome, hz_atual)
        }
        Veredito::ModosIlegiveis => format!(
            "Não consegui ler os modos de vídeo de {}. Nada foi concluído sobre a taxa dele.",
            nome
        ),
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Pura: o modo pedido

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(specta::Type))]
#[serde(tag = "recusa")]
pub enum RecusaHz {
    /// Jogo em tela cheia exclusiva: trocar o modo agora derruba o jogo do modo dele.
    JogoEmTelaCheia,
    /// Já há uma troca desta tela no histórico: aplicar por cima guardaria como "anterior" o que o Otimiza escreveu.
    JaAlteradaPeloOtimiza { id: String },
    OutraTrocaEsperandoConfirmacao,
    MonitorNaoEncontrado { dispositivo: String },
    LeituraFalhou { detalhe: String },
    JaEstaNessaTaxa { hz: u32 },
    HzIndisponivel {
        hz: u32,
        largura: u32,
        altura: u32,
        disponiveis: Vec<u32>,
    },
    DriverRecusou { codigo: i32, motivo: String },
    /// A confirmação chegou depois do prazo: a tela já voltou (ou está voltando).
    PrazoVencido,
    /// Nenhuma troca pendente com esse número.
    TrocaDesconhecida,
    HistoricoFalhou { detalhe: String },
    SoNoWindows,
}

impl RecusaHz {
    pub fn texto(&self) -> String {
        match self {
            RecusaHz::JogoEmTelaCheia => "Há um jogo em tela cheia exclusiva agora. Trocar a taxa tiraria o jogo do \
                                         modo dele: feche o jogo ou passe para a área de trabalho e tente de novo."
                .to_string(),
            RecusaHz::JaAlteradaPeloOtimiza { .. } => "A taxa deste monitor já foi trocada pelo Otimiza. Desfaça \
                                                       pelo histórico antes de trocar de novo."
                .to_string(),
            RecusaHz::OutraTrocaEsperandoConfirmacao => {
                "Já existe uma troca de taxa esperando confirmação. Confirme ou volte aquela primeiro.".to_string()
            }
            RecusaHz::MonitorNaoEncontrado { dispositivo } => format!(
                "Não encontrei o monitor `{}`. Ele pode ter sido desconectado depois do diagnóstico.",
                dispositivo
            ),
            RecusaHz::LeituraFalhou { detalhe } => {
                format!("Não consegui ler o modo de vídeo atual ({}). Nada foi alterado.", detalhe)
            }
            RecusaHz::JaEstaNessaTaxa { hz } => format!("O monitor já está em {} Hz.", hz),
            RecusaHz::HzIndisponivel {
                hz,
                largura,
                altura,
                disponiveis,
            } => format!(
                "O monitor não aceita {} Hz em {}x{} agora. Disponíveis: {}. Nada foi alterado.",
                hz,
                largura,
                altura,
                disponiveis
                    .iter()
                    .map(|v| format!("{} Hz", v))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            RecusaHz::DriverRecusou { motivo, .. } => {
                format!("Não deu para trocar a taxa: {}. A tela continua como estava.", motivo)
            }
            RecusaHz::PrazoVencido => "O prazo para confirmar passou e a taxa anterior voltou.".to_string(),
            RecusaHz::TrocaDesconhecida => "Não há troca de taxa esperando confirmação.".to_string(),
            RecusaHz::HistoricoFalhou { detalhe } => format!(
                "Não consegui gravar a troca no histórico ({}), então a taxa anterior voltou: mudança sem caminho \
                 de volta não fica.",
                detalhe
            ),
            RecusaHz::SoNoWindows => "Trocar a taxa do monitor só existe no Windows.".to_string(),
        }
    }
}

impl std::fmt::Display for RecusaHz {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.texto())
    }
}

/// Pura: o modo novo é o atual com SÓ a taxa trocada. Resolução, cor, orientação e posição ficam como estão.
pub fn modo_alvo(atual: &ModoDeVideo, modos: &[Modo], hz: u32) -> Result<ModoDeVideo, RecusaHz> {
    if atual.hz == hz {
        return Err(RecusaHz::JaEstaNessaTaxa { hz });
    }
    let mut disponiveis: Vec<u32> = modos
        .iter()
        .filter(|m| {
            !m.entrelacado
                && m.hz > 1
                && m.largura == atual.largura
                && m.altura == atual.altura
                && m.bits == atual.bits
        })
        .map(|m| m.hz)
        .collect();
    disponiveis.sort_unstable();
    disponiveis.dedup();
    if !disponiveis.contains(&hz) {
        return Err(RecusaHz::HzIndisponivel {
            hz,
            largura: atual.largura,
            altura: atual.altura,
            disponiveis,
        });
    }
    Ok(ModoDeVideo { hz, ..*atual })
}

/// Pura. O valor cru (`-2`, `-4`) não ajuda ninguém.
pub fn explicar_recusa(codigo: i32) -> &'static str {
    match codigo {
        -1 => "a placa de vídeo recusou o modo",
        -2 => "este monitor não aceita esta combinação",
        -3 => "não foi possível gravar a configuração no registro do Windows",
        -4 => "o driver de vídeo devolveu um erro",
        -5 => "o Windows recusou os parâmetros do modo",
        -6 => "a configuração de dois monitores não permite este modo",
        1 => "o modo exige reiniciar o computador",
        _ => "o Windows recusou a mudança",
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Pura: a guarda do "manter estas configurações?"

/// Uma troca aplicada e ainda não confirmada. `dados` é o que a máquina precisa para voltar (no teste, nada).
#[derive(Debug)]
pub struct Pendente<T> {
    pub token: u64,
    pub prazo: Instant,
    pub dados: T,
}

/// Uma pendência por vez: duas trocas abertas confundiriam qual "anterior" é o de verdade.
#[derive(Debug)]
pub struct Guarda<T> {
    pendente: Option<Pendente<T>>,
    proximo: u64,
}

impl<T> Guarda<T> {
    pub const fn nova() -> Self {
        Guarda {
            pendente: None,
            proximo: 1,
        }
    }

    pub fn ocupada(&self) -> bool {
        self.pendente.is_some()
    }

    pub fn abrir(&mut self, agora: Instant, prazo: Duration, dados: T) -> Result<u64, RecusaHz> {
        if self.pendente.is_some() {
            return Err(RecusaHz::OutraTrocaEsperandoConfirmacao);
        }
        let token = self.proximo;
        self.proximo += 1;
        self.pendente = Some(Pendente {
            token,
            prazo: agora + prazo,
            dados,
        });
        Ok(token)
    }

    /// Antes do prazo e com o número certo: entrega a pendência para gravar. Depois do prazo, recusa: o relógio já
    /// decidiu voltar, e gravar agora gravaria um modo que talvez nem esteja mais na tela.
    pub fn confirmar(&mut self, token: u64, agora: Instant) -> Result<Pendente<T>, RecusaHz> {
        match &self.pendente {
            Some(p) if p.token == token && agora < p.prazo => Ok(self.pendente.take().expect("conferido")),
            Some(p) if p.token == token => Err(RecusaHz::PrazoVencido),
            _ => Err(RecusaHz::TrocaDesconhecida),
        }
    }

    /// A pessoa pediu para voltar: vale a qualquer momento.
    pub fn cancelar(&mut self, token: u64) -> Option<Pendente<T>> {
        match &self.pendente {
            Some(p) if p.token == token => self.pendente.take(),
            _ => None,
        }
    }

    /// O relógio: só entrega a pendência para voltar se ela ainda é a mesma e o prazo passou.
    pub fn vencer(&mut self, token: u64, agora: Instant) -> Option<Pendente<T>> {
        match &self.pendente {
            Some(p) if p.token == token && agora >= p.prazo => self.pendente.take(),
            _ => None,
        }
    }
}

/// A troca aplicada, para a tela mostrar "Manter? 15 s".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct TrocaAplicada {
    pub token: u64,
    pub dispositivo: String,
    pub hz_anterior: u32,
    pub hz_novo: u32,
    pub prazo_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(specta::Type))]
#[serde(tag = "estado")]
pub enum FimDaTroca {
    /// Gravada no registro do Windows e no histórico.
    Mantida { id: String, hz: u32 },
    /// Voltou ao modo anterior (pedido da pessoa).
    Voltou { hz: u32 },
}

// ---------------------------------------------------------------------------------------------------------------
// Máquina (Windows)

#[cfg(target_os = "windows")]
mod maquina {
    use super::*;
    use windows_sys::Win32::Graphics::Gdi::{
        ChangeDisplaySettingsExW, EnumDisplayDevicesW, EnumDisplaySettingsExW, CDS_NORESET, CDS_TEST,
        CDS_TYPE, CDS_UPDATEREGISTRY, DEVMODEW, DISPLAY_DEVICEW, DISPLAY_DEVICE_ATTACHED_TO_DESKTOP,
        DISPLAY_DEVICE_MIRRORING_DRIVER, DISPLAY_DEVICE_PRIMARY_DEVICE, DISP_CHANGE_SUCCESSFUL, DM_BITSPERPEL,
        DM_DISPLAYFLAGS, DM_DISPLAYFREQUENCY, DM_DISPLAYORIENTATION, DM_INTERLACED, DM_PELSHEIGHT, DM_PELSWIDTH,
        DM_POSITION, ENUM_CURRENT_SETTINGS,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::EDD_GET_DEVICE_INTERFACE_NAME;

    fn texto(bruto: &[u16]) -> String {
        let fim = bruto.iter().position(|c| *c == 0).unwrap_or(bruto.len());
        String::from_utf16_lossy(&bruto[..fim])
    }

    fn largo(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn para_modo(d: &DEVMODEW) -> ModoDeVideo {
        // SAFETY: monitores usam o ramo de vídeo das uniões do DEVMODEW (posição/orientação e flags de exibição).
        let (posicao, orientacao, flags) = unsafe {
            (
                d.Anonymous1.Anonymous2.dmPosition,
                d.Anonymous1.Anonymous2.dmDisplayOrientation,
                d.Anonymous2.dmDisplayFlags,
            )
        };
        ModoDeVideo {
            largura: d.dmPelsWidth,
            altura: d.dmPelsHeight,
            bits: d.dmBitsPerPel,
            hz: d.dmDisplayFrequency,
            flags,
            orientacao,
            x: posicao.x,
            y: posicao.y,
        }
    }

    pub(super) fn modo_atual_cru(dispositivo: &str) -> Option<DEVMODEW> {
        let nome = largo(dispositivo);
        unsafe {
            let mut d: DEVMODEW = std::mem::zeroed();
            d.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
            (EnumDisplaySettingsExW(nome.as_ptr(), ENUM_CURRENT_SETTINGS, &mut d, 0) != 0).then_some(d)
        }
    }

    pub(super) fn modo_atual(dispositivo: &str) -> Option<ModoDeVideo> {
        modo_atual_cru(dispositivo).map(|d| para_modo(&d))
    }

    /// Todos os modos, em todas as resoluções: o diagnóstico decide o que é "desta resolução".
    pub(super) fn modos(dispositivo: &str) -> Vec<Modo> {
        let nome = largo(dispositivo);
        let mut v: Vec<Modo> = Vec::new();
        unsafe {
            for i in 0..4096u32 {
                let mut d: DEVMODEW = std::mem::zeroed();
                d.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
                if EnumDisplaySettingsExW(nome.as_ptr(), i, &mut d, 0) == 0 {
                    break;
                }
                let m = Modo {
                    largura: d.dmPelsWidth,
                    altura: d.dmPelsHeight,
                    bits: d.dmBitsPerPel,
                    hz: d.dmDisplayFrequency,
                    entrelacado: d.Anonymous2.dmDisplayFlags & DM_INTERLACED != 0,
                };
                if !v.contains(&m) {
                    v.push(m);
                }
            }
        }
        v
    }

    fn ler_edid_da_instancia(modelo: &str, instancia: &str) -> LeituraEdid {
        use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ};
        use winreg::RegKey;

        let caminho = format!(
            r"SYSTEM\CurrentControlSet\Enum\DISPLAY\{}\{}\Device Parameters",
            modelo, instancia
        );
        let chave = match RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey_with_flags(&caminho, KEY_READ) {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return LeituraEdid::NaoAchada,
            Err(e) => {
                return LeituraEdid::Ilegivel {
                    motivo: format!("não abri {}: {}", caminho, e),
                }
            }
        };
        match chave.get_raw_value("EDID") {
            Ok(bruto) => match ler_edid(&bruto.bytes) {
                Some(e) => LeituraEdid::Lida(e),
                None => LeituraEdid::Ilegivel {
                    motivo: format!("o valor EDID ({} bytes) não é uma EDID", bruto.bytes.len()),
                },
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => LeituraEdid::NaoAchada,
            Err(e) => LeituraEdid::Ilegivel {
                motivo: format!("não li o EDID: {}", e),
            },
        }
    }

    /// O monitor ativo ligado a esta saída, pelo caminho de interface: a instância exata, não "o primeiro do modelo".
    fn edid_e_nome(adaptador: &[u16; 32]) -> (LeituraEdid, Option<String>) {
        unsafe {
            for m in 0..8u32 {
                let mut mon: DISPLAY_DEVICEW = std::mem::zeroed();
                mon.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;
                if EnumDisplayDevicesW(adaptador.as_ptr(), m, &mut mon, EDD_GET_DEVICE_INTERFACE_NAME) == 0 {
                    break;
                }
                // DISPLAY_DEVICE_ACTIVE (1): o monitor está recebendo imagem desta saída.
                if mon.StateFlags & 1 == 0 {
                    continue;
                }
                let nome = Some(texto(&mon.DeviceString)).filter(|s| !s.trim().is_empty());
                return match instancia_do_caminho(&texto(&mon.DeviceID)) {
                    Some((modelo, instancia)) => (ler_edid_da_instancia(&modelo, &instancia), nome),
                    None => (LeituraEdid::NaoAchada, nome),
                };
            }
        }
        (LeituraEdid::NaoAchada, None)
    }

    pub(super) fn ler_telas() -> Vec<LeituraDaTela> {
        let mut telas = Vec::new();
        unsafe {
            for i in 0..16u32 {
                let mut a: DISPLAY_DEVICEW = std::mem::zeroed();
                a.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;
                if EnumDisplayDevicesW(std::ptr::null(), i, &mut a, 0) == 0 {
                    break;
                }
                // Driver de espelhamento é software de captura fingindo ser monitor.
                if a.StateFlags & DISPLAY_DEVICE_ATTACHED_TO_DESKTOP == 0
                    || a.StateFlags & DISPLAY_DEVICE_MIRRORING_DRIVER != 0
                {
                    continue;
                }
                let dispositivo = texto(&a.DeviceName);
                let (edid, nome_do_monitor) = edid_e_nome(&a.DeviceName);
                telas.push(LeituraDaTela {
                    nome: nome_do_monitor.unwrap_or_else(|| texto(&a.DeviceString)),
                    principal: a.StateFlags & DISPLAY_DEVICE_PRIMARY_DEVICE != 0,
                    atual: modo_atual(&dispositivo),
                    modos: modos(&dispositivo),
                    edid,
                    dispositivo,
                });
            }
        }
        telas
    }

    /// Monta o DEVMODE a partir do atual LIDO (o driver guarda campos que não conhecemos) e põe o modo pedido.
    /// Posição e orientação só entram nos campos quando mudam: a troca de taxa não deve nem tocá-las.
    fn montar(dispositivo: &str, alvo: &ModoDeVideo) -> Result<DEVMODEW, RecusaHz> {
        let mut d = modo_atual_cru(dispositivo).ok_or_else(|| RecusaHz::LeituraFalhou {
            detalhe: format!("o Windows não devolveu o modo de {}", dispositivo),
        })?;
        let agora = para_modo(&d);
        d.dmPelsWidth = alvo.largura;
        d.dmPelsHeight = alvo.altura;
        d.dmBitsPerPel = alvo.bits;
        d.dmDisplayFrequency = alvo.hz;
        d.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_BITSPERPEL | DM_DISPLAYFREQUENCY | DM_DISPLAYFLAGS;
        // Escrever em campo de união é seguro; é o ramo de vídeo, o mesmo lido em `para_modo`.
        d.Anonymous2.dmDisplayFlags = alvo.flags;
        if (agora.x, agora.y) != (alvo.x, alvo.y) {
            d.Anonymous1.Anonymous2.dmPosition.x = alvo.x;
            d.Anonymous1.Anonymous2.dmPosition.y = alvo.y;
            d.dmFields |= DM_POSITION;
        }
        if agora.orientacao != alvo.orientacao {
            d.Anonymous1.Anonymous2.dmDisplayOrientation = alvo.orientacao;
            d.dmFields |= DM_DISPLAYORIENTATION;
        }
        Ok(d)
    }

    fn mudar(dispositivo: &str, d: &DEVMODEW, flags: CDS_TYPE) -> Result<(), RecusaHz> {
        let nome = largo(dispositivo);
        let r = unsafe {
            ChangeDisplaySettingsExW(nome.as_ptr(), d, std::ptr::null_mut(), flags, std::ptr::null())
        };
        if r == DISP_CHANGE_SUCCESSFUL {
            Ok(())
        } else {
            Err(RecusaHz::DriverRecusou {
                codigo: r,
                motivo: explicar_recusa(r).to_string(),
            })
        }
    }

    /// `CDS_TEST` primeiro (pergunta ao driver sem mudar nada), depois aplica com `flags`.
    pub(super) fn trocar(dispositivo: &str, alvo: &ModoDeVideo, flags: CDS_TYPE) -> Result<(), RecusaHz> {
        let d = montar(dispositivo, alvo)?;
        mudar(dispositivo, &d, CDS_TEST)?;
        mudar(dispositivo, &d, flags)
    }

    pub(super) fn ensaiar(dispositivo: &str, alvo: &ModoDeVideo) -> Result<(), RecusaHz> {
        let d = montar(dispositivo, alvo)?;
        mudar(dispositivo, &d, CDS_TEST)
    }

    /// Sem gravar no registro: se o processo morrer, a próxima troca de sessão ou de driver volta ao gravado.
    pub(super) const DINAMICO: CDS_TYPE = 0;
    pub(super) const GRAVAR: CDS_TYPE = CDS_UPDATEREGISTRY;
    /// Grava no registro o que já está na tela, sem aplicar de novo (sem uma segunda piscada).
    pub(super) const GRAVAR_SEM_APLICAR: CDS_TYPE = CDS_UPDATEREGISTRY | CDS_NORESET;

    /// Último recurso de uma volta que falhou: `NULL` devolve o modo gravado no registro, que nunca foi tocado
    /// enquanto a troca esperava confirmação.
    pub(super) fn voltar_ao_registro(dispositivo: &str) -> bool {
        let nome = largo(dispositivo);
        unsafe {
            ChangeDisplaySettingsExW(nome.as_ptr(), std::ptr::null(), std::ptr::null_mut(), 0, std::ptr::null())
                == DISP_CHANGE_SUCCESSFUL
        }
    }
}

/// O que a guarda segura enquanto espera: o caminho de volta e o diário (que só some quando a troca termina).
#[cfg(target_os = "windows")]
struct EmEspera {
    dispositivo: String,
    nome: String,
    anterior: ModoDeVideo,
    novo: ModoDeVideo,
    diario: crate::modules::transacao::Diario,
}

#[cfg(not(target_os = "windows"))]
type EmEspera = ();

static GUARDA: Mutex<Guarda<EmEspera>> = Mutex::new(Guarda::nova());

fn guarda() -> std::sync::MutexGuard<'static, Guarda<EmEspera>> {
    // Envenenado só se uma volta entrou em pânico: a pendência ainda é a verdade, então segue com ela.
    GUARDA.lock().unwrap_or_else(|e| e.into_inner())
}

/// LIVRE (só lê). Um diagnóstico por monitor ativo. Custo: enumerar modos e ler um valor do registro por monitor,
/// dezenas de milissegundos; sem PowerShell.
#[cfg(target_os = "windows")]
pub fn diagnosticar_telas() -> Vec<DiagnosticoDaTela> {
    maquina::ler_telas().iter().map(diagnosticar).collect()
}

#[cfg(not(target_os = "windows"))]
pub fn diagnosticar_telas() -> Vec<DiagnosticoDaTela> {
    Vec::new()
}

/// Há troca esperando confirmação? A tela nova pergunta ao abrir (a janela pode ter sido recarregada no meio).
pub fn troca_pendente() -> bool {
    guarda().ocupada()
}

/// Aplica SÓ a taxa, sem gravar, e arma a volta automática. A tela mostra "Manter? 15 s" e chama
/// [`confirmar_troca`] ou [`voltar_troca`]; sem nenhum dos dois, volta sozinha.
#[cfg(target_os = "windows")]
pub fn aplicar_hz(
    dispositivo: &str,
    hz: u32,
    log: &crate::modules::changelog::ChangeLog,
) -> Result<TrocaAplicada, RecusaHz> {
    use crate::modules::changelog::ChangeRecord;
    use crate::modules::transacao::{abrir, Intencao, Pendencia};

    let id = id_no_historico(dispositivo);
    let antigo = format!("monitor:{}", dispositivo.to_lowercase());
    for ja in [&id, &antigo] {
        if log.is_applied(ja) {
            return Err(RecusaHz::JaAlteradaPeloOtimiza { id: ja.clone() });
        }
    }
    if crate::modules::windows::geracao::tela_cheia_exclusiva() {
        return Err(RecusaHz::JogoEmTelaCheia);
    }

    let mut g = guarda();
    if g.ocupada() {
        return Err(RecusaHz::OutraTrocaEsperandoConfirmacao);
    }

    let tela = maquina::ler_telas()
        .into_iter()
        .find(|t| t.dispositivo.eq_ignore_ascii_case(dispositivo))
        .ok_or_else(|| RecusaHz::MonitorNaoEncontrado {
            dispositivo: dispositivo.to_string(),
        })?;
    let anterior = tela.atual.ok_or_else(|| RecusaHz::LeituraFalhou {
        detalhe: "o Windows não devolveu o modo atual".to_string(),
    })?;
    let novo = modo_alvo(&anterior, &tela.modos, hz)?;
    maquina::ensaiar(&tela.dispositivo, &novo)?;

    // O diário antes de tocar: se o Otimiza morrer com a troca pendente, a abertura seguinte mostra a pendência com
    // o modo anterior e oferece voltar (`concluir_recuperacao`).
    let diario = abrir(&Pendencia::nova(
        &id,
        format!("{} em {} Hz", tela.nome, hz),
        Intencao::Aplicar,
        crate::modules::changelog::now_timestamp(),
        vec![ChangeRecord::ModoDeTela {
            dispositivo: tela.dispositivo.clone(),
            anterior,
        }],
    ))
    .map_err(|detalhe| RecusaHz::HistoricoFalhou { detalhe })?;

    if let Err(e) = maquina::trocar(&tela.dispositivo, &novo, maquina::DINAMICO) {
        let _ = diario.concluir();
        return Err(e);
    }

    let token = g
        .abrir(
            Instant::now(),
            PRAZO_PARA_CONFIRMAR,
            EmEspera {
                dispositivo: tela.dispositivo.clone(),
                nome: tela.nome.clone(),
                anterior,
                novo,
                diario,
            },
        )
        .expect("a guarda foi conferida livre com o cadeado na mão");
    drop(g);

    std::thread::spawn(move || {
        std::thread::sleep(PRAZO_PARA_CONFIRMAR);
        if let Some(p) = guarda().vencer(token, Instant::now()) {
            crate::utils::Logger::warn(&format!(
                "tela: {} não foi confirmada em {} s; voltando a {} Hz",
                p.dados.dispositivo,
                PRAZO_PARA_CONFIRMAR.as_secs(),
                p.dados.anterior.hz
            ));
            // A volta não pergunta pelo jogo em tela cheia: tela preta sem confirmação pesa mais que derrubar o modo.
            voltar(p.dados);
        }
    });

    Ok(TrocaAplicada {
        token,
        dispositivo: tela.dispositivo,
        hz_anterior: anterior.hz,
        hz_novo: hz,
        prazo_ms: PRAZO_PARA_CONFIRMAR.as_millis() as u64,
    })
}

#[cfg(target_os = "windows")]
fn voltar(e: EmEspera) -> u32 {
    if maquina::trocar(&e.dispositivo, &e.anterior, maquina::DINAMICO).is_err() {
        maquina::voltar_ao_registro(&e.dispositivo);
    }
    let _ = e.diario.concluir();
    e.anterior.hz
}

/// A pessoa viu a imagem e confirmou: grava no Windows e no histórico. Falhar ao gravar no histórico desfaz.
#[cfg(target_os = "windows")]
pub fn confirmar_troca(
    token: u64,
    log: &mut crate::modules::changelog::ChangeLog,
) -> Result<FimDaTroca, RecusaHz> {
    use crate::modules::changelog::{AppliedOptimization, ChangeRecord};

    let p = guarda().confirmar(token, Instant::now())?;
    let e = p.dados;

    if let Err(erro) = maquina::trocar(&e.dispositivo, &e.novo, maquina::GRAVAR_SEM_APLICAR) {
        voltar(e);
        return Err(erro);
    }

    let id = id_no_historico(&e.dispositivo);
    let gravado = log.record(AppliedOptimization {
        optimization_id: id.clone(),
        name: format!("{} em {} Hz", e.nome, e.novo.hz),
        timestamp: crate::modules::changelog::now_timestamp(),
        changes: vec![ChangeRecord::ModoDeTela {
            dispositivo: e.dispositivo.clone(),
            anterior: e.anterior,
        }],
    });
    if let Err(detalhe) = gravado {
        let _ = maquina::trocar(&e.dispositivo, &e.anterior, maquina::GRAVAR);
        let _ = e.diario.concluir();
        return Err(RecusaHz::HistoricoFalhou { detalhe });
    }

    let hz = e.novo.hz;
    let _ = e.diario.concluir();
    Ok(FimDaTroca::Mantida { id, hz })
}

/// A pessoa pediu para voltar antes do prazo.
#[cfg(target_os = "windows")]
pub fn voltar_troca(token: u64) -> Result<FimDaTroca, RecusaHz> {
    let p = guarda().cancelar(token).ok_or(RecusaHz::TrocaDesconhecida)?;
    Ok(FimDaTroca::Voltou { hz: voltar(p.dados) })
}

/// O desfazer do histórico (`revert_changes`, "Desfazer tudo", recuperação de pendência): devolve o modo LIDO antes,
/// gravando no registro. Recusa com jogo em tela cheia exclusiva; o registro fica no histórico para depois.
#[cfg(target_os = "windows")]
pub fn restaurar_modo(dispositivo: &str, anterior: &ModoDeVideo) -> Result<(), String> {
    if crate::modules::windows::geracao::tela_cheia_exclusiva() {
        return Err(RecusaHz::JogoEmTelaCheia.texto());
    }
    if maquina::modo_atual(dispositivo).as_ref() == Some(anterior) {
        // Já está como antes (a pendência morreu antes de gravar, ou a pessoa voltou à mão): o registro do Windows
        // ainda pode guardar o novo, então grava sem aplicar.
        return maquina::trocar(dispositivo, anterior, maquina::GRAVAR_SEM_APLICAR).map_err(|e| e.texto());
    }
    maquina::trocar(dispositivo, anterior, maquina::GRAVAR).map_err(|e| e.texto())
}

#[cfg(not(target_os = "windows"))]
pub fn aplicar_hz(
    _dispositivo: &str,
    _hz: u32,
    _log: &crate::modules::changelog::ChangeLog,
) -> Result<TrocaAplicada, RecusaHz> {
    Err(RecusaHz::SoNoWindows)
}

#[cfg(not(target_os = "windows"))]
pub fn confirmar_troca(
    _token: u64,
    _log: &mut crate::modules::changelog::ChangeLog,
) -> Result<FimDaTroca, RecusaHz> {
    Err(RecusaHz::SoNoWindows)
}

#[cfg(not(target_os = "windows"))]
pub fn voltar_troca(_token: u64) -> Result<FimDaTroca, RecusaHz> {
    Err(RecusaHz::SoNoWindows)
}

#[cfg(not(target_os = "windows"))]
pub fn restaurar_modo(_dispositivo: &str, _anterior: &ModoDeVideo) -> Result<(), String> {
    Err(RecusaHz::SoNoWindows.texto())
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- EDID de laboratório ---

    /// DTD de 18 bytes: relógio em 10 kHz, ativos e brancos.
    fn dtd(relogio_10khz: u16, ha: u32, hb: u32, va: u32, vb: u32) -> [u8; 18] {
        let mut d = [0u8; 18];
        d[0..2].copy_from_slice(&relogio_10khz.to_le_bytes());
        d[2] = ha as u8;
        d[3] = hb as u8;
        d[4] = (((ha >> 8) & 0xF) << 4 | ((hb >> 8) & 0xF)) as u8;
        d[5] = va as u8;
        d[6] = vb as u8;
        d[7] = (((va >> 8) & 0xF) << 4 | ((vb >> 8) & 0xF)) as u8;
        d[17] = 0x18;
        d
    }

    /// Relógio que dá `hz` com os brancos do CVT-RB (160 e 50 linhas).
    fn dtd_hz(largura: u32, altura: u32, hz: u32) -> [u8; 18] {
        let total = (largura + 160) as u64 * (altura + 50) as u64;
        let relogio = (total * hz as u64 + 5_000) / 10_000;
        dtd(relogio as u16, largura, 160, altura, 50)
    }

    fn edid(base: &[[u8; 18]], nome: &str, faixa_max: Option<u8>, cta: &[[u8; 18]]) -> Vec<u8> {
        let mut b = vec![0u8; if cta.is_empty() { 128 } else { 256 }];
        b[0..8].copy_from_slice(&[0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00]);
        let mut slots = vec![54usize, 72, 90, 108].into_iter();
        for t in base {
            let s = slots.next().unwrap();
            b[s..s + 18].copy_from_slice(t);
        }
        if let Some(max) = faixa_max {
            let s = slots.next().unwrap();
            b[s..s + 5].copy_from_slice(&[0, 0, 0, 0xFD, 0]);
            b[s + 5] = 48;
            b[s + 6] = max;
        }
        if let Some(s) = slots.next() {
            b[s..s + 5].copy_from_slice(&[0, 0, 0, 0xFC, 0]);
            for (i, c) in nome.bytes().chain(std::iter::once(0x0A)).take(13).enumerate() {
                b[s + 5 + i] = c;
            }
        }
        if !cta.is_empty() {
            b[126] = 1;
            b[128] = 0x02;
            b[129] = 0x03;
            b[130] = 4;
            for (i, t) in cta.iter().enumerate() {
                let s = 128 + 4 + i * 18;
                b[s..s + 18].copy_from_slice(t);
            }
        }
        b
    }

    #[test]
    fn dtd_de_165_hz_sai_em_mhz() {
        let t = ler_dtd(&dtd_hz(1920, 1080, 165)).unwrap();
        assert_eq!((t.largura, t.altura, t.hz()), (1920, 1080, 165));
        assert!(t.mhz.abs_diff(165_000) < 50, "{}", t.mhz);
    }

    #[test]
    fn dtd_de_59_94_fica_no_degrau_de_60() {
        // 1080p60 do CEA: 148,5 MHz, 2200x1125 → 60 Hz; 148,35 MHz → 59,94.
        let t = ler_dtd(&dtd(14835, 1920, 280, 1080, 45)).unwrap();
        assert!(t.mhz.abs_diff(59_940) < 10, "{}", t.mhz);
        assert_eq!(t.hz(), 60);
        assert!(mesmo_degrau(59, t.hz()), "o Windows diz 59, a EDID diz 59,94");
        assert!(mesmo_degrau(143, 144));
        assert!(mesmo_degrau(239, 240));
        assert!(!mesmo_degrau(120, 144));
    }

    #[test]
    fn entrelacada_e_descritor_nao_viram_temporizacao() {
        let mut d = dtd_hz(1920, 1080, 60);
        d[17] |= 0x80;
        assert_eq!(ler_dtd(&d), None);
        assert_eq!(ler_dtd(&[0u8; 18]), None);
    }

    #[test]
    fn edid_le_preferida_nome_faixa_e_cta() {
        let b = edid(
            &[dtd_hz(1920, 1080, 60)],
            "24G4",
            Some(180),
            &[dtd_hz(1920, 1080, 180), dtd_hz(1280, 720, 60)],
        );
        let e = ler_edid(&b).unwrap();
        assert_eq!(e.nome.as_deref(), Some("24G4"));
        assert_eq!(e.preferida.map(|p| (p.largura, p.altura)), Some((1920, 1080)));
        assert_eq!(e.faixa_max_hz, Some(180));
        let hz: Vec<u32> = e.temporizacoes.iter().map(|t| t.hz()).collect();
        assert_eq!(hz, vec![60, 180, 60]);
    }

    #[test]
    fn displayid_tipo_i_e_lido() {
        let mut b = edid(&[dtd_hz(2560, 1440, 60)], "X", None, &[]);
        b.resize(256, 0);
        b[126] = 1;
        let ext = &mut b[128..256];
        ext[0] = 0x70;
        ext[1] = 0x12;
        ext[2] = 23; // um bloco: 3 de cabeçalho + 20
        // Bloco 0x03 com uma temporização 2560x1440@165 (CVT-RB: 2720x1490).
        let total = 2720u64 * 1490;
        let relogio = (total * 165 + 5_000) / 10_000 - 1;
        let mut t = [0u8; 20];
        t[0..3].copy_from_slice(&(relogio as u32).to_le_bytes()[..3]);
        t[4..6].copy_from_slice(&(2560u16 - 1).to_le_bytes());
        t[6..8].copy_from_slice(&(160u16 - 1).to_le_bytes());
        t[12..14].copy_from_slice(&(1440u16 - 1).to_le_bytes());
        t[14..16].copy_from_slice(&(50u16 - 1).to_le_bytes());
        ext[5] = 0x03;
        ext[6] = 0;
        ext[7] = 20;
        ext[8..28].copy_from_slice(&t);
        let e = ler_edid(&b).unwrap();
        assert!(
            e.temporizacoes.iter().any(|t| t.largura == 2560 && t.altura == 1440 && t.hz() == 165),
            "{:?}",
            e.temporizacoes
        );
    }

    #[test]
    fn lixo_nao_e_edid() {
        assert_eq!(ler_edid(&[0u8; 128]), None);
        assert_eq!(ler_edid(&[0u8; 10]), None);
    }

    #[test]
    fn caminho_de_interface_da_a_instancia_exata() {
        assert_eq!(
            instancia_do_caminho(r"\\?\DISPLAY#AOCB401#5&1a2b3c&0&UID4352#{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7}"),
            Some(("AOCB401".to_string(), "5&1a2b3c&0&UID4352".to_string()))
        );
        assert_eq!(instancia_do_caminho(r"MONITOR\AOCB401\{4d36e96e}\0003"), None);
        assert_eq!(instancia_do_caminho(""), None);
    }

    // --- Diagnóstico ---

    fn modo(l: u32, a: u32, hz: u32) -> Modo {
        Modo {
            largura: l,
            altura: a,
            bits: 32,
            hz,
            entrelacado: false,
        }
    }

    fn atual(l: u32, a: u32, hz: u32) -> ModoDeVideo {
        ModoDeVideo {
            largura: l,
            altura: a,
            bits: 32,
            hz,
            flags: 0,
            orientacao: 0,
            x: 0,
            y: 0,
        }
    }

    fn tela(a: ModoDeVideo, modos: Vec<Modo>, edid: LeituraEdid) -> LeituraDaTela {
        LeituraDaTela {
            dispositivo: r"\\.\DISPLAY1".to_string(),
            nome: "Monitor genérico".to_string(),
            principal: true,
            atual: Some(a),
            modos,
            edid,
        }
    }

    fn edid_lida(base: &[[u8; 18]], cta: &[[u8; 18]]) -> LeituraEdid {
        LeituraEdid::Lida(ler_edid(&edid(base, "24G4", Some(180), cta)).unwrap())
    }

    #[test]
    fn a_taxa_maior_na_mesma_resolucao_e_um_clique() {
        let t = tela(
            atual(1920, 1080, 60),
            vec![modo(1920, 1080, 60), modo(1920, 1080, 144), modo(1920, 1080, 165)],
            edid_lida(&[dtd_hz(1920, 1080, 165)], &[]),
        );
        let d = diagnosticar(&t);
        assert_eq!(
            d.veredito,
            Veredito::UmClique {
                hz_atual: 60,
                hz_alvo: 165
            }
        );
        assert_eq!(d.cabo, Cabo::NaoLimita);
        assert!(d.texto.contains("aguenta 165 Hz e está em 60"), "{}", d.texto);
        assert!(d.texto.contains("não cria quadros"), "sem promessa de FPS: {}", d.texto);
        assert_eq!(d.nome, "24G4", "o nome da EDID vale mais que o do Windows");
    }

    #[test]
    fn taxa_maior_so_em_outra_resolucao_nao_e_oferecida() {
        let t = tela(
            atual(2560, 1440, 144),
            vec![modo(2560, 1440, 60), modo(2560, 1440, 144), modo(1920, 1080, 240)],
            LeituraEdid::NaoAchada,
        );
        let d = diagnosticar(&t);
        assert_eq!(
            d.veredito,
            Veredito::SoEmOutraResolucao {
                hz_atual: 144,
                largura: 1920,
                altura: 1080,
                hz: 240
            }
        );
        assert!(d.texto.contains("não faz sozinho"));
        assert_eq!(d.cabo, Cabo::EdidNaoAchada);
        assert_eq!(
            d.nativa.map(|n| n.fonte),
            Some(FonteDaNativa::MaiorModoListado)
        );
    }

    #[test]
    fn edid_anuncia_mais_que_a_ligacao_e_cabo_ou_porta() {
        // HDMI 1.4: o monitor de 165 Hz só recebe 120 em 1080p.
        let t = tela(
            atual(1920, 1080, 120),
            vec![modo(1920, 1080, 60), modo(1920, 1080, 120)],
            edid_lida(&[dtd_hz(1920, 1080, 60)], &[dtd_hz(1920, 1080, 165)]),
        );
        let d = diagnosticar(&t);
        assert_eq!(
            d.veredito,
            Veredito::CaboOuPortaLimitando {
                hz_atual: 120,
                hz_oferecido: 120,
                hz_anunciado: 165
            }
        );
        assert!(d.texto.contains("DisplayPort"));
        assert!(d.texto.contains("HDMI 2.0"));
    }

    #[test]
    fn um_clique_ainda_avisa_o_cabo_quando_ha_os_dois() {
        let t = tela(
            atual(1920, 1080, 60),
            vec![modo(1920, 1080, 60), modo(1920, 1080, 144)],
            edid_lida(&[dtd_hz(1920, 1080, 60)], &[dtd_hz(1920, 1080, 240)]),
        );
        let d = diagnosticar(&t);
        assert_eq!(
            d.veredito,
            Veredito::UmClique {
                hz_atual: 60,
                hz_alvo: 144
            }
        );
        assert!(matches!(d.cabo, Cabo::Limitando { hz_oferecido: 144, .. }));
        assert!(d.texto.contains("anuncia 240 Hz"), "{}", d.texto);
    }

    #[test]
    fn so_a_faixa_da_edid_nao_acusa_cabo() {
        // A faixa diz 180, mas nenhuma temporização em 1080p passa de 60: não há fonte para acusar.
        let t = tela(
            atual(1920, 1080, 60),
            vec![modo(1920, 1080, 60)],
            edid_lida(&[dtd_hz(1920, 1080, 60)], &[]),
        );
        let d = diagnosticar(&t);
        assert_eq!(d.veredito, Veredito::NoMaximo { hz_atual: 60 });
        assert_eq!(d.cabo, Cabo::NaoLimita);
        assert_eq!(d.faixa_max_hz, Some(180));
    }

    #[test]
    fn edid_sem_temporizacao_na_resolucao_e_estado_nomeado() {
        let t = tela(
            atual(1920, 1080, 60),
            vec![modo(1920, 1080, 60)],
            edid_lida(&[dtd_hz(2560, 1440, 60)], &[]),
        );
        let d = diagnosticar(&t);
        // A nativa da EDID (1440p) não aparece na lista do Windows: não dá para comparar nada ali.
        assert_eq!(d.hz_maximo_na_nativa, None);
        assert_eq!(d.cabo, Cabo::NaoLimita, "achou temporização na nativa, sem oferta para comparar");

        let t = tela(
            atual(1920, 1080, 60),
            vec![modo(1920, 1080, 60)],
            LeituraEdid::Ilegivel {
                motivo: "x".to_string(),
            },
        );
        assert!(matches!(diagnosticar(&t).cabo, Cabo::EdidIlegivel { .. }));
    }

    #[test]
    fn fracionarias_nao_viram_achado() {
        let t = tela(
            atual(1920, 1080, 59),
            vec![modo(1920, 1080, 59), modo(1920, 1080, 60)],
            edid_lida(&[dtd(14835, 1920, 280, 1080, 45)], &[]),
        );
        let d = diagnosticar(&t);
        assert_eq!(d.veredito, Veredito::NoMaximo { hz_atual: 59 });
        assert_eq!(d.cabo, Cabo::NaoLimita);

        let t = tela(
            atual(1920, 1080, 143),
            vec![modo(1920, 1080, 143), modo(1920, 1080, 144)],
            LeituraEdid::NaoAchada,
        );
        assert!(matches!(diagnosticar(&t).veredito, Veredito::NoMaximo { .. }));
    }

    #[test]
    fn leitura_falha_e_modos_ilegiveis_nunca_no_maximo() {
        let mut t = tela(atual(1920, 1080, 60), vec![], LeituraEdid::NaoAchada);
        assert_eq!(diagnosticar(&t).veredito, Veredito::ModosIlegiveis);
        t.atual = None;
        assert_eq!(diagnosticar(&t).veredito, Veredito::ModosIlegiveis);
        // Lista sem o modo em uso: filtro ou driver errado, não "nada a fazer".
        let t = tela(
            atual(1920, 1080, 60),
            vec![modo(1920, 1080, 144)],
            LeituraEdid::NaoAchada,
        );
        assert_eq!(diagnosticar(&t).veredito, Veredito::ModosIlegiveis);
    }

    #[test]
    fn outra_profundidade_de_cor_nao_conta() {
        let mut m16 = modo(1920, 1080, 144);
        m16.bits = 16;
        let t = tela(
            atual(1920, 1080, 60),
            vec![modo(1920, 1080, 60), m16],
            LeituraEdid::NaoAchada,
        );
        assert_eq!(diagnosticar(&t).veredito, Veredito::NoMaximo { hz_atual: 60 });
    }

    #[test]
    fn cada_monitor_e_julgado_pelos_proprios_modos() {
        // O defeito antigo: o segundo monitor herdava o Hz do principal.
        let principal = tela(
            atual(1920, 1080, 180),
            vec![modo(1920, 1080, 60), modo(1920, 1080, 180)],
            LeituraEdid::NaoAchada,
        );
        let mut segundo = tela(
            atual(1920, 1080, 60),
            vec![modo(1920, 1080, 60), modo(1920, 1080, 75)],
            LeituraEdid::NaoAchada,
        );
        segundo.dispositivo = r"\\.\DISPLAY2".to_string();
        segundo.principal = false;
        let d: Vec<_> = [principal, segundo].iter().map(diagnosticar).collect();
        assert_eq!(d[0].veredito, Veredito::NoMaximo { hz_atual: 180 });
        assert_eq!(
            d[1].veredito,
            Veredito::UmClique {
                hz_atual: 60,
                hz_alvo: 75
            }
        );
        assert_eq!(d[1].dispositivo, r"\\.\DISPLAY2");
    }

    // --- O modo pedido ---

    #[test]
    fn modo_alvo_troca_so_a_taxa() {
        let a = ModoDeVideo {
            largura: 1920,
            altura: 1080,
            bits: 32,
            hz: 60,
            flags: 0,
            orientacao: 1,
            x: -1920,
            y: 120,
        };
        let modos = vec![modo(1920, 1080, 60), modo(1920, 1080, 165), modo(1280, 720, 240)];
        let n = modo_alvo(&a, &modos, 165).unwrap();
        assert_eq!(n, ModoDeVideo { hz: 165, ..a });

        assert!(matches!(
            modo_alvo(&a, &modos, 240),
            Err(RecusaHz::HzIndisponivel { hz: 240, ref disponiveis, .. }) if disponiveis == &vec![60, 165]
        ));
        assert_eq!(modo_alvo(&a, &modos, 60), Err(RecusaHz::JaEstaNessaTaxa { hz: 60 }));
    }

    #[test]
    fn toda_recusa_tem_frase() {
        assert!(explicar_recusa(-2).contains("monitor"));
        assert!(RecusaHz::JogoEmTelaCheia.texto().contains("tela cheia"));
        assert!(RecusaHz::PrazoVencido.texto().contains("voltou"));
    }

    // --- A guarda ---

    #[test]
    fn confirmar_antes_do_prazo_entrega_a_pendencia() {
        let mut g: Guarda<&str> = Guarda::nova();
        let t0 = Instant::now();
        let token = g.abrir(t0, Duration::from_secs(15), "dados").unwrap();
        assert!(g.ocupada());
        assert!(g.vencer(token, t0 + Duration::from_secs(14)).is_none(), "o relógio não volta antes");
        let p = g.confirmar(token, t0 + Duration::from_secs(14)).unwrap();
        assert_eq!(p.dados, "dados");
        assert!(!g.ocupada());
        assert!(g.vencer(token, t0 + Duration::from_secs(16)).is_none(), "confirmada não volta");
    }

    #[test]
    fn sem_confirmacao_o_relogio_volta_e_a_confirmacao_tardia_e_recusada() {
        let mut g: Guarda<()> = Guarda::nova();
        let t0 = Instant::now();
        let token = g.abrir(t0, Duration::from_secs(15), ()).unwrap();
        assert_eq!(
            g.confirmar(token, t0 + Duration::from_secs(15)).unwrap_err(),
            RecusaHz::PrazoVencido
        );
        assert!(g.vencer(token, t0 + Duration::from_secs(15)).is_some());
        assert!(!g.ocupada());
        assert_eq!(
            g.confirmar(token, t0).unwrap_err(),
            RecusaHz::TrocaDesconhecida
        );
    }

    #[test]
    fn uma_troca_por_vez_e_token_velho_nao_mexe_na_nova() {
        let mut g: Guarda<u8> = Guarda::nova();
        let t0 = Instant::now();
        let a = g.abrir(t0, Duration::from_secs(15), 1).unwrap();
        assert_eq!(
            g.abrir(t0, Duration::from_secs(15), 2).unwrap_err(),
            RecusaHz::OutraTrocaEsperandoConfirmacao
        );
        assert_eq!(g.cancelar(a).map(|p| p.dados), Some(1));
        let b = g.abrir(t0, Duration::from_secs(15), 3).unwrap();
        assert_ne!(a, b);
        // O relógio da troca A acorda depois: não pode voltar a troca B.
        assert!(g.vencer(a, t0 + Duration::from_secs(60)).is_none());
        assert!(g.cancelar(a).is_none());
        assert!(g.ocupada());
    }

    // --- Histórico ---

    #[test]
    fn o_modo_anterior_vai_inteiro_para_o_historico() {
        use crate::modules::changelog::ChangeRecord;
        let anterior = ModoDeVideo {
            largura: 2560,
            altura: 1440,
            bits: 32,
            hz: 60,
            flags: 0,
            orientacao: 0,
            x: 1920,
            y: 0,
        };
        let c = ChangeRecord::ModoDeTela {
            dispositivo: r"\\.\DISPLAY2".to_string(),
            anterior,
        };
        let json = serde_json::to_string(&c).unwrap();
        let volta: ChangeRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(volta, c);
        assert!(c.describe().contains("DISPLAY2"));
        assert!(c.describe().contains("2560x1440 a 60 Hz"), "{}", c.describe());
        assert_eq!(id_no_historico(r"\\.\DISPLAY2"), r"tela:\\.\display2");
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn aplicar_recusa_tela_ja_trocada_pelo_otimiza() {
        // O histórico é conferido antes de qualquer leitura da máquina: dado controlado, resposta única.
        use crate::modules::changelog::{AppliedOptimization, ChangeLog, ChangeRecord};
        let mut log = ChangeLog::em_memoria();
        log.record(AppliedOptimization {
            optimization_id: "monitor:\\\\.\\display9".to_string(),
            name: "x".to_string(),
            timestamp: 0,
            changes: vec![ChangeRecord::RefreshRate {
                device: r"\\.\DISPLAY9".to_string(),
                previous_hz: 60,
            }],
        })
        .unwrap();
        match aplicar_hz(r"\\.\DISPLAY9", 144, &log) {
            Err(RecusaHz::JaAlteradaPeloOtimiza { id }) => assert_eq!(id, r"monitor:\\.\display9"),
            outro => panic!("deveria recusar: {:?}", outro),
        }
    }

    // --- Esta máquina ---

    /// Só lê: `cargo test --lib plataforma::hz -- --ignored --nocapture diagnostico_desta_maquina`.
    #[test]
    #[ignore = "lê esta máquina"]
    #[cfg(target_os = "windows")]
    fn diagnostico_desta_maquina() {
        for t in maquina::ler_telas() {
            let resumo_edid = match &t.edid {
                LeituraEdid::Lida(e) => format!(
                    "EDID '{}' preferida {:?} faixa_max {:?} temporizações {:?}",
                    e.nome.clone().unwrap_or_default(),
                    e.preferida.map(|p| (p.largura, p.altura, p.mhz)),
                    e.faixa_max_hz,
                    e.temporizacoes
                        .iter()
                        .map(|t| format!("{}x{}@{:.2}", t.largura, t.altura, t.mhz as f64 / 1000.0))
                        .collect::<Vec<_>>()
                ),
                outro => format!("EDID {:?}", outro),
            };
            let d = diagnosticar(&t);
            println!(
                "{} principal={} atual={:?}\n  {}\n  modos na resolução atual: {:?}\n  {:?} | nativa {:?} máx {:?} | cabo {:?}\n  {}\n",
                d.dispositivo,
                d.principal,
                d.atual,
                resumo_edid,
                {
                    let mut v: Vec<u32> = t
                        .modos
                        .iter()
                        .filter(|m| t.atual.is_some_and(|a| a.largura == m.largura && a.altura == m.altura && a.bits == m.bits))
                        .map(|m| m.hz)
                        .collect();
                    v.sort_unstable();
                    v.dedup();
                    v
                },
                d.veredito,
                d.nativa,
                d.hz_maximo_na_nativa,
                d.cabo,
                d.texto
            );
        }
    }

    /// Para no `CDS_TEST` (o driver responde sem mudar a tela). Nunca aplica.
    #[test]
    #[ignore = "pergunta ao driver desta máquina (CDS_TEST, sem mudar nada)"]
    #[cfg(target_os = "windows")]
    fn ensaio_desta_maquina() {
        for t in maquina::ler_telas() {
            let Some(a) = t.atual else { continue };
            let outra = t
                .modos
                .iter()
                .filter(|m| m.largura == a.largura && m.altura == a.altura && m.bits == a.bits && m.hz != a.hz)
                .map(|m| m.hz)
                .max();
            if let Some(hz) = outra {
                let alvo = modo_alvo(&a, &t.modos, hz).unwrap();
                println!("{}: CDS_TEST {} → {} Hz: {:?}", t.dispositivo, a.hz, hz, maquina::ensaiar(&t.dispositivo, &alvo));
            }
        }
    }
}
