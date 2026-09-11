// Ajustes por dispositivo
//
// As duas otimizações mais profundas do catálogo moram aqui, e nenhuma delas
// pode ser escrita como um caminho fixo de registro: o identificador do
// dispositivo muda de PC para PC. É preciso enumerar e descobrir qual é qual.
//
// - MSI (Message Signaled Interrupts) na placa de vídeo: muda COMO a GPU avisa a
//   CPU de que terminou algo. No modo antigo, por linha de interrupção, o aviso
//   disputa uma fila compartilhada com outros dispositivos. Com MSI cada aviso é
//   direto. É latência de verdade, e é o tipo de ajuste que quase nenhum
//   "otimizador" faz porque dá trabalho.
//
// - Economia de energia da placa de rede: o Windows pode desligar a placa para
//   poupar energia. Ao acordar, o primeiro pacote atrasa — e é isso que aparece
//   como pico de ping no meio da partida.

use super::registry;
use crate::modules::changelog::{ChangeRecord, PreviousValue};

const PCI_ENUM: &str = r"SYSTEM\CurrentControlSet\Enum\PCI";
/// Classe "Adaptadores de rede" do Windows. O GUID é fixo em qualquer instalação.
const NET_CLASS: &str =
    r"SYSTEM\CurrentControlSet\Control\Class\{4d36e972-e325-11ce-bfc1-08002be10318}";

/// Serviços de driver de vídeo conhecidos, em minúsculas.
/// Identificar a GPU pelo driver é mais confiável que por nome comercial, que
/// varia com o modelo e o idioma.
const DRIVERS_DE_VIDEO: [&str; 6] = [
    "nvlddmkm", // NVIDIA
    "amdkmdag", // AMD moderna
    "amdkmdap", // AMD
    "igfx",     // Intel integrada
    "igfxn",    // Intel
    "iigd",     // Intel
];

/// Caminhos de "Device Parameters" das placas de vídeo instaladas.
///
/// A árvore é PCI\<dispositivo>\<instância>, então são dois níveis de
/// enumeração. Um dispositivo sem `Service` reconhecido é ignorado — mexer em
/// interrupção do dispositivo errado é o tipo de erro que trava o boot.
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

            // Dispositivo cujo `Service` não dá para ler fica de fora, e AQUI esse
            // é o lado seguro: a lista serve para escrever, e escrever na
            // interrupção de um dispositivo que não se identificou é o erro que
            // trava o boot. A enumeração passa por todo dispositivo PCI da
            // máquina, e uma ponte ilegível não pode derrubar a placa de vídeo.
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

/// Se todas as GPUs encontradas já estão em modo MSI.
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

/// Liga o modo MSI em cada placa de vídeo encontrada.
/// Liga o MSI das placas de vídeo, acumulando no histórico do chamador.
///
/// **Recebe o vetor em vez de devolvê-lo.** Devolvendo `Result<Vec<_>>`, uma
/// falha na segunda placa descartaria o registro da primeira — que JÁ foi
/// gravada no registro do cliente — e a reversão automática não teria como
/// desfazê-la. O sistema ficaria pela metade sem rastro, exatamente o que a
/// regra nº 2 do `mod.rs` proíbe.
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

/// Valor de `PnPCapabilities` que desliga o gerenciamento de energia da placa.
///
/// 24 = 0x18: soma de "não desligar o dispositivo para economizar energia" (8) e
/// "não permitir que ele acorde o computador" (16).
const PNP_SEM_ECONOMIA: u32 = 24;

/// Caminhos das placas de rede FÍSICAS instaladas.
///
/// A classe de rede do Windows lista muito mais que placas: WAN Miniports do
/// VPN, adaptadores virtuais do Hyper-V, o adaptador do depurador de kernel.
/// Nenhum deles tem energia para economizar, e escrever neles seria uma mexida
/// inútil no registro de dez dispositivos.
///
/// O `ComponentId` separa os dois mundos: dispositivo físico começa com o
/// barramento (`pci\`, `usb\`), enquanto os virtuais da Microsoft começam com
/// `ms_` ou `vms_`.
///
/// `Err` quando a classe ou o `ComponentId` de um adaptador não dá para ler. Até
/// a 2.0 isso virava "não é placa física": a placa de verdade do cliente era
/// descartada como virtual, e a otimização de rede dela sumia sem uma palavra.
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

/// Só as subchaves numeradas (`0000`, `0001`…) são adaptadores.
///
/// A classe tem também `Properties`, que o Windows não deixa ler. Enquanto a
/// leitura ilegível virava "não é placa", ela saía da lista sozinha; com o erro
/// passando a subir, lê-la como adaptador faria TODA máquina dar erro.
fn e_indice_de_adaptador(nome: &str) -> bool {
    !nome.is_empty() && nome.chars().all(|c| c.is_ascii_digit())
}

/// Se o adaptador está num barramento físico. Sem `ComponentId` nenhum, a
/// resposta é não: dispositivo que não declara barramento não é placa que se
/// possa ajustar.
fn e_placa_fisica(caminho: &str) -> Result<bool, String> {
    let componente = registry::read_text("HKLM", caminho, "ComponentId")?
        .unwrap_or_default()
        .to_lowercase();

    Ok(componente.starts_with("pci\\") || componente.starts_with("usb\\"))
}

pub fn economia_de_energia_da_rede_desligada() -> Option<bool> {
    // A lista que não deu para ler vira `None`, como a máquina sem placa: a
    // leitura que falhou não vira resposta. Quem aplica (`desligar_…`) recebe o
    // erro de verdade.
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

/// Desliga a economia de energia das placas de rede, acumulando no histórico do
/// chamador.
///
/// Mesma razão do `ativar_msi`, e aqui o risco é maior: uma máquina comum tem
/// vários adaptadores — cinco na máquina de desenvolvimento —, então uma falha
/// no terceiro deixaria dois já alterados sem registro para desfazer. E é
/// justamente a otimização que mexe em placa de rede de cliente.
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
    fn encontra_as_placas_de_video_desta_maquina() {
        let caminhos = caminhos_msi_das_gpus();

        for caminho in &caminhos {
            println!("GPU: {}", caminho);
        }

        // Todo caminho precisa terminar na chave certa: escrever `MSISupported`
        // no lugar errado é mexer em interrupção de dispositivo alheio.
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

        // A conferência é pelo ComponentId, não pelo nome do driver.
        //
        // A primeira versão deste teste rejeitava qualquer nome contendo
        // "virtual", e quebrou numa máquina virtual da Azure: a placa de lá se
        // chama "Mellanox ConnectX Virtual Ethernet Adapter" e é um dispositivo
        // PCI de verdade, com energia real para gerenciar. Nome é marketing;
        // o barramento é fato — a mesma lição do `TIPO_DE_INÍCIO`.
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

            // Os que motivaram o filtro — WAN Miniport de VPN, comutador do
            // Hyper-V, depurador de kernel — são todos `ms_*` ou `vms_*`.
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
        // `Properties` é negada pelo Windows a qualquer um; lida como
        // adaptador, derrubaria a lista de toda máquina.
        assert!(!e_indice_de_adaptador("Properties"));
        assert!(!e_indice_de_adaptador("Configuration"));
        assert!(!e_indice_de_adaptador(""));
    }

    #[test]
    fn valor_de_pnp_desliga_economia_e_despertar() {
        // 8 = não desligar para economizar energia; 16 = não deixar acordar o PC.
        assert_eq!(PNP_SEM_ECONOMIA, 8 | 16);
    }
}
