// Quem é esta máquina
//
// POR QUE O PRODUTO PRECISA DISSO
//
// A licença do Otimiza é conferida sem servidor. Isso levanta um problema que
// só tem uma saída honesta: sem alguém a quem perguntar, o programa não tem
// como saber que uma chave já foi usada em outro PC.
//
// A saída é a chave NASCER presa a uma máquina. O comprador manda este
// identificador junto com o pagamento, e a chave é assinada para ele. Repassar
// a chave no grupo não serve para ninguém, porque em outro PC ela não valida.
//
// O QUE SERVE DE IDENTIFICADOR, E O QUE NÃO SERVE
//
// `Win32_Processor.ProcessorId` é a armadilha clássica, e foi conferida nesta
// máquina: devolveu `BFEBFBFF000A0653`. Aquilo não é número de série — são as
// marcas de recurso do processador, iguais em TODO processador daquele modelo.
// Usar como identidade daria a mesma chave para milhares de PCs.
//
// O que sobra, em ordem de precedência:
//
//   1. Número de série da placa-mãe. Sobrevive à FORMATAÇÃO, que é o que o
//      público deste produto mais faz.
//   2. `MachineGuid` do registro. Sobrevive à troca de peça, mas morre na
//      formatação — por isso é reserva, e não primeira escolha.
//
// A CONSEQUÊNCIA QUE PRECISA ESTAR NA TELA
//
// Trocar a placa-mãe muda o identificador, e a chave para de valer. Isso não é
// defeito: é o preço de não ter servidor. O cliente precisa saber disso ANTES
// de comprar, não quando acontecer.

use serde::{Deserialize, Serialize};

/// De onde o identificador desta máquina saiu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Fonte {
    /// Número de série da placa-mãe. O melhor caso.
    PlacaMae,
    /// Identificador que o Windows cria na instalação. Reserva.
    Windows,
    /// Nenhuma das duas pôde ser lida.
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

    /// Se o identificador sobrevive a uma formatação.
    ///
    /// Muda o texto que a tela mostra: com a placa-mãe, formatar não custa
    /// chave nova; com o identificador do Windows, custa.
    pub fn sobrevive_formatacao(self) -> bool {
        matches!(self, Fonte::PlacaMae)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identidade {
    /// O código que o cliente copia e manda no Discord.
    pub id: String,
    pub fonte: Fonte,
}

/// Valores que fabricantes escrevem no lugar de um número de série de verdade.
///
/// Placa de PC montado costuma vir com série real. Notebook e máquina de
/// escritório costumam vir com um destes — e aceitar qualquer um deles daria o
/// MESMO identificador para milhares de máquinas diferentes, que é exatamente
/// o defeito que este módulo existe para não ter.
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

/// Um número de série serve como identidade?
///
/// **Função pura.** É a decisão mais importante do módulo e precisa ser
/// testável sem depender da placa-mãe de quem roda os testes.
pub fn serie_e_util(bruto: &str) -> bool {
    let limpo = bruto.trim().to_lowercase();

    if limpo.len() < 6 {
        return false;
    }

    if SERIE_SEM_VALOR.contains(&limpo.as_str()) {
        return false;
    }

    // Série que é só repetição de um caractere — "0000000000", "XXXXXXXX" — é
    // preenchimento de fábrica, não identidade.
    let primeiro = limpo.chars().next();
    if limpo.chars().all(|c| Some(c) == primeiro) {
        return false;
    }

    // Precisa misturar letra e número. Série de verdade mistura; preenchimento
    // de fábrica costuma ser só um ou só outro.
    limpo.chars().any(|c| c.is_ascii_digit()) && limpo.chars().any(|c| c.is_ascii_alphabetic())
}

/// Transforma o dado bruto no código que o cliente vê.
///
/// **Função pura.** O formato é `OTZ-XXXX-XXXX-XXXX`: curto o bastante para
/// alguém digitar no Discord sem errar, e agrupado porque bloco de quatro é o
/// que a pessoa consegue conferir de olho.
///
/// O alfabeto exclui as letras que se confundem com número — I, O, S, Z — pelo
/// mesmo motivo: este código vai ser copiado à mão por gente com pressa.
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

/// Resumo de 12 bytes do dado de origem.
///
/// Não é criptografia, e não precisa ser: aqui não há ninguém tentando forjar
/// colisão. A segurança da licença vem da ASSINATURA, não deste resumo.
///
/// O que ele entrega é tamanho fixo e o número de série do cliente não viajando
/// em texto puro quando ele mandar o código no Discord.
fn resumo(dados: &[u8]) -> [u8; 12] {
    let mut saida = [0u8; 12];

    // FNV-1a de 64 bits, três vezes com semente diferente.
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

// ------------------------------------------------------------------- leitura

/// O número de série da placa-mãe, quando ele presta.
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

/// O identificador que o Windows cria na instalação.
#[cfg(target_os = "windows")]
fn guid_do_windows() -> Option<String> {
    let bruto = crate::modules::windows::registry::read_text(
        "HKLM",
        "SOFTWARE\\Microsoft\\Cryptography",
        "MachineGuid",
    )?;

    let limpo = bruto.trim().to_string();
    (limpo.len() >= 32).then_some(limpo)
}

#[cfg(not(target_os = "windows"))]
fn guid_do_windows() -> Option<String> {
    None
}

/// Quem é esta máquina.
///
/// O resultado é guardado depois da primeira leitura: ele não muda enquanto o
/// programa estiver aberto, e a tela de licença pergunta mais de uma vez.
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

            // Sem identificar a máquina não há licença presa a ela. O produto
            // diz isso em vez de inventar um código que mudaria a cada abertura
            // e deixaria o cliente sem entender por que a chave dele parou de
            // funcionar.
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
        // A armadilha clássica, medida nesta máquina: `ProcessorId` devolveu
        // `BFEBFBFF000A0653`, que são as marcas de recurso do processador — o
        // MESMO valor em todo processador daquele modelo. Usar como identidade
        // daria a mesma chave para milhares de PCs.
        let producao = include_str!("maquina.rs").split("#[cfg(test)]").next().unwrap();

        // A conferência é pela CONSULTA, não pela palavra: o comentário no topo
        // do arquivo precisa continuar citando `Win32_Processor` para explicar
        // por que ele não serve, senão daqui a um ano alguém acrescenta
        // achando que é uma boa ideia.
        assert!(
            !producao.contains("Get-CimInstance Win32_Processor"),
            "o identificador do processador não é número de série: ele é igual em todo processador do mesmo modelo"
        );
    }

    #[test]
    fn serie_de_fabrica_nao_e_identidade() {
        // Aceitar qualquer um destes daria o MESMO identificador para milhares
        // de máquinas — o defeito exato que este módulo existe para não ter.
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
        // O valor real lido da placa MSI desta máquina.
        assert!(serie_e_util("07D8211_M31E600685"));
        assert!(serie_e_util("PF2K9L7X"));
        assert!(serie_e_util("5CD1234ABC"));
    }

    #[test]
    fn o_codigo_e_legivel_e_estavel() {
        let a = codificar("placa:07D8211_M31E600685");
        let b = codificar("placa:07D8211_M31E600685");

        // Estável: a mesma máquina precisa dar o mesmo código sempre, ou a
        // chave do cliente para de funcionar sozinha.
        assert_eq!(a, b);

        assert!(a.starts_with("OTZ-"));
        assert_eq!(a.len(), "OTZ-XXXX-XXXX-XXXX".len());

        // Nada de I, O, S ou Z no corpo: este código é copiado à mão por gente
        // com pressa, e essas letras se confundem com 1, 0, 5 e 2.
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
        // Sem o prefixo, uma placa e um GUID com o mesmo texto dariam o mesmo
        // código — improvável, e barato de impedir.
        assert_ne!(codificar("placa:abc123"), codificar("windows:abc123"));
    }

    #[test]
    fn o_codigo_nao_carrega_o_numero_de_serie() {
        // O cliente vai mandar isto no Discord. O número de série da placa dele
        // não precisa viajar junto.
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
            // Duas chamadas seguidas precisam dar o mesmo resultado.
            assert_eq!(quem.id, identidade().id);
        }
    }
}
