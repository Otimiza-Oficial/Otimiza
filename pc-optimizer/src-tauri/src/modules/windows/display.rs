// Monitor: resolução e taxa de atualização
//
// Monitor de 144Hz rodando a 60Hz é comum — acontece depois de troca de driver,
// de cabo, ou porque o Windows escolheu o modo conservador e ninguém notou. É
// a maior diferença de fluidez que existe num PC, e é invisível para quem não
// sabe onde olhar.
//
// TRÊS FONTES, DUAS ERRADAS — E ELAS ERRAM PARA LADOS OPOSTOS
//
// Escolher a fonte aqui é a decisão inteira deste módulo, e as duas fontes
// óbvias falham. Fica registrado porque quem vier depois vai tropeçar nas
// mesmas pedras.
//
// 1. `Win32_VideoController.MaxRefreshRate` — erra PARA MAIS. Informa o que a
//    PLACA consegue emitir, não o que o monitor mostra. Numa máquina com placa
//    boa e monitor simples, acusaria uma perda de fluidez que não existe.
//
// 2. `root\wmi WmiMonitorListedSupportedSourceModes` — erra PARA MENOS, e foi
//    a que quase me convenceu. Parece a fonte definitiva, porque vem da EDID
//    do próprio monitor. Mas ela só expõe as tabelas de temporização BÁSICAS
//    da EDID; os modos de alta taxa moram nos blocos de extensão, que essa
//    classe não devolve.
//
//    Na máquina onde este módulo foi escrito, ela informou 1920x1080 @ 60Hz
//    como único modo — para dois monitores AOC 24G4, que são painéis de 180Hz.
//    Confiar nela teria feito o produto ficar CALADO diante de dois monitores
//    de 180Hz rodando a 60, que é exatamente o achado mais valioso que ele tem
//    para dar.
//
// 3. `EnumDisplaySettingsExW` — a que este módulo usa. Enumera os modos que o
//    Windows aceita de fato para aquele monitor, com o driver e o cabo que
//    estão ali. É a MESMA função que aplicaria a mudança: se ela não lista, a
//    mudança não aconteceria; se lista, acontece.
//
// A lição, e ela vale para o produto inteiro: a fonte certa é a que decide o
// resultado, não a que parece mais oficial.
//
// E um cuidado a mais: a enumeração precisa fixar resolução e profundidade de
// cor nos valores atuais. Sem isso a lista vem cheia de modo de 8 bits e de
// resolução menor, e o produto ofereceria uma frequência que não existe na
// configuração em que a pessoa está.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Monitor {
    /// Nome interno do dispositivo, do tipo `\\.\DISPLAY1`. É o que a API pede
    /// de volta na hora de mudar o modo.
    pub dispositivo: String,
    /// Nome comercial do monitor ("AOC 24G4"), com o nome do adaptador como
    /// reserva quando a EDID não puder ser lida.
    pub descricao: String,
    pub principal: bool,
    pub largura: u32,
    pub altura: u32,
    pub hz_atual: u32,
    /// Frequências que o Windows aceita NESTA resolução, em ordem crescente.
    pub hz_disponiveis: Vec<u32>,
}

impl Monitor {
    /// A maior frequência disponível na resolução atual.
    pub fn hz_maximo(&self) -> u32 {
        self.hz_disponiveis.iter().copied().max().unwrap_or(self.hz_atual)
    }

    /// Está abaixo do que o monitor aceita nesta resolução.
    pub fn abaixo_do_maximo(&self) -> bool {
        self.hz_maximo() > self.hz_atual
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayFinding {
    pub id: String,
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

/// Abaixo disto não vale falar: 60 para 62 não é diferença que alguém sinta, e
/// alguns monitores anunciam 59 e 60 como modos distintos.
const DIFERENCA_QUE_IMPORTA: u32 = 15;

// ------------------------------------------------------------------- leitura

/// Nomes comerciais dos monitores, na ordem em que o Windows os enumera.
///
/// `EnumDisplayDevicesW` devolve o nome do ADAPTADOR ("NVIDIA GeForce GTX
/// 1650"), o mesmo para todos os monitores ligados nele. Num PC com dois
/// monitores isso faz o produto escrever a mesma frase duas vezes, e o cliente
/// não tem como saber de qual tela estamos falando.
///
/// O nome de verdade — "AOC 24G4" — está na EDID, e só sai por aqui. Falhar
/// nesta leitura não é grave: o nome do adaptador continua servindo de reserva.
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

            // Monitor desligado ou desconectado não interessa. E driver de
            // espelhamento não é monitor: é software de captura fingindo ser
            // um, e mexer no modo dele não muda nada na tela de ninguém.
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

            // Os modos aceitos NESTA resolução e NESTA profundidade de cor.
            // Sem esse filtro a lista vem poluída e o produto ofereceria uma
            // frequência que não existe na configuração em que a pessoa está.
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

                // 0 e 1 são códigos de "taxa padrão do hardware", não valores.
                if mesmo_modo && candidato.dmDisplayFrequency > 1 {
                    hz_disponiveis.push(candidato.dmDisplayFrequency);
                }
            }

            hz_disponiveis.sort_unstable();
            hz_disponiveis.dedup();

            // O nome comercial quando ele existe; o do adaptador como reserva.
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
            });
        }
    }

    encontrados
}

#[cfg(not(target_os = "windows"))]
pub fn monitores() -> Vec<Monitor> {
    Vec::new()
}

// --------------------------------------------------------------- diagnóstico

/// Regras puras, testáveis sem depender do monitor de quem roda os testes.
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
            title: "Monitor rodando abaixo da taxa que ele aceita".to_string(),
            measured: format!(
                "{} está em {} Hz e aceita até {} Hz em {}x{}.",
                monitor.descricao,
                monitor.hz_atual,
                maximo,
                monitor.largura,
                monitor.altura
            ),
            // A honestidade que ninguém escreve: subir a taxa não sobe o FPS.
            // Sobe o TETO. Se a placa entrega 70 quadros, continuam 70 — só que
            // agora eles aparecem quando ficam prontos, em vez de esperar a
            // tela. Prometer FPS aqui seria mentira fácil de vender.
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

pub fn analyze() -> DisplayReport {
    let monitores = monitores();
    let findings = diagnosticar(&monitores);

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
        // A mentira mais fácil de vender neste módulo seria "ganhe FPS
        // colocando o monitor em 144Hz". A taxa não cria quadro nenhum.
        let f = diagnosticar(&[monitor(60, &[60, 144])]);

        assert!(f[0].advice.contains("não espere um número de FPS maior"));
        assert!(f[0].advice.contains("não cria quadros"));
    }

    #[test]
    fn monitor_de_60_que_so_aceita_60_fica_calado() {
        // O caso da máquina onde este módulo foi escrito. A placa emite até
        // 180 Hz; os monitores aceitam 60. Falar aqui seria inventar um
        // problema que não existe.
        assert!(diagnosticar(&[monitor(60, &[60])]).is_empty());
        assert!(diagnosticar(&[monitor(60, &[59, 60])]).is_empty());
    }

    #[test]
    fn diferenca_pequena_demais_nao_vira_achado() {
        // Alguns monitores anunciam 59 e 60 como modos distintos, e uma tela de
        // 75 Hz rodando a 74 não é problema de ninguém.
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
        // Identificadores distintos: senão o segundo monitor sobrescreve o
        // primeiro no histórico e some da tela.
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

        // Toda máquina com tela tem pelo menos um monitor ligado. Zero aqui
        // significa que a leitura falhou, e falha silenciosa é o defeito que
        // este produto não pode ter.
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
