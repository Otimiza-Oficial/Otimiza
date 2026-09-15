// As faixas do PCI Express até a placa de vídeo
//
// A placa de vídeo conversa com o processador por um número de faixas. Uma placa
// de jogo espera dezesseis. Quando ela roda em oito, ou em quatro, o custo
// aparece no jogo — e não há ajuste de registro, plano de energia ou perfil que
// conserte, porque o problema é físico:
//
//   - a placa foi espetada no slot de baixo da placa-mãe, que costuma ser x4;
//   - há um SSD M.2 dividindo as faixas com o slot da placa (comum em placa-mãe
//     de entrada, e o manual avisa em letra miúda);
//   - a placa está num cabo de extensão (riser) que negociou menos;
//   - o encaixe está sujo ou mal encostado.
//
// Nenhuma dessas quatro coisas o cliente descobre sozinho, e todas custam
// quadro. É leitura pura e vale a pena dizer.
//
// ─────────────────────────────────────────────────────────────────────────
// A ARMADILHA, e é por ela que este módulo existe em vez de ser três linhas
//
// A leitura traz DOIS pares: geração e largura, cada um com o valor atual e o
// máximo. E eles não valem a mesma coisa:
//
//   LARGURA (x16, x8, x4) é negociada quando a máquina liga e não muda depois.
//   Largura atual menor que a máxima é achado de verdade.
//
//   GERAÇÃO (1, 2, 3, 4...) MUDA SOZINHA. Com a placa parada, o driver derruba
//   o enlace para a geração 1 para economizar energia, e sobe de volta quando o
//   jogo abre. Ler a geração com a área de trabalho aberta e gritar "sua placa
//   está em PCIe 1.0" seria criar um problema que não existe — e é exatamente o
//   tipo de alarme falso que os otimizadores do mercado vendem.
//
// Então: a largura vira achado a qualquer momento; a geração SÓ é julgada com a
// placa sob carga, e fora disso o produto cala a boca sobre ela.
//
// Conferido nesta máquina, com a área de trabalho aberta e a GPU a 2%:
//
//     gen atual 3 · gen máx 3 · largura atual 16 · largura máx 16
//
// ─────────────────────────────────────────────────────────────────────────
//
// SÓ NVIDIA POR ENQUANTO. O `nvidia-smi` vem com o driver e fica em
// `C:\Windows\system32`, então não há nada a instalar. A AMD não tem
// equivalente de linha de comando, e a resposta ali é "não deu para ler" — que
// é honesta e melhor que um palpite.

use serde::{Deserialize, Serialize};

/// Abaixo de quanto da carga a geração não pode ser julgada.
///
/// Vinte por cento: acima disso a placa já saiu do repouso e subiu o enlace.
/// Abaixo, qualquer leitura de geração é do estado de economia.
pub const CARGA_PARA_JULGAR_A_GERACAO: f64 = 20.0;

/// O que se leu do enlace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Enlace {
    pub geracao_atual: u32,
    pub geracao_maxima: u32,
    pub largura_atual: u32,
    pub largura_maxima: u32,
}

/// O veredito sobre as faixas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "estado")]
pub enum Faixas {
    /// A placa está com todas as faixas que ela suporta.
    Completo { largura: u32 },
    /// Menos faixas do que a placa suporta. Isto é físico e custa quadro.
    Estreito { atual: u32, maxima: u32 },
    /// A geração está abaixo da máxima E a placa está sob carga — aí vale
    /// dizer. Em repouso este estado nunca é produzido.
    GeracaoAbaixoSobCarga { atual: u32, maxima: u32, largura: u32 },
    /// Não deu para ler. Não é "está tudo bem".
    NaoDeuParaLer { motivo: String },
}

/// Lê a saída do `nvidia-smi` no formato CSV sem cabeçalho.
///
/// **Função pura.** `None` quando a linha não traz os quatro números — e uma
/// linha incompleta não vira zero, que seria "a placa está em zero faixas".
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

    // Zero em qualquer um dos quatro é resposta inválida do driver, não uma
    // placa com zero faixas.
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

/// A REGRA. **Função pura**, e é a parte que precisa estar certa.
///
/// `carga_gpu` é a utilização da placa no momento da leitura. Ela existe só
/// para decidir se a geração pode ser julgada — a largura não depende dela.
pub fn julgar(enlace: Enlace, carga_gpu: Option<f64>) -> Faixas {
    // A largura primeiro, porque é a que importa e a que não mente.
    if enlace.largura_atual < enlace.largura_maxima {
        return Faixas::Estreito {
            atual: enlace.largura_atual,
            maxima: enlace.largura_maxima,
        };
    }

    // A geração só é julgada com a placa trabalhando. Sem saber a carga, o
    // produto NÃO julga: `None` aqui é "não medi", e não "está parada".
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

/// A frase que o cliente lê. Regra de produto, e por isso tem teste.
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
    // `nvidia-smi` vem com o driver e mora em `System32`. Não é dependência
    // nova: ou o driver da NVIDIA está instalado e ele existe, ou a máquina
    // não é NVIDIA e a resposta é "não deu para ler".
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

/// A utilização da placa, que é o quinto campo da mesma linha.
///
/// **Função pura.** `None` quando não veio — e `None` faz a geração NÃO ser
/// julgada, que é o lado seguro.
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

    /// A ARMADILHA DO MÓDULO. Com a área de trabalho aberta, a placa derruba o
    /// enlace para economizar energia. Gritar "sua placa está em PCIe 1.0" aí é
    /// inventar um problema — e é o alarme falso que o mercado vende.
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

    /// Sem saber a carga, o produto também não julga a geração. `None` é "não
    /// medi", e não "está parada".
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

    /// A largura NÃO depende da carga: ela é negociada quando a máquina liga e
    /// não muda depois. Uma placa em x4 é uma placa em x4 com a máquina parada.
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

    /// A largura manda: com as duas coisas erradas, a que aparece é a física,
    /// porque é a que a pessoa consegue consertar abrindo o gabinete.
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

    /// Zero é resposta inválida do driver, não uma placa com zero faixas.
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

    /// A lacuna não pode soar como aprovação.
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
