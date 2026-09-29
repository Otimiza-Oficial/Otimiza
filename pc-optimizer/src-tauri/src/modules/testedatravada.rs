// Teste ativo da travada (docs/planos/PIP-ENGENHARIA.md, seção 9): o programa suspeito é acalmado e devolvido em
// janelas de 30 s alternadas, com o jogo aberto, e o que se compara é a TAXA de travadas, não o FPS médio. Na prova
// alternada desta máquina o menor efeito detectável no FPS médio foi ~22% (ruído de cena do FiveM); uma causa real de
// travada, removida, derruba a taxa dela muito mais que isso.
//
// Estatística: duas taxas de Poisson com correção de superdispersão (quasi-Poisson; a cena muda de janela para
// janela), razão de taxas com IC em log. Mantém só com redução ≥ 30% e o FPS guardado.
//
// Desenho 2 (29/09/2026). O desenho 1 (8 janelas, 4 min, duas olhadas) FALHOU o critério pré-registrado: com 60% menos
// travadas a 12/min, confirmava a causa em 32% das simulações (pedido: 80%). A grade de poder
// (`grade_de_poder::poder_por_desenho`) mostrou 0,33 com 4 min, 0,67 com 8 min e 0,82 com 12 min. Por isso: até 24
// janelas de 30 s, três olhadas (8, 16, 24) com os limites de O'Brien-Fleming (z = 3,47 · 2,45 · 2,00, α total 0,05):
// para cedo só com efeito enorme e guarda quase todo o poder do desenho fixo. Bonferroni (2,39 em cada olhada) foi
// medido antes e ficou em 0,69: conservador demais. E o teste não é oferecido abaixo de 6 travadas por minuto.
//
// Pura: recebe os quadros da captura e as janelas (instantes QPC de cada troca) e devolve o julgamento.

use serde::{Deserialize, Serialize};

use super::causadatravada::{detectar_episodios, QuadroDoJogo};
use crate::core::estatistica::mediana;

pub const SEGUNDOS_POR_JANELA: u64 = 30;
/// Depois de cada troca: o Windows leva um instante para reescalonar, e o quadro em voo é do lado anterior.
pub const DESCARTE_APOS_TROCA_S: f64 = 2.0;
pub const JANELAS_POR_OLHADA: usize = 8;
pub const OLHADAS: usize = 3;
/// ABBA·BAAB em cada olhada: anula deriva linear (calor, cena que muda devagar).
pub const ORDEM_DA_OLHADA: [LadoDoTeste; 8] = [
    LadoDoTeste::Normal,
    LadoDoTeste::Acalmado,
    LadoDoTeste::Acalmado,
    LadoDoTeste::Normal,
    LadoDoTeste::Acalmado,
    LadoDoTeste::Normal,
    LadoDoTeste::Normal,
    LadoDoTeste::Acalmado,
];
/// O'Brien-Fleming para três olhadas igualmente espaçadas, α = 0,05 bicaudal.
pub const Z_POR_OLHADA: [f64; OLHADAS] = [3.471, 2.454, 2.004];
/// Abaixo disso, nem as 24 janelas enxergam uma redução de 80% com poder razoável: o teste não é oferecido.
pub const TRAVADAS_POR_MINUTO_PARA_TESTAR: f64 = 6.0;

/// O lado da janela `i` (0 a 23).
pub fn lado_da_janela(i: usize) -> LadoDoTeste {
    ORDEM_DA_OLHADA[i % JANELAS_POR_OLHADA]
}
/// Melhoria mínima que importa: 30% menos travadas. Abaixo disso não vale manter uma mudança.
pub const RAZAO_QUE_IMPORTA: f64 = 0.7;
/// Queda de FPS médio que impede MANTER, sem precisar de significância ("nunca menos FPS").
pub const QUEDA_DE_FPS_MAXIMA: f64 = 0.05;
/// Queda que para o teste em qualquer olhada. Nas olhadas do meio, com 4 pares, o ruído de cena passa de 5% em ~12%
/// das vezes: parar por 5% ali encerrava testes bons por acaso (medido: derrubava o poder).
pub const QUEDA_DE_FPS_QUE_PARA: f64 = 0.15;
/// Janela com menos que isto dos quadros esperados: menu, carregamento ou jogo minimizado.
const FRACAO_MINIMA_DE_QUADROS: f64 = 0.5;
/// Buraco maior que isto entre quadros: PC suspenso ou jogo pausado.
const BURACO_MAXIMO_S: f64 = 1.0;
const MINIMO_DE_TRAVADAS: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LadoDoTeste {
    Normal,
    Acalmado,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Janela {
    pub lado: LadoDoTeste,
    pub inicio_qpc: i64,
    pub fim_qpc: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResumoDaJanela {
    pub lado: LadoDoTeste,
    pub travadas: usize,
    pub minutos: f64,
    pub fps: Option<f64>,
    pub valida: bool,
    pub motivo: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Decisao {
    /// Redução ≥ 30%, IC abaixo de 1 e FPS guardado.
    Manter,
    /// Reduziu com certeza, mas menos que o mínimo que importa.
    SemMelhoriaQueImporta,
    SemMelhoriaConfiavel,
    PiorouAsTravadas,
    PiorouOFps,
    Invalido,
    /// Olhada sem decisão: mais oito janelas.
    Continuar,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Julgamento {
    pub janelas: Vec<ResumoDaJanela>,
    pub travadas_normal: usize,
    pub minutos_normal: f64,
    pub travadas_acalmado: usize,
    pub minutos_acalmado: f64,
    /// Razão de taxas acalmado / normal: 0,4 = 60% menos travadas.
    pub razao: Option<f64>,
    /// IC com o z da olhada: já corrigido pelas três olhadas.
    pub ic: Option<(f64, f64)>,
    /// φ: variação entre janelas além do Poisson (1 = nenhuma).
    pub dispersao: f64,
    /// Variação média do FPS acalmado sobre o normal, pareada por bloco.
    pub variacao_do_fps: Option<f64>,
    pub decisao: Decisao,
    pub frase: String,
}

/// Resume cada janela: travadas (episódios da captura inteira que caem nela), minutos válidos e FPS.
pub fn resumir_janelas(quadros: &[QuadroDoJogo], janelas: &[Janela], frequencia: i64) -> Vec<ResumoDaJanela> {
    let episodios = detectar_episodios(quadros, frequencia);
    let f = frequencia.max(1) as f64;
    let mut intervalos: Vec<f64> = quadros.iter().map(|q| q.intervalo_ms).collect();
    intervalos.sort_by(f64::total_cmp);
    let fps_da_captura = mediana(&intervalos).filter(|m| *m > 0.0).map(|m| 1000.0 / m);

    janelas
        .iter()
        .map(|j| {
            let inicio = j.inicio_qpc + (DESCARTE_APOS_TROCA_S * f) as i64;
            let fim = j.fim_qpc;
            let segundos = ((fim - inicio) as f64 / f).max(0.0);
            let dentro: Vec<&QuadroDoJogo> = quadros.iter().filter(|q| q.qpc >= inicio && q.qpc < fim).collect();
            let travadas = episodios.iter().filter(|e| e.qpc >= inicio && e.qpc < fim).count();
            let esperado = fps_da_captura.map(|fps| fps * segundos).unwrap_or(0.0);
            let mut pontos: Vec<i64> = vec![inicio];
            pontos.extend(dentro.iter().map(|q| q.qpc));
            pontos.push(fim);
            let maior_buraco = pontos.windows(2).map(|w| (w[1] - w[0]) as f64 / f).fold(0.0, f64::max);
            let motivo = if dentro.is_empty() {
                Some("nenhum quadro do jogo (fechado, minimizado ou a medição parou)".to_string())
            } else if (dentro.len() as f64) < FRACAO_MINIMA_DE_QUADROS * esperado {
                Some(format!("só {} de ~{:.0} quadros esperados (menu, carregamento ou minimizado)", dentro.len(), esperado))
            } else if maior_buraco > BURACO_MAXIMO_S {
                Some(format!("{:.1} s sem nenhum quadro (PC suspenso ou jogo pausado)", maior_buraco))
            } else {
                None
            };
            ResumoDaJanela {
                lado: j.lado,
                travadas,
                minutos: segundos / 60.0,
                fps: (segundos > 0.0 && !dentro.is_empty()).then(|| dentro.len() as f64 / segundos),
                valida: motivo.is_none(),
                motivo,
            }
        })
        .collect()
}

/// φ de Pearson entre janelas do mesmo lado, com piso 1.
fn dispersao(janelas: &[&ResumoDaJanela]) -> f64 {
    let mut soma = 0.0;
    let mut gl = 0usize;
    for lado in [LadoDoTeste::Normal, LadoDoTeste::Acalmado] {
        let deste: Vec<&&ResumoDaJanela> = janelas.iter().filter(|j| j.lado == lado).collect();
        if deste.len() < 2 {
            continue;
        }
        let k: usize = deste.iter().map(|j| j.travadas).sum();
        let t: f64 = deste.iter().map(|j| j.minutos).sum();
        if k == 0 || t <= 0.0 {
            continue;
        }
        let taxa = k as f64 / t;
        soma += deste.iter().map(|j| (j.travadas as f64 - taxa * j.minutos).powi(2) / (taxa * j.minutos)).sum::<f64>();
        gl += deste.len() - 1;
    }
    if gl == 0 {
        1.0
    } else {
        (soma / gl as f64).max(1.0)
    }
}

/// FPS acalmado sobre normal, pareado em blocos de duas janelas vizinhas (uma de cada lado).
fn variacao_do_fps(janelas: &[ResumoDaJanela]) -> Option<f64> {
    let pares: Vec<f64> = janelas
        .chunks(2)
        .filter_map(|b| {
            let n = b.iter().find(|j| j.lado == LadoDoTeste::Normal && j.valida)?.fps?;
            let a = b.iter().find(|j| j.lado == LadoDoTeste::Acalmado && j.valida)?.fps?;
            (n > 0.0).then(|| (a - n) / n)
        })
        .collect();
    (!pares.is_empty()).then(|| pares.iter().sum::<f64>() / pares.len() as f64)
}

/// **Pura.** `olhada`: 1 a `OLHADAS`; na última não existe "continuar".
pub fn julgar(janelas: &[ResumoDaJanela], processo: &str, olhada: usize) -> Julgamento {
    let olhada = olhada.clamp(1, OLHADAS);
    let ultima = olhada == OLHADAS;
    let z = Z_POR_OLHADA[olhada - 1];
    let validas: Vec<&ResumoDaJanela> = janelas.iter().filter(|j| j.valida).collect();
    let soma = |lado| -> (usize, f64) {
        validas.iter().filter(|j| j.lado == lado).fold((0, 0.0), |(k, t), j| (k + j.travadas, t + j.minutos))
    };
    let (kn, tn) = soma(LadoDoTeste::Normal);
    let (ka, ta) = soma(LadoDoTeste::Acalmado);
    let phi = dispersao(&validas);
    let fps = variacao_do_fps(janelas);
    let base = |decisao, frase: String, razao, ic| Julgamento {
        janelas: janelas.to_vec(),
        travadas_normal: kn,
        minutos_normal: tn,
        travadas_acalmado: ka,
        minutos_acalmado: ta,
        razao,
        ic,
        dispersao: phi,
        variacao_do_fps: fps,
        decisao,
        frase,
    };

    let invalidas = janelas.len() - validas.len();
    if tn <= 0.0 || ta <= 0.0 {
        let motivo = janelas.iter().find_map(|j| j.motivo.clone()).unwrap_or_else(|| "sem janelas medidas".to_string());
        return base(Decisao::Invalido, format!("O teste não vale: faltou medir um dos lados ({}).", motivo), None, None);
    }
    if kn + ka < MINIMO_DE_TRAVADAS {
        return if ultima {
            base(
                Decisao::Invalido,
                format!(
                    "Só {} travada(s) nas janelas medidas: poucas para comparar. O jogo travou pouco durante o teste.",
                    kn + ka
                ),
                None,
                None,
            )
        } else {
            base(Decisao::Continuar, "Poucas travadas até agora: mais oito janelas.".to_string(), None, None)
        };
    }

    // Correção de continuidade só com zero de um lado (log de zero não existe).
    let (kn_c, ka_c) = if kn == 0 || ka == 0 { (kn as f64 + 0.5, ka as f64 + 0.5) } else { (kn as f64, ka as f64) };
    let razao = (ka_c / ta) / (kn_c / tn);
    let ep = (phi * (1.0 / kn_c + 1.0 / ka_c)).sqrt();
    let ic = ((razao.ln() - z * ep).exp(), (razao.ln() + z * ep).exp());
    let por_min = |k: usize, t: f64| k as f64 / t;
    let numeros = format!(
        "{:.1} travadas por minuto normal, {:.1} com o {} acalmado ({:+.0}%, faixa de {:+.0}% a {:+.0}%)",
        por_min(kn, tn),
        por_min(ka, ta),
        processo,
        (razao - 1.0) * 100.0,
        (ic.0 - 1.0) * 100.0,
        (ic.1 - 1.0) * 100.0
    );
    let aviso_invalidas =
        if invalidas > 0 { format!(" {} janela(s) ficaram de fora por não valerem.", invalidas) } else { String::new() };

    let fps_caiu = fps.is_some_and(|v| v < -QUEDA_DE_FPS_MAXIMA);
    let fps_despencou = fps.is_some_and(|v| v < -QUEDA_DE_FPS_QUE_PARA);
    // Sem FPS pareado não se sabe se o FPS caiu: não mantém ("nunca menos FPS" exige medir).
    let manteria = ic.1 < 1.0 && razao <= RAZAO_QUE_IMPORTA && fps.is_some();
    let (decisao, frase) = if fps_despencou || (fps_caiu && (manteria || ultima)) {
        (
            Decisao::PiorouOFps,
            format!(
                "O FPS caiu {:.0}% com o {} acalmado: não fica, porque o Otimiza não troca FPS por menos travadas. {}.",
                -fps.unwrap_or(0.0) * 100.0,
                processo,
                numeros
            ),
        )
    } else if manteria {
        (
            Decisao::Manter,
            format!(
                "Causa demonstrada: acalmar o {} diminuiu as travadas. {}. O FPS médio variou {:+.1}%.",
                processo,
                numeros,
                fps.unwrap_or(0.0) * 100.0
            ),
        )
    } else if ic.1 < 1.0 && razao <= RAZAO_QUE_IMPORTA {
        (
            Decisao::SemMelhoriaConfiavel,
            format!(
                "Acalmar o {} diminuiu as travadas, mas o FPS não teve comparação pareada (janelas vizinhas não valeram): \
                 sem medir o FPS, não fica. {}.",
                processo, numeros
            ),
        )
    } else if ic.1 < 1.0 {
        (
            Decisao::SemMelhoriaQueImporta,
            format!("Acalmar o {} diminuiu um pouco as travadas, menos que os 30% que valem manter: não fica. {}.", processo, numeros),
        )
    } else if ic.0 > 1.0 {
        (Decisao::PiorouAsTravadas, format!("Com o {} acalmado o jogo travou MAIS: não fica. {}.", processo, numeros))
    } else if !ultima {
        (Decisao::Continuar, format!("Ainda sem resposta: {}. Mais oito janelas.", numeros))
    } else {
        (
            Decisao::SemMelhoriaConfiavel,
            format!(
                "Não encontramos melhoria confiável acalmando o {}: não fica. {}. A associação que apareceu na \
                 investigação não se confirmou como causa.",
                processo, numeros
            ),
        )
    };
    base(decisao, format!("{}{}", frase, aviso_invalidas), Some(razao), Some(ic))
}

/// Um teste guardado: a memória de efetividade da travada, com os negativos.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Registro {
    pub quando: u64,
    /// `gamemode::chave_do_processo` do jogo.
    pub jogo: String,
    pub processo: String,
    pub decisao: Decisao,
    pub razao: Option<f64>,
    pub ic: Option<(f64, f64)>,
    pub travadas_normal: usize,
    pub minutos_normal: f64,
    pub travadas_acalmado: usize,
    pub minutos_acalmado: f64,
    pub variacao_do_fps: Option<f64>,
    pub versao_do_otimiza: String,
    pub driver: Option<String>,
    pub windows: Option<String>,
}

pub const REGISTROS_GUARDADOS: usize = 200;

pub fn registro(jogo: &str, processo: &str, j: &Julgamento, driver: Option<String>, windows: Option<String>) -> Registro {
    Registro {
        quando: crate::modules::changelog::now_timestamp(),
        jogo: jogo.to_string(),
        processo: processo.to_string(),
        decisao: j.decisao,
        razao: j.razao,
        ic: j.ic,
        travadas_normal: j.travadas_normal,
        minutos_normal: j.minutos_normal,
        travadas_acalmado: j.travadas_acalmado,
        minutos_acalmado: j.minutos_acalmado,
        variacao_do_fps: j.variacao_do_fps,
        versao_do_otimiza: env!("CARGO_PKG_VERSION").to_string(),
        driver,
        windows,
    }
}

/// **Pura.** Oferecer o teste só onde ele enxerga: abaixo de 6 travadas por minuto nem 12 min bastam.
pub fn pode_testar(travadas_por_minuto: f64) -> Result<(), String> {
    if travadas_por_minuto < TRAVADAS_POR_MINUTO_PARA_TESTAR {
        return Err(format!(
            "{:.1} travadas por minuto é pouco para um teste de até 12 minutos separar a causa do acaso (precisa de \
             {:.0} ou mais). Investigue num lugar do jogo onde ele trava mais.",
            travadas_por_minuto, TRAVADAS_POR_MINUTO_PARA_TESTAR
        ));
    }
    Ok(())
}

fn data(quando: u64) -> String {
    chrono::DateTime::from_timestamp(quando as i64, 0)
        .map(|d| d.with_timezone(&chrono::Local).format("%d/%m/%Y").to_string())
        .unwrap_or_else(|| "data desconhecida".to_string())
}

/// **Pura.** Liga cada hipótese de processo ao último teste dela neste jogo. Um teste que NÃO confirmou a causa, no
/// mesmo driver e no mesmo Windows, tira a oferta do teste: o Otimiza não repete o que já não funcionou aqui.
pub fn anotar_com_memoria(
    inv: &mut crate::modules::causadatravada::Investigacao,
    jogo: &str,
    memoria: &[Registro],
    driver: Option<&str>,
    windows: Option<&str>,
) {
    use crate::modules::causadatravada::{Hipotese, Teste};
    for a in inv.avaliacoes.iter_mut() {
        let Hipotese::Processo { nome } = &a.hipotese else { continue };
        let Some(r) = memoria.iter().rev().find(|r| r.jogo == jogo && r.processo.eq_ignore_ascii_case(nome)) else {
            continue;
        };
        let resultado = match r.decisao {
            Decisao::Manter => format!(
                "acalmar diminuiu as travadas ({:+.0}%)",
                r.razao.map(|x| (x - 1.0) * 100.0).unwrap_or(0.0)
            ),
            Decisao::SemMelhoriaConfiavel => "sem melhoria confiável".to_string(),
            Decisao::SemMelhoriaQueImporta => "melhoria pequena demais para manter".to_string(),
            Decisao::PiorouAsTravadas => "o jogo travou mais".to_string(),
            Decisao::PiorouOFps => "o FPS caiu".to_string(),
            Decisao::Invalido | Decisao::Continuar => "o teste não chegou a valer".to_string(),
        };
        a.ja_testado = Some(format!("Testado em {}: {}.", data(r.quando), resultado));
        let negativo = matches!(
            r.decisao,
            Decisao::SemMelhoriaConfiavel | Decisao::SemMelhoriaQueImporta | Decisao::PiorouAsTravadas | Decisao::PiorouOFps
        );
        let mesmo_contexto = r.driver.as_deref() == driver && r.windows.as_deref() == windows;
        if negativo && mesmo_contexto && matches!(a.teste, Teste::AcalmarProcesso { .. }) {
            a.teste = Teste::Recomendacao {
                texto: format!(
                    "Já testamos acalmar o {} nesta máquina e não resolveu: o Otimiza não repete o teste até o driver de \
                     vídeo ou o Windows mudar. A associação existe, mas a causa deve ser outra coisa que acontece junto.",
                    nome
                ),
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::causadatravada::simulacao::{quadros, Sorteio, FREQ};

    /// Uma captura com as janelas na ORDEM, travadas Poisson com a cena variando por janela (gama de forma 5) e o FPS
    /// variando 6% por janela. `efeito`: multiplicador da taxa no lado acalmado; `efeito_fps`: idem no FPS.
    fn teste_simulado(s: &mut Sorteio, janelas: usize, taxa_por_min: f64, efeito: f64, efeito_fps: f64) -> (Vec<QuadroDoJogo>, Vec<Janela>) {
        let mut todos: Vec<QuadroDoJogo> = Vec::new();
        let mut js = Vec::new();
        let mut t0 = 0.0f64;
        for i in 0..janelas {
            let lado = &lado_da_janela(i);
            // Gama(5, 1/5) por soma de exponenciais: média 1, CV ~45%.
            let cena: f64 = (0..5).map(|_| s.exp(1.0)).sum::<f64>() / 5.0;
            let fator = if *lado == LadoDoTeste::Acalmado { efeito } else { 1.0 };
            let taxa_s = taxa_por_min * cena * fator / 60.0;
            let mut travadas = Vec::new();
            let mut t = s.exp(taxa_s);
            while t < SEGUNDOS_POR_JANELA as f64 {
                travadas.push(t);
                t += s.exp(taxa_s);
            }
            let ms = 11.0 * (1.0 + 0.06 * s.normal()) / if *lado == LadoDoTeste::Acalmado { efeito_fps } else { 1.0 };
            let q = quadros(s, SEGUNDOS_POR_JANELA as f64, ms, &travadas);
            let deslocamento = (t0 * FREQ as f64) as i64;
            todos.extend(q.into_iter().map(|mut x| {
                x.qpc += deslocamento;
                x
            }));
            let inicio = deslocamento;
            js.push(Janela { lado: *lado, inicio_qpc: inicio, fim_qpc: inicio + SEGUNDOS_POR_JANELA as i64 * FREQ });
            t0 += SEGUNDOS_POR_JANELA as f64;
        }
        (todos, js)
    }

    /// O procedimento inteiro: olha em 8, 16 e 24. Devolve a decisão e quantas janelas usou.
    fn procedimento(s: &mut Sorteio, taxa: f64, efeito: f64, efeito_fps: f64) -> (Decisao, usize) {
        let total = JANELAS_POR_OLHADA * OLHADAS;
        let (q, j) = teste_simulado(s, total, taxa, efeito, efeito_fps);
        let r = resumir_janelas(&q, &j, FREQ);
        for olhada in 1..=OLHADAS {
            let ate = olhada * JANELAS_POR_OLHADA;
            let d = julgar(&r[..ate], "Rajada.exe", olhada).decisao;
            if d != Decisao::Continuar {
                return (d, ate);
            }
        }
        unreachable!("a última olhada sempre decide")
    }

    #[test]
    fn efeito_grande_e_mantido_e_frase_tem_os_numeros() {
        let mut s = Sorteio(42);
        let (q, j) = teste_simulado(&mut s, 8, 12.0, 0.2, 1.0);
        let r = resumir_janelas(&q, &j, FREQ);
        assert!(r.iter().all(|x| x.valida), "{:?}", r);
        let jg = julgar(&r, "Rajada.exe", OLHADAS);
        assert_eq!(jg.decisao, Decisao::Manter, "{}", jg.frase);
        assert!(jg.frase.contains("Rajada.exe") && jg.frase.contains("por minuto"), "{}", jg.frase);
        assert!(jg.ic.unwrap().1 < 1.0);
    }

    #[test]
    fn sem_fps_pareado_nao_mantem() {
        let janela = |lado, travadas, valida| ResumoDaJanela {
            lado,
            travadas,
            minutos: 0.47,
            fps: Some(100.0),
            valida,
            motivo: (!valida).then(|| "x".to_string()),
        };
        // Em cada bloco só um lado valeu: há minutos dos dois lados, mas nenhum par para o FPS.
        let r = vec![
            janela(LadoDoTeste::Normal, 12, true),
            janela(LadoDoTeste::Acalmado, 0, false),
            janela(LadoDoTeste::Acalmado, 1, true),
            janela(LadoDoTeste::Normal, 0, false),
            janela(LadoDoTeste::Acalmado, 1, true),
            janela(LadoDoTeste::Normal, 0, false),
            janela(LadoDoTeste::Normal, 12, true),
            janela(LadoDoTeste::Acalmado, 0, false),
        ];
        let jg = julgar(&r, "X.exe", OLHADAS);
        assert!(jg.variacao_do_fps.is_none());
        assert_ne!(jg.decisao, Decisao::Manter, "{}", jg.frase);
    }

    #[test]
    fn queda_de_fps_desfaz_mesmo_com_menos_travadas() {
        let mut s = Sorteio(43);
        let (q, j) = teste_simulado(&mut s, 8, 12.0, 0.2, 0.85);
        let r = resumir_janelas(&q, &j, FREQ);
        assert_eq!(julgar(&r, "Rajada.exe", OLHADAS).decisao, Decisao::PiorouOFps);
    }

    #[test]
    fn jogo_fechado_no_meio_invalida_as_janelas_sem_quadro() {
        let mut s = Sorteio(44);
        let (q, j) = teste_simulado(&mut s, 8, 12.0, 0.2, 1.0);
        let corte = j[5].inicio_qpc;
        let q: Vec<QuadroDoJogo> = q.into_iter().filter(|x| x.qpc < corte).collect();
        let r = resumir_janelas(&q, &j, FREQ);
        assert!(r[5..].iter().all(|x| !x.valida && x.motivo.is_some()));
        assert!(r[..5].iter().all(|x| x.valida));
        let jg = julgar(&r, "Rajada.exe", OLHADAS);
        assert!(jg.frase.contains("janela(s) ficaram de fora"), "{}", jg.frase);
    }

    #[test]
    fn pc_suspenso_no_meio_da_janela_invalida() {
        let mut s = Sorteio(45);
        let (mut q, j) = teste_simulado(&mut s, 4, 12.0, 1.0, 1.0);
        // Cinco segundos sem quadro no meio da janela 2 e o relógio pulando.
        let meio = j[2].inicio_qpc + 15 * FREQ;
        q.retain(|x| x.qpc < meio || x.qpc > meio + 5 * FREQ);
        let r = resumir_janelas(&q, &j, FREQ);
        assert!(!r[2].valida);
        assert!(r[2].motivo.as_deref().unwrap().contains("sem nenhum quadro"), "{:?}", r[2]);
    }

    #[test]
    fn um_lado_inteiro_sem_medida_e_invalido() {
        let r = vec![
            ResumoDaJanela { lado: LadoDoTeste::Normal, travadas: 5, minutos: 0.47, fps: Some(90.0), valida: true, motivo: None },
            ResumoDaJanela {
                lado: LadoDoTeste::Acalmado,
                travadas: 0,
                minutos: 0.47,
                fps: None,
                valida: false,
                motivo: Some("nenhum quadro do jogo".into()),
            },
        ];
        let jg = julgar(&r, "X.exe", OLHADAS);
        assert_eq!(jg.decisao, Decisao::Invalido);
        assert!(jg.frase.contains("não vale"));
    }

    #[test]
    fn poucas_travadas_continuam_e_depois_invalidam() {
        let janela = |lado, travadas| ResumoDaJanela { lado, travadas, minutos: 0.47, fps: Some(90.0), valida: true, motivo: None };
        let r = vec![janela(LadoDoTeste::Normal, 1), janela(LadoDoTeste::Acalmado, 0), janela(LadoDoTeste::Acalmado, 0), janela(LadoDoTeste::Normal, 0)];
        assert_eq!(julgar(&r, "X.exe", 1).decisao, Decisao::Continuar);
        assert_eq!(julgar(&r, "X.exe", OLHADAS).decisao, Decisao::Invalido);
    }

    #[test]
    fn superdispersao_alarga_o_intervalo() {
        let janela = |lado, travadas| ResumoDaJanela { lado, travadas, minutos: 0.5, fps: Some(90.0), valida: true, motivo: None };
        let calma = vec![janela(LadoDoTeste::Normal, 6), janela(LadoDoTeste::Acalmado, 3), janela(LadoDoTeste::Acalmado, 3), janela(LadoDoTeste::Normal, 6)];
        let agitada = vec![janela(LadoDoTeste::Normal, 11), janela(LadoDoTeste::Acalmado, 1), janela(LadoDoTeste::Acalmado, 5), janela(LadoDoTeste::Normal, 1)];
        let (a, b) = (julgar(&calma, "X", OLHADAS), julgar(&agitada, "X", OLHADAS));
        assert_eq!(a.dispersao, 1.0);
        assert!(b.dispersao > 1.0);
        let largura = |j: &Julgamento| j.ic.unwrap().1 / j.ic.unwrap().0;
        assert!(largura(&b) > largura(&a));
    }

    fn registro_de(decisao: Decisao, driver: &str) -> Registro {
        Registro {
            quando: 1_790_000_000,
            jogo: "fivem_".into(),
            processo: "Rajada.exe".into(),
            decisao,
            razao: Some(0.9),
            ic: Some((0.6, 1.3)),
            travadas_normal: 40,
            minutos_normal: 5.0,
            travadas_acalmado: 36,
            minutos_acalmado: 5.0,
            variacao_do_fps: Some(0.0),
            versao_do_otimiza: "3.2.0".into(),
            driver: Some(driver.into()),
            windows: Some("19045".into()),
        }
    }

    #[test]
    fn negativo_no_mesmo_contexto_tira_a_oferta_do_teste() {
        use crate::modules::causadatravada::{investigar, simulacao::sessao, Teste};
        let mut s = Sorteio(11);
        let (q, a) = sessao(&mut s, 120.0, 0.15, 0.6, 1.0);
        let base = investigar(&q, &a, FREQ, None, &|_: &str| false);
        assert!(matches!(base.avaliacoes[0].teste, Teste::AcalmarProcesso { .. }));
        let memoria = [registro_de(Decisao::SemMelhoriaConfiavel, "560.94")];

        let mut inv = base.clone();
        anotar_com_memoria(&mut inv, "fivem_", &memoria, Some("560.94"), Some("19045"));
        assert!(matches!(inv.avaliacoes[0].teste, Teste::Recomendacao { .. }));
        assert!(inv.avaliacoes[0].ja_testado.as_deref().unwrap().contains("sem melhoria confiável"));

        // Driver novo: vale testar de novo, e a tela ainda mostra o resultado de antes.
        let mut inv = base.clone();
        anotar_com_memoria(&mut inv, "fivem_", &memoria, Some("566.03"), Some("19045"));
        assert!(matches!(inv.avaliacoes[0].teste, Teste::AcalmarProcesso { .. }));
        assert!(inv.avaliacoes[0].ja_testado.is_some());

        // Outro jogo: nada anotado.
        let mut inv = base.clone();
        anotar_com_memoria(&mut inv, "cs2", &memoria, Some("560.94"), Some("19045"));
        assert!(inv.avaliacoes[0].ja_testado.is_none());

        // Teste que confirmou: continua oferecendo (revalidar).
        let mut inv = base;
        anotar_com_memoria(&mut inv, "fivem_", &[registro_de(Decisao::Manter, "560.94")], Some("560.94"), Some("19045"));
        assert!(matches!(inv.avaliacoes[0].teste, Teste::AcalmarProcesso { .. }));
    }

    #[test]
    fn teste_so_e_oferecido_com_travadas_suficientes() {
        assert!(pode_testar(5.9).is_err());
        assert!(pode_testar(6.0).is_ok());
    }

    /// Critério de aceite 5 (PIP-ENGENHARIA, seção 11), com o procedimento sequencial inteiro. Sementes novas em
    /// relação às já vistas (20_000/60_000 no desenho 1, 120_000/160_000 com Bonferroni): o critério é o mesmo, a amostra não é reaproveitada.
    #[test]
    fn aceite_poder_e_falso_positivo_do_teste() {
        let sims = 200;
        let (mut mantidos_com_efeito, mut mantidos_sem_efeito, mut janelas_usadas) = (0, 0, 0);
        for semente in 0..sims {
            let mut s = Sorteio(220_000 + semente);
            let (d, usadas) = procedimento(&mut s, 12.0, 0.4, 1.0);
            janelas_usadas += usadas;
            if d == Decisao::Manter {
                mantidos_com_efeito += 1;
            }
            let mut s = Sorteio(260_000 + semente);
            if procedimento(&mut s, 12.0, 1.0, 1.0).0 == Decisao::Manter {
                mantidos_sem_efeito += 1;
            }
        }
        let poder = mantidos_com_efeito as f64 / sims as f64;
        let falso = mantidos_sem_efeito as f64 / sims as f64;
        println!(
            "teste ativo: mantém com 60% menos travadas {:.3}, mantém sem efeito {:.3}, minutos médios com efeito {:.1}",
            poder,
            falso,
            janelas_usadas as f64 * SEGUNDOS_POR_JANELA as f64 / 60.0 / sims as f64
        );
        assert!(poder >= 0.80, "poder {}", poder);
        assert!(falso <= 0.05, "falso positivo {}", falso);
    }
}

#[cfg(test)]
mod grade_de_poder {
    //! Exploração (ignorada): poder do desenho por número de janelas, taxa de travadas e efeito. Uma decisão só.
    use super::*;
    use crate::modules::causadatravada::simulacao::{quadros, Sorteio, FREQ};

    fn simular(s: &mut Sorteio, janelas: usize, segundos: f64, taxa: f64, efeito: f64) -> Vec<ResumoDaJanela> {
        let mut todos = Vec::new();
        let mut js = Vec::new();
        for i in 0..janelas {
            let lado = lado_da_janela(i);
            let cena: f64 = (0..5).map(|_| s.exp(1.0)).sum::<f64>() / 5.0;
            let f = if lado == LadoDoTeste::Acalmado { efeito } else { 1.0 };
            let taxa_s = taxa * cena * f / 60.0;
            let mut tr = Vec::new();
            let mut t = s.exp(taxa_s);
            while t < segundos {
                tr.push(t);
                t += s.exp(taxa_s);
            }
            let desl = (i as f64 * segundos * FREQ as f64) as i64;
            todos.extend(quadros(s, segundos, 11.0, &tr).into_iter().map(|mut x| {
                x.qpc += desl;
                x
            }));
            js.push(Janela { lado, inicio_qpc: desl, fim_qpc: desl + (segundos * FREQ as f64) as i64 });
        }
        resumir_janelas(&todos, &js, FREQ)
    }

    #[test]
    #[ignore = "exploração do desenho"]
    fn poder_por_desenho() {
        for (janelas, segundos) in [(8usize, 30.0), (16, 15.0), (16, 30.0), (24, 30.0)] {
            for taxa in [6.0, 12.0, 24.0] {
                for efeito in [0.4, 0.2] {
                    let mut manteve = 0;
                    for sem in 0..150u64 {
                        let mut s = Sorteio(90_000 + sem);
                        let r = simular(&mut s, janelas, segundos, taxa, efeito);
                        let meio = janelas / 2;
                        let d = match julgar(&r[..meio], "X", 1).decisao {
                            Decisao::Continuar => julgar(&r, "X", OLHADAS).decisao,
                            d => d,
                        };
                        // (A grade foi feita com duas olhadas e o z do desenho 1; fica como registro da decisão.)
                        if d == Decisao::Manter {
                            manteve += 1;
                        }
                    }
                    println!("{:>2} janelas × {:>2.0} s ({:>3.0} min) · {:>4.0}/min · efeito -{:.0}% → poder {:.2}", janelas, segundos, janelas as f64 * segundos / 60.0, taxa, (1.0 - efeito) * 100.0, manteve as f64 / 150.0);
                }
            }
        }
    }
}

#[cfg(test)]
mod custo_do_motor {
    use super::*;
    use crate::modules::causadatravada::simulacao::{quadros, sessao, Sorteio, FREQ};

    /// Custo do motor puro no volume máximo: 24 janelas a ~180 FPS, e uma investigação de 60 s.
    #[test]
    #[ignore = "medição de desempenho (rodar em --release)"]
    fn custo_no_volume_maximo() {
        let mut s = Sorteio(7);
        let mut todos = Vec::new();
        let mut js = Vec::new();
        for i in 0..24 {
            let desl = i as i64 * SEGUNDOS_POR_JANELA as i64 * FREQ;
            todos.extend(quadros(&mut s, SEGUNDOS_POR_JANELA as f64, 5.5, &[3.0, 11.0, 19.0, 27.0]).into_iter().map(|mut x| {
                x.qpc += desl;
                x
            }));
            js.push(Janela { lado: lado_da_janela(i), inicio_qpc: desl, fim_qpc: desl + SEGUNDOS_POR_JANELA as i64 * FREQ });
        }
        let t = std::time::Instant::now();
        let r = resumir_janelas(&todos, &js, FREQ);
        for olhada in 1..=OLHADAS {
            let _ = julgar(&r[..olhada * JANELAS_POR_OLHADA], "X", olhada);
        }
        println!("teste de 12 min: {} quadros, {:.1} ms", todos.len(), t.elapsed().as_secs_f64() * 1000.0);

        let (q, a) = sessao(&mut s, 60.0, 0.15, 0.6, 1.0);
        let q: Vec<_> = q.into_iter().map(|mut x| { x.intervalo_ms /= 2.0; x }).collect();
        let t = std::time::Instant::now();
        let inv = crate::modules::causadatravada::investigar(&q, &a, FREQ, None, &|_: &str| false);
        println!("investigação: {} quadros, {} amostras, {:.1} ms ({} hipóteses)", q.len(), a.len(), t.elapsed().as_secs_f64() * 1000.0, inv.avaliacoes.len());
    }
}
