// Diagnóstico de DPC e interrupções (2.9)
//
// Driver mal escrito (rede, áudio, USB, placa de vídeo, controle de energia)
// prende um núcleo em rotina de interrupção (ISR) ou em chamada adiada (DPC).
// Enquanto isso o núcleo não roda o jogo, e o que se sente é engasgo e som
// estalando — com FPS médio normal.
//
// O QUE ESTE MÓDULO MEDE, E O QUE NÃO MEDE
//
// Mede, pelos contadores do Windows, quanto de cada núcleo foi gasto em DPC e
// em interrupção durante alguns segundos. Diz QUAL NÚCLEO e QUANTO.
//
// Não diz QUAL DRIVER. Isso exige rastreamento do kernel (ETW), que precisa de
// administrador e de uma sessão de rastreio própria — ainda não está no
// produto, e a tela diz isso em vez de chutar um culpado.
//
// Só lê. Nada é escrito.
//
// Os limites de "alto" abaixo são REFERÊNCIA, não validados em várias
// máquinas: em PC ocioso o normal é bem abaixo de 1%; um núcleo passando de
// alguns por cento em média, ou com picos de dois dígitos, merece olhar.

use serde::Serialize;

/// Média em DPC + interrupção, num núcleo, a partir da qual vale olhar (%).
pub const MEDIA_ALTA: f64 = 3.0;
/// Pico de uma amostra, num núcleo, a partir do qual vale olhar (%).
pub const PICO_ALTO: f64 = 15.0;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NucleoDpc {
    pub nucleo: String,
    pub dpc_medio: f64,
    pub dpc_pico: f64,
    pub interrupcao_media: f64,
    pub interrupcao_pico: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "estado")]
pub enum EstadoDpc {
    /// Nenhum núcleo passou das referências.
    Normal,
    /// Um núcleo passou. `nucleo` é o pior.
    Alto { nucleo: String },
    /// Os contadores não responderam.
    NaoDeuParaLer { motivo: String },
}

#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticoDpc {
    pub segundos: u32,
    pub nucleos: Vec<NucleoDpc>,
    pub estado: EstadoDpc,
}

/// Instância de núcleo, não de total (`_Total`, `0,_Total`).
fn e_nucleo(nome: &str) -> bool {
    !nome.contains("_Total")
}

/// **Pura.** Junta as amostras (uma lista por coleta, com o valor de cada
/// núcleo) em média e pico por núcleo.
pub fn resumir(dpc: &[Vec<(String, f64)>], interrupcao: &[Vec<(String, f64)>]) -> Vec<NucleoDpc> {
    use std::collections::BTreeMap;
    let mut soma: BTreeMap<String, (f64, f64, usize, f64, f64, usize)> = BTreeMap::new();
    for amostra in dpc {
        for (n, v) in amostra.iter().filter(|(n, _)| e_nucleo(n)) {
            let e = soma.entry(n.clone()).or_default();
            e.0 += v;
            e.1 = e.1.max(*v);
            e.2 += 1;
        }
    }
    for amostra in interrupcao {
        for (n, v) in amostra.iter().filter(|(n, _)| e_nucleo(n)) {
            let e = soma.entry(n.clone()).or_default();
            e.3 += v;
            e.4 = e.4.max(*v);
            e.5 += 1;
        }
    }
    soma.into_iter()
        .map(|(nucleo, (sd, pd, nd, si, pi, ni))| NucleoDpc {
            nucleo,
            dpc_medio: if nd > 0 { sd / nd as f64 } else { 0.0 },
            dpc_pico: pd,
            interrupcao_media: if ni > 0 { si / ni as f64 } else { 0.0 },
            interrupcao_pico: pi,
        })
        .collect()
}

/// **Pura.** O pior núcleo, se algum passou das referências.
pub fn julgar(nucleos: &[NucleoDpc]) -> EstadoDpc {
    let carga = |n: &NucleoDpc| n.dpc_medio + n.interrupcao_media;
    let pico = |n: &NucleoDpc| n.dpc_pico.max(n.interrupcao_pico);
    let pior = nucleos
        .iter()
        .filter(|n| carga(n) >= MEDIA_ALTA || pico(n) >= PICO_ALTO)
        .max_by(|a, b| carga(a).total_cmp(&carga(b)));
    match pior {
        Some(n) => EstadoDpc::Alto { nucleo: n.nucleo.clone() },
        None => EstadoDpc::Normal,
    }
}

/// Mede por `segundos` (amostra a cada meio segundo).
#[cfg(windows)]
pub fn medir(segundos: u32) -> DiagnosticoDpc {
    use crate::core::pdh::Consulta;
    let falha = |motivo: &str| DiagnosticoDpc {
        segundos,
        nucleos: Vec::new(),
        estado: EstadoDpc::NaoDeuParaLer { motivo: motivo.to_string() },
    };
    let Some(consulta) = Consulta::nova() else {
        return falha("os contadores de desempenho do Windows não abriram nesta máquina.");
    };
    let c_dpc = consulta.adicionar(r"\Processor Information(*)\% DPC Time");
    let c_int = consulta.adicionar(r"\Processor Information(*)\% Interrupt Time");
    consulta.coletar();
    let mut dpc = Vec::new();
    let mut int = Vec::new();
    for _ in 0..(segundos * 2) {
        std::thread::sleep(std::time::Duration::from_millis(500));
        if consulta.coletar() {
            dpc.push(consulta.lista(c_dpc));
            int.push(consulta.lista(c_int));
        }
    }
    let nucleos = resumir(&dpc, &int);
    if nucleos.is_empty() {
        return falha("os contadores de DPC e de interrupção não responderam — costuma ser Windows modificado sem o serviço de contadores.");
    }
    let estado = julgar(&nucleos);
    DiagnosticoDpc { segundos, nucleos, estado }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a(v: &[(&str, f64)]) -> Vec<(String, f64)> {
        v.iter().map(|(n, x)| (n.to_string(), *x)).collect()
    }

    #[test]
    fn o_total_nao_entra_como_nucleo() {
        let r = resumir(&[a(&[("0,0", 1.0), ("0,_Total", 5.0), ("_Total", 5.0)])], &[]);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].nucleo, "0,0");
    }

    #[test]
    fn media_e_pico_por_nucleo() {
        let dpc = [a(&[("0,0", 1.0), ("0,1", 0.0)]), a(&[("0,0", 3.0), ("0,1", 0.0)])];
        let int = [a(&[("0,0", 0.5)]), a(&[("0,0", 1.5)])];
        let r = resumir(&dpc, &int);
        let n0 = r.iter().find(|n| n.nucleo == "0,0").unwrap();
        assert_eq!(n0.dpc_medio, 2.0);
        assert_eq!(n0.dpc_pico, 3.0);
        assert_eq!(n0.interrupcao_media, 1.0);
        assert_eq!(n0.interrupcao_pico, 1.5);
    }

    #[test]
    fn maquina_quieta_e_normal_e_nucleo_preso_e_apontado() {
        let quieta = resumir(&[a(&[("0,0", 0.2), ("0,1", 0.1)])], &[a(&[("0,0", 0.1), ("0,1", 0.1)])]);
        assert_eq!(julgar(&quieta), EstadoDpc::Normal);

        let presa = resumir(&[a(&[("0,0", 0.2), ("0,3", 4.0)])], &[a(&[("0,0", 0.1), ("0,3", 1.0)])]);
        assert_eq!(julgar(&presa), EstadoDpc::Alto { nucleo: "0,3".into() });

        let pico = resumir(&[a(&[("0,2", 0.0)]), a(&[("0,2", 20.0)])], &[]);
        assert_eq!(julgar(&pico), EstadoDpc::Alto { nucleo: "0,2".into() });
    }
}

#[cfg(test)]
mod nesta_maquina {
    #[test]
    #[ignore]
    fn dpc_desta_maquina() {
        println!("{:#?}", super::medir(5));
    }
}
