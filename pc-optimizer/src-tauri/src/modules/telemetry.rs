// O formato de toda medição do produto: valor OPCIONAL, unidade, origem, qualidade e motivo. Zero é uma
// afirmação (o coletor antigo devolvia `temperature: 0.0`, e "sem problema térmico" saía de um número que ninguém
// mediu). MEASURED: lido agora. ESTIMATED: derivado ou por caminho que não distingue zero de falha, não fecha
// veredito sozinho. UNKNOWN: sem valor, com o motivo. Uma coleta nasce com todo o `CATALOG` em UNKNOWN, para
// métrica sem provedor aparecer declarada em vez de sumir.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Versão desconhecida é motivo para a tela não desenhar, não para adivinhar.
pub const SCHEMA_VERSION: u32 = 1;

pub const SEM_PROVEDOR: &str = "sem provedor de medição nesta versão";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Quality {
    Measured,
    Estimated,
    Unknown,
}

/// Tipo, não texto: milissegundo comparado com segundo não dá erro, dá conclusão errada.
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
    /// Para "está em throttling?" poder ser UNKNOWN: um `bool` obrigaria a responder sem medir.
    Boolean,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Metric {
    /// `None` sempre que UNKNOWN: é o campo que impede o zero inventado.
    pub value: Option<f64>,
    pub unit: Unit,
    pub source: String,
    pub quality: Quality,
    /// Obrigatório fora de MEASURED: é o texto que a tela mostra no lugar do número.
    pub reason: Option<String>,
    /// `None` quando lida agora. Sensores caros (a GPU pelo WMI passa de um segundo) são lidos de vez em quando e
    /// reaproveitados; a idade precisa estar escrita, porque 95% agora e 95% antes de o jogo fechar decidem vereditos
    /// diferentes.
    #[serde(default)]
    pub age_ms: Option<u64>,
}

impl Metric {
    /// Não finito vira UNKNOWN: `NaN` não existe em JSON, e `0.0` seria a mentira que o módulo evita.
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

    /// O motivo diz ao próximo módulo o quanto apostar neste número.
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

    /// Leitura velha sai como ESTIMATED: descreve um instante que passou. UNKNOWN fica como está.
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

    /// Para grandezas que não envelhecem sozinhas (a frequência do monitor): registra a idade sem rebaixar. Usar
    /// com parcimônia.
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

/// No contrato, e não recalculado pela tela, para painel e backend não discordarem.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Summary {
    pub measured: usize,
    pub estimated: usize,
    pub unknown: usize,
    pub total: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Telemetry {
    pub schema_version: u32,
    pub collected_at: u64,
    /// `None` na primeira leitura da sessão, em que toda taxa é desconhecida.
    pub since_previous_ms: Option<u64>,
    /// Inclui a espera de amostragem da CPU: é tempo de parede, não overhead.
    pub collection_duration_ms: u64,
    pub summary: Summary,
    pub metrics: BTreeMap<String, Metric>,
}

impl Telemetry {
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

    /// Unidade que não bate com o catálogo vira UNKNOWN (um fator de mil parece certo). Id fora do catálogo é
    /// ignorado, e `todos_os_ids_gravados_existem_no_catalogo` impede que isso aconteça calado.
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

    pub fn set_series(&mut self, id: String, metric: Metric) {
        self.metrics.insert(id, metric);
    }

    /// Hoje só os testes chamam (`mod modules` é privado): é por aqui que um módulo lê a telemetria sem refazer a
    /// coleta.
    #[allow(dead_code)]
    pub fn get(&self, id: &str) -> Option<&Metric> {
        self.metrics.get(id)
    }

    #[allow(dead_code)]
    pub fn value(&self, id: &str) -> Option<f64> {
        self.metrics.get(id).and_then(|m| m.value)
    }

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

pub fn id_do_nucleo(indice: usize) -> String {
    format!("cpu.core.{indice}.usage")
}

/// A lista vem do que a plataforma se propõe a olhar, não do que mede hoje.
pub const CATALOG: &[(&str, Unit)] = &[
    // Renderizado, gerado e exibido são TRÊS números e nunca saem um do outro: esconder o FPS nativo atrás do
    // exibido é placebo.
    ("fps.rendered", Unit::Fps),
    ("fps.generated", Unit::Fps),
    ("fps.displayed", Unit::Fps),
    ("fps.average", Unit::Fps),
    ("fps.low_1pct", Unit::Fps),
    ("fps.low_01pct", Unit::Fps),
    ("frametime.mean", Unit::Milliseconds),
    ("frametime.p95", Unit::Milliseconds),
    ("frametime.p99", Unit::Milliseconds),
    // Relativo à mediana: um limiar fixo acusaria engasgo permanente a 30 quadros e nunca a 240.
    ("frametime.stutters_per_minute", Unit::Count),
    // Medidos DENTRO da janela dos quadros: `cpu.usage.overall` e `gpu.usage` dizem o agora, e misturar os dois é
    // comparar momentos diferentes.
    ("match.cpu_usage", Unit::Percent),
    ("match.gpu_usage", Unit::Percent),
    // Separa asset do disco de shader compilando: o mesmo buraco no frametime, e só o instante diz qual foi.
    ("frametime.stutters_with_disk", Unit::Percent),
    ("cpu.usage.overall", Unit::Percent),
    ("cpu.cores.logical", Unit::Count),
    // O clock informado é o da tabela de frequências, não o entregue no período: por isso dois ids.
    ("cpu.clock.reported", Unit::Megahertz),
    ("cpu.clock.effective", Unit::Megahertz),
    ("cpu.temperature", Unit::Celsius),
    ("cpu.package_power", Unit::Watts),
    ("cpu.throttling.thermal", Unit::Boolean),
    ("cpu.throttling.power", Unit::Boolean),
    ("cpu.performance_limit", Unit::Percent),
    ("cpu.cores.parked", Unit::Count),
    ("ram.total", Unit::Gigabytes),
    ("ram.used", Unit::Gigabytes),
    ("ram.available", Unit::Gigabytes),
    ("ram.cached", Unit::Gigabytes),
    ("ram.usage", Unit::Percent),
    ("gpu.usage", Unit::Percent),
    ("gpu.clock", Unit::Megahertz),
    ("gpu.temperature", Unit::Celsius),
    ("gpu.power", Unit::Watts),
    ("vram.used", Unit::Gigabytes),
    ("vram.total", Unit::Gigabytes),
    ("vram.usage", Unit::Percent),
    // Mede pressão de memória de vídeo (não a porcentagem): placa DERRAMANDO para a RAM. Interpretada por
    // `modules::vram`.
    ("vram.shared_used", Unit::Gigabytes),
    ("storage.read_rate", Unit::MegabytesPerSecond),
    ("storage.write_rate", Unit::MegabytesPerSecond),
    // Atividade, não capacidade nem latência.
    ("storage.busy", Unit::Percent),
    ("storage.latency", Unit::Milliseconds),
    ("storage.capacity_used", Unit::Percent),
    // Latência, jitter e perda são três perguntas: chamar tudo de "ping" o produto não faz.
    ("network.download_rate", Unit::MegabytesPerSecond),
    ("network.upload_rate", Unit::MegabytesPerSecond),
    ("network.latency", Unit::Milliseconds),
    ("network.jitter", Unit::Milliseconds),
    ("network.packet_loss", Unit::Percent),
    ("input.mouse_polling", Unit::Hertz),
    ("input.consistency", Unit::Percent),
    ("display.refresh", Unit::Hertz),
    // Interpretada por `modules::mouse`; em milissegundos do orçamento por `modules::latencia`.
    ("input.polling_rate", Unit::Hertz),
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

        let temperatura = t.get("cpu.temperature").expect("catálogo");
        assert_eq!(temperatura.value, None);
        assert_eq!(temperatura.quality, Quality::Unknown);
        assert!(temperatura.reason.is_some());
    }

    #[test]
    fn zero_medido_e_diferente_de_nao_medido() {
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

        // Desconhecido chega como null, não ausente: a tela diz "não medido" em vez de não dizer nada.
        assert!(json.contains("\"value\":null"));
        assert!(json.contains("\"quality\":\"UNKNOWN\""));
    }
}
