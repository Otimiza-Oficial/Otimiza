// Hardware: o que a máquina tem (placa, memória, monitores, VRR, perfil de hardware, compatibilidade) e a
// BIOS, que é onde boa parte disso se ajusta.

use super::*;

#[derive(Serialize)]
pub struct PlatformInfoResponse {
    pub platform: String,
    pub os_type: String,
    pub arch: String,
    pub version: String,
}

#[tauri::command(async)]
pub fn get_platform_info() -> Result<PlatformInfoResponse, String> {
    let info = PlatformDetector::get_info();
    Ok(PlatformInfoResponse {
        platform: format!("{:?}", info.platform),
        os_type: info.os_type,
        arch: info.arch,
        version: info.version,
    })
}

#[derive(Serialize)]
pub struct HardwareProfileResponse {
    pub storage: String,
    pub total_ram_gb: f64,
    pub logical_cores: usize,
    pub cpu_name: String,
    pub gpu_name: String,
}

/// Permite recusar o que faria mal a este PC.
#[tauri::command(async)]
pub fn get_hardware_profile() -> Result<HardwareProfileResponse, String> {
    #[cfg(target_os = "windows")]
    {
        use crate::modules::windows::hardware::{profile, StorageKind};

        let hardware = profile();
        Ok(HardwareProfileResponse {
            storage: match hardware.system_storage {
                StorageKind::Ssd => "SSD".to_string(),
                StorageKind::Hdd => "HD mecânico".to_string(),
                StorageKind::Unknown => "não identificado".to_string(),
            },
            total_ram_gb: hardware.total_ram_gb,
            logical_cores: hardware.logical_cores,
            cpu_name: hardware.cpu_name.clone(),
            gpu_name: hardware.gpu_name.clone(),
        })
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// `LIVRES`: **não escreve absolutamente nada** (uma ferramenta capaz de alterar firmware deixaria a máquina sem
/// ligar). Memória abaixo do nominal e Resizable BAR vêm de quem já mede: remedir daria dois números.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn passo_a_passo_da_bios() -> BiosNaTela {
    #[cfg(target_os = "windows")]
    {
        use crate::modules::windows::{bios, firmware, planoenergia, rbar};

        let leitura = bios::ler();

        let memoria_abaixo = firmware::analyze_memory_ou_lacuna()
            .map(|achados| achados.iter().any(|a| a.id == "memory_xmp_off"))
            .unwrap_or(false);

        let rebar_desligado = matches!(
            rbar::analyze().estado,
            rbar::EstadoDoRbar::DesligadoESuportado
        );

        let amd = matches!(
            planoenergia::detectar().fabricante_da_cpu,
            planoenergia::FabricanteDaCpu::Amd
        );

        BiosNaTela {
            passos: bios::montar(&leitura, memoria_abaixo, rebar_desligado, amd),
            leitura,
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        BiosNaTela::default()
    }
}

/// `LIVRES`: só lê.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn ficha_da_bios() -> Result<crate::modules::windows::fichabios::Ficha, String> {
    tokio::task::spawn_blocking(crate::modules::windows::fichabios::ler)
        .await
        .map_err(|e| format!("Falha ao ler o firmware: {}", e))
}

/// `LIVRES`: grava a ficha num arquivo da Área de Trabalho, com nome escolhido aqui. O texto vem da tela (é o que
/// ela mostra), limitado em tamanho.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn salvar_ficha_da_bios(texto: String) -> Result<String, String> {
    const LIMITE: usize = 64 * 1024;
    if texto.trim().is_empty() || texto.len() > LIMITE {
        return Err("A ficha está vazia ou grande demais para salvar.".to_string());
    }
    crate::modules::report::salvar_texto("ficha da BIOS", &texto)
}

/// `LIVRES`: não altera configuração; a tela confirma antes.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn reiniciar_na_bios() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        crate::modules::windows::fichabios::reiniciar_na_bios()
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct BiosNaTela {
    pub leitura: crate::modules::windows::bios::Leitura,
    pub passos: Vec<crate::modules::windows::bios::Passo>,
}

/// `LIVRES`: o `rbar` só lê, e só a BIOS liga. `(async)`: chama `nvidia-smi`, processo externo.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn analyze_rbar() -> Result<crate::modules::windows::rbar::RelatorioDoRbar, String> {
    #[cfg(target_os = "windows")]
    {
        Ok(crate::modules::windows::rbar::analyze())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// O retrato do PC. `atualizar` relê tudo (PowerShell e WMI); sem ele vale o cache de 10 minutos. `LIVRES`.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn hardware_perfil(
    atualizar: bool,
) -> Result<crate::plataforma::hardware::PerfilDeHardware, String> {
    Ok(if atualizar {
        crate::plataforma::hardware::atualizar()
    } else {
        crate::plataforma::hardware::perfil()
    })
}

/// O último retrato gravado, para a abertura mostrar algo antes da leitura terminar. `LIVRES`.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn hardware_ultimo() -> Result<Option<crate::plataforma::hardware::PerfilDeHardware>, String> {
    Ok(crate::plataforma::hardware::ultimo_conhecido())
}

/// O que funciona neste PC: Frame Boost, baixa latência, mira, cores. `LIVRES`: só lê.
#[tauri::command(async)]
pub fn compatibilidade() -> Result<crate::plataforma::Compatibilidade, String> {
    #[cfg(target_os = "windows")]
    let barrada = {
        use crate::modules::windows::anticheat;
        anticheat::permite(
            anticheat::Acao::SobreporJanela,
            &anticheat::detectar_agora(),
        )
        .motivo()
        .map(str::to_string)
    };
    #[cfg(not(target_os = "windows"))]
    let barrada = None;

    Ok(crate::plataforma::compatibilidade(
        crate::plataforma::hardware::perfil(),
        barrada,
    ))
}

/// Sem a lista, o cliente lia "seu monitor está em 60 Hz" sem saber QUAL. `LIVRES`.
#[tauri::command]
pub async fn monitores() -> Result<Vec<crate::modules::windows::display::Monitor>, String> {
    #[cfg(target_os = "windows")]
    {
        Ok(crate::modules::windows::display::monitores())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// `LIVRES`.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn memoria_instalada(
) -> Result<crate::modules::windows::firmware::MemoriaInstalada, String> {
    #[cfg(target_os = "windows")]
    {
        Ok(crate::modules::windows::firmware::memoria_instalada())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

#[derive(serde::Serialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct PlacaDeVideo {
    pub marca: String,
    pub nome: Option<String>,
    pub driver: Option<String>,
    pub driver_data: Option<String>,
    pub driver_dias: Option<i64>,
    pub driver_origem: Option<String>,
    pub driver_generico: bool,
    pub vram_gb: f64,
}

/// Pelo NOME, o mesmo texto do Gerenciador de Dispositivos: quando erra, o cliente percebe. Na dúvida,
/// `desconhecida`.
pub fn marca_da_placa(nome: &str) -> &'static str {
    let n = nome.to_lowercase();

    if n.contains("nvidia") || n.contains("geforce") || n.contains("quadro") || n.contains("rtx") {
        "nvidia"
    } else if n.contains("amd") || n.contains("radeon") || n.contains("ati ") {
        "amd"
    } else if n.contains("intel") || n.contains("arc ") || n.contains("iris") {
        "intel"
    } else {
        "desconhecida"
    }
}

/// O nome de `shaders`, a memória de `bottleneck` (o WMI satura em 4 GB). `LIVRES`.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn placa_de_video() -> Result<PlacaDeVideo, String> {
    #[cfg(target_os = "windows")]
    {
        let s = crate::modules::windows::shaders::analyze();
        let nome = s.gpu.clone();
        let origem = crate::modules::windows::shaders::provedor_do_driver();

        Ok(PlacaDeVideo {
            marca: nome
                .as_deref()
                .map(marca_da_placa)
                .unwrap_or("desconhecida")
                .to_string(),
            nome,
            driver: s.driver_version.clone(),
            driver_data: s.driver_date.clone(),
            driver_dias: s.driver_age_days,
            driver_generico: origem.as_deref().is_some_and(|p| {
                crate::modules::windows::shaders::driver_generico(p, s.gpu.as_deref())
            }),
            driver_origem: origem,
            vram_gb: crate::modules::windows::bottleneck::vram_total_gb(),
        })
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// `LIVRES`: o que os monitores ligados anunciam de taxa variável, pelo EDID.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn vrr_dos_monitores() -> Result<Vec<crate::modules::windows::vrr::MonitorVrr>, String> {
    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(crate::modules::windows::vrr::monitores)
            .await
            .map_err(|e| format!("Falha ao ler os monitores: {}", e))
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// `id` porque é o nome que o botão do diagnóstico manda; aqui é o dispositivo (`\.\DISPLAY1`).
#[tauri::command]
pub async fn set_max_refresh_rate(
    id: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    crate::modules::licenca::exigir()?;

    #[cfg(target_os = "windows")]
    {
        let mut log = state.changes.lock().await;

        crate::modules::windows::WindowsOptimizer::new()
            .set_max_refresh_rate(&id, &mut log)
            .map(|resultado| resultado.message)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (id, state);
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// `LIVRES`: é de quem ainda não comprou, ou reclamou, que vem o dado das máquinas que o projeto não tem.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn relatorio_de_compatibilidade() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        crate::modules::windows::labcompat::gerar()
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}
