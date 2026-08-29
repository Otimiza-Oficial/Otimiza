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
    /// O monitor a corrigir, no formato `\.\DISPLAY1`. É o que a API pede de
    /// volta na hora de mudar o modo, e é o que vai no botão do diagnóstico.
    pub dispositivo: String,
    /// A frequência que o botão vai aplicar.
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

// ------------------------------------------------------------------ aplicar

/// Coloca um monitor na maior frequência que ele aceita na resolução atual.
///
/// Devolve a frequência ANTERIOR, que é o que o histórico precisa guardar para
/// saber voltar.
///
/// POR QUE O TESTE ANTES
///
/// Errar um modo de vídeo apaga a tela, e uma tela apagada é o pior defeito que
/// um otimizador pode causar: o cliente não consegue nem desfazer, porque não
/// enxerga o botão. Por isso são duas chamadas.
///
/// A primeira, com `CDS_TEST`, pergunta ao driver se o modo é aceito e não muda
/// nada. Só depois de ela aprovar é que a segunda aplica de verdade. É a mesma
/// sequência que a janela de configuração do próprio Windows usa.
///
/// A segurança de fundo é a mesma do módulo inteiro: só é oferecida frequência
/// que veio de `EnumDisplaySettingsExW` na resolução e profundidade de cor
/// atuais. A função que lista é a mesma que aplica — se ela listou, o modo
/// existe.
#[cfg(target_os = "windows")]
pub fn aplicar_hz(dispositivo: &str, hz: u32) -> Result<u32, String> {
    mudar_hz(dispositivo, hz, false)
}

/// Faz tudo que [`aplicar_hz`] faz — inclusive perguntar ao driver se o modo é
/// aceito — e para antes de mexer na tela.
///
/// Existe para que o caminho inteiro possa ser conferido numa máquina de
/// verdade sem apagar a tela de ninguém.
///
/// Só em compilação de teste: nada em produção chama, e código que existe "por
/// via das dúvidas" é código que ninguém executa e ninguém mantém. No dia em
/// que a tela quiser conferir antes de oferecer o botão, o `cfg` sai.
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

    // A conferência que impede o produto de pedir ao driver um modo que ele não
    // ofereceu. Sem ela, uma mudança de cabo entre o diagnóstico e o clique
    // viraria uma tentativa de aplicar frequência inexistente.
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

/// Traduz o código de recusa do Windows para o que o cliente precisa saber.
///
/// O valor cru (`-2`, `-4`) não ajuda ninguém, e é o que a maioria dos
/// programas mostra.
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

    /// Ensaio na máquina de quem roda o teste. Ignorado por padrão porque
    /// depende do monitor que estiver ligado ali.
    ///
    ///     cargo test --lib -- --ignored ensaio_de_taxa --nocapture
    ///
    /// Ele NÃO muda a tela: para no `CDS_TEST`, que é a pergunta ao driver.
    /// Serve para provar que o caminho inteiro — encontrar o monitor, montar o
    /// modo, falar com o Windows — funciona antes de alguém clicar no botão.
    #[test]
    #[ignore]
    #[cfg(target_os = "windows")]
    fn ensaio_de_taxa_nesta_maquina() {
        for m in monitores() {
            println!(
                "  {} [{}]  {}x{} @ {} Hz   disponíveis: {:?}",
                m.descricao, m.dispositivo, m.largura, m.altura, m.hz_atual, m.hz_disponiveis
            );

            // Numa máquina já ajustada — que é o caso depois que o produto
            // funciona — pedir o máximo sai pelo atalho e não exercita nada.
            // Então o ensaio pergunta por OUTRA frequência da lista. A chamada
            // ao driver é a mesma; só o número muda, e nada é aplicado.
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
