// Ryzen X3D com dois blocos de núcleos (3.0)
//
// No 7900X3D, 7950X3D, 9900X3D e 9950X3D só um dos dois blocos (CCD) tem o
// cache 3D. Quem manda o jogo para ele é o serviço "3D V-Cache Performance
// Optimizer" do driver de chipset da AMD, junto da Game Bar do Windows, que
// avisa que um jogo abriu. A AMD recomenda o plano Equilibrado: o Alto
// Desempenho desfaz o estacionamento que prende o jogo no bloco certo.
//
// Faltando um desses, o jogo se espalha pelos dois blocos e perde o cache,
// sem aviso nenhum. Este módulo só lê e diz o que falta; não instala nem troca
// plano.

use super::achados::{FindingSeverity, FixLocation};

/// O plano Equilibrado do Windows.
pub const EQUILIBRADO: &str = "381b4222-f694-41f0-9685-ff5bb260df2e";

/// Dois blocos, um com cache 3D. **Função pura.**
pub fn e_x3d_de_dois_blocos(cpu: &str) -> bool {
    let n = cpu.to_lowercase();
    ["7900x3d", "7950x3d", "9900x3d", "9950x3d"].iter().any(|m| n.contains(m))
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Leitura {
    /// O serviço da AMD existe e está rodando. `None`: não deu para ler.
    pub servico_rodando: Option<bool>,
    pub game_bar_instalada: Option<bool>,
    /// GUID do plano ativo, em minúsculas.
    pub plano_ativo: Option<String>,
    /// O plano ativo é o OTIMIZA (cópia do Equilibrado com ajustes).
    pub plano_do_otimiza: bool,
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

/// **Função pura.**
pub fn achados(cpu: &str, l: &Leitura) -> Vec<Achado> {
    let mut faltas = Vec::new();
    if l.servico_rodando == Some(false) {
        faltas.push("o serviço 3D V-Cache Performance Optimizer da AMD não está rodando");
    }
    if l.game_bar_instalada == Some(false) {
        faltas.push("a Xbox Game Bar não está instalada");
    }
    let plano_outro = l.plano_ativo.as_deref().is_some_and(|g| g != EQUILIBRADO);
    if plano_outro && l.plano_do_otimiza {
        faltas.push("o plano ativo é o OTIMIZA, uma cópia do Equilibrado com ajustes, e a AMD recomenda o Equilibrado original");
    } else if plano_outro {
        faltas.push("o plano de energia ativo não é o Equilibrado do Windows");
    }
    if faltas.is_empty() {
        return Vec::new();
    }
    vec![Achado {
        id: "x3d_bloco_errado",
        titulo: "O jogo pode estar rodando no bloco de núcleos sem o cache 3D".to_string(),
        medido: format!("No {}: {}.", cpu.trim(), faltas.join("; ")),
        conselho: "Nestes processadores só um dos dois blocos tem o cache 3D, e quem manda o jogo para \
                   ele é o serviço da AMD com a Game Bar. Instale o driver de chipset mais novo do site da \
                   AMD, mantenha a Xbox Game Bar instalada, e use o plano Equilibrado, que é o que a AMD \
                   recomenda."
            .to_string(),
        severidade: FindingSeverity::Important,
        onde: FixLocation::Software,
    }]
}

/// O processador, direto do registro.
#[cfg(windows)]
pub fn cpu() -> Option<String> {
    super::registry::read_text("HKLM", r"HARDWARE\DESCRIPTION\System\CentralProcessor\0", "ProcessorNameString")
        .ok()
        .flatten()
}

/// Lê o que falta. Só chamado com X3D de dois blocos.
#[cfg(windows)]
pub fn ler() -> Leitura {
    // Consulta que falha fica nula, e não "ausente": sem isso, um erro de
    // permissão viraria "o serviço da AMD não está instalado".
    let script = "$servico = $null; $gamebar = $null; \
                  try { $s = Get-Service -DisplayName '*V-Cache*' -ErrorAction Stop | Select-Object -First 1; \
                        $servico = if ($s) { [string]$s.Status } else { 'ausente' } } catch { }; \
                  try { $gamebar = [bool](Get-AppxPackage -Name Microsoft.XboxGamingOverlay -ErrorAction Stop) } catch { }; \
                  ConvertTo-Json -Compress -InputObject ([ordered]@{ Servico = $servico; GameBar = $gamebar })";

    #[derive(serde::Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct Bruto {
        servico: Option<String>,
        game_bar: Option<bool>,
    }

    let bruto: Option<Bruto> = super::shell::powershell(script)
        .ok()
        .filter(|s| s.success)
        .and_then(|s| serde_json::from_str(s.stdout.trim()).ok());

    let saida_do_plano = super::shell::run_checked("powercfg", &["/getactivescheme"]).ok();
    let plano_ativo = saida_do_plano.as_deref().and_then(super::power::parse_active_guid).map(|g| g.to_lowercase());

    Leitura {
        servico_rodando: bruto.as_ref().and_then(|b| b.servico.as_deref()).map(|s| s == "Running"),
        game_bar_instalada: bruto.as_ref().and_then(|b| b.game_bar),
        plano_ativo,
        plano_do_otimiza: saida_do_plano.is_some_and(|s| s.to_uppercase().contains("OTIMIZA")),
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn reconhece_os_quatro_de_dois_blocos() {
        assert!(e_x3d_de_dois_blocos("AMD Ryzen 9 7950X3D 16-Core Processor"));
        assert!(e_x3d_de_dois_blocos("AMD Ryzen 9 9900X3D 12-Core Processor"));
        assert!(!e_x3d_de_dois_blocos("AMD Ryzen 7 7800X3D 8-Core Processor"), "um bloco só");
        assert!(!e_x3d_de_dois_blocos("AMD Ryzen 9 7950X 16-Core Processor"));
    }

    #[test]
    fn tudo_certo_nao_acusa() {
        let l = Leitura { servico_rodando: Some(true), game_bar_instalada: Some(true), plano_ativo: Some(EQUILIBRADO.into()), plano_do_otimiza: false };
        assert!(achados("AMD Ryzen 9 7950X3D", &l).is_empty());
    }

    #[test]
    fn diz_cada_coisa_que_falta() {
        let l = Leitura { servico_rodando: Some(false), game_bar_instalada: Some(true), plano_ativo: Some("8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c".into()), plano_do_otimiza: false };
        let a = achados("AMD Ryzen 9 9950X3D", &l);
        assert_eq!(a.len(), 1);
        assert!(a[0].medido.contains("serviço") && a[0].medido.contains("Equilibrado"), "{}", a[0].medido);
        assert!(!a[0].medido.contains("Game Bar"), "{}", a[0].medido);
    }

    #[test]
    fn leitura_que_falhou_nao_vira_acusacao() {
        assert!(achados("AMD Ryzen 9 7950X3D", &Leitura::default()).is_empty());
    }
}
