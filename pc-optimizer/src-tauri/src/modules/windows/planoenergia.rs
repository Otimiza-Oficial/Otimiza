// O plano de energia OTIMIZA, próprio. Antes o produto ativava o "Alto Desempenho" por GUID fixo (que não existe
// em Modern Standby ou imagem OEM enxuta: o `-duplicatescheme` criava cópia com GUID NOVO, jogado fora, e cada
// tentativa deixava um plano órfão) e escrevia DENTRO do plano do cliente. Agora cria o OTIMIZA, guarda o GUID que o
// Windows devolveu, configura só nele e o ativa; desfazer é reativar o anterior. Cada ajuste é RELIDO depois de
// gravado (`FalhouNaVerificacao` ≠ `FalhouAoAplicar`). Ajuste ausente (conferido em `Control\Power\PowerSettings`,
// igual em qualquer idioma) vira `NaoSuportado` e o plano segue.

use super::{hardware, power, registry, shell};
use serde::{Deserialize, Serialize};

/// É por ele que o plano é reencontrado: não pode ser traduzido.
pub const NOME_DO_PLANO: &str = "OTIMIZA";

const DESCRICAO_DO_PLANO: &str =
    "Plano criado pelo Otimiza para este computador. Pode ser apagado a qualquer momento.";

/// Existe em toda instalação, inclusive Modern Standby.
pub const EQUILIBRADO_GUID: &str = "381b4222-f694-41f0-9685-ff5bb260df2e";

pub const SUB_PROCESSADOR: &str = "54533251-82be-4824-96c1-47b60b740d00";
const PROCTHROTTLEMIN: &str = "893dee8e-2bef-41e0-89c6-b55d0929964c";
const PROCTHROTTLEMAX: &str = "bc5038f7-23e0-4960-96da-33abaf5935ec";
const CPMINCORES: &str = "0cc5b647-c1df-4637-891a-dec35c318583";
const PERFBOOSTMODE: &str = "be337238-0d82-4146-a960-4f3749d470c7";

/// 0 a 100 POR CENTO de ECONOMIA ("favor energy savings over performance"): 0 é desempenho total. Escrever 100
/// achando que é o máximo repetiria o erro da 2.1.0.
const PERFEPP: &str = "36687f9e-e3a5-4dbf-b1dc-15eb381c6863";

/// Só LÊ e relata: trocar quem manda no processador por cima do fabricante é o tweak de internet que o projeto
/// recusa. Com `1` governa o EPP; com `0`, o mínimo/máximo. Os TRÊS planos internos trazem `0` (conferido; a
/// primeira versão deste comentário inferia o contrário). Ver `governa_o_processador`.
const PERFAUTONOMOUS: &str = "8baa4a8a-14c6-4451-8e8b-14bdbd197537";

/// Constante publicada pela Microsoft. Só para LER o padrão de fábrica quando o plano ativo é próprio (fora de
/// `DefaultPowerSchemeValues`).
const EQUILIBRADO: &str = "381b4222-f694-41f0-9685-ff5bb260df2e";

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
    /// Do registro, não do `powercfg /a`, que é traduzido.
    pub modern_standby: bool,
    pub build_do_windows: u32,
    pub windows11: bool,
}

/// `CurrentVersion` diz 10.0 nos dois: o build 22000 é a única separação.
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

/// Em imagem modificada o `PCSystemType` pode vir errado: a bateria decide.
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

/// Tomada e bateria usam lados opostos do plano: sem isto, "mínimo = 5%" não quer dizer nada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Alimentacao {
    Tomada,
    Bateria,
    NaoSei,
}

/// `SYSTEM_POWER_STATUS`: 0 fora da tomada, 1 na tomada, 255 desconhecido. Números, iguais em qualquer idioma.
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

    // Chamada falha e máquina que não sabe dão `NaoSei`: nos dois casos não temos a resposta.
    if unsafe { GetSystemPowerStatus(&mut status) } == 0 {
        return Alimentacao::NaoSei;
    }

    alimentacao_do_status(status.ACLineStatus)
}

/// Decide desktop ou notebook, e isso muda todos os valores da bateria.
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

/// Por padrão só `Segura` e `Recomendada`. `Avancada` existe para o relatório dizer que não foi aplicada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Classe {
    Segura,
    Recomendada,
    Avancada,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatusDoAjuste {
    Aplicado,
    JaEstavaBom,
    NaoSuportado,
    FalhouAoAplicar,
    FalhouNaVerificacao,
    /// Separado de `Pulado`: a tela dizia "não se aplica aqui" sobre o boost que mudaria de 1 para 2.
    Mudaria,
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
    /// Fora da tela de propósito: comando cru e stderr são para o log.
    #[serde(skip)]
    pub execucoes: Vec<Execucao>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DesfechoDoPlano {
    Sucesso,
    EmParte,
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

/// `None` é NÃO MEXER, diferente de "mexer para o padrão": protege a autonomia do notebook na bateria.
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

/// Desktop nos dois lados (com nobreak o Windows usa o da bateria). Notebook só na tomada: mínimo 100% fora dela é
/// autonomia queimada.
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
        classe: Classe::Avancada,
        porque: "Substituído pelo motor de energia adaptativo (aba Energia): este número só é certo quando medido nesta máquina, e o plano não o aplica mais sozinho. Com o mínimo baixo, o Windows derruba a frequência entre um quadro e outro \
                 e a leva de volta tarde demais. IMPORTANTE: este ajuste só governa quando \
                 o processador NÃO está em modo autônomo — em Intel com Speed Shift e AMD \
                 com CPPC, que é quase todo PC recente, quem manda é a preferência de \
                 energia (EPP) logo abaixo. O diagnóstico de energia diz qual é o caso aqui.",
        alvo: |m| tomada_sempre_bateria_se_desktop(m, 100),
    },
    Ajuste {
        nome: "Preferência entre energia e desempenho (EPP)",
        subgrupo: SUB_PROCESSADOR,
        ajuste: PERFEPP,
        classe: Classe::Avancada,
        porque: "Substituído pelo motor de energia adaptativo (aba Energia): este número só é certo quando medido nesta máquina, e o plano não o aplica mais sozinho. É este número que comanda a frequência nos processadores modernos, e não o \
                 estado mínimo acima. O Windows o define como \"o quanto o processador deve \
                 favorecer ECONOMIA sobre desempenho\", de 0 a 100 por cento — então 0 é \
                 desempenho total. Na tomada vai a 0; na bateria fica como o fabricante \
                 deixou, porque ali economia é o que a pessoa quer.",
        alvo: |_| Alvo::so_na_tomada(0),
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
        classe: Classe::Avancada,
        porque: "Substituído pelo motor de energia adaptativo (aba Energia): este número só é certo quando medido nesta máquina, e o plano não o aplica mais sozinho. Manter 100% dos núcleos acordados evita o atraso de desestacionar um núcleo \
                 quando a carga chega de repente. Não desativa proteção nenhuma: o núcleo \
                 continua com todos os estados de economia dentro dele.",
        alvo: |m| tomada_sempre_bateria_se_desktop(m, 100),
    },
    Ajuste {
        nome: "Modo de aumento de desempenho (boost)",
        subgrupo: SUB_PROCESSADOR,
        ajuste: PERFBOOSTMODE,
        classe: Classe::Avancada,
        // O Windows documenta 2 = Aggressive, "Always select the highest possible target frequency above nominal" (0
        // Disabled, 1 Enabled, 3 Efficient Enabled, 4 Efficient Aggressive, 5 Aggressive At Guaranteed, 6 Efficient
        // Aggressive At Guaranteed). NÃO desliga proteção térmica: muda o que o Windows PEDE.
        porque: "Substituído pelo motor de energia adaptativo (aba Energia): este número só é certo quando medido nesta máquina, e o plano não o aplica mais sozinho. O Windows chama o valor 2 de \"agressivo\" e o define como sempre escolher a \
                 maior frequência possível acima da nominal. É o que mantém o turbo ligado no \
                 jogo em vez de ele subir e descer. Os limites de temperatura e de potência \
                 do processador continuam valendo por cima disso.",
        alvo: |m| tomada_sempre_bateria_se_desktop(m, 2),
    },
    Ajuste {
        nome: "Economia de energia do PCI Express (ASPM)",
        subgrupo: SUB_PCIEXPRESS,
        ajuste: ASPM,
        classe: Classe::Avancada,
        porque: "Substituído pelo motor de energia adaptativo (aba Energia): este número só é certo quando medido nesta máquina, e o plano não o aplica mais sozinho. O ASPM adormece a via até a placa de vídeo, e acordá-la custa em cada \
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
        classe: Classe::Avancada,
        porque: "Substituído pelo motor de energia adaptativo (aba Energia): este número só é certo quando medido nesta máquina, e o plano não o aplica mais sozinho. É o que faz mouse e teclado perderem o primeiro movimento depois de um tempo \
                 parados. Na bateria continua ligado, porque ali ele economiza de verdade.",
        alvo: |_| Alvo::so_na_tomada(0),
    },
    // INCIDENTE 2.1.0: alvo `1` "preferir desempenho" derrubou FPS (200 → 80-120). O Windows documenta 0 = "No
    // preference", 1 = "Prefer low-power GPU": o jogo saía da placa dedicada. Alvo `0` (o padrão), e não remoção, para
    // quem já recebeu o `1` ser corrigido ao aplicar ou reparar. O significado veio do nome da chave em vez da
    // descrição do Windows. Ver `os_valores_de_cada_ajuste_sao_os_que_o_windows_documenta`.
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

pub fn entra_por_padrao(classe: Classe) -> bool {
    matches!(classe, Classe::Segura | Classe::Recomendada)
}

/// Só o GUID e o nome entre parênteses são estáveis; o resto é traduzido.
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

/// Impede texto traduzido de virar GUID.
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

pub fn achar_na_lista(planos: &[(String, String)], nome: &str) -> Option<String> {
    planos
        .iter()
        .find(|(_, n)| n.eq_ignore_ascii_case(nome))
        .map(|(guid, _)| guid.clone())
}

pub(crate) fn listar_planos() -> Result<Vec<(String, String)>, String> {
    let saida = shell::run_checked("powercfg", &["/list"])?;
    Ok(planos_da_saida(&saida))
}

pub fn escolher_molde(_planos: &[(String, String)]) -> &'static str {
    // SEMPRE o Equilibrado desde o motor adaptativo: o Alto Desempenho traz núcleos todos acordados e mínimo alto, os
    // números universais que o motor recusa.
    EQUILIBRADO_GUID
}

/// Devolve O GUID QUE O WINDOWS GEROU, nunca o do molde: o conserto do defeito principal.
pub(crate) fn criar_plano(molde: &str) -> Result<String, String> {
    let saida = shell::run_checked("powercfg", &["-duplicatescheme", molde])?;

    let novo = power::parse_active_guid(&saida).ok_or_else(|| {
        format!(
            "O Windows criou o plano mas não disse qual é o GUID dele. Resposta: {}",
            saida.trim()
        )
    })?;

    validar_guid_novo(&novo, molde)?;

    // Sem renomear, a execução seguinte não o reencontraria e criaria outro.
    shell::run_checked(
        "powercfg",
        &["-changename", &novo, NOME_DO_PLANO, DESCRICAO_DO_PLANO],
    )
    .map_err(|e| format!("O plano foi criado mas não pôde ser nomeado: {}", e))?;

    Ok(novo)
}

/// Se o `powercfg` devolver o GUID do molde, gravaríamos no plano do cliente.
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

/// "Não consegui ler" não pode virar "não está aplicado", senão a lista oferece aplicar de novo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstadoDoPlano {
    Ativo,
    ExisteEnaoEstaAtivo,
    NaoExiste,
    NaoConsegui,
}

pub fn estado_do_plano(nosso: Option<&str>, ativo: Option<&str>) -> EstadoDoPlano {
    match (nosso, ativo) {
        (_, None) => EstadoDoPlano::NaoConsegui,
        (None, Some(_)) => EstadoDoPlano::NaoExiste,
        (Some(n), Some(a)) if a.eq_ignore_ascii_case(n) => EstadoDoPlano::Ativo,
        (Some(_), Some(_)) => EstadoDoPlano::ExisteEnaoEstaAtivo,
    }
}

/// Dois comandos, sem PowerShell: a lista chama isto uma vez por item a cada atualização.
pub fn onde_esta_o_plano() -> EstadoDoPlano {
    let Ok(planos) = listar_planos() else {
        return EstadoDoPlano::NaoConsegui;
    };

    let nosso = achar_na_lista(&planos, NOME_DO_PLANO);
    let ativo = power::active_scheme().ok();

    estado_do_plano(nosso.as_deref(), ativo.as_deref())
}

/// LEITURA PURA.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "estado")]
pub enum Vistoria {
    NaoExiste,
    Integro,
    /// Instalador de driver, utilitário do fabricante e concorrentes trocam o plano ativo sem avisar.
    DesativadoPorFora,
    Desviado {
        ativo: bool,
        /// O nome do ajuste é o que o cliente consegue conferir.
        ajustes: Vec<String>,
    },
    NaoConsegui,
}

/// Desvio vence "não está ativo": reativar sem reparar devolveria os valores errados com a tela dizendo que está
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

pub fn vistoriar() -> Vistoria {
    // Num plano que existe, `Mudaria` é exatamente "não é o que deixamos".
    let Ok(relatorio) = montar(true) else {
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

/// Caminho próprio: o motor recusa reaplicar o que está no histórico, e o plano é a única otimização que OUTRO
/// programa desfaz pelas costas. Não cria plano (criação tem o registro de desfazer) e não mexe no histórico.
pub fn reparar() -> Result<RelatorioDoPlano, String> {
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

    // O mesmo motor com outra porta: só escreve o que está fora do alvo e relê.
    montar(false)
}

/// Pela árvore de definições, igual em qualquer idioma. Leitura negada conta como SUPORTADO: marcar os onze como
/// inexistentes seria mais forte e mais provável de errar; a escrita é tentada e a releitura decide.
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

/// Três estados: com o processador autônomo o "estado mínimo" é quase decorativo e governa o EPP. Só LÊ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GovernoDoProcessador {
    OProcessador,
    OWindows,
    NaoDeuParaLer,
}

/// Plano próprio não tem padrão declarado: `DefaultPowerSchemeValues` só existe para os três planos internos, e a
/// resposta vinha `None`. Lê o padrão do EQUILIBRADO e diz que foi isso que leu.
pub fn governa_o_processador(plano: &str, bateria: bool) -> GovernoDoProcessador {
    let lido = power::valor_efetivo(plano, SUB_PROCESSADOR, PERFAUTONOMOUS, bateria).or_else(
        || power::valor_efetivo(EQUILIBRADO, SUB_PROCESSADOR, PERFAUTONOMOUS, bateria),
    );

    match lido {
        Some(1) => GovernoDoProcessador::OProcessador,
        Some(0) => GovernoDoProcessador::OWindows,
        _ => GovernoDoProcessador::NaoDeuParaLer,
    }
}

/// Regra de produto, com teste: a tela recebe o ESTADO.
pub fn explicar_governo(governo: GovernoDoProcessador) -> &'static str {
    match governo {
        GovernoDoProcessador::OProcessador => {
            "Neste computador é o próprio processador que escolhe a frequência — é assim \
             que funcionam os Intel com Speed Shift e os AMD com CPPC. Por isso o ajuste \
             que mais pesa aqui é a preferência entre energia e desempenho (EPP), e não o \
             estado mínimo do processador, que quase não muda nada nessas máquinas."
        }
        GovernoDoProcessador::OWindows => {
            "Neste computador quem escolhe a frequência é o Windows. Aqui o estado mínimo \
             do processador governa de verdade, e é ele que evita a frequência cair entre \
             um quadro e outro."
        }
        GovernoDoProcessador::NaoDeuParaLer => {
            "Não deu para ler se quem escolhe a frequência é o processador ou o Windows. \
             Os dois ajustes são aplicados mesmo assim: qual dos dois pesa mais depende \
             dessa resposta, e nenhum dos dois faz mal no outro caso."
        }
    }
}

/// Aqui mora "não confie no código de saída", testável sem máquina.
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

pub fn ja_satisfeito(alvo: &Alvo, ac: Option<u32>, dc: Option<u32>) -> bool {
    alvo.mexe_em_alguma_coisa()
        && alvo.ac.map_or(true, |q| ac == Some(q))
        && alvo.dc.map_or(true, |q| dc == Some(q))
}

/// Sem o comando exato, quem lê o log não consegue REPETIR à mão o que o produto fez.
#[derive(Debug, Clone)]
pub struct Execucao {
    pub comando: String,
    pub codigo: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub erro: Option<String>,
}

impl Execucao {
    /// Saída vazia não vira `stdout= stderr=`: o `powercfg` que dá certo não escreve nada.
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

    // Montado dos MESMOS argumentos que rodam: um log com comando diferente do que rodou é pior que nenhum.
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
        Err(e) => Execucao {
            comando,
            codigo: None,
            stdout: String::new(),
            stderr: String::new(),
            erro: Some(e),
        },
    }
}

/// A linha que o cliente manda: existe no Windows? antes, pedido, depois, e quanto levou ("falhou" ou "travou").
/// `tomada 5->100 ficou 5` mostra que o `powercfg` aceitou e o Windows não obedeceu. Pura, para o formato não mudar
/// por acidente.
pub fn linha_do_log(r: &ResultadoDoAjuste, duracao: std::time::Duration) -> String {
    let n = |v: Option<u32>| match v {
        Some(v) => v.to_string(),
        None => "-".to_string(),
    };

    // Espaço em volta da seta: ausente sai `-`, e `-->-` é ilegível.
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

    // A mensagem do Windows INTEIRA e sem tradução: traduzida, não se pesquisa.
    if !r.mensagem.is_empty() {
        linha.push_str(" | ");
        linha.push_str(&r.mensagem);
    }

    // Por último: quem lê procura primeiro o estado.
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

    // Recusado na tomada, repetir na bateria só enche o log.
    if erro.is_none() {
        if let Some(v) = alvo.dc {
            let execucao = escrever("-setdcvalueindex", plano, a.subgrupo, a.ajuste, v);
            erro = execucao.erro.clone();
            resultado.execucoes.push(execucao);
        }
    }

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

pub fn contar(ajustes: &[ResultadoDoAjuste]) -> (usize, usize, usize, usize) {
    let conta = |alvo: StatusDoAjuste| ajustes.iter().filter(|r| r.status == alvo).count();

    (
        conta(StatusDoAjuste::Aplicado),
        conta(StatusDoAjuste::JaEstavaBom),
        conta(StatusDoAjuste::NaoSuportado),
        conta(StatusDoAjuste::FalhouAoAplicar) + conta(StatusDoAjuste::FalhouNaVerificacao),
    )
}

/// Trinta bons e dois inexistentes naquele Windows NÃO é falha.
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

/// Em `simulacao` nada é escrito. UM DONO POR AJUSTE (2.9): o plano aplica só o que vale igual em qualquer máquina
/// (disco, Wi-Fi, multimídia, preferência de placa, teto do processador); mínimo, EPP, estacionamento, boost e
/// ASPM/USB são do motor de energia, que mede antes. Os `Avancada` ficam no relatório, sem caminho de escrita.
pub fn montar(simulacao: bool) -> Result<RelatorioDoPlano, String> {
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
        // Em simulação sem plano nosso, o "antes" sai do plano ATIVO.
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
        if !entra_por_padrao(a.classe) {
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

        // O relógio começa ANTES da leitura: um `powercfg` trava em qualquer um dos dois.
        let relogio = std::time::Instant::now();
        let resultado = aplicar_um(&guid, a, &maquina, simulacao);

        crate::utils::Logger::info(&linha_do_log(&resultado, relogio.elapsed()));

        ajustes.push(resultado);
    }

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

/// UMA operação, e não trinta escritas de volta: nada foi escrito no plano do cliente.
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

    // Só com o anterior ATIVO E CONFERIDO: plano ativo não se apaga, e antes arriscaria deixar a máquina sem plano.
    if let Some(nosso) = achar_na_lista(&listar_planos()?, NOME_DO_PLANO) {
        if let Err(e) = shell::run_checked("powercfg", &["-delete", &nosso]) {
            crate::utils::Logger::warn(&format!("plano OTIMIZA não foi apagado: {}", e));
        }
    }

    Ok(())
}

/// Nada escreve: responde "por que não funcionou no seu PC?" com causas conferidas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostico {
    pub maquina: Maquina,
    /// O PROCESSO elevado, não o usuário no grupo Administradores.
    pub elevado: bool,
    pub powercfg_responde: bool,
    pub planos_legiveis: bool,
    pub planos: Vec<(String, String)>,
    pub plano_otimiza_existe: bool,
    /// Sem ela todo ajuste apareceria como "não suportado", e seria mentira.
    pub registro_de_energia_legivel: bool,
    pub ajustes_suportados: usize,
    pub ajustes_totais: usize,
    pub ajustes_ausentes: Vec<String>,
    /// Num processo de 32 bits o registro cai no `WOW6432Node` e sai errado.
    pub processo_64_bits: bool,
    pub governo_do_processador: GovernoDoProcessador,
    pub explicacao_do_governo: String,
    pub avisos: Vec<String>,
}

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

    // `None` (leitura negada) é exatamente o caso que este campo denuncia.
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

    // No plano ATIVO e na tomada: é onde o cliente joga.
    let governo = power::active_scheme()
        .map(|plano| governa_o_processador(&plano, false))
        .unwrap_or(GovernoDoProcessador::NaoDeuParaLer);

    // Montada ANTES de `maquina` ir para a struct: precisa do nome do processador. A leitura decide; a geração só
    // explica.
    let explicacao_do_governo = match super::cpugeracao::o_que_governa(
        governo,
        maquina.fabricante_da_cpu,
        &maquina.cpu,
    ) {
        super::cpugeracao::OQueGoverna::OEpp { porque }
        | super::cpugeracao::OQueGoverna::OEstadoMinimo { porque }
        | super::cpugeracao::OQueGoverna::NaoDeuParaSaber { porque } => porque,
    };

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
        governo_do_processador: governo,
        explicacao_do_governo,
        avisos,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Contra o Windows desta máquina: confere que os GUIDs novos existem aqui (o passo pulado na 2.1.0).

    /// Equilibrado EPP na tomada = 33, Alto desempenho = 0: plano feito do Equilibrado nasce com um terço puxado para
    /// economia.

    #[test]
    #[ignore = "toca o Windows desta máquina"]
    fn a_explicacao_do_governo_desta_maquina() {
        let d = diagnosticar();

        println!("cpu: {}", d.maquina.cpu);
        println!("governo: {:?}", d.governo_do_processador);
        println!("{}", d.explicacao_do_governo);

        assert!(
            !d.explicacao_do_governo.is_empty(),
            "o diagnóstico precisa explicar quem governa o processador"
        );
    }

    #[test]
    #[ignore = "toca o Windows desta máquina"]
    fn o_equilibrado_nasce_puxado_para_economia() {
        let equilibrado = power::valor_efetivo(EQUILIBRADO, SUB_PROCESSADOR, PERFEPP, false);
        let alto = power::valor_efetivo(
            "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c",
            SUB_PROCESSADOR,
            PERFEPP,
            false,
        );

        println!("EPP padrão — Equilibrado: {equilibrado:?} · Alto desempenho: {alto:?}");

        let (Some(equilibrado), Some(alto)) = (equilibrado, alto) else {
            println!("um dos dois planos não declara padrão de EPP neste Windows");
            return;
        };

        assert!(
            equilibrado > alto,
            "o Equilibrado deixou de ser mais econômico que o Alto desempenho; \
             se isso mudou, a justificativa do ajuste de EPP precisa ser reescrita"
        );
    }

    #[test]
    #[ignore = "toca o Windows desta máquina"]
    fn quem_governa_o_processador_desta_maquina() {
        let plano = power::active_scheme().expect("plano ativo");

        println!("plano ativo: {plano}");
        println!(
            "modo autônomo (tomada): {:?}",
            power::valor_efetivo(&plano, SUB_PROCESSADOR, PERFAUTONOMOUS, false)
        );
        println!(
            "EPP (tomada): {:?}",
            power::valor_efetivo(&plano, SUB_PROCESSADOR, PERFEPP, false)
        );
        println!(
            "estado mínimo (tomada): {:?}",
            power::valor_efetivo(&plano, SUB_PROCESSADOR, PROCTHROTTLEMIN, false)
        );

        let governo = governa_o_processador(&plano, false);
        println!("veredito: {governo:?}");
        println!("{}", explicar_governo(governo));

        assert!(suportado(SUB_PROCESSADOR, PERFEPP), "o EPP não existe neste Windows");
        assert!(
            suportado(SUB_PROCESSADOR, PERFAUTONOMOUS),
            "o modo autônomo não existe neste Windows"
        );
    }

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
        // "(* Ativos)" tem parênteses e nenhum GUID.
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
        // O caso do cliente: notebook com Modern Standby, sem Alto Desempenho.
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
    fn nem_com_alto_desempenho_ele_vira_molde() {
        let planos = vec![(
            power::HIGH_PERFORMANCE_GUID.to_string(),
            "Alto desempenho".to_string(),
        )];

        assert_eq!(escolher_molde(&planos), EQUILIBRADO_GUID);
    }

    #[test]
    fn o_guid_do_molde_devolvido_como_novo_e_recusado() {
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
        // Uma opção inexistente derrubava a otimização inteira e desfazia o que já tinha dado certo.
        let alvo = Alvo::nos_dois(100);
        assert_eq!(
            classificar(false, &alvo, None, None, None),
            StatusDoAjuste::NaoSuportado
        );
    }

    #[test]
    fn comando_aceito_com_valor_errado_nao_e_sucesso() {
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
        assert!(ja_satisfeito(&Alvo::nos_dois(100), Some(100), Some(100)));
        assert!(!ja_satisfeito(&Alvo::nos_dois(100), Some(100), Some(5)));
        assert!(ja_satisfeito(&Alvo::so_na_tomada(0), Some(0), Some(2)));
        assert!(!ja_satisfeito(&Alvo::nenhum(), None, None));
    }

    #[test]
    fn bateria_herdando_o_padrao_do_windows_nao_conta_como_aplicada() {
        // Medido: só a tomada era gravada e a bateria herdava 5%; num notebook fora da tomada nada mudava e a lista
        // dizia aplicada. No desktop os dois lados são alvo.
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
        let a = AJUSTES.iter().find(|a| a.ajuste == PROCTHROTTLEMIN).unwrap();

        assert_eq!((a.alvo)(&maquina_de_teste(true)), Alvo::so_na_tomada(100));
        assert_eq!((a.alvo)(&maquina_de_teste(false)), Alvo::nos_dois(100));
    }

    #[test]
    fn desktop_leva_o_valor_nos_dois_modos() {
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
        // Trava de escopo: nada aqui mexe em limite térmico nem em política de resfriamento.
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
        // O conserto é o estado separado, não uma frase diferente no TypeScript.
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
        // O registro é lido linha a linha.
        assert!(r.contains("especificada não existe."), "{}", r);
        assert!(!r.contains('\n'), "{}", r);
    }

    #[test]
    fn processo_sem_codigo_e_dito_e_nao_virado_zero() {
        // Encerrado por nós: "saiu 0" afirmaria que deu certo.
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
        // Sem o "ficou", não se separa "recusou" de "aceitou e não obedeceu".
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
        // "0" é valor válido (ASPM desligado): ausência não pode sair como zero.
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
        // Aqui basta reativar.
        assert_eq!(
            classificar_vistoria(true, false, vec![]),
            Vistoria::DesativadoPorFora
        );
        assert_eq!(classificar_vistoria(true, true, vec![]), Vistoria::Integro);
    }

    #[test]
    fn sem_plano_nao_ha_o_que_vistoriar() {
        assert_eq!(classificar_vistoria(false, false, vec![]), Vistoria::NaoExiste);
        // Sem plano nosso, os valores são do cliente: chamá-los de desvio acusaria o dono de mexer no que é dele.
        assert_eq!(
            classificar_vistoria(false, false, vec!["qualquer".into()]),
            Vistoria::NaoExiste
        );
    }

    #[test]
    fn reparar_sem_plano_recusa_em_vez_de_criar() {
        // Reparo que criasse deixaria um plano ativo SEM linha no histórico, sem botão de voltar.
        if registry::is_elevated() && onde_esta_o_plano() == EstadoDoPlano::NaoExiste {
            let erro = reparar().unwrap_err();
            assert!(erro.contains("Criar e ativar"), "{}", erro);
        }
    }

    #[test]
    fn nao_conseguir_ler_nao_e_plano_ausente() {
        assert_eq!(estado_do_plano(Some("a"), None), EstadoDoPlano::NaoConsegui);
        assert_eq!(estado_do_plano(None, None), EstadoDoPlano::NaoConsegui);
    }

    #[test]
    fn o_plano_so_esta_aplicado_quando_e_o_ativo() {
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
        let avisos = avisos_do_diagnostico(true, true, true, 2, true);
        assert_eq!(avisos.len(), 1);
        assert!(avisos[0].contains("não é falha"));
    }

    /// Diagnóstico e SIMULAÇÃO contra a máquina real, sem escrever.
    /// `cargo test --lib planoenergia -- --ignored --nocapture`
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

        match montar(true) {
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

    /// Cria, configura, ativa, relê, RODA DE NOVO (idempotência) e desfaz. Escreve na máquina e precisa de
    /// administrador. `cargo test --lib planoenergia -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn cria_configura_ativa_confere_e_desfaz() {
        if !registry::is_elevated() {
            println!("PULADO: precisa de administrador.");
            return;
        }

        let antes = power::active_scheme().expect("plano ativo");
        println!("\nplano ativo antes: {}", antes);

        let primeira = montar(false).expect("primeira passada");

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

        let segunda = montar(false).expect("segunda passada");

        println!(
            "2ª passada: existia={} guid={:?} aplicados={} já bons={}",
            segunda.plano_existia, segunda.guid_do_plano, segunda.aplicados, segunda.ja_estavam_bons
        );

        assert!(segunda.plano_existia, "a segunda passada não reencontrou o plano");
        assert_eq!(segunda.guid_do_plano, primeira.guid_do_plano, "criou um plano duplicado");
        assert_eq!(segunda.aplicados, 0, "a segunda passada reescreveu o que já estava bom");

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

    /// Estraga um ajuste por fora, confere que a vistoria o nomeia, repara e desfaz. Escreve e precisa de
    /// administrador. `cargo test --lib planoenergia -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn vistoria_acha_o_desvio_e_o_reparo_conserta() {
        if !registry::is_elevated() {
            println!("PULADO: precisa de administrador.");
            return;
        }

        let antes = power::active_scheme().expect("plano ativo");
        let inicial = montar(false).expect("criar o plano");

        assert!(inicial.plano_ativo, "o plano não ficou ativo");
        assert_eq!(vistoriar(), Vistoria::Integro, "nasceu desviado");

        let guid = inicial.guid_do_plano.clone().unwrap();

        // Estacionamento de 100 para 50: valor válido e diferente do que deixamos, como faria um concorrente.
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

        let reparo = reparar().expect("reparar");

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

    // A 2.1.0 gravou 1 ("Prefer low-power GPU") e tirou o jogo da placa dedicada. As três travas abaixo impedem a
    // volta e corrigem o PC já atingido.

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
        // Quem já aplicou a 2.1.0 tem o 1 gravado: o alvo precisa ser 0 nos dois modos, notebook e desktop.
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
        // A causa raiz foi deduzir o significado pelo NOME da chave: todo ajuste que grava número explica, em `porque`, o
        // que ele quer dizer para o Windows.
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
