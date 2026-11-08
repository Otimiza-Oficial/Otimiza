// BANDA DO SEGUNDO PLANO. Enquanto o jogo está aberto, o que sincroniza e semeia em segundo plano (OneDrive, Google
// Drive, Dropbox, torrents, lançadores) tem o ENVIO limitado, e o Windows Update baixa devagar. Quando o jogo fecha,
// volta ao que estava. É o "só durante o jogo" de `energia_jogo.rs`: o de antes é LIDO e gravado em disco antes de
// escrever, e volta ao fechar o jogo, na abertura seguinte do Otimiza e no "Restaurar meu PC".
//
// O QUE FUNCIONA DE VERDADE (medido no PC do dono em nov/2026, Windows 11 Pro, administrador):
// - A política QoS do Windows (`New-NetQosPolicy … -ThrottleRateActionBitsPerSecond`) limita só o que o PC ENVIA. A
//   documentação diz isso, e a medição confirmou o lado do download: curl com política de 8 Mb/s baixou a 51 Mb/s
//   (6,4 MB/s), igual a sem política (6,2 MB/s). Download de outro programa NÃO se limita por aqui — nem do navegador,
//   nem da Steam, nem da Epic. A tela não pode prometer isso.
// - O envio saturado é a causa clássica do bufferbloat (a fila do modem enche e o ping do jogo vai junto), e é o
//   envio que torrent semeando e nuvem sincronizando fazem. Por isso o alvo das políticas é quem envia em segundo plano.
// - Com VPN de túnel em kernel (WireGuard/ProtonVPN), NADA disso vale: os pacotes saem pelo driver do túnel, sem o
//   agendador de pacotes do Windows (ms_pacer) e sem dono de programa. Medido: nem uma política para TODO o UDP
//   limitou o túnel (envio 20 Mb/s com teto de 8). Então, antes de criar política, olha-se a placa por onde sai a
//   rota padrão: sem o agendador, as políticas não são criadas e a tela diz por quê (`EfeitoDaRota::SemEfeito`).
// - O Windows Update (Otimização de Entrega) roda dentro de `svchost`, que é protegido e não tem programa próprio para
//   a política. O caminho documentado e reversível é a política `DOMaxBackgroundDownloadBandwidth` (KB/s), em
//   HKLM\SOFTWARE\Policies\Microsoft\Windows\DeliveryOptimization: o próprio serviço segura o download, então vale para
//   a entrada e vale dentro de VPN. Medido: escrita, `Get-DOConfig` passou na hora a `DownBackLimitBps = 1280000`
//   (provedor MdmProvider); apagada, voltou a 0 (DefaultProvider). Só o download em SEGUNDO PLANO: o que a pessoa
//   mandou baixar na Loja não é tocado, e o Update nunca é desligado nem pausado (o `DODownloadMode` não é tocado).
// - A Steam fica de fora de propósito: por padrão ela já pausa download durante o jogo ("Permitir downloads durante o
//   jogo", desmarcado), e o envio dela é o Remote Play e a transmissão — limitar quebraria quem joga junto. Escrever o
//   config da Steam não é seguro: ela regrava o arquivo ao fechar. Navegadores também ficam de fora: voz e
//   transmissão pelo navegador (Discord web) usam o envio deles.
//
// ActiveStore: a política some ao reiniciar o Windows (conferido: `Get-NetQosPolicy -PolicyStore ActiveStore` lista só
// o que foi criado nesta inicialização). É a rede de segurança natural; a anotação em disco cobre o Otimiza cair.
// O Windows devolve o nome em minúsculas ("Otimiza-x" volta "otimiza-x"): o prefixo é comparado sem caixa.

use crate::modules::changelog::PreviousValue;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// O prefixo de toda política do Otimiza. Só o que começa com ele é criado ou removido.
pub const PREFIXO: &str = "Otimiza-";

/// Teto do envio de quem está em segundo plano: 2 Mb/s. Sobra para a nuvem continuar andando e tira a fila do modem
/// na maior parte das conexões de fibra do Brasil (envio de 10 Mb/s ou mais).
pub const TETO_DE_ENVIO_BPS: u64 = 2_000_000;

/// Teto do Windows Update em segundo plano, em KB/s (a unidade da política): 256 KB/s ≈ 2 Mb/s. Continua baixando.
pub const TETO_DO_UPDATE_KBPS: u32 = 256;

/// Download medido acima disto vira aviso (não dá para limitar, mas dá para dizer quem é).
const AVISO_DE_DOWNLOAD_BPS: u64 = 1_000_000;

#[allow(dead_code)] // fronteira: o comando e o relatório da sessão ligam (proposta no commit)
/// Acima da mediana, isto é um pico de ping.
pub const LIMIAR_DE_PICO_MS: f64 = 50.0;

const CHAVE_DO_UPDATE: &str = r"SOFTWARE\Policies\Microsoft\Windows\DeliveryOptimization";
const VALOR_DO_UPDATE: &str = "DOMaxBackgroundDownloadBandwidth";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Categoria {
    /// OneDrive, Google Drive, Dropbox: sobem arquivo sem a pessoa ver.
    Nuvem,
    /// Semeando, o envio vai ao teto da conexão.
    Torrent,
    /// Epic, Battle.net, EA, Ubisoft: salvamento na nuvem e telemetria.
    Lancador,
    /// Steam: não limitada (Remote Play e transmissão usam o envio dela; download ela mesma pausa no jogo).
    Steam,
    /// Navegador: não limitado (voz e transmissão pelo navegador usam o envio).
    Navegador,
}

/// Os programas conhecidos, pelo nome do executável em minúsculas.
const CONHECIDOS: &[(&str, Categoria)] = &[
    ("onedrive.exe", Categoria::Nuvem),
    ("googledrivefs.exe", Categoria::Nuvem),
    ("dropbox.exe", Categoria::Nuvem),
    ("megasync.exe", Categoria::Nuvem),
    ("icloudservices.exe", Categoria::Nuvem),
    ("icloudreminders.exe", Categoria::Nuvem),
    ("qbittorrent.exe", Categoria::Torrent),
    ("utorrent.exe", Categoria::Torrent),
    ("utweb.exe", Categoria::Torrent),
    ("bittorrent.exe", Categoria::Torrent),
    ("transmission-qt.exe", Categoria::Torrent),
    ("deluge.exe", Categoria::Torrent),
    ("epicgameslauncher.exe", Categoria::Lancador),
    ("epicwebhelper.exe", Categoria::Lancador),
    ("battle.net.exe", Categoria::Lancador),
    ("eadesktop.exe", Categoria::Lancador),
    ("eabackgroundservice.exe", Categoria::Lancador),
    ("upc.exe", Categoria::Lancador),
    ("ubisoftconnect.exe", Categoria::Lancador),
    ("steam.exe", Categoria::Steam),
    ("steamwebhelper.exe", Categoria::Steam),
    ("chrome.exe", Categoria::Navegador),
    ("msedge.exe", Categoria::Navegador),
    ("firefox.exe", Categoria::Navegador),
    ("opera.exe", Categoria::Navegador),
    ("brave.exe", Categoria::Navegador),
];

pub fn categoria(exe: &str) -> Option<Categoria> {
    let n = exe.to_lowercase();
    CONHECIDOS.iter().find(|(e, _)| *e == n).map(|(_, c)| *c)
}

fn limita_envio(c: Categoria) -> bool {
    matches!(c, Categoria::Nuvem | Categoria::Torrent | Categoria::Lancador)
}

/// Um processo aberto. O uso de rede é `None` quando não foi medido: o Windows não dá contador de bytes por processo
/// sem rastreamento de eventos, e "não medi" não é "zero".
#[derive(Debug, Clone)]
pub struct Processo {
    pub pid: u32,
    pub nome: String,
    pub caminho: Option<PathBuf>,
    pub enviado_bps: Option<u64>,
    pub recebido_bps: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Alvo {
    /// O nome do executável em minúsculas: é a condição da política (vale para todas as cópias dele).
    pub exe: String,
    pub categoria: Categoria,
    pub enviado_bps: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Aviso {
    pub exe: String,
    pub recebido_bps: u64,
    pub frase: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Decisao {
    pub limitar: Vec<Alvo>,
    /// Baixando muito e fora do alcance do Windows: só se diz quem é.
    pub avisos: Vec<Aviso>,
    /// Conhecidos que ficaram de fora por estarem protegidos (voz, transmissão, anticheat, pasta do jogo).
    pub protegidos: Vec<String>,
}

fn mbps(bps: u64) -> String {
    format!("{:.1}", bps as f64 / 1_000_000.0).replace('.', ",")
}

fn aviso(exe: &str, c: Categoria, recebido: u64) -> Aviso {
    let quanto = mbps(recebido);
    let frase = match c {
        Categoria::Steam => format!(
            "A Steam está baixando {} Mb/s com o jogo aberto. Em Configurações > Downloads, desmarque \"Permitir downloads durante o jogo\".",
            quanto
        ),
        Categoria::Torrent => format!(
            "{} está baixando {} Mb/s. O Windows só limita o envio dele; o download se limita no próprio programa.",
            exe, quanto
        ),
        _ => format!(
            "{} está baixando {} Mb/s. O Windows não limita o download de outro programa; pause-o se o ping pular.",
            exe, quanto
        ),
    };
    Aviso {
        exe: exe.to_string(),
        recebido_bps: recebido,
        frase,
    }
}

/// **Pura.** Quem tem o envio limitado com este jogo aberto. Só programas conhecidos; nunca o jogo, nada da pasta ou
/// do prefixo de nome dele (o FiveM roda em vários processos), nada protegido pelo governador (voz, transmissão,
/// áudio, anticheat, sistema, o Otimiza) e nunca o próprio Otimiza. Um alvo por executável: a política vale pelo nome.
pub fn decidir(
    processos: &[Processo],
    jogo_pid: u32,
    jogo_caminho: Option<&Path>,
    meu_pid: u32,
) -> Decisao {
    let pasta_do_jogo = jogo_caminho.and_then(|c| c.parent());
    let nome_do_jogo = jogo_caminho
        .and_then(|c| c.file_name())
        .map(|s| s.to_string_lossy().to_lowercase());
    let prefixo_do_jogo = jogo_caminho
        .and_then(|c| c.file_stem())
        .map(|s| s.to_string_lossy().to_lowercase())
        .map(|s| s.split(['_', '-', '.']).next().unwrap_or("").to_string())
        .filter(|s| s.len() >= 3);

    let mut d = Decisao::default();
    for p in processos {
        let exe = p.nome.to_lowercase();
        let Some(c) = categoria(&exe) else { continue };
        let do_jogo = p.pid == jogo_pid
            || nome_do_jogo.as_deref() == Some(exe.as_str())
            || matches!((pasta_do_jogo, &p.caminho), (Some(pasta), Some(cam)) if cam.starts_with(pasta))
            || prefixo_do_jogo
                .as_deref()
                .is_some_and(|pre| exe.starts_with(pre));
        if do_jogo || p.pid == meu_pid || crate::modules::windows::governador::protegido(&exe) {
            if !d.protegidos.contains(&exe) {
                d.protegidos.push(exe);
            }
            continue;
        }
        if let Some(r) = p.recebido_bps.filter(|r| *r >= AVISO_DE_DOWNLOAD_BPS) {
            if !d.avisos.iter().any(|a| a.exe == exe) {
                d.avisos.push(aviso(&exe, c, r));
            }
        }
        if limita_envio(c) && !d.limitar.iter().any(|a| a.exe == exe) {
            d.limitar.push(Alvo {
                exe,
                categoria: c,
                enviado_bps: p.enviado_bps,
            });
        }
    }
    // Um executável protegido em uma cópia fica protegido em todas: a política pegaria as duas.
    let protegidos = d.protegidos.clone();
    d.limitar.retain(|a| !protegidos.contains(&a.exe));
    d
}

/// **Pura.** O nome da política de um executável: só letras, números, ponto e hífen (vai entre aspas no PowerShell).
pub fn nome_da_politica(exe: &str) -> String {
    let limpo: String = exe
        .to_lowercase()
        .trim_end_matches(".exe")
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '-' { c } else { '-' })
        .collect();
    format!("{}{}", PREFIXO, limpo)
}

/// **Pura.** A política é do Otimiza? O Windows devolve o nome em minúsculas.
pub fn e_nossa(nome: &str) -> bool {
    nome.to_lowercase().starts_with(&PREFIXO.to_lowercase())
}

/// Uma política que já está no Windows (a leitura vem antes de criar qualquer coisa).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PoliticaExistente {
    pub nome: String,
    #[serde(default)]
    pub app: Option<String>,
}

/// **Pura.** O que criar: um por alvo, menos o que já existe com o mesmo nome e menos o executável que já tem uma
/// política de OUTRO dono (da empresa, do próprio usuário): essa vence e não é sobreposta.
pub fn a_criar(alvos: &[Alvo], existentes: &[PoliticaExistente]) -> Vec<(String, String)> {
    alvos
        .iter()
        .filter(|a| {
            !existentes.iter().any(|p| {
                !e_nossa(&p.nome)
                    && p.app
                        .as_deref()
                        .is_some_and(|app| app.eq_ignore_ascii_case(&a.exe))
            })
        })
        .map(|a| (nome_da_politica(&a.exe), a.exe.clone()))
        .filter(|(nome, _)| !existentes.iter().any(|p| p.nome.eq_ignore_ascii_case(nome)))
        .collect()
}

/// **Pura.** O que remover: toda política com o nosso prefixo que existe de fato — a anotada e qualquer resto de uma
/// versão que caiu antes de anotar. O que não existe fica de fora (remover dá erro em texto traduzido, que não se
/// lê), e o que não é nosso nunca entra.
pub fn a_remover(existentes: &[PoliticaExistente]) -> Vec<String> {
    existentes
        .iter()
        .filter(|p| e_nossa(&p.nome))
        .map(|p| p.nome.clone())
        .collect()
}

/// A placa por onde sai a rota padrão.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Rota {
    pub interface: String,
    pub descricao: String,
    /// O agendador de pacotes do Windows (ms_pacer) está ligado nela?
    pub pacer: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum EfeitoDaRota {
    Vale,
    /// Túnel de VPN em kernel (sem o agendador): a política não pega nada.
    SemEfeito { interface: String, descricao: String },
    /// Não consegui ler a rota: cria assim mesmo (não faz mal) e diz que não conferiu.
    NaoConferido(String),
}

/// **Pura.**
pub fn efeito_da_rota(rota: Result<&Rota, &str>) -> EfeitoDaRota {
    match rota {
        Ok(r) if r.pacer => EfeitoDaRota::Vale,
        Ok(r) => EfeitoDaRota::SemEfeito {
            interface: r.interface.clone(),
            descricao: r.descricao.clone(),
        },
        Err(e) => EfeitoDaRota::NaoConferido(e.to_string()),
    }
}

/// **Pura.** O limite do Windows Update a escrever, lido o de antes. Já mais apertado (ou valor de tipo estranho, que
/// não é nosso para interpretar): nada a escrever.
pub fn limite_do_update(anterior: &PreviousValue) -> Option<u32> {
    match anterior {
        PreviousValue::Absent | PreviousValue::AbsentKey => Some(TETO_DO_UPDATE_KBPS),
        // 0 é "sem limite" na política.
        PreviousValue::Dword(0) => Some(TETO_DO_UPDATE_KBPS),
        PreviousValue::Dword(v) if *v > TETO_DO_UPDATE_KBPS => Some(TETO_DO_UPDATE_KBPS),
        _ => None,
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Prova: picos de ping por minuto, antes × depois. Só os dados; a tela é de quem monta o relatório da sessão.

#[allow(dead_code)] // fronteira: o comando e o relatório da sessão ligam (proposta no commit)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Amostra {
    /// Segundos desde o começo da janela.
    pub t_s: f64,
    /// `None` é pacote perdido.
    pub ms: Option<f64>,
}

#[allow(dead_code)] // fronteira: o comando e o relatório da sessão ligam (proposta no commit)
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResumoDePicos {
    pub minutos: f64,
    pub mediana_ms: f64,
    /// Picos seguidos contam como um: é o "tranco" que a pessoa sente.
    pub picos: u32,
    pub picos_por_minuto: f64,
    pub perdidos: u32,
}

#[allow(dead_code)] // fronteira: o comando e o relatório da sessão ligam (proposta no commit)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Veredito {
    Melhorou,
    Igual,
    Piorou,
    /// Menos de um minuto de cada lado: não se afirma nada.
    PoucosDados,
}

#[allow(dead_code)] // fronteira: o comando e o relatório da sessão ligam (proposta no commit)
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ComparacaoDePicos {
    pub antes: Option<ResumoDePicos>,
    pub depois: Option<ResumoDePicos>,
    pub veredito: Veredito,
    /// Sempre MEDIDO: vem do ping da sessão, não de estimativa.
    pub origem: &'static str,
}

#[allow(dead_code)] // fronteira: o comando e o relatório da sessão ligam (proposta no commit)
fn mediana(v: &mut [f64]) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    let m = v.len() / 2;
    Some(if v.len() % 2 == 0 {
        (v[m - 1] + v[m]) / 2.0
    } else {
        v[m]
    })
}

#[allow(dead_code)] // fronteira: o comando e o relatório da sessão ligam (proposta no commit)
/// **Pura.** Sem nenhuma resposta, não há resumo.
pub fn resumir_picos(amostras: &[Amostra]) -> Option<ResumoDePicos> {
    let mut vivos: Vec<f64> = amostras.iter().filter_map(|a| a.ms).collect();
    let med = mediana(&mut vivos)?;
    let (ini, fim) = amostras.iter().fold((f64::MAX, f64::MIN), |(i, f), a| {
        (i.min(a.t_s), f.max(a.t_s))
    });
    let minutos = ((fim - ini) / 60.0).max(0.0);
    let mut picos = 0u32;
    let mut dentro = false;
    for a in amostras {
        let alto = a.ms.is_some_and(|ms| ms >= med + LIMIAR_DE_PICO_MS);
        if alto && !dentro {
            picos += 1;
        }
        dentro = alto;
    }
    Some(ResumoDePicos {
        minutos,
        mediana_ms: med,
        picos,
        picos_por_minuto: if minutos > 0.0 { picos as f64 / minutos } else { 0.0 },
        perdidos: amostras.iter().filter(|a| a.ms.is_none()).count() as u32,
    })
}

#[allow(dead_code)] // fronteira: o comando e o relatório da sessão ligam (proposta no commit)
/// **Pura.** Melhorou: 30% menos picos por minuto e pelo menos meio pico a menos (de 0,2 para 0,1 não é ganho que
/// se sente). Piorou: o espelho.
pub fn comparar_picos(antes: &[Amostra], depois: &[Amostra]) -> ComparacaoDePicos {
    let a = resumir_picos(antes);
    let d = resumir_picos(depois);
    let veredito = match (&a, &d) {
        (Some(a), Some(d)) if a.minutos >= 1.0 && d.minutos >= 1.0 => {
            let (x, y) = (a.picos_por_minuto, d.picos_por_minuto);
            if y <= x * 0.7 && x - y >= 0.5 {
                Veredito::Melhorou
            } else if y >= x * 1.3 && y - x >= 0.5 {
                Veredito::Piorou
            } else {
                Veredito::Igual
            }
        }
        _ => Veredito::PoucosDados,
    };
    ComparacaoDePicos {
        antes: a,
        depois: d,
        veredito,
        origem: "MEDIDO",
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Estado em disco.

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Aplicado {
    pub jogo: String,
    /// Nomes das políticas criadas (anotados ANTES de criar).
    pub politicas: Vec<String>,
    /// O valor de antes do limite do Windows Update, lido. `None`: não foi escrito.
    pub update_antes: Option<PreviousValue>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Estado {
    /// Vem desligado: exige administrador e muda o comportamento da nuvem; a pessoa liga.
    pub ligado: bool,
    pub aplicado: Option<Aplicado>,
}

fn pasta() -> PathBuf {
    std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("pc-optimizer")
        .join("rede")
}

fn arquivo() -> PathBuf {
    pasta().join("banda_de_fundo.json")
}

pub fn ler() -> Estado {
    std::fs::read_to_string(arquivo())
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn gravar(e: &Estado) -> Result<(), String> {
    std::fs::create_dir_all(pasta()).map_err(|e| e.to_string())?;
    let texto = serde_json::to_string_pretty(e).map_err(|e| e.to_string())?;
    std::fs::write(arquivo(), texto)
        .map_err(|e| format!("Não foi possível gravar o controle de banda: {}", e))
}

// ---------------------------------------------------------------------------------------------------------------
// Resultados com nome.

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
#[cfg_attr(test, serde(rename = "ResultadoDaBanda"))]
#[serde(tag = "tipo")]
pub enum Resultado {
    Aplicado {
        politicas: Vec<String>,
        update_limitado: bool,
        /// O Windows confirmou o limite do Update (`Get-DOConfig`)?
        update_confirmado: Confirmacao,
        avisos: Vec<String>,
        ressalva: Option<String>,
    },
    /// Nada aberto que dê para segurar, e o Update já estava limitado.
    NadaALimitar { avisos: Vec<String> },
    SemAdministrador,
    /// Sem o módulo de QoS ou o Windows recusou criar a política (edição sem o recurso).
    IndisponivelNesteWindows { motivo: String },
    Desligado,
    JaAplicado,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
#[cfg_attr(test, serde(rename = "ConfirmacaoDaBanda"))]
pub enum Confirmacao {
    Confirmado,
    /// Leu-se outro valor: o Windows não está usando o nosso (outra política mais forte, por exemplo).
    Divergente(u64),
    NaoConsegui(String),
    NaoSeAplica,
}

pub const SEM_ADMINISTRADOR: &str =
    "Segurar a banda dos programas de fundo exige abrir o Otimiza como administrador.";

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
#[cfg_attr(test, serde(rename = "PainelDaBanda"))]
pub struct Painel {
    pub ligado: bool,
    pub agindo_em: Option<String>,
    pub politicas: Vec<String>,
    pub update_limitado: bool,
    pub impedimento: Option<String>,
    /// O que faz, sem prometer o que não faz.
    pub o_que_faz: &'static str,
}

pub const O_QUE_FAZ: &str = "Com o jogo aberto, OneDrive, Google Drive, Dropbox, torrents e lançadores (Epic, Battle.net, EA, Ubisoft) enviam no máximo 2 Mb/s, e o Windows Update baixa em segundo plano a até 2 Mb/s. Envio saturado é o que mais faz o ping pular em casa. O Windows não limita o DOWNLOAD de outros programas: quem estiver baixando muito aparece no aviso. Voz, transmissão, navegador, a Steam e o jogo não são tocados. Tudo volta quando o jogo fecha.";

// ---------------------------------------------------------------------------------------------------------------
// A máquina.

#[cfg(windows)]
mod maquina {
    use super::*;
    use crate::modules::windows::{registry, shell};

    pub fn qos_existe() -> Result<bool, String> {
        shell::json_da_saida(
            shell::powershell(
                "ConvertTo-Json -Compress -InputObject ([bool](Get-Command New-NetQosPolicy -ErrorAction SilentlyContinue))",
            ),
            "se o Windows tem políticas de QoS",
        )
    }

    pub fn politicas() -> Result<Vec<PoliticaExistente>, String> {
        shell::json_da_saida(
            shell::powershell(
                "ConvertTo-Json -Compress -InputObject @(Get-NetQosPolicy -PolicyStore ActiveStore -ErrorAction Stop | ForEach-Object { [pscustomobject]@{ nome = $_.Name; app = $_.AppPathName } })",
            ),
            "as políticas de QoS",
        )
    }

    /// Ausência do agendador na placa é a resposta que importa (e não erro): `SilentlyContinue` só ali.
    pub fn rota() -> Result<Rota, String> {
        shell::json_da_saida(
            shell::powershell(
                "$ErrorActionPreference='Stop'; $r = @(Find-NetRoute -RemoteIPAddress 1.1.1.1)[0]; $a = Get-NetAdapter -InterfaceIndex $r.InterfaceIndex; $b = @(Get-NetAdapterBinding -Name $a.Name -ComponentID ms_pacer -ErrorAction SilentlyContinue); [pscustomobject]@{ interface = $a.Name; descricao = $a.InterfaceDescription; pacer = [bool]($b.Count -gt 0 -and $b[0].Enabled) } | ConvertTo-Json -Compress",
            ),
            "a rota padrão",
        )
    }

    fn aspas(s: &str) -> String {
        format!("'{}'", s.replace('\'', "''"))
    }

    pub fn criar(nome: &str, exe: &str) -> Result<(), String> {
        let script = format!(
            "New-NetQosPolicy -Name {} -AppPathNameMatchCondition {} -ThrottleRateActionBitsPerSecond {} -PolicyStore ActiveStore -ErrorAction Stop | Out-Null",
            aspas(nome),
            aspas(exe),
            TETO_DE_ENVIO_BPS
        );
        shell::powershell_checked(&script).map(|_| ())
    }

    pub fn remover(nome: &str) -> Result<(), String> {
        if !e_nossa(nome) {
            return Err(format!("{} não é uma política do Otimiza", nome));
        }
        let script = format!(
            "Remove-NetQosPolicy -Name {} -PolicyStore ActiveStore -Confirm:$false -ErrorAction Stop",
            aspas(nome)
        );
        shell::powershell_checked(&script).map(|_| ())
    }

    pub fn update_antes() -> Result<PreviousValue, String> {
        registry::read("HKLM", CHAVE_DO_UPDATE, VALOR_DO_UPDATE)
    }

    pub fn escrever_update(kbps: u32) -> Result<(), String> {
        registry::set_dword("HKLM", CHAVE_DO_UPDATE, VALOR_DO_UPDATE, kbps).map(|_| ())
    }

    pub fn devolver_update(antes: &PreviousValue) -> Result<(), String> {
        registry::restore("HKLM", CHAVE_DO_UPDATE, VALOR_DO_UPDATE, antes)
    }

    /// O limite que o serviço está usando agora, em bytes por segundo.
    pub fn update_efetivo() -> Result<u64, String> {
        shell::json_da_saida(
            shell::powershell(
                "ConvertTo-Json -Compress -InputObject ([uint64](Get-DOConfig -ErrorAction Stop).DownBackLimitBps)",
            ),
            "o limite do Windows Update",
        )
    }

    pub fn processos() -> Vec<Processo> {
        use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
        let mut s = System::new();
        s.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing().with_exe(sysinfo::UpdateKind::OnlyIfNotSet),
        );
        s.processes()
            .values()
            .map(|p| Processo {
                pid: p.pid().as_u32(),
                nome: p.name().to_string_lossy().to_string(),
                caminho: p.exe().map(PathBuf::from),
                // Sem contador por processo no Windows sem ETW: não medido.
                enviado_bps: None,
                recebido_bps: None,
            })
            .collect()
    }

    pub fn caminho_do_pid(pid: u32) -> Option<PathBuf> {
        use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
        let mut s = System::new();
        let alvo = Pid::from_u32(pid);
        s.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[alvo]),
            true,
            ProcessRefreshKind::nothing().with_exe(sysinfo::UpdateKind::OnlyIfNotSet),
        );
        s.process(alvo).and_then(|p| p.exe()).map(PathBuf::from)
    }
}

#[cfg(windows)]
fn impedimento() -> Option<String> {
    (!crate::modules::windows::registry::is_elevated()).then(|| SEM_ADMINISTRADOR.to_string())
}

#[cfg(windows)]
pub fn painel() -> Painel {
    let e = ler();
    Painel {
        ligado: e.ligado,
        agindo_em: e.aplicado.as_ref().map(|a| a.jogo.clone()),
        politicas: e
            .aplicado
            .as_ref()
            .map(|a| a.politicas.clone())
            .unwrap_or_default(),
        update_limitado: e.aplicado.as_ref().is_some_and(|a| a.update_antes.is_some()),
        impedimento: impedimento(),
        o_que_faz: O_QUE_FAZ,
    }
}

/// Liga ou desliga. Ligado com o jogo já aberto, entra agora (o vigia só age quando o jogo muda).
#[cfg(windows)]
pub fn definir(ligado: bool) -> Result<Resultado, String> {
    if ligado && impedimento().is_some() {
        return Ok(Resultado::SemAdministrador);
    }
    let mut e = ler();
    e.ligado = ligado;
    gravar(&e)?;
    if !ligado {
        devolver()?;
        return Ok(Resultado::Desligado);
    }
    match super::sessao::atual() {
        Some(j) => entrar(&j.nome, j.pid),
        None => Ok(Resultado::NadaALimitar { avisos: Vec::new() }),
    }
}

/// Com o jogo aberto: lê o que existe, anota, e só então cria. Sem administrador ou sem QoS, resultado com nome.
#[cfg(windows)]
pub fn entrar(jogo: &str, jogo_pid: u32) -> Result<Resultado, String> {
    let mut e = ler();
    if !e.ligado {
        return Ok(Resultado::Desligado);
    }
    if e.aplicado.is_some() {
        return Ok(Resultado::JaAplicado);
    }
    if impedimento().is_some() {
        return Ok(Resultado::SemAdministrador);
    }

    let decisao = decidir(
        &maquina::processos(),
        jogo_pid,
        maquina::caminho_do_pid(jogo_pid).as_deref(),
        std::process::id(),
    );
    let avisos: Vec<String> = decisao.avisos.iter().map(|a| a.frase.clone()).collect();

    // Rota pelo túnel: políticas não pegam nada, não se cria.
    let rota = maquina::rota();
    let efeito = efeito_da_rota(rota.as_ref().map_err(|e| e.as_str()));
    let (criar, ressalva) = match &efeito {
        EfeitoDaRota::SemEfeito { interface, descricao } => (
            Vec::new(),
            Some(format!(
                "Sua internet sai por \"{}\" ({}), um túnel de VPN: o Windows não limita programa dentro dele. Só o limite do Windows Update vale.",
                interface, descricao
            )),
        ),
        _ if decisao.limitar.is_empty() => (Vec::new(), None),
        outro => {
            if !maquina::qos_existe()? {
                return Ok(Resultado::IndisponivelNesteWindows {
                    motivo: "Este Windows não tem o módulo de políticas de QoS (NetQos).".into(),
                });
            }
            let existentes = maquina::politicas()?;
            let ressalva = match outro {
                EfeitoDaRota::NaoConferido(erro) => Some(format!(
                    "Não consegui conferir por onde sua internet sai ({}); se for VPN, o limite não pega.",
                    erro
                )),
                _ => None,
            };
            (a_criar(&decisao.limitar, &existentes), ressalva)
        }
    };

    let update_antes = maquina::update_antes()?;
    let update_novo = limite_do_update(&update_antes);

    if criar.is_empty() && update_novo.is_none() {
        return Ok(Resultado::NadaALimitar { avisos });
    }

    // Anotado ANTES de escrever: se cair no meio, a abertura seguinte devolve.
    e.aplicado = Some(Aplicado {
        jogo: jogo.to_string(),
        politicas: criar.iter().map(|(n, _)| n.clone()).collect(),
        update_antes: update_novo.map(|_| update_antes.clone()),
    });
    gravar(&e)?;

    let mut criadas = Vec::new();
    for (nome, exe) in &criar {
        if let Err(erro) = maquina::criar(nome, exe) {
            // A primeira recusada é o teste de que o recurso existe nesta edição: desfaz tudo e diz.
            let _ = devolver();
            return Ok(Resultado::IndisponivelNesteWindows {
                motivo: format!("o Windows recusou criar a política para {}: {}", exe, erro),
            });
        }
        criadas.push(nome.clone());
    }

    let update_confirmado = match update_novo {
        None => Confirmacao::NaoSeAplica,
        Some(kbps) => {
            if let Err(erro) = maquina::escrever_update(kbps) {
                let _ = devolver();
                return Err(format!("limite do Windows Update: {}", erro));
            }
            match maquina::update_efetivo() {
                Ok(bps) if bps == kbps as u64 * 1024 => Confirmacao::Confirmado,
                Ok(bps) => Confirmacao::Divergente(bps),
                Err(erro) => Confirmacao::NaoConsegui(erro),
            }
        }
    };

    Ok(Resultado::Aplicado {
        politicas: criadas,
        update_limitado: update_novo.is_some(),
        update_confirmado,
        avisos,
        ressalva,
    })
}

/// Devolve tudo: remove as políticas com o nosso prefixo (anotadas ou restos) e o limite do Update ao valor lido antes.
/// Chamado quando o jogo fecha, ao desligar, no "Restaurar meu PC" e na abertura do Otimiza.
#[cfg(windows)]
pub fn devolver() -> Result<bool, String> {
    let mut e = ler();
    let anotadas = e
        .aplicado
        .as_ref()
        .map(|a| a.politicas.clone())
        .unwrap_or_default();

    let mut falhas = Vec::new();
    let mut mexeu = false;
    // Sem o módulo, nada foi criado. Não conseguir conferir COM política anotada é falha, não "nada a fazer": a
    // anotação fica para a próxima tentativa.
    let qos = if impedimento().is_some() {
        Err(SEM_ADMINISTRADOR.to_string())
    } else {
        maquina::qos_existe().and_then(|existe| {
            if existe {
                maquina::politicas().map(Some)
            } else {
                Ok(None)
            }
        })
    };
    match qos {
        Ok(Some(existentes)) => {
            for nome in a_remover(&existentes) {
                match maquina::remover(&nome) {
                    Ok(()) => mexeu = true,
                    Err(erro) => falhas.push(format!("{}: {}", nome, erro)),
                }
            }
        }
        Ok(None) => {}
        Err(erro) if !anotadas.is_empty() => falhas.push(erro),
        Err(_) => {}
    }
    if let Some(antes) = e.aplicado.as_ref().and_then(|a| a.update_antes.clone()) {
        match maquina::devolver_update(&antes) {
            Ok(()) => mexeu = true,
            Err(erro) => falhas.push(format!("limite do Windows Update: {}", erro)),
        }
    }
    if !falhas.is_empty() {
        // A anotação fica: a próxima tentativa (fechar o jogo, abrir o Otimiza) volta a tentar.
        return Err(falhas.join("; "));
    }
    if e.aplicado.take().is_some() {
        gravar(&e)?;
    }
    Ok(mexeu)
}

/// Um passo do vigia: entra com o jogo aberto, devolve com ele fechado.
#[cfg(windows)]
pub fn passo(jogo: Option<(&str, u32)>) -> Option<String> {
    let e = ler();
    match (jogo, e.aplicado.is_some()) {
        (Some((nome, pid)), false) if e.ligado => match entrar(nome, pid) {
            Ok(Resultado::Aplicado { politicas, update_limitado, ressalva, .. }) => Some(format!(
                "Banda de fundo segurada para {}: {} programa(s){}{}",
                nome,
                politicas.len(),
                if update_limitado { " e o Windows Update" } else { "" },
                ressalva.map(|r| format!(". {}", r)).unwrap_or_default()
            )),
            Ok(Resultado::IndisponivelNesteWindows { motivo }) => {
                Some(format!("Banda de fundo indisponível neste Windows: {}", motivo))
            }
            Ok(_) => None,
            Err(erro) => Some(format!("Banda de fundo: {}", erro)),
        },
        (None, true) => match devolver() {
            Ok(_) => Some("Banda de fundo: os programas voltaram a enviar sem limite.".to_string()),
            Err(erro) => Some(format!(
                "Banda de fundo: não consegui devolver ({}). Tento de novo.",
                erro
            )),
        },
        _ => None,
    }
}

/// O vigia, numa thread própria: devolve o que ficou de uma sessão que caiu e acompanha o jogo abrir e fechar. Duas
/// olhadas sem jogo antes de devolver (a tela de carregamento troca de processo), como a energia de jogo.
#[cfg(windows)]
pub fn vigiar() {
    let log = |f: &str| crate::utils::Logger::info(f);
    if super::sessao::atual().is_none() && (ler().aplicado.is_some() || ler().ligado) {
        match devolver() {
            Ok(true) => log("banda de fundo, na abertura: devolvida"),
            Ok(false) => {}
            Err(e) => crate::utils::Logger::warn(&format!("banda de fundo, na abertura: {}", e)),
        }
    }
    let mut sem_jogo = 0u32;
    let mut visto: Option<u32> = None;
    loop {
        std::thread::sleep(std::time::Duration::from_secs(4));
        let jogo = super::sessao::atual().map(|j| (j.nome, j.pid));
        let pid = jogo.as_ref().map(|j| j.1);
        sem_jogo = if jogo.is_some() { 0 } else { sem_jogo + 1 };
        if (jogo.is_none() && sem_jogo < 2) || pid == visto {
            continue;
        }
        visto = pid;
        if let Some(f) = passo(jogo.as_ref().map(|(n, p)| (n.as_str(), *p))) {
            log(&f);
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn p(pid: u32, nome: &str, caminho: Option<&str>) -> Processo {
        Processo {
            pid,
            nome: nome.into(),
            caminho: caminho.map(PathBuf::from),
            enviado_bps: None,
            recebido_bps: None,
        }
    }

    const JOGO: &str = r"C:\Jogos\FiveM\FiveM.exe";

    #[test]
    fn nuvem_torrent_e_lancador_tem_o_envio_limitado() {
        let lista = [
            p(10, "OneDrive.exe", None),
            p(11, "qbittorrent.exe", None),
            p(12, "EpicGamesLauncher.exe", None),
            p(13, "notepad.exe", None),
        ];
        let d = decidir(&lista, 1, Some(Path::new(JOGO)), 2);
        let exes: Vec<&str> = d.limitar.iter().map(|a| a.exe.as_str()).collect();
        assert_eq!(exes, ["onedrive.exe", "qbittorrent.exe", "epicgameslauncher.exe"]);
    }

    #[test]
    fn steam_e_navegador_nunca_sao_limitados() {
        let lista = [
            p(10, "steam.exe", None),
            p(11, "steamwebhelper.exe", None),
            p(12, "chrome.exe", None),
            p(13, "msedge.exe", None),
        ];
        assert!(decidir(&lista, 1, None, 2).limitar.is_empty());
    }

    #[test]
    fn voz_transmissao_anticheat_e_sistema_nao_entram() {
        let lista = [
            p(10, "Discord.exe", None),
            p(11, "obs64.exe", None),
            p(12, "svchost.exe", None),
            p(13, "EasyAntiCheat.exe", None),
            p(14, "ts3client_win64.exe", None),
        ];
        assert!(decidir(&lista, 1, None, 2).limitar.is_empty());
    }

    #[test]
    fn nada_da_pasta_do_jogo_nem_o_proprio_otimiza() {
        // Um lançador dentro da pasta do jogo (alguns jogos trazem o da Epic junto) fica de fora, em todas as cópias.
        let lista = [
            p(10, "EpicWebHelper.exe", Some(r"C:\Jogos\FiveM\EpicWebHelper.exe")),
            p(11, "EpicWebHelper.exe", Some(r"C:\Epic\EpicWebHelper.exe")),
            p(2, "OneDrive.exe", None),
        ];
        let d = decidir(&lista, 1, Some(Path::new(JOGO)), 2);
        assert!(d.limitar.is_empty(), "{:?}", d.limitar);
        assert!(d.protegidos.contains(&"epicwebhelper.exe".to_string()));
    }

    #[test]
    fn um_alvo_por_executavel() {
        let lista = [p(10, "OneDrive.exe", None), p(11, "onedrive.exe", None)];
        assert_eq!(decidir(&lista, 1, None, 2).limitar.len(), 1);
    }

    #[test]
    fn download_medido_vira_aviso_e_nao_medido_nao() {
        let mut steam = p(10, "steam.exe", None);
        steam.recebido_bps = Some(40_000_000);
        let mut chrome = p(11, "chrome.exe", None);
        chrome.recebido_bps = None;
        let d = decidir(&[steam, chrome], 1, None, 2);
        assert_eq!(d.avisos.len(), 1);
        assert!(d.avisos[0].frase.contains("40,0 Mb/s"));
        assert!(d.avisos[0].frase.contains("Permitir downloads durante o jogo"));
    }

    #[test]
    fn nome_da_politica_tem_o_prefixo_e_so_caracteres_seguros() {
        assert_eq!(nome_da_politica("OneDrive.exe"), "Otimiza-onedrive");
        assert_eq!(nome_da_politica("battle.net.exe"), "Otimiza-battle.net");
        assert_eq!(nome_da_politica("x'y z.exe"), "Otimiza-x-y-z");
        // O Windows devolve em minúsculas.
        assert!(e_nossa("otimiza-onedrive"));
        assert!(!e_nossa("Politica-da-empresa"));
    }

    fn existente(nome: &str, app: Option<&str>) -> PoliticaExistente {
        PoliticaExistente {
            nome: nome.into(),
            app: app.map(str::to_string),
        }
    }

    fn alvo(exe: &str) -> Alvo {
        Alvo {
            exe: exe.into(),
            categoria: Categoria::Nuvem,
            enviado_bps: None,
        }
    }

    #[test]
    fn politica_de_outro_dono_vence_e_a_nossa_nao_se_repete() {
        let existentes = [
            existente("Empresa-OneDrive", Some("OneDrive.exe")),
            existente("otimiza-dropbox", Some("dropbox.exe")),
        ];
        let criar = a_criar(
            &[alvo("onedrive.exe"), alvo("dropbox.exe"), alvo("megasync.exe")],
            &existentes,
        );
        assert_eq!(criar, [("Otimiza-megasync".to_string(), "megasync.exe".to_string())]);
    }

    #[test]
    fn so_remove_o_que_e_nosso_e_existe() {
        let existentes = [
            existente("otimiza-onedrive", None),
            existente("otimiza-resto-antigo", None),
            existente("Empresa-x", None),
        ];
        assert_eq!(
            a_remover(&existentes),
            ["otimiza-onedrive", "otimiza-resto-antigo"]
        );
    }

    #[test]
    fn tunel_de_vpn_sem_agendador_nao_recebe_politica() {
        let wg = Rota {
            interface: "ProtonVPN".into(),
            descricao: "ProtonVPN WireGuard Tunnel".into(),
            pacer: false,
        };
        assert!(matches!(efeito_da_rota(Ok(&wg)), EfeitoDaRota::SemEfeito { .. }));
        let cabo = Rota { pacer: true, ..wg };
        assert_eq!(efeito_da_rota(Ok(&cabo)), EfeitoDaRota::Vale);
        assert!(matches!(efeito_da_rota(Err("x")), EfeitoDaRota::NaoConferido(_)));
    }

    #[test]
    fn limite_do_update_respeita_o_que_ja_era_mais_apertado() {
        assert_eq!(limite_do_update(&PreviousValue::Absent), Some(TETO_DO_UPDATE_KBPS));
        assert_eq!(limite_do_update(&PreviousValue::AbsentKey), Some(TETO_DO_UPDATE_KBPS));
        assert_eq!(limite_do_update(&PreviousValue::Dword(0)), Some(TETO_DO_UPDATE_KBPS));
        assert_eq!(limite_do_update(&PreviousValue::Dword(5000)), Some(TETO_DO_UPDATE_KBPS));
        assert_eq!(limite_do_update(&PreviousValue::Dword(100)), None);
        assert_eq!(limite_do_update(&PreviousValue::Text("x".into())), None);
    }

    #[test]
    fn o_estado_guardado_volta_inteiro_do_disco() {
        let e = Estado {
            ligado: true,
            aplicado: Some(Aplicado {
                jogo: "GTA V".into(),
                politicas: vec!["Otimiza-onedrive".into()],
                update_antes: Some(PreviousValue::Dword(5000)),
            }),
        };
        let volta: Estado = serde_json::from_str(&serde_json::to_string(&e).unwrap()).unwrap();
        assert_eq!(volta.aplicado, e.aplicado);
    }

    fn janela(minutos: u32, picos_a_cada_s: Option<u32>) -> Vec<Amostra> {
        (0..minutos * 60)
            .map(|s| Amostra {
                t_s: s as f64,
                ms: Some(match picos_a_cada_s {
                    Some(n) if s % n == 0 => 180.0,
                    _ => 30.0,
                }),
            })
            .collect()
    }

    #[test]
    fn picos_seguidos_contam_como_um_e_perda_e_separada() {
        let mut a = janela(1, None);
        a[10].ms = Some(200.0);
        a[11].ms = Some(210.0);
        a[30].ms = None;
        let r = resumir_picos(&a).unwrap();
        assert_eq!(r.picos, 1);
        assert_eq!(r.perdidos, 1);
        assert_eq!(r.mediana_ms, 30.0);
        assert!(resumir_picos(&[Amostra { t_s: 0.0, ms: None }]).is_none());
    }

    #[test]
    fn de_um_pico_por_minuto_a_cada_dez_segundos_para_nenhum_melhorou() {
        let c = comparar_picos(&janela(3, Some(10)), &janela(3, None));
        assert_eq!(c.veredito, Veredito::Melhorou);
        assert!((c.antes.unwrap().picos_por_minuto - 6.0).abs() < 0.2);
        assert_eq!(c.origem, "MEDIDO");
        assert_eq!(
            comparar_picos(&janela(3, None), &janela(3, Some(10))).veredito,
            Veredito::Piorou
        );
        assert_eq!(comparar_picos(&janela(3, None), &janela(3, None)).veredito, Veredito::Igual);
    }

    #[test]
    fn menos_de_um_minuto_nao_afirma_nada() {
        let curta: Vec<Amostra> = janela(1, Some(10)).into_iter().take(30).collect();
        assert_eq!(
            comparar_picos(&curta, &janela(3, None)).veredito,
            Veredito::PoucosDados
        );
    }
}

#[cfg(all(test, windows))]
mod na_maquina {
    use super::maquina;

    /// Só lê: `cargo test --lib -- --ignored banda::na_maquina::le`.
    #[test]
    #[ignore = "lê esta máquina"]
    fn le_rota_politicas_e_update() {
        println!("QoS existe: {:?}", maquina::qos_existe());
        println!("rota: {:?}", maquina::rota());
        println!("políticas: {:?}", maquina::politicas());
        println!("Update antes: {:?}", maquina::update_antes());
        println!("Update efetivo: {:?}", maquina::update_efetivo());
        let d = super::decidir(&maquina::processos(), 0, None, std::process::id());
        println!("decisão: {:#?}", d);
    }

    /// ESCREVE (administrador): cria uma política do Otimiza para um executável que não existe e a remove, e põe e
    /// tira o limite do Update, conferindo que tudo voltou ao lido antes.
    /// `cargo test --lib -- --ignored banda::na_maquina::cria_e_remove`.
    #[test]
    #[ignore = "escreve nesta máquina (administrador)"]
    fn cria_e_remove() {
        let nome = super::nome_da_politica("otimiza-teste-inexistente.exe");
        maquina::criar(&nome, "otimiza-teste-inexistente.exe").expect("criar");
        let depois = maquina::politicas().expect("ler");
        assert!(depois.iter().any(|p| p.nome.eq_ignore_ascii_case(&nome)), "{:?}", depois);
        maquina::remover(&nome).expect("remover");
        assert!(!maquina::politicas()
            .expect("ler")
            .iter()
            .any(|p| p.nome.eq_ignore_ascii_case(&nome)));

        let antes = maquina::update_antes().expect("ler o Update");
        let alvo = super::limite_do_update(&antes).expect("já havia limite mais apertado");
        maquina::escrever_update(alvo).expect("escrever");
        assert_eq!(maquina::update_efetivo().expect("Get-DOConfig"), alvo as u64 * 1024);
        maquina::devolver_update(&antes).expect("devolver");
        assert_eq!(maquina::update_antes().expect("reler"), antes);
    }
}
