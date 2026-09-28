// Receita (MASTER-PLAN, item 3): liga cada causa do diagnóstico (`doctor`) ao ajuste do Otimiza que a ataca, diz se
// ele já está aplicado NESTE jogo, como será provado, e LEMBRA o que já piorou este jogo nesta máquina.
//
// A memória não é arquivo novo: sai das provas que o Otimiza já guarda — o portão (perfil gráfico e modo jogo,
// partida a partida) e a prova alternada no jogo (plano de energia). O contexto de cada lembrança (driver e Windows)
// vem da última medição antes da decisão. Estratégia que piorou volta a ser oferecida só quando o driver ou o build
// do Windows mudou (a atualização mensal, o número depois do ponto, não conta), e só onde o Otimiza de fato prova de
// novo: o modo jogo reprovado num jogo fica parado nele, então não se promete nova prova.
//
// Os jogos se comparam pela CHAVE (`fivem`, `gta5`...): o FiveM troca o nome do executável a cada build.
//
// Só lê. Aplicar continua sendo o botão de cada ajuste, que grava histórico e é provado pelo caminho de sempre.

use serde::Serialize;

use crate::modules::deriva::Ambiente;
use crate::modules::doctor::{Ajuste, Causa, Confianca, Diagnostico};
use crate::modules::medicoes::MedicaoAutomatica;
use crate::modules::portao::{self, Veredito};
use crate::modules::provaalternada::{Desfecho as DesfechoDaProva, Resultado as ProvaAlternada};

/// Os perfis gráficos do FiveM/GTA V (`config_jogo_<perfil>`), do mais leve ao que só tira o limite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PerfilDoJogo {
    /// Só tira o limite de FPS: não deixa o gráfico mais leve.
    SemTeto,
    Equilibrado,
    /// Também reduz distância de visão e população (LOD), que é o que alivia o processador.
    Competitivo,
}

impl PerfilDoJogo {
    pub fn do_id(id: &str) -> Option<PerfilDoJogo> {
        match id.strip_prefix("config_jogo_")? {
            "sem_teto" => Some(PerfilDoJogo::SemTeto),
            "equilibrado" => Some(PerfilDoJogo::Equilibrado),
            "competitivo" => Some(PerfilDoJogo::Competitivo),
            _ => None,
        }
    }

    /// Este perfil, aplicado, já faz o que `pedido` faz?
    fn cobre(self, pedido: PerfilDoJogo) -> bool {
        match pedido {
            PerfilDoJogo::SemTeto => self == PerfilDoJogo::SemTeto,
            PerfilDoJogo::Equilibrado => matches!(self, PerfilDoJogo::Equilibrado | PerfilDoJogo::Competitivo),
            PerfilDoJogo::Competitivo => self == PerfilDoJogo::Competitivo,
        }
    }
}

/// O que o Otimiza sabe aplicar e provar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "tipo", content = "perfil")]
pub enum Estrategia {
    PlanoOtimiza,
    PerfilGrafico(PerfilDoJogo),
    /// Modo jogo (o governador acalma programas em segundo plano).
    ModoJogo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Resultado {
    Ganhou,
    Piorou,
    SemDiferenca,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Lembranca {
    pub estrategia: Estrategia,
    pub jogo: String,
    pub resultado: Resultado,
    pub quando: u64,
    /// Driver e Windows da última medição antes da decisão. `None`: não havia medição com ambiente.
    pub ambiente: Option<Ambiente>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Contexto {
    /// Driver e Windows lidos e iguais aos de agora.
    Igual,
    /// Um dos lados não foi lido (medição antiga, leitura falha): não se sabe se mudou.
    NaoLido,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "estado")]
pub enum Oferta {
    Oferecer,
    /// Já piorou este jogo nesta máquina.
    NaoOferecer { quando: u64, contexto: Contexto },
    /// Piorou antes, mas o driver ou o Windows mudou desde então, e o Otimiza prova de novo.
    ProvarDeNovo { quando: u64 },
    /// Já provou ganho ou ficou sem diferença: a tela mostra, e o cliente decide.
    JaProvado { resultado: Resultado, quando: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ComoProvar {
    /// Rodadas alternadas no jogo (área Otimizar).
    ProvaNoJogo,
    /// As próximas partidas contra as de antes (o portão).
    ProximasPartidas,
    /// O Otimiza não mexe nisso: é o cliente quem faz, e a medição mostra depois.
    PeloCliente,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Passo {
    pub ajuste: Ajuste,
    /// `None`: ajuste que o Otimiza não aplica (limite de FPS, refrigeração, SSD, resolução).
    pub estrategia: Option<Estrategia>,
    pub aplicado: bool,
    pub oferta: Oferta,
    pub como_provar: ComoProvar,
}

/// O que está aplicado agora, lido do histórico e das preferências.
#[derive(Debug, Clone, Default)]
pub struct Aplicados {
    pub plano: bool,
    pub modo_jogo: bool,
    /// `(perfil, chave do jogo)`.
    pub perfis: Vec<(PerfilDoJogo, String)>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Receita {
    /// O diagnóstico inteiro, com as hipóteses, para a tela não pedir duas vezes.
    pub diagnostico: Diagnostico,
    /// Um bloco por causa do diagnóstico, na mesma ordem; hipótese não entra.
    pub por_causa: Vec<(Causa, Vec<Passo>)>,
    pub lembrancas: Vec<Lembranca>,
    /// O registro das provas não pôde ser lido: sem ele não se sabe o que já piorou.
    pub memoria_ilegivel: Option<String>,
}

/// **Pura.** O mesmo jogo, de build em build: `FiveM_b3258_GTAProcess.exe` e `FiveM_b2944_...` são `fivem`.
pub fn chave_do_jogo(nome: &str) -> String {
    let n: String = nome.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect();
    if n.contains("fivem") {
        "fivem".into()
    } else if n.contains("gta5") || n.contains("gtav") {
        "gta5".into()
    } else {
        n.trim_end_matches("exe").to_string()
    }
}

/// **Pura.** O ajuste do diagnóstico que o Otimiza sabe fazer, para este jogo.
pub fn estrategia_de(ajuste: Ajuste, jogo: &str) -> Option<Estrategia> {
    let perfil_existe = matches!(chave_do_jogo(jogo).as_str(), "fivem" | "gta5");
    match ajuste {
        Ajuste::PlanoDeEnergiaOtimiza => Some(Estrategia::PlanoOtimiza),
        Ajuste::PerfilGraficoMaisLeve if perfil_existe => Some(Estrategia::PerfilGrafico(PerfilDoJogo::Equilibrado)),
        Ajuste::MenosCargaNoProcessador if perfil_existe => Some(Estrategia::PerfilGrafico(PerfilDoJogo::Competitivo)),
        Ajuste::FecharProgramasPesados => Some(Estrategia::ModoJogo),
        _ => None,
    }
}

/// A última medição do jogo antes da decisão (senão, a primeira depois): o ambiente em que a prova foi feita.
fn ambiente_da_decisao(quando: u64, chave: &str, medicoes: &[MedicaoAutomatica]) -> Option<Ambiente> {
    let do_jogo = || medicoes.iter().filter(|m| chave_do_jogo(&m.jogo) == chave && m.ambiente.is_some());
    do_jogo()
        .filter(|m| m.quando <= quando)
        .max_by_key(|m| m.quando)
        .or_else(|| do_jogo().filter(|m| m.quando > quando).min_by_key(|m| m.quando))
        .and_then(|m| m.ambiente.clone())
}

/// **Pura.** As lembranças que as provas guardadas dão.
pub fn lembrancas(estado: &portao::Estado, prova: Option<&ProvaAlternada>, medicoes: &[MedicaoAutomatica]) -> Vec<Lembranca> {
    let mut saida = Vec::new();
    for d in &estado.decididos {
        let estrategia = if let Some(p) = PerfilDoJogo::do_id(&d.vigiado.id) {
            Estrategia::PerfilGrafico(p)
        } else if d.vigiado.id.starts_with("governador:") {
            Estrategia::ModoJogo
        } else {
            continue;
        };
        let resultado = match d.veredito {
            Veredito::Desfazer => Resultado::Piorou,
            Veredito::Melhorou => Resultado::Ganhou,
            Veredito::SemMudanca => Resultado::SemDiferenca,
            Veredito::Aguardando { .. } => continue,
        };
        let jogo = chave_do_jogo(&d.vigiado.processo);
        saida.push(Lembranca { estrategia, ambiente: ambiente_da_decisao(d.quando, &jogo, medicoes), jogo, resultado, quando: d.quando });
    }
    if let Some(p) = prova {
        let resultado = match p.desfecho {
            DesfechoDaProva::Ganhou => Some(Resultado::Ganhou),
            DesfechoDaProva::Piorou => Some(Resultado::Piorou),
            DesfechoDaProva::Indistinguivel => Some(Resultado::SemDiferenca),
            DesfechoDaProva::SemComparacaoJusta => None,
        };
        if let Some(resultado) = resultado {
            let jogo = chave_do_jogo(&p.jogo);
            saida.push(Lembranca {
                estrategia: Estrategia::PlanoOtimiza,
                ambiente: ambiente_da_decisao(p.quando, &jogo, medicoes),
                jogo,
                resultado,
                quando: p.quando,
            });
        }
    }
    saida.sort_by_key(|l| std::cmp::Reverse(l.quando));
    saida
}

/// O build do Windows sem a atualização mensal: "19045.5011" → "19045".
fn build(windows: &Option<String>) -> Option<&str> {
    windows.as_deref().map(|w| w.split('.').next().unwrap_or(w))
}

/// `Some(true)`: driver ou build mudou. `Some(false)`: os dois lidos e iguais. `None`: algum lado sem leitura.
fn contexto_mudou(antes: &Option<Ambiente>, agora: &Option<Ambiente>) -> Option<bool> {
    let (a, b) = (antes.as_ref()?, agora.as_ref()?);
    let driver = match (&a.driver, &b.driver) {
        (Some(x), Some(y)) => Some(x != y),
        _ => None,
    };
    let windows = match (build(&a.windows), build(&b.windows)) {
        (Some(x), Some(y)) => Some(x != y),
        _ => None,
    };
    match (driver, windows) {
        (Some(true), _) | (_, Some(true)) => Some(true),
        (Some(false), Some(false)) => Some(false),
        _ => None,
    }
}

/// **Pura.** A lembrança mais recente desta estratégia neste jogo decide.
pub fn oferta(estrategia: Estrategia, jogo: &str, agora: &Option<Ambiente>, memoria: &[Lembranca]) -> Oferta {
    let chave = chave_do_jogo(jogo);
    let ultima = memoria.iter().filter(|l| l.estrategia == estrategia && l.jogo == chave).max_by_key(|l| l.quando);
    match ultima {
        None => Oferta::Oferecer,
        Some(l) if l.resultado == Resultado::Piorou => match contexto_mudou(&l.ambiente, agora) {
            // O governador reprovado fica parado naquele jogo: não há nova prova a prometer.
            Some(true) if estrategia != Estrategia::ModoJogo => Oferta::ProvarDeNovo { quando: l.quando },
            Some(false) => Oferta::NaoOferecer { quando: l.quando, contexto: Contexto::Igual },
            _ => Oferta::NaoOferecer { quando: l.quando, contexto: Contexto::NaoLido },
        },
        Some(l) => Oferta::JaProvado { resultado: l.resultado, quando: l.quando },
    }
}

fn aplicado(e: Estrategia, chave: &str, a: &Aplicados) -> bool {
    match e {
        Estrategia::PlanoOtimiza => a.plano,
        Estrategia::ModoJogo => a.modo_jogo,
        Estrategia::PerfilGrafico(pedido) => a.perfis.iter().any(|(p, j)| j == chave && p.cobre(pedido)),
    }
}

/// **Pura.**
pub fn montar(
    diagnostico: Diagnostico,
    memoria: Vec<Lembranca>,
    memoria_ilegivel: Option<String>,
    aplicados: &Aplicados,
    agora: &Option<Ambiente>,
) -> Receita {
    let jogo = diagnostico.jogo.clone().unwrap_or_default();
    let chave = chave_do_jogo(&jogo);
    let por_causa = diagnostico
        .causas
        .iter()
        .filter(|c| c.confianca != Confianca::Hipotese)
        .map(|c| {
            let passos = c
                .ajuda
                .iter()
                .map(|&ajuste| {
                    let estrategia = estrategia_de(ajuste, &jogo);
                    Passo {
                        ajuste,
                        estrategia,
                        aplicado: estrategia.is_some_and(|e| aplicado(e, &chave, aplicados)),
                        // Sem memória lida não se afirma nada sobre o passado.
                        oferta: match (estrategia, &memoria_ilegivel) {
                            (Some(e), None) => oferta(e, &jogo, agora, &memoria),
                            _ => Oferta::Oferecer,
                        },
                        como_provar: match estrategia {
                            Some(Estrategia::PlanoOtimiza) => ComoProvar::ProvaNoJogo,
                            Some(_) => ComoProvar::ProximasPartidas,
                            None => ComoProvar::PeloCliente,
                        },
                    }
                })
                .collect();
            (c.clone(), passos)
        })
        .collect();
    Receita { diagnostico, por_causa, lembrancas: memoria, memoria_ilegivel }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::doctor::TipoDeCausa;
    use crate::modules::portao::{Decidido, Vigiado};

    fn amb(driver: &str, windows: &str) -> Option<Ambiente> {
        Some(Ambiente { driver: Some(driver.into()), windows: Some(windows.into()), ..Default::default() })
    }

    fn lembranca(estrategia: Estrategia, resultado: Resultado, quando: u64, driver: &str) -> Lembranca {
        Lembranca { estrategia, jogo: "fivem".into(), resultado, quando, ambiente: amb(driver, "19045.1") }
    }

    const FIVEM: &str = "FiveM_b3258_GTAProcess.exe";
    const PERFIL: Estrategia = Estrategia::PerfilGrafico(PerfilDoJogo::Competitivo);

    #[test]
    fn o_que_piorou_neste_contexto_nao_e_oferecido_de_novo() {
        let m = [lembranca(PERFIL, Resultado::Piorou, 10, "a")];
        assert_eq!(oferta(PERFIL, FIVEM, &amb("a", "19045.1"), &m), Oferta::NaoOferecer { quando: 10, contexto: Contexto::Igual });
    }

    #[test]
    fn driver_novo_devolve_a_chance_de_provar_mas_a_atualizacao_mensal_nao() {
        let m = [lembranca(PERFIL, Resultado::Piorou, 10, "a")];
        assert_eq!(oferta(PERFIL, FIVEM, &amb("b", "19045.1"), &m), Oferta::ProvarDeNovo { quando: 10 });
        assert!(matches!(oferta(PERFIL, FIVEM, &amb("a", "19045.9"), &m), Oferta::NaoOferecer { contexto: Contexto::Igual, .. }));
    }

    #[test]
    fn sem_leitura_nao_se_diz_que_e_o_mesmo_contexto() {
        let m = [lembranca(PERFIL, Resultado::Piorou, 10, "a")];
        assert_eq!(oferta(PERFIL, FIVEM, &None, &m), Oferta::NaoOferecer { quando: 10, contexto: Contexto::NaoLido });
    }

    #[test]
    fn modo_jogo_reprovado_nao_promete_nova_prova() {
        let m = [lembranca(Estrategia::ModoJogo, Resultado::Piorou, 10, "a")];
        assert!(matches!(oferta(Estrategia::ModoJogo, FIVEM, &amb("b", "19045.1"), &m), Oferta::NaoOferecer { .. }));
    }

    #[test]
    fn build_novo_do_fivem_e_o_mesmo_jogo_e_cada_perfil_tem_a_sua_memoria() {
        let m = [lembranca(PERFIL, Resultado::Piorou, 10, "a")];
        assert!(matches!(oferta(PERFIL, "FiveM_b2944_GTAProcess.exe", &amb("a", "19045.1"), &m), Oferta::NaoOferecer { .. }));
        let leve = Estrategia::PerfilGrafico(PerfilDoJogo::Equilibrado);
        assert_eq!(oferta(leve, FIVEM, &amb("a", "19045.1"), &m), Oferta::Oferecer, "o competitivo que piorou não risca o equilibrado");
    }

    #[test]
    fn sem_teto_aplicado_nao_conta_como_grafico_mais_leve_nem_em_outro_jogo() {
        let a = Aplicados { perfis: vec![(PerfilDoJogo::SemTeto, "fivem".into()), (PerfilDoJogo::Competitivo, "gta5".into())], ..Default::default() };
        assert!(!aplicado(Estrategia::PerfilGrafico(PerfilDoJogo::Equilibrado), "fivem", &a));
        assert!(aplicado(Estrategia::PerfilGrafico(PerfilDoJogo::Equilibrado), "gta5", &a), "o competitivo cobre o mais leve");
    }

    #[test]
    fn as_provas_guardadas_viram_lembrancas_com_o_ambiente_de_antes_da_decisao() {
        let v = Vigiado { id: "config_jogo_competitivo".into(), nome: "x".into(), processo: "fivem_".into(), aplicado_em: 5 };
        let d = |veredito| Decidido { vigiado: v.clone(), veredito, quando: 50, fps_antes: None, fps_depois: None, low_antes: None, low_depois: None, erro: None };
        let estado = portao::Estado { decididos: vec![d(Veredito::Desfazer), d(Veredito::Aguardando { antes: 1, depois: 0 })], ..Default::default() };
        let medicao = |quando, driver: &str| MedicaoAutomatica {
            jogo: FIVEM.into(),
            quando,
            fps: 90.0,
            low_1pct: 45.0,
            engasgos_por_minuto: 1.0,
            segundos: 20.0,
            confiavel: true,
            mudancas_aplicadas: 0,
            ambiente: amb(driver, "19045.1"),
            frametime_medio_ms: None,
            frametime_p95_ms: None,
            frametime_p99_ms: None,
            placa: None,
            cpu_uso_pct: None,
            gpu_uso_pct: None,
            trancos_com_disco_pct: None,
            trancos_medidos: None,
            governador: None,
            geracao: None,
            presentmon: None,
            quadros: None,
            configuracao_do_jogo: None,
            versao_do_formato: 1,
            eventos: None,
        };
        let l = lembrancas(&estado, None, &[medicao(40, "antigo"), medicao(55, "novo")]);
        assert_eq!(l.len(), 1, "aguardando ainda não é lembrança");
        assert_eq!((l[0].estrategia, l[0].resultado, l[0].jogo.as_str()), (PERFIL, Resultado::Piorou, "fivem"));
        assert_eq!(l[0].ambiente.as_ref().and_then(|a| a.driver.as_deref()), Some("antigo"), "o ambiente é o de quando provou");
    }

    #[test]
    fn hipotese_nao_entra_e_memoria_ilegivel_nao_afirma_passado() {
        let causa = |confianca, ajuda: Vec<Ajuste>| Causa { tipo: TipoDeCausa::Processador, confianca, evidencia: String::new(), ajuda, nao_ajuda: vec![], janelas: 2 };
        let d = Diagnostico {
            jogo: Some(FIVEM.into()),
            janelas_lidas: 2,
            causas: vec![
                causa(Confianca::Alta, vec![Ajuste::PlanoDeEnergiaOtimiza, Ajuste::MenosCargaNoProcessador, Ajuste::JogoNoSsd]),
                causa(Confianca::Hipotese, vec![Ajuste::TirarLimiteDeFps]),
            ],
            lacunas: vec![],
        };
        let memoria = vec![lembranca(PERFIL, Resultado::Piorou, 10, "a")];
        let a = Aplicados { plano: true, ..Default::default() };
        let r = montar(d.clone(), memoria.clone(), None, &a, &amb("a", "19045.1"));
        assert_eq!(r.por_causa.len(), 1);
        let passos = &r.por_causa[0].1;
        assert!(passos[0].aplicado && passos[0].como_provar == ComoProvar::ProvaNoJogo);
        assert!(matches!(passos[1].oferta, Oferta::NaoOferecer { .. }));
        assert_eq!((passos[2].estrategia, passos[2].como_provar), (None, ComoProvar::PeloCliente));
        let sem_memoria = montar(d, memoria, Some("portao.json ilegível".into()), &a, &amb("a", "19045.1"));
        assert_eq!(sem_memoria.por_causa[0].1[1].oferta, Oferta::Oferecer);
    }
}
