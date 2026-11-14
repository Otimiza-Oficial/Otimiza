// HISTÓRICO DA ROTA (Plano 1.1, Onda 3b). Cada rota medida vira uma linha curta (hora, ping e perda no fim da rota,
// veredito e trecho), guardada em `%APPDATA%\pc-optimizer\rota-historico.json`. Com medidas em horários diferentes, o
// resumo diz QUANDO a conexão piora ("à noite o ping sobe de 31 para 62 ms") e o relatório junta, em texto pronto para
// colar no chamado, as vezes em que o problema estava na operadora — com data, hora, salto e número, que é o que o
// atendimento pede.
//
// Só número e endereço de servidor: nada de nome de usuário, rede Wi-Fi ou caminho de disco.

use super::rota::{Rota, Trecho, Veredito};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Quantas medidas ficam guardadas (as mais antigas saem).
pub const GUARDADOS: usize = 400;

/// Medidas que um período precisa para entrar na comparação.
const MINIMO_POR_PERIODO: usize = 2;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct RegistroDaRota {
    /// Segundos desde 1970 (UTC).
    pub quando: i64,
    /// Hora local (0–23) e dia da semana (0 = domingo) da medida: o "à noite" é o da pessoa.
    pub hora: u8,
    pub dia_da_semana: u8,
    pub destino: String,
    pub do_jogo: bool,
    pub chegou: bool,
    /// Ping e perda no último salto que respondeu.
    pub mediana_ms: Option<u32>,
    pub jitter_ms: Option<f64>,
    pub perda_pct: f64,
    /// `limpa`, `perda`, `variacao`, `servidor_nao_responde` ou `nao_mediu`.
    pub veredito: String,
    pub trecho: Option<Trecho>,
    pub salto: Option<u8>,
    pub operadora: Option<String>,
}

/// **Pura.** A linha guardada de uma rota.
pub fn registro_de(rota: &Rota, quando: i64, hora: u8, dia_da_semana: u8, do_jogo: bool) -> RegistroDaRota {
    let ultimo = rota.saltos.iter().rev().find(|s| s.respondidos > 0);
    let (veredito, trecho, salto, operadora) = match &rota.veredito {
        Veredito::Limpa => ("limpa", None, None, None),
        Veredito::Problema { problema, trecho, salto, operadora, .. } => (
            match problema {
                super::rota::Problema::Perda => "perda",
                super::rota::Problema::Variacao => "variacao",
            },
            Some(*trecho),
            Some(*salto),
            operadora.clone(),
        ),
        Veredito::ServidorNaoResponde { .. } => ("servidor_nao_responde", None, None, None),
        Veredito::NaoMediu => ("nao_mediu", None, None, None),
    };
    // Sem operadora no veredito, a do primeiro salto que tem dono conhecido (é a da pessoa na maioria das rotas).
    let operadora = operadora.or_else(|| {
        rota.saltos
            .iter()
            .find(|s| s.trecho == Trecho::Operadora)
            .and_then(|s| s.operadora.clone())
    });
    RegistroDaRota {
        quando,
        hora: hora.min(23),
        dia_da_semana: dia_da_semana.min(6),
        destino: rota.destino.to_string(),
        do_jogo,
        chegou: rota.chegou,
        mediana_ms: ultimo.and_then(|s| s.mediana_ms),
        jitter_ms: ultimo.and_then(|s| s.jitter_ms),
        perda_pct: ultimo.map(|s| s.perda_pct).unwrap_or(100.0),
        veredito: veredito.to_string(),
        trecho,
        salto,
        operadora,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum Periodo {
    Madrugada,
    Manha,
    Tarde,
    Noite,
}

impl Periodo {
    pub fn da_hora(hora: u8) -> Self {
        match hora {
            0..=5 => Periodo::Madrugada,
            6..=11 => Periodo::Manha,
            12..=17 => Periodo::Tarde,
            _ => Periodo::Noite,
        }
    }

    fn nome(self) -> &'static str {
        match self {
            Periodo::Madrugada => "de madrugada",
            Periodo::Manha => "de manhã",
            Periodo::Tarde => "à tarde",
            Periodo::Noite => "à noite",
        }
    }

    const TODOS: [Periodo; 4] = [Periodo::Madrugada, Periodo::Manha, Periodo::Tarde, Periodo::Noite];
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct NumerosDoPeriodo {
    pub periodo: Periodo,
    pub medidas: u32,
    /// Mediana dos pings (ms) das medidas que chegaram a ter número.
    pub ping_ms: Option<u32>,
    /// Média da perda no fim da rota.
    pub perda_pct: f64,
    /// Medidas em que o veredito apontou perda ou variação.
    pub com_problema: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct ResumoDaRede {
    pub medidas: u32,
    pub periodos: Vec<NumerosDoPeriodo>,
    /// "À noite o ping sobe…": só quando há medidas bastantes em dois períodos e a diferença é de sentir.
    pub frase: Option<String>,
    /// Medidas com problema na operadora: o relatório para o chamado sai delas.
    pub na_operadora: u32,
}

fn mediana(mut v: Vec<u32>) -> Option<u32> {
    if v.is_empty() {
        return None;
    }
    v.sort_unstable();
    Some(v[v.len() / 2])
}

fn tem_problema(r: &RegistroDaRota) -> bool {
    r.veredito == "perda" || r.veredito == "variacao"
}

/// **Pura.** Os números por período do dia e a frase de quando piora.
pub fn resumir(registros: &[RegistroDaRota]) -> ResumoDaRede {
    let periodos: Vec<NumerosDoPeriodo> = Periodo::TODOS
        .iter()
        .filter_map(|&p| {
            let deste: Vec<&RegistroDaRota> = registros
                .iter()
                .filter(|r| r.veredito != "nao_mediu" && Periodo::da_hora(r.hora) == p)
                .collect();
            if deste.is_empty() {
                return None;
            }
            Some(NumerosDoPeriodo {
                periodo: p,
                medidas: deste.len() as u32,
                ping_ms: mediana(deste.iter().filter_map(|r| r.mediana_ms).collect()),
                perda_pct: deste.iter().map(|r| r.perda_pct).sum::<f64>() / deste.len() as f64,
                com_problema: deste.iter().filter(|r| tem_problema(r)).count() as u32,
            })
        })
        .collect();

    ResumoDaRede {
        medidas: registros.len() as u32,
        frase: frase(&periodos),
        na_operadora: registros
            .iter()
            .filter(|r| tem_problema(r) && r.trecho == Some(Trecho::Operadora))
            .count() as u32,
        periodos,
    }
}

/// **Pura.** Compara o pior período com o melhor. Diferença pequena (menos de 30% e 10 ms no ping, menos de 5 pontos na
/// perda) não vira frase: seria barulho de uma medida só.
fn frase(periodos: &[NumerosDoPeriodo]) -> Option<String> {
    let validos: Vec<&NumerosDoPeriodo> = periodos
        .iter()
        .filter(|p| p.medidas as usize >= MINIMO_POR_PERIODO)
        .collect();
    if validos.len() < 2 {
        return None;
    }

    // Perda pesa mais que ping: um pacote perdido é tranco na hora.
    let pior_perda = validos.iter().max_by(|a, b| a.perda_pct.total_cmp(&b.perda_pct))?;
    let melhor_perda = validos.iter().min_by(|a, b| a.perda_pct.total_cmp(&b.perda_pct))?;
    if pior_perda.perda_pct - melhor_perda.perda_pct >= 5.0 {
        let mut f = format!(
            "{} a conexão perde mais pacotes: {:.0}% em média, contra {:.0}% {}.",
            capitalizar(pior_perda.periodo.nome()),
            pior_perda.perda_pct,
            melhor_perda.perda_pct,
            melhor_perda.periodo.nome()
        );
        if pior_perda.com_problema > 0 {
            f.push_str(&format!(
                " {} de {} medidas desse horário tiveram problema.",
                pior_perda.com_problema, pior_perda.medidas
            ));
        }
        return Some(f);
    }

    let com_ping: Vec<(&NumerosDoPeriodo, u32)> =
        validos.iter().filter_map(|p| p.ping_ms.map(|ms| (*p, ms))).collect();
    let (pior, pior_ms) = *com_ping.iter().max_by_key(|(_, ms)| *ms)?;
    let (melhor, melhor_ms) = *com_ping.iter().min_by_key(|(_, ms)| *ms)?;
    if pior_ms >= melhor_ms + 10 && pior_ms as f64 >= melhor_ms as f64 * 1.3 {
        return Some(format!(
            "{} o ping sobe: {} ms, contra {} ms {}.",
            capitalizar(pior.periodo.nome()),
            pior_ms,
            melhor_ms,
            melhor.periodo.nome()
        ));
    }
    Some("O ping fica parecido em todos os horários que você mediu.".to_string())
}

fn capitalizar(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|p| p.to_uppercase().collect::<String>() + c.as_str())
        .unwrap_or_default()
}

/// **Pura.** Texto para colar no chamado da operadora: as medidas em que o problema começou dentro da rede dela, com data
/// e hora locais (já formatadas por quem chama), salto e números. `None` sem nenhuma medida assim.
pub fn relatorio(registros: &[RegistroDaRota], data_local: impl Fn(i64) -> String) -> Option<String> {
    let casos: Vec<&RegistroDaRota> = registros
        .iter()
        .filter(|r| tem_problema(r) && r.trecho == Some(Trecho::Operadora))
        .collect();
    if casos.is_empty() {
        return None;
    }
    let operadora = casos
        .iter()
        .rev()
        .find_map(|r| r.operadora.clone())
        .unwrap_or_else(|| "a operadora".to_string());

    let mut t = String::new();
    t.push_str(&format!(
        "Relatório de rota (medido pelo Otimiza, no estilo do MTR: 10 pacotes por salto)\n\
         Operadora: {}\n\
         Medidas com problema dentro da rede da operadora: {} de {}\n\n",
        operadora,
        casos.len(),
        registros.iter().filter(|r| r.veredito != "nao_mediu").count()
    ));
    // As mais recentes primeiro; vinte linhas bastam para o atendimento.
    for r in casos.iter().rev().take(20) {
        let problema = if r.veredito == "perda" {
            format!("perda de {:.0}%", r.perda_pct)
        } else {
            format!("variação de {:.0} ms", r.jitter_ms.unwrap_or(0.0))
        };
        t.push_str(&format!(
            "- {} · destino {} · começa no salto {} · {} · ping {}\n",
            data_local(r.quando),
            r.destino,
            r.salto.map(|s| s.to_string()).unwrap_or_else(|| "?".into()),
            problema,
            r.mediana_ms.map(|m| format!("{} ms", m)).unwrap_or_else(|| "sem resposta".into())
        ));
    }
    t.push_str(
        "\nO problema persiste do salto indicado até o fim da rota (regra do MTR: não é roteador poupando resposta). \
         Peço a verificação do trecho e da rota até o destino.\n",
    );
    Some(t)
}

fn caminho() -> PathBuf {
    let base = std::env::var("APPDATA")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    base.join("pc-optimizer").join("rota-historico.json")
}

/// Arquivo inexistente é vazio; existente e ilegível é `Err` (e não é sobrescrito).
pub fn ler_de(caminho: &Path) -> Result<Vec<RegistroDaRota>, String> {
    match fs::read_to_string(caminho) {
        Ok(bruto) => serde_json::from_str(&bruto)
            .map_err(|e| format!("o histórico da rede está ilegível: {}", e)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(format!("não consegui ler o histórico da rede: {}", e)),
    }
}

pub fn ler() -> Result<Vec<RegistroDaRota>, String> {
    ler_de(&caminho())
}

pub fn registrar_em(caminho: &Path, registro: RegistroDaRota) -> Result<(), String> {
    let mut todos = ler_de(caminho)?;
    todos.push(registro);
    let excesso = todos.len().saturating_sub(GUARDADOS);
    todos.drain(..excesso);
    if let Some(pasta) = caminho.parent() {
        fs::create_dir_all(pasta).map_err(|e| format!("não consegui criar a pasta de dados: {}", e))?;
    }
    let bruto = serde_json::to_string(&todos)
        .map_err(|e| format!("não consegui preparar o histórico da rede: {}", e))?;
    fs::write(caminho, bruto).map_err(|e| format!("não consegui gravar o histórico da rede: {}", e))
}

/// Guarda a rota com a hora local de agora. Falha ao gravar não estraga a medida: só não entra no histórico.
pub fn guardar(rota: &Rota, do_jogo: bool) {
    use chrono::{Datelike, Local, Timelike};
    let agora = Local::now();
    let r = registro_de(
        rota,
        agora.timestamp(),
        agora.hour() as u8,
        agora.weekday().num_days_from_sunday() as u8,
        do_jogo,
    );
    let _ = registrar_em(&caminho(), r);
}

/// Data e hora locais no formato do relatório.
/// **Pura.** As janelas de ping das partidas (`rede_partida`, gravadas sozinhas durante o jogo) viram medidas do
/// resumo por horário: o "à noite piora" aparece sem a pessoa medir a rota na mão. Não entram nos recentes nem no
/// relatório da operadora (não dizem em que trecho da rota foi). `hora_e_dia` dá a hora e o dia locais de cada momento.
pub fn das_partidas(janelas: &[super::rede_partida::JanelaDeRede], hora_e_dia: impl Fn(i64) -> (u8, u8)) -> Vec<RegistroDaRota> {
    use super::rede_partida::{PERDA_QUE_SE_SENTE_PCT, VARIACAO_QUE_SE_SENTE_MS};
    janelas
        .iter()
        .map(|j| {
            let quando = j.quando as i64;
            let (hora, dia) = hora_e_dia(quando);
            let veredito = if j.perda_pct >= PERDA_QUE_SE_SENTE_PCT {
                "perda"
            } else if j.variacao_ms >= VARIACAO_QUE_SE_SENTE_MS {
                "variacao"
            } else {
                "limpa"
            };
            RegistroDaRota {
                quando,
                hora: hora.min(23),
                dia_da_semana: dia.min(6),
                destino: "1.1.1.1".into(),
                do_jogo: true,
                chegou: true,
                mediana_ms: Some(j.ping_ms.round().max(0.0) as u32),
                jitter_ms: Some(j.variacao_ms),
                perda_pct: j.perda_pct,
                veredito: veredito.into(),
                trecho: None,
                salto: None,
                operadora: None,
            }
        })
        .collect()
}

/// Hora (0–23) e dia da semana (0 = domingo) locais de um momento.
pub fn hora_e_dia_local(quando: i64) -> (u8, u8) {
    use chrono::{Datelike, Local, TimeZone, Timelike};
    Local
        .timestamp_opt(quando, 0)
        .single()
        .map(|d| (d.hour() as u8, d.weekday().num_days_from_sunday() as u8))
        .unwrap_or((12, 0))
}

pub fn data_local(quando: i64) -> String {
    use chrono::{Local, TimeZone};
    Local
        .timestamp_opt(quando, 0)
        .single()
        .map(|d| d.format("%d/%m/%Y %H:%M").to_string())
        .unwrap_or_else(|| quando.to_string())
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn janelas_das_partidas_viram_medidas_por_horario() {
        use crate::plataforma::rede_partida::JanelaDeRede;
        let j = |quando: u64, ping: f64, var: f64, perda: f64| JanelaDeRede {
            jogo: "FiveM".into(),
            quando,
            ping_ms: ping,
            variacao_ms: var,
            perda_pct: perda,
        };
        // A hora vem da função: 21h para as duas primeiras, 15h para as outras.
        let regs = das_partidas(
            &[j(1, 40.0, 30.0, 0.0), j(2, 38.0, 25.0, 3.0), j(3, 20.0, 2.0, 0.0), j(4, 21.0, 3.0, 0.0)],
            |q| (if q <= 2 { 21 } else { 15 }, 3),
        );
        assert_eq!(regs.iter().map(|r| r.veredito.as_str()).collect::<Vec<_>>(), ["variacao", "perda", "limpa", "limpa"]);
        assert!(regs.iter().all(|r| r.trecho.is_none() && r.do_jogo));
        let resumo = resumir(&regs);
        let noite = resumo.periodos.iter().find(|p| p.periodo == Periodo::Noite).unwrap();
        assert_eq!(noite.medidas, 2);
        assert_eq!(noite.com_problema, 2);
        // Partida nunca conta como problema na operadora: a janela não diz o trecho.
        assert_eq!(resumo.na_operadora, 0);
    }

    fn reg(hora: u8, ms: u32, perda: f64, veredito: &str, trecho: Option<Trecho>) -> RegistroDaRota {
        RegistroDaRota {
            quando: 1_700_000_000 + hora as i64 * 3600,
            hora,
            dia_da_semana: 2,
            destino: "185.1.2.3".into(),
            do_jogo: true,
            chegou: true,
            mediana_ms: Some(ms),
            jitter_ms: Some(3.0),
            perda_pct: perda,
            veredito: veredito.into(),
            trecho,
            salto: trecho.map(|_| 4),
            operadora: Some("Vivo".into()),
        }
    }

    #[test]
    fn rota_sem_resposta_vira_registro_de_nao_mediu() {
        let rota = crate::plataforma::rota::montar("1.1.1.1".parse().unwrap(), &[], &[]);
        let r = registro_de(&rota, 10, 30, 9, false);
        assert_eq!(r.veredito, "nao_mediu");
        assert_eq!((r.hora, r.dia_da_semana, r.perda_pct), (23, 6, 100.0));
        assert_eq!(r.destino, "1.1.1.1");
    }

    #[test]
    fn as_horas_caem_no_periodo_certo() {
        assert_eq!(Periodo::da_hora(0), Periodo::Madrugada);
        assert_eq!(Periodo::da_hora(5), Periodo::Madrugada);
        assert_eq!(Periodo::da_hora(6), Periodo::Manha);
        assert_eq!(Periodo::da_hora(12), Periodo::Tarde);
        assert_eq!(Periodo::da_hora(18), Periodo::Noite);
        assert_eq!(Periodo::da_hora(23), Periodo::Noite);
    }

    #[test]
    fn ping_que_sobe_a_noite_vira_frase_com_os_dois_numeros() {
        let v = vec![
            reg(9, 30, 0.0, "limpa", None),
            reg(10, 32, 0.0, "limpa", None),
            reg(20, 60, 0.0, "limpa", None),
            reg(21, 64, 0.0, "limpa", None),
        ];
        let f = resumir(&v).frase.unwrap();
        assert!(f.starts_with("À noite o ping sobe"), "{f}");
        assert!(f.contains("64 ms") && f.contains("32 ms") && f.contains("de manhã"), "{f}");
    }

    #[test]
    fn perda_pesa_mais_que_ping() {
        let v = vec![
            reg(9, 30, 0.0, "limpa", None),
            reg(10, 30, 0.0, "limpa", None),
            reg(20, 31, 20.0, "perda", Some(Trecho::Operadora)),
            reg(21, 31, 10.0, "perda", Some(Trecho::Operadora)),
        ];
        let r = resumir(&v);
        let f = r.frase.unwrap();
        assert!(f.contains("perde mais pacotes") && f.contains("15%"), "{f}");
        assert!(f.contains("2 de 2"), "{f}");
        assert_eq!(r.na_operadora, 2);
    }

    #[test]
    fn um_periodo_so_ou_medida_isolada_nao_tira_conclusao() {
        let v = vec![reg(9, 30, 0.0, "limpa", None), reg(10, 31, 0.0, "limpa", None), reg(20, 90, 0.0, "limpa", None)];
        assert_eq!(resumir(&v).frase, None);
    }

    #[test]
    fn diferenca_pequena_diz_que_esta_parecido() {
        let v = vec![
            reg(9, 30, 0.0, "limpa", None),
            reg(10, 31, 0.0, "limpa", None),
            reg(20, 34, 0.0, "limpa", None),
            reg(21, 35, 0.0, "limpa", None),
        ];
        assert!(resumir(&v).frase.unwrap().contains("parecido"));
    }

    #[test]
    fn o_relatorio_so_traz_o_que_e_da_operadora() {
        let v = vec![
            reg(9, 30, 0.0, "limpa", None),
            reg(20, 40, 30.0, "perda", Some(Trecho::Operadora)),
            reg(21, 40, 20.0, "perda", Some(Trecho::Casa)),
        ];
        let t = relatorio(&v, |q| format!("T{q}")).unwrap();
        assert!(t.contains("Operadora: Vivo"));
        assert!(t.contains("1 de 3"));
        assert!(t.contains("perda de 30%") && t.contains("salto 4"));
        assert!(!t.contains("perda de 20%"), "problema da casa não vai para a operadora");
        assert_eq!(relatorio(&v[..1], |q| q.to_string()), None);
    }

    #[test]
    fn guarda_no_maximo_o_limite_e_nao_sobrescreve_arquivo_ilegivel() {
        let pasta = std::env::temp_dir().join(format!("otimiza-rota-hist-{}", std::process::id()));
        let arq = pasta.join("h.json");
        let _ = fs::remove_dir_all(&pasta);
        for i in 0..(GUARDADOS + 5) {
            registrar_em(&arq, reg((i % 24) as u8, 30, 0.0, "limpa", None)).unwrap();
        }
        assert_eq!(ler_de(&arq).unwrap().len(), GUARDADOS);
        fs::write(&arq, "lixo").unwrap();
        assert!(registrar_em(&arq, reg(1, 1, 0.0, "limpa", None)).is_err());
        assert_eq!(fs::read_to_string(&arq).unwrap(), "lixo");
        let _ = fs::remove_dir_all(&pasta);
    }
}
