// Monitor de 144 Hz rodando a 60 é comum e invisível. A fonte é a decisão: `MaxRefreshRate` erra PARA MAIS (o
// que a placa emite); `WmiMonitorListedSupportedSourceModes` erra PARA MENOS (só as temporizações básicas da
// EDID: dois AOC 24G4 de 180 Hz saíam como 60). `EnumDisplaySettingsExW` é a mesma função que aplica: se lista,
// acontece. Fixando resolução e profundidade de cor atuais, senão viriam modos que não existem nesta configuração.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Monitor {
    pub dispositivo: String,
    pub descricao: String,
    pub principal: bool,
    pub largura: u32,
    pub altura: u32,
    pub hz_atual: u32,
    pub hz_disponiveis: Vec<u32>,
    #[serde(default)]
    pub adaptador: String,
}

impl Monitor {
    pub fn hz_maximo(&self) -> u32 {
        self.hz_disponiveis.iter().copied().max().unwrap_or(self.hz_atual)
    }

    pub fn abaixo_do_maximo(&self) -> bool {
        self.hz_maximo() > self.hz_atual
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayFinding {
    pub id: String,
    pub dispositivo: String,
    pub hz_alvo: u32,
    pub title: String,
    pub measured: String,
    pub advice: String,
    pub severity: super::achados::FindingSeverity,
    pub fix_location: super::achados::FixLocation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayReport {
    pub monitores: Vec<Monitor>,
    pub findings: Vec<DisplayFinding>,
}

/// Alguns monitores anunciam 59 e 60 como modos distintos.
const DIFERENCA_QUE_IMPORTA: u32 = 15;

/// `EnumDisplayDevicesW` dá o nome do ADAPTADOR, igual para todos os monitores dele; o nome real está na EDID.
/// Falhar aqui não é grave: o do adaptador fica de reserva.
#[cfg(target_os = "windows")]
fn nomes_comerciais() -> Vec<String> {
    let script = "@(Get-CimInstance -Namespace root\\wmi -ClassName WmiMonitorID \
                  -ErrorAction SilentlyContinue | ForEach-Object { \
                    ($_.UserFriendlyName | Where-Object { $_ -gt 0 } | \
                     ForEach-Object { [char]$_ }) -join '' })";

    match super::shell::powershell(script) {
        Ok(saida) if saida.success => saida
            .stdout
            .lines()
            .map(|linha| linha.trim().to_string())
            .filter(|nome| !nome.is_empty())
            .collect(),
        _ => Vec::new(),
    }
}

#[cfg(target_os = "windows")]
pub fn monitores() -> Vec<Monitor> {
    use windows_sys::Win32::Graphics::Gdi::{
        EnumDisplayDevicesW, EnumDisplaySettingsExW, DEVMODEW, DISPLAY_DEVICEW,
        DISPLAY_DEVICE_ATTACHED_TO_DESKTOP, DISPLAY_DEVICE_MIRRORING_DRIVER,
        DISPLAY_DEVICE_PRIMARY_DEVICE, ENUM_CURRENT_SETTINGS,
    };

    fn texto(bruto: &[u16]) -> String {
        let fim = bruto.iter().position(|c| *c == 0).unwrap_or(bruto.len());
        String::from_utf16_lossy(&bruto[..fim])
    }

    let comerciais = nomes_comerciais();
    let mut encontrados = Vec::new();

    unsafe {
        for indice in 0..16u32 {
            let mut dispositivo: DISPLAY_DEVICEW = std::mem::zeroed();
            dispositivo.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;

            if EnumDisplayDevicesW(std::ptr::null(), indice, &mut dispositivo, 0) == 0 {
                break;
            }

            // Driver de espelhamento é software de captura fingindo ser monitor.
            let ligado = dispositivo.StateFlags & DISPLAY_DEVICE_ATTACHED_TO_DESKTOP != 0;
            let espelho = dispositivo.StateFlags & DISPLAY_DEVICE_MIRRORING_DRIVER != 0;

            if !ligado || espelho {
                continue;
            }

            let nome = texto(&dispositivo.DeviceName);
            let mut nome_utf16: Vec<u16> = nome.encode_utf16().chain(std::iter::once(0)).collect();

            let mut atual: DEVMODEW = std::mem::zeroed();
            atual.dmSize = std::mem::size_of::<DEVMODEW>() as u16;

            if EnumDisplaySettingsExW(nome_utf16.as_mut_ptr(), ENUM_CURRENT_SETTINGS, &mut atual, 0)
                == 0
            {
                continue;
            }

            let mut hz_disponiveis: Vec<u32> = Vec::new();

            for modo in 0..1024u32 {
                let mut candidato: DEVMODEW = std::mem::zeroed();
                candidato.dmSize = std::mem::size_of::<DEVMODEW>() as u16;

                if EnumDisplaySettingsExW(nome_utf16.as_mut_ptr(), modo, &mut candidato, 0) == 0 {
                    break;
                }

                let mesmo_modo = candidato.dmPelsWidth == atual.dmPelsWidth
                    && candidato.dmPelsHeight == atual.dmPelsHeight
                    && candidato.dmBitsPerPel == atual.dmBitsPerPel;

                // 0 e 1 são "taxa padrão do hardware", não valores.
                if mesmo_modo && candidato.dmDisplayFrequency > 1 {
                    hz_disponiveis.push(candidato.dmDisplayFrequency);
                }
            }

            hz_disponiveis.sort_unstable();
            hz_disponiveis.dedup();

            // A ordem de `WmiMonitorID` acompanha a de `EnumDisplayDevicesW`.
            let descricao = comerciais
                .get(encontrados.len())
                .cloned()
                .unwrap_or_else(|| texto(&dispositivo.DeviceString));

            encontrados.push(Monitor {
                dispositivo: nome,
                descricao,
                principal: dispositivo.StateFlags & DISPLAY_DEVICE_PRIMARY_DEVICE != 0,
                largura: atual.dmPelsWidth,
                altura: atual.dmPelsHeight,
                hz_atual: atual.dmDisplayFrequency,
                hz_disponiveis,
                adaptador: texto(&dispositivo.DeviceString),
            });
        }
    }

    encontrados
}

#[cfg(not(target_os = "windows"))]
pub fn monitores() -> Vec<Monitor> {
    Vec::new()
}

pub fn diagnosticar(monitores: &[Monitor]) -> Vec<DisplayFinding> {
    use super::achados::{FindingSeverity, FixLocation};

    let mut findings = Vec::new();

    for monitor in monitores {
        let maximo = monitor.hz_maximo();

        if !monitor.abaixo_do_maximo() || maximo - monitor.hz_atual < DIFERENCA_QUE_IMPORTA {
            continue;
        }

        findings.push(DisplayFinding {
            id: format!("hz_abaixo_{}", monitor.dispositivo.replace(['\\', '.'], "")),
            dispositivo: monitor.dispositivo.clone(),
            hz_alvo: maximo,
            title: "Monitor rodando abaixo da taxa que ele aceita".to_string(),
            measured: format!(
                "{} está em {} Hz e aceita até {} Hz em {}x{}.",
                monitor.descricao,
                monitor.hz_atual,
                maximo,
                monitor.largura,
                monitor.altura
            ),
            // Subir a taxa não sobe o FPS, sobe o TETO: prometer FPS aqui seria mentira fácil de vender.
            advice: format!(
                "Colocar o monitor em {} Hz é a maior diferença de fluidez que existe num \
                 PC, e não custa desempenho nenhum. Mas não espere um número de FPS maior: \
                 a taxa do monitor não cria quadros, ela deixa de segurar os que a placa já \
                 entrega. O jogo fica mais suave, e o contador continua onde estava.",
                maximo
            ),
            severity: FindingSeverity::Important,
            fix_location: FixLocation::Software,
        });
    }

    findings
}

/// Devolve a frequência ANTERIOR, para o histórico saber voltar. Duas chamadas: `CDS_TEST` pergunta ao driver
/// sem mudar nada, e só então aplica. Um modo errado apaga a tela, e aí o cliente nem enxerga o desfazer.
#[cfg(target_os = "windows")]
pub fn aplicar_hz(dispositivo: &str, hz: u32) -> Result<u32, String> {
    mudar_hz(dispositivo, hz, false)
}

/// Todo o caminho de [`aplicar_hz`] até o `CDS_TEST`, sem mexer na tela. Só em teste: nada em produção chama.
#[cfg(all(test, target_os = "windows"))]
pub fn ensaiar_hz(dispositivo: &str, hz: u32) -> Result<u32, String> {
    mudar_hz(dispositivo, hz, true)
}

#[cfg(target_os = "windows")]
fn mudar_hz(dispositivo: &str, hz: u32, apenas_ensaio: bool) -> Result<u32, String> {
    use windows_sys::Win32::Graphics::Gdi::{
        ChangeDisplaySettingsExW, EnumDisplaySettingsExW, CDS_TEST, CDS_UPDATEREGISTRY, DEVMODEW,
        DISP_CHANGE_SUCCESSFUL, DM_BITSPERPEL, DM_DISPLAYFREQUENCY, DM_PELSHEIGHT, DM_PELSWIDTH,
        ENUM_CURRENT_SETTINGS,
    };

    let alvo = monitores()
        .into_iter()
        .find(|m| m.dispositivo == dispositivo)
        .ok_or_else(|| {
            format!(
                "Não encontrei o monitor `{}`. Ele pode ter sido desconectado \
                 depois do diagnóstico.",
                dispositivo
            )
        })?;

    if alvo.hz_atual == hz {
        return Ok(hz);
    }

    // Uma troca de cabo entre o diagnóstico e o clique viraria pedido de frequência inexistente.
    if !alvo.hz_disponiveis.contains(&hz) {
        return Err(format!(
            "{} não aceita {} Hz em {}x{}. As taxas disponíveis agora são: {}.",
            alvo.descricao,
            hz,
            alvo.largura,
            alvo.altura,
            alvo
                .hz_disponiveis
                .iter()
                .map(|v| format!("{} Hz", v))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    let mut nome: Vec<u16> = dispositivo.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        let mut modo: DEVMODEW = std::mem::zeroed();
        modo.dmSize = std::mem::size_of::<DEVMODEW>() as u16;

        if EnumDisplaySettingsExW(nome.as_mut_ptr(), ENUM_CURRENT_SETTINGS, &mut modo, 0) == 0 {
            return Err("Não consegui ler o modo de vídeo atual deste monitor.".to_string());
        }

        let anterior = modo.dmDisplayFrequency;

        modo.dmDisplayFrequency = hz;
        modo.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_BITSPERPEL | DM_DISPLAYFREQUENCY;

        let teste = ChangeDisplaySettingsExW(
            nome.as_mut_ptr(),
            &modo,
            std::ptr::null_mut(),
            CDS_TEST,
            std::ptr::null(),
        );

        if teste != DISP_CHANGE_SUCCESSFUL {
            return Err(explicar_recusa(teste, hz));
        }

        if apenas_ensaio {
            return Ok(anterior);
        }

        let feito = ChangeDisplaySettingsExW(
            nome.as_mut_ptr(),
            &modo,
            std::ptr::null_mut(),
            CDS_UPDATEREGISTRY,
            std::ptr::null(),
        );

        if feito != DISP_CHANGE_SUCCESSFUL {
            return Err(explicar_recusa(feito, hz));
        }

        Ok(anterior)
    }
}

#[cfg(not(target_os = "windows"))]
pub fn aplicar_hz(_dispositivo: &str, _hz: u32) -> Result<u32, String> {
    Err("Mudar a taxa do monitor só existe no Windows.".to_string())
}

/// O valor cru (`-2`, `-4`) não ajuda ninguém.
#[cfg(target_os = "windows")]
fn explicar_recusa(codigo: i32, hz: u32) -> String {
    let motivo = match codigo {
        -1 => "a placa de vídeo recusou o modo",
        -2 => "este monitor não aceita esta combinação",
        -3 => "não foi possível gravar a configuração no registro do Windows",
        -4 => "o driver de vídeo devolveu um erro",
        -5 => "o modo exige reiniciar o computador",
        _ => "o Windows recusou a mudança",
    };

    format!(
        "Não deu para colocar em {} Hz: {}. Nada foi alterado — a tela continua \
         como estava.",
        hz, motivo
    )
}

pub fn e_integrada(nome: &str) -> bool {
    let n = nome.to_lowercase();
    if n.contains("intel") {
        return !n.contains("arc");
    }
    // Ryzen com vídeo: "AMD Radeon(TM) Graphics", sem o "RX" das dedicadas.
    (n.contains("radeon") && n.contains("graphics") && !n.contains(" rx") && !n.contains("pro"))
        || n.contains("vega") && n.contains("graphics")
}

pub fn e_dedicada(nome: &str) -> bool {
    let n = nome.to_lowercase();
    n.contains("nvidia") || n.contains("radeon rx") || n.contains("radeon pro") || (n.contains("intel") && n.contains("arc"))
}

/// Notebook fica de fora: a tela interna passa pela integrada de propósito.
pub fn ligados_na_integrada(monitores: &[Monitor], placas: &[String], notebook: bool) -> Vec<DisplayFinding> {
    use super::achados::{FindingSeverity, FixLocation};
    if notebook {
        return Vec::new();
    }
    let Some(dedicada) = placas.iter().find(|p| e_dedicada(p)) else { return Vec::new() };
    monitores
        .iter()
        .filter(|m| e_integrada(&m.adaptador))
        .map(|m| DisplayFinding {
            id: format!("monitor_na_integrada_{}", m.dispositivo.replace(['\\', '.'], "")),
            dispositivo: m.dispositivo.clone(),
            hz_alvo: 0,
            title: format!("{} está ligado no vídeo da placa-mãe", m.descricao),
            measured: format!("O monitor recebe imagem de {}, e este PC tem {}.", m.adaptador, dedicada),
            advice: "O cabo está na saída da placa-mãe. Ou o jogo roda no vídeo do processador, que entrega \
                     uma fração da placa de vídeo, ou a placa desenha e o Windows copia cada quadro para \
                     essa saída, o que também custa desempenho e atraso. Passe o cabo para uma das saídas da \
                     placa de vídeo, aquelas mais embaixo na traseira do gabinete, deitadas."
                .to_string(),
            severity: FindingSeverity::Important,
            fix_location: FixLocation::Hardware,
        })
        .collect()
}

pub fn analyze() -> DisplayReport {
    let monitores = monitores();
    let mut findings = diagnosticar(&monitores);

    if monitores.iter().any(|m| e_integrada(&m.adaptador)) {
        let placas = super::gpupref::placas();
        let notebook = super::planoenergia::detectar().notebook;
        findings.extend(ligados_na_integrada(&monitores, &placas, notebook));
    }

    DisplayReport {
        monitores,
        findings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monitor(hz_atual: u32, disponiveis: &[u32]) -> Monitor {
        Monitor {
            dispositivo: r"\\.\DISPLAY1".to_string(),
            descricao: "Monitor de teste".to_string(),
            principal: true,
            largura: 1920,
            altura: 1080,
            hz_atual,
            hz_disponiveis: disponiveis.to_vec(),
            adaptador: "NVIDIA GeForce RTX 4060".to_string(),
        }
    }

    #[test]
    fn classifica_integrada_e_dedicada_pelo_nome() {
        assert!(e_integrada("Intel(R) UHD Graphics 630"));
        assert!(e_integrada("AMD Radeon(TM) Graphics"));
        assert!(e_integrada("AMD Radeon(TM) Vega 8 Graphics"));
        assert!(!e_integrada("AMD Radeon RX 7600"));
        assert!(!e_integrada("Intel(R) Arc(TM) A770 Graphics"));
        assert!(!e_integrada("NVIDIA GeForce RTX 4060"));
        assert!(e_dedicada("NVIDIA GeForce GTX 1650"));
        assert!(e_dedicada("AMD Radeon RX 6600"));
        assert!(e_dedicada("Intel(R) Arc(TM) A750 Graphics"));
        assert!(!e_dedicada("Intel(R) UHD Graphics 630"));
    }

    #[test]
    fn monitor_na_integrada_so_acusa_desktop_com_dedicada() {
        let mut m = monitor(144, &[60, 144]);
        m.adaptador = "Intel(R) UHD Graphics 630".to_string();
        let placas = vec!["Intel(R) UHD Graphics 630".to_string(), "NVIDIA GeForce RTX 3060".to_string()];

        let achados = ligados_na_integrada(&[m.clone()], &placas, false);
        assert_eq!(achados.len(), 1);
        assert!(achados[0].measured.contains("RTX 3060"));

        assert!(ligados_na_integrada(&[m.clone()], &placas, true).is_empty(), "notebook fica de fora");
        assert!(ligados_na_integrada(&[m.clone()], &placas[..1], false).is_empty(), "sem dedicada não há o que trocar");

        let certo = monitor(144, &[60, 144]);
        assert!(ligados_na_integrada(&[certo], &placas, false).is_empty());
    }

    /// Para no `CDS_TEST`, sem mudar a tela. `cargo test --lib -- --ignored ensaio_de_taxa --nocapture`
    #[test]
    #[ignore]
    #[cfg(target_os = "windows")]
    fn ensaio_de_taxa_nesta_maquina() {
        for m in monitores() {
            println!(
                "  {} [{}]  {}x{} @ {} Hz   disponíveis: {:?}",
                m.descricao, m.dispositivo, m.largura, m.altura, m.hz_atual, m.hz_disponiveis
            );

            // Numa máquina já ajustada, pedir o máximo sai pelo atalho: o ensaio pergunta por OUTRA frequência da lista.
            let alvo = if m.abaixo_do_maximo() {
                m.hz_maximo()
            } else {
                match m.hz_disponiveis.iter().rev().find(|hz| **hz != m.hz_atual) {
                    Some(outra) => {
                        println!(
                            "     já está no máximo; ensaiando {} Hz só para conferir                              a chamada",
                            outra
                        );
                        *outra
                    }
                    None => {
                        println!("     só existe uma frequência aqui
");
                        continue;
                    }
                }
            };

            match ensaiar_hz(&m.dispositivo, alvo) {
                Ok(anterior) => println!(
                    "     o driver ACEITA {} Hz (está em {}) — nada foi alterado
",
                    alvo, anterior
                ),
                Err(erro) => println!("     recusado: {}
", erro),
            }
        }
    }

    #[test]
    fn monitor_de_144_rodando_em_60_e_apontado() {
        let f = diagnosticar(&[monitor(60, &[60, 120, 144])]);

        assert_eq!(f.len(), 1);
        assert!(f[0].measured.contains("60 Hz e aceita até 144 Hz"));
    }

    #[test]
    fn nao_promete_fps_onde_o_ganho_e_de_fluidez() {
        let f = diagnosticar(&[monitor(60, &[60, 144])]);

        assert!(f[0].advice.contains("não espere um número de FPS maior"));
        assert!(f[0].advice.contains("não cria quadros"));
    }

    #[test]
    fn monitor_de_60_que_so_aceita_60_fica_calado() {
        assert!(diagnosticar(&[monitor(60, &[60])]).is_empty());
        assert!(diagnosticar(&[monitor(60, &[59, 60])]).is_empty());
    }

    #[test]
    fn diferenca_pequena_demais_nao_vira_achado() {
        assert!(diagnosticar(&[monitor(60, &[60, 61, 62])]).is_empty());
        assert!(diagnosticar(&[monitor(144, &[144, 150])]).is_empty());
    }

    #[test]
    fn ja_no_maximo_nao_vira_achado() {
        assert!(diagnosticar(&[monitor(144, &[60, 120, 144])]).is_empty());
    }

    #[test]
    fn cada_monitor_ganha_o_seu_achado() {
        let mut segundo = monitor(60, &[60, 165]);
        segundo.dispositivo = r"\\.\DISPLAY2".to_string();
        segundo.principal = false;

        let f = diagnosticar(&[monitor(60, &[60, 144]), segundo]);

        assert_eq!(f.len(), 2);
        // Senão o segundo monitor sobrescreve o primeiro no histórico.
        assert_ne!(f[0].id, f[1].id);
    }

    #[test]
    fn le_os_monitores_desta_maquina() {
        let r = analyze();

        for m in &r.monitores {
            println!(
                "  {} ({}) — {}x{} @ {} Hz · aceita {:?}{}",
                m.dispositivo,
                m.descricao,
                m.largura,
                m.altura,
                m.hz_atual,
                m.hz_disponiveis,
                if m.principal { " · principal" } else { "" }
            );
        }
        for f in &r.findings {
            println!("  [{:?}] {}", f.severity, f.measured);
        }

        // Zero monitores significa leitura falha.
        assert!(
            !r.monitores.is_empty(),
            "nenhum monitor lido — a enumeração falhou"
        );

        for m in &r.monitores {
            assert!(m.hz_atual > 0, "taxa atual zerada em {}", m.dispositivo);
            assert!(
                m.hz_disponiveis.contains(&m.hz_atual),
                "a taxa em uso ({} Hz) não apareceu na lista de modos aceitos — \
                 a enumeração está filtrando errado",
                m.hz_atual
            );
        }
    }
}
