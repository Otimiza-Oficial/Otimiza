// Inteligência de memória de vídeo
//
// POR QUE ISTO EXISTE
//
// O painel já mostrava `vram.usage`, e o classificador de gargalo já apontava
// memória de vídeo acima de 90%. As duas coisas estavam erradas pelo mesmo
// motivo: PORCENTAGEM DE MEMÓRIA DEDICADA NÃO É PRESSÃO DE MEMÓRIA.
//
// O driver de vídeo é um cache. Ele carrega textura, mantém carregada e só
// libera quando alguém precisa do espaço. Numa placa de 8 GB rodando um jogo
// que cabe em 4, é normal o contador marcar 7,5 GB depois de meia hora de
// partida — o driver não devolve o que não está fazendo falta. Ler aquilo como
// "memória de vídeo no limite" manda o cliente baixar a qualidade das texturas
// de um jogo que não tem problema nenhum de memória. É exatamente o ajuste
// placebo que o produto se propõe a não vender.
//
// O QUE REALMENTE MEDE PRESSÃO
//
// Quando a memória dedicada acaba de verdade, o driver não trava: ele começa a
// usar memória do SISTEMA para guardar o que não coube — a memória
// compartilhada. Isso o Windows publica, e isso tem consequência medível: o
// que está do outro lado do PCIe é lido muito mais devagar que o que está na
// placa, e é daí que sai o engasgo ao virar a câmera para um lado do mapa onde
// nunca se foi.
//
// Então a pergunta deste módulo não é "quanto por cento está usado". É:
//
//   1. A memória dedicada está no fim?
//   2. A placa está derramando para a memória do sistema ACIMA do que ela já
//      derramava com a máquina parada?
//
// As duas juntas são pressão. A primeira sozinha é cache cheio, que é o estado
// normal e saudável de uma placa de vídeo.
//
// O PISO
//
// Toda máquina derrama um pouco: a área de trabalho, o navegador e o próprio
// compositor do Windows vivem em memória compartilhada mesmo com a placa
// parada. Comparar o derramamento da partida com ZERO acusaria transbordo em
// toda máquina ligada. Por isso o que conta é o que subiu acima do PISO — o
// menor derramamento já observado NESTA máquina com a placa em repouso.
//
// Piso não observado não vira zero. Vira "ainda não sei", e o módulo responde
// que não avaliou.
//
// PLACA INTEGRADA
//
// Numa placa integrada não existe memória dedicada: toda a memória dela é
// memória do sistema, por projeto. Aplicar a regra de transbordo ali acusaria
// transbordo permanente numa máquina que está funcionando como foi desenhada.
// Por isso o estado é próprio, e o módulo diz o que aquilo significa em vez de
// fingir um diagnóstico.

use serde::{Deserialize, Serialize};

use super::telemetry::Telemetry;

/// A partir de quanto a memória dedicada é considerada no fim.
///
/// Noventa por cento, e não os 92% de processador e disco: aqui o número alto
/// sozinho não acusa nada — ele só habilita a segunda pergunta. O limite pode
/// ser generoso porque não é ele que produz o veredito.
pub const DEDICADA_NO_FIM_PCT: f64 = 90.0;

/// Quanto precisa ter derramado acima do piso para contar.
///
/// Duzentos megabytes. Abaixo disso é a oscilação normal de quem abriu uma aba
/// a mais no navegador enquanto o jogo rodava, e apontar aquilo como transbordo
/// da placa seria culpar o hardware pelo navegador.
pub const TRANSBORDO_QUE_CONTA_GB: f64 = 0.20;

/// Abaixo disto a placa está em repouso, e o derramamento vale como piso.
///
/// Trinta por cento: o suficiente para a área de trabalho e um vídeo tocando,
/// e claramente abaixo de qualquer jogo em execução.
pub const PLACA_EM_REPOUSO_PCT: f64 = 30.0;

/// Quantas leituras em repouso antes de acreditar no piso.
///
/// Uma leitura só pode ter pegado o instante em que um programa acabou de
/// fechar e o driver ainda não devolveu a memória. Cinco leituras em repouso
/// descrevem o comportamento da máquina, e não um instante dela.
pub const AMOSTRAS_PARA_PISO: usize = 5;

/// Total de memória dedicada abaixo do qual a placa é tratada como integrada.
///
/// Meio giga. Integradas reservam um pedaço simbólico e chamam de dedicado;
/// nenhuma placa dedicada de verdade tem menos que isso.
pub const SEM_DEDICADA_ATE_GB: f64 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Estado {
    /// Faltou medida. O que faltou está em `falta`.
    NaoAvaliado,
    /// Placa integrada: usar memória do sistema é o projeto dela, não um
    /// sintoma.
    PlacaIntegrada,
    /// Memória dedicada com folga.
    Folgada,
    /// Dedicada no fim, sem derramar acima do piso.
    ///
    /// O ESTADO NORMAL de uma placa depois de um tempo de jogo. Existe como
    /// estado próprio para poder ser dito ao cliente — "está cheia e está tudo
    /// bem" é uma resposta, e é a que evita o ajuste desnecessário.
    CacheCheio,
    /// Dedicada no fim E derramando para a memória do sistema.
    Transbordando,
    /// Derramando sem a dedicada estar no fim.
    ///
    /// Não é a placa entregando o jogo: é outra coisa usando memória
    /// compartilhada — segunda placa, captura de tela, navegador com
    /// aceleração. O módulo diz o que viu sem atribuir ao jogo.
    DerramaSemPressao,
}

/// O que fazer, quando há o que fazer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Conselho {
    /// Quanto precisa deixar de caber na placa para parar de derramar.
    ///
    /// É subtração de duas medidas, não uma tabela: o que derramou acima do
    /// piso é exatamente o que não coube.
    pub liberar_gb: f64,
    pub texto: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Analise {
    pub estado: Estado,
    /// Quanto da dedicada está em uso, quando dá para saber.
    pub dedicada_pct: Option<f64>,
    /// Quanto sobra de dedicada, em GB.
    pub folga_gb: Option<f64>,
    /// Quanto está derramado ACIMA do piso.
    pub derramado_gb: Option<f64>,
    /// O piso usado na conta, para quem quiser conferir.
    pub piso_gb: Option<f64>,
    /// O que faltou medir. Vazio quando nada faltou.
    pub falta: Vec<String>,
    pub explicacao: String,
    pub conselho: Option<Conselho>,
}

/// O menor derramamento já visto com a placa em repouso.
///
/// Guardado entre leituras pelo monitor. Não é configuração: é uma medida
/// desta máquina, e por isso não tem valor padrão que sirva.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Piso {
    minimo_gb: Option<f64>,
    amostras: usize,
}

impl Piso {
    /// Registra uma leitura. Só conta quando a placa está em repouso.
    ///
    /// Leitura com a placa carregada não pode baixar o piso: seria tomar o
    /// meio da partida como referência de repouso, e o transbordo dali em
    /// diante ficaria medido contra ele mesmo.
    pub fn observar(&mut self, compartilhada_gb: Option<f64>, gpu_pct: Option<f64>) {
        let (Some(gb), Some(uso)) = (compartilhada_gb, gpu_pct) else {
            return;
        };

        if !gb.is_finite() || gb < 0.0 || uso > PLACA_EM_REPOUSO_PCT {
            return;
        }

        self.amostras += 1;
        self.minimo_gb = Some(match self.minimo_gb {
            Some(m) => m.min(gb),
            None => gb,
        });
    }

    /// O piso, quando já dá para confiar nele.
    pub fn valor(&self) -> Option<f64> {
        if self.amostras >= AMOSTRAS_PARA_PISO {
            self.minimo_gb
        } else {
            None
        }
    }

    /// Quantas leituras em repouso ainda faltam.
    pub fn faltam(&self) -> usize {
        AMOSTRAS_PARA_PISO.saturating_sub(self.amostras)
    }
}

/// Avalia a pressão de memória de vídeo a partir do contrato.
pub fn avaliar(t: &Telemetry, piso: &Piso) -> Analise {
    let usada = t.value("vram.used");
    let total = t.value("vram.total");
    let compartilhada = t.value("vram.shared_used");
    let piso_gb = piso.valor();

    let mut falta = Vec::new();

    // Integrada primeiro: sem isto, uma máquina sem placa dedicada cairia nas
    // contas de transbordo e sairia acusada de um problema que é o projeto
    // dela.
    if total.is_some_and(|tot| tot <= SEM_DEDICADA_ATE_GB) {
        return Analise {
            estado: Estado::PlacaIntegrada,
            dedicada_pct: None,
            folga_gb: None,
            derramado_gb: None,
            piso_gb,
            falta,
            explicacao: "Esta máquina usa vídeo integrado: a memória da placa é a memória do \
                 sistema, por projeto. A conta de transbordo não se aplica aqui — o que limita \
                 é a RAM total e a velocidade dela."
                .to_string(),
            conselho: None,
        };
    }

    let (Some(usada), Some(total)) = (usada, total) else {
        if usada.is_none() {
            falta.push("vram.used".to_string());
        }
        if total.is_none() {
            falta.push("vram.total".to_string());
        }
        return Analise {
            estado: Estado::NaoAvaliado,
            dedicada_pct: None,
            folga_gb: None,
            derramado_gb: None,
            piso_gb,
            falta,
            explicacao: "Sem o uso e o total da memória da placa não há o que avaliar.".to_string(),
            conselho: None,
        };
    };

    let pct = usada / total * 100.0;
    let folga = (total - usada).max(0.0);
    let no_fim = pct >= DEDICADA_NO_FIM_PCT;

    // O derramamento só existe como número quando as DUAS pontas existem: a
    // leitura de agora e o piso desta máquina.
    let derramado = match (compartilhada, piso_gb) {
        (Some(agora), Some(p)) => Some((agora - p).max(0.0)),
        _ => {
            if compartilhada.is_none() {
                falta.push("vram.shared_used".to_string());
            } else {
                falta.push(format!(
                    "piso da memória compartilhada: faltam {} leituras com a placa em repouso",
                    piso.faltam()
                ));
            }
            None
        }
    };

    let transbordou = derramado.is_some_and(|d| d >= TRANSBORDO_QUE_CONTA_GB);

    let (estado, explicacao, conselho) = match (no_fim, derramado.filter(|_| transbordou)) {
        (true, Some(d)) => (
            Estado::Transbordando,
            format!(
                "A memória da placa acabou: {usada:.1} de {total:.1} GB em uso, e {d:.1} GB de \
                 textura foram parar na memória do sistema. O que está do outro lado do PCIe é \
                 lido muito mais devagar, e é daí que vem o engasgo ao entrar numa área nova."
            ),
            Some(Conselho {
                liberar_gb: d,
                texto: format!(
                    "Precisa deixar de caber na placa cerca de {d:.1} GB — é exatamente o que \
                     derramou. Textura e distância de visão são o que ocupa mais; resolução de \
                     tela quase não entra nesta conta."
                ),
            }),
        ),
        (false, Some(d)) => (
            Estado::DerramaSemPressao,
            format!(
                "Há {d:.1} GB em memória compartilhada, mas a memória da placa NÃO está no fim \
                 ({pct:.0}% de {total:.1} GB). Isto não é o jogo transbordando: é outro programa \
                 usando memória de vídeo pelo sistema — captura de tela, navegador com aceleração \
                 ou uma segunda placa."
            ),
            None,
        ),
        (true, None) => (
            Estado::CacheCheio,
            format!(
                "A memória da placa está em {pct:.0}%, e isso não é um problema. O driver não \
                 devolve textura que já carregou enquanto ninguém precisa do espaço — placa cheia \
                 depois de um tempo de jogo é o estado normal. Sem derramamento para a memória do \
                 sistema, não há o que ajustar aqui."
            ),
            None,
        ),
        (false, None) => (
            Estado::Folgada,
            format!("Memória da placa com folga: {folga:.1} GB livres de {total:.1} GB."),
            None,
        ),
    };

    Analise {
        estado,
        dedicada_pct: Some(pct),
        folga_gb: Some(folga),
        derramado_gb: derramado,
        piso_gb,
        falta,
        explicacao,
        conselho,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::telemetry::{Metric, Unit};

    fn com(pares: &[(&str, f64)]) -> Telemetry {
        let mut t = Telemetry::new(0, None);
        for (id, v) in pares {
            let unidade = match *id {
                "vram.usage" | "gpu.usage" => Unit::Percent,
                _ => Unit::Gigabytes,
            };
            t.set(id, Metric::measured(*v, unidade, "teste"));
        }
        t.finish(0)
    }

    fn piso_de(gb: f64) -> Piso {
        let mut p = Piso::default();
        for _ in 0..AMOSTRAS_PARA_PISO {
            p.observar(Some(gb), Some(5.0));
        }
        p
    }

    /// A regressão que este módulo existe para consertar.
    ///
    /// Placa a 94% sem derramar nada: o classificador antigo chamava de
    /// memória de vídeo no limite e mandava baixar textura. Aqui a resposta é
    /// que está cheia e está tudo bem.
    #[test]
    fn dedicada_cheia_sem_derramar_nao_e_problema() {
        let t = com(&[
            ("vram.used", 7.5),
            ("vram.total", 8.0),
            ("vram.shared_used", 0.4),
        ]);

        let a = avaliar(&t, &piso_de(0.4));

        assert_eq!(a.estado, Estado::CacheCheio);
        assert!(a.conselho.is_none(), "não há o que ajustar");
        assert!(a.explicacao.contains("normal"));
    }

    #[test]
    fn dedicada_cheia_derramando_e_transbordo() {
        let t = com(&[
            ("vram.used", 7.8),
            ("vram.total", 8.0),
            ("vram.shared_used", 1.9),
        ]);

        let a = avaliar(&t, &piso_de(0.4));

        assert_eq!(a.estado, Estado::Transbordando);
        let c = a.conselho.expect("conselho");
        // 1,9 agora contra 0,4 de piso: 1,5 GB que não coube.
        assert!((c.liberar_gb - 1.5).abs() < 1e-9, "{}", c.liberar_gb);
    }

    /// O piso é o que separa "derramou" de "sempre derramou".
    #[test]
    fn derramamento_igual_ao_piso_nao_conta() {
        let t = com(&[
            ("vram.used", 7.9),
            ("vram.total", 8.0),
            ("vram.shared_used", 1.6),
        ]);

        // Esta máquina já vivia com 1,6 GB compartilhados, parada.
        let a = avaliar(&t, &piso_de(1.6));

        assert_eq!(a.estado, Estado::CacheCheio);
        assert_eq!(a.derramado_gb, Some(0.0));
    }

    #[test]
    fn sem_piso_nao_inventa_transbordo() {
        let t = com(&[
            ("vram.used", 7.8),
            ("vram.total", 8.0),
            ("vram.shared_used", 3.0),
        ]);

        let a = avaliar(&t, &Piso::default());

        assert_eq!(a.estado, Estado::CacheCheio);
        assert_eq!(a.derramado_gb, None);
        assert!(
            a.falta.iter().any(|f| f.contains("piso")),
            "a ausência do piso tem de ser declarada: {:?}",
            a.falta
        );
    }

    #[test]
    fn derramar_com_dedicada_folgada_nao_e_o_jogo() {
        let t = com(&[
            ("vram.used", 3.0),
            ("vram.total", 8.0),
            ("vram.shared_used", 2.0),
        ]);

        let a = avaliar(&t, &piso_de(0.3));

        assert_eq!(a.estado, Estado::DerramaSemPressao);
        assert!(a.conselho.is_none());
    }

    #[test]
    fn integrada_nao_e_acusada_de_transbordo() {
        let t = com(&[
            ("vram.used", 0.1),
            ("vram.total", 0.128),
            ("vram.shared_used", 6.0),
        ]);

        let a = avaliar(&t, &piso_de(1.0));

        assert_eq!(a.estado, Estado::PlacaIntegrada);
        assert!(a.conselho.is_none());
    }

    #[test]
    fn sem_medida_nao_avalia() {
        let a = avaliar(&Telemetry::new(0, None).finish(0), &Piso::default());

        assert_eq!(a.estado, Estado::NaoAvaliado);
        assert!(a.falta.contains(&"vram.used".to_string()));
        assert!(a.falta.contains(&"vram.total".to_string()));
    }

    /// Leitura com a placa carregada não pode virar piso.
    #[test]
    fn piso_ignora_leitura_com_a_placa_ocupada() {
        let mut p = Piso::default();
        for _ in 0..AMOSTRAS_PARA_PISO {
            p.observar(Some(2.0), Some(5.0));
        }
        // Durante a partida a compartilhada até cai num instante — isso não
        // pode baixar a referência de repouso.
        p.observar(Some(0.1), Some(97.0));

        assert_eq!(p.valor(), Some(2.0));
    }

    #[test]
    fn piso_exige_repeticao_antes_de_valer() {
        let mut p = Piso::default();
        p.observar(Some(0.5), Some(2.0));

        assert_eq!(p.valor(), None);
        assert_eq!(p.faltam(), AMOSTRAS_PARA_PISO - 1);
    }
}
