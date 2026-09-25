// Baseline: um retrato da telemetria com a carga em que foi tirado e a identidade da máquina (no FiveM o menu
// roda a 300 e a rua a 90: comparar os dois fabrica ganho). Recusa: perfil de carga diferente, máquina diferente
// (placa, driver, Windows, plano) e métrica sem valor dos dois lados. Não decide se a mudança foi boa: entrega a
// diferença com a ressalva de cada métrica.

use serde::{Deserialize, Serialize};
use std::path::Path;

use super::telemetry::{Quality, Telemetry};

/// Versão desconhecida é ERRO: "não há baseline" apagaria o passado do cliente na próxima gravação.
pub const SCHEMA_VERSION: u32 = 1;

/// Os mesmos 3% de `prova.rs`: duas medições seguidas sem mexer em nada variam nessa ordem.
pub const RUIDO_PCT: f64 = 3.0;

pub const IDADE_FIRME_MS: u64 = 5_000;

/// Não detectado sozinho: a carga diz o que a máquina faz, não o que se QUIS medir, e retrato rotulado errado
/// autoriza uma comparação que não devia existir.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Perfil {
    Ocioso,
    AreaDeTrabalho,
    Cpu,
    Gpu,
    Jogo,
    Disco,
}

impl Perfil {
    pub fn nome(self) -> &'static str {
        match self {
            Perfil::Ocioso => "ocioso",
            Perfil::AreaDeTrabalho => "área de trabalho",
            Perfil::Cpu => "carga de processador",
            Perfil::Gpu => "carga de placa",
            Perfil::Jogo => "jogo",
            Perfil::Disco => "carga de disco",
        }
    }
}

/// Sem isto, trocar placa, driver ou plano por fora vira mérito do Otimiza por acidente.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Identidade {
    pub cpu: String,
    pub gpu: String,
    pub nucleos_logicos: usize,
    pub ram_gb: f64,
    pub windows_build: u32,
    pub plano_de_energia: Option<String>,
    /// Separa "antes" de "depois" quando o resto é igual.
    pub mudancas_aplicadas: usize,
}

impl Identidade {
    /// A contagem de mudanças do Otimiza não entra: é o que se espera que mude.
    pub fn diferencas(&self, outra: &Identidade) -> Vec<String> {
        let mut fora = Vec::new();

        let mut conferir = |nome: &str, a: String, b: String| {
            if a != b {
                fora.push(format!("{nome}: era {a}, agora é {b}"));
            }
        };

        conferir("processador", self.cpu.clone(), outra.cpu.clone());
        conferir("placa de vídeo", self.gpu.clone(), outra.gpu.clone());
        conferir(
            "núcleos lógicos",
            self.nucleos_logicos.to_string(),
            outra.nucleos_logicos.to_string(),
        );
        conferir(
            "memória",
            format!("{:.1} GB", self.ram_gb),
            format!("{:.1} GB", outra.ram_gb),
        );
        conferir(
            "build do Windows",
            self.windows_build.to_string(),
            outra.windows_build.to_string(),
        );
        conferir(
            "plano de energia",
            self.plano_de_energia
                .clone()
                .unwrap_or_else(|| "desconhecido".into()),
            outra
                .plano_de_energia
                .clone()
                .unwrap_or_else(|| "desconhecido".into()),
        );

        fora
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Baseline {
    pub schema_version: u32,
    pub quando: u64,
    pub perfil: Perfil,
    pub identidade: Identidade,
    pub telemetria: Telemetry,
    /// Vazio num retrato de coleta só: continua valendo, mas não diz se a diferença passa do ruído desta máquina.
    #[serde(default)]
    pub incerteza: Vec<super::repeticoes::Resumo>,
}

impl Baseline {
    pub fn novo(
        quando: u64,
        perfil: Perfil,
        identidade: Identidade,
        telemetria: Telemetry,
    ) -> Self {
        Baseline {
            schema_version: SCHEMA_VERSION,
            quando,
            perfil,
            identidade,
            telemetria,
            incerteza: Vec::new(),
        }
    }

    pub fn com_incerteza(mut self, incerteza: Vec<super::repeticoes::Resumo>) -> Self {
        self.incerteza = incerteza;
        self
    }

    fn incerteza_de(&self, id: &str) -> Option<&super::repeticoes::Resumo> {
        self.incerteza.iter().find(|r| r.id == id)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Recusa {
    MaquinaDiferente(Vec<String>),
    PerfilDiferente { antes: Perfil, depois: Perfil },
    VersaoDesconhecida { encontrada: u32 },
    NadaEmComum,
}

impl Recusa {
    pub fn explicacao(&self) -> String {
        match self {
            Recusa::MaquinaDiferente(mudou) => format!(
                "A máquina mudou entre as duas medições ({}), então a diferença \
                 não é sobre o que o Otimiza fez.",
                mudou.join("; ")
            ),
            Recusa::PerfilDiferente { antes, depois } => format!(
                "Um retrato foi tirado com {} e o outro com {}. Isso mede a diferença \
                 entre as duas situações, não o efeito de um ajuste.",
                antes.nome(),
                depois.nome()
            ),
            Recusa::VersaoDesconhecida { encontrada } => format!(
                "Este retrato foi gravado no formato {encontrada}, que esta versão do \
                 Otimiza não sabe ler."
            ),
            Recusa::NadaEmComum => "Nenhuma métrica foi medida nos dois retratos, então \
                                    não há o que comparar."
                .to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Delta {
    pub id: String,
    pub antes: f64,
    pub depois: f64,
    /// `None` quando o "antes" era zero.
    pub variacao_pct: Option<f64>,
    pub firme: bool,
    pub ressalva: Option<String>,
    pub acima_do_ruido: bool,
    /// Aprovado pelo limiar fixo é palpite calibrado; pelos intervalos, é medição. A tela precisa separar.
    pub criterio: Criterio,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Criterio {
    Intervalos { folga: Option<f64> },
    LimiarFixo,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NaoComparavel {
    pub id: String,
    pub motivo: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Comparacao {
    pub perfil: Perfil,
    pub deltas: Vec<Delta>,
    pub nao_comparaveis: Vec<NaoComparavel>,
    pub firmes: usize,
}

pub fn comparar(antes: &Baseline, depois: &Baseline) -> Result<Comparacao, Recusa> {
    for b in [antes, depois] {
        if b.schema_version != SCHEMA_VERSION {
            return Err(Recusa::VersaoDesconhecida {
                encontrada: b.schema_version,
            });
        }
    }

    if antes.perfil != depois.perfil {
        return Err(Recusa::PerfilDiferente {
            antes: antes.perfil,
            depois: depois.perfil,
        });
    }

    let mudou = antes.identidade.diferencas(&depois.identidade);
    if !mudou.is_empty() {
        return Err(Recusa::MaquinaDiferente(mudou));
    }

    let mut deltas = Vec::new();
    let mut nao_comparaveis = Vec::new();

    for (id, a) in &antes.telemetria.metrics {
        let Some(d) = depois.telemetria.metrics.get(id) else {
            nao_comparaveis.push(NaoComparavel {
                id: id.clone(),
                motivo: "não existe no retrato mais novo".to_string(),
            });
            continue;
        };

        // Um lado desconhecido não vale como zero nem como "igual".
        let (Some(va), Some(vd)) = (a.value, d.value) else {
            nao_comparaveis.push(NaoComparavel {
                id: id.clone(),
                motivo: match (a.value.is_some(), d.value.is_some()) {
                    (false, false) => "não foi medida em nenhum dos dois".to_string(),
                    (true, false) => "não foi medida no retrato mais novo".to_string(),
                    _ => "não foi medida no retrato antigo".to_string(),
                },
            });
            continue;
        };

        // Unidade diferente para o mesmo id é defeito, não diferença.
        if a.unit != d.unit {
            nao_comparaveis.push(NaoComparavel {
                id: id.clone(),
                motivo: "a unidade mudou entre os dois retratos".to_string(),
            });
            continue;
        }

        let ressalva = ressalva_da_comparacao(a, d);
        let variacao_pct = (va != 0.0).then(|| (vd - va) / va.abs() * 100.0);

        // O ruído medido ganha do suposto; o limiar fixo fica para quem mediu uma vez só.
        let (acima_do_ruido, criterio) = match (antes.incerteza_de(id), depois.incerteza_de(id)) {
            (Some(ia), Some(id_)) => match super::repeticoes::comparar(ia, id_) {
                super::repeticoes::Diferenca::Real { folga, .. } => {
                    (true, Criterio::Intervalos { folga: Some(folga) })
                }
                super::repeticoes::Diferenca::Indistinguivel { .. } => {
                    (false, Criterio::Intervalos { folga: None })
                }
                super::repeticoes::Diferenca::SemRepeticoes { .. } => (
                    variacao_pct.is_some_and(|p| p.abs() >= RUIDO_PCT),
                    Criterio::LimiarFixo,
                ),
            },
            _ => (
                variacao_pct.is_some_and(|p| p.abs() >= RUIDO_PCT),
                Criterio::LimiarFixo,
            ),
        };

        deltas.push(Delta {
            id: id.clone(),
            antes: va,
            depois: vd,
            variacao_pct,
            firme: ressalva.is_none(),
            ressalva,
            acima_do_ruido,
            criterio,
        });
    }

    if deltas.is_empty() {
        return Err(Recusa::NadaEmComum);
    }

    // Delta sem porcentagem vai para o fim.
    deltas.sort_by(|x, y| {
        let px = x.variacao_pct.map(f64::abs).unwrap_or(-1.0);
        let py = y.variacao_pct.map(f64::abs).unwrap_or(-1.0);
        py.partial_cmp(&px).unwrap_or(std::cmp::Ordering::Equal)
    });

    let firmes = deltas.iter().filter(|d| d.firme).count();

    Ok(Comparacao {
        perfil: antes.perfil,
        deltas,
        nao_comparaveis,
        firmes,
    })
}

fn ressalva_da_comparacao(
    a: &super::telemetry::Metric,
    d: &super::telemetry::Metric,
) -> Option<String> {
    let estimada = a.quality == Quality::Estimated || d.quality == Quality::Estimated;
    let velha = [a.age_ms, d.age_ms]
        .into_iter()
        .flatten()
        .any(|ms| ms > IDADE_FIRME_MS);

    match (estimada, velha) {
        (false, false) => None,
        (true, false) => Some("pelo menos um dos lados é estimativa, não medição".to_string()),
        (false, true) => Some("pelo menos um dos lados já era leitura velha".to_string()),
        (true, true) => Some("um dos lados é estimativa, e pelo menos um já era velho".to_string()),
    }
}

/// Existe e não se lê é ERRO: a lista vazia fingiria resposta e a gravação seguinte apagaria o que havia.
pub fn ler_de(caminho: &Path) -> Result<Vec<Baseline>, String> {
    // Temporário sobrevivente (gravação interrompida) não é apagado nem promovido: são decisões sobre os dados do
    // cliente.
    let pendente = caminho.with_extension("json.pending");
    if pendente.exists() {
        return Err(format!(
            "há uma gravação de baseline interrompida em {}. O arquivo bom não foi tocado; \
             remova o pendente à mão depois de conferir.",
            pendente.display()
        ));
    }

    match std::fs::read_to_string(caminho) {
        Ok(bruto) => {
            let lidos: Vec<Baseline> = serde_json::from_str(&bruto)
                .map_err(|e| format!("o arquivo de baselines está ilegível: {e}"))?;

            if let Some(b) = lidos.iter().find(|b| b.schema_version != SCHEMA_VERSION) {
                return Err(Recusa::VersaoDesconhecida {
                    encontrada: b.schema_version,
                }
                .explicacao());
            }

            Ok(lidos)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(format!("não consegui ler os baselines: {e}")),
    }
}

/// Um retrato por perfil. Temporário e rename: escrever direto no arquivo bom, com queda de energia no meio,
/// deixa o cliente sem passado e sem presente.
pub fn guardar_em(caminho: &Path, baseline: Baseline) -> Result<(), String> {
    let mut todos = ler_de(caminho)?;
    todos.retain(|b| b.perfil != baseline.perfil);
    todos.push(baseline);
    todos.sort_by_key(|b| b.perfil);

    if let Some(pasta) = caminho.parent() {
        std::fs::create_dir_all(pasta)
            .map_err(|e| format!("não consegui criar a pasta de dados: {e}"))?;
    }

    let bruto = serde_json::to_string(&todos)
        .map_err(|e| format!("não consegui preparar os baselines: {e}"))?;

    let pendente = caminho.with_extension("json.pending");

    // `create_new`: temporário existente é gravação em curso ou morta, e passar por cima seria pior.
    {
        use std::io::Write;

        let mut arquivo = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&pendente)
            .map_err(|e| format!("não consegui abrir o arquivo temporário: {e}"))?;

        arquivo
            .write_all(bruto.as_bytes())
            .map_err(|e| format!("não consegui gravar o temporário: {e}"))?;

        // `sync_all` antes do rename: renomear com o conteúdo ainda em cache troca um arquivo bom por um vazio.
        arquivo
            .sync_all()
            .map_err(|e| format!("não consegui confirmar a gravação em disco: {e}"))?;
    }

    std::fs::rename(&pendente, caminho)
        .map_err(|e| format!("não consegui substituir o arquivo de baselines: {e}"))
}

fn caminho_padrao() -> std::path::PathBuf {
    let base = std::env::var("APPDATA")
        .or_else(|_| std::env::var("HOME"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."));

    base.join("pc-optimizer").join("baselines.json")
}

pub fn ler() -> Result<Vec<Baseline>, String> {
    ler_de(&caminho_padrao())
}

pub fn guardar(baseline: Baseline) -> Result<(), String> {
    guardar_em(&caminho_padrao(), baseline)
}

/// Só leitura barata: plano de energia pelo registro, não pelo `powercfg`, que abre processo.
#[cfg(target_os = "windows")]
pub fn identidade_desta_maquina(mudancas_aplicadas: usize) -> Identidade {
    let h = super::windows::hardware::profile();
    let maquina = super::windows::planoenergia::detectar();

    Identidade {
        cpu: h.cpu_name.clone(),
        gpu: h.gpu_name.clone(),
        nucleos_logicos: h.logical_cores,
        ram_gb: (h.total_ram_gb * 10.0).round() / 10.0,
        windows_build: maquina.build_do_windows,
        plano_de_energia: plano_ativo_do_registro(),
        mudancas_aplicadas,
    }
}

#[cfg(target_os = "windows")]
fn plano_ativo_do_registro() -> Option<String> {
    use crate::modules::changelog::PreviousValue;

    match super::windows::registry::read(
        "HKLM",
        r"SYSTEM\CurrentControlSet\Control\Power\User\PowerSchemes",
        "ActivePowerScheme",
    ) {
        Ok(PreviousValue::Text(guid)) => Some(guid),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::telemetry::{Metric, Unit};

    fn identidade() -> Identidade {
        Identidade {
            cpu: "Ryzen 5 5600".into(),
            gpu: "RTX 3060".into(),
            nucleos_logicos: 12,
            ram_gb: 16.0,
            windows_build: 19045,
            plano_de_energia: Some("Otimiza".into()),
            mudancas_aplicadas: 0,
        }
    }

    /// A unidade tem de bater com a do catálogo: o contrato recusa a gravação divergente.
    fn unidade(id: &str) -> Unit {
        if id.starts_with("fps.") {
            Unit::Fps
        } else {
            Unit::Percent
        }
    }

    fn retrato(perfil: Perfil, pares: &[(&str, f64, Quality)]) -> Baseline {
        let mut t = Telemetry::new(0, None);

        for (id, valor, qualidade) in pares {
            let u = unidade(id);
            let m = match qualidade {
                Quality::Measured => Metric::measured(*valor, u, "teste"),
                Quality::Estimated => Metric::estimated(*valor, u, "teste", "derivado"),
                Quality::Unknown => Metric::unknown(u, "sem provedor"),
            };
            t.set(id, m);
        }

        Baseline::novo(0, perfil, identidade(), t.finish(0))
    }

    #[test]
    fn carga_diferente_nao_e_comparacao() {
        let antes = retrato(
            Perfil::Ocioso,
            &[("cpu.usage.overall", 5.0, Quality::Measured)],
        );
        let depois = retrato(
            Perfil::Jogo,
            &[("cpu.usage.overall", 80.0, Quality::Measured)],
        );

        let erro = comparar(&antes, &depois).expect_err("perfis diferentes");
        assert!(matches!(erro, Recusa::PerfilDiferente { .. }));
        assert!(erro.explicacao().contains("ocioso"));
        assert!(erro.explicacao().contains("jogo"));
    }

    #[test]
    fn maquina_diferente_nao_e_comparacao() {
        let antes = retrato(
            Perfil::Jogo,
            &[("cpu.usage.overall", 50.0, Quality::Measured)],
        );
        let mut depois = retrato(
            Perfil::Jogo,
            &[("cpu.usage.overall", 40.0, Quality::Measured)],
        );
        depois.identidade.gpu = "RTX 4070".into();

        let erro = comparar(&antes, &depois).expect_err("hardware diferente");
        match erro {
            Recusa::MaquinaDiferente(mudou) => {
                assert_eq!(mudou.len(), 1);
                assert!(mudou[0].contains("placa de vídeo"), "{}", mudou[0]);
            }
            outro => panic!("esperava máquina diferente, veio {outro:?}"),
        }
    }

    #[test]
    fn a_contagem_de_mudancas_nao_impede_comparar() {
        let antes = retrato(
            Perfil::Jogo,
            &[("cpu.usage.overall", 50.0, Quality::Measured)],
        );
        let mut depois = retrato(
            Perfil::Jogo,
            &[("cpu.usage.overall", 40.0, Quality::Measured)],
        );
        depois.identidade.mudancas_aplicadas = 7;

        assert!(comparar(&antes, &depois).is_ok());
    }

    #[test]
    fn metrica_sem_os_dois_lados_nao_entra() {
        let antes = retrato(
            Perfil::Jogo,
            &[
                ("cpu.usage.overall", 50.0, Quality::Measured),
                ("gpu.usage", 90.0, Quality::Measured),
            ],
        );
        let depois = retrato(
            Perfil::Jogo,
            &[
                ("cpu.usage.overall", 40.0, Quality::Measured),
                ("gpu.usage", 0.0, Quality::Unknown),
            ],
        );

        let c = comparar(&antes, &depois).expect("comparável");

        assert_eq!(c.deltas.len(), 1);
        assert_eq!(c.deltas[0].id, "cpu.usage.overall");

        let gpu = c
            .nao_comparaveis
            .iter()
            .find(|n| n.id == "gpu.usage")
            .expect("gpu listada");
        assert!(gpu.motivo.contains("mais novo"), "{}", gpu.motivo);
    }

    #[test]
    fn estimativa_e_leitura_velha_enfraquecem_a_comparacao() {
        let antes = retrato(Perfil::Jogo, &[("gpu.usage", 40.0, Quality::Measured)]);
        let depois = retrato(Perfil::Jogo, &[("gpu.usage", 80.0, Quality::Estimated)]);

        let c = comparar(&antes, &depois).expect("comparável");
        let d = c
            .deltas
            .iter()
            .find(|d| d.id == "gpu.usage")
            .expect("delta");

        assert!(!d.firme);
        assert!(d.ressalva.as_deref().unwrap().contains("estimativa"));
        assert_eq!(c.firmes, 0);

        assert_eq!(d.antes, 40.0);
        assert_eq!(d.depois, 80.0);
        assert_eq!(d.variacao_pct, Some(100.0));
    }

    #[test]
    fn leitura_velha_tambem_enfraquece() {
        let mut a = Telemetry::new(0, None);
        a.set("gpu.usage", Metric::measured(40.0, Unit::Percent, "pdh"));
        let antes = Baseline::novo(0, Perfil::Jogo, identidade(), a.finish(0));

        let mut d = Telemetry::new(0, None);
        d.set(
            "gpu.usage",
            Metric::measured(44.0, Unit::Percent, "pdh").com_idade(20_000),
        );
        let depois = Baseline::novo(0, Perfil::Jogo, identidade(), d.finish(0));

        let c = comparar(&antes, &depois).expect("comparável");
        let delta = c
            .deltas
            .iter()
            .find(|x| x.id == "gpu.usage")
            .expect("delta");

        assert!(!delta.firme);
        assert!(delta.ressalva.is_some());
    }

    #[test]
    fn com_repeticoes_o_ruido_medido_ganha_do_suposto() {
        use crate::modules::repeticoes::resumir;

        let antes = retrato(
            Perfil::Jogo,
            &[("cpu.usage.overall", 84.0, Quality::Measured)],
        )
        .com_incerteza(vec![
            resumir("cpu.usage.overall", &[78.0, 84.0, 90.0]).expect("três")
        ]);
        let depois = retrato(
            Perfil::Jogo,
            &[("cpu.usage.overall", 87.0, Quality::Measured)],
        )
        .com_incerteza(vec![
            resumir("cpu.usage.overall", &[81.0, 87.0, 93.0]).expect("três")
        ]);

        let c = comparar(&antes, &depois).expect("comparável");
        let d = &c.deltas[0];

        assert!(
            (d.variacao_pct.unwrap() - 3.57).abs() < 0.01,
            "a variação continua sendo 3,57%"
        );
        assert!(!d.acima_do_ruido, "mas as medições não distinguem os dois");
        assert!(matches!(d.criterio, Criterio::Intervalos { folga: None }));
    }

    #[test]
    fn com_repeticoes_apertadas_um_ganho_pequeno_e_real() {
        use crate::modules::repeticoes::resumir;

        let antes = retrato(Perfil::Jogo, &[("fps.average", 100.0, Quality::Measured)])
            .com_incerteza(vec![
                resumir("fps.average", &[100.0, 100.1, 99.9, 100.0]).expect("quatro")
            ]);
        let depois = retrato(Perfil::Jogo, &[("fps.average", 102.0, Quality::Measured)])
            .com_incerteza(vec![
                resumir("fps.average", &[102.0, 102.1, 101.9, 102.0]).expect("quatro")
            ]);

        let c = comparar(&antes, &depois).expect("comparável");
        let d = &c.deltas[0];

        assert_eq!(d.variacao_pct, Some(2.0));
        assert!(
            d.acima_do_ruido,
            "o limiar fixo teria descartado este ganho"
        );
        assert!(matches!(
            d.criterio,
            Criterio::Intervalos { folga: Some(_) }
        ));
    }

    #[test]
    fn sem_repeticoes_o_criterio_diz_que_e_limiar_fixo() {
        let antes = retrato(Perfil::Jogo, &[("fps.average", 100.0, Quality::Measured)]);
        let depois = retrato(Perfil::Jogo, &[("fps.average", 110.0, Quality::Measured)]);

        let c = comparar(&antes, &depois).expect("comparável");
        assert!(c.deltas[0].acima_do_ruido);
        assert_eq!(c.deltas[0].criterio, Criterio::LimiarFixo);
    }

    #[test]
    fn diferenca_dentro_do_ruido_nao_e_ganho() {
        let antes = retrato(
            Perfil::Jogo,
            &[("cpu.usage.overall", 50.0, Quality::Measured)],
        );
        let depois = retrato(
            Perfil::Jogo,
            &[("cpu.usage.overall", 51.0, Quality::Measured)],
        );

        let c = comparar(&antes, &depois).expect("comparável");
        let d = &c.deltas[0];

        assert_eq!(d.variacao_pct, Some(2.0));
        assert!(!d.acima_do_ruido, "2% é ruído de medição, não ganho");
    }

    #[test]
    fn nao_existe_porcentagem_sobre_zero() {
        let antes = retrato(
            Perfil::Jogo,
            &[("cpu.usage.overall", 0.0, Quality::Measured)],
        );
        let depois = retrato(
            Perfil::Jogo,
            &[("cpu.usage.overall", 30.0, Quality::Measured)],
        );

        let c = comparar(&antes, &depois).expect("comparável");
        assert_eq!(c.deltas[0].variacao_pct, None);
        assert!(!c.deltas[0].acima_do_ruido);
        assert_eq!(c.deltas[0].depois, 30.0);
    }

    #[test]
    fn formato_desconhecido_e_erro_e_nao_lista_vazia() {
        let mut antes = retrato(
            Perfil::Jogo,
            &[("cpu.usage.overall", 50.0, Quality::Measured)],
        );
        antes.schema_version = 99;
        let depois = retrato(
            Perfil::Jogo,
            &[("cpu.usage.overall", 40.0, Quality::Measured)],
        );

        let erro = comparar(&antes, &depois).expect_err("versão desconhecida");
        assert!(matches!(
            erro,
            Recusa::VersaoDesconhecida { encontrada: 99 }
        ));
    }

    fn pasta(nome: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("otimiza-baseline-{nome}"));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).expect("pasta de teste");
        p
    }

    #[test]
    fn um_perfil_um_retrato() {
        let arquivo = pasta("perfil").join("baselines.json");

        guardar_em(
            &arquivo,
            retrato(
                Perfil::Ocioso,
                &[("cpu.usage.overall", 5.0, Quality::Measured)],
            ),
        )
        .expect("grava");
        guardar_em(
            &arquivo,
            retrato(
                Perfil::Jogo,
                &[("cpu.usage.overall", 80.0, Quality::Measured)],
            ),
        )
        .expect("grava");
        guardar_em(
            &arquivo,
            retrato(
                Perfil::Ocioso,
                &[("cpu.usage.overall", 7.0, Quality::Measured)],
            ),
        )
        .expect("grava");

        let todos = ler_de(&arquivo).expect("lê");
        assert_eq!(todos.len(), 2, "um por perfil");

        let ocioso = todos
            .iter()
            .find(|b| b.perfil == Perfil::Ocioso)
            .expect("ocioso");
        assert_eq!(ocioso.telemetria.value("cpu.usage.overall"), Some(7.0));
    }

    #[test]
    fn arquivo_inexistente_e_lista_vazia_mas_ilegivel_e_erro() {
        let base = pasta("ilegivel");

        assert_eq!(ler_de(&base.join("nao-existe.json")).unwrap().len(), 0);

        let quebrado = base.join("quebrado.json");
        std::fs::write(&quebrado, "{isto não é json}").expect("escreve");

        let erro = ler_de(&quebrado).expect_err("ilegível");
        assert!(erro.contains("ilegível"), "{erro}");
    }

    #[test]
    fn gravacao_interrompida_nao_e_apagada_em_silencio() {
        let base = pasta("pendente");
        let arquivo = base.join("baselines.json");

        guardar_em(
            &arquivo,
            retrato(
                Perfil::Jogo,
                &[("cpu.usage.overall", 50.0, Quality::Measured)],
            ),
        )
        .expect("grava");

        std::fs::write(arquivo.with_extension("json.pending"), "meio arquivo").expect("escreve");

        let erro = ler_de(&arquivo).expect_err("pendente bloqueia");
        assert!(erro.contains("interrompida"), "{erro}");

        let bruto = std::fs::read_to_string(&arquivo).expect("o bom sobreviveu");
        assert!(bruto.contains("cpu.usage.overall"));
    }

    #[test]
    fn nada_em_comum_e_recusa_e_nao_comparacao_vazia() {
        let antes = retrato(
            Perfil::Jogo,
            &[("cpu.usage.overall", 0.0, Quality::Unknown)],
        );
        let depois = retrato(
            Perfil::Jogo,
            &[("cpu.usage.overall", 0.0, Quality::Unknown)],
        );

        let erro = comparar(&antes, &depois).expect_err("nada medido");
        assert!(matches!(erro, Recusa::NadaEmComum));
    }
}
