// Qual placa cada jogo usa: em notebook com duas placas, o jogo na integrada roda a uma fração do que a máquina
// consegue, e acertar isso rende de duas a cinco vezes. Com uma placa só, o ganho é zero e o produto fica calado.
// `HKCU\SOFTWARE\Microsoft\DirectX\UserGpuPreferences`, um valor por jogo (`GpuPreference=N;`, com o ponto e
// vírgula), a mesma chave das Configurações do Windows.

use crate::modules::changelog::ChangeRecord;
use serde::{Deserialize, Serialize};
use std::path::Path;

const CHAVE: &str = r"SOFTWARE\Microsoft\DirectX\UserGpuPreferences";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Preferencia {
    Automatica,
    Economia,
    Desempenho,
}

impl Preferencia {
    fn codigo(self) -> u8 {
        match self {
            Preferencia::Automatica => 0,
            Preferencia::Economia => 1,
            Preferencia::Desempenho => 2,
        }
    }

    fn do_codigo(codigo: u8) -> Option<Self> {
        match codigo {
            0 => Some(Preferencia::Automatica),
            1 => Some(Preferencia::Economia),
            2 => Some(Preferencia::Desempenho),
            _ => None,
        }
    }

    pub fn nome(self) -> &'static str {
        match self {
            Preferencia::Automatica => "decidida pelo Windows",
            Preferencia::Economia => "placa de economia",
            Preferencia::Desempenho => "placa de desempenho",
        }
    }
}

/// O ponto e vírgula final faz parte do formato.
pub fn texto_da_preferencia(preferencia: Preferencia) -> String {
    format!("GpuPreference={};", preferencia.codigo())
}

/// O valor pode ter mais de um ajuste: procura o campo, não compara a string inteira.
pub fn preferencia_do_texto(bruto: &str) -> Option<Preferencia> {
    bruto
        .split(';')
        .filter_map(|campo| campo.split_once('='))
        .find(|(chave, _)| chave.trim().eq_ignore_ascii_case("GpuPreference"))
        .and_then(|(_, valor)| valor.trim().parse::<u8>().ok())
        .and_then(Preferencia::do_codigo)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuPrefFinding {
    pub id: String,
    pub title: String,
    pub measured: String,
    pub advice: String,
    pub severity: super::achados::FindingSeverity,
    pub fix_location: super::achados::FixLocation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuPrefReport {
    pub placas: Vec<String>,
    pub tem_placa_dupla: bool,
    pub definidos: Vec<(String, Preferencia)>,
    pub findings: Vec<GpuPrefFinding>,
    /// Aí `definidos` vazio NÃO quer dizer "nenhum jogo fixado".
    pub erro_de_leitura: Option<String>,
}

#[cfg(target_os = "windows")]
pub fn placas() -> Vec<String> {
    // `PCI\` descarta adaptador de área de trabalho remota e software de captura.
    let script = "@(Get-CimInstance Win32_VideoController | \
                  Where-Object { $_.PNPDeviceID -like 'PCI\\*' } | \
                  Select-Object -ExpandProperty Name)";

    match super::shell::powershell(script) {
        Ok(saida) if saida.success => saida
            .stdout
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect(),
        _ => Vec::new(),
    }
}

#[cfg(not(target_os = "windows"))]
pub fn placas() -> Vec<String> {
    Vec::new()
}

/// `Err` quando a chave existe e não se lê: lista vazia afirmaria que o Windows escolhe sozinho para todos.
pub fn definidos() -> Result<Vec<(String, Preferencia)>, String> {
    let mut definidos = Vec::new();

    for caminho in super::registry::value_names("HKCU", CHAVE)? {
        let Some(bruto) = super::registry::read_text("HKCU", CHAVE, &caminho)? else {
            continue;
        };

        if let Some(preferencia) = preferencia_do_texto(&bruto) {
            definidos.push((caminho, preferencia));
        }
    }

    Ok(definidos)
}

pub fn diagnosticar(placas: &[String], definidos: &[(String, Preferencia)]) -> Vec<GpuPrefFinding> {
    use super::achados::{FindingSeverity, FixLocation};

    // Com uma placa só, escolher placa não existe.
    if placas.len() < 2 {
        return Vec::new();
    }

    let na_economia: Vec<&String> = definidos
        .iter()
        .filter(|(_, p)| *p == Preferencia::Economia)
        .map(|(caminho, _)| caminho)
        .filter(|caminho| Path::new(caminho).exists())
        .collect();

    if na_economia.is_empty() {
        return Vec::new();
    }

    let nomes: Vec<String> = na_economia
        .iter()
        .take(3)
        .map(|caminho| {
            Path::new(caminho)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| (*caminho).clone())
        })
        .collect();

    vec![GpuPrefFinding {
        id: "jogo_na_placa_errada".to_string(),
        title: "Jogo configurado para a placa de vídeo mais fraca".to_string(),
        measured: format!(
            "{} programa(s) estão fixados na placa de economia, numa máquina com {} placas: {}.",
            na_economia.len(),
            placas.len(),
            nomes.join(", ")
        ),
        advice: "Num PC com duas placas de vídeo, jogo rodando na placa de economia entrega \
                 uma fração do que a máquina consegue — e é invisível para quem está jogando: \
                 o jogo abre normalmente e só roda mal. Trocar para a placa de desempenho é \
                 o maior ganho de FPS que existe nesse caso, e vale na próxima vez que o jogo \
                 abrir."
            .to_string(),
        severity: FindingSeverity::Critical,
        fix_location: FixLocation::Software,
    }]
}

pub fn analyze() -> GpuPrefReport {
    let placas = placas();
    let (definidos, erro_de_leitura) = match definidos() {
        Ok(lista) => (lista, None),
        Err(erro) => (Vec::new(), Some(erro)),
    };
    let findings = diagnosticar(&placas, &definidos);

    GpuPrefReport {
        tem_placa_dupla: placas.len() >= 2,
        placas,
        definidos,
        findings,
        erro_de_leitura,
    }
}

pub fn definir(executavel: &Path, preferencia: Preferencia) -> Result<ChangeRecord, String> {
    // O nome do valor é o caminho: precisa ser completo e existir, senão isto gravaria texto arbitrário no registro.
    if !executavel.is_absolute() {
        return Err("O caminho do jogo precisa ser completo.".to_string());
    }

    if !executavel.exists() {
        return Err(format!(
            "`{}` não existe. Só dá para escolher a placa de um jogo instalado.",
            executavel.display()
        ));
    }

    let chave_do_valor = executavel.to_string_lossy().to_string();

    // Sem `unwrap_or`: leitura falha viraria "não havia preferência", e o desfazer APAGARIA a escolha do cliente.
    let anterior = super::registry::read("HKCU", CHAVE, &chave_do_valor)?;

    super::registry::set_string(
        "HKCU",
        CHAVE,
        &chave_do_valor,
        &texto_da_preferencia(preferencia),
    )?;

    Ok(ChangeRecord::RegistryValue {
        hive: "HKCU".to_string(),
        path: CHAVE.to_string(),
        name: chave_do_valor,
        previous: anterior,
    })
}

/// A OTIMIZAÇÃO DE JANELA DO WINDOWS para um jogo (Configurações → Tela → Gráficos, "Otimizações para jogos em janela").
/// Jogo DirectX 10/11 em janela ou sem bordas sai do modelo de cópia (`Composed: Copy with GPU GDI`) para o de flip.
/// Medido no FiveM do dono: sem bordas, na cópia, 61 fps; em tela cheia (flip direto), 85. O Frame Boost precisa de
/// janela ou sem bordas, então é aqui que ele perde ou não esses 24 fps. Vale ao reabrir o jogo.
pub const OTIMIZACAO_DE_JANELA: &str = "SwapEffectUpgradeEnable";

/// Pura: o valor do jogo com a otimização ligada, mantendo o resto da linha (o Windows guarda tudo no mesmo valor:
/// `GpuPreference`, `AutoHDREnable`, `AppStatus`…). `None` = já estava ligada.
pub fn com_otimizacao_de_janela(atual: Option<&str>) -> Option<String> {
    let mut partes: Vec<String> = atual
        .unwrap_or("")
        .split(';')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .collect();
    let prefixo = format!("{}=", OTIMIZACAO_DE_JANELA);
    if partes.iter().any(|p| p.eq_ignore_ascii_case(&format!("{}1", prefixo))) {
        return None;
    }
    partes.retain(|p| !p.to_ascii_lowercase().starts_with(&prefixo.to_ascii_lowercase()));
    partes.push(format!("{}1", prefixo));
    Some(partes.iter().map(|p| format!("{};", p)).collect())
}

/// Liga a otimização de janela para o executável (caminho completo), guardando o valor de antes. `Ok(None)`: já estava.
pub fn ligar_otimizacao_de_janela(executavel: &Path) -> Result<Option<ChangeRecord>, String> {
    if !executavel.is_absolute() || !executavel.exists() {
        return Err(format!("`{}` não é o caminho de um jogo instalado.", executavel.display()));
    }
    let nome = executavel.to_string_lossy().to_string();
    let anterior = super::registry::read("HKCU", CHAVE, &nome)?;
    let texto = super::registry::read_text("HKCU", CHAVE, &nome)?;
    let Some(novo) = com_otimizacao_de_janela(texto.as_deref()) else {
        return Ok(None);
    };
    super::registry::set_string("HKCU", CHAVE, &nome, &novo)?;
    Ok(Some(ChangeRecord::RegistryValue {
        hive: "HKCU".to_string(),
        path: CHAVE.to_string(),
        name: nome,
        previous: anterior,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn otimizacao_de_janela_entra_sem_apagar_o_resto_da_linha() {
        // O valor do FiveM do dono antes: só `AppStatus=0;`.
        assert_eq!(
            com_otimizacao_de_janela(Some("AppStatus=0;")).as_deref(),
            Some("AppStatus=0;SwapEffectUpgradeEnable=1;")
        );
        assert_eq!(
            com_otimizacao_de_janela(Some("GpuPreference=2;SwapEffectUpgradeEnable=0;")).as_deref(),
            Some("GpuPreference=2;SwapEffectUpgradeEnable=1;"),
            "desligada por alguém: liga, sem duplicar a chave"
        );
        assert_eq!(com_otimizacao_de_janela(None).as_deref(), Some("SwapEffectUpgradeEnable=1;"));
        assert_eq!(
            com_otimizacao_de_janela(Some("SwapEffectUpgradeEnable=1;GpuPreference=1;")),
            None,
            "já ligada: nada a gravar"
        );
    }

    #[test]
    fn o_formato_do_windows_leva_ponto_e_virgula() {
        // Sem o ponto e vírgula o Windows ignora o valor em silêncio.
        assert_eq!(
            texto_da_preferencia(Preferencia::Desempenho),
            "GpuPreference=2;"
        );
        assert_eq!(
            texto_da_preferencia(Preferencia::Economia),
            "GpuPreference=1;"
        );
        assert_eq!(
            texto_da_preferencia(Preferencia::Automatica),
            "GpuPreference=0;"
        );
    }

    #[test]
    fn le_a_preferencia_mesmo_com_outros_ajustes_na_mesma_linha() {
        assert_eq!(
            preferencia_do_texto("GpuPreference=2;"),
            Some(Preferencia::Desempenho)
        );
        assert_eq!(
            preferencia_do_texto("AutoHDREnable=1;GpuPreference=1;"),
            Some(Preferencia::Economia)
        );
        assert_eq!(preferencia_do_texto("AutoHDREnable=1;"), None);
        assert_eq!(preferencia_do_texto(""), None);
        assert_eq!(preferencia_do_texto("GpuPreference=9;"), None);
    }

    #[test]
    fn maquina_de_uma_placa_nao_ganha_achado_nenhum() {
        let definidos = vec![(r"C:\Jogo\jogo.exe".to_string(), Preferencia::Economia)];

        assert!(diagnosticar(&["NVIDIA GeForce GTX 1650".to_string()], &definidos).is_empty());
        assert!(diagnosticar(&[], &definidos).is_empty());
    }

    #[test]
    fn placa_dupla_sem_jogo_na_economia_tambem_fica_calado() {
        let placas = vec![
            "Intel UHD Graphics".to_string(),
            "NVIDIA GeForce RTX 4060".to_string(),
        ];

        assert!(diagnosticar(&placas, &[]).is_empty());
        assert!(diagnosticar(
            &placas,
            &[(r"C:\Jogo\jogo.exe".to_string(), Preferencia::Desempenho)]
        )
        .is_empty());
    }

    #[test]
    fn jogo_apagado_nao_conta() {
        // O Windows nunca limpa esta chave: jogo desinstalado há anos não pode virar achado.
        let placas = vec![
            "Intel UHD Graphics".to_string(),
            "NVIDIA GeForce RTX 4060".to_string(),
        ];
        let definidos = vec![(
            r"C:\Jogo\Que\Nao\Existe\Mais\jogo.exe".to_string(),
            Preferencia::Economia,
        )];

        assert!(diagnosticar(&placas, &definidos).is_empty());
    }

    #[test]
    fn caminho_relativo_ou_inexistente_e_recusado() {
        assert!(definir(Path::new("jogo.exe"), Preferencia::Desempenho).is_err());
        assert!(definir(
            Path::new(r"C:\Nao\Existe\jogo.exe"),
            Preferencia::Desempenho
        )
        .is_err());
    }

    #[test]
    fn analisa_esta_maquina() {
        let r = analyze();

        println!("  placas: {:?}", r.placas);
        println!("  placa dupla: {}", r.tem_placa_dupla);
        for (caminho, p) in r.definidos.iter().take(6) {
            println!("    {} → {}", caminho, p.nome());
        }
        for f in &r.findings {
            println!("  [{:?}] {}", f.severity, f.measured);
        }

        if !r.tem_placa_dupla {
            assert!(
                r.findings.is_empty(),
                "máquina de uma placa não pode receber achado de escolha de placa"
            );
        }
    }
}
