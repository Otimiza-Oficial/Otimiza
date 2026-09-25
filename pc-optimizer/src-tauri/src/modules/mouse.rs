// O caminho do mouse até o pixel. NÃO mede mira (seria inventar, ou entrar no jogo) e NÃO instala gancho global
// de entrada (indistinguível de keylogger). Olha o que o Windows faz com o movimento: a aceleração e a barra de
// velocidade fora do meio (abaixo descarta contagem, acima pula pixel). A tabela de multiplicadores não está aqui
// porque não foi medida. A taxa de varredura tem a conta pronta, mas ainda ninguém a alimenta: sai como não medida.

use serde::{Deserialize, Serialize};

/// Seis, de vinte: o padrão, e o único ponto 1:1.
pub const BARRA_UM_PARA_UM: u32 = 6;

/// A 1 kHz é um quinto de segundo de movimento, o bastante para a mediana não depender de relatos atrasados.
pub const INTERVALOS_PARA_A_TAXA: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Achado {
    AceleracaoLigada,
    BarraAbaixoDoMeio,
    BarraAcimaDoMeio,
}

impl Achado {
    /// O texto mora aqui, e não na tela: duas cópias do mesmo texto passam a dizer coisas diferentes.
    pub fn titulo(self) -> &'static str {
        match self {
            Achado::AceleracaoLigada => "Aceleração do ponteiro ligada",
            Achado::BarraAbaixoDoMeio => "Barra de velocidade abaixo do meio",
            Achado::BarraAcimaDoMeio => "Barra de velocidade acima do meio",
        }
    }

    pub fn explicacao(self) -> &'static str {
        match self {
            Achado::AceleracaoLigada => {
                "A \"precisão aprimorada do ponteiro\" está ligada. Com ela, o mesmo movimento de \
                 mão percorre distâncias diferentes conforme a velocidade — o gesto que hoje \
                 chega no alvo não chega amanhã se a mão for um pouco mais devagar."
            }
            Achado::BarraAbaixoDoMeio => {
                "A barra de velocidade do ponteiro está abaixo do meio. Aí o Windows DESCARTA \
                 contagem do mouse: movimentos pequenos da mão podem não mover nada, e dois \
                 gestos iguais podem terminar em lugares diferentes."
            }
            Achado::BarraAcimaDoMeio => {
                "A barra de velocidade do ponteiro está acima do meio. Aí o Windows MULTIPLICA as \
                 contagens e pula pixels: o ponteiro anda aos saltos, e a precisão que o sensor \
                 do mouse tem se perde no caminho."
            }
        }
    }

    pub fn onde_mexer(self) -> &'static str {
        match self {
            Achado::AceleracaoLigada => {
                "Configurações do Windows → Bluetooth e dispositivos → Mouse → Configurações \
                 adicionais do mouse → Opções do ponteiro → desmarcar \"Aprimorar precisão do \
                 ponteiro\". O Otimiza também faz isso, e o desfazer devolve como estava."
            }
            Achado::BarraAbaixoDoMeio | Achado::BarraAcimaDoMeio => {
                "Na mesma tela, em Opções do ponteiro, deixe a barra de velocidade exatamente no \
                 meio — é o único ponto em que uma contagem do mouse vira um passo do ponteiro. \
                 Para mudar a sensibilidade, use o ajuste DENTRO do jogo, que não descarta \
                 contagem."
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Caminho {
    /// `None` quando não se leu: absolver o que ninguém olhou é o defeito que este produto não comete.
    pub aceleracao: Option<bool>,
    pub barra: Option<u32>,
    pub taxa_hz: Option<f64>,
    pub achados: Vec<Achado>,
    pub falta: Vec<String>,
}

/// `MouseSpeed` e `MouseSensitivity` são texto no registro; texto que não é número vira ausente, nunca zero.
pub fn avaliar(
    mouse_speed: Option<&str>,
    sensibilidade: Option<&str>,
    taxa_hz: Option<f64>,
) -> Caminho {
    let mut achados = Vec::new();
    let mut falta = Vec::new();

    let aceleracao = match mouse_speed.and_then(|t| t.trim().parse::<i64>().ok()) {
        Some(v) => Some(v != 0),
        None => {
            falta.push("a chave da precisão aprimorada do ponteiro não pôde ser lida".to_string());
            None
        }
    };

    if aceleracao == Some(true) {
        achados.push(Achado::AceleracaoLigada);
    }

    // Fora de 1 a 20 é leitura estragada.
    let barra = sensibilidade
        .and_then(|t| t.trim().parse::<u32>().ok())
        .filter(|v| (1..=20).contains(v));

    match barra {
        Some(v) if v < BARRA_UM_PARA_UM => achados.push(Achado::BarraAbaixoDoMeio),
        Some(v) if v > BARRA_UM_PARA_UM => achados.push(Achado::BarraAcimaDoMeio),
        Some(_) => {}
        None => {
            falta.push("a posição da barra de velocidade do ponteiro não pôde ser lida".to_string())
        }
    }

    if taxa_hz.is_none() {
        falta.push(
            "a taxa de varredura do mouse exige capturar entrada bruta na janela do aplicativo, \
             que ainda não está ligada"
                .to_string(),
        );
    }

    Caminho {
        aceleracao,
        barra,
        taxa_hz,
        achados,
        falta,
    }
}

/// MEDIANA: um relato perdido dobra um intervalo, e a média daria uma taxa que o mouse nunca teve. `None` abaixo
/// de `INTERVALOS_PARA_A_TAXA` ou com mediana não positiva.
pub fn taxa_de_varredura(intervalos_us: &[u64]) -> Option<f64> {
    let mut validos: Vec<u64> = intervalos_us.iter().copied().filter(|i| *i > 0).collect();

    if validos.len() < INTERVALOS_PARA_A_TAXA {
        return None;
    }

    validos.sort_unstable();

    let meio = validos.len() / 2;
    let mediana = if validos.len().is_multiple_of(2) {
        (validos[meio - 1] + validos[meio]) as f64 / 2.0
    } else {
        validos[meio] as f64
    };

    (mediana > 0.0).then(|| 1_000_000.0 / mediana)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AchadoNaTela {
    pub achado: Achado,
    pub titulo: String,
    pub explicacao: String,
    pub onde_mexer: String,
}

impl Caminho {
    pub fn achados_na_tela(&self) -> Vec<AchadoNaTela> {
        self.achados
            .iter()
            .map(|a| AchadoNaTela {
                achado: *a,
                titulo: a.titulo().to_string(),
                explicacao: a.explicacao().to_string(),
                onde_mexer: a.onde_mexer().to_string(),
            })
            .collect()
    }
}

#[cfg(target_os = "windows")]
pub fn desta_maquina(taxa_hz: Option<f64>) -> Caminho {
    use super::windows::registry::read_text;

    const CHAVE: &str = r"Control Panel\Mouse";

    let ler = |nome: &str| read_text("HKCU", CHAVE, nome).ok().flatten();

    avaliar(
        ler("MouseSpeed").as_deref(),
        ler("MouseSensitivity").as_deref(),
        taxa_hz,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Achado sem texto apareceria como cartão vazio: a guarda é este teste.
    #[test]
    fn todo_achado_tem_texto() {
        for a in [
            Achado::AceleracaoLigada,
            Achado::BarraAbaixoDoMeio,
            Achado::BarraAcimaDoMeio,
        ] {
            assert!(!a.titulo().is_empty(), "{a:?} sem título");
            assert!(!a.explicacao().is_empty(), "{a:?} sem explicação");
            assert!(!a.onde_mexer().is_empty(), "{a:?} sem onde mexer");
        }
    }

    #[test]
    fn os_achados_saem_com_o_texto_embutido() {
        let c = avaliar(Some("1"), Some("3"), None);
        let na_tela = c.achados_na_tela();

        assert_eq!(na_tela.len(), 2);
        assert_eq!(na_tela[0].achado, Achado::AceleracaoLigada);
        assert_eq!(na_tela[0].titulo, Achado::AceleracaoLigada.titulo());
        assert!(na_tela[1].onde_mexer.contains("meio"));
    }

    #[test]
    fn caminho_limpo_nao_tem_achado() {
        let c = avaliar(Some("0"), Some("6"), None);

        assert_eq!(c.aceleracao, Some(false));
        assert_eq!(c.barra, Some(6));
        assert!(c.achados.is_empty());
    }

    #[test]
    fn aceleracao_ligada_e_achado() {
        let c = avaliar(Some("1"), Some("6"), None);

        assert_eq!(c.achados, vec![Achado::AceleracaoLigada]);
        assert!(c.achados[0].explicacao().contains("velocidade"));
        assert!(c.achados[0].onde_mexer().contains("Aprimorar"));
    }

    #[test]
    fn barra_abaixo_do_meio_descarta_contagem() {
        let c = avaliar(Some("0"), Some("4"), None);

        assert_eq!(c.achados, vec![Achado::BarraAbaixoDoMeio]);
        assert!(c.achados[0].explicacao().contains("DESCARTA"));
    }

    #[test]
    fn barra_acima_do_meio_pula_pixel() {
        let c = avaliar(Some("0"), Some("12"), None);

        assert_eq!(c.achados, vec![Achado::BarraAcimaDoMeio]);
        assert!(c.achados[0].explicacao().contains("MULTIPLICA"));
    }

    #[test]
    fn ausencia_nao_e_absolvicao() {
        let c = avaliar(None, None, None);

        assert_eq!(c.aceleracao, None);
        assert_eq!(c.barra, None);
        assert!(c.achados.is_empty(), "não olhou, não acusa");
        assert_eq!(c.falta.len(), 3, "{:?}", c.falta);
    }

    #[test]
    fn valor_estragado_e_ausencia_e_nao_zero() {
        let c = avaliar(Some("sim"), Some("rápido"), None);

        assert_eq!(c.aceleracao, None);
        assert_eq!(c.barra, None);
    }

    #[test]
    fn barra_fora_da_faixa_e_leitura_estragada() {
        assert_eq!(avaliar(Some("0"), Some("0"), None).barra, None);
        assert_eq!(avaliar(Some("0"), Some("57"), None).barra, None);
    }

    #[test]
    fn poucos_intervalos_nao_viram_taxa() {
        let poucos = vec![1000u64; INTERVALOS_PARA_A_TAXA - 1];

        assert_eq!(taxa_de_varredura(&poucos), None);
    }

    #[test]
    fn mil_hertz_saem_de_intervalos_de_um_milissegundo() {
        let intervalos = vec![1000u64; INTERVALOS_PARA_A_TAXA];

        let hz = taxa_de_varredura(&intervalos).expect("taxa");
        assert!((hz - 1000.0).abs() < 0.001, "{hz}");
    }

    #[test]
    fn relato_atrasado_nao_derruba_a_taxa() {
        let mut intervalos = vec![1000u64; INTERVALOS_PARA_A_TAXA];
        for i in intervalos.iter_mut().take(INTERVALOS_PARA_A_TAXA / 10) {
            *i = 2000;
        }

        let hz = taxa_de_varredura(&intervalos).expect("taxa");
        assert!(
            (hz - 1000.0).abs() < 0.001,
            "a mediana devia ignorar os atrasados: {hz}"
        );

        let media: f64 = intervalos.iter().sum::<u64>() as f64 / intervalos.len() as f64;
        assert!(1_000_000.0 / media < 920.0);
    }

    #[test]
    fn intervalo_zero_nao_vira_taxa_infinita() {
        let zeros = vec![0u64; INTERVALOS_PARA_A_TAXA * 2];

        assert_eq!(taxa_de_varredura(&zeros), None);
    }

    #[test]
    fn a_taxa_medida_sai_da_lista_de_faltas() {
        let c = avaliar(Some("0"), Some("6"), Some(1000.0));

        assert_eq!(c.taxa_hz, Some(1000.0));
        assert!(c.falta.is_empty(), "{:?}", c.falta);
    }
}
