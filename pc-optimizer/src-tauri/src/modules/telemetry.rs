// Telemetria central — de onde veio cada número
//
// POR QUE ISTO EXISTE
//
// O coletor antigo devolvia `temperature: 0.0` com o comentário "Placeholder -
// requer acesso baixo nível" ao lado. Na tela isso não chega como "não medi":
// chega como zero grau. Um classificador de gargalo que lesse esse campo
// concluiria "sem problema térmico" com toda a confiança do mundo, a partir de
// um número que ninguém mediu. O mesmo valia para memória em cache, para o
// clock quando o sistema não informa, e para a velocidade de disco e de rede na
// primeira leitura — onde o comentário no código chegava a dizer que o valor
// saía "como desconhecido em vez de zero", enquanto a linha logo abaixo
// retornava `(0.0, 0.0)`.
//
// Zero é uma afirmação. "0 MB/s" quer dizer que o disco está parado, e essa é
// uma frase sobre a máquina do cliente que o programa não tinha como sustentar.
//
// O QUE ESTE MÓDULO FAZ
//
// Ele não mede nada. Ele é o formato em que toda medição do produto passa a ser
// entregue: valor OPCIONAL, unidade, origem, qualidade e motivo. Quem não foi
// medido chega como `null` com qualidade UNKNOWN e uma frase dizendo por quê —
// e a tela mostra essa frase em vez de um número.
//
// QUALIDADE
//
//   MEASURED   lido de um sensor ou contador desta máquina, agora.
//   ESTIMATED  derivado, informado pelo sistema, ou medido por um caminho que
//              não distingue "zero de verdade" de "falha silenciosa". Serve
//              para exibir; não serve para fechar veredito sozinho.
//   UNKNOWN    não há valor. O campo existe para que a ausência seja dita.
//
// O CATÁLOGO
//
// Toda métrica central do produto está listada em `CATALOG`, e uma telemetria
// nova nasce com TODAS elas em UNKNOWN. Os coletores sobem o que conseguem
// medir. Essa inversão é o ponto do módulo: uma métrica sem provedor aparece
// declarada como desconhecida em vez de sumir da resposta — some o campo, some
// a pergunta, e ninguém percebe que o produto parou de olhar para temperatura.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Versão do contrato. Sobe quando um campo muda de significado.
///
/// A tela lê este número antes de confiar no resto: uma versão que ela não
/// conhece é motivo para não desenhar, não para adivinhar.
pub const SCHEMA_VERSION: u32 = 1;

/// Motivo padrão de quem ainda não tem quem meça.
pub const SEM_PROVEDOR: &str = "sem provedor de medição nesta versão";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Quality {
    Measured,
    Estimated,
    Unknown,
}

/// Unidade da métrica.
///
/// É um tipo, e não texto livre, porque o consumidor desta telemetria vai
/// comparar números entre si. Milissegundo comparado com segundo não dá erro em
/// lugar nenhum: dá conclusão errada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Unit {
    Percent,
    Megahertz,
    Hertz,
    Celsius,
    Gigabytes,
    MegabytesPerSecond,
    Milliseconds,
    Fps,
    Watts,
    Hours,
    Count,
    /// Zero é não, um é sim. Existe para que "está em throttling?" possa ser
    /// UNKNOWN — um `bool` obrigaria a responder sim ou não sem ter medido.
    Boolean,
}

/// Uma medição, com procedência.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Metric {
    /// `None` sempre que a qualidade é UNKNOWN. É o campo que impede o zero
    /// inventado de existir.
    pub value: Option<f64>,
    pub unit: Unit,
    /// Quem leu. `"sysinfo"`, `"powercfg"`, `"etw"`, `"none"`.
    pub source: String,
    pub quality: Quality,
    /// Por que não foi medido, ou por que a medição é apenas estimada.
    /// Obrigatório fora de MEASURED — é o texto que a tela mostra no lugar do
    /// número.
    pub reason: Option<String>,
    /// IDADE DA LEITURA, em milissegundos. `None` quando foi lida agora.
    ///
    /// Nem todo sensor pode ser consultado a cada coleta. O uso da placa de
    /// vídeo sai de uma consulta ao WMI que custa mais de um segundo; fazer
    /// isso a cada dois segundos seria o monitor virando a carga que ele veio
    /// medir. A saída é ler de vez em quando e reaproveitar — e aí o número na
    /// tela não é de agora, é de alguns segundos atrás.
    ///
    /// Esse "alguns segundos atrás" precisa estar escrito. Um uso de GPU de 12
    /// segundos atrás, apresentado como se fosse instantâneo, é a mesma classe
    /// de mentira que o zero inventado: a diferença entre 95% agora e 95% antes
    /// de o jogo fechar decide o veredito.
    #[serde(default)]
    pub age_ms: Option<u64>,
}

impl Metric {
    /// Lido agora, desta máquina.
    ///
    /// Valor não finito vira UNKNOWN em vez de viajar como NaN: `NaN` em JSON
    /// nem sequer é representável, e um `0.0` no lugar seria exatamente a
    /// mentira que este módulo existe para evitar.
    pub fn measured(value: f64, unit: Unit, source: impl Into<String>) -> Self {
        if !value.is_finite() {
            return Metric::unknown(unit, "a leitura devolveu um número inválido");
        }

        Metric {
            value: Some(value),
            unit,
            source: source.into(),
            quality: Quality::Measured,
            reason: None,
            age_ms: None,
        }
    }

    /// Derivado, informado pelo sistema, ou medido por caminho que não
    /// distingue zero real de falha. O motivo é obrigatório porque é ele que
    /// diz ao próximo módulo o quanto pode apostar neste número.
    pub fn estimated(
        value: f64,
        unit: Unit,
        source: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        if !value.is_finite() {
            return Metric::unknown(unit, "a leitura devolveu um número inválido");
        }

        Metric {
            value: Some(value),
            unit,
            source: source.into(),
            quality: Quality::Estimated,
            reason: Some(reason.into()),
            age_ms: None,
        }
    }

    /// Não medido. Continua sendo uma resposta: diz qual pergunta ficou sem
    /// número e por quê.
    pub fn unknown(unit: Unit, reason: impl Into<String>) -> Self {
        Metric {
            value: None,
            unit,
            source: "none".to_string(),
            quality: Quality::Unknown,
            reason: Some(reason.into()),
            age_ms: None,
        }
    }

    /// Derruba para UNKNOWN o valor que caiu fora do intervalo possível.
    ///
    /// Usar em porcentagem, principalmente. Um uso de CPU de 340% não é um
    /// número para arredondar até caber: é sinal de que a leitura ou a conta
    /// está errada, e o que se faz com leitura errada é não usá-la. Cortar em
    /// 100 esconderia o defeito atrás de um número plausível.
    /// Marca a leitura como vinda de alguns milissegundos atrás.
    ///
    /// Uma leitura velha que ainda vale para exibir sai como ESTIMATED: ela
    /// descreve um instante que já passou, e nenhum módulo deve fechar veredito
    /// em cima dela sem olhar a idade. Idade num UNKNOWN não faz sentido —
    /// não há leitura para envelhecer — então esse caso fica como está.
    pub fn com_idade(mut self, idade_ms: u64) -> Self {
        if self.quality == Quality::Unknown {
            return self;
        }

        if self.quality == Quality::Measured {
            self.quality = Quality::Estimated;
            self.reason = Some(if idade_ms < 1000 {
                "leitura de menos de um segundo atrás".to_string()
            } else {
                format!("leitura de {} s atrás", idade_ms / 1000)
            });
        }

        self.age_ms = Some(idade_ms);
        self
    }

    /// Idade numa grandeza que não se deteriora sozinha.
    ///
    /// A frequência do monitor é o caso: ela muda quando alguém troca o modo
    /// de vídeo, não com a passagem do tempo. Uma leitura de dez segundos
    /// atrás continua verdadeira, então a idade é registrada — quem lê decide
    /// o que é velho demais — mas a qualidade não cai. Usar com parcimônia: a
    /// maioria das medições descreve um instante e envelhece de verdade.
    pub fn com_idade_estavel(mut self, idade_ms: u64) -> Self {
        if self.quality == Quality::Unknown {
            return self;
        }

        self.age_ms = Some(idade_ms);
        self
    }

    pub fn require_range(self, min: f64, max: f64) -> Self {
        match self.value {
            Some(valor) if valor < min || valor > max => Metric::unknown(
                self.unit,
                format!("leitura fora do intervalo possível ({valor:.2})"),
            ),
            _ => self,
        }
    }
}

/// Quantas métricas de cada qualidade há nesta coleta.
///
/// Vai junto no contrato, e não é recalculado pela tela, para que o painel de
/// evidências e o backend não possam discordar sobre quantas coisas o produto
/// realmente mediu.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Summary {
    pub measured: usize,
    pub estimated: usize,
    pub unknown: usize,
    pub total: usize,
}

/// Uma coleta inteira.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Telemetry {
    pub schema_version: u32,
    /// Segundos desde 1970. Quando esta coleta foi feita.
    pub collected_at: u64,
    /// Milissegundos desde a coleta anterior. `None` na primeira leitura da
    /// sessão — e é justamente nessa que toda taxa é desconhecida, porque taxa
    /// é diferença dividida por tempo e ainda não existe diferença.
    pub since_previous_ms: Option<u64>,
    /// Quanto esta coleta levou, do começo ao fim.
    ///
    /// INCLUI a espera de amostragem da CPU, que é a maior parte. Não é
    /// percentual de overhead e não deve ser apresentado como tal: é o tempo
    /// de parede da coleta, útil para ver se o laço está atrasando.
    pub collection_duration_ms: u64,
    pub summary: Summary,
    pub metrics: BTreeMap<String, Metric>,
}

impl Telemetry {
    /// Nasce com todo o catálogo em UNKNOWN. Os coletores sobem o que medirem.
    pub fn new(collected_at: u64, since_previous_ms: Option<u64>) -> Self {
        let metrics = CATALOG
            .iter()
            .map(|(id, unit)| (id.to_string(), Metric::unknown(*unit, SEM_PROVEDOR)))
            .collect();

        Telemetry {
            schema_version: SCHEMA_VERSION,
            collected_at,
            since_previous_ms,
            collection_duration_ms: 0,
            summary: Summary::default(),
            metrics,
        }
    }

    /// Grava uma métrica do catálogo.
    ///
    /// Se a unidade não bater com a do catálogo, o que fica guardado é UNKNOWN
    /// dizendo isso. É de propósito: uma unidade trocada é um número que parece
    /// certo e está errado por um fator de mil, e o produto prefere não
    /// responder a responder assim. Id fora do catálogo é ignorado, e o teste
    /// `todos_os_ids_gravados_existem_no_catalogo` existe para que isso não
    /// aconteça calado.
    pub fn set(&mut self, id: &str, metric: Metric) {
        let Some(atual) = self.metrics.get_mut(id) else {
            debug_assert!(false, "métrica fora do catálogo: {id}");
            return;
        };

        if atual.unit != metric.unit {
            let esperada = atual.unit;
            *atual = Metric::unknown(esperada, format!("unidade divergente do catálogo em {id}"));
            return;
        }

        *atual = metric;
    }

    /// Grava uma métrica que não cabe no catálogo por ser uma série de tamanho
    /// variável — o uso de cada núcleo lógico, que depende da CPU da máquina.
    pub fn set_series(&mut self, id: String, metric: Metric) {
        self.metrics.insert(id, metric);
    }

    /// O lado de leitura do contrato.
    ///
    /// Hoje só os testes chamam — o `mod modules` é privado, então o compilador
    /// não enxerga uso externo. Ficam aqui porque são por onde um módulo lê a
    /// telemetria em vez de refazer a coleta, e porque é com `get` que se
    /// verifica a qualidade antes de usar o número.
    #[allow(dead_code)]
    pub fn get(&self, id: &str) -> Option<&Metric> {
        self.metrics.get(id)
    }

    /// Valor, se houver. Atalho para quem só quer o número e já aceita que ele
    /// pode não existir.
    #[allow(dead_code)]
    pub fn value(&self, id: &str) -> Option<f64> {
        self.metrics.get(id).and_then(|m| m.value)
    }

    /// Fecha a coleta: carimba a duração e conta as qualidades.
    pub fn finish(mut self, collection_duration_ms: u64) -> Self {
        self.collection_duration_ms = collection_duration_ms;
        self.summary = self.count();
        self
    }

    fn count(&self) -> Summary {
        let mut resumo = Summary::default();

        for metric in self.metrics.values() {
            match metric.quality {
                Quality::Measured => resumo.measured += 1,
                Quality::Estimated => resumo.estimated += 1,
                Quality::Unknown => resumo.unknown += 1,
            }
        }

        resumo.total = self.metrics.len();
        resumo
    }
}

/// Id da métrica de uso de um núcleo lógico.
pub fn id_do_nucleo(indice: usize) -> String {
    format!("cpu.core.{indice}.usage")
}

/// As métricas centrais do produto.
///
/// A lista vem do que a plataforma se propõe a olhar, não do que ela consegue
/// medir hoje. Quem não tem provedor aparece como UNKNOWN — é assim que a
/// distância entre o prometido e o medido fica visível na própria resposta, em
/// vez de virar um campo que ninguém nota que sumiu.
pub const CATALOG: &[(&str, Unit)] = &[
    // Quadros. Renderizado, gerado e exibido são TRÊS números diferentes e
    // nunca podem ser preenchidos um a partir do outro: esconder o FPS nativo
    // atrás do exibido é o placebo que o produto existe para não fazer.
    ("fps.rendered", Unit::Fps),
    ("fps.generated", Unit::Fps),
    ("fps.displayed", Unit::Fps),
    ("fps.average", Unit::Fps),
    ("fps.low_1pct", Unit::Fps),
    ("fps.low_01pct", Unit::Fps),
    ("frametime.mean", Unit::Milliseconds),
    ("frametime.p95", Unit::Milliseconds),
    ("frametime.p99", Unit::Milliseconds),
    // Quadros que demoraram mais que o dobro da mediana DAQUELA partida, por
    // minuto. O limiar é relativo porque um fixo trataria um jogo a 30 quadros
    // como engasgo permanente e nunca acusaria nada num jogo a 240.
    ("frametime.stutters_per_minute", Unit::Count),
    // CPU.
    ("cpu.usage.overall", Unit::Percent),
    ("cpu.cores.logical", Unit::Count),
    // O clock informado pelo sistema é o que a tabela de frequências diz, não
    // o que os núcleos entregaram no período. São grandezas diferentes, e é por
    // isso que existem dois ids.
    ("cpu.clock.reported", Unit::Megahertz),
    ("cpu.clock.effective", Unit::Megahertz),
    ("cpu.temperature", Unit::Celsius),
    ("cpu.package_power", Unit::Watts),
    ("cpu.throttling.thermal", Unit::Boolean),
    ("cpu.throttling.power", Unit::Boolean),
    // `% Performance Limit` do Windows: 100 = o firmware não está segurando.
    ("cpu.performance_limit", Unit::Percent),
    // Quantos núcleos lógicos o Windows deixou estacionados agora.
    ("cpu.cores.parked", Unit::Count),
    // Memória.
    ("ram.total", Unit::Gigabytes),
    ("ram.used", Unit::Gigabytes),
    ("ram.available", Unit::Gigabytes),
    ("ram.cached", Unit::Gigabytes),
    ("ram.usage", Unit::Percent),
    // GPU.
    ("gpu.usage", Unit::Percent),
    ("gpu.clock", Unit::Megahertz),
    ("gpu.temperature", Unit::Celsius),
    ("gpu.power", Unit::Watts),
    ("vram.used", Unit::Gigabytes),
    ("vram.total", Unit::Gigabytes),
    ("vram.usage", Unit::Percent),
    // Armazenamento.
    ("storage.read_rate", Unit::MegabytesPerSecond),
    ("storage.write_rate", Unit::MegabytesPerSecond),
    // `% Disk Time`: fração do tempo com o disco ocupado. É atividade, e não
    // capacidade nem latência — as três respondem perguntas diferentes.
    ("storage.busy", Unit::Percent),
    ("storage.latency", Unit::Milliseconds),
    ("storage.capacity_used", Unit::Percent),
    // Rede. Latência, jitter e perda são três perguntas distintas — chamar
    // tudo de "ping" é o que o produto não vai fazer.
    ("network.download_rate", Unit::MegabytesPerSecond),
    ("network.upload_rate", Unit::MegabytesPerSecond),
    ("network.latency", Unit::Milliseconds),
    ("network.jitter", Unit::Milliseconds),
    ("network.packet_loss", Unit::Percent),
    // Entrada e tela.
    ("input.mouse_polling", Unit::Hertz),
    ("input.consistency", Unit::Percent),
    ("display.refresh", Unit::Hertz),
    // Sistema.
    ("system.uptime", Unit::Hours),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn telemetria_nova_declara_tudo_como_desconhecido() {
        let t = Telemetry::new(0, None).finish(0);

        assert_eq!(t.summary.total, CATALOG.len());
        assert_eq!(t.summary.unknown, CATALOG.len());
        assert_eq!(t.summary.measured, 0);

        // O ponto do catálogo: temperatura não some da resposta por falta de
        // sensor, ela aparece dizendo que não foi medida.
        let temperatura = t.get("cpu.temperature").expect("catálogo");
        assert_eq!(temperatura.value, None);
        assert_eq!(temperatura.quality, Quality::Unknown);
        assert!(temperatura.reason.is_some());
    }

    #[test]
    fn zero_medido_e_diferente_de_nao_medido() {
        // A distinção que o coletor antigo não tinha. Os dois sairiam como
        // "0.0" no contrato velho.
        let medido = Metric::measured(0.0, Unit::MegabytesPerSecond, "sysinfo");
        let ausente = Metric::unknown(Unit::MegabytesPerSecond, "primeira leitura da sessão");

        assert_eq!(medido.value, Some(0.0));
        assert_eq!(medido.quality, Quality::Measured);

        assert_eq!(ausente.value, None);
        assert_eq!(ausente.quality, Quality::Unknown);
    }

    #[test]
    fn numero_invalido_nao_vira_valor() {
        assert_eq!(
            Metric::measured(f64::NAN, Unit::Percent, "sysinfo").quality,
            Quality::Unknown
        );
        assert_eq!(
            Metric::measured(f64::INFINITY, Unit::MegabytesPerSecond, "sysinfo").value,
            None
        );
        assert_eq!(
            Metric::estimated(
                f64::NAN,
                Unit::Megahertz,
                "sysinfo",
                "informado pelo sistema"
            )
            .value,
            None
        );
    }

    #[test]
    fn porcentagem_impossivel_e_descartada_em_vez_de_cortada() {
        let fora = Metric::measured(340.0, Unit::Percent, "sysinfo").require_range(0.0, 100.0);

        assert_eq!(fora.value, None, "cortar em 100 esconderia o defeito");
        assert_eq!(fora.quality, Quality::Unknown);
        assert!(
            fora.reason.unwrap().contains("340"),
            "o motivo mostra o que foi lido"
        );

        let dentro = Metric::measured(42.0, Unit::Percent, "sysinfo").require_range(0.0, 100.0);
        assert_eq!(dentro.value, Some(42.0));
        assert_eq!(dentro.quality, Quality::Measured);
    }

    #[test]
    fn unidade_trocada_nao_passa() {
        let mut t = Telemetry::new(0, None);

        // Alguém grava temperatura em porcentagem. O valor é plausível; a
        // grandeza é outra.
        t.set(
            "cpu.temperature",
            Metric::measured(65.0, Unit::Percent, "sysinfo"),
        );

        let guardada = t.get("cpu.temperature").expect("catálogo");
        assert_eq!(guardada.value, None);
        assert_eq!(
            guardada.unit,
            Unit::Celsius,
            "a unidade do catálogo é preservada"
        );
        assert!(guardada
            .reason
            .as_deref()
            .unwrap()
            .contains("unidade divergente"));
    }

    #[test]
    fn contagem_acompanha_o_que_foi_gravado() {
        let mut t = Telemetry::new(0, Some(2000));
        t.set(
            "cpu.usage.overall",
            Metric::measured(30.0, Unit::Percent, "sysinfo"),
        );
        t.set(
            "cpu.clock.reported",
            Metric::estimated(3800.0, Unit::Megahertz, "sysinfo", "informado pelo sistema"),
        );

        let t = t.finish(210);

        assert_eq!(t.summary.measured, 1);
        assert_eq!(t.summary.estimated, 1);
        assert_eq!(t.summary.unknown, CATALOG.len() - 2);
        assert_eq!(t.collection_duration_ms, 210);
    }

    #[test]
    fn serie_de_nucleos_entra_fora_do_catalogo() {
        let mut t = Telemetry::new(0, None);
        t.set_series(
            id_do_nucleo(0),
            Metric::measured(12.0, Unit::Percent, "sysinfo"),
        );
        t.set_series(
            id_do_nucleo(1),
            Metric::measured(4.0, Unit::Percent, "sysinfo"),
        );

        let t = t.finish(0);

        assert_eq!(t.summary.total, CATALOG.len() + 2);
        assert_eq!(t.value("cpu.core.1.usage"), Some(4.0));
    }

    #[test]
    fn os_tres_fps_sao_ids_separados() {
        // Nenhum deles pode ser preenchido a partir do outro, e a garantia
        // começa por existirem como três entradas distintas do catálogo.
        let ids: Vec<&str> = CATALOG.iter().map(|(id, _)| *id).collect();

        for id in ["fps.rendered", "fps.generated", "fps.displayed"] {
            assert!(ids.contains(&id), "{id} sumiu do catálogo");
        }
    }

    #[test]
    fn catalogo_nao_tem_id_repetido() {
        let mut ids: Vec<&str> = CATALOG.iter().map(|(id, _)| *id).collect();
        let antes = ids.len();
        ids.sort_unstable();
        ids.dedup();

        assert_eq!(
            antes,
            ids.len(),
            "id repetido no catálogo sobrescreveria o anterior"
        );
    }

    #[test]
    fn contrato_sobrevive_a_ida_e_volta_em_json() {
        let mut t = Telemetry::new(1_700_000_000, Some(2000));
        t.set(
            "ram.usage",
            Metric::measured(51.5, Unit::Percent, "sysinfo"),
        );
        let t = t.finish(205);

        let json = serde_json::to_string(&t).expect("serializa");
        let volta: Telemetry = serde_json::from_str(&json).expect("desserializa");

        assert_eq!(volta, t);
        assert_eq!(volta.schema_version, SCHEMA_VERSION);

        // Desconhecido chega ao frontend como null, não como ausente: é o que
        // permite a tela dizer "não medido" em vez de não dizer nada.
        assert!(json.contains("\"value\":null"));
        assert!(json.contains("\"quality\":\"UNKNOWN\""));
    }
}
