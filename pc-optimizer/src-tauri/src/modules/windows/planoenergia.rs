// O plano de energia OTIMIZA
//
// POR QUE ESTE MÓDULO EXISTE, E O DEFEITO QUE ELE VEIO CONSERTAR
//
// O produto trocava o plano de energia do cliente ativando o "Alto Desempenho"
// pelo GUID FIXO `8c5e7fda-…`, que é o mesmo em toda instalação do Windows — e
// escrevia os ajustes finos DENTRO DO PLANO QUE O CLIENTE JÁ USAVA. Duas coisas
// dão errado com isso, e nenhuma delas aparece na máquina de desenvolvimento:
//
// 1. O GUID FIXO NÃO EXISTE EM TODA MÁQUINA. Em notebook com Modern Standby, e
//    em imagem OEM enxuta, o "Alto Desempenho" não vem. O código de antes
//    chamava `powercfg -duplicatescheme <guid fixo>`, que CRIA UMA CÓPIA COM
//    GUID NOVO, JOGAVA ESSE GUID FORA, e em seguida mandava ativar o GUID FIXO
//    — que continua não existindo. Medido aqui: `-duplicatescheme` do Alto
//    Desempenho respondeu `GUID do Esquema de Energia: 15a86c79-…`, um GUID
//    diferente do pedido. Na máquina do cliente aquilo falhava, e cada tentativa
//    deixava para trás mais um plano órfão chamado "Alto desempenho".
//
// 2. ESCREVER NO PLANO DO CLIENTE NÃO TEM VOLTA LIMPA. Desfazer dependia de ter
//    lido certo cada valor anterior e de conseguir gravar cada um de volta.
//
// A saída é a mesma para os dois: O OTIMIZA PASSA A TER PLANO PRÓPRIO. Ele cria
// um plano chamado OTIMIZA, descobre o GUID QUE O WINDOWS DEVOLVEU, guarda esse
// GUID, configura só dentro dele e o ativa. O plano do cliente não é tocado, e
// desfazer é reativar o plano que estava ativo antes — uma operação só, que não
// depende de ter lido trinta valores corretamente.
//
// A REGRA DA CASA, APLICADA AQUI: NÃO CONFIE QUE FUNCIONOU.
//
// `powercfg` devolve 0 em quase tudo que aceita, e devolve 1 quando o ajuste não
// existe naquele Windows — conferido nesta máquina com um GUID de ajuste
// inventado. Mas código de saída não é prova de que o valor entrou. Cada ajuste
// aqui é LIDO DE VOLTA DO REGISTRO depois de gravado, e só então vira
// `Aplicado`. Se o número não bateu, vira `FalhouNaVerificacao` — que é
// diferente de `FalhouAoAplicar`, e a diferença é o que faz o registro de
// suporte servir para alguma coisa.
//
// E AJUSTE QUE NÃO EXISTE NÃO É FALHA DO PLANO. O produto conferia o ajuste
// executando e olhando a mensagem de erro — que é traduzida. Agora confere
// ANTES, na árvore `Control\Power\PowerSettings`, que é a mesma em qualquer
// idioma. Ajuste ausente vira `NaoSuportado` e o plano segue — antes, uma opção
// que não existe no Windows do cliente derrubava a otimização inteira e desfazia
// o que já tinha dado certo.

use super::{hardware, power, registry, shell};
use serde::{Deserialize, Serialize};

/// O nome do plano. É por ele que o plano é reencontrado na execução seguinte,
/// e é por isso que ele não pode ser traduzido.
pub const NOME_DO_PLANO: &str = "OTIMIZA";

const DESCRICAO_DO_PLANO: &str =
    "Plano criado pelo Otimiza para este computador. Pode ser apagado a qualquer momento.";

/// O plano "Equilibrado". Ao contrário do Alto Desempenho, ESTE existe em toda
/// instalação do Windows — é o padrão de fábrica, e o Windows não o esconde nem
/// em Modern Standby. Serve de molde quando o Alto Desempenho não está lá.
pub const EQUILIBRADO_GUID: &str = "381b4222-f694-41f0-9685-ff5bb260df2e";

// ─── Os subgrupos e ajustes, todos documentados pela Microsoft ────────────────

pub const SUB_PROCESSADOR: &str = "54533251-82be-4824-96c1-47b60b740d00";
const PROCTHROTTLEMIN: &str = "893dee8e-2bef-41e0-89c6-b55d0929964c";
const PROCTHROTTLEMAX: &str = "bc5038f7-23e0-4960-96da-33abaf5935ec";
const CPMINCORES: &str = "0cc5b647-c1df-4637-891a-dec35c318583";
const PERFBOOSTMODE: &str = "be337238-0d82-4146-a960-4f3749d470c7";

const SUB_PCIEXPRESS: &str = "501a4d13-42af-4429-9fd1-a8218c268e20";
const ASPM: &str = "ee12f906-d277-404b-b6da-e5fa1a576df5";

const SUB_DISCO: &str = "0012ee47-9041-4b5d-9b77-535fba8b1442";
const DISKIDLE: &str = "6738e2c4-e8a5-4a42-b16a-e040e769756e";

const SUB_USB: &str = "2a737441-1930-4402-8d77-b2bebba308a3";
const USBSELECTIVESUSPEND: &str = "48e6b7a6-50f5-4782-a5d4-53bb8f07e226";

const SUB_GRAFICOS: &str = "5fb4938d-1ee8-4b0f-9a3c-5036b0ab995c";
const GPUPREFERENCEPOLICY: &str = "dd848b2a-8a5d-4451-9ae2-39cd41658f6c";

const SUB_SEM_FIO: &str = "19cbb8fa-5279-450e-9fac-8a3d5fedd0c1";
const POWERSAVINGMODE: &str = "12bbebe6-58d6-4636-95bb-3217ef867c1a";

const SUB_MULTIMIDIA: &str = "9596fb26-9850-41fd-ac3e-f7c3c00afd4b";
const IDLEBACKGROUND: &str = "03680956-93bc-4294-bba6-4e0f09bb717f";

const SUB_SUSPENSAO: &str = "238c9fa8-0aad-41ed-83f4-97be242c8f20";
const STANDBYIDLE: &str = "29f6c1db-86da-48c5-9fdb-f2b67b1f44da";

// ─── O que sabemos da máquina antes de escolher qualquer número ───────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FabricanteDaCpu {
    Intel,
    Amd,
    Outro,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Maquina {
    pub notebook: bool,
    pub tem_bateria: bool,
    pub fabricante_da_cpu: FabricanteDaCpu,
    pub cpu: String,
    pub nucleos_logicos: usize,
    /// `CsEnabled` do registro: 1 quando a máquina usa Modern Standby (S0).
    /// Lido do registro e não da saída do `powercfg /a`, que é traduzida.
    pub modern_standby: bool,
    pub build_do_windows: u32,
    pub windows11: bool,
}

/// Windows 11 começa no build 22000. É a única separação que o registro dá:
/// `CurrentVersion` diz 10.0 nos dois.
pub fn e_windows11(build: u32) -> bool {
    build >= 22000
}

pub fn fabricante_pelo_nome(cpu: &str) -> FabricanteDaCpu {
    let nome = cpu.to_lowercase();

    if nome.contains("intel") {
        FabricanteDaCpu::Intel
    } else if nome.contains("amd") || nome.contains("ryzen") {
        FabricanteDaCpu::Amd
    } else {
        FabricanteDaCpu::Outro
    }
}

/// `PCSystemType` do WMI: 2 é móvel. A bateria confirma — e em imagem
/// modificada, onde o `PCSystemType` pode vir errado, é a bateria que decide.
pub fn e_notebook(pc_system_type: Option<u32>, tem_bateria: bool) -> bool {
    matches!(pc_system_type, Some(2)) || tem_bateria
}

fn ler_u32_do_registro(caminho: &str, nome: &str) -> Option<u32> {
    use crate::modules::changelog::PreviousValue;

    match registry::read("HKLM", caminho, nome) {
        Ok(PreviousValue::Dword(v)) => Some(v),
        _ => None,
    }
}

fn build_do_windows() -> u32 {
    registry::read_text(
        "HKLM",
        r"SOFTWARE\Microsoft\Windows NT\CurrentVersion",
        "CurrentBuildNumber",
    )
    .ok()
    .flatten()
    .and_then(|v| v.trim().parse().ok())
    .unwrap_or(0)
}

/// Lê a máquina. As duas perguntas que não têm resposta no registro — tipo de
/// chassi e presença de bateria — saem de uma chamada só ao PowerShell.
pub fn detectar() -> Maquina {
    let (tipo, tem_bateria) = chassi_e_bateria();
    let perfil = hardware::profile();
    let build = build_do_windows();

    Maquina {
        notebook: e_notebook(tipo, tem_bateria),
        tem_bateria,
        fabricante_da_cpu: fabricante_pelo_nome(&perfil.cpu_name),
        cpu: perfil.cpu_name.clone(),
        nucleos_logicos: perfil.logical_cores,
        modern_standby: ler_u32_do_registro(
            r"SYSTEM\CurrentControlSet\Control\Power",
            "CsEnabled",
        ) == Some(1),
        build_do_windows: build,
        windows11: e_windows11(build),
    }
}

/// Como a máquina está alimentada NESTE MOMENTO.
///
/// Diferente de "tem bateria": um notebook na tomada e o mesmo notebook fora
/// dela usam lados opostos do plano de energia. Sem isto, um relatório que diz
/// "estado mínimo do processador = 5%" não quer dizer nada — pode ser o valor
/// certo da bateria ou o valor errado da tomada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Alimentacao {
    Tomada,
    Bateria,
    /// O Windows respondeu 255, que é o valor documentado para "desconhecido".
    NaoSei,
}

/// Regra pura da leitura do `ACLineStatus`, separada para poder ser testada.
///
/// Os valores são os documentados em `SYSTEM_POWER_STATUS`: 0 fora da tomada,
/// 1 na tomada, 255 desconhecido. São NÚMEROS, e portanto iguais em qualquer
/// idioma do Windows.
pub fn alimentacao_do_status(ac_line_status: u8) -> Alimentacao {
    match ac_line_status {
        0 => Alimentacao::Bateria,
        1 => Alimentacao::Tomada,
        _ => Alimentacao::NaoSei,
    }
}

pub fn alimentacao() -> Alimentacao {
    use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

    let mut status = SYSTEM_POWER_STATUS {
        ACLineStatus: 255,
        BatteryFlag: 255,
        BatteryLifePercent: 255,
        SystemStatusFlag: 0,
        BatteryLifeTime: u32::MAX,
        BatteryFullLifeTime: u32::MAX,
    };

    // A chamada falhar é diferente de a máquina não saber, mas o produto trata
    // os dois igual de propósito: nos dois casos não temos a resposta, e
    // `NaoSei` já diz exatamente isso.
    if unsafe { GetSystemPowerStatus(&mut status) } == 0 {
        return Alimentacao::NaoSei;
    }

    alimentacao_do_status(status.ACLineStatus)
}

/// Separada da execução para poder ser testada: é ela que decide desktop ou
/// notebook, e essa decisão muda todos os valores da bateria.
pub fn ler_chassi_e_bateria(saida: &str) -> (Option<u32>, bool) {
    let mut tipo = None;
    let mut baterias = 0u32;

    for linha in saida.lines() {
        let linha = linha.trim();

        if let Some(v) = linha.strip_prefix("TIPO=") {
            tipo = v.trim().parse().ok();
        } else if let Some(v) = linha.strip_prefix("BATERIAS=") {
            baterias = v.trim().parse().unwrap_or(0);
        }
    }

    (tipo, baterias > 0)
}

fn chassi_e_bateria() -> (Option<u32>, bool) {
    let script = "\"TIPO=\" + (Get-CimInstance Win32_ComputerSystem).PCSystemType; \
                  \"BATERIAS=\" + @(Get-CimInstance Win32_Battery).Count";

    match shell::powershell(script) {
        Ok(saida) if saida.success => ler_chassi_e_bateria(&saida.stdout),
        _ => (None, false),
    }
}

// ─── Classificação e resultado ────────────────────────────────────────────────

/// O quanto o produto se compromete com cada ajuste.
///
/// Por padrão só entram `Segura` e `Recomendada`. `Avancada` existe para que o
/// relatório possa DIZER que ela existe e não foi aplicada, em vez de o produto
/// fingir que o assunto não existe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Classe {
    Segura,
    Recomendada,
    Avancada,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatusDoAjuste {
    /// Gravado E CONFERIDO relendo do Windows.
    Aplicado,
    /// A máquina já estava no valor desejado. Não se escreveu nada.
    JaEstavaBom,
    /// Este Windows não tem este ajuste. Não é falha.
    NaoSuportado,
    /// O `powercfg` recusou.
    FalhouAoAplicar,
    /// O `powercfg` aceitou e o valor relido NÃO é o que pedimos.
    FalhouNaVerificacao,
    /// SÓ EM SIMULAÇÃO: este ajuste mudaria, e nada foi escrito.
    ///
    /// Existe separado de `Pulado` por uma mentira que a tela contou antes de
    /// ele existir. Os dois casos chegavam como `Pulado`, e o painel dizia "não
    /// se aplica aqui" sobre o modo de boost — que mudaria de 1 para 2 se o
    /// cliente clicasse. Era a simulação desencorajando exatamente o que ela
    /// existe para mostrar.
    Mudaria,
    /// Não se aplica a esta máquina.
    Pulado,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultadoDoAjuste {
    pub nome: String,
    pub subgrupo: String,
    pub ajuste: String,
    pub classe: Classe,
    pub porque: String,
    pub suportado: bool,
    pub ac_antes: Option<u32>,
    pub dc_antes: Option<u32>,
    pub ac_alvo: Option<u32>,
    pub dc_alvo: Option<u32>,
    pub ac_depois: Option<u32>,
    pub dc_depois: Option<u32>,
    pub status: StatusDoAjuste,
    pub mensagem: String,
    /// Os `powercfg` que rodaram por este ajuste, para o registro em arquivo.
    ///
    /// FORA DO QUE VAI PARA A TELA, de propósito. Comando cru, código de saída
    /// e stderr servem a quem está depurando uma máquina pelo log; na tela do
    /// cliente seriam a poluição que faz um otimizador parecer um terminal. A
    /// tela já recebe o antes, o alvo, o depois e a frase — que é a mesma
    /// informação, dita para quem vai ler.
    #[serde(skip)]
    pub execucoes: Vec<Execucao>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DesfechoDoPlano {
    /// Todo ajuste aplicável entrou e foi conferido.
    Sucesso,
    /// Entrou o que dava; alguma coisa não existe nesta máquina ou não entrou.
    EmParte,
    /// O plano não pôde ser criado ou ativado.
    Falhou,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatorioDoPlano {
    pub maquina: Maquina,
    pub simulacao: bool,
    pub plano_existia: bool,
    pub guid_do_plano: Option<String>,
    pub guid_anterior: Option<String>,
    pub plano_ativo: bool,
    pub ajustes: Vec<ResultadoDoAjuste>,
    pub aplicados: usize,
    pub ja_estavam_bons: usize,
    pub nao_suportados: usize,
    pub falhas: usize,
    pub desfecho: DesfechoDoPlano,
}

/// O alvo de um ajuste nos dois modos. `None` quer dizer NÃO MEXER — é o que
/// protege a autonomia do notebook na bateria, e é diferente de "mexer para o
/// padrão".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Alvo {
    pub ac: Option<u32>,
    pub dc: Option<u32>,
}

impl Alvo {
    const fn nos_dois(v: u32) -> Self {
        Alvo { ac: Some(v), dc: Some(v) }
    }

    const fn so_na_tomada(v: u32) -> Self {
        Alvo { ac: Some(v), dc: None }
    }

    #[cfg(test)]
    const fn nenhum() -> Self {
        Alvo { ac: None, dc: None }
    }

    pub fn mexe_em_alguma_coisa(&self) -> bool {
        self.ac.is_some() || self.dc.is_some()
    }
}

pub struct Ajuste {
    pub nome: &'static str,
    pub subgrupo: &'static str,
    pub ajuste: &'static str,
    pub classe: Classe,
    pub porque: &'static str,
    pub alvo: fn(&Maquina) -> Alvo,
}

/// O DESKTOP LEVA O VALOR NOS DOIS MODOS; O NOTEBOOK, SÓ NA TOMADA.
///
/// Um desktop também tem `DCSettingIndex`, e ele não é decoração: numa queda de
/// energia com nobreak o Windows passa a usar o lado da bateria. Deixar os dois
/// iguais num desktop é o que faz o ajuste valer sempre.
///
/// No notebook, a bateria fica com o padrão do Windows de propósito. Estado
/// mínimo de processador em 100% fora da tomada é autonomia queimada e
/// temperatura alta por nada — e quem comprou um otimizador de jogo não pediu
/// isso.
fn tomada_sempre_bateria_se_desktop(m: &Maquina, v: u32) -> Alvo {
    if m.notebook {
        Alvo::so_na_tomada(v)
    } else {
        Alvo::nos_dois(v)
    }
}

pub static AJUSTES: &[Ajuste] = &[
    Ajuste {
        nome: "Estado mínimo do processador",
        subgrupo: SUB_PROCESSADOR,
        ajuste: PROCTHROTTLEMIN,
        classe: Classe::Recomendada,
        porque: "Com o mínimo baixo, o Windows derruba a frequência entre um quadro e outro \
                 e a leva de volta tarde demais. É a causa mais comum de engasgo em jogo \
                 sem que o uso de CPU pareça alto.",
        alvo: |m| tomada_sempre_bateria_se_desktop(m, 100),
    },
    Ajuste {
        nome: "Estado máximo do processador",
        subgrupo: SUB_PROCESSADOR,
        ajuste: PROCTHROTTLEMAX,
        classe: Classe::Segura,
        porque: "Garante que nada esteja limitando o teto da frequência. Em quase toda \
                 máquina já está em 100% — e nesse caso nada é escrito.",
        alvo: |_| Alvo::nos_dois(100),
    },
    Ajuste {
        nome: "Estacionamento de núcleos",
        subgrupo: SUB_PROCESSADOR,
        ajuste: CPMINCORES,
        classe: Classe::Recomendada,
        porque: "Manter 100% dos núcleos acordados evita o atraso de desestacionar um núcleo \
                 quando a carga chega de repente. Não desativa proteção nenhuma: o núcleo \
                 continua com todos os estados de economia dentro dele.",
        alvo: |m| tomada_sempre_bateria_se_desktop(m, 100),
    },
    Ajuste {
        nome: "Modo de aumento de desempenho (boost)",
        subgrupo: SUB_PROCESSADOR,
        ajuste: PERFBOOSTMODE,
        classe: Classe::Recomendada,
        porque: "Modo agressivo deixa o processador subir de frequência sem esperar a média \
                 de carga confirmar. É o comportamento que a Intel e a AMD documentam para \
                 carga de resposta rápida.",
        alvo: |m| tomada_sempre_bateria_se_desktop(m, 2),
    },
    Ajuste {
        nome: "Economia de energia do PCI Express (ASPM)",
        subgrupo: SUB_PCIEXPRESS,
        ajuste: ASPM,
        classe: Classe::Recomendada,
        porque: "O ASPM adormece a via até a placa de vídeo, e acordá-la custa em cada \
                 transferência. Desligado na tomada, o caminho até a GPU fica acordado.",
        alvo: |m| tomada_sempre_bateria_se_desktop(m, 0),
    },
    Ajuste {
        nome: "Desligar o disco rígido por inatividade",
        subgrupo: SUB_DISCO,
        ajuste: DISKIDLE,
        classe: Classe::Segura,
        porque: "Um HD que dormiu precisa de segundos para voltar, e o jogo trava no meio de \
                 um carregamento. Em SSD o ajuste não muda nada — e por isso ele é seguro \
                 nas duas máquinas.",
        alvo: |m| tomada_sempre_bateria_se_desktop(m, 0),
    },
    Ajuste {
        nome: "Suspensão seletiva do USB",
        subgrupo: SUB_USB,
        ajuste: USBSELECTIVESUSPEND,
        classe: Classe::Recomendada,
        porque: "É o que faz mouse e teclado perderem o primeiro movimento depois de um tempo \
                 parados. Na bateria continua ligado, porque ali ele economiza de verdade.",
        alvo: |_| Alvo::so_na_tomada(0),
    },
    // ─────────────────────────────────────────────────────────────────────
    // INCIDENTE 2.1.0 — ESTE AJUSTE DERRUBOU O FPS DE CLIENTES.
    //
    // Ele foi escrito com alvo `1` e o comentário dizia "preferir desempenho".
    // Isso estava ERRADO. O Windows documenta esta chave assim, lido da árvore
    // `PowerSettings` numa máquina real:
    //
    //     dd848b2a-…  "Policy to determine GPU preference"
    //        0 => None      — "No preference"
    //        1 => Low Power — "Prefer low-power GPU"
    //
    // Não existe valor "preferir alto desempenho" aqui. O `1` empurra o jogo
    // para a placa de BAIXO CONSUMO — em notebook e em desktop com vídeo
    // integrado, isso é o jogo saindo da placa dedicada. Um cliente relatou
    // cair de 200 para 80-120 FPS.
    //
    // POR QUE O ALVO É 0 EM VEZ DE O AJUSTE SER REMOVIDO: remover faria as
    // máquinas novas escaparem e deixaria as já atingidas com o `1` gravado
    // para sempre. Com alvo `0` — que é o padrão do Windows — quem já recebeu a
    // 2.1.0 é corrigido sozinho ao aplicar ou reparar o plano.
    //
    // A LIÇÃO, e ela vale mais que o conserto: eu inferi o significado de um
    // valor a partir do nome da chave em vez de ler a descrição que o próprio
    // Windows publica ao lado dela. Ver `os_valores_de_cada_ajuste_sao_os_que_o_windows_documenta`.
    // ─────────────────────────────────────────────────────────────────────
    Ajuste {
        nome: "Preferência de placa de vídeo do Windows",
        subgrupo: SUB_GRAFICOS,
        ajuste: GPUPREFERENCEPOLICY,
        classe: Classe::Segura,
        porque: "Garante que o Windows não esteja preferindo a placa de baixo consumo. O \
                 valor 1 desta chave significa \"preferir a placa mais econômica\" — o \
                 contrário do que se quer em jogo — e o 0 devolve a escolha ao aplicativo.",
        alvo: |_| Alvo::nos_dois(0),
    },
    Ajuste {
        nome: "Economia de energia do adaptador sem fio",
        subgrupo: SUB_SEM_FIO,
        ajuste: POWERSAVINGMODE,
        classe: Classe::Recomendada,
        porque: "O modo de economia do Wi-Fi junta pacotes para dormir entre eles, e isso \
                 vira variação de ping. Desligado na tomada; na bateria fica como estava.",
        alvo: |_| Alvo::so_na_tomada(0),
    },
    Ajuste {
        nome: "Não ociosar durante multimídia",
        subgrupo: SUB_MULTIMIDIA,
        ajuste: IDLEBACKGROUND,
        classe: Classe::Segura,
        porque: "Impede o PC de começar a dormir enquanto há mídia tocando ou sendo \
                 compartilhada com outro aparelho.",
        alvo: |_| Alvo::so_na_tomada(1),
    },
    Ajuste {
        nome: "Suspender o computador por inatividade",
        subgrupo: SUB_SUSPENSAO,
        ajuste: STANDBYIDLE,
        classe: Classe::Avancada,
        porque: "Nunca suspender na tomada evita o PC dormir durante um download longo — mas \
                 muda o comportamento que a pessoa escolheu, e por isso não entra sozinho.",
        alvo: |_| Alvo::so_na_tomada(0),
    },
];

/// As classes que entram sem ninguém pedir.
pub fn entra_por_padrao(classe: Classe) -> bool {
    matches!(classe, Classe::Segura | Classe::Recomendada)
}

// ─── Achar, criar e nomear o plano ────────────────────────────────────────────

/// Lê a lista do `powercfg /list`. Só o GUID e o nome entre parênteses são
/// estáveis; o resto da linha é traduzido.
pub fn planos_da_saida(saida: &str) -> Vec<(String, String)> {
    let mut planos = Vec::new();

    for linha in saida.lines() {
        let Some(inicio) = linha.find('(') else { continue };
        let Some(fim) = linha.rfind(')') else { continue };

        if fim <= inicio {
            continue;
        }

        let nome = linha[inicio + 1..fim].trim().to_string();

        let guid = linha[..inicio]
            .split_whitespace()
            .find(|t| e_guid(t))
            .map(|t| t.to_lowercase());

        if let Some(guid) = guid {
            planos.push((guid, nome));
        }
    }

    planos
}

/// Um GUID de plano tem 36 caracteres, quatro hífens nas posições conhecidas e
/// só dígitos hexadecimais no resto. Conferir isso é o que impede um pedaço de
/// texto traduzido de virar GUID.
pub fn e_guid(token: &str) -> bool {
    token.len() == 36
        && token.chars().enumerate().all(|(i, c)| {
            if matches!(i, 8 | 13 | 18 | 23) {
                c == '-'
            } else {
                c.is_ascii_hexdigit()
            }
        })
}

/// O GUID do plano com este nome, se ele já existir.
pub fn achar_na_lista(planos: &[(String, String)], nome: &str) -> Option<String> {
    planos
        .iter()
        .find(|(_, n)| n.eq_ignore_ascii_case(nome))
        .map(|(guid, _)| guid.clone())
}

fn listar_planos() -> Result<Vec<(String, String)>, String> {
    let saida = shell::run_checked("powercfg", &["/list"])?;
    Ok(planos_da_saida(&saida))
}

/// De qual plano o OTIMIZA é copiado.
///
/// O Alto Desempenho é o molde melhor — já vem com estacionamento de núcleos
/// desligado e o mínimo do processador alto. Mas ELE PODE NÃO EXISTIR: em
/// notebook com Modern Standby o Windows não o oferece. O Equilibrado existe
/// sempre, e é a rede de segurança que faltava.
pub fn escolher_molde(planos: &[(String, String)]) -> &'static str {
    if planos
        .iter()
        .any(|(guid, _)| guid == power::HIGH_PERFORMANCE_GUID)
    {
        power::HIGH_PERFORMANCE_GUID
    } else {
        EQUILIBRADO_GUID
    }
}

/// Cria o plano e devolve O GUID QUE O WINDOWS GEROU — nunca o do molde.
///
/// Esta função é o conserto do defeito principal.
fn criar_plano(molde: &str) -> Result<String, String> {
    let saida = shell::run_checked("powercfg", &["-duplicatescheme", molde])?;

    let novo = power::parse_active_guid(&saida).ok_or_else(|| {
        format!(
            "O Windows criou o plano mas não disse qual é o GUID dele. Resposta: {}",
            saida.trim()
        )
    })?;

    validar_guid_novo(&novo, molde)?;

    // O nome é o que reencontra o plano na próxima execução. Se ele não puder
    // ser trocado, o plano ficaria chamado como o molde e a execução seguinte
    // criaria outro — é a duplicação que este módulo existe para não fazer.
    shell::run_checked(
        "powercfg",
        &["-changename", &novo, NOME_DO_PLANO, DESCRICAO_DO_PLANO],
    )
    .map_err(|e| format!("O plano foi criado mas não pôde ser nomeado: {}", e))?;

    Ok(novo)
}

/// O GUID devolvido precisa ser um GUID, e precisa ser OUTRO.
///
/// Se um dia o `powercfg` responder o GUID do molde, gravar em cima dele seria
/// escrever no plano do cliente achando que se está escrevendo no nosso.
pub fn validar_guid_novo(novo: &str, molde: &str) -> Result<(), String> {
    if !e_guid(novo) {
        return Err(format!("`{}` não é um GUID de plano de energia.", novo));
    }

    if novo.eq_ignore_ascii_case(molde) {
        return Err(
            "O Windows devolveu o GUID do plano de origem em vez de um plano novo. \
             Nada foi alterado."
                .to_string(),
        );
    }

    Ok(())
}

/// Onde o plano OTIMIZA está, do ponto de vista da LISTA de otimizações.
///
/// Quatro estados e não dois, pelo motivo de sempre: "não consegui ler" não pode
/// virar "não está aplicado", senão a lista oferece ao cliente aplicar de novo o
/// que talvez já esteja lá.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstadoDoPlano {
    /// Existe e é o plano ativo.
    Ativo,
    /// Existe, mas o computador está usando outro.
    ExisteEnaoEstaAtivo,
    NaoExiste,
    /// O `powercfg` não respondeu.
    NaoConsegui,
}

/// Regra pura, separada da execução para poder ser testada.
pub fn estado_do_plano(nosso: Option<&str>, ativo: Option<&str>) -> EstadoDoPlano {
    match (nosso, ativo) {
        (_, None) => EstadoDoPlano::NaoConsegui,
        (None, Some(_)) => EstadoDoPlano::NaoExiste,
        (Some(n), Some(a)) if a.eq_ignore_ascii_case(n) => EstadoDoPlano::Ativo,
        (Some(_), Some(_)) => EstadoDoPlano::ExisteEnaoEstaAtivo,
    }
}

/// Checagem barata para a lista de otimizações: dois comandos, sem PowerShell.
///
/// A lista é redesenhada a cada atualização e chama isto uma vez por item do
/// catálogo. O relatório completo — ajuste por ajuste — sai do painel, que o
/// cliente abre quando quer.
pub fn onde_esta_o_plano() -> EstadoDoPlano {
    let Ok(planos) = listar_planos() else {
        return EstadoDoPlano::NaoConsegui;
    };

    let nosso = achar_na_lista(&planos, NOME_DO_PLANO);
    let ativo = power::active_scheme().ok();

    estado_do_plano(nosso.as_deref(), ativo.as_deref())
}

// ─── Vistoria: o plano continua de pé como o deixamos? ───────────────────────

/// O que uma vistoria encontrou. É LEITURA PURA: nada é escrito para descobrir.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "estado")]
pub enum Vistoria {
    /// Não há plano OTIMIZA nesta máquina.
    NaoExiste,
    /// Existe, está ativo, e todo ajuste está onde o deixamos.
    Integro,
    /// Existe e está íntegro, mas o computador está usando outro plano.
    ///
    /// Acontece sozinho com mais frequência do que se imagina: instalador de
    /// driver de vídeo, utilitário do fabricante e "otimizadores" concorrentes
    /// trocam o plano ativo sem avisar.
    DesativadoPorFora,
    /// Existe e algum ajuste saiu do alvo.
    Desviado {
        ativo: bool,
        /// Os ajustes que mudaram, pelo nome. É o que transforma "algo mudou"
        /// em uma frase que o cliente consegue conferir.
        ajustes: Vec<String>,
    },
    NaoConsegui,
}

/// Regra pura da vistoria, separada da leitura para poder ser testada.
///
/// A ORDEM IMPORTA: desvio vence "não está ativo". Um plano alterado E
/// desativado é, antes de tudo, um plano alterado — reativá-lo sem reparar
/// devolveria ao cliente os valores errados, com a tela dizendo que está tudo
/// certo.
pub fn classificar_vistoria(existe: bool, ativo: bool, desviados: Vec<String>) -> Vistoria {
    if !existe {
        return Vistoria::NaoExiste;
    }

    if !desviados.is_empty() {
        return Vistoria::Desviado {
            ativo,
            ajustes: desviados,
        };
    }

    if ativo {
        Vistoria::Integro
    } else {
        Vistoria::DesativadoPorFora
    }
}

/// Vistoria o plano. Não escreve nada.
pub fn vistoriar() -> Vistoria {
    // Simulação: lê cada ajuste do nosso plano sem tocar em nada. Num plano que
    // existe, `Mudaria` quer dizer exatamente "este valor não é o que
    // deixamos" — que é a definição de desvio.
    let Ok(relatorio) = montar(true, false) else {
        return Vistoria::NaoConsegui;
    };

    if !relatorio.plano_existia {
        return Vistoria::NaoExiste;
    }

    let desviados = relatorio
        .ajustes
        .iter()
        .filter(|a| a.status == StatusDoAjuste::Mudaria)
        .map(|a| a.nome.clone())
        .collect();

    let ativo = matches!(onde_esta_o_plano(), EstadoDoPlano::Ativo);

    classificar_vistoria(true, ativo, desviados)
}

/// Reaplica só o que saiu do alvo, e reativa o plano se preciso.
///
/// POR QUE ISTO É UM CAMINHO PRÓPRIO, E NÃO "aplicar de novo".
///
/// O motor recusa reaplicar uma otimização que já está no histórico — e com
/// razão, senão o "Otimizar agora" refaria tudo a cada clique. Só que o plano de
/// energia é a única otimização do catálogo que OUTRO PROGRAMA PODE DESFAZER
/// pelas costas: instalador de driver, utilitário do fabricante, ou um
/// concorrente trocando o plano ativo. Quando isso acontece, o histórico diz
/// "aplicada", a máquina discorda, e o cliente não tem botão nenhum — a lista
/// responde "Otimização já estava aplicada" sobre um PC que não está.
///
/// O reparo é a saída: ele não cria plano (se não existe, é criação, e criação
/// tem o seu próprio caminho com o registro de desfazer) e não mexe no
/// histórico, porque o estado anterior guardado lá continua sendo o certo — o
/// plano do cliente nunca deixou de ser o plano do cliente.
pub fn reparar(incluir_avancadas: bool) -> Result<RelatorioDoPlano, String> {
    if !registry::is_elevated() {
        return Err(
            "Reparar o plano de energia exige executar o Otimiza como administrador.".to_string(),
        );
    }

    let planos = listar_planos()?;

    if achar_na_lista(&planos, NOME_DO_PLANO).is_none() {
        return Err(
            "Não há plano OTIMIZA nesta máquina para reparar. Use \"Criar e ativar\"."
                .to_string(),
        );
    }

    crate::utils::Logger::info("plano OTIMIZA: reparo pedido");

    // Daqui para frente é o mesmo caminho da aplicação, e é de propósito: ele
    // já só escreve o que está fora do alvo e já confere cada escrita relendo.
    // Reparo não é um motor diferente; é o mesmo motor com outra porta.
    montar(false, incluir_avancadas)
}

// ─── Aplicar um ajuste, e provar que ele entrou ───────────────────────────────

/// Este Windows tem este ajuste?
///
/// Lido da árvore de definições, que é a mesma em qualquer idioma e não depende
/// de executar nada. Antes isto era descoberto executando e lendo a mensagem de
/// erro — que é traduzida.
///
/// Leitura negada conta como SUPORTADO, e não como ausente — o contrário do
/// resto do módulo, de propósito. Se a árvore de definições não pôde ser lida, a
/// alternativa seria marcar os onze ajustes como "este Windows não tem", que é
/// uma afirmação muito mais forte e muito mais provável de estar errada. Dizendo
/// que existe, a escrita é tentada e a releitura decide — e ela não mente.
pub fn suportado(subgrupo: &str, ajuste: &str) -> bool {
    registry::key_exists(
        "HKLM",
        &format!(
            r"SYSTEM\CurrentControlSet\Control\Power\PowerSettings\{}\{}",
            subgrupo, ajuste
        ),
    )
    .unwrap_or(true)
}

/// O que o relatório diz de um ajuste, dado o antes, o alvo e o depois.
///
/// Separada da execução de propósito: é aqui que mora a regra de "não confie no
/// código de saída", e ela precisa de teste sem depender de máquina nenhuma.
pub fn classificar(
    suportado: bool,
    alvo: &Alvo,
    ac_depois: Option<u32>,
    dc_depois: Option<u32>,
    erro: Option<&str>,
) -> StatusDoAjuste {
    if !suportado {
        return StatusDoAjuste::NaoSuportado;
    }

    if !alvo.mexe_em_alguma_coisa() {
        return StatusDoAjuste::Pulado;
    }

    if erro.is_some() {
        return StatusDoAjuste::FalhouAoAplicar;
    }

    let ac_ok = alvo.ac.map_or(true, |q| ac_depois == Some(q));
    let dc_ok = alvo.dc.map_or(true, |q| dc_depois == Some(q));

    if ac_ok && dc_ok {
        StatusDoAjuste::Aplicado
    } else {
        StatusDoAjuste::FalhouNaVerificacao
    }
}

/// O alvo já está satisfeito pelo que a máquina usa hoje?
pub fn ja_satisfeito(alvo: &Alvo, ac: Option<u32>, dc: Option<u32>) -> bool {
    alvo.mexe_em_alguma_coisa()
        && alvo.ac.map_or(true, |q| ac == Some(q))
        && alvo.dc.map_or(true, |q| dc == Some(q))
}

/// O que um `powercfg` deixou para trás, para o registro poder contar.
///
/// Você pediu comando, código de saída, stdout e stderr no log de cada ajuste,
/// e a razão é boa: sem o comando exato, quem lê o arquivo não consegue REPETIR
/// o que o produto fez — e repetir à mão é a primeira coisa que se faz para
/// entender uma falha numa máquina que não está na sua frente.
#[derive(Debug, Clone)]
pub struct Execucao {
    pub comando: String,
    pub codigo: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    /// `None` quando o comando deu certo.
    pub erro: Option<String>,
}

impl Execucao {
    /// Uma linha só, para caber no registro ao lado dos outros campos.
    ///
    /// Saída vazia não vira `stdout= stderr=`: campo vazio é ruído, e a linha
    /// já é longa. O `powercfg` que dá certo não escreve nada, que é o caso
    /// comum.
    pub fn resumo(&self) -> String {
        let mut partes = vec![format!("`{}`", self.comando)];

        partes.push(match self.codigo {
            Some(c) => format!("saiu {}", c),
            None => "não terminou sozinho".to_string(),
        });

        for (rotulo, texto) in [("stdout", &self.stdout), ("stderr", &self.stderr)] {
            let limpo = texto.trim();

            if !limpo.is_empty() {
                partes.push(format!("{}: {}", rotulo, limpo.replace('\n', " ")));
            }
        }

        partes.join(" | ")
    }
}

fn escrever(indice: &str, plano: &str, sub: &str, ajuste: &str, valor: u32) -> Execucao {
    let valor = valor.to_string();
    let args = [indice, plano, sub, ajuste, &valor];

    // O COMANDO É MONTADO A PARTIR DOS MESMOS ARGUMENTOS QUE RODAM, e não
    // escrito à mão numa string ao lado. Duas fontes divergem no primeiro
    // conserto, e um log que mostra um comando diferente do que rodou é pior
    // que um log sem comando nenhum.
    let comando = format!("powercfg {}", args.join(" "));

    match shell::run("powercfg", &args) {
        Ok(saida) => Execucao {
            comando,
            codigo: saida.codigo,
            erro: (!saida.success).then(|| {
                let detalhe = if saida.stderr.trim().is_empty() {
                    saida.stdout.trim()
                } else {
                    saida.stderr.trim()
                };

                format!("o Windows recusou o comando: {}", detalhe)
            }),
            stdout: saida.stdout,
            stderr: saida.stderr,
        },
        // Nem chegou a rodar, ou estourou o prazo. Não há código nem saída.
        Err(e) => Execucao {
            comando,
            codigo: None,
            stdout: String::new(),
            stderr: String::new(),
            erro: Some(e),
        },
    }
}

/// A linha que cada ajuste deixa em `otimiza.log`.
///
/// POR QUE ELA TEM TANTO NÚMERO. Esta é a linha que o cliente manda quando diz
/// "não funcionou no meu PC". Sozinha, ela precisa responder: o ajuste existe
/// naquele Windows? qual era o valor antes? qual pedimos? qual ficou depois? e
/// quanto tempo levou — que é o que separa "falhou" de "travou".
///
/// O antes E o depois juntos são o ponto. Uma linha dizendo só "falhou" manda o
/// atendimento adivinhar; uma dizendo `tomada 5->100 ficou 5` mostra que o
/// `powercfg` aceitou e o Windows não obedeceu, que é um problema completamente
/// diferente de o comando ter sido recusado.
///
/// Função pura, separada da execução para poder ser testada — e para que o
/// formato não mude por acidente quando alguém mexer no motor.
pub fn linha_do_log(r: &ResultadoDoAjuste, duracao: std::time::Duration) -> String {
    let n = |v: Option<u32>| match v {
        Some(v) => v.to_string(),
        None => "-".to_string(),
    };

    // ESPAÇO EM VOLTA DA SETA, e não `5->100`. Quando o valor é ausente ele sai
    // como `-`, e `-->-` é ilegível justamente na linha que existe para ser
    // lida por alguém tentando entender uma falha.
    let mut linha = format!(
        "plano OTIMIZA: [{:?}] {} | suportado={} | tomada {} -> {}, ficou {} | bateria {} -> {}, ficou {} | {} ms",
        r.status,
        r.nome,
        r.suportado,
        n(r.ac_antes),
        n(r.ac_alvo),
        n(r.ac_depois),
        n(r.dc_antes),
        n(r.dc_alvo),
        n(r.dc_depois),
        duracao.as_millis()
    );

    // A mensagem de erro do Windows entra INTEIRA e sem tradução. Traduzir uma
    // mensagem do `powercfg` a torna impesquisável, que é exatamente o que se
    // faz com ela.
    if !r.mensagem.is_empty() {
        linha.push_str(" | ");
        linha.push_str(&r.mensagem);
    }

    // O comando exato, o código e a saída, por último: são o que permite
    // REPETIR à mão o que o produto fez, numa máquina que não está na sua
    // frente. Ficam no fim porque quem lê procura primeiro o estado.
    for execucao in &r.execucoes {
        linha.push_str("\n    ");
        linha.push_str(&execucao.resumo());
    }

    linha
}

fn aplicar_um(plano: &str, a: &Ajuste, m: &Maquina, simulacao: bool) -> ResultadoDoAjuste {
    let alvo = (a.alvo)(m);
    let tem = suportado(a.subgrupo, a.ajuste);

    let ac_antes = power::valor_efetivo(plano, a.subgrupo, a.ajuste, false);
    let dc_antes = power::valor_efetivo(plano, a.subgrupo, a.ajuste, true);

    let mut resultado = ResultadoDoAjuste {
        execucoes: Vec::new(),
        nome: a.nome.to_string(),
        subgrupo: a.subgrupo.to_string(),
        ajuste: a.ajuste.to_string(),
        classe: a.classe,
        porque: a.porque.to_string(),
        suportado: tem,
        ac_antes,
        dc_antes,
        ac_alvo: alvo.ac,
        dc_alvo: alvo.dc,
        ac_depois: ac_antes,
        dc_depois: dc_antes,
        status: StatusDoAjuste::Pulado,
        mensagem: String::new(),
    };

    if !tem {
        resultado.status = StatusDoAjuste::NaoSuportado;
        resultado.mensagem = "Este Windows não tem este ajuste. O plano segue sem ele.".to_string();
        return resultado;
    }

    if !alvo.mexe_em_alguma_coisa() {
        resultado.mensagem = "Não se aplica a esta máquina.".to_string();
        return resultado;
    }

    if ja_satisfeito(&alvo, ac_antes, dc_antes) {
        resultado.status = StatusDoAjuste::JaEstavaBom;
        resultado.mensagem = "A máquina já estava assim. Nada foi escrito.".to_string();
        return resultado;
    }

    if simulacao {
        resultado.status = StatusDoAjuste::Mudaria;
        resultado.mensagem = "Mudaria se você aplicar. Nada foi escrito.".to_string();
        return resultado;
    }

    let mut erro: Option<String> = None;

    if let Some(v) = alvo.ac {
        let execucao = escrever("-setacvalueindex", plano, a.subgrupo, a.ajuste, v);
        erro = execucao.erro.clone();
        resultado.execucoes.push(execucao);
    }

    // Só tenta a bateria se a tomada deu certo: num ajuste que o Windows recusa,
    // repetir o comando do outro lado só enche o registro com a mesma recusa.
    if erro.is_none() {
        if let Some(v) = alvo.dc {
            let execucao = escrever("-setdcvalueindex", plano, a.subgrupo, a.ajuste, v);
            erro = execucao.erro.clone();
            resultado.execucoes.push(execucao);
        }
    }

    // NÃO CONFIE QUE FUNCIONOU: relê do Windows.
    resultado.ac_depois = power::valor_efetivo(plano, a.subgrupo, a.ajuste, false);
    resultado.dc_depois = power::valor_efetivo(plano, a.subgrupo, a.ajuste, true);

    resultado.status = classificar(
        tem,
        &alvo,
        resultado.ac_depois,
        resultado.dc_depois,
        erro.as_deref(),
    );

    resultado.mensagem = match resultado.status {
        StatusDoAjuste::Aplicado => "Gravado e conferido relendo do Windows.".to_string(),
        StatusDoAjuste::FalhouAoAplicar => {
            erro.unwrap_or_else(|| "O Windows recusou.".to_string())
        }
        StatusDoAjuste::FalhouNaVerificacao => format!(
            "O comando foi aceito, mas o valor relido não é o pedido \
             (tomada: {:?}, bateria: {:?}).",
            resultado.ac_depois, resultado.dc_depois
        ),
        _ => String::new(),
    };

    resultado
}

// ─── O fluxo inteiro ──────────────────────────────────────────────────────────

pub fn contar(ajustes: &[ResultadoDoAjuste]) -> (usize, usize, usize, usize) {
    let conta = |alvo: StatusDoAjuste| ajustes.iter().filter(|r| r.status == alvo).count();

    (
        conta(StatusDoAjuste::Aplicado),
        conta(StatusDoAjuste::JaEstavaBom),
        conta(StatusDoAjuste::NaoSuportado),
        conta(StatusDoAjuste::FalhouAoAplicar) + conta(StatusDoAjuste::FalhouNaVerificacao),
    )
}

/// O desfecho geral.
///
/// TRINTA AJUSTES BONS E DOIS QUE NÃO EXISTEM NAQUELE WINDOWS NÃO É FALHA. Essa
/// distinção é a diferença entre um relatório que ajuda e um "FALHOU" que não
/// diz nada.
pub fn desfecho(
    plano_ativo: bool,
    aplicados: usize,
    ja_bons: usize,
    nao_suportados: usize,
    falhas: usize,
) -> DesfechoDoPlano {
    if !plano_ativo {
        return DesfechoDoPlano::Falhou;
    }

    if falhas > 0 || nao_suportados > 0 {
        return DesfechoDoPlano::EmParte;
    }

    if aplicados + ja_bons == 0 {
        return DesfechoDoPlano::Falhou;
    }

    DesfechoDoPlano::Sucesso
}

/// Cria (ou reencontra), configura, ativa e CONFERE o plano OTIMIZA.
///
/// Em `simulacao`, nada é escrito: o relatório diz o que mudaria. É o modo para
/// olhar a máquina de um cliente antes de mexer nela.
pub fn montar(simulacao: bool, incluir_avancadas: bool) -> Result<RelatorioDoPlano, String> {
    let maquina = detectar();

    crate::utils::Logger::info(&format!(
        "plano OTIMIZA: começou (simulação={}, notebook={}, cpu={}, build={})",
        simulacao, maquina.notebook, maquina.cpu, maquina.build_do_windows
    ));

    if !simulacao && !registry::is_elevated() {
        return Err(
            "Criar um plano de energia exige executar o Otimiza como administrador.".to_string(),
        );
    }

    let guid_anterior = power::active_scheme().ok();
    let planos = listar_planos()?;
    let existente = achar_na_lista(&planos, NOME_DO_PLANO);
    let plano_existia = existente.is_some();

    let guid = match (&existente, simulacao) {
        (Some(g), _) => g.clone(),
        // Em simulação não se cria nada. Sem plano nosso, a leitura do "antes"
        // sai do plano ATIVO, que é a máquina como ela está hoje.
        (None, true) => guid_anterior
            .clone()
            .ok_or("Não foi possível ler o plano de energia ativo.")?,
        (None, false) => criar_plano(escolher_molde(&planos))?,
    };

    crate::utils::Logger::info(&format!(
        "plano OTIMIZA: guid={} (já existia={})",
        guid, plano_existia
    ));

    let mut ajustes = Vec::new();

    for a in AJUSTES {
        if !incluir_avancadas && !entra_por_padrao(a.classe) {
            ajustes.push(ResultadoDoAjuste {
                execucoes: Vec::new(),
                nome: a.nome.to_string(),
                subgrupo: a.subgrupo.to_string(),
                ajuste: a.ajuste.to_string(),
                classe: a.classe,
                porque: a.porque.to_string(),
                suportado: suportado(a.subgrupo, a.ajuste),
                ac_antes: None,
                dc_antes: None,
                ac_alvo: None,
                dc_alvo: None,
                ac_depois: None,
                dc_depois: None,
                status: StatusDoAjuste::Pulado,
                mensagem: "Fora do conjunto padrão: muda comportamento que a pessoa escolheu."
                    .to_string(),
            });
            continue;
        }

        // O relógio começa ANTES da leitura do "antes", e não só na escrita: um
        // `powercfg` que trava, trava em qualquer um dos dois, e a duração é o
        // que separa "falhou" de "ficou preso aqui".
        let relogio = std::time::Instant::now();
        let resultado = aplicar_um(&guid, a, &maquina, simulacao);

        crate::utils::Logger::info(&linha_do_log(&resultado, relogio.elapsed()));

        ajustes.push(resultado);
    }

    // Ativar depois de configurar, e CONFERIR relendo qual plano está ativo.
    let plano_ativo = if simulacao {
        false
    } else {
        match power::set_active_scheme(&guid) {
            Ok(()) => power::active_scheme()
                .map(|a| a.eq_ignore_ascii_case(&guid))
                .unwrap_or(false),
            Err(e) => {
                crate::utils::Logger::error(&format!("plano OTIMIZA: não ativou: {}", e));
                false
            }
        }
    };

    let (aplicados, ja_estavam_bons, nao_suportados, falhas) = contar(&ajustes);

    let desfecho_final = if simulacao {
        DesfechoDoPlano::EmParte
    } else {
        desfecho(plano_ativo, aplicados, ja_estavam_bons, nao_suportados, falhas)
    };

    crate::utils::Logger::info(&format!(
        "plano OTIMIZA: terminou — ativo={}, aplicados={}, já bons={}, sem suporte={}, falhas={}",
        plano_ativo, aplicados, ja_estavam_bons, nao_suportados, falhas
    ));

    Ok(RelatorioDoPlano {
        maquina,
        simulacao,
        plano_existia,
        guid_do_plano: Some(guid),
        guid_anterior,
        plano_ativo,
        ajustes,
        aplicados,
        ja_estavam_bons,
        nao_suportados,
        falhas,
        desfecho: desfecho_final,
    })
}

/// Reativa o plano que estava ativo antes e apaga o plano OTIMIZA.
///
/// Desfazer é UMA operação, e não trinta escritas de volta — porque nada foi
/// escrito no plano do cliente. É a razão inteira de o plano ser próprio.
pub fn desfazer(guid_anterior: &str) -> Result<(), String> {
    if !e_guid(guid_anterior) {
        return Err(format!("`{}` não é um GUID de plano de energia.", guid_anterior));
    }

    power::set_active_scheme(guid_anterior)?;

    let ativo = power::active_scheme()?;

    if !ativo.eq_ignore_ascii_case(guid_anterior) {
        return Err(format!(
            "O Windows aceitou o comando mas o plano ativo continua {}.",
            ativo
        ));
    }

    // Só depois de o plano anterior estar ATIVO E CONFERIDO. Um plano ativo não
    // pode ser apagado, e tentar antes deixaria a máquina sem plano nenhum se a
    // ativação tivesse falhado em silêncio.
    if let Some(nosso) = achar_na_lista(&listar_planos()?, NOME_DO_PLANO) {
        if let Err(e) = shell::run_checked("powercfg", &["-delete", &nosso]) {
            // O plano é inofensivo parado. A reversão já valeu.
            crate::utils::Logger::warn(&format!("plano OTIMIZA não foi apagado: {}", e));
        }
    }

    Ok(())
}

// ─── Diagnóstico: o que dá para saber ANTES de mexer em qualquer coisa ────────

/// O que o Otimiza consegue e não consegue nesta máquina.
///
/// Nada aqui escreve. Existe para responder, com o cliente do outro lado do
/// Discord, a pergunta que hoje não tem resposta: "por que não funcionou no seu
/// PC?". Cada campo é uma causa possível, conferida de verdade em vez de
/// suposta.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostico {
    pub maquina: Maquina,
    /// O PROCESSO está elevado — não é "o usuário pertence ao grupo
    /// Administradores", que é outra pergunta e não serve para nada aqui.
    pub elevado: bool,
    /// O `powercfg` responde nesta máquina.
    pub powercfg_responde: bool,
    /// Os planos de energia puderam ser listados.
    pub planos_legiveis: bool,
    pub planos: Vec<(String, String)>,
    /// O plano OTIMIZA já existe aqui.
    pub plano_otimiza_existe: bool,
    /// A árvore de definições de energia do registro pôde ser lida. Quando não,
    /// todo ajuste apareceria como "não suportado" — e seria mentira.
    pub registro_de_energia_legivel: bool,
    /// Quantos dos ajustes do produto existem neste Windows.
    pub ajustes_suportados: usize,
    pub ajustes_totais: usize,
    /// Os ajustes que este Windows não tem, pelo nome.
    pub ajustes_ausentes: Vec<String>,
    /// O processo é de 64 bits. Num processo de 32 bits sobre Windows de 64, as
    /// leituras do registro caem no espelho `WOW6432Node` e saem erradas.
    pub processo_64_bits: bool,
    pub avisos: Vec<String>,
}

/// Regra pura de quais avisos o diagnóstico levanta, separada da execução para
/// poder ser testada sem máquina.
pub fn avisos_do_diagnostico(
    elevado: bool,
    powercfg_responde: bool,
    registro_legivel: bool,
    ajustes_ausentes: usize,
    processo_64_bits: bool,
) -> Vec<String> {
    let mut avisos = Vec::new();

    if !elevado {
        avisos.push(
            "O Otimiza não está rodando como administrador. Nenhum plano de energia pode ser \
             criado assim."
                .to_string(),
        );
    }

    if !powercfg_responde {
        avisos.push(
            "O `powercfg` não respondeu. Sem ele não há como criar nem ativar plano de energia \
             nesta máquina."
                .to_string(),
        );
    }

    if !registro_legivel {
        avisos.push(
            "A árvore de energia do registro não pôde ser lida. Sem ela não dá para saber quais \
             ajustes existem neste Windows."
                .to_string(),
        );
    }

    if ajustes_ausentes > 0 {
        avisos.push(format!(
            "{} ajuste(s) do produto não existem neste Windows. O plano é montado sem eles, e \
             isso não é falha.",
            ajustes_ausentes
        ));
    }

    if !processo_64_bits {
        avisos.push(
            "O Otimiza está rodando em 32 bits. Num Windows de 64 bits as leituras do registro \
             caem no espelho WOW6432Node e saem erradas."
                .to_string(),
        );
    }

    avisos
}

pub fn diagnosticar() -> Diagnostico {
    let maquina = detectar();
    let elevado = registry::is_elevated();

    let lista = listar_planos();
    let powercfg_responde = lista.is_ok();
    let planos = lista.unwrap_or_default();

    // `None` (leitura negada) conta como ilegível: é exatamente o caso que
    // este campo do diagnóstico existe para denunciar.
    let registro_de_energia_legivel = registry::key_exists(
        "HKLM",
        r"SYSTEM\CurrentControlSet\Control\Power\PowerSettings",
    ) == Some(true);

    let ausentes: Vec<String> = AJUSTES
        .iter()
        .filter(|a| !suportado(a.subgrupo, a.ajuste))
        .map(|a| a.nome.to_string())
        .collect();

    let processo_64_bits = cfg!(target_pointer_width = "64");

    let avisos = avisos_do_diagnostico(
        elevado,
        powercfg_responde,
        registro_de_energia_legivel,
        ausentes.len(),
        processo_64_bits,
    );

    Diagnostico {
        plano_otimiza_existe: achar_na_lista(&planos, NOME_DO_PLANO).is_some(),
        planos_legiveis: powercfg_responde && !planos.is_empty(),
        ajustes_suportados: AJUSTES.len() - ausentes.len(),
        ajustes_totais: AJUSTES.len(),
        ajustes_ausentes: ausentes,
        maquina,
        elevado,
        powercfg_responde,
        planos,
        registro_de_energia_legivel,
        processo_64_bits,
        avisos,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_a_lista_de_planos_em_portugues() {
        let saida = "Esquemas de Energia Existentes (* Ativos)\r\n\
                     -----------------------------------\r\n\
                     GUID do Esquema de Energia: 381b4222-f694-41f0-9685-ff5bb260df2e  (Equilibrado)\r\n\
                     GUID do Esquema de Energia: 8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c  (Alto desempenho) *\r\n";

        let planos = planos_da_saida(saida);

        assert_eq!(planos.len(), 2);
        assert_eq!(planos[0].0, "381b4222-f694-41f0-9685-ff5bb260df2e");
        assert_eq!(planos[1].1, "Alto desempenho");
    }

    #[test]
    fn le_a_lista_em_ingles_tambem() {
        let saida = "Existing Power Schemes (* Active)\r\n\
                     Power Scheme GUID: 381b4222-f694-41f0-9685-ff5bb260df2e  (Balanced) *\r\n";

        assert_eq!(planos_da_saida(saida)[0].1, "Balanced");
    }

    #[test]
    fn o_cabecalho_com_parenteses_nao_vira_plano() {
        // "(* Ativos)" tem parênteses e nenhum GUID. Sem a conferência de GUID,
        // ele entraria na lista como um plano chamado "* Ativos".
        let saida = "Esquemas de Energia Existentes (* Ativos)\r\n";
        assert!(planos_da_saida(saida).is_empty());
    }

    #[test]
    fn acha_o_nosso_plano_sem_ligar_para_caixa() {
        let planos = vec![
            (
                "381b4222-f694-41f0-9685-ff5bb260df2e".to_string(),
                "Equilibrado".to_string(),
            ),
            (
                "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".to_string(),
                "Otimiza".to_string(),
            ),
        ];

        assert_eq!(
            achar_na_lista(&planos, NOME_DO_PLANO).as_deref(),
            Some("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee")
        );
    }

    #[test]
    fn sem_alto_desempenho_o_molde_e_o_equilibrado() {
        // O CASO DO CLIENTE. Notebook com Modern Standby não tem Alto
        // Desempenho, e era exatamente aí que o produto de antes tentava ativar
        // um GUID inexistente e falhava — deixando um plano órfão por tentativa.
        let planos = vec![
            (
                "381b4222-f694-41f0-9685-ff5bb260df2e".to_string(),
                "Equilibrado".to_string(),
            ),
            (
                "a1841308-3541-4fab-bc81-f71556f20b4a".to_string(),
                "Economia de energia".to_string(),
            ),
        ];

        assert_eq!(escolher_molde(&planos), EQUILIBRADO_GUID);
    }

    #[test]
    fn com_alto_desempenho_ele_e_o_molde() {
        let planos = vec![(
            power::HIGH_PERFORMANCE_GUID.to_string(),
            "Alto desempenho".to_string(),
        )];

        assert_eq!(escolher_molde(&planos), power::HIGH_PERFORMANCE_GUID);
    }

    #[test]
    fn o_guid_do_molde_devolvido_como_novo_e_recusado() {
        // Se isto passasse, gravaríamos dentro do plano do cliente achando que
        // era o nosso.
        assert!(validar_guid_novo(EQUILIBRADO_GUID, EQUILIBRADO_GUID).is_err());
        assert!(validar_guid_novo("nao-e-guid", EQUILIBRADO_GUID).is_err());
        assert!(validar_guid_novo("15a86c79-6f77-4d39-ab92-138b83b1b489", EQUILIBRADO_GUID).is_ok());
    }

    #[test]
    fn guid_precisa_ter_a_forma_certa() {
        assert!(e_guid("15a86c79-6f77-4d39-ab92-138b83b1b489"));
        assert!(!e_guid("15a86c79-6f77-4d39-ab92-138b83b1b48"));
        assert!(!e_guid("15a86c79z6f77-4d39-ab92-138b83b1b489"));
        assert!(!e_guid("Esquema de Energia Existente aqui!!!!"));
    }

    #[test]
    fn ajuste_inexistente_nao_e_falha() {
        // O DEFEITO QUE MAIS DOÍA NO PC DO CLIENTE: uma opção que não existe
        // naquele Windows derrubava a otimização inteira e desfazia o que já
        // tinha dado certo.
        let alvo = Alvo::nos_dois(100);
        assert_eq!(
            classificar(false, &alvo, None, None, None),
            StatusDoAjuste::NaoSuportado
        );
    }

    #[test]
    fn comando_aceito_com_valor_errado_nao_e_sucesso() {
        // A regra da casa: código de saída zero não é prova.
        let alvo = Alvo::nos_dois(100);
        assert_eq!(
            classificar(true, &alvo, Some(5), Some(5), None),
            StatusDoAjuste::FalhouNaVerificacao
        );
    }

    #[test]
    fn so_a_tomada_certa_nao_basta_quando_a_bateria_tambem_e_alvo() {
        let alvo = Alvo::nos_dois(100);
        assert_eq!(
            classificar(true, &alvo, Some(100), Some(5), None),
            StatusDoAjuste::FalhouNaVerificacao
        );
    }

    #[test]
    fn quando_a_bateria_nao_e_alvo_ela_nao_atrapalha() {
        // Notebook: a bateria fica no padrão do Windows de propósito, e isso
        // não pode fazer o ajuste da tomada parecer falho.
        let alvo = Alvo::so_na_tomada(0);
        assert_eq!(
            classificar(true, &alvo, Some(0), Some(2), None),
            StatusDoAjuste::Aplicado
        );
    }

    #[test]
    fn erro_do_powercfg_e_falha_de_aplicacao_e_nao_de_verificacao() {
        let alvo = Alvo::nos_dois(100);
        assert_eq!(
            classificar(true, &alvo, Some(100), Some(100), Some("acesso negado")),
            StatusDoAjuste::FalhouAoAplicar
        );
    }

    #[test]
    fn maquina_ja_no_alvo_nao_e_escrita_de_novo() {
        // Idempotência: rodar duas vezes não reescreve nada.
        assert!(ja_satisfeito(&Alvo::nos_dois(100), Some(100), Some(100)));
        assert!(!ja_satisfeito(&Alvo::nos_dois(100), Some(100), Some(5)));
        assert!(ja_satisfeito(&Alvo::so_na_tomada(0), Some(0), Some(2)));
        assert!(!ja_satisfeito(&Alvo::nenhum(), None, None));
    }

    #[test]
    fn bateria_herdando_o_padrao_do_windows_nao_conta_como_aplicada() {
        // VEIO DE `power::power_setting_satisfeito`, QUE FOI REMOVIDA NA
        // MIGRAÇÃO. O defeito que ela guardava foi medido na máquina do dono:
        // gravávamos só o lado da tomada, e a bateria seguia herdando o padrão
        // do Windows — 5% de estado mínimo do processador. Num notebook fora da
        // tomada a otimização não fazia nada, e a lista dizia que estava
        // aplicada.
        //
        // Num DESKTOP os dois lados são alvo, e herdar 5% na bateria continua
        // não sendo "aplicado".
        let alvo = Alvo::nos_dois(100);

        assert!(!ja_satisfeito(&alvo, Some(100), power::resolver_valor(None, Some(5))));
        assert!(ja_satisfeito(&alvo, Some(100), power::resolver_valor(None, Some(100))));
        assert!(!ja_satisfeito(&alvo, Some(100), None));
    }

    fn maquina_de_teste(notebook: bool) -> Maquina {
        Maquina {
            notebook,
            tem_bateria: notebook,
            fabricante_da_cpu: FabricanteDaCpu::Intel,
            cpu: "Intel(R) Core(TM) i3-10100F".to_string(),
            nucleos_logicos: 8,
            modern_standby: false,
            build_do_windows: 19045,
            windows11: false,
        }
    }

    #[test]
    fn notebook_nao_leva_estado_minimo_alto_na_bateria() {
        // Não é detalhe: 100% de estado mínimo fora da tomada queima autonomia e
        // esquenta o notebook por nada.
        let a = AJUSTES.iter().find(|a| a.ajuste == PROCTHROTTLEMIN).unwrap();

        assert_eq!((a.alvo)(&maquina_de_teste(true)), Alvo::so_na_tomada(100));
        assert_eq!((a.alvo)(&maquina_de_teste(false)), Alvo::nos_dois(100));
    }

    #[test]
    fn desktop_leva_o_valor_nos_dois_modos() {
        // Desktop também usa o lado "bateria" quando há nobreak.
        for a in AJUSTES.iter().filter(|a| entra_por_padrao(a.classe)) {
            let alvo = (a.alvo)(&maquina_de_teste(false));

            if let (Some(ac), Some(dc)) = (alvo.ac, alvo.dc) {
                assert_eq!(ac, dc, "ajuste {} discorda entre os modos num desktop", a.nome);
            }
        }
    }

    #[test]
    fn a_suspensao_por_inatividade_nao_entra_sozinha() {
        let a = AJUSTES.iter().find(|a| a.ajuste == STANDBYIDLE).unwrap();

        assert_eq!(a.classe, Classe::Avancada);
        assert!(!entra_por_padrao(a.classe));
    }

    #[test]
    fn nenhum_ajuste_desliga_protecao_termica() {
        // Trava de escopo: nada aqui pode mexer em limite térmico nem em política
        // de resfriamento. Se um ajuste desses for acrescentado um dia, este
        // teste obriga a encarar a decisão de propósito.
        const PROIBIDOS: &[&str] = &[
            // Política de resfriamento do sistema
            "94d3a615-a899-4ac5-ae2b-e4d8f634367f",
            // Limite de temperatura do processador
            "68dd2f27-a4ce-4e11-8487-3794e4135dfa",
        ];

        for a in AJUSTES {
            assert!(
                !PROIBIDOS.contains(&a.ajuste),
                "o ajuste `{}` mexe em proteção térmica",
                a.nome
            );
        }
    }

    #[test]
    fn todo_ajuste_tem_justificativa_escrita() {
        for a in AJUSTES {
            assert!(
                a.porque.len() > 40,
                "o ajuste `{}` não explica por que existe",
                a.nome
            );
            assert!(e_guid(a.subgrupo), "subgrupo de `{}` não é GUID", a.nome);
            assert!(e_guid(a.ajuste), "ajuste de `{}` não é GUID", a.nome);
        }
    }

    #[test]
    fn nenhum_ajuste_esta_repetido() {
        let mut vistos = std::collections::BTreeSet::new();

        for a in AJUSTES {
            assert!(
                vistos.insert((a.subgrupo, a.ajuste)),
                "o ajuste `{}` está duas vezes na lista",
                a.nome
            );
        }
    }

    #[test]
    fn dois_sem_suporte_entre_trinta_bons_nao_e_falha() {
        assert_eq!(desfecho(true, 28, 0, 2, 0), DesfechoDoPlano::EmParte);
        assert_eq!(desfecho(true, 30, 0, 0, 0), DesfechoDoPlano::Sucesso);
        assert_eq!(desfecho(true, 0, 30, 0, 0), DesfechoDoPlano::Sucesso);
    }

    #[test]
    fn plano_que_nao_ativou_e_falha_mesmo_com_tudo_gravado() {
        // Ajuste gravado em plano que não está ativo não muda nada na máquina.
        assert_eq!(desfecho(false, 30, 0, 0, 0), DesfechoDoPlano::Falhou);
    }

    #[test]
    fn le_o_chassi_e_a_bateria() {
        assert_eq!(
            ler_chassi_e_bateria("TIPO=2\r\nBATERIAS=1\r\n"),
            (Some(2), true)
        );
        assert_eq!(
            ler_chassi_e_bateria("TIPO=1\r\nBATERIAS=0\r\n"),
            (Some(1), false)
        );
        assert_eq!(ler_chassi_e_bateria("lixo"), (None, false));
    }

    #[test]
    fn bateria_sozinha_ja_faz_notebook() {
        // O `PCSystemType` pode vir errado em imagem modificada. A bateria não.
        assert!(e_notebook(Some(1), true));
        assert!(e_notebook(None, true));
        assert!(e_notebook(Some(2), false));
        assert!(!e_notebook(Some(1), false));
    }

    #[test]
    fn reconhece_o_fabricante_da_cpu() {
        assert_eq!(
            fabricante_pelo_nome("Intel(R) Core(TM) i3-10100F"),
            FabricanteDaCpu::Intel
        );
        assert_eq!(fabricante_pelo_nome("AMD Ryzen 5 5600X"), FabricanteDaCpu::Amd);
        assert_eq!(fabricante_pelo_nome("Ryzen 7 7800X3D"), FabricanteDaCpu::Amd);
        assert_eq!(
            fabricante_pelo_nome("Qualcomm Snapdragon"),
            FabricanteDaCpu::Outro
        );
    }

    #[test]
    fn separa_windows_10_de_11_pelo_build() {
        assert!(!e_windows11(19045));
        assert!(e_windows11(22000));
        assert!(e_windows11(26100));
    }

    #[test]
    fn simulacao_nao_chama_de_inaplicavel_o_que_mudaria() {
        // O defeito estava na TELA e foi visto na tela: o painel escrevia "não
        // se aplica aqui" sobre o modo de boost, que mudaria de 1 para 2. Os
        // dois casos chegavam como `Pulado`. O conserto é o estado separado —
        // e não uma frase diferente no TypeScript, que voltaria a se confundir
        // no primeiro campo novo.
        assert_ne!(StatusDoAjuste::Mudaria, StatusDoAjuste::Pulado);
    }

    fn resultado_de_teste(status: StatusDoAjuste) -> ResultadoDoAjuste {
        ResultadoDoAjuste {
            execucoes: Vec::new(),
            nome: "Estado mínimo do processador".to_string(),
            subgrupo: SUB_PROCESSADOR.to_string(),
            ajuste: PROCTHROTTLEMIN.to_string(),
            classe: Classe::Recomendada,
            porque: "porque sim".to_string(),
            suportado: true,
            ac_antes: Some(5),
            dc_antes: Some(5),
            ac_alvo: Some(100),
            dc_alvo: Some(100),
            ac_depois: Some(100),
            dc_depois: Some(100),
            status,
            mensagem: String::new(),
        }
    }

    #[test]
    fn o_resumo_da_execucao_traz_o_comando_e_o_codigo() {
        // O COMANDO EXATO É O PONTO. Sem ele, quem lê o log não consegue
        // repetir à mão o que o produto fez — e repetir à mão é a primeira
        // coisa que se faz numa máquina que não está na sua frente.
        let e = Execucao {
            comando: "powercfg -setacvalueindex SCHEME_CURRENT SUB AJUSTE 100".to_string(),
            codigo: Some(0),
            stdout: String::new(),
            stderr: String::new(),
            erro: None,
        };

        assert_eq!(
            e.resumo(),
            "`powercfg -setacvalueindex SCHEME_CURRENT SUB AJUSTE 100` | saiu 0"
        );
    }

    #[test]
    fn saida_vazia_nao_vira_campo_vazio() {
        // O `powercfg` que dá certo não escreve nada, e esse é o caso comum.
        // "stdout= stderr=" em toda linha é ruído numa linha que já é longa.
        let e = Execucao {
            comando: "powercfg /x".to_string(),
            codigo: Some(0),
            stdout: "   \n ".to_string(),
            stderr: String::new(),
            erro: None,
        };

        assert!(!e.resumo().contains("stdout"), "{}", e.resumo());
        assert!(!e.resumo().contains("stderr"), "{}", e.resumo());
    }

    #[test]
    fn a_recusa_do_windows_entra_com_codigo_e_saida() {
        let e = Execucao {
            comando: "powercfg -setacvalueindex A B C 1".to_string(),
            codigo: Some(1),
            stdout: "Esquema de energia, subgrupo ou configuração\nespecificada não existe."
                .to_string(),
            stderr: String::new(),
            erro: Some("o Windows recusou o comando".to_string()),
        };

        let r = e.resumo();

        assert!(r.contains("saiu 1"), "{}", r);
        // Numa linha só: o registro é lido linha a linha, e uma quebra no meio
        // parte a informação em duas que ninguém junta de volta.
        assert!(r.contains("especificada não existe."), "{}", r);
        assert!(!r.contains('\n'), "{}", r);
    }

    #[test]
    fn processo_sem_codigo_e_dito_e_nao_virado_zero() {
        // Prazo estourado: o processo foi encerrado por nós e não terminou
        // sozinho. Escrever "saiu 0" ali seria afirmar que deu certo.
        let e = Execucao {
            comando: "powercfg /x".to_string(),
            codigo: None,
            stdout: String::new(),
            stderr: String::new(),
            erro: Some("estourou o prazo".to_string()),
        };

        assert!(e.resumo().contains("não terminou sozinho"), "{}", e.resumo());
    }

    #[test]
    fn a_linha_do_log_mostra_o_antes_e_o_depois() {
        // O PONTO INTEIRO DESTA LINHA. Sem o "ficou", o atendimento não consegue
        // separar "o Windows recusou o comando" de "o Windows aceitou e não
        // obedeceu" — dois problemas completamente diferentes.
        let linha = linha_do_log(
            &resultado_de_teste(StatusDoAjuste::Aplicado),
            std::time::Duration::from_millis(42),
        );

        assert!(linha.contains("tomada 5 -> 100, ficou 100"), "{}", linha);
        assert!(linha.contains("bateria 5 -> 100, ficou 100"), "{}", linha);
        assert!(linha.contains("42 ms"), "{}", linha);
        assert!(linha.contains("Aplicado"), "{}", linha);
    }

    #[test]
    fn o_valor_ausente_vira_traco_e_nao_zero() {
        // "0" é um valor válido de vários ajustes de energia — ASPM desligado é
        // 0. Imprimir ausência como zero faria o registro afirmar um valor que
        // não existe, justamente no arquivo que serve de prova.
        let mut r = resultado_de_teste(StatusDoAjuste::Aplicado);
        r.dc_antes = None;
        r.dc_alvo = None;
        r.dc_depois = None;

        let linha = linha_do_log(&r, std::time::Duration::from_millis(1));

        assert!(linha.contains("bateria - -> -, ficou -"), "{}", linha);
        assert!(!linha.contains("bateria 0"), "{}", linha);
    }

    #[test]
    fn a_mensagem_do_windows_entra_inteira() {
        // Sem tradução e sem corte: mensagem de erro traduzida é mensagem
        // impesquisável, e pesquisar é a primeira coisa que se faz com ela.
        let mut r = resultado_de_teste(StatusDoAjuste::FalhouAoAplicar);
        r.mensagem = "Acesso negado. (5)".to_string();

        let linha = linha_do_log(&r, std::time::Duration::from_millis(3));

        assert!(linha.ends_with("Acesso negado. (5)"), "{}", linha);
    }

    #[test]
    fn ajuste_sem_mensagem_nao_deixa_separador_solto() {
        let linha = linha_do_log(
            &resultado_de_teste(StatusDoAjuste::JaEstavaBom),
            std::time::Duration::from_millis(1),
        );

        assert!(!linha.ends_with(" | "), "{}", linha);
    }

    #[test]
    fn desvio_vence_o_plano_estar_desativado() {
        // Um plano alterado E desativado é, antes de tudo, um plano alterado.
        // Reativá-lo sem reparar devolveria ao cliente os valores errados, com
        // a tela dizendo que está tudo certo — que é pior do que deixá-lo
        // desativado.
        assert_eq!(
            classificar_vistoria(true, false, vec!["Estacionamento de núcleos".into()]),
            Vistoria::Desviado {
                ativo: false,
                ajustes: vec!["Estacionamento de núcleos".into()]
            }
        );
    }

    #[test]
    fn a_vistoria_diz_quais_ajustes_mudaram() {
        // "Algo mudou" não serve para nada. O nome do ajuste é o que o cliente
        // consegue conferir sozinho no painel do Windows — e é o que separa
        // este aviso de um alarme genérico de otimizador.
        let v = classificar_vistoria(
            true,
            true,
            vec!["Estado mínimo do processador".into(), "ASPM".into()],
        );

        match v {
            Vistoria::Desviado { ajustes, ativo } => {
                assert!(ativo);
                assert_eq!(ajustes.len(), 2);
            }
            outro => panic!("esperava desvio, veio {:?}", outro),
        }
    }

    #[test]
    fn plano_intacto_mas_trocado_por_fora_tem_nome_proprio() {
        // Instalador de driver de vídeo e utilitário de fabricante trocam o
        // plano ativo sem avisar. É diferente de alguém ter mexido nos valores,
        // e o conserto é outro: aqui basta reativar.
        assert_eq!(
            classificar_vistoria(true, false, vec![]),
            Vistoria::DesativadoPorFora
        );
        assert_eq!(classificar_vistoria(true, true, vec![]), Vistoria::Integro);
    }

    #[test]
    fn sem_plano_nao_ha_o_que_vistoriar() {
        assert_eq!(classificar_vistoria(false, false, vec![]), Vistoria::NaoExiste);
        // Nem mesmo com desvio: sem plano nosso, os valores lidos são do plano
        // do cliente, e chamá-los de "desvio" seria acusar o dono da máquina de
        // ter mexido no que é dele.
        assert_eq!(
            classificar_vistoria(false, false, vec!["qualquer".into()]),
            Vistoria::NaoExiste
        );
    }

    #[test]
    fn reparar_sem_plano_recusa_em_vez_de_criar() {
        // Criar tem caminho próprio, com o registro de desfazer. Se o reparo
        // criasse, o cliente ficaria com um plano ativo e SEM linha no
        // histórico — ou seja, sem botão de voltar.
        if registry::is_elevated() && onde_esta_o_plano() == EstadoDoPlano::NaoExiste {
            let erro = reparar(false).unwrap_err();
            assert!(erro.contains("Criar e ativar"), "{}", erro);
        }
    }

    #[test]
    fn nao_conseguir_ler_nao_e_plano_ausente() {
        // Se isto virasse `NaoExiste`, a lista ofereceria aplicar de novo o que
        // talvez já esteja aplicado — e o cliente criaria um plano em cima do
        // outro por causa de um `powercfg` que não respondeu.
        assert_eq!(estado_do_plano(Some("a"), None), EstadoDoPlano::NaoConsegui);
        assert_eq!(estado_do_plano(None, None), EstadoDoPlano::NaoConsegui);
    }

    #[test]
    fn o_plano_so_esta_aplicado_quando_e_o_ativo() {
        // Existir e estar ativo são coisas diferentes: ajuste gravado em plano
        // parado não muda nada na máquina.
        assert_eq!(
            estado_do_plano(Some("AAAAAAAA-bbbb-cccc-dddd-eeeeeeeeeeee"), Some("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee")),
            EstadoDoPlano::Ativo
        );
        assert_eq!(
            estado_do_plano(Some("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"), Some("381b4222-f694-41f0-9685-ff5bb260df2e")),
            EstadoDoPlano::ExisteEnaoEstaAtivo
        );
        assert_eq!(
            estado_do_plano(None, Some("381b4222-f694-41f0-9685-ff5bb260df2e")),
            EstadoDoPlano::NaoExiste
        );
    }

    #[test]
    fn desfazer_recusa_guid_invalido() {
        assert!(desfazer("nada disso").is_err());
    }

    #[test]
    fn sem_administrador_o_diagnostico_avisa() {
        let avisos = avisos_do_diagnostico(false, true, true, 0, true);
        assert_eq!(avisos.len(), 1);
        assert!(avisos[0].contains("administrador"));
    }

    #[test]
    fn maquina_inteira_em_ordem_nao_levanta_aviso() {
        assert!(avisos_do_diagnostico(true, true, true, 0, true).is_empty());
    }

    #[test]
    fn ajuste_ausente_vira_aviso_e_nao_erro() {
        // A frase precisa dizer que NÃO é falha: é exatamente a confusão que
        // fazia o produto reportar fracasso na máquina do cliente.
        let avisos = avisos_do_diagnostico(true, true, true, 2, true);
        assert_eq!(avisos.len(), 1);
        assert!(avisos[0].contains("não é falha"));
    }

    /// Roda o diagnóstico e a SIMULAÇÃO contra a máquina de verdade e imprime o
    /// relatório. Não escreve nada.
    ///
    /// `#[ignore]` de propósito: depende da máquina, e o resto da suíte é de
    /// função pura para poder rodar na esteira. Para ver:
    ///
    ///   cargo test --lib planoenergia -- --ignored --nocapture
    #[test]
    #[ignore]
    fn relatorio_desta_maquina() {
        let d = diagnosticar();

        println!("\n=== DIAGNÓSTICO ===");
        println!("CPU .................. {} ({:?})", d.maquina.cpu, d.maquina.fabricante_da_cpu);
        println!("Notebook ............. {}", d.maquina.notebook);
        println!("Bateria .............. {}", d.maquina.tem_bateria);
        println!("Modern Standby ....... {}", d.maquina.modern_standby);
        println!("Build do Windows ..... {} (Win11={})", d.maquina.build_do_windows, d.maquina.windows11);
        println!("Elevado .............. {}", d.elevado);
        println!("powercfg responde .... {}", d.powercfg_responde);
        println!("Planos legíveis ...... {}", d.planos_legiveis);
        println!("Plano OTIMIZA existe . {}", d.plano_otimiza_existe);
        println!("Ajustes suportados ... {}/{}", d.ajustes_suportados, d.ajustes_totais);
        println!("Ausentes ............. {:?}", d.ajustes_ausentes);
        println!("Processo 64 bits ..... {}", d.processo_64_bits);

        for a in &d.avisos {
            println!("AVISO: {}", a);
        }

        println!("\n=== SIMULAÇÃO (nada é escrito) ===");

        match montar(true, false) {
            Ok(r) => {
                for a in &r.ajustes {
                    println!(
                        "[{:?}] {} — tomada {:?}->{:?} bateria {:?}->{:?} {}",
                        a.status, a.nome, a.ac_antes, a.ac_alvo, a.dc_antes, a.dc_alvo, a.mensagem
                    );
                }
                println!(
                    "já bons={} sem suporte={} pulados={}",
                    r.ja_estavam_bons,
                    r.nao_suportados,
                    r.ajustes.len() - r.ja_estavam_bons - r.nao_suportados
                );
            }
            Err(e) => println!("FALHOU: {}", e),
        }
    }

    /// O CAMINHO INTEIRO, contra o Windows de verdade: cria o plano, configura,
    /// ativa, confere relendo, RODA DE NOVO para provar a idempotência, e
    /// desfaz — deixando a máquina como estava.
    ///
    /// `#[ignore]`: escreve na máquina. Só roda quando alguém pede, e precisa de
    /// administrador.
    ///
    ///   cargo test --lib planoenergia -- --ignored --nocapture
    #[test]
    #[ignore]
    fn cria_configura_ativa_confere_e_desfaz() {
        if !registry::is_elevated() {
            println!("PULADO: precisa de administrador.");
            return;
        }

        let antes = power::active_scheme().expect("plano ativo");
        println!("\nplano ativo antes: {}", antes);

        let primeira = montar(false, false).expect("primeira passada");

        println!(
            "1ª passada: existia={} guid={:?} ativo={} aplicados={} já bons={} sem suporte={} falhas={} desfecho={:?}",
            primeira.plano_existia,
            primeira.guid_do_plano,
            primeira.plano_ativo,
            primeira.aplicados,
            primeira.ja_estavam_bons,
            primeira.nao_suportados,
            primeira.falhas,
            primeira.desfecho
        );

        for a in primeira.ajustes.iter().filter(|a| {
            !matches!(a.status, StatusDoAjuste::JaEstavaBom | StatusDoAjuste::Pulado)
        }) {
            println!("  [{:?}] {} — {}", a.status, a.nome, a.mensagem);
        }

        assert!(primeira.plano_ativo, "o plano OTIMIZA não ficou ativo");
        assert_eq!(primeira.falhas, 0, "houve falha de aplicação ou de verificação");

        let guid = primeira.guid_do_plano.clone().unwrap();
        assert_ne!(guid, antes, "o plano criado é o mesmo que já estava ativo");

        // IDEMPOTÊNCIA: a segunda passada não cria plano nenhum e não escreve
        // nada — tudo que ela toca já está no alvo.
        let segunda = montar(false, false).expect("segunda passada");

        println!(
            "2ª passada: existia={} guid={:?} aplicados={} já bons={}",
            segunda.plano_existia, segunda.guid_do_plano, segunda.aplicados, segunda.ja_estavam_bons
        );

        assert!(segunda.plano_existia, "a segunda passada não reencontrou o plano");
        assert_eq!(segunda.guid_do_plano, primeira.guid_do_plano, "criou um plano duplicado");
        assert_eq!(segunda.aplicados, 0, "a segunda passada reescreveu o que já estava bom");

        // E o Windows precisa ter UM plano chamado OTIMIZA, e não três.
        let lista = listar_planos().unwrap();
        let nossos = lista.iter().filter(|(_, n)| n.eq_ignore_ascii_case(NOME_DO_PLANO)).count();
        assert_eq!(nossos, 1, "sobrou mais de um plano OTIMIZA na máquina");

        desfazer(&antes).expect("desfazer");

        assert_eq!(power::active_scheme().unwrap(), antes, "não voltou ao plano de antes");
        assert!(
            achar_na_lista(&listar_planos().unwrap(), NOME_DO_PLANO).is_none(),
            "o plano OTIMIZA continua na máquina depois do desfazer"
        );

        println!("desfeito: plano ativo voltou a {} e o OTIMIZA foi apagado", antes);
    }

    /// O CAMINHO DO REPARO, contra o Windows de verdade.
    ///
    /// Cria o plano, **estraga um ajuste por fora** — como faria um instalador
    /// de driver ou um concorrente —, confere que a vistoria acha e nomeia o
    /// ajuste, repara, confere que ficou íntegro, e desfaz.
    ///
    /// `#[ignore]`: escreve na máquina e precisa de administrador.
    ///
    ///   cargo test --lib planoenergia -- --ignored --nocapture
    #[test]
    #[ignore]
    fn vistoria_acha_o_desvio_e_o_reparo_conserta() {
        if !registry::is_elevated() {
            println!("PULADO: precisa de administrador.");
            return;
        }

        let antes = power::active_scheme().expect("plano ativo");
        let inicial = montar(false, false).expect("criar o plano");

        assert!(inicial.plano_ativo, "o plano não ficou ativo");
        assert_eq!(vistoriar(), Vistoria::Integro, "nasceu desviado");

        let guid = inicial.guid_do_plano.clone().unwrap();

        // ── Alguém mexe no plano por fora ────────────────────────────────
        //
        // Estacionamento de núcleos de 100 para 50: valor válido, aceito pelo
        // Windows, e diferente do que deixamos. É exatamente o que um
        // "otimizador" concorrente faz.
        shell::run_checked(
            "powercfg",
            &["-setacvalueindex", &guid, SUB_PROCESSADOR, CPMINCORES, "50"],
        )
        .expect("estragar o ajuste");

        let vistoria = vistoriar();
        println!("depois de estragar: {:?}", vistoria);

        match &vistoria {
            Vistoria::Desviado { ajustes, ativo } => {
                assert!(*ativo, "o plano deveria continuar ativo");
                assert!(
                    ajustes.iter().any(|a| a.contains("Estacionamento")),
                    "a vistoria não nomeou o ajuste que eu estraguei: {:?}",
                    ajustes
                );
            }
            outro => panic!("a vistoria não viu o desvio: {:?}", outro),
        }

        // ── O reparo ─────────────────────────────────────────────────────
        let reparo = reparar(false).expect("reparar");

        println!(
            "reparo: aplicados={} já bons={} falhas={}",
            reparo.aplicados, reparo.ja_estavam_bons, reparo.falhas
        );

        assert_eq!(reparo.falhas, 0, "o reparo falhou");
        assert!(
            reparo.aplicados >= 1,
            "o reparo não reescreveu nada, e havia um ajuste fora do lugar"
        );

        assert_eq!(vistoriar(), Vistoria::Integro, "continuou desviado");

        desfazer(&antes).expect("desfazer");
        assert_eq!(power::active_scheme().unwrap(), antes);

        println!("plano ativo voltou a {}", antes);
    }

    #[test]
    fn processo_de_32_bits_e_avisado() {
        let avisos = avisos_do_diagnostico(true, true, true, 0, false);
        assert!(avisos.iter().any(|a| a.contains("WOW6432Node")));
    }

    // ── Incidente 2.1.0: a preferência de placa de vídeo ──────────────────
    //
    // A 2.1.0 gravou 1 nesta chave achando que era "preferir desempenho". O
    // Windows documenta 1 como "Prefer low-power GPU". Em máquina com gráficos
    // híbridos isso tira o jogo da placa dedicada, e um cliente caiu de ~200
    // para 80-120 FPS. As três travas abaixo existem para que esse valor não
    // volte por descuido nem sobreviva num PC já atingido.

    #[test]
    fn a_preferencia_de_placa_de_video_nunca_pede_a_placa_de_baixo_consumo() {
        let ajuste = AJUSTES
            .iter()
            .find(|a| a.ajuste == GPUPREFERENCEPOLICY)
            .expect("o ajuste precisa continuar na tabela para consertar quem já recebeu a 2.1.0");

        for notebook in [false, true] {
            let alvo = (ajuste.alvo)(&maquina_de_teste(notebook));

            assert_ne!(
                alvo.ac,
                Some(1),
                "1 é \"preferir a placa de baixo consumo\" — foi isso que derrubou o FPS na 2.1.0"
            );
            assert_ne!(alvo.dc, Some(1), "idem na bateria");
        }
    }

    #[test]
    fn a_preferencia_de_placa_de_video_desfaz_o_estrago_da_2_1_0() {
        // Não basta parar de escrever 1: quem já aplicou a 2.1.0 tem o 1
        // gravado. `montar` e `reparar` só tocam o que está na tabela, então o
        // alvo precisa ser 0 nos dois modos, em notebook e em desktop, para que
        // a próxima aplicação ou reparo devolva a máquina ao padrão do Windows.
        let ajuste = AJUSTES
            .iter()
            .find(|a| a.ajuste == GPUPREFERENCEPOLICY)
            .expect("ajuste sumiu");

        for notebook in [false, true] {
            let alvo = (ajuste.alvo)(&maquina_de_teste(notebook));
            assert_eq!(alvo.ac, Some(0), "na tomada precisa reescrever 0");
            assert_eq!(alvo.dc, Some(0), "na bateria também, senão o 1 fica lá");
        }
    }

    #[test]
    fn os_valores_de_cada_ajuste_sao_os_que_o_windows_documenta() {
        // A causa raiz do incidente não foi o número: foi eu ter deduzido o
        // significado do valor a partir do NOME da chave. O texto de `porque`
        // é o único lugar onde esse significado fica escrito, então ele não
        // pode ser vago. Cada ajuste que grava um número precisa explicar o
        // que o número quer dizer para o Windows.
        for ajuste in AJUSTES {
            assert!(
                ajuste.porque.len() >= 60,
                "{}: a justificativa precisa dizer o que o valor significa para o Windows, \
                 não só que ele é bom",
                ajuste.nome
            );
        }
    }
}
