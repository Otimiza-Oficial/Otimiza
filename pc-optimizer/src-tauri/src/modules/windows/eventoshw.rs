// Quedas do driver de vídeo e erros de hardware que o Windows registrou (3.0)
//
// Duas evidências que o cliente confere sozinho no Visualizador de Eventos:
//
// - Display, evento 4101: o driver de vídeo parou de responder e o Windows o
//   reiniciou. É a tela preta de dois segundos no meio da partida.
// - WHEA-Logger: erro de hardware. Como erro (nível 1 ou 2), é o processador,
//   a memória ou o PCIe falhando de verdade, quase sempre por overclock, XMP
//   ou undervolt instável. Como aviso (nível 3), o hardware corrigiu sozinho.
//
// Como em `exhaustion.rs`, nada aqui lê a mensagem traduzida do evento: só o
// provedor, o número, o nível e a hora, que são iguais em qualquer idioma.

use super::achados::{FindingSeverity, FixLocation};
use serde::Deserialize;

const DIAS: u64 = 30;
const MS_POR_DIA: u64 = 86_400_000;

/// O teto da consulta. Atingido, a contagem é "pelo menos", e não o número real.
pub const LIMITE_DE_EVENTOS: usize = 2000;

/// Acima disto, erros corrigidos deixam de ser ruído.
const CORRIGIDOS_DEMAIS: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Evento {
    pub quando: String,
    pub provedor: String,
    pub id: u32,
    /// 1 crítico, 2 erro, 3 aviso, 4 informação.
    pub nivel: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tipo {
    VideoReiniciou,
    HardwareFalhou,
    HardwareCorrigiu,
}

/// **Função pura.**
pub fn tipo(e: &Evento) -> Option<Tipo> {
    match (e.provedor.as_str(), e.id) {
        ("Display", 4101) => Some(Tipo::VideoReiniciou),
        ("Microsoft-Windows-WHEA-Logger", _) if e.nivel <= 2 => Some(Tipo::HardwareFalhou),
        ("Microsoft-Windows-WHEA-Logger", _) => Some(Tipo::HardwareCorrigiu),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Achado {
    pub id: &'static str,
    pub titulo: String,
    pub medido: String,
    pub conselho: String,
    pub severidade: FindingSeverity,
    pub onde: FixLocation,
}

/// "2026-09-24T21:40:12" → "24/09 às 21:40". **Função pura.**
fn quando_legivel(s: &str) -> String {
    match (s.get(8..10), s.get(5..7), s.get(11..16)) {
        (Some(d), Some(m), Some(h)) => format!("{}/{} às {}", d, m, h),
        _ => s.to_string(),
    }
}

/// Os achados, do mais recente para trás. **Função pura.**
pub fn achados(eventos: &[Evento]) -> Vec<Achado> {
    let mut ordenados: Vec<&Evento> = eventos.iter().collect();
    ordenados.sort_by(|a, b| b.quando.cmp(&a.quando));
    let pelo_menos = if eventos.len() >= LIMITE_DE_EVENTOS { "pelo menos " } else { "" };
    let de = |t: Tipo| ordenados.iter().filter(|e| tipo(e) == Some(t.clone())).copied().collect::<Vec<_>>();

    let mut v = Vec::new();

    let video = de(Tipo::VideoReiniciou);
    if let Some(ultimo) = video.first() {
        v.push(Achado {
            id: "driver_de_video_reiniciou",
            titulo: "O driver de vídeo caiu e foi reiniciado".to_string(),
            medido: format!(
                "{}{} vez(es) nos últimos {} dias; a última em {}.",
                pelo_menos,
                video.len(),
                DIAS,
                quando_legivel(&ultimo.quando)
            ),
            conselho: "É a tela preta de dois segundos no meio do jogo. As causas mais comuns: driver com \
                       defeito (reinstale o do fabricante do zero), overclock ou undervolt da placa, \
                       temperatura e fonte. Confira no Visualizador de Eventos: Sistema, origem Display, \
                       evento 4101."
                .to_string(),
            severidade: FindingSeverity::Important,
            onde: FixLocation::Software,
        });
    }

    let falhas = de(Tipo::HardwareFalhou);
    if let Some(ultimo) = falhas.first() {
        v.push(Achado {
            id: "erro_de_hardware_whea",
            titulo: "O Windows registrou erro de hardware".to_string(),
            medido: format!(
                "{}{} erro(s) do WHEA nos últimos {} dias; o último em {}.",
                pelo_menos,
                falhas.len(),
                DIAS,
                quando_legivel(&ultimo.quando)
            ),
            conselho: "Erro de hardware assim costuma ser processador ou memória instável: overclock, perfil \
                       XMP/EXPO que a máquina não segura, ou undervolt. Volte a BIOS ao padrão e veja se \
                       o erro some. Se continuar, é peça com defeito. Visualizador de Eventos: Sistema, \
                       origem WHEA-Logger."
                .to_string(),
            severidade: FindingSeverity::Critical,
            onde: FixLocation::Bios,
        });
    }

    let corrigidos = de(Tipo::HardwareCorrigiu);
    if corrigidos.len() >= CORRIGIDOS_DEMAIS {
        v.push(Achado {
            id: "erros_corrigidos_whea",
            titulo: "Muitos erros de hardware corrigidos".to_string(),
            medido: format!("{}{} nos últimos {} dias.", pelo_menos, corrigidos.len(), DIAS),
            conselho: "O hardware corrigiu sozinho, e nada travou por isso. Tantos assim costumam vir de \
                       memória ou processador no limite (XMP, overclock) ou do PCIe: vale conferir antes \
                       que vire erro de verdade."
                .to_string(),
            severidade: FindingSeverity::Important,
            onde: FixLocation::Bios,
        });
    }

    v
}

/// Lê os eventos dos últimos 30 dias. Nenhum evento é boa notícia, e não erro.
#[cfg(windows)]
pub fn ler() -> Result<Vec<Evento>, String> {
    let script = format!(
        "try {{ $e = Get-WinEvent -LogName System -FilterXPath \
           \"*[System[(Provider[@Name='Display'] and EventID=4101) or Provider[@Name='Microsoft-Windows-WHEA-Logger']] \
             and System[TimeCreated[timediff(@SystemTime) <= {}]]]\" \
           -MaxEvents {} -ErrorAction Stop }} catch {{ if ($_.FullyQualifiedErrorId -like 'NoMatchingEventsFound*') {{ $e = @() }} else {{ throw }} }}; \
         ConvertTo-Json -Compress -InputObject @($e | ForEach-Object {{ [ordered]@{{ \
           quando = $_.TimeCreated.ToString('s'); provedor = $_.ProviderName; id = [int]$_.Id; nivel = [int]$_.Level }} }})",
        DIAS * MS_POR_DIA,
        LIMITE_DE_EVENTOS
    );
    let saida = super::shell::powershell(&script).map_err(|e| format!("Não foi possível ler o registro de eventos: {}", e))?;
    if !saida.success {
        return Err("O registro de eventos do Windows não pôde ser lido nesta máquina.".to_string());
    }
    if saida.stdout.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(saida.stdout.trim()).map_err(|e| format!("Registro de eventos em formato inesperado: {}", e))
}

#[cfg(test)]
mod testes {
    use super::*;

    fn ev(quando: &str, provedor: &str, id: u32, nivel: u32) -> Evento {
        Evento { quando: quando.into(), provedor: provedor.into(), id, nivel }
    }

    #[test]
    fn classifica_pelo_provedor_numero_e_nivel() {
        assert_eq!(tipo(&ev("", "Display", 4101, 3)), Some(Tipo::VideoReiniciou));
        assert_eq!(tipo(&ev("", "Display", 4100, 3)), None);
        assert_eq!(tipo(&ev("", "Microsoft-Windows-WHEA-Logger", 18, 2)), Some(Tipo::HardwareFalhou));
        assert_eq!(tipo(&ev("", "Microsoft-Windows-WHEA-Logger", 19, 3)), Some(Tipo::HardwareCorrigiu));
        assert_eq!(tipo(&ev("", "Kernel-Power", 41, 1)), None);
    }

    #[test]
    fn video_diz_quantas_vezes_e_a_ultima() {
        let a = achados(&[
            ev("2026-09-20T10:00:00", "Display", 4101, 3),
            ev("2026-09-24T21:40:12", "Display", 4101, 3),
        ]);
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].id, "driver_de_video_reiniciou");
        assert!(a[0].medido.starts_with("2 vez(es)"), "{}", a[0].medido);
        assert!(a[0].medido.contains("24/09 às 21:40"), "{}", a[0].medido);
    }

    #[test]
    fn whea_fatal_e_critico_e_poucos_corrigidos_sao_ruido() {
        let mut eventos = vec![ev("2026-09-10T08:00:00", "Microsoft-Windows-WHEA-Logger", 18, 2)];
        eventos.extend((0..3).map(|i| ev(&format!("2026-09-1{}T00:00:00", i), "Microsoft-Windows-WHEA-Logger", 19, 3)));
        let a = achados(&eventos);
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].severidade, FindingSeverity::Critical);

        let muitos: Vec<Evento> = (0..10).map(|_| ev("2026-09-11T00:00:00", "Microsoft-Windows-WHEA-Logger", 19, 3)).collect();
        assert_eq!(achados(&muitos)[0].id, "erros_corrigidos_whea");
    }

    #[test]
    fn sem_evento_nao_ha_achado() {
        assert!(achados(&[]).is_empty());
    }
}
