// Linha do tempo de medições ("estava bom semana passada"), com a identidade da máquina em cada registro.
// Coincidência no tempo NÃO é causa: sai como SUSPEITOS, com o aviso e o caminho de provar. Compara as DUAS
// ÚLTIMAS medições, nunca o melhor de sempre (um extremo escolhido acha regressão em toda máquina). As mudanças
// vêm do `changelog` na hora de responder, não são gravadas aqui. O descarte pelo teto é contado.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::baseline::Identidade;
use super::changelog::AppliedOptimization;
use super::repeticoes::{comparar, Diferenca, Resumo};

pub const VERSAO: u32 = 1;

/// Uma lista, não as cinquenta do contrato: todas encheriam o teto em nove capturas.
pub const METRICAS_GUARDADAS: &[&str] = &[
    "fps.average",
    "fps.low_1pct",
    "frametime.mean",
    "frametime.p99",
    "frametime.stutters_per_minute",
    "cpu.usage.overall",
    "cpu.temperature",
    "gpu.usage",
    "vram.shared_used",
    "storage.latency",
];

/// Tabela, e não palpite de quem chama: errar o sinal chama de regressão a melhora que o produto produziu.
/// `None` fora da lista: não há regressão a julgar.
pub fn maior_e_melhor(id: &str) -> Option<bool> {
    match id {
        "fps.average" | "fps.low_1pct" => Some(true),
        "frametime.mean"
        | "frametime.p99"
        | "frametime.stutters_per_minute"
        | "cpu.usage.overall"
        | "cpu.temperature"
        | "gpu.usage"
        | "vram.shared_used"
        | "storage.latency" => Some(false),
        _ => None,
    }
}

/// Mais de um ano a uma medição e uma mudança por dia, em poucos megabytes.
pub const LIMITE_DE_REGISTROS: usize = 500;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Evento {
    /// O resumo, não as amostras: a margem basta para julgar depois.
    Medicao(Resumo),
    Mudanca {
        nome: String,
        aplicada: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Registro {
    pub quando: u64,
    /// Em CADA registro: trocar placa ou plano no meio do histórico é o que explica uma queda.
    pub identidade: Identidade,
    pub evento: Evento,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Historico {
    #[serde(default = "versao_antiga")]
    pub schema_version: u32,
    #[serde(default)]
    pub registros: Vec<Registro>,
    /// Sem isto, "nenhuma mudança entre as medições" seria indistinguível de "o arquivo encheu".
    #[serde(default)]
    pub descartados: usize,
}

fn versao_antiga() -> u32 {
    0
}

impl Default for Historico {
    fn default() -> Self {
        Historico {
            schema_version: VERSAO,
            registros: Vec::new(),
            descartados: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Suspeitos {
    pub mudancas: Vec<String>,
    /// Quando não está vazia, costuma pesar mais que tudo o que o produto fez.
    pub maquina_mudou: Vec<String>,
    pub descartados: usize,
    pub aviso: String,
    pub como_provar: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Regressao {
    pub id: String,
    pub anterior: Resumo,
    pub atual: Resumo,
    pub diferenca: Diferenca,
    pub piorou: bool,
    pub suspeitos: Suspeitos,
}

const AVISO: &str = "Estas mudanças apenas CAEM NO INTERVALO entre as duas medições. Isso não \
                     prova que alguma delas causou a diferença: no mesmo período o Windows pode \
                     ter atualizado, o driver mudado e o jogo recebido correção.";

const COMO_PROVAR: &str = "Para provar, desfaça uma de cada vez e meça de novo com repetição. É a \
                           única forma de separar a mudança do resto — e é o que o autoajuste faz.";

impl Historico {
    pub fn anotar(&mut self, quando: u64, identidade: Identidade, evento: Evento) {
        self.schema_version = VERSAO;
        self.registros.push(Registro {
            quando,
            identidade,
            evento,
        });

        let sobra = self.registros.len().saturating_sub(LIMITE_DE_REGISTROS);
        if sobra > 0 {
            self.registros.drain(0..sobra);
            self.descartados += sobra;
        }
    }

    /// A identidade de cada mudança é a da medição mais próxima ANTES dela; sem medição anterior, fica de fora.
    pub fn com_mudancas(&self, aplicadas: &[AppliedOptimization]) -> Historico {
        let mut copia = self.clone();

        for a in aplicadas {
            let Some(identidade) = self.identidade_em_ou_antes(a.timestamp).cloned() else {
                continue;
            };

            copia.registros.push(Registro {
                quando: a.timestamp,
                identidade,
                evento: Evento::Mudanca {
                    nome: a.name.clone(),
                    aplicada: true,
                },
            });
        }

        copia.registros.sort_by_key(|r| r.quando);
        copia
    }

    pub fn duas_ultimas(&self, id: &str) -> Option<(&Resumo, &Resumo)> {
        let mut encontradas = self.registros.iter().rev().filter_map(|r| match &r.evento {
            Evento::Medicao(resumo) if resumo.id == id => Some(resumo),
            _ => None,
        });

        let atual = encontradas.next()?;
        let anterior = encontradas.next()?;

        Some((anterior, atual))
    }

    /// Aberto nas pontas: mudança no mesmo segundo da medição anterior já valia no "antes".
    pub fn suspeitos_entre(&self, inicio: u64, fim: u64) -> Suspeitos {
        let no_intervalo = || {
            self.registros
                .iter()
                .filter(move |r| r.quando > inicio && r.quando < fim)
        };

        let mudancas = no_intervalo()
            .filter_map(|r| match &r.evento {
                Evento::Mudanca { nome, aplicada } => Some(format!(
                    "{nome} ({})",
                    if *aplicada { "aplicada" } else { "desfeita" }
                )),
                _ => None,
            })
            .collect();

        // Compara as pontas, não registro a registro: importa se a máquina de hoje é a do "antes".
        let maquina_mudou = match (
            self.identidade_em_ou_antes(inicio),
            self.identidade_em_ou_antes(fim),
        ) {
            (Some(a), Some(b)) => a.diferencas(b),
            _ => Vec::new(),
        };

        Suspeitos {
            mudancas,
            maquina_mudou,
            descartados: self.descartados,
            aviso: AVISO.to_string(),
            como_provar: COMO_PROVAR.to_string(),
        }
    }

    fn identidade_em_ou_antes(&self, quando: u64) -> Option<&Identidade> {
        self.registros
            .iter()
            .rev()
            .find(|r| r.quando <= quando)
            .map(|r| &r.identidade)
    }

    /// `maior_e_melhor` sem padrão: um padrão chamaria de regressão toda melhora de latência.
    pub fn regressao(&self, id: &str, maior_e_melhor: bool) -> Option<Regressao> {
        let (anterior, atual) = self.duas_ultimas(id)?;
        let diferenca = comparar(anterior, atual);

        let piorou = match &diferenca {
            Diferenca::Real { delta, .. } => {
                if maior_e_melhor {
                    *delta < 0.0
                } else {
                    *delta > 0.0
                }
            }
            // Não provar uma queda é diferente de provar que não houve: aqui não se acusa.
            _ => false,
        };

        let (inicio, fim) = self.instantes_das_duas_ultimas(id)?;

        Some(Regressao {
            id: id.to_string(),
            anterior: anterior.clone(),
            atual: atual.clone(),
            diferenca,
            piorou,
            suspeitos: self.suspeitos_entre(inicio, fim),
        })
    }

    fn instantes_das_duas_ultimas(&self, id: &str) -> Option<(u64, u64)> {
        let mut quandos = self.registros.iter().rev().filter_map(|r| match &r.evento {
            Evento::Medicao(resumo) if resumo.id == id => Some(r.quando),
            _ => None,
        });

        let fim = quandos.next()?;
        let inicio = quandos.next()?;

        Some((inicio, fim))
    }
}

/// Ilegível é ERRO: tratar como vazio faria o próximo registro apagar o histórico do cliente.
pub fn ler_de(caminho: &Path) -> Result<Historico, String> {
    match std::fs::read_to_string(caminho) {
        Ok(bruto) => serde_json::from_str(&bruto)
            .map_err(|e| format!("o histórico existe mas não pôde ser lido: {e}")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Historico::default()),
        Err(e) => Err(format!("não consegui abrir o histórico: {e}")),
    }
}

/// `create_new` e `sync_all` antes do rename: renomear com o conteúdo ainda em cache troca o histórico por um vazio.
pub fn guardar_em(caminho: &Path, historico: &Historico) -> Result<(), String> {
    use std::io::Write;

    if let Some(pasta) = caminho.parent() {
        std::fs::create_dir_all(pasta)
            .map_err(|e| format!("não consegui criar a pasta de dados: {e}"))?;
    }

    let bruto = serde_json::to_string(historico)
        .map_err(|e| format!("não consegui preparar o histórico: {e}"))?;

    let pendente = caminho.with_extension("json.pending");

    {
        let mut arquivo = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&pendente)
            .map_err(|e| format!("não consegui abrir o arquivo temporário: {e}"))?;

        arquivo
            .write_all(bruto.as_bytes())
            .map_err(|e| format!("não consegui gravar o temporário: {e}"))?;

        arquivo
            .sync_all()
            .map_err(|e| format!("não consegui confirmar a gravação em disco: {e}"))?;
    }

    std::fs::rename(&pendente, caminho)
        .map_err(|e| format!("não consegui substituir o histórico: {e}"))
}

fn caminho_padrao() -> std::path::PathBuf {
    let base = std::env::var("APPDATA")
        .or_else(|_| std::env::var("HOME"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."));

    base.join("pc-optimizer").join("historico.json")
}

pub fn ler() -> Result<Historico, String> {
    ler_de(&caminho_padrao())
}

pub fn guardar(historico: &Historico) -> Result<(), String> {
    guardar_em(&caminho_padrao(), historico)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::repeticoes::resumir;

    fn identidade(gpu: &str) -> Identidade {
        Identidade {
            cpu: "i5-3470".to_string(),
            gpu: gpu.to_string(),
            nucleos_logicos: 4,
            ram_gb: 12.0,
            windows_build: 19045,
            plano_de_energia: Some("equilibrado".to_string()),
            mudancas_aplicadas: 0,
        }
    }

    fn medicao(centro: f64) -> Evento {
        Evento::Medicao(
            resumir(
                "fps.average",
                &[centro - 0.4, centro, centro + 0.4, centro - 0.2, centro + 0.2],
            )
            .expect("resumo"),
        )
    }

    fn mudanca(nome: &str, quando: u64) -> AppliedOptimization {
        AppliedOptimization {
            optimization_id: nome.to_string(),
            name: nome.to_string(),
            timestamp: quando,
            changes: Vec::new(),
        }
    }

    fn com_queda() -> Historico {
        let mut h = Historico::default();
        h.anotar(100, identidade("GTX 770"), medicao(60.0));
        h.anotar(200, identidade("GTX 770"), medicao(48.0));
        h.com_mudancas(&[mudanca("plano de energia Otimiza", 150)])
    }

    #[test]
    fn a_queda_e_medida_e_os_suspeitos_saem_junto() {
        let r = com_queda().regressao("fps.average", true).expect("regressão");

        assert!(r.piorou);
        assert_eq!(r.suspeitos.mudancas.len(), 1);
        assert!(r.suspeitos.mudancas[0].contains("aplicada"));
    }

    #[test]
    fn suspeito_nao_e_culpado() {
        let r = com_queda().regressao("fps.average", true).expect("regressão");

        assert!(r.suspeitos.aviso.contains("não prova"), "{}", r.suspeitos.aviso);
        assert!(r.suspeitos.como_provar.contains("uma de cada vez"));
    }

    #[test]
    fn mudanca_anterior_ao_antes_nao_e_suspeita() {
        let mut h = Historico::default();
        h.anotar(10, identidade("GTX 770"), medicao(59.0));
        h.anotar(100, identidade("GTX 770"), medicao(60.0));
        h.anotar(200, identidade("GTX 770"), medicao(48.0));

        let h = h.com_mudancas(&[mudanca("já estava valendo", 50)]);
        let r = h.regressao("fps.average", true).expect("regressão");
        assert!(r.suspeitos.mudancas.is_empty(), "{:?}", r.suspeitos.mudancas);
    }

    #[test]
    fn a_maquina_mudando_entra_na_resposta() {
        let mut h = Historico::default();
        h.anotar(100, identidade("GTX 770"), medicao(60.0));
        h.anotar(200, identidade("GT 710"), medicao(30.0));

        let r = h.regressao("fps.average", true).expect("regressão");
        assert!(
            r.suspeitos.maquina_mudou.iter().any(|d| d.contains("GT 710")),
            "{:?}",
            r.suspeitos.maquina_mudou
        );
    }

    #[test]
    fn diferenca_dentro_do_ruido_nao_acusa_queda() {
        let mut h = Historico::default();
        h.anotar(100, identidade("GTX 770"), medicao(60.0));
        h.anotar(200, identidade("GTX 770"), medicao(59.9));

        let r = h.regressao("fps.average", true).expect("regressão");
        assert!(!r.piorou, "{:?}", r.diferenca);
    }

    #[test]
    fn tempo_de_quadro_subindo_e_que_e_piora() {
        let mut h = Historico::default();
        let subiu = |centro: f64| {
            Evento::Medicao(resumir("frametime.mean", &[centro - 0.2, centro, centro + 0.2]).unwrap())
        };
        h.anotar(100, identidade("GTX 770"), subiu(14.0));
        h.anotar(200, identidade("GTX 770"), subiu(22.0));

        assert!(h.regressao("frametime.mean", false).expect("r").piorou);
        assert!(!h.regressao("frametime.mean", true).expect("r").piorou);
    }

    #[test]
    fn uma_medicao_so_nao_da_regressao() {
        let mut h = Historico::default();
        h.anotar(100, identidade("GTX 770"), medicao(60.0));

        assert!(h.regressao("fps.average", true).is_none());
    }

    #[test]
    fn o_descarte_e_contado_e_nao_silencioso() {
        let mut h = Historico::default();
        for i in 0..(LIMITE_DE_REGISTROS + 7) {
            h.anotar(i as u64, identidade("GTX 770"), medicao(60.0));
        }

        assert_eq!(h.registros.len(), LIMITE_DE_REGISTROS);
        assert_eq!(h.descartados, 7);
        assert_eq!(h.suspeitos_entre(0, u64::MAX).descartados, 7);
    }

    #[test]
    fn grava_e_le_de_volta() {
        let caminho = std::env::temp_dir().join("otimiza-historico-ida-e-volta.json");
        let _ = std::fs::remove_file(&caminho);

        let mut h = Historico::default();
        h.anotar(100, identidade("GTX 770"), medicao(60.0));
        h.anotar(200, identidade("GTX 770"), medicao(48.0));

        guardar_em(&caminho, &h).expect("gravar");

        assert_eq!(ler_de(&caminho).expect("ler"), h);
        let _ = std::fs::remove_file(&caminho);
    }

    #[test]
    fn arquivo_ausente_e_historico_vazio_mas_ilegivel_e_erro() {
        let ausente = std::env::temp_dir().join("otimiza-historico-que-nao-existe.json");
        let _ = std::fs::remove_file(&ausente);
        assert_eq!(ler_de(&ausente).expect("vazio").registros.len(), 0);

        let quebrado = std::env::temp_dir().join("otimiza-historico-quebrado.json");
        std::fs::write(&quebrado, "{ isto nao e json").expect("escrever");

        assert!(ler_de(&quebrado).is_err());
        let _ = std::fs::remove_file(&quebrado);
    }

    /// Arquivo de versão anterior, sem os campos novos, continua legível.
    #[test]
    fn arquivo_antigo_nao_quebra() {
        let caminho = std::env::temp_dir().join("otimiza-historico-antigo.json");
        std::fs::write(&caminho, "{}").expect("escrever");

        let h = ler_de(&caminho).expect("ler");
        assert_eq!(h.schema_version, 0);
        assert_eq!(h.descartados, 0);
        let _ = std::fs::remove_file(&caminho);
    }
}
