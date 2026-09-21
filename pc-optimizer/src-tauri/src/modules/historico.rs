// Histórico de desempenho: o que mudou entre a vez que estava bom e agora
//
// A PERGUNTA QUE NENHUM OTIMIZADOR RESPONDE
//
// "Estava bom semana passada e agora está ruim." É a queixa mais difícil do
// suporte, porque a máquina de hoje não tem nada que conte o que ela era. O
// produto já media — `repeticoes` entrega média e margem, `baseline` guarda um
// retrato, `changelog` guarda o que foi aplicado — mas cada um guardava a SUA
// coisa, e ninguém guardava a LINHA DO TEMPO das duas juntas.
//
// Este módulo é essa linha do tempo: medições e mudanças, na ordem em que
// aconteceram, com a identidade da máquina em cada registro.
//
// COINCIDÊNCIA NO TEMPO NÃO É CAUSA
//
// É a regra inteira deste módulo. Ele consegue dizer "entre a medição boa e a
// ruim, estas três coisas foram aplicadas". Não consegue dizer que alguma delas
// causou a queda: no mesmo intervalo o Windows atualizou, o driver mudou, o
// jogo recebeu patch e a temperatura ambiente subiu dez graus. Por isso o que
// sai chama-se SUSPEITOS, carrega o aviso junto e aponta o caminho de provar —
// que é desfazer uma, medir com repetição, e ver.
//
// POR QUE A COMPARAÇÃO NÃO É CONTRA O MELHOR DE SEMPRE
//
// Seria a escolha óbvia e está errada. "O melhor já medido" é um EXTREMO por
// construção: de vinte medições, a maior é a que mais teve sorte — menos coisa
// aberta, cache quente, nenhuma varredura no meio. Comparar o de agora contra
// ela encontra regressão em máquina nenhuma mudou, sempre, porque a régua foi
// escolhida justamente por ser a mais alta.
//
// A comparação aqui é entre as DUAS ÚLTIMAS medições da mesma métrica. Nenhuma
// das duas foi escolhida por ser extrema, e a diferença entre elas é a que
// `repeticoes` sabe julgar com intervalo de confiança.
//
// O ARQUIVO GUARDA MEDIÇÕES, E NÃO MUDANÇAS
//
// As mudanças já têm dono: `changelog` é quem sabe o que foi aplicado e
// quando, e é ele que o botão de desfazer consulta. Gravá-las também aqui
// criaria duas listas do mesmo fato, que é como um produto passa a mostrar dois
// históricos diferentes para o mesmo cliente. Então elas entram na linha do
// tempo por `com_mudancas`, na hora de responder, vindas da fonte.
//
// O DESCARTE É DECLARADO
//
// O arquivo tem teto. Ao estourar, os registros mais antigos saem — e a
// contagem do que saiu fica gravada. Um histórico que apaga em silêncio produz
// a frase "não há nenhuma mudança entre as duas medições" quando o que houve
// foi o arquivo encher.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::baseline::Identidade;
use super::changelog::AppliedOptimization;
use super::repeticoes::{comparar, Diferenca, Resumo};

pub const VERSAO: u32 = 1;

/// As métricas que entram no histórico.
///
/// UMA LISTA, e não todas. O contrato tem mais de cinquenta métricas, e gravar
/// todas a cada captura encheria o teto em nove capturas — o histórico de um
/// ano viraria o histórico de uma semana. Estas são as que respondem "piorou?".
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

/// Para esta métrica, maior é melhor?
///
/// Tabela e não palpite de quem chama. `autoajuste` já mostrou o estrago de
/// deixar esse sentido por conta de cada chamador: quadros melhoram subindo,
/// tempo de quadro e temperatura melhoram descendo, e errar o sinal faz o
/// produto chamar de regressão exatamente a melhora que ele produziu.
///
/// `None` para métrica que não está na lista — e aí não há regressão a julgar.
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

/// Quantos registros o arquivo guarda.
///
/// Quinhentos. Com uma medição e uma mudança por dia, dá mais de um ano — e o
/// arquivo fica em alguns poucos megabytes, que é o que se pode pedir do disco
/// de alguém sem avisar.
pub const LIMITE_DE_REGISTROS: usize = 500;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Evento {
    /// Uma medição repetida, resumida.
    ///
    /// Guarda o RESUMO e não as amostras: a margem é o que permite julgar a
    /// diferença depois, e as amostras cruas encheriam o arquivo sem
    /// acrescentar resposta nenhuma.
    Medicao(Resumo),
    /// Uma mudança do produto foi aplicada ou desfeita.
    Mudanca {
        nome: String,
        /// Verdadeiro para aplicada, falso para desfeita.
        aplicada: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Registro {
    /// Segundos desde 1970, como o resto do produto.
    pub quando: u64,
    /// A máquina no momento do registro.
    ///
    /// Vai em CADA registro, e não uma vez no arquivo: trocar de placa ou de
    /// plano de energia no meio do histórico é exatamente o tipo de coisa que
    /// explica uma queda, e um cabeçalho único apagaria isso.
    pub identidade: Identidade,
    pub evento: Evento,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Historico {
    #[serde(default = "versao_antiga")]
    pub schema_version: u32,
    #[serde(default)]
    pub registros: Vec<Registro>,
    /// Quantos registros antigos já saíram por causa do teto.
    ///
    /// Gravado junto de propósito. Sem ele, "não há nenhuma mudança entre as
    /// duas medições" seria indistinguível de "o arquivo encheu e as mudanças
    /// que havia foram embora".
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

/// O que estava aplicado entre duas medições, e o aviso que vai junto.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Suspeitos {
    /// As mudanças do produto no intervalo, na ordem.
    pub mudancas: Vec<String>,
    /// O que mudou na própria máquina entre as duas medições.
    ///
    /// Placa trocada, plano de energia mudado por fora, Windows atualizado. Na
    /// prática, quando esta lista não está vazia ela costuma pesar mais que
    /// tudo que o produto fez.
    pub maquina_mudou: Vec<String>,
    /// Quantos registros o teto já engoliu. Ver `Historico::descartados`.
    pub descartados: usize,
    pub aviso: String,
    pub como_provar: String,
}

/// A queda medida entre as duas últimas medições de uma métrica.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Regressao {
    pub id: String,
    pub anterior: Resumo,
    pub atual: Resumo,
    pub diferenca: Diferenca,
    /// Verdadeiro quando a diferença é real E na direção ruim.
    pub piorou: bool,
    pub suspeitos: Suspeitos,
}

const AVISO: &str = "Estas mudanças apenas CAEM NO INTERVALO entre as duas medições. Isso não \
                     prova que alguma delas causou a diferença: no mesmo período o Windows pode \
                     ter atualizado, o driver mudado e o jogo recebido correção.";

const COMO_PROVAR: &str = "Para provar, desfaça uma de cada vez e meça de novo com repetição. É a \
                           única forma de separar a mudança do resto — e é o que o autoajuste faz.";

impl Historico {
    /// Acrescenta um registro, respeitando o teto.
    pub fn anotar(&mut self, quando: u64, identidade: Identidade, evento: Evento) {
        self.schema_version = VERSAO;
        self.registros.push(Registro {
            quando,
            identidade,
            evento,
        });

        // Sai pela frente: o mais antigo é o que menos descreve a máquina de
        // hoje.
        let sobra = self.registros.len().saturating_sub(LIMITE_DE_REGISTROS);
        if sobra > 0 {
            self.registros.drain(0..sobra);
            self.descartados += sobra;
        }
    }

    /// Uma cópia com as mudanças do `changelog` na linha do tempo.
    ///
    /// As mudanças NÃO são gravadas no arquivo: quem sabe o que está aplicado
    /// é o `changelog`, e duas listas do mesmo fato acabam discordando. Elas
    /// entram aqui, na hora de responder, vindas da fonte.
    ///
    /// A identidade de cada mudança é a do registro de medição mais próximo
    /// ANTES dela — é o que se sabia da máquina naquele momento. Sem nenhuma
    /// medição anterior, a mudança fica de fora: ela é anterior a tudo que o
    /// histórico conhece, e não cai em intervalo nenhum.
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

    /// As duas últimas medições de uma métrica, da mais antiga para a mais
    /// recente.
    ///
    /// As DUAS ÚLTIMAS, e não a melhor contra a de agora. Ver o cabeçalho: a
    /// melhor de sempre é um extremo escolhido, e comparar contra ela acha
    /// regressão em toda máquina.
    pub fn duas_ultimas(&self, id: &str) -> Option<(&Resumo, &Resumo)> {
        let mut encontradas = self.registros.iter().rev().filter_map(|r| match &r.evento {
            Evento::Medicao(resumo) if resumo.id == id => Some(resumo),
            _ => None,
        });

        let atual = encontradas.next()?;
        let anterior = encontradas.next()?;

        Some((anterior, atual))
    }

    /// O que aconteceu entre dois instantes.
    ///
    /// O intervalo é aberto nas pontas: a mudança feita no mesmo segundo da
    /// medição anterior aconteceu ANTES dela, e incluí-la culparia algo que já
    /// estava valendo quando o "antes" foi medido.
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

        // A identidade é comparada entre as pontas do intervalo, e não registro
        // a registro: o que interessa é se a máquina de hoje é a mesma de
        // quando o "antes" foi medido.
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

    /// A métrica piorou entre as duas últimas medições?
    ///
    /// `maior_e_melhor` sem valor padrão, pela mesma razão de `autoajuste`:
    /// quadros por segundo melhoram subindo e tempo de quadro melhora
    /// descendo, e um padrão aqui faria o histórico chamar de regressão toda
    /// melhora de latência.
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
            // Sem diferença que as medições distingam, não há regressão a
            // declarar. Não provar uma queda é diferente de provar que não
            // houve — e aqui a resposta honesta é não acusar.
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

/// Lê o histórico. Arquivo ausente é histórico vazio; ilegível é ERRO.
///
/// A distinção é a mesma de `baseline`: tratar arquivo corrompido como vazio
/// faria o produto apagar por cima do histórico do cliente no próximo registro.
pub fn ler_de(caminho: &Path) -> Result<Historico, String> {
    match std::fs::read_to_string(caminho) {
        Ok(bruto) => serde_json::from_str(&bruto)
            .map_err(|e| format!("o histórico existe mas não pôde ser lido: {e}")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Historico::default()),
        Err(e) => Err(format!("não consegui abrir o histórico: {e}")),
    }
}

/// Grava o histórico inteiro, de forma atômica.
///
/// Temporário com `create_new`, `sync_all` antes do rename: o mesmo cuidado de
/// `baseline` e `changelog`, e pela mesma razão — renomear um arquivo cujo
/// conteúdo ainda está no cache troca o histórico do cliente por um vazio.
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

/// Onde o histórico mora: ao lado dos baselines, pela mesma conta de caminho.
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

    /// Duas medições com uma mudança do produto no meio.
    ///
    /// A mudança entra por `com_mudancas`, que é o caminho de verdade: o
    /// arquivo guarda medições, e quem sabe o que foi aplicado é o
    /// `changelog`.
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

    /// A regra inteira do módulo, escrita como teste.
    #[test]
    fn suspeito_nao_e_culpado() {
        let r = com_queda().regressao("fps.average", true).expect("regressão");

        assert!(r.suspeitos.aviso.contains("não prova"), "{}", r.suspeitos.aviso);
        assert!(r.suspeitos.como_provar.contains("uma de cada vez"));
    }

    /// Mudança feita ANTES da medição de referência não entra na lista.
    #[test]
    fn mudanca_anterior_ao_antes_nao_e_suspeita() {
        let mut h = Historico::default();
        // Uma medição bem antiga, só para a mudança ter identidade conhecida.
        h.anotar(10, identidade("GTX 770"), medicao(59.0));
        h.anotar(100, identidade("GTX 770"), medicao(60.0));
        h.anotar(200, identidade("GTX 770"), medicao(48.0));

        // Aplicada ANTES da medição de referência: já estava valendo quando o
        // "antes" foi medido, então não explica a diferença.
        let h = h.com_mudancas(&[mudanca("já estava valendo", 50)]);
        let r = h.regressao("fps.average", true).expect("regressão");
        assert!(r.suspeitos.mudancas.is_empty(), "{:?}", r.suspeitos.mudancas);
    }

    /// Trocar a placa no meio pesa mais que tudo que o produto fez.
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

    /// Diferença que as medições não distinguem NÃO vira regressão.
    #[test]
    fn diferenca_dentro_do_ruido_nao_acusa_queda() {
        let mut h = Historico::default();
        h.anotar(100, identidade("GTX 770"), medicao(60.0));
        h.anotar(200, identidade("GTX 770"), medicao(59.9));

        let r = h.regressao("fps.average", true).expect("regressão");
        assert!(!r.piorou, "{:?}", r.diferenca);
    }

    /// O sentido da métrica decide o que é piora.
    #[test]
    fn tempo_de_quadro_subindo_e_que_e_piora() {
        let mut h = Historico::default();
        let subiu = |centro: f64| {
            Evento::Medicao(resumir("frametime.mean", &[centro - 0.2, centro, centro + 0.2]).unwrap())
        };
        h.anotar(100, identidade("GTX 770"), subiu(14.0));
        h.anotar(200, identidade("GTX 770"), subiu(22.0));

        // Menor é melhor: subir é piorar.
        assert!(h.regressao("frametime.mean", false).expect("r").piorou);
        // Lido ao contrário, o mesmo dado vira melhora.
        assert!(!h.regressao("frametime.mean", true).expect("r").piorou);
    }

    #[test]
    fn uma_medicao_so_nao_da_regressao() {
        let mut h = Historico::default();
        h.anotar(100, identidade("GTX 770"), medicao(60.0));

        assert!(h.regressao("fps.average", true).is_none());
    }

    /// O teto corta os antigos, e a contagem do que saiu fica.
    #[test]
    fn o_descarte_e_contado_e_nao_silencioso() {
        let mut h = Historico::default();
        for i in 0..(LIMITE_DE_REGISTROS + 7) {
            h.anotar(i as u64, identidade("GTX 770"), medicao(60.0));
        }

        assert_eq!(h.registros.len(), LIMITE_DE_REGISTROS);
        assert_eq!(h.descartados, 7);
        // E a contagem acompanha a resposta, para não parecer que não houve
        // mudança nenhuma no intervalo.
        assert_eq!(h.suspeitos_entre(0, u64::MAX).descartados, 7);
    }

    #[test]
    fn grava_e_le_de_volta() {
        let caminho = std::env::temp_dir().join("otimiza-historico-ida-e-volta.json");
        let _ = std::fs::remove_file(&caminho);

        // Só medições: é o que o arquivo guarda. As mudanças entram na hora de
        // responder, vindas do `changelog`.
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

        // Tratar corrompido como vazio faria o próximo registro apagar por cima
        // do histórico do cliente.
        assert!(ler_de(&quebrado).is_err());
        let _ = std::fs::remove_file(&quebrado);
    }

    /// Arquivo de uma versão anterior, sem os campos novos, continua legível.
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
