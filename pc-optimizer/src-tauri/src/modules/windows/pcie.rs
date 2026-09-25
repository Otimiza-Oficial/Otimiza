// As faixas do PCIe até a placa. Menos que o máximo é físico e custa quadro: slot de baixo, M.2 dividindo
// faixas, riser, encaixe sujo. A LARGURA é negociada no boot e vale a qualquer momento; a GERAÇÃO cai sozinha
// com a placa parada, e só é julgada com a placa sob carga (senão seria o alarme falso "sua placa está em PCIe
// 1.0"). Só NVIDIA (`nvidia-smi`); na AMD, "não deu para ler".

use serde::{Deserialize, Serialize};

/// Acima de 20% de carga a placa já saiu do repouso e subiu o enlace.
pub const CARGA_PARA_JULGAR_A_GERACAO: f64 = 20.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Enlace {
    pub geracao_atual: u32,
    pub geracao_maxima: u32,
    pub largura_atual: u32,
    pub largura_maxima: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "estado")]
pub enum Faixas {
    Completo { largura: u32 },
    Estreito { atual: u32, maxima: u32 },
    /// Em repouso este estado nunca é produzido.
    GeracaoAbaixoSobCarga { atual: u32, maxima: u32, largura: u32 },
    /// Não é "está tudo bem".
    NaoDeuParaLer { motivo: String },
}

/// `None` sem os quatro números: linha incompleta não vira "zero faixas".
pub fn ler_enlace(saida: &str) -> Option<Enlace> {
    let linha = saida.lines().find(|l| !l.trim().is_empty())?;

    let numeros: Vec<u32> = linha
        .split(',')
        .map(|campo| campo.trim())
        .map(|campo| campo.trim_start_matches('x'))
        .filter_map(|campo| campo.parse::<u32>().ok())
        .collect();

    if numeros.len() < 4 {
        return None;
    }

    if numeros[..4].iter().any(|n| *n == 0) {
        return None;
    }

    Some(Enlace {
        geracao_atual: numeros[0],
        geracao_maxima: numeros[1],
        largura_atual: numeros[2],
        largura_maxima: numeros[3],
    })
}

/// `carga_gpu` só decide se a geração pode ser julgada; a largura não depende dela.
pub fn julgar(enlace: Enlace, carga_gpu: Option<f64>) -> Faixas {
    if enlace.largura_atual < enlace.largura_maxima {
        return Faixas::Estreito {
            atual: enlace.largura_atual,
            maxima: enlace.largura_maxima,
        };
    }

    // Sem saber a carga, NÃO julga: `None` é "não medi", não "está parada".
    let sob_carga = carga_gpu.is_some_and(|c| c >= CARGA_PARA_JULGAR_A_GERACAO);

    if sob_carga && enlace.geracao_atual < enlace.geracao_maxima {
        return Faixas::GeracaoAbaixoSobCarga {
            atual: enlace.geracao_atual,
            maxima: enlace.geracao_maxima,
            largura: enlace.largura_atual,
        };
    }

    Faixas::Completo { largura: enlace.largura_atual }
}

pub fn explicar(faixas: &Faixas) -> String {
    match faixas {
        Faixas::Completo { largura } => format!(
            "A placa de vídeo está usando as {largura} faixas de PCI Express que ela \
             suporta. Nada a fazer aqui."
        ),
        Faixas::Estreito { atual, maxima } => format!(
            "A placa de vídeo está usando {atual} faixas de PCI Express, e ela suporta \
             {maxima}. Isso custa quadro e nenhum ajuste de software resolve, porque é \
             físico. As causas comuns são: a placa espetada no slot de baixo da \
             placa-mãe (que costuma ser mais estreito), um SSD M.2 dividindo as faixas \
             com o slot da placa — o manual da placa-mãe diz quais slots brigam entre \
             si —, um cabo de extensão, ou o encaixe mal encostado."
        ),
        Faixas::GeracaoAbaixoSobCarga { atual, maxima, largura } => format!(
            "Com a placa trabalhando, o PCI Express está na geração {atual} e a placa \
             suporta a {maxima}, com as {largura} faixas completas. Costuma ser \
             configuração da BIOS fixando a geração, ou um cabo de extensão. Vale \
             conferir no manual da placa-mãe antes de mexer em qualquer coisa."
        ),
        Faixas::NaoDeuParaLer { motivo } => format!(
            "Não deu para ler quantas faixas de PCI Express a placa de vídeo está \
             usando: {motivo} Isso não quer dizer que esteja tudo bem — quer dizer \
             que esta verificação não aconteceu nesta máquina."
        ),
    }
}

#[cfg(target_os = "windows")]
pub fn analisar() -> Faixas {
    let saida = super::shell::run(
        "nvidia-smi",
        &[
            "--query-gpu=pcie.link.gen.current,pcie.link.gen.max,\
             pcie.link.width.current,pcie.link.width.max,utilization.gpu",
            "--format=csv,noheader,nounits",
        ],
    );

    let saida = match saida {
        Ok(s) if s.success => s.stdout,
        _ => {
            return Faixas::NaoDeuParaLer {
                motivo: "esta leitura usa o `nvidia-smi`, que vem junto com o driver da \
                         NVIDIA, e ele não respondeu aqui. Em placa AMD ou Intel a \
                         consulta não existe."
                    .to_string(),
            }
        }
    };

    let Some(enlace) = ler_enlace(&saida) else {
        return Faixas::NaoDeuParaLer {
            motivo: "o driver respondeu, mas não com os quatro números do enlace."
                .to_string(),
        };
    };

    julgar(enlace, carga_da_saida(&saida))
}

#[cfg(not(target_os = "windows"))]
pub fn analisar() -> Faixas {
    Faixas::NaoDeuParaLer { motivo: "só no Windows.".to_string() }
}

/// `None` faz a geração NÃO ser julgada, que é o lado seguro.
pub fn carga_da_saida(saida: &str) -> Option<f64> {
    let linha = saida.lines().find(|l| !l.trim().is_empty())?;
    linha.split(',').nth(4)?.trim().replace('%', "").trim().parse::<f64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const AQUI: &str = "3, 3, 16, 16, 2";

    #[test]
    fn le_a_saida_desta_maquina() {
        let enlace = ler_enlace(AQUI).expect("os quatro números estão na linha");

        assert_eq!(enlace.geracao_atual, 3);
        assert_eq!(enlace.largura_atual, 16);
        assert_eq!(carga_da_saida(AQUI), Some(2.0));
    }

    #[test]
    fn aceita_a_largura_escrita_com_x() {
        let enlace = ler_enlace("3, 4, x8, x16, 40").expect("lido");
        assert_eq!(enlace.largura_atual, 8);
        assert_eq!(enlace.largura_maxima, 16);
    }

    #[test]
    fn geracao_baixa_com_a_placa_parada_nao_e_achado() {
        let parada = Enlace {
            geracao_atual: 1,
            geracao_maxima: 3,
            largura_atual: 16,
            largura_maxima: 16,
        };

        assert_eq!(julgar(parada, Some(2.0)), Faixas::Completo { largura: 16 });
    }

    #[test]
    fn sem_saber_a_carga_a_geracao_nao_e_julgada() {
        let enlace = Enlace {
            geracao_atual: 1,
            geracao_maxima: 4,
            largura_atual: 16,
            largura_maxima: 16,
        };

        assert_eq!(julgar(enlace, None), Faixas::Completo { largura: 16 });
    }

    #[test]
    fn geracao_baixa_com_a_placa_trabalhando_e_achado() {
        let enlace = Enlace {
            geracao_atual: 2,
            geracao_maxima: 4,
            largura_atual: 16,
            largura_maxima: 16,
        };

        assert_eq!(
            julgar(enlace, Some(85.0)),
            Faixas::GeracaoAbaixoSobCarga { atual: 2, maxima: 4, largura: 16 }
        );
    }

    #[test]
    fn largura_estreita_e_achado_mesmo_com_a_placa_parada() {
        let estreito = Enlace {
            geracao_atual: 3,
            geracao_maxima: 3,
            largura_atual: 4,
            largura_maxima: 16,
        };

        assert_eq!(
            julgar(estreito, Some(0.0)),
            Faixas::Estreito { atual: 4, maxima: 16 }
        );
    }

    /// A largura manda: é o que a pessoa conserta abrindo o gabinete.
    #[test]
    fn com_as_duas_erradas_a_largura_vem_primeiro() {
        let tudo_errado = Enlace {
            geracao_atual: 1,
            geracao_maxima: 4,
            largura_atual: 4,
            largura_maxima: 16,
        };

        assert_eq!(
            julgar(tudo_errado, Some(90.0)),
            Faixas::Estreito { atual: 4, maxima: 16 }
        );
    }

    #[test]
    fn linha_incompleta_nao_vira_zero_faixas() {
        assert!(ler_enlace("3, 3").is_none());
        assert!(ler_enlace("").is_none());
        assert!(ler_enlace("[N/A], [N/A], [N/A], [N/A]").is_none());
    }

    #[test]
    fn zero_nao_vira_leitura_valida() {
        assert!(ler_enlace("0, 0, 0, 0, 0").is_none());
    }

    #[test]
    fn a_frase_do_estreito_diz_o_que_olhar() {
        let frase = explicar(&Faixas::Estreito { atual: 4, maxima: 16 });

        assert!(frase.contains('4') && frase.contains("16"));
        assert!(frase.contains("M.2"), "a causa mais comum precisa estar na frase");
        assert!(
            frase.contains("nenhum ajuste de software"),
            "o cliente precisa saber que não adianta otimizar isto"
        );
    }

    #[test]
    fn nao_deu_para_ler_nao_soa_como_esta_tudo_bem() {
        let frase = explicar(&Faixas::NaoDeuParaLer { motivo: "sem driver.".to_string() });
        assert!(frase.contains("não quer dizer que esteja tudo bem"));
    }

    #[test]
    #[ignore = "toca o Windows desta máquina"]
    fn as_faixas_desta_maquina() {
        let faixas = analisar();
        println!("{faixas:?}");
        println!("{}", explicar(&faixas));
    }
}
