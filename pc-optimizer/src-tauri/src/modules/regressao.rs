// O produto olhando para o próprio trabalho
//
// POR QUE ESTE MÓDULO EXISTE, sem rodeio: na 2.1.0 um cliente aplicou tudo que
// o Otimiza oferece e o FPS dele caiu de cerca de 200 para 80-120 no FiveM. O
// produto estava instalado naquela máquina. Ele mediu os quadros ANTES — o
// vigia de `medicoes.rs` faz isso sozinho a cada vinte minutos de jogo — e
// mediu DEPOIS. Os dois números estavam no disco do cliente, no mesmo arquivo,
// um embaixo do outro.
//
// E ninguém comparou. O produto não tinha como perceber que tinha piorado a
// máquina que ele cobrou para melhorar. Quem percebeu foi o cliente, olhando o
// contador de quadros, e quem contou foi ele — dias depois, por mensagem.
//
// Este módulo é a comparação que faltava, e nada além dela: recebe as medições
// já lidas e devolve um veredito. Nenhuma leitura de disco, nenhuma escrita,
// nenhuma chamada ao Windows — para que as regras abaixo possam ser provadas
// sem abrir jogo nenhum.
//
// ─────────────────────────────────────────────────────────────────────────
// AS QUATRO REGRAS, e o que cada uma impede
//
// 1. SÓ COMPARA O MESMO JOGO. FPS de jogos diferentes não se compara, e
//    misturar dois jogos no mesmo cálculo produz "queda" onde só houve o
//    cliente trocar de jogo.
//
// 2. SÓ COMPARA MEDIÇÃO CONFIÁVEL. `MedicaoAutomatica::confiavel` é falso
//    quando a amostra foi curta demais. Amostra curta não vira evidência de
//    nada — nem a favor nem contra.
//
// 3. PRECISA DE AMOSTRA DOS DOIS LADOS. Uma medição antes e uma depois é uma
//    coincidência, não uma tendência: num jogo aberto o FPS muda com o lugar
//    do mapa, com quanta gente tem na tela e com o que mais está aberto. Por
//    isso o piso é `AMOSTRAS_MINIMAS` de cada lado, e a conta é sobre a
//    MEDIANA — uma partida ruim não arrasta a mediana como arrasta a média.
//
// 4. A MARGEM É LARGA DE PROPÓSITO. Variação de 5% entre duas sessões do
//    mesmo jogo é o normal do mundo, não sinal de nada. Gritar "piorou" em
//    cima de ruído gasta a confiança do cliente exatamente como prometer ganho
//    que não existe — e este produto se vende por não fazer nenhum dos dois.
//
// ─────────────────────────────────────────────────────────────────────────
// O QUE ESTE MÓDULO NÃO FAZ, e por que não faz
//
// Ele NÃO desfaz nada sozinho. As medições de antes e de depois vêm de sessões
// diferentes, em dias diferentes, em lugares diferentes do mapa. Isso é forte o
// bastante para AVISAR e mostrar os números; não é forte o bastante para o
// produto desfazer, sozinho e sem a pessoa por perto, um trabalho que ela pediu
// — inclusive porque desfazer exige administrador e às vezes reiniciar.
//
// O veredito vai para a tela com o número dos dois lados e um botão. Quem
// decide é quem joga. Ver `Desfecho::PiorouMuito`, que é onde o aviso aparece
// em cima de tudo.

use serde::{Deserialize, Serialize};

use crate::modules::medicoes::MedicaoAutomatica;

/// Quantas medições confiáveis são necessárias de CADA lado.
///
/// Duas, e não uma: uma medição de cada lado é uma coincidência. Não é três
/// porque três de cada lado significa esperar mais de uma hora de jogo depois
/// de otimizar para o produto poder avisar — e o cliente que perdeu FPS
/// percebe em dez minutos.
pub const AMOSTRAS_MINIMAS: usize = 2;

/// A partir de quanto uma diferença deixa de ser ruído.
///
/// Oito por cento. Abaixo disso é a variação normal entre duas sessões do mesmo
/// jogo — lugar do mapa, gente na tela, o que mais estava aberto.
pub const MARGEM_DE_RUIDO_PCT: f64 = 8.0;

/// A partir de quanto a queda é grande demais para ser sessão ruim.
///
/// Vinte por cento. O caso que originou o módulo foi de cerca de 50%.
pub const QUEDA_GRAVE_PCT: f64 = 20.0;

/// O lado de uma comparação, já resumido.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Lado {
    /// Mediana do FPS das medições deste lado.
    pub fps: f64,
    /// Mediana do 1% pior.
    pub low_1pct: f64,
    pub amostras: usize,
}

/// O que as medições dizem sobre o trabalho do Otimiza neste jogo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "desfecho")]
pub enum Desfecho {
    /// Não há medição de antes, de depois, ou não há o bastante dos dois lados.
    ///
    /// NÃO é "está tudo bem". É "ainda não dá para dizer" — e a tela precisa
    /// dizer isso com essas palavras, porque a alternativa é o cliente ler
    /// silêncio como aprovação.
    SemAmostra,
    /// A diferença está dentro do ruído.
    Igual,
    Melhorou,
    /// Caiu mais que a margem, menos que o grave.
    Piorou,
    /// Caiu muito. É o caso do incidente da 2.1.0, e o único que interrompe o
    /// cliente em vez de esperar ele abrir a tela.
    PiorouMuito,
}

/// O veredito de um jogo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Veredito {
    pub jogo: String,
    pub desfecho: Desfecho,
    /// `None` quando faltou amostra daquele lado — e aí a tela não pode
    /// inventar um número para preencher.
    pub antes: Option<Lado>,
    pub depois: Option<Lado>,
    /// Variação do FPS, em por cento. Negativo é queda.
    pub variacao_fps_pct: Option<f64>,
    /// Variação do 1% pior, em por cento. É o número que descreve o engasgo, e
    /// por isso ele decide o desfecho junto com o FPS, e não depois dele.
    pub variacao_low_pct: Option<f64>,
}

/// Mediana de uma lista não vazia. `None` para lista vazia, em vez de zero:
/// zero FPS é um número que significa alguma coisa, e "não medi" não é ele.
fn mediana(mut valores: Vec<f64>) -> Option<f64> {
    if valores.is_empty() {
        return None;
    }

    valores.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let meio = valores.len() / 2;
    Some(if valores.len() % 2 == 0 {
        (valores[meio - 1] + valores[meio]) / 2.0
    } else {
        valores[meio]
    })
}

fn resumir(medicoes: &[&MedicaoAutomatica]) -> Option<Lado> {
    if medicoes.len() < AMOSTRAS_MINIMAS {
        return None;
    }

    Some(Lado {
        fps: mediana(medicoes.iter().map(|m| m.fps).collect())?,
        low_1pct: mediana(medicoes.iter().map(|m| m.low_1pct).collect())?,
        amostras: medicoes.len(),
    })
}

/// Variação percentual de `antes` para `depois`. `None` quando `antes` é zero
/// ou negativo — dividir por zero produziria infinito, e infinito na tela é
/// pior que a ausência do número.
fn variacao_pct(antes: f64, depois: f64) -> Option<f64> {
    if antes <= 0.0 {
        return None;
    }
    Some((depois - antes) / antes * 100.0)
}

/// Compara as medições de UM jogo.
///
/// `mudancas_na_epoca` separa os dois lados: medição feita com menos mudanças
/// aplicadas do que hoje é "antes"; com o mesmo tanto, é "depois". É o mesmo
/// campo que o vigia já grava desde a 2.0, e por isso esta comparação funciona
/// com o histórico que JÁ existe na máquina de todo cliente — não precisa
/// esperar ninguém medir nada de novo.
pub fn comparar(jogo: &str, medicoes: &[MedicaoAutomatica], aplicadas_hoje: usize) -> Veredito {
    // Regra 1 e Regra 2: mesmo jogo, e só o que é confiável.
    let deste_jogo: Vec<&MedicaoAutomatica> = medicoes
        .iter()
        .filter(|m| m.jogo.eq_ignore_ascii_case(jogo) && m.confiavel)
        .collect();

    let antes: Vec<&MedicaoAutomatica> = deste_jogo
        .iter()
        .copied()
        .filter(|m| m.mudancas_aplicadas < aplicadas_hoje)
        .collect();

    let depois: Vec<&MedicaoAutomatica> = deste_jogo
        .iter()
        .copied()
        .filter(|m| m.mudancas_aplicadas >= aplicadas_hoje)
        .collect();

    let (antes, depois) = (resumir(&antes), resumir(&depois));

    // Regra 3: sem os dois lados não há comparação. Devolver `Igual` aqui seria
    // o produto dizendo "não mudou nada" sobre uma conta que ele não fez.
    let (Some(a), Some(d)) = (antes.clone(), depois.clone()) else {
        return Veredito {
            jogo: jogo.to_string(),
            desfecho: Desfecho::SemAmostra,
            antes,
            depois,
            variacao_fps_pct: None,
            variacao_low_pct: None,
        };
    };

    let variacao_fps_pct = variacao_pct(a.fps, d.fps);
    let variacao_low_pct = variacao_pct(a.low_1pct, d.low_1pct);

    // A pior das duas manda. Um ajuste que segura o FPS médio e destrói o 1%
    // pior deixou o jogo pior de jogar, e chamar isso de "igual" seria usar a
    // média para esconder exatamente o que o cliente sente.
    let pior = [variacao_fps_pct, variacao_low_pct]
        .into_iter()
        .flatten()
        .fold(f64::INFINITY, f64::min);

    let desfecho = if !pior.is_finite() {
        Desfecho::SemAmostra
    } else if pior <= -QUEDA_GRAVE_PCT {
        Desfecho::PiorouMuito
    } else if pior <= -MARGEM_DE_RUIDO_PCT {
        Desfecho::Piorou
    } else if variacao_fps_pct.unwrap_or(0.0) >= MARGEM_DE_RUIDO_PCT {
        Desfecho::Melhorou
    } else {
        Desfecho::Igual
    };

    Veredito {
        jogo: jogo.to_string(),
        desfecho,
        antes,
        depois,
        variacao_fps_pct,
        variacao_low_pct,
    }
}

/// Um veredito por jogo medido, do pior para o melhor.
///
/// A ordem não é estética: quem tem um jogo que piorou precisa ver esse jogo
/// primeiro, e não rolar uma lista atrás dele.
pub fn todos(medicoes: &[MedicaoAutomatica], aplicadas_hoje: usize) -> Vec<Veredito> {
    let mut jogos: Vec<String> = Vec::new();
    for m in medicoes {
        if !jogos.iter().any(|j| j.eq_ignore_ascii_case(&m.jogo)) {
            jogos.push(m.jogo.clone());
        }
    }

    let mut vereditos: Vec<Veredito> = jogos
        .iter()
        .map(|j| comparar(j, medicoes, aplicadas_hoje))
        .collect();

    vereditos.sort_by(|a, b| {
        let chave = |v: &Veredito| v.variacao_fps_pct.unwrap_or(f64::INFINITY);
        chave(a)
            .partial_cmp(&chave(b))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    vereditos
}

/// O jogo que piorou mais, se algum piorou.
///
/// É o que a tela usa para interromper o cliente na abertura, em vez de esperar
/// ele procurar. Um aviso que só aparece para quem foi olhar não teria evitado
/// o incidente da 2.1.0.
pub fn pior_regressao(vereditos: &[Veredito]) -> Option<&Veredito> {
    vereditos
        .iter()
        .find(|v| matches!(v.desfecho, Desfecho::Piorou | Desfecho::PiorouMuito))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn medicao(jogo: &str, fps: f64, low: f64, aplicadas: usize) -> MedicaoAutomatica {
        MedicaoAutomatica {
            jogo: jogo.to_string(),
            quando: 0,
            fps,
            low_1pct: low,
            engasgos_por_minuto: 0.0,
            segundos: 20.0,
            confiavel: true,
            mudancas_aplicadas: aplicadas,
            frametime_medio_ms: None,
            frametime_p95_ms: None,
            frametime_p99_ms: None,
            cpu_uso_pct: None,
            gpu_uso_pct: None,
        }
    }

    /// O caso que originou o módulo, com os números que o cliente relatou.
    #[test]
    fn a_queda_de_200_para_100_e_apontada_como_grave() {
        let medicoes = vec![
            medicao("FiveM.exe", 205.0, 150.0, 0),
            medicao("FiveM.exe", 198.0, 144.0, 0),
            medicao("FiveM.exe", 96.0, 70.0, 14),
            medicao("FiveM.exe", 108.0, 74.0, 14),
        ];

        let v = comparar("FiveM.exe", &medicoes, 14);

        assert_eq!(v.desfecho, Desfecho::PiorouMuito);
        assert!(v.variacao_fps_pct.unwrap() < -40.0);
        assert_eq!(v.antes.unwrap().amostras, 2);
    }

    /// Regra 4: a variação normal entre duas sessões não pode virar acusação.
    #[test]
    fn variacao_pequena_nao_vira_acusacao() {
        let medicoes = vec![
            medicao("gta.exe", 100.0, 80.0, 0),
            medicao("gta.exe", 104.0, 82.0, 0),
            medicao("gta.exe", 98.0, 79.0, 9),
            medicao("gta.exe", 101.0, 78.0, 9),
        ];

        assert_eq!(comparar("gta.exe", &medicoes, 9).desfecho, Desfecho::Igual);
    }

    #[test]
    fn ganho_de_verdade_e_reconhecido() {
        let medicoes = vec![
            medicao("gta.exe", 60.0, 45.0, 0),
            medicao("gta.exe", 62.0, 46.0, 0),
            medicao("gta.exe", 88.0, 66.0, 7),
            medicao("gta.exe", 91.0, 68.0, 7),
        ];

        assert_eq!(comparar("gta.exe", &medicoes, 7).desfecho, Desfecho::Melhorou);
    }

    /// O ajuste que segura a média e destrói o engasgo deixou o jogo pior. A
    /// média sozinha diria "igual", e é por isso que ela não decide sozinha.
    #[test]
    fn media_estavel_com_o_um_por_cento_pior_desabando_nao_e_igual() {
        let medicoes = vec![
            medicao("fivem.exe", 120.0, 95.0, 0),
            medicao("fivem.exe", 118.0, 93.0, 0),
            medicao("fivem.exe", 119.0, 55.0, 11),
            medicao("fivem.exe", 121.0, 52.0, 11),
        ];

        let v = comparar("fivem.exe", &medicoes, 11);

        assert_eq!(v.desfecho, Desfecho::PiorouMuito);
        assert!(v.variacao_fps_pct.unwrap().abs() < 3.0, "o FPS médio mal se moveu");
    }

    /// Regra 3: sem os dois lados o produto diz que não sabe. `Igual` aqui
    /// seria afirmar o resultado de uma conta que não foi feita.
    #[test]
    fn um_lado_so_nao_vira_veredito() {
        let medicoes = vec![
            medicao("fivem.exe", 200.0, 150.0, 0),
            medicao("fivem.exe", 198.0, 148.0, 0),
            medicao("fivem.exe", 90.0, 60.0, 14),
        ];

        let v = comparar("fivem.exe", &medicoes, 14);

        assert_eq!(v.desfecho, Desfecho::SemAmostra);
        assert!(v.depois.is_none(), "uma medição só não resume nada");
        assert!(v.variacao_fps_pct.is_none(), "não há variação para mostrar");
    }

    /// Regra 2: amostra curta não é evidência nem a favor nem contra.
    #[test]
    fn medicao_nao_confiavel_fica_de_fora_dos_dois_lados() {
        let mut curta = medicao("fivem.exe", 30.0, 10.0, 14);
        curta.confiavel = false;

        let medicoes = vec![
            medicao("fivem.exe", 100.0, 80.0, 0),
            medicao("fivem.exe", 102.0, 81.0, 0),
            medicao("fivem.exe", 101.0, 80.0, 14),
            medicao("fivem.exe", 99.0, 79.0, 14),
            curta,
        ];

        let v = comparar("fivem.exe", &medicoes, 14);

        assert_eq!(v.desfecho, Desfecho::Igual);
        assert_eq!(v.depois.unwrap().amostras, 2, "a medição curta entrou na conta");
    }

    /// Regra 1: dois jogos no mesmo arquivo não se misturam.
    #[test]
    fn o_fps_de_um_jogo_nao_entra_na_conta_do_outro() {
        let medicoes = vec![
            medicao("fivem.exe", 100.0, 80.0, 0),
            medicao("fivem.exe", 102.0, 81.0, 0),
            medicao("cs2.exe", 400.0, 300.0, 0),
            medicao("cs2.exe", 410.0, 305.0, 0),
            medicao("fivem.exe", 99.0, 79.0, 5),
            medicao("fivem.exe", 101.0, 80.0, 5),
        ];

        let v = comparar("fivem.exe", &medicoes, 5);

        assert_eq!(v.antes.unwrap().amostras, 2, "o CS entrou na conta do FiveM");
        assert_eq!(v.desfecho, Desfecho::Igual);
    }

    #[test]
    fn a_lista_traz_o_pior_jogo_primeiro() {
        let medicoes = vec![
            medicao("bom.exe", 60.0, 50.0, 0),
            medicao("bom.exe", 61.0, 50.0, 0),
            medicao("bom.exe", 90.0, 70.0, 6),
            medicao("bom.exe", 92.0, 71.0, 6),
            medicao("ruim.exe", 200.0, 150.0, 0),
            medicao("ruim.exe", 202.0, 152.0, 0),
            medicao("ruim.exe", 100.0, 70.0, 6),
            medicao("ruim.exe", 98.0, 68.0, 6),
        ];

        let lista = todos(&medicoes, 6);

        assert_eq!(lista[0].jogo, "ruim.exe");
        assert_eq!(pior_regressao(&lista).unwrap().jogo, "ruim.exe");
    }

    #[test]
    fn sem_nenhuma_regressao_nao_ha_o_que_avisar() {
        let medicoes = vec![
            medicao("bom.exe", 60.0, 50.0, 0),
            medicao("bom.exe", 61.0, 50.0, 0),
            medicao("bom.exe", 90.0, 70.0, 6),
            medicao("bom.exe", 92.0, 71.0, 6),
        ];

        assert!(pior_regressao(&todos(&medicoes, 6)).is_none());
    }

    /// Nenhum histórico é o caso de toda máquina recém-instalada, e ele não
    /// pode virar pânico nem elogio.
    #[test]
    fn arquivo_vazio_nao_inventa_veredito() {
        assert!(todos(&[], 0).is_empty());
    }

    #[test]
    fn fps_zero_antes_nao_vira_divisao_por_zero() {
        let medicoes = vec![
            medicao("x.exe", 0.0, 0.0, 0),
            medicao("x.exe", 0.0, 0.0, 0),
            medicao("x.exe", 100.0, 80.0, 3),
            medicao("x.exe", 102.0, 81.0, 3),
        ];

        let v = comparar("x.exe", &medicoes, 3);

        assert!(v.variacao_fps_pct.is_none(), "infinito não vai para a tela");
        assert_eq!(v.desfecho, Desfecho::SemAmostra);
    }
}
