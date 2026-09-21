// O caminho do mouse: do movimento da mão até o pixel que o jogo lê
//
// O QUE ESTE MÓDULO NÃO FAZ, E ISTO VEM PRIMEIRO
//
// NÃO MEDE MIRA. Não olha para dentro do jogo, não lê onde o cliente acertou,
// não conta tiro certo e errado, não pontua desempenho de ninguém. Mira é
// habilidade de pessoa, e um programa que a mede sem estar dentro do jogo está
// inventando um número; um que se mete dentro do jogo para medir está do lado
// errado de qualquer anticheat, e coloca a conta do cliente em risco para
// entregar um gráfico.
//
// NÃO INSTALA GANCHO DE ENTRADA. Um gancho global de mouse — o caminho pelo
// qual se mediria a entrada de outro processo — é indistinguível de um
// keylogger para qualquer sistema de proteção, e a semelhança não é injusta:
// é literalmente a mesma chamada.
//
// O QUE ELE FAZ
//
// Olha para o CAMINHO, que é do sistema e não do jogo: o que o Windows faz com
// o movimento entre o sensor do mouse e o pixel que o jogo recebe. Ali há
// coisas que quebram a relação 1:1 do movimento, que são medíveis de fora, e
// que o cliente pode mudar.
//
// AS DUAS COISAS QUE QUEBRAM O 1:1
//
//   1. A "precisão aprimorada do ponteiro" — a aceleração. Com ela ligada, o
//      MESMO movimento de mão percorre distâncias diferentes conforme a
//      velocidade. O produto já sabia desligar isto; o que faltava era dizer
//      que está ligado e por que importa.
//
//   2. A barra de velocidade do ponteiro. Esta quase ninguém olha, e ela é
//      pior: fora do meio, o Windows MULTIPLICA ou DIVIDE as contagens que o
//      mouse mandou. Abaixo do meio ele descarta contagem — um movimento
//      pequeno da mão pode virar zero pixel. Acima, ele pula pixels. Nos dois
//      casos, dois movimentos iguais da mão podem terminar em lugares
//      diferentes, e nenhuma quantidade de treino conserta isso.
//
// POR QUE A TABELA DE MULTIPLICADORES NÃO ESTÁ AQUI
//
// Porque eu não a medi. Existe uma tabela conhecida de quanto cada passo da
// barra multiplica, e reproduzi-la aqui seria publicar um número que este
// produto não conferiu — exatamente o que ele não faz com nenhum outro número.
// O que é certo e basta para agir: o meio da barra é o único ponto 1:1; abaixo
// dele há contagem descartada e acima há pixel pulado.
//
// A TAXA DE VARREDURA
//
// Quantas vezes por segundo o mouse relata posição. Ela fecha a primeira etapa
// do orçamento de latência, que hoje está declarada como não medida. Medi-la
// exige capturar entrada bruta NA JANELA DESTE aplicativo — o que é legítimo,
// porque é a nossa própria janela — e essa captura ainda não está ligada. A
// conta que transforma os intervalos em taxa está aqui, pronta e testada; o
// que falta é quem a alimente, e enquanto faltar a métrica sai como não
// medida em vez de sair como um palpite.

use serde::{Deserialize, Serialize};

/// O passo da barra de velocidade do ponteiro que é 1:1.
///
/// Seis, de vinte. É o padrão do Windows e o único ponto em que uma contagem
/// do mouse vira exatamente um passo do ponteiro.
pub const BARRA_UM_PARA_UM: u32 = 6;

/// Quantos intervalos são necessários para afirmar uma taxa.
///
/// Duzentos. A um relato por milissegundo, é um quinto de segundo de
/// movimento — curto para quem mexe o mouse e longo o bastante para a mediana
/// não ser decidida por um punhado de relatos atrasados.
pub const INTERVALOS_PARA_A_TAXA: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Achado {
    /// A aceleração está ligada.
    AceleracaoLigada,
    /// A barra está abaixo do meio: o Windows descarta contagem.
    BarraAbaixoDoMeio,
    /// A barra está acima do meio: o Windows pula pixel.
    BarraAcimaDoMeio,
}

impl Achado {
    /// O título curto, do jeito que aparece na tela.
    ///
    /// O TEXTO MORA AQUI, e não na tela. A primeira versão deste painel tinha
    /// as mesmas três frases escritas duas vezes — uma em Rust e outra em
    /// TypeScript — e duas cópias de um texto que o cliente lê é como um
    /// produto passa a dizer duas coisas diferentes sobre o mesmo achado
    /// conforme quem for corrigido primeiro.
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

    /// O que fazer, em palavras de onde a pessoa vai mexer.
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
    /// `None` quando a chave não pôde ser lida. Ausente não vira "desligada":
    /// absolver o que ninguém olhou é o defeito que este produto não comete.
    pub aceleracao: Option<bool>,
    /// A posição da barra, de 1 a 20.
    pub barra: Option<u32>,
    /// Taxa de varredura do mouse, quando medida.
    pub taxa_hz: Option<f64>,
    pub achados: Vec<Achado>,
    /// O que não pôde ser lido ou medido.
    pub falta: Vec<String>,
}

/// Monta o diagnóstico a partir dos valores crus do registro.
///
/// Recebe texto porque é assim que essas chaves são guardadas — `MouseSpeed` e
/// `MouseSensitivity` são cadeias de caracteres no registro do Windows, e não
/// números. Texto que não é número vira ausente, e nunca zero.
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

    // Fora da faixa de 1 a 20 é leitura estragada, e não uma barra exótica.
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

/// A taxa de varredura a partir dos intervalos entre relatos, em microssegundos.
///
/// MEDIANA e não média: um relato perdido produz um intervalo do dobro do
/// tamanho, e a média de mil intervalos com dez dobrados devolve uma taxa que o
/// mouse nunca teve. A mediana ignora os atrasados.
///
/// `None` abaixo de `INTERVALOS_PARA_A_TAXA`, e `None` quando a mediana não é
/// positiva — uma taxa calculada sobre intervalo zero seria infinita.
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

/// Um achado com o texto que a tela mostra, pronto.
///
/// A tela recebe o texto em vez de ter a própria tabela: ver `Achado::titulo`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AchadoNaTela {
    pub achado: Achado,
    pub titulo: String,
    pub explicacao: String,
    pub onde_mexer: String,
}

impl Caminho {
    /// Os achados com o texto embutido.
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

/// Lê o caminho do mouse desta máquina.
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

    /// Nenhum achado pode chegar à tela sem texto.
    ///
    /// A tela não tem mais tabela própria: ela mostra o que vem daqui. Um
    /// achado novo sem texto apareceria como um cartão vazio no produto, e a
    /// guarda é este teste.
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

    /// A barra fora do meio é o achado que quase ninguém olha.
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

    /// Chave ilegível não vira "desligada".
    #[test]
    fn ausencia_nao_e_absolvicao() {
        let c = avaliar(None, None, None);

        assert_eq!(c.aceleracao, None);
        assert_eq!(c.barra, None);
        assert!(c.achados.is_empty(), "não olhou, não acusa");
        // Mas a falta é declarada, nos três pontos.
        assert_eq!(c.falta.len(), 3, "{:?}", c.falta);
    }

    /// Texto que não é número não vira zero.
    #[test]
    fn valor_estragado_e_ausencia_e_nao_zero() {
        let c = avaliar(Some("sim"), Some("rápido"), None);

        assert_eq!(c.aceleracao, None);
        assert_eq!(c.barra, None);
    }

    /// Barra fora de 1..20 é leitura estragada, e não uma barra exótica.
    #[test]
    fn barra_fora_da_faixa_e_leitura_estragada() {
        assert_eq!(avaliar(Some("0"), Some("0"), None).barra, None);
        assert_eq!(avaliar(Some("0"), Some("57"), None).barra, None);
    }

    /// A taxa exige amostra, e abaixo dela não se inventa um número.
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

    /// A razão de ser mediana e não média.
    ///
    /// Um décimo dos relatos chegando atrasados em dobro: a média devolveria
    /// uma taxa que o mouse nunca teve, e a mediana ignora os atrasados.
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

        // E a média, que é o que NÃO se usa, erraria por quase 10%.
        let media: f64 = intervalos.iter().sum::<u64>() as f64 / intervalos.len() as f64;
        assert!(1_000_000.0 / media < 920.0);
    }

    #[test]
    fn intervalo_zero_nao_vira_taxa_infinita() {
        let zeros = vec![0u64; INTERVALOS_PARA_A_TAXA * 2];

        // Todos descartados: sobra amostra nenhuma, e não uma taxa infinita.
        assert_eq!(taxa_de_varredura(&zeros), None);
    }

    /// Com a taxa medida, ela deixa de constar como faltante.
    #[test]
    fn a_taxa_medida_sai_da_lista_de_faltas() {
        let c = avaliar(Some("0"), Some("6"), Some(1000.0));

        assert_eq!(c.taxa_hz, Some(1000.0));
        assert!(c.falta.is_empty(), "{:?}", c.falta);
    }
}
