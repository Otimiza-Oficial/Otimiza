// Ajustes por dispositivo, que exigem enumerar (o identificador muda de PC para PC): MSI na placa de vídeo
// (interrupção direta, e não por linha compartilhada) e a economia de energia da placa de rede (acordar atrasa o
// primeiro pacote, o pico de ping no meio da partida).

use super::registry;
use crate::modules::changelog::{ChangeRecord, PreviousValue};

const PCI_ENUM: &str = r"SYSTEM\CurrentControlSet\Enum\PCI";
const NET_CLASS: &str =
    r"SYSTEM\CurrentControlSet\Control\Class\{4d36e972-e325-11ce-bfc1-08002be10318}";

/// Pelo driver, mais confiável que o nome comercial, que muda com o modelo e o idioma.
const DRIVERS_DE_VIDEO: [&str; 6] = [
    "nvlddmkm", // NVIDIA
    "amdkmdag", // AMD moderna
    "amdkmdap", // AMD
    "igfx",     // Intel integrada
    "igfxn",    // Intel
    "iigd",     // Intel
];

/// Dispositivo sem `Service` reconhecido é ignorado: mexer na interrupção do dispositivo errado trava o boot.
pub fn caminhos_msi_das_gpus() -> Vec<String> {
    let mut caminhos = Vec::new();

    let dispositivos = match registry::subkeys("HKLM", PCI_ENUM) {
        Ok(lista) => lista,
        Err(_) => return caminhos,
    };

    for dispositivo in dispositivos {
        let caminho_dispositivo = format!("{}\\{}", PCI_ENUM, dispositivo);

        let instancias = match registry::subkeys("HKLM", &caminho_dispositivo) {
            Ok(lista) => lista,
            Err(_) => continue,
        };

        for instancia in instancias {
            let caminho = format!("{}\\{}", caminho_dispositivo, instancia);

            // `Service` ilegível fica de fora, e aqui esse é o lado seguro: a lista serve para escrever.
            let servico = match registry::read_text("HKLM", &caminho, "Service") {
                Ok(Some(servico)) => servico.to_lowercase(),
                Ok(None) | Err(_) => continue,
            };

            if servico.is_empty() || !DRIVERS_DE_VIDEO.iter().any(|d| servico.starts_with(d)) {
                continue;
            }

            caminhos.push(format!(
                "{}\\Device Parameters\\Interrupt Management\\MessageSignaledInterruptProperties",
                caminho
            ));
        }
    }

    caminhos
}

pub fn msi_ja_ativo() -> Option<bool> {
    let caminhos = caminhos_msi_das_gpus();

    if caminhos.is_empty() {
        return None;
    }

    Some(caminhos.iter().all(|caminho| {
        matches!(
            registry::read("HKLM", caminho, "MSISupported"),
            Ok(PreviousValue::Dword(1))
        )
    }))
}

/// Recebe o vetor em vez de devolvê-lo: uma falha na segunda placa descartaria o registro da primeira, já gravada.
pub fn ativar_msi(mudancas: &mut Vec<ChangeRecord>) -> Result<(), String> {
    let caminhos = caminhos_msi_das_gpus();

    if caminhos.is_empty() {
        return Err("Nenhuma placa de vídeo reconhecida para este ajuste.".to_string());
    }

    for caminho in caminhos {
        let anterior = registry::set_dword("HKLM", &caminho, "MSISupported", 1)?;
        mudancas.push(ChangeRecord::RegistryValue {
            hive: "HKLM".to_string(),
            path: caminho,
            name: "MSISupported".to_string(),
            previous: anterior,
        });
    }

    Ok(())
}

// MSI por dispositivo: SÓ LEITURA. Ligar MSI num driver que não aguenta trava o boot, e aí o Otimiza nem abre
// para desfazer. Só a placa de vídeo passa no "como desfazer?".

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct DispositivoMsi {
    pub nome: String,
    pub servico: String,
    /// `None`: o driver não declara MSI (a chave não existe).
    pub msi: Option<bool>,
    pub placa_de_video: bool,
}

/// `FriendlyName`, senão o que vem depois do último `;` do `DeviceDesc`.
pub fn nome_legivel(amigavel: Option<&str>, descricao: Option<&str>) -> Option<String> {
    let bruto = amigavel.map(str::trim).filter(|a| !a.is_empty()).or(descricao.map(str::trim))?;
    let nome = resolver_nome(bruto);
    (!nome.is_empty()).then_some(nome)
}

/// Os dois formatos: `@recurso;Nome` e o molde com argumentos `@recurso;%1 USB %2 Controller;(Intel(R),3.20)`.
fn resolver_nome(bruto: &str) -> String {
    let partes: Vec<&str> = bruto.split(';').collect();
    let ultima = partes.last().copied().unwrap_or(bruto).trim();
    if partes.len() >= 3 && ultima.starts_with('(') && ultima.ends_with(')') {
        let mut molde = partes[partes.len() - 2].trim().to_string();
        let argumentos = &ultima[1..ultima.len() - 1];
        // "Intel(R)" tem parêntese dentro: separa só nas vírgulas.
        for (i, arg) in argumentos.split(',').collect::<Vec<_>>().iter().enumerate().rev() {
            molde = molde.replace(&format!("%{}", i + 1), arg.trim());
        }
        return molde;
    }
    ultima.to_string()
}

pub fn msi_por_dispositivo() -> Result<Vec<DispositivoMsi>, String> {
    let dispositivos = registry::subkeys("HKLM", PCI_ENUM)?;
    let mut lista = Vec::new();
    for dispositivo in dispositivos {
        let caminho_dispositivo = format!("{}\\{}", PCI_ENUM, dispositivo);
        let Ok(instancias) = registry::subkeys("HKLM", &caminho_dispositivo) else { continue };
        for instancia in instancias {
            let caminho = format!("{}\\{}", caminho_dispositivo, instancia);
            let Ok(Some(servico)) = registry::read_text("HKLM", &caminho, "Service") else { continue };
            if servico.trim().is_empty() {
                continue;
            }
            let amigavel = registry::read_text("HKLM", &caminho, "FriendlyName").ok().flatten();
            let descricao = registry::read_text("HKLM", &caminho, "DeviceDesc").ok().flatten();
            let Some(nome) = nome_legivel(amigavel.as_deref(), descricao.as_deref()) else { continue };
            let chave = format!("{}\\Device Parameters\\Interrupt Management\\MessageSignaledInterruptProperties", caminho);
            let msi = match registry::read("HKLM", &chave, "MSISupported") {
                Ok(PreviousValue::Dword(v)) => Some(v != 0),
                _ => None,
            };
            let s = servico.to_lowercase();
            lista.push(DispositivoMsi {
                placa_de_video: DRIVERS_DE_VIDEO.iter().any(|d| s.starts_with(d)),
                nome,
                servico,
                msi,
            });
        }
    }
    lista.sort_by(|a, b| a.nome.cmp(&b.nome));
    lista.dedup_by(|a, b| a.nome == b.nome && a.servico == b.servico && a.msi == b.msi);
    Ok(lista)
}

/// 24 = 0x18: não desligar para economizar (8) + não acordar o computador (16).
const PNP_SEM_ECONOMIA: u32 = 24;

/// Só as FÍSICAS, pelo `ComponentId` (`pci\`, `usb\`; os virtuais são `ms_` ou `vms_`). `Err` quando não se
/// lê: "não é placa física" descartava a placa de verdade do cliente.
pub fn caminhos_das_placas_de_rede() -> Result<Vec<String>, String> {
    let mut caminhos = Vec::new();

    for indice in registry::subkeys("HKLM", NET_CLASS)?
        .into_iter()
        .filter(|indice| e_indice_de_adaptador(indice))
    {
        let caminho = format!("{}\\{}", NET_CLASS, indice);

        if e_placa_fisica(&caminho)? {
            caminhos.push(caminho);
        }
    }

    Ok(caminhos)
}

/// Só `0000`, `0001`...: `Properties` o Windows não deixa ler, e lida como adaptador daria erro em toda máquina.
fn e_indice_de_adaptador(nome: &str) -> bool {
    !nome.is_empty() && nome.chars().all(|c| c.is_ascii_digit())
}

fn e_placa_fisica(caminho: &str) -> Result<bool, String> {
    let componente = registry::read_text("HKLM", caminho, "ComponentId")?
        .unwrap_or_default()
        .to_lowercase();

    Ok(componente.starts_with("pci\\") || componente.starts_with("usb\\"))
}

pub fn economia_de_energia_da_rede_desligada() -> Option<bool> {
    let caminhos = caminhos_das_placas_de_rede().ok()?;

    if caminhos.is_empty() {
        return None;
    }

    Some(caminhos.iter().all(|caminho| {
        matches!(
            registry::read("HKLM", caminho, "PnPCapabilities"),
            Ok(PreviousValue::Dword(v)) if v == PNP_SEM_ECONOMIA
        )
    }))
}

/// Mesma razão do `ativar_msi`, com risco maior: uma máquina tem vários adaptadores.
pub fn desligar_economia_de_energia_da_rede(
    mudancas: &mut Vec<ChangeRecord>,
) -> Result<(), String> {
    let caminhos = caminhos_das_placas_de_rede()?;

    if caminhos.is_empty() {
        return Err("Nenhuma placa de rede encontrada.".to_string());
    }

    for caminho in caminhos {
        let anterior = registry::set_dword("HKLM", &caminho, "PnPCapabilities", PNP_SEM_ECONOMIA)?;
        mudancas.push(ChangeRecord::RegistryValue {
            hive: "HKLM".to_string(),
            path: caminho,
            name: "PnPCapabilities".to_string(),
            previous: anterior,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn o_nome_vem_do_amigavel_ou_do_fim_da_descricao() {
        assert_eq!(
            nome_legivel(None, Some("@oem11.inf,%rtl8168.devicedesc%;Realtek PCIe GbE Family Controller")).as_deref(),
            Some("Realtek PCIe GbE Family Controller")
        );
        assert_eq!(nome_legivel(Some(" Placa X "), Some("@a;Y")).as_deref(), Some("Placa X"));
        assert_eq!(nome_legivel(Some(""), Some("Controlador")).as_deref(), Some("Controlador"));
        assert_eq!(nome_legivel(None, None), None);
        assert_eq!(
            nome_legivel(
                Some(r"@System32\drivers\usbxhci.sys,#1073807361;%1 USB %2 eXtensible Host Controller - %3 (Microsoft);(Intel(R),3.20,1.20)"),
                None
            )
            .as_deref(),
            Some("Intel(R) USB 3.20 eXtensible Host Controller - 1.20 (Microsoft)")
        );
    }

    #[test]
    fn encontra_as_placas_de_video_desta_maquina() {
        let caminhos = caminhos_msi_das_gpus();

        for caminho in &caminhos {
            println!("GPU: {}", caminho);
        }

        for caminho in &caminhos {
            assert!(caminho.ends_with("MessageSignaledInterruptProperties"));
            assert!(caminho.starts_with(PCI_ENUM));
        }
    }

    #[test]
    fn encontra_as_placas_de_rede_desta_maquina() {
        let caminhos = caminhos_das_placas_de_rede()
            .expect("as placas de rede desta máquina precisam ser legíveis");

        for caminho in &caminhos {
            let nome = registry::read_text("HKLM", caminho, "DriverDesc")
                .ok()
                .flatten()
                .unwrap_or_default();
            println!("Rede: {} — {}", nome, caminho);
        }

        // Pelo ComponentId, não pelo nome: a placa da Azure se chama "...Virtual Ethernet Adapter" e é PCI de verdade.
        for caminho in &caminhos {
            let componente = registry::read_text("HKLM", caminho, "ComponentId")
                .expect("o ComponentId de uma placa da lista foi lido para ela entrar")
                .unwrap_or_default()
                .to_lowercase();

            assert!(
                componente.starts_with("pci\\") || componente.starts_with("usb\\"),
                "entrou na lista algo que não está num barramento físico: {}",
                componente
            );

            assert!(
                !componente.starts_with("ms_") && !componente.starts_with("vms_"),
                "adaptador virtual da Microsoft entrou na lista: {}",
                componente
            );
        }
    }

    #[test]
    fn so_subchave_numerada_e_adaptador() {
        assert!(e_indice_de_adaptador("0000"));
        assert!(e_indice_de_adaptador("0012"));
        assert!(!e_indice_de_adaptador("Properties"));
        assert!(!e_indice_de_adaptador("Configuration"));
        assert!(!e_indice_de_adaptador(""));
    }

    #[test]
    fn valor_de_pnp_desliga_economia_e_despertar() {
        assert_eq!(PNP_SEM_ECONOMIA, 8 | 16);
    }
}

#[cfg(test)]
mod nesta_maquina {
    #[test]
    #[ignore]
    fn msi_desta_maquina() {
        for d in super::msi_por_dispositivo().expect("leu") {
            println!("{:<55} {:<12} {:?}", d.nome, d.servico, d.msi);
        }
    }
}
