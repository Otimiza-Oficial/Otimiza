// Núcleos e a máscara de afinidade. Em processador híbrido, jogo nos núcleos de eficiência entrega muito menos
// quadro, e isso é medível. Afinidade não é ganho grátis (tirar núcleo de jogo que usa todos reduz), morre com o
// processo, e empurrar programa de fundo para outro núcleo quase não faz nada no Windows moderno. A conta da
// máscara é pura e testada: errar um deslocamento prende o jogo no núcleo errado.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Classe {
    Desempenho,
    Eficiencia,
    /// NÃO é "não descobri": é a resposta certa para um processador comum.
    Uniforme,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NucleoLogico {
    pub indice: u32,
    /// Dois lógicos no mesmo físico são irmãos de SMT.
    pub fisico: u32,
    pub classe: Classe,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Topologia {
    pub nucleos: Vec<NucleoLogico>,
    pub hibrido: bool,
}

impl Topologia {
    pub fn quantos(&self) -> usize {
        self.nucleos.len()
    }

    pub fn fisicos(&self) -> usize {
        let mut vistos: Vec<u32> = self.nucleos.iter().map(|n| n.fisico).collect();
        vistos.sort_unstable();
        vistos.dedup();
        vistos.len()
    }

    pub fn da_classe(&self, classe: Classe) -> Vec<u32> {
        self.nucleos
            .iter()
            .filter(|n| n.classe == classe)
            .map(|n| n.indice)
            .collect()
    }
}

/// `None` com lista vazia (o Windows recusaria sem explicar) ou índice acima de 63 (acima disso são GRUPOS de
/// processador, e fingir que o bit 64 cabe prenderia o processo no núcleo 0).
pub fn mascara_de(indices: &[u32]) -> Option<u64> {
    if indices.is_empty() || indices.iter().any(|i| *i >= 64) {
        return None;
    }

    Some(indices.iter().fold(0u64, |m, i| m | (1u64 << i)))
}

pub fn indices_de(mascara: u64) -> Vec<u32> {
    (0..64).filter(|i| mascara & (1u64 << i) != 0).collect()
}

/// `None` em processador uniforme: não há núcleo melhor, e prender em metade só tiraria metade da máquina.
pub fn mascara_de_desempenho(t: &Topologia) -> Option<u64> {
    if !t.hibrido {
        return None;
    }

    let rapidos = t.da_classe(Classe::Desempenho);

    // Se o Windows responder assim, a resposta é não mexer.
    mascara_de(&rapidos)
}

/// Sai JUNTO da máscara: quem lê o número de núcleos precisa ler que isso some quando o jogo fechar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Conselho {
    pub cabe: bool,
    pub explicacao: String,
}

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

    #[test]
    fn mascara_sem_nucleo_e_recusada() {
        assert_eq!(mascara_de(&[]), None);
    }

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

    #[test]
    fn processador_uniforme_nao_ganha_mascara() {
        assert_eq!(mascara_de_desempenho(&uniforme(8)), None);

        let c = conselho(&uniforme(8));
        assert!(!c.cabe);
        assert!(c.explicacao.contains("todos iguais"), "{}", c.explicacao);
    }

    #[test]
    fn o_conselho_do_hibrido_diz_o_que_corrige_e_o_que_nao_dura() {
        let c = conselho(&hibrido());

        assert!(c.cabe);
        assert!(c.explicacao.contains("4 núcleos lógicos de desempenho"));
        assert!(c.explicacao.contains("some quando o jogo fechar"));
    }
}
