// Cache de shader e idade do driver. O cache não é limpo quando o driver atualiza, e entrada compilada pelo
// antigo faz o jogo recompilar no meio da partida: a explicação mais comum de "atualizei o driver e começou a
// travar". Apagar é seguro (recalculável), mas a primeira partida recompila, e a tela diz isso.

use super::shell;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShaderCache {
    pub id: String,
    pub name: String,
    pub path: String,
    pub bytes: u64,
    pub formatted: String,
    pub files: usize,
    pub oldest: Option<String>,
    pub stale: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShaderReport {
    pub caches: Vec<ShaderCache>,
    pub total_bytes: u64,
    pub total_formatted: String,
    pub gpu: Option<String>,
    pub driver_version: Option<String>,
    pub driver_date: Option<String>,
    pub driver_age_days: Option<i64>,
    pub note: String,
}

pub fn format_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let b = bytes as f64;

    if b >= GB {
        format!("{:.1} GB", b / GB)
    } else if b >= MB {
        format!("{:.0} MB", b / MB)
    } else if b >= KB {
        format!("{:.0} KB", b / KB)
    } else {
        format!("{} B", bytes)
    }
}

/// Lista explícita: varrer atrás de "pasta que parece cache" apagaria o que não é.
fn locais() -> Vec<(&'static str, &'static str, PathBuf)> {
    let local = PathBuf::from(std::env::var("LOCALAPPDATA").unwrap_or_default());
    let temp = std::env::temp_dir();

    vec![
        (
            "nvidia_dx",
            "Cache do DirectX (NVIDIA)",
            local.join("NVIDIA").join("DXCache"),
        ),
        (
            "nvidia_gl",
            "Cache do OpenGL e Vulkan (NVIDIA)",
            local.join("NVIDIA").join("GLCache"),
        ),
        (
            "nvidia_nv",
            "Cache temporário (NVIDIA)",
            temp.join("NVIDIA Corporation").join("NV_Cache"),
        ),
        ("amd_dx", "Cache do DirectX (AMD)", local.join("AMD").join("DxCache")),
        (
            "amd_gl",
            "Cache do OpenGL (AMD)",
            local.join("AMD").join("GLCache"),
        ),
        (
            "intel",
            "Cache de shader (Intel)",
            local.join("Intel").join("ShaderCache"),
        ),
        ("windows", "Cache de shader do Windows", local.join("D3DSCache")),
    ]
}

fn medir(dir: &Path) -> (u64, usize, Option<std::time::SystemTime>) {
    let Ok(entradas) = std::fs::read_dir(dir) else {
        return (0, 0, None);
    };

    let mut bytes = 0u64;
    let mut arquivos = 0usize;
    let mut antigo: Option<std::time::SystemTime> = None;

    for entrada in entradas.flatten() {
        let Ok(meta) = entrada.metadata() else { continue };

        if meta.is_dir() {
            let (b, n, a) = medir(&entrada.path());
            bytes += b;
            arquivos += n;

            if let Some(data) = a {
                antigo = Some(antigo.map_or(data, |atual| atual.min(data)));
            }
            continue;
        }

        bytes += meta.len();
        arquivos += 1;

        if let Ok(data) = meta.modified() {
            antigo = Some(antigo.map_or(data, |atual| atual.min(data)));
        }
    }

    (bytes, arquivos, antigo)
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
struct RawGpu {
    name: Option<String>,
    driver_version: Option<String>,
    driver_date: Option<String>,
}

fn placa_de_video() -> Option<RawGpu> {
    // Convertida no PowerShell: o formato de data do WMI muda com o idioma.
    let script = "ConvertTo-Json -Compress -InputObject (Get-CimInstance Win32_VideoController \
                  -ErrorAction SilentlyContinue | Where-Object { $_.DriverDate } | \
                  Sort-Object AdapterRAM -Descending | Select-Object -First 1 Name,DriverVersion,\
                  @{n='DriverDate';e={$_.DriverDate.ToString('yyyy-MM-dd')}})";

    shell::powershell(script)
        .ok()
        .filter(|s| s.success && !s.stdout.trim().is_empty())
        .and_then(|s| serde_json::from_str(&s.stdout).ok())
}

pub fn provedor_do_driver() -> Option<String> {
    // Lido UMA VEZ por execução: é WMI pelo PowerShell, e quem troca o driver reinicia o PC de qualquer jeito.
    static LEMBRADO: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    LEMBRADO
        .get_or_init(|| {
            let script = "(Get-CimInstance Win32_PnPSignedDriver -Filter \"DeviceClass='DISPLAY'\" \
                          -ErrorAction Stop | Where-Object { $_.DriverProviderName } | \
                          Select-Object -First 1).DriverProviderName";
            shell::powershell(script)
                .ok()
                .filter(|s| s.success)
                .map(|s| s.stdout.trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .clone()
}

/// Com o driver genérico do Windows a placa roda sem aceleração completa: atualizar muda FPS de verdade.
pub fn driver_generico(provedor: &str, nome_da_placa: Option<&str>) -> bool {
    provedor.trim().eq_ignore_ascii_case("microsoft")
        || nome_da_placa.is_some_and(|n| n.to_lowercase().contains("basic display"))
}

fn para_texto(momento: std::time::SystemTime) -> String {
    let data: chrono::DateTime<chrono::Local> = momento.into();
    data.format("%d/%m/%Y").to_string()
}

pub fn e_obsoleto(cache_mais_antigo: Option<chrono::NaiveDate>, driver: Option<chrono::NaiveDate>) -> bool {
    match (cache_mais_antigo, driver) {
        (Some(cache), Some(driver)) => cache < driver,
        _ => false,
    }
}

pub fn montar_nota(total: u64, obsoletos: usize, idade_driver: Option<i64>) -> String {
    let mut nota = String::new();

    if total > 0 {
        nota.push_str(&format!(
            "{} em cache de shader. Isso é resultado de compilação que o jogo guardou para não \
             repetir o trabalho, e apagar não perde nada além de tempo: a primeira partida \
             depois da limpeza compila de novo e engasga um pouco, e a partir da segunda fica \
             melhor que antes. ",
            format_size(total)
        ));
    }

    if obsoletos > 0 {
        nota.push_str(&format!(
            "Há {} cache(s) com entrada mais antiga que o driver de vídeo instalado — ou seja, \
             compilada por um driver que não existe mais nesta máquina. É a explicação mais \
             comum para \"atualizei o driver e começou a travar\", e nesses casos limpar \
             costuma resolver de vez. ",
            obsoletos
        ));
    }

    if let Some(dias) = idade_driver {
        if dias > 540 {
            nota.push_str(&format!(
                "Além disso, o driver de vídeo tem {} dias — mais de um ano e meio. Em placa \
                 usada para jogo isso costuma custar desempenho e estabilidade, e atualizar é \
                 de graça.",
                dias
            ));
        }
    }

    if nota.is_empty() {
        nota.push_str("Nenhum cache de shader encontrado nesta máquina.");
    }

    nota
}

pub fn analyze() -> ShaderReport {
    let gpu = placa_de_video();

    let data_driver = gpu
        .as_ref()
        .and_then(|g| g.driver_date.as_deref())
        .and_then(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok());

    let driver_age_days = data_driver.map(|d| (chrono::Local::now().date_naive() - d).num_days());

    let mut caches = Vec::new();
    let mut total_bytes = 0u64;
    let mut obsoletos = 0usize;

    for (id, nome, caminho) in locais() {
        if !caminho.is_dir() {
            continue;
        }

        let (bytes, files, antigo) = medir(&caminho);

        if bytes == 0 {
            continue;
        }

        let data_cache = antigo.map(|m| {
            let d: chrono::DateTime<chrono::Local> = m.into();
            d.date_naive()
        });

        let stale = e_obsoleto(data_cache, data_driver);
        if stale {
            obsoletos += 1;
        }

        total_bytes += bytes;

        caches.push(ShaderCache {
            id: id.to_string(),
            name: nome.to_string(),
            path: caminho.to_string_lossy().to_string(),
            formatted: format_size(bytes),
            bytes,
            files,
            oldest: antigo.map(para_texto),
            stale,
        });
    }

    caches.sort_by(|a, b| b.stale.cmp(&a.stale).then(b.bytes.cmp(&a.bytes)));

    ShaderReport {
        note: montar_nota(total_bytes, obsoletos, driver_age_days),
        caches,
        total_bytes,
        total_formatted: format_size(total_bytes),
        gpu: gpu.as_ref().and_then(|g| g.name.clone()),
        driver_version: gpu.as_ref().and_then(|g| g.driver_version.clone()),
        driver_date: gpu.and_then(|g| g.driver_date),
        driver_age_days,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanOutcome {
    pub freed_mb: f64,
    pub message: String,
}

/// A única trava é a lista de pastas conhecidas: o comando vem por IPC e não pode receber caminho de fora.
pub fn limpar(id: &str) -> Result<CleanOutcome, String> {
    let (_, nome, caminho) = locais()
        .into_iter()
        .find(|(chave, _, _)| *chave == id)
        .ok_or_else(|| format!("`{}` não é um cache conhecido.", id))?;

    if !caminho.is_dir() {
        return Err(format!("`{}` não existe nesta máquina.", nome));
    }

    let (antes, _, _) = medir(&caminho);
    esvaziar(&caminho);
    let (depois, _, _) = medir(&caminho);

    let liberado = antes.saturating_sub(depois);

    Ok(CleanOutcome {
        freed_mb: liberado as f64 / 1_048_576.0,
        message: format!(
            "{} liberados de {}. A primeira partida vai compilar de novo e pode engasgar; da \
             segunda em diante fica melhor que estava.",
            format_size(liberado),
            nome
        ),
    })
}

/// Arquivo travado pelo driver é pulado; o número relatado é medido de novo depois.
fn esvaziar(dir: &Path) {
    let Ok(entradas) = std::fs::read_dir(dir) else {
        return;
    };

    for entrada in entradas.flatten() {
        let caminho = entrada.path();

        let _ = if caminho.is_dir() {
            std::fs::remove_dir_all(&caminho)
        } else {
            std::fs::remove_file(&caminho)
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn data(a: i32, m: u32, d: u32) -> Option<NaiveDate> {
        NaiveDate::from_ymd_opt(a, m, d)
    }

    #[test]
    fn driver_da_microsoft_e_generico_e_o_do_fabricante_nao() {
        assert!(driver_generico("Microsoft", Some("Microsoft Basic Display Adapter")));
        assert!(driver_generico("  microsoft ", None));
        assert!(!driver_generico("NVIDIA", Some("NVIDIA GeForce GTX 1650")));
        assert!(driver_generico("NVIDIA", Some("Microsoft Basic Display Adapter")));
    }

    #[test]
    fn cache_anterior_ao_driver_e_obsoleto() {
        assert!(e_obsoleto(data(2026, 2, 14), data(2026, 7, 21)));
        assert!(!e_obsoleto(data(2026, 8, 1), data(2026, 7, 21)));
    }

    #[test]
    fn sem_uma_das_datas_nao_se_afirma_nada() {
        // Sem a data do driver, dizer que o cache está velho seria chute.
        assert!(!e_obsoleto(data(2020, 1, 1), None));
        assert!(!e_obsoleto(None, data(2026, 7, 21)));
        assert!(!e_obsoleto(None, None));
    }

    #[test]
    fn a_nota_declara_a_contrapartida() {
        let nota = montar_nota(9_000_000_000, 1, Some(19));

        assert!(nota.contains("engasga um pouco"));
        assert!(nota.contains("a partir da segunda"));
        assert!(nota.contains("não existe mais nesta máquina"));
    }

    #[test]
    fn driver_recente_nao_vira_alarme() {
        let recente = montar_nota(1_000_000, 0, Some(19));
        assert!(!recente.contains("mais de um ano e meio"));

        let velho = montar_nota(1_000_000, 0, Some(800));
        assert!(velho.contains("mais de um ano e meio"));
    }

    #[test]
    fn sem_cache_a_nota_nao_fica_vazia() {
        let nota = montar_nota(0, 0, None);
        assert!(!nota.is_empty());
        assert!(nota.contains("Nenhum cache"));
    }

    #[test]
    fn limpar_recusa_caminho_de_fora() {
        let erro = limpar("../../Windows/System32").unwrap_err();
        assert!(erro.contains("não é um cache conhecido"));
    }

    #[test]
    fn tamanho_sai_na_unidade_certa() {
        assert_eq!(format_size(5 * 1024 * 1024), "5 MB");
        assert_eq!(format_size(9 * 1024 * 1024 * 1024), "9.0 GB");
    }

    #[test]
    fn analisa_esta_maquina() {
        let r = analyze();

        println!(
            "placa: {:?} | driver {:?} de {:?} ({:?} dias)",
            r.gpu, r.driver_version, r.driver_date, r.driver_age_days
        );
        println!("total: {}", r.total_formatted);
        println!("nota: {}", r.note);

        for c in &r.caches {
            println!(
                "  {:>9}  {:<38} {} arquivos, mais antigo {:?}{}",
                c.formatted,
                c.name,
                c.files,
                c.oldest,
                if c.stale { "  [OBSOLETO]" } else { "" }
            );
        }

        assert!(!r.note.is_empty());

        assert!(r.caches.iter().all(|c| c.bytes > 0 && c.files > 0));

        let chave = |c: &ShaderCache| (!c.stale, std::cmp::Reverse(c.bytes));
        assert!(r.caches.windows(2).all(|p| chave(&p[0]) <= chave(&p[1])));

        let soma: u64 = r.caches.iter().map(|c| c.bytes).sum();
        assert_eq!(soma, r.total_bytes);
    }
}

#[cfg(test)]
mod nesta_maquina {
    #[test]
    #[ignore]
    fn provedor_desta_placa() {
        println!("provedor: {:?}", super::provedor_do_driver());
    }
}
