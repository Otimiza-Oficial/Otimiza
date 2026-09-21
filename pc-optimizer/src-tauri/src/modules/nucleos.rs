// Os núcleos do processador, e a conta da máscara de afinidade
//
// POR QUE ISTO EXISTE
//
// Processador híbrido. Do 12ª geração da Intel em diante — e este produto roda
// num i9-14900HX — os núcleos NÃO SÃO IGUAIS: há os de desempenho, que são os
// rápidos, e os de eficiência, que são pequenos, lentos e existem para tarefa
// de fundo. Um jogo que cai nos de eficiência entrega muito menos quadro que a
// mesma máquina entregaria, e nada na tela do Windows diz que foi isso que
// aconteceu.
//
// Essa é a única coisa nesta área que vale mexer, e ela é MEDÍVEL: o uso por
// núcleo já está no contrato de telemetria, e a classe de cada núcleo o próprio
// Windows informa.
//
// O QUE ESTE MÓDULO NÃO PROMETE
//
// "Otimização de afinidade" é uma das áreas mais cheias de promessa vazia que
// existe, e três coisas precisam estar ditas antes de qualquer botão:
//
//   1. AFINIDADE NÃO É GANHO GRÁTIS. Tirar núcleos de um jogo que usa todos
//      eles REDUZ o que a máquina entrega. Prender o jogo em menos núcleos só
//      ajuda quando ele estava nos núcleos errados.
//
//   2. AFINIDADE MORRE COM O PROCESSO. Ela é uma propriedade do processo em
//      execução, não uma configuração do Windows: fechou o jogo, acabou. Um
//      produto que vende isso como ajuste permanente está vendendo uma coisa
//      que some sozinha.
//
//   3. EMPURRAR OS PROGRAMAS DE FUNDO PARA OUTROS NÚCLEOS quase não faz nada
//      no Windows moderno. O escalonador já evita o núcleo ocupado, e nos
//      híbridos o Thread Director faz isso em hardware. É o tipo de ajuste que
//      rende número de vídeo e não rende quadro.
//
// A CONTA DA MÁSCARA MORA AQUI, PURA
//
// Uma máscara de afinidade é um número onde cada bit é um núcleo lógico. Errar
// um deslocamento prende o jogo no núcleo errado — ou em nenhum, o que o
// Windows recusa com um erro que não explica nada. É aritmética, é fácil de
// errar, e por isso está separada da chamada do sistema e coberta por teste.

use serde::{Deserialize, Serialize};

/// A classe de um núcleo físico.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Classe {
    /// Os rápidos. É onde um jogo tem de estar.
    Desempenho,
    /// Os pequenos. Existem para tarefa de fundo.
    Eficiencia,
    /// Processador sem núcleos diferentes — a maioria até hoje.
    ///
    /// NÃO é "não descobri": é a resposta certa para um processador comum, e
    /// nele não há o que escolher.
    Uniforme,
}

/// Um núcleo lógico, do jeito que a tela e a máscara precisam.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NucleoLogico {
    /// O índice do bit na máscara de afinidade. É ele que o Windows usa.
    pub indice: u32,
    /// Qual núcleo físico ele pertence. Dois lógicos no mesmo físico são
    /// irmãos de SMT — o "hyper-threading".
    pub fisico: u32,
    pub classe: Classe,
}

/// O processador desta máquina.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Topologia {
    pub nucleos: Vec<NucleoLogico>,
    /// Este processador tem núcleos de classes diferentes.
    pub hibrido: bool,
}

impl Topologia {
    /// Quantos lógicos há.
    pub fn quantos(&self) -> usize {
        self.nucleos.len()
    }

    /// Quantos físicos há.
    pub fn fisicos(&self) -> usize {
        let mut vistos: Vec<u32> = self.nucleos.iter().map(|n| n.fisico).collect();
        vistos.sort_unstable();
        vistos.dedup();
        vistos.len()
    }

    /// Os lógicos de uma classe.
    pub fn da_classe(&self, classe: Classe) -> Vec<u32> {
        self.nucleos
            .iter()
            .filter(|n| n.classe == classe)
            .map(|n| n.indice)
            .collect()
    }
}

/// Monta uma máscara a partir dos índices.
///
/// `None` quando a lista está vazia ou tem índice acima de 63. Os dois casos
/// existem de verdade:
///
///   - Máscara vazia seria "nenhum núcleo", e o Windows recusa com um erro que
///     não explica nada. Recusar aqui dá uma frase que o cliente entende.
///   - Acima de 63 bits o Windows usa GRUPOS de processador, e uma máscara
///     simples não alcança. Máquina com mais de 64 lógicos é rara e existe;
///     fingir que o bit 64 cabe num `u64` prenderia o processo no núcleo 0.
pub fn mascara_de(indices: &[u32]) -> Option<u64> {
    if indices.is_empty() || indices.iter().any(|i| *i >= 64) {
        return None;
    }

    Some(indices.iter().fold(0u64, |m, i| m | (1u64 << i)))
}

/// Os índices de uma máscara.
pub fn indices_de(mascara: u64) -> Vec<u32> {
    (0..64).filter(|i| mascara & (1u64 << i) != 0).collect()
}

/// A máscara dos núcleos de desempenho, quando vale a pena usá-la.
///
/// `None` num processador uniforme, e é de propósito: ali não existe "núcleo
/// melhor", então prender o jogo em metade deles só tiraria metade da máquina.
/// Devolver uma máscara qualquer daria um botão que piora.
pub fn mascara_de_desempenho(t: &Topologia) -> Option<u64> {
    if !t.hibrido {
        return None;
    }

    let rapidos = t.da_classe(Classe::Desempenho);

    // Híbrido sem núcleo rápido nenhum não existe — mas se o Windows
    // responder assim, a resposta é não mexer.
    mascara_de(&rapidos)
}

/// O que a mudança de afinidade custa e rende, em palavras.
///
/// Sai JUNTO da máscara, nunca depois: quem lê o número de núcleos precisa ler,
/// na mesma tela, que isso some quando o jogo fechar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Conselho {
    pub cabe: bool,
    pub explicacao: String,
}

/// Vale mexer na afinidade desta máquina?
pub fn conselho(t: &Topologia) -> Conselho {
    if !t.hibrido {
        return Conselho {
            cabe: false,
            explicacao: format!(
                "Este processador tem {} núcleos lógicos, todos iguais. Não existe \"núcleo \
                 melhor\" para prender o jogo: tirar núcleos dele só reduziria o que a máquina \
                 entrega. Aqui a afinidade não tem o que resolver.",
                t.quantos()
            ),
        };
    }

    let rapidos = t.da_classe(Classe::Desempenho).len();
    let lentos = t.da_classe(Classe::Eficiencia).len();

    Conselho {
        cabe: true,
        explicacao: format!(
            "Este processador é híbrido: {rapidos} núcleos lógicos de desempenho e {lentos} de \
             eficiência. Um jogo que cai nos de eficiência entrega bem menos quadro, e nada na \
             tela do Windows diz que foi isso. Prender o jogo nos de desempenho corrige esse \
             caso — e só ele. A mudança vale para o processo aberto e some quando o jogo fechar."
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nucleo(indice: u32, fisico: u32, classe: Classe) -> NucleoLogico {
        NucleoLogico {
            indice,
            fisico,
            classe,
        }
    }

    /// Um híbrido pequeno: 2 físicos de desempenho com SMT (4 lógicos) e
    /// 4 de eficiência sem SMT.
    fn hibrido() -> Topologia {
        let mut nucleos = vec![
            nucleo(0, 0, Classe::Desempenho),
            nucleo(1, 0, Classe::Desempenho),
            nucleo(2, 1, Classe::Desempenho),
            nucleo(3, 1, Classe::Desempenho),
        ];
        for i in 0..4 {
            nucleos.push(nucleo(4 + i, 2 + i, Classe::Eficiencia));
        }

        Topologia {
            nucleos,
            hibrido: true,
        }
    }

    fn uniforme(quantos: u32) -> Topologia {
        Topologia {
            nucleos: (0..quantos)
                .map(|i| nucleo(i, i / 2, Classe::Uniforme))
                .collect(),
            hibrido: false,
        }
    }

    #[test]
    fn a_mascara_liga_o_bit_de_cada_nucleo() {
        assert_eq!(mascara_de(&[0]), Some(0b1));
        assert_eq!(mascara_de(&[0, 1]), Some(0b11));
        assert_eq!(mascara_de(&[3]), Some(0b1000));
        assert_eq!(mascara_de(&[0, 2, 4]), Some(0b10101));
    }

    /// Máscara vazia seria "nenhum núcleo", e o Windows recusa com um erro que
    /// não explica nada. Recusar aqui dá uma frase que o cliente entende.
    #[test]
    fn mascara_sem_nucleo_e_recusada() {
        assert_eq!(mascara_de(&[]), None);
    }

    /// Acima de 63 o Windows usa grupos de processador, e uma máscara simples
    /// não alcança. Fingir que o bit 64 cabe prenderia o processo no núcleo 0.
    #[test]
    fn indice_fora_do_alcance_e_recusado() {
        assert_eq!(mascara_de(&[64]), None);
        assert_eq!(mascara_de(&[0, 1, 100]), None);
        assert_eq!(mascara_de(&[63]), Some(1u64 << 63), "63 ainda cabe");
    }

    #[test]
    fn a_volta_da_mascara_devolve_os_mesmos_nucleos() {
        for indices in [vec![0u32], vec![0, 1, 2, 3], vec![5, 11, 30, 63]] {
            let m = mascara_de(&indices).expect("máscara");
            assert_eq!(indices_de(m), indices);
        }
    }

    #[test]
    fn a_topologia_conta_logicos_e_fisicos() {
        let t = hibrido();

        assert_eq!(t.quantos(), 8);
        assert_eq!(t.fisicos(), 6, "2 de desempenho com SMT + 4 de eficiência");
    }

    #[test]
    fn a_mascara_de_desempenho_pega_so_os_rapidos() {
        let m = mascara_de_desempenho(&hibrido()).expect("máscara");

        assert_eq!(indices_de(m), vec![0, 1, 2, 3]);
    }

    /// A recusa que impede um botão que piora.
    ///
    /// Num processador uniforme não existe núcleo melhor: prender o jogo em
    /// metade deles tira metade da máquina. Devolver uma máscara qualquer aqui
    /// daria exatamente esse botão.
    #[test]
    fn processador_uniforme_nao_ganha_mascara() {
        assert_eq!(mascara_de_desempenho(&uniforme(8)), None);

        let c = conselho(&uniforme(8));
        assert!(!c.cabe);
        assert!(c.explicacao.contains("todos iguais"), "{}", c.explicacao);
    }

    /// E o conselho do híbrido diz as duas coisas: o que corrige, e que some
    /// quando o jogo fechar.
    #[test]
    fn o_conselho_do_hibrido_diz_o_que_corrige_e_o_que_nao_dura() {
        let c = conselho(&hibrido());

        assert!(c.cabe);
        assert!(c.explicacao.contains("4 núcleos lógicos de desempenho"));
        assert!(c.explicacao.contains("some quando o jogo fechar"));
    }
}
