// Quem é esta máquina: a licença é conferida sem servidor, então a chave NASCE presa a um identificador.
// `Win32_Processor.ProcessorId` não serve (são marcas de recurso, iguais em todo processador do modelo). Série da
// placa-mãe primeiro (sobrevive à formatação); `MachineGuid` de reserva. Trocar a placa-mãe muda o código, e o
// cliente precisa saber disso antes de comprar.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Fonte {
    PlacaMae,
    Windows,
    Nenhuma,
}

impl Fonte {
    pub fn descricao(self) -> &'static str {
        match self {
            Fonte::PlacaMae => "número de série da placa-mãe",
            Fonte::Windows => "identificador da instalação do Windows",
            Fonte::Nenhuma => "não foi possível identificar esta máquina",
        }
    }

    /// Muda o texto da tela: com a placa-mãe, formatar não custa chave nova.
    pub fn sobrevive_formatacao(self) -> bool {
        matches!(self, Fonte::PlacaMae)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identidade {
    pub id: String,
    pub fonte: Fonte,
}

/// Preenchimentos de fábrica: aceitá-los daria o MESMO identificador para milhares de máquinas.
const SERIE_SEM_VALOR: &[&str] = &[
    "default string",
    "to be filled by o.e.m.",
    "to be filled by o.e.m",
    "none",
    "n/a",
    "na",
    "not applicable",
    "system serial number",
    "0",
    "00000000",
    "123456789",
    "unknown",
    "invalid",
];

pub fn serie_e_util(bruto: &str) -> bool {
    let limpo = bruto.trim().to_lowercase();

    if limpo.len() < 6 {
        return false;
    }

    if SERIE_SEM_VALOR.contains(&limpo.as_str()) {
        return false;
    }

    // Repetição de um caractere é preenchimento de fábrica.
    let primeiro = limpo.chars().next();
    if limpo.chars().all(|c| Some(c) == primeiro) {
        return false;
    }

    // Série de verdade mistura letra e número.
    limpo.chars().any(|c| c.is_ascii_digit()) && limpo.chars().any(|c| c.is_ascii_alphabetic())
}

/// `OTZ-XXXX-XXXX-XXXX`: digitável sem errar. Sem I, O, S, Z, que se confundem com número.
pub fn codificar(bruto: &str) -> String {
    const ALFABETO: &[u8] = b"ABCDEFGHJKLMNPQRTUVWXY0123456789";

    let digest = resumo(bruto.trim().to_lowercase().as_bytes());

    let mut blocos = Vec::with_capacity(3);

    for pedaco in digest.chunks(4).take(3) {
        let letras: String = pedaco
            .iter()
            .map(|b| ALFABETO[(*b as usize) % ALFABETO.len()] as char)
            .collect();
        blocos.push(letras);
    }

    format!("OTZ-{}", blocos.join("-"))
}

/// Não é criptografia (a segurança vem da ASSINATURA): dá tamanho fixo e não manda o número de série em texto.
fn resumo(dados: &[u8]) -> [u8; 12] {
    let mut saida = [0u8; 12];

    for (rodada, pedaco) in saida.chunks_mut(4).enumerate() {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325 ^ (rodada as u64).wrapping_mul(0x9E37_79B9);

        for byte in dados {
            h ^= *byte as u64;
            h = h.wrapping_mul(0x100_0000_01b3);
        }

        pedaco.copy_from_slice(&(h as u32).to_le_bytes());
    }

    saida
}

#[cfg(target_os = "windows")]
fn serie_da_placa() -> Option<String> {
    let script = "(Get-CimInstance Win32_BaseBoard -ErrorAction SilentlyContinue).SerialNumber";

    let saida = crate::modules::windows::shell::powershell(script).ok()?;

    if !saida.success {
        return None;
    }

    let bruto = saida.stdout.trim().to_string();
    serie_e_util(&bruto).then_some(bruto)
}

#[cfg(not(target_os = "windows"))]
fn serie_da_placa() -> Option<String> {
    None
}

#[cfg(target_os = "windows")]
fn guid_do_windows() -> Option<String> {
    let bruto = crate::modules::windows::registry::read_text(
        "HKLM",
        "SOFTWARE\\Microsoft\\Cryptography",
        "MachineGuid",
    )
    .ok()
    .flatten()?;

    let limpo = bruto.trim().to_string();
    (limpo.len() >= 32).then_some(limpo)
}

#[cfg(not(target_os = "windows"))]
fn guid_do_windows() -> Option<String> {
    None
}

/// Guardado depois da primeira leitura: não muda com o programa aberto.
pub fn identidade() -> Identidade {
    use std::sync::OnceLock;

    static CACHE: OnceLock<Identidade> = OnceLock::new();

    CACHE
        .get_or_init(|| {
            if let Some(serie) = serie_da_placa() {
                return Identidade {
                    id: codificar(&format!("placa:{}", serie)),
                    fonte: Fonte::PlacaMae,
                };
            }

            if let Some(guid) = guid_do_windows() {
                return Identidade {
                    id: codificar(&format!("windows:{}", guid)),
                    fonte: Fonte::Windows,
                };
            }

            // Sem identificar a máquina não há licença: um código inventado mudaria a cada abertura.
            Identidade {
                id: String::new(),
                fonte: Fonte::Nenhuma,
            }
        })
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn o_identificador_do_processador_nunca_serve() {
        let producao = include_str!("maquina.rs").split("#[cfg(test)]").next().unwrap();

        // Pela CONSULTA, não pela palavra: o cabeçalho precisa continuar citando `Win32_Processor` para explicar por
        // que ele não serve.
        assert!(
            !producao.contains("Get-CimInstance Win32_Processor"),
            "o identificador do processador não é número de série: ele é igual em todo processador do mesmo modelo"
        );
    }

    #[test]
    fn serie_de_fabrica_nao_e_identidade() {
        for lixo in [
            "Default string",
            "To be filled by O.E.M.",
            "None",
            "N/A",
            "0",
            "00000000",
            "123456789",
            "XXXXXXXXXXXX",
            "",
            "   ",
            "ABC",
        ] {
            assert!(!serie_e_util(lixo), "`{}` não pode passar por identidade", lixo);
        }
    }

    #[test]
    fn serie_de_verdade_passa() {
        assert!(serie_e_util("07D8211_M31E600685"));
        assert!(serie_e_util("PF2K9L7X"));
        assert!(serie_e_util("5CD1234ABC"));
    }

    #[test]
    fn o_codigo_e_legivel_e_estavel() {
        let a = codificar("placa:07D8211_M31E600685");
        let b = codificar("placa:07D8211_M31E600685");

        // A mesma máquina precisa dar o mesmo código sempre.
        assert_eq!(a, b);

        assert!(a.starts_with("OTZ-"));
        assert_eq!(a.len(), "OTZ-XXXX-XXXX-XXXX".len());

        for proibida in ['I', 'O', 'S', 'Z'] {
            assert!(
                !a["OTZ-".len()..].contains(proibida),
                "`{}` se confunde com número: {}",
                proibida,
                a
            );
        }
    }

    #[test]
    fn maquinas_diferentes_dao_codigos_diferentes() {
        let um = codificar("placa:07D8211_M31E600685");
        let outro = codificar("placa:07D8211_M31E600686");

        assert_ne!(um, outro, "um dígito de diferença precisa mudar o código");
    }

    #[test]
    fn a_fonte_muda_o_codigo() {
        // Sem o prefixo, placa e GUID com o mesmo texto dariam o mesmo código.
        assert_ne!(codificar("placa:abc123"), codificar("windows:abc123"));
    }

    #[test]
    fn o_codigo_nao_carrega_o_numero_de_serie() {
        let serie = "07D8211_M31E600685";
        let codigo = codificar(&format!("placa:{}", serie));

        assert!(!codigo.contains("07D8211"));
        assert!(!codigo.contains("M31E600685"));
    }

    #[test]
    fn identifica_esta_maquina() {
        let quem = identidade();

        println!("  id    : {}", quem.id);
        println!("  fonte : {:?} — {}", quem.fonte, quem.fonte.descricao());
        println!("  sobrevive a formatação: {}", quem.fonte.sobrevive_formatacao());

        if quem.fonte != Fonte::Nenhuma {
            assert!(quem.id.starts_with("OTZ-"));
            assert_eq!(quem.id, identidade().id);
        }
    }
}
