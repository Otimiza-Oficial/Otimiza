// Classificador de gargalo sobre a telemetria central
//
// POR QUE ISTO EXISTE, SE JÁ HÁ UM ANALISADOR DE GARGALO
//
// `windows::bottleneck` é uma SESSÃO de medição: ele abre a própria amostragem,
// mede alguns segundos com o cliente parado esperando, e responde uma vez. Vale
// o que vale — é a análise que o cliente pede de propósito.
//
// Este aqui é outra coisa. Ele não mede nada: recebe a telemetria que o painel
// já coletou e classifica o que estiver lá, a cada leitura, sem custo nenhum.
// E, principalmente, ele classifica com o que o contrato traz junto do número:
// a qualidade e a idade.
//
// AS DUAS REGRAS QUE ESTE MÓDULO EXISTE PARA SUSTENTAR
//
// 1. MÉTRICA AUSENTE NÃO É MÉTRICA FOLGADA.
//
//    Um classificador que lê `vram.usage` como `None` e segue em frente está
//    dizendo "a memória de vídeo está bem" sobre algo que ninguém olhou. Aqui
//    a classe que não pôde ser avaliada entra em `nao_verificado`, com o id da
//    métrica que faltou. A resposta "não olhei isto" aparece ao lado da
//    resposta "olhei e está no limite".
//
// 2. HIPÓTESE NÃO É CAUSA.
//
//    A flag de limite térmico do firmware ligada é uma CAUSA: o Windows está
//    dizendo, agora, que está segurando o processador. Uma placa de vídeo a
//    95% numa leitura de dez segundos atrás é uma HIPÓTESE: o número é real,
//    mas descreve um instante que já passou e veio de um contador agregado.
//    As duas coisas não podem aparecer na tela com o mesmo peso — é essa
//    diferença que separa um diagnóstico de um palpite bem formatado.
//
// O QUE ELE NÃO FAZ
//
// Não recomenda ajuste. O prompt do produto é explícito: nenhum tweak genérico
// antes da classificação. A recomendação é de quem tem o veredito na mão, e
// depende de um baseline que ainda não existe.

use serde::{Deserialize, Serialize};

use super::telemetry::{Quality, Telemetry};

/// A partir de quanto um recurso é considerado no limite.
///
/// Os mesmos 92% de `windows::bottleneck`, pela mesma razão: nenhum contador
/// fica cravado em 100, ele oscila; e um recurso a 80% ainda tem folga, então
/// chamar aquilo de gargalo mandaria o cliente trocar peça à toa.
pub const SATURADO: f64 = 92.0;

/// Abaixo disto o recurso está claramente sobrando.
///
/// Serve para o contraste: um núcleo a 99% ao lado de placa de vídeo a 35% é
/// uma história diferente de tudo a 95%.
pub const FOLGADO: f64 = 60.0;

/// Carga mínima para valer a pena julgar.
///
/// Máquina parada não tem gargalo, e apontar um seria inventar.
pub const CARGA_MINIMA: f64 = 15.0;

/// Memória cheia dói antes dos 92%.
///
/// RAM e VRAM não se comportam como processador: o sistema começa a paginar e
/// a despejar textura muito antes de o contador encostar no topo, e a partir
/// daí o problema aparece como engasgo, não como número alto.
pub const MEMORIA_APERTADA: f64 = 90.0;

/// A partir de quantos engasgos por minuto vale falar no assunto.
///
/// Seis é um a cada dez segundos — o ponto em que deixa de ser um tranco
/// isolado e vira a experiência da partida. Abaixo disso, apontar engasgo
/// mandaria o cliente caçar um problema que ele não sente.
pub const ENGASGOS_POR_MINUTO: f64 = 6.0;

/// A partir de que proporção o disco deixa de ser coincidência.
///
/// Sessenta por cento: a maioria clara dos trancos caindo junto com atividade
/// de disco. Abaixo disso é ruído — numa partida qualquer o disco trabalha de
/// vez em quando, e algum tranco vai cair por cima sem ter relação nenhuma.
pub const TRANCOS_COM_DISCO_PCT: f64 = 60.0;

/// A partir de quanta variação a rede atrapalha a partida.
///
/// Trinta milissegundos. Não é o ping: é o quanto ele PULA de uma resposta
/// para a outra. Um ping de 120 ms estável dá uma partida jogável; um de 40 ms
/// que vira 90 e volta é o que produz o teletransporte que o cliente reclama.
pub const JITTER_QUE_ATRAPALHA: f64 = 30.0;

/// A partir de quanta perda de pacote a partida sente.
///
/// Dois por cento. Abaixo disso o jogo reconstrói o que faltou sem que ninguém
/// perceba; acima, começa a aparecer como engasgo de movimento.
pub const PERDA_QUE_ATRAPALHA: f64 = 2.0;

/// Acima desta idade a leitura vira hipótese, nunca causa.
///
/// Cinco segundos é mais que o intervalo do painel: o que passa disso não
/// descreve o instante que a tela está mostrando.
pub const IDADE_DE_CAUSA_MS: u64 = 5_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Classe {
    /// Todos os núcleos no limite.
    CpuTodosNucleos,
    /// Um núcleo no talo e os outros sobrando. O caso clássico de jogo.
    CpuUmNucleo,
    Gpu,
    MemoriaRam,
    MemoriaVideo,
    Disco,
    /// O firmware está segurando o processador por temperatura.
    LimiteTermico,
    /// O firmware está segurando o processador por energia.
    LimiteEletrico,
    /// O jogo está entregando exatamente a taxa do monitor.
    TetoDeQuadros,
    /// Quadros muito acima do normal daquela partida, com frequência.
    ///
    /// A contagem é medida; a causa NÃO está nesta classe. Shader compilando,
    /// asset chegando do disco e disputa de memória dão o mesmo sintoma.
    Engasgo,
    /// Processador e placa SOBRANDO os dois, durante a partida.
    ///
    /// O limite não está no hardware. Pode ser o motor do jogo, um teto de
    /// quadros, ou uma espera que nenhum dos dois contadores mostra — e é por
    /// isso que a classe diz onde o limite NÃO está, em vez de nomear a causa.
    ForaDoHardware,
    /// Os trancos caem quando o disco está ocupado.
    ///
    /// O jogo esperando o disco entregar conteúdo. Separado do engasgo genérico
    /// porque aqui há uma evidência a mais: o INSTANTE de cada tranco bateu com
    /// atividade de disco, e não com o disco quieto.
    StreamingDeAssets,
    /// A conexão está instável ou perdendo pacote.
    ///
    /// LATÊNCIA ALTA NÃO ENTRA AQUI. Ping é distância: um servidor do outro
    /// lado do mundo responde em 200 ms porque a luz leva esse tempo, e
    /// nenhum ajuste no PC muda isso — `windows::network` abre dizendo
    /// exatamente isso. O que estraga a partida e TEM conserto é a variação e
    /// a perda.
    Rede,
}

impl Classe {
    /// Nenhum plano de energia, ajuste de registro ou perfil resolve.
    ///
    /// Marca as classes em que a resposta honesta é sobre hardware ou
    /// refrigeração — e onde vender otimização seria vender fumaça.
    pub fn software_nao_resolve(self) -> bool {
        matches!(self, Classe::LimiteTermico | Classe::LimiteEletrico)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Forca {
    /// O próprio sistema afirmou o fato, agora, por medição direta.
    Causa,
    /// O número é real, mas é indireto, agregado ou de alguns segundos atrás.
    Hipotese,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Achado {
    pub classe: Classe,
    pub forca: Forca,
    /// O número que sustenta o achado, em texto, com o id da métrica.
    pub evidencia: String,
    /// Idade da leitura que sustentou o achado, quando não é desta coleta.
    pub idade_ms: Option<u64>,
}

/// Uma classe que o produto se propõe a detectar e não pôde avaliar agora.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NaoVerificado {
    /// Nome da classe, na língua do cliente.
    pub classe: String,
    /// O que faltou para poder olhar.
    pub falta: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Conclusao {
    /// Nada foi medido: não há o que classificar.
    SemEvidencia,
    /// A máquina está parada.
    SemCarga,
    /// Há carga e nada encostou no limite.
    NadaNoLimite,
    /// Pelo menos um recurso no limite.
    Encontrado,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Diagnostico {
    pub conclusao: Conclusao,
    /// Causas primeiro, hipóteses depois.
    pub achados: Vec<Achado>,
    pub nao_verificado: Vec<NaoVerificado>,
    /// Quantas das classes que o produto promete detectar puderam ser
    /// avaliadas nesta leitura, e quantas existem.
    ///
    /// É o número que impede o diagnóstico de parecer completo quando não é.
    pub classes_avaliadas: usize,
    pub classes_totais: usize,
}

/// Todas as classes de gargalo que o produto se propõe a detectar.
///
/// A lista é a do prompt do produto, e não a do que o código alcança hoje. É
/// dela que sai o denominador de `classes_avaliadas`: a distância entre o
/// prometido e o verificável precisa ser visível, e não some junto com as
/// classes que ninguém implementou.
const CLASSES_PROMETIDAS: usize = 14;

/// Classifica o que houver na telemetria.
pub fn classificar(t: &Telemetry) -> Diagnostico {
    let mut achados = Vec::new();
    let mut nao_verificado = Vec::new();
    let mut avaliadas = 0usize;

    // ---- limites de firmware
    //
    // Vêm primeiro porque são os únicos em que o sistema AFIRMA o fato em vez
    // de deixar deduzir, e porque são os únicos que nenhum ajuste resolve.
    // O bloco existe para encerrar os empréstimos do fecho antes do resto da
    // função voltar a mexer nas mesmas listas.
    {
        let mut limite = |id: &str, classe: Classe, nome: &str| match leitura(t, id) {
            Some(l) => {
                avaliadas += 1;
                if l.valor >= 0.5 {
                    achados.push(Achado {
                        classe,
                        forca: Forca::Causa,
                        evidencia: format!("{id}: o Windows reporta a flag ligada"),
                        idade_ms: l.idade_ms,
                    });
                }
            }
            None => nao_verificado.push(NaoVerificado {
                classe: nome.to_string(),
                falta: format!("{id} não foi medido"),
            }),
        };

        limite(
            "cpu.throttling.thermal",
            Classe::LimiteTermico,
            "Limite térmico",
        );
        limite(
            "cpu.throttling.power",
            Classe::LimiteEletrico,
            "Limite elétrico",
        );
    }

    // ---- processador
    let uso_cpu = leitura(t, "cpu.usage.overall");
    let pior_nucleo = pior_nucleo(t);

    match uso_cpu {
        Some(l) => {
            avaliadas += 1;

            if l.valor >= SATURADO {
                achados.push(Achado {
                    classe: Classe::CpuTodosNucleos,
                    forca: l.forca(),
                    evidencia: format!("cpu.usage.overall: {:.0}%", l.valor),
                    idade_ms: l.idade_ms,
                });
            }
        }
        None => nao_verificado.push(NaoVerificado {
            classe: "Processador saturado".to_string(),
            falta: "cpu.usage.overall não foi medido".to_string(),
        }),
    }

    // UM núcleo no talo com a média folgada. Só dá para dizer isso com as duas
    // leituras: a média sozinha esconde o caso, e o pico sozinho não prova que
    // os outros estão sobrando.
    match (uso_cpu, pior_nucleo) {
        (Some(media), Some((indice, pico))) => {
            avaliadas += 1;
            if pico >= SATURADO && media.valor <= FOLGADO {
                achados.push(Achado {
                    classe: Classe::CpuUmNucleo,
                    forca: Forca::Causa,
                    evidencia: format!(
                        "cpu.core.{indice}.usage: {pico:.0}% com a média em {:.0}%",
                        media.valor
                    ),
                    idade_ms: None,
                });
            }
        }
        _ => nao_verificado.push(NaoVerificado {
            classe: "Um núcleo no limite".to_string(),
            falta: "falta o uso por núcleo ou a média".to_string(),
        }),
    }

    // ---- os quatro recursos com um número só
    for (id, classe, nome, teto) in [
        ("gpu.usage", Classe::Gpu, "Placa de vídeo", SATURADO),
        (
            "ram.usage",
            Classe::MemoriaRam,
            "Memória do sistema",
            MEMORIA_APERTADA,
        ),
        (
            "vram.usage",
            Classe::MemoriaVideo,
            "Memória de vídeo",
            MEMORIA_APERTADA,
        ),
        ("storage.busy", Classe::Disco, "Disco", SATURADO),
    ] {
        match leitura(t, id) {
            Some(l) => {
                avaliadas += 1;
                if l.valor >= teto {
                    achados.push(Achado {
                        classe,
                        forca: l.forca(),
                        evidencia: format!("{id}: {:.0}%", l.valor),
                        idade_ms: l.idade_ms,
                    });
                }
            }
            None => nao_verificado.push(NaoVerificado {
                classe: nome.to_string(),
                falta: format!("{id} não foi medido"),
            }),
        }
    }

    // ---- teto de quadros
    //
    // O jogo entregando exatamente a taxa do monitor é o sinal de V-Sync, de
    // limite dentro do próprio jogo ou de limitador do driver. Vale como
    // hipótese e nunca como causa, por uma razão que precisa ficar DITA na
    // evidência: os dois números são de momentos diferentes — o FPS é da
    // partida, a taxa do monitor é de agora. Eles só podem ser comparados
    // porque a taxa do monitor não muda sozinha.
    match (t.value("fps.average"), t.value("display.refresh")) {
        (Some(fps), Some(hz)) if hz > 0.0 => {
            avaliadas += 1;

            if (fps - hz).abs() / hz <= 0.02 {
                achados.push(Achado {
                    classe: Classe::TetoDeQuadros,
                    forca: Forca::Hipotese,
                    evidencia: format!(
                        "fps.average: {fps:.0} contra display.refresh: {hz:.0} Hz, \
                         medidos em momentos diferentes"
                    ),
                    idade_ms: t.get("fps.average").and_then(|m| m.age_ms),
                });
            }
        }
        _ => nao_verificado.push(NaoVerificado {
            classe: "Teto de quadros".to_string(),
            falta: "exige fps.average e display.refresh".to_string(),
        }),
    }

    // ---- engasgo
    //
    // A CONTAGEM é medida. A CAUSA não: shader compilando, asset chegando do
    // disco e disputa de memória produzem o mesmo sintoma, e separá-los exige
    // a série de frametime junto da atividade de disco na MESMA janela. O
    // achado diz o que foi visto e para aí — apontar "shader" sem essa
    // correlação seria escolher a causa mais vendável.
    match leitura(t, "frametime.stutters_per_minute") {
        Some(l) => {
            avaliadas += 1;
            if l.valor >= ENGASGOS_POR_MINUTO {
                achados.push(Achado {
                    classe: Classe::Engasgo,
                    forca: Forca::Hipotese,
                    evidencia: format!(
                        "frametime.stutters_per_minute: {:.0} por minuto; a causa não é \
                         separável sem a série de frametime",
                        l.valor
                    ),
                    idade_ms: l.idade_ms,
                });
            }
        }
        None => nao_verificado.push(NaoVerificado {
            classe: "Engasgo".to_string(),
            falta: "exige uma medição de quadros durante a partida".to_string(),
        }),
    }

    // ---- streaming de assets
    //
    // A proporção já vem do cruzamento entre o instante de cada tranco e a
    // atividade de disco na mesma janela (`frames::trancos_com_disco`). Aqui
    // só se lê o resultado e se aplica o corte.
    //
    // O CORTE PARA CIMA APONTA; O CORTE PARA BAIXO NÃO APONTA O CONTRÁRIO.
    // Trancos em sua maioria com o disco ocupado é indício de o jogo estar
    // esperando o disco. Trancos com o disco quieto descartam o disco — e só
    // isso: shader compilando, disputa de memória e simulação pesada continuam
    // todos possíveis, e escolher um deles seria escolher o mais vendável.
    match leitura(t, "frametime.stutters_with_disk") {
        Some(l) => {
            avaliadas += 1;
            if l.valor >= TRANCOS_COM_DISCO_PCT {
                achados.push(Achado {
                    classe: Classe::StreamingDeAssets,
                    forca: Forca::Hipotese,
                    evidencia: format!(
                        "frametime.stutters_with_disk: {:.0}% dos trancos caíram com o \
                         disco ocupado",
                        l.valor
                    ),
                    idade_ms: l.idade_ms,
                });
            }
        }
        None => nao_verificado.push(NaoVerificado {
            classe: "Streaming de assets".to_string(),
            falta: "exige o instante de cada tranco cruzado com a atividade de disco".to_string(),
        }),
    }

    // ---- limite fora do hardware
    //
    // Os dois usos são da MESMA janela em que os quadros foram contados, e é
    // só por isso que a conclusão se sustenta: processador e placa sobrando
    // AGORA não diriam nada sobre uma partida que já acabou.
    //
    // A classe diz onde o limite NÃO está. Nomear a causa — motor do jogo,
    // teto de quadros, espera de memória — exigiria evidência que estes dois
    // números não trazem, e escolher uma delas seria escolher a mais vendável.
    match (t.value("match.cpu_usage"), t.value("match.gpu_usage")) {
        (Some(cpu), Some(gpu)) => {
            avaliadas += 1;

            if cpu <= FOLGADO && gpu <= FOLGADO {
                achados.push(Achado {
                    classe: Classe::ForaDoHardware,
                    forca: Forca::Hipotese,
                    evidencia: format!(
                        "match.cpu_usage: {cpu:.0}% e match.gpu_usage: {gpu:.0}% na mesma \
                         janela dos quadros"
                    ),
                    idade_ms: t.get("match.cpu_usage").and_then(|m| m.age_ms),
                });
            }
        }
        _ => nao_verificado.push(NaoVerificado {
            classe: "Limite fora do hardware".to_string(),
            falta: "exige o uso de processador e placa na janela da partida".to_string(),
        }),
    }

    // ---- rede
    //
    // Jitter e perda, e NÃO latência. A latência é medida e publicada, mas não
    // vira achado: ping é distância, e apontá-lo como gargalo mandaria o
    // cliente procurar conserto para a velocidade da luz.
    //
    // A classe é avaliada quando QUALQUER um dos dois existe. Perda que a
    // sonda não conseguiu determinar — servidor que filtra ICMP — chega como
    // ausente, e aí só o jitter responde.
    let jitter = leitura(t, "network.jitter");
    let perda = leitura(t, "network.packet_loss");

    match (jitter, perda) {
        (None, None) => nao_verificado.push(NaoVerificado {
            classe: "Limitado pela rede".to_string(),
            falta: "exige jitter ou perda medidos contra o servidor do jogo".to_string(),
        }),
        _ => {
            avaliadas += 1;

            let ruim = jitter.filter(|l| l.valor >= JITTER_QUE_ATRAPALHA);
            let com_perda = perda.filter(|l| l.valor >= PERDA_QUE_ATRAPALHA);

            if let Some(l) = com_perda.or(ruim) {
                let texto = match (com_perda, ruim) {
                    (Some(p), Some(j)) => format!(
                        "network.packet_loss: {:.1}% com network.jitter: {:.0} ms",
                        p.valor, j.valor
                    ),
                    (Some(p), None) => format!("network.packet_loss: {:.1}%", p.valor),
                    (None, Some(j)) => format!("network.jitter: {:.0} ms", j.valor),
                    (None, None) => unreachable!("um dos dois existe"),
                };

                achados.push(Achado {
                    classe: Classe::Rede,
                    forca: Forca::Hipotese,
                    evidencia: texto,
                    idade_ms: l.idade_ms,
                });
            }
        }
    }

    // ---- o que este classificador ainda não alcança
    //
    // Cinco classes dependem de correlação temporal, de histórico de driver ou
    // de sondagem que o painel não faz. Elas não somem da resposta: aparecem
    // aqui dizendo o que falta, para que ninguém leia "não achei gargalo" como
    // "olhei tudo".
    for (nome, falta) in [
        // A distribuição já é guardada — média, P95 e P99. O que ainda falta
        // para separar shader de streaming é o EIXO DO TEMPO: saber em que
        // instante cada tranco caiu, e o que o disco estava fazendo naquele
        // instante. Sem isso os dois continuam sendo o mesmo sintoma.
        (
            "Engasgo de shader",
            "descartar o disco não prova shader: memória e simulação dão o mesmo buraco",
        ),
        (
            "Problema de driver",
            "exige comparar quadros com a versão anterior do driver",
        ),
        (
            "Limite do motor do jogo",
            "exige distinguir motor, teto e espera: os contadores de uso não separam os três",
        ),
        (
            "Placa híbrida em notebook",
            "exige o caminho de apresentação do jogo",
        ),
    ] {
        nao_verificado.push(NaoVerificado {
            classe: nome.to_string(),
            falta: falta.to_string(),
        });
    }

    // Causa antes de hipótese, e dentro de cada uma o que o software não
    // resolve primeiro: é a informação que muda a decisão do cliente.
    achados.sort_by_key(|a| (a.forca == Forca::Hipotese, !a.classe.software_nao_resolve()));

    let conclusao = concluir(t, &achados, avaliadas);

    Diagnostico {
        conclusao,
        achados,
        nao_verificado,
        classes_avaliadas: avaliadas,
        classes_totais: CLASSES_PROMETIDAS,
    }
}

fn concluir(t: &Telemetry, achados: &[Achado], avaliadas: usize) -> Conclusao {
    if avaliadas == 0 {
        return Conclusao::SemEvidencia;
    }
    if !achados.is_empty() {
        return Conclusao::Encontrado;
    }

    // Sem carga em lugar nenhum não há gargalo a encontrar — e é diferente de
    // "procurei e não achei". Um limite de firmware ligado é carga por si só:
    // o firmware não segura máquina parada.
    let cargas = ["cpu.usage.overall", "gpu.usage", "storage.busy"];
    let vistas: Vec<f64> = cargas.iter().filter_map(|id| t.value(id)).collect();

    if !vistas.is_empty() && vistas.iter().all(|v| *v < CARGA_MINIMA) {
        return Conclusao::SemCarga;
    }

    Conclusao::NadaNoLimite
}

/// Um número do contrato, com o que decide o peso dele.
#[derive(Debug, Clone, Copy)]
struct Leitura {
    valor: f64,
    qualidade: Quality,
    idade_ms: Option<u64>,
}

impl Leitura {
    /// Causa exige medição direta e recente.
    ///
    /// ESTIMATED não vira causa por definição: o contrato marca assim
    /// justamente o que é derivado, agregado ou reaproveitado de uma leitura
    /// anterior. E medição recente é o que descreve a tela que o cliente está
    /// olhando agora.
    fn forca(&self) -> Forca {
        let recente = self.idade_ms.is_none_or(|ms| ms <= IDADE_DE_CAUSA_MS);

        if self.qualidade == Quality::Measured && recente {
            Forca::Causa
        } else {
            Forca::Hipotese
        }
    }
}

fn leitura(t: &Telemetry, id: &str) -> Option<Leitura> {
    let m = t.get(id)?;
    Some(Leitura {
        valor: m.value?,
        qualidade: m.quality,
        idade_ms: m.age_ms,
    })
}

/// O núcleo mais carregado, e qual é ele.
fn pior_nucleo(t: &Telemetry) -> Option<(usize, f64)> {
    let mut pior: Option<(usize, f64)> = None;

    for indice in 0..1024 {
        let Some(valor) = t.value(&super::telemetry::id_do_nucleo(indice)) else {
            break;
        };

        if pior.is_none_or(|(_, p)| valor > p) {
            pior = Some((indice, valor));
        }
    }

    pior
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::telemetry::{id_do_nucleo, Metric, Unit};

    fn vazia() -> Telemetry {
        Telemetry::new(0, None)
    }

    fn com(pares: &[(&str, f64)]) -> Telemetry {
        let mut t = vazia();
        for (id, valor) in pares {
            t.set(id, Metric::measured(*valor, unidade(id), "teste"));
        }
        t
    }

    fn unidade(id: &str) -> Unit {
        match id {
            "cpu.throttling.thermal" | "cpu.throttling.power" => Unit::Boolean,
            "frametime.stutters_per_minute" => Unit::Count,
            "network.jitter" | "network.latency" => Unit::Milliseconds,
            "match.cpu_usage" | "match.gpu_usage" | "frametime.stutters_with_disk" => Unit::Percent,
            "display.refresh" => Unit::Hertz,
            "fps.average" | "fps.low_1pct" => Unit::Fps,
            _ => Unit::Percent,
        }
    }

    #[test]
    fn sem_telemetria_nao_ha_diagnostico() {
        let d = classificar(&vazia().finish(0));

        assert_eq!(d.conclusao, Conclusao::SemEvidencia);
        assert!(d.achados.is_empty());
        assert_eq!(d.classes_avaliadas, 0);

        // E o que não foi olhado continua listado. Um diagnóstico vazio que
        // não diz o que deixou de olhar é indistinguível de "está tudo bem".
        assert!(!d.nao_verificado.is_empty());
    }

    #[test]
    fn metrica_ausente_nao_conta_como_folgada() {
        // Só a CPU foi medida, e está tranquila. O diagnóstico NÃO pode dizer
        // que não há gargalo: ninguém olhou placa, memória nem disco.
        let d = classificar(&com(&[("cpu.usage.overall", 20.0)]).finish(0));

        assert!(d.achados.is_empty());
        assert!(d.classes_avaliadas < d.classes_totais);

        let faltando: Vec<&str> = d.nao_verificado.iter().map(|n| n.classe.as_str()).collect();
        assert!(faltando.contains(&"Placa de vídeo"));
        assert!(faltando.contains(&"Memória de vídeo"));
        assert!(faltando.contains(&"Disco"));
    }

    #[test]
    fn flag_de_firmware_e_causa_e_vem_primeiro() {
        let t = com(&[
            ("cpu.throttling.thermal", 1.0),
            ("cpu.usage.overall", 95.0),
            ("gpu.usage", 96.0),
            ("ram.usage", 50.0),
            ("vram.usage", 40.0),
            ("storage.busy", 10.0),
        ]);
        let d = classificar(&t.finish(0));

        assert_eq!(d.conclusao, Conclusao::Encontrado);

        let primeiro = &d.achados[0];
        assert_eq!(primeiro.classe, Classe::LimiteTermico);
        assert_eq!(primeiro.forca, Forca::Causa);
        assert!(primeiro.classe.software_nao_resolve());

        // As outras duas continuam no relatório, atrás.
        let classes: Vec<Classe> = d.achados.iter().map(|a| a.classe).collect();
        assert!(classes.contains(&Classe::CpuTodosNucleos));
        assert!(classes.contains(&Classe::Gpu));
    }

    #[test]
    fn flag_baixa_e_resposta_e_nao_ausencia() {
        let d = classificar(&com(&[("cpu.throttling.thermal", 0.0)]).finish(0));

        // A classe foi AVALIADA — a flag foi lida e estava baixa. Isso é
        // diferente de não ter olhado, e por isso não entra em nao_verificado.
        assert!(d.achados.is_empty());
        let faltando: Vec<&str> = d.nao_verificado.iter().map(|n| n.classe.as_str()).collect();
        assert!(!faltando.contains(&"Limite térmico"));
        assert!(faltando.contains(&"Limite elétrico"));
    }

    #[test]
    fn leitura_velha_vira_hipotese() {
        let mut t = vazia();
        t.set(
            "gpu.usage",
            Metric::measured(97.0, Unit::Percent, "wmi").com_idade(12_000),
        );

        let d = classificar(&t.finish(0));
        let gpu = d
            .achados
            .iter()
            .find(|a| a.classe == Classe::Gpu)
            .expect("achado");

        assert_eq!(
            gpu.forca,
            Forca::Hipotese,
            "12 s atrás não é o instante da tela"
        );
        assert_eq!(gpu.idade_ms, Some(12_000));

        // A mesma leitura, agora, é causa.
        let mut fresca = vazia();
        fresca.set("gpu.usage", Metric::measured(97.0, Unit::Percent, "wmi"));
        let d = classificar(&fresca.finish(0));
        assert_eq!(d.achados[0].forca, Forca::Causa);
    }

    #[test]
    fn um_nucleo_no_talo_com_media_folgada() {
        // O caso do FiveM: um núcleo a 99%, sete quase parados, média em 20%.
        // A média sozinha diz "sobra máquina" e manda o cliente comprar a peça
        // errada.
        let mut t = vazia();
        t.set(
            "cpu.usage.overall",
            Metric::measured(20.0, Unit::Percent, "teste"),
        );
        t.set_series(
            id_do_nucleo(0),
            Metric::measured(8.0, Unit::Percent, "teste"),
        );
        t.set_series(
            id_do_nucleo(1),
            Metric::measured(99.0, Unit::Percent, "teste"),
        );
        t.set_series(
            id_do_nucleo(2),
            Metric::measured(6.0, Unit::Percent, "teste"),
        );

        let d = classificar(&t.finish(0));
        let achado = d
            .achados
            .iter()
            .find(|a| a.classe == Classe::CpuUmNucleo)
            .expect("um núcleo no limite");

        assert_eq!(achado.forca, Forca::Causa);
        assert!(
            achado.evidencia.contains("cpu.core.1.usage"),
            "{}",
            achado.evidencia
        );
        assert!(achado.evidencia.contains("99"));

        // E não pode virar "todos os núcleos": a média está em 20%.
        assert!(!d
            .achados
            .iter()
            .any(|a| a.classe == Classe::CpuTodosNucleos));
    }

    #[test]
    fn maquina_parada_nao_tem_gargalo() {
        let d = classificar(
            &com(&[
                ("cpu.usage.overall", 3.0),
                ("gpu.usage", 1.0),
                ("storage.busy", 0.0),
                ("ram.usage", 40.0),
                ("vram.usage", 10.0),
            ])
            .finish(0),
        );

        assert_eq!(d.conclusao, Conclusao::SemCarga);
        assert!(d.achados.is_empty());
    }

    #[test]
    fn com_carga_e_sem_limite_e_outra_resposta() {
        let d = classificar(
            &com(&[
                ("cpu.usage.overall", 55.0),
                ("gpu.usage", 70.0),
                ("storage.busy", 20.0),
                ("ram.usage", 60.0),
                ("vram.usage", 50.0),
            ])
            .finish(0),
        );

        // "Há carga e nada no limite" e "a máquina está parada" são vereditos
        // diferentes, e o cliente faz coisas diferentes com cada um.
        assert_eq!(d.conclusao, Conclusao::NadaNoLimite);
    }

    #[test]
    fn memoria_aperta_antes_dos_noventa_e_dois() {
        // 91% de VRAM não passa do teto do processador, mas já é despejo de
        // textura. Os dois recursos não têm o mesmo limiar.
        let d = classificar(&com(&[("vram.usage", 91.0)]).finish(0));
        assert!(d.achados.iter().any(|a| a.classe == Classe::MemoriaVideo));

        let d = classificar(&com(&[("gpu.usage", 91.0)]).finish(0));
        assert!(d.achados.is_empty(), "91% de placa ainda tem folga");
    }

    #[test]
    fn fps_colado_na_taxa_do_monitor_e_teto_de_quadros() {
        let mut t = vazia();
        t.set(
            "display.refresh",
            Metric::measured(60.0, Unit::Hertz, "win32"),
        );
        t.set(
            "fps.average",
            Metric::estimated(59.8, Unit::Fps, "etw", "partida").com_idade(300_000),
        );

        let d = classificar(&t.finish(0));
        let achado = d
            .achados
            .iter()
            .find(|a| a.classe == Classe::TetoDeQuadros)
            .expect("teto de quadros");

        // Nunca causa: os dois números são de momentos diferentes, e a
        // evidência precisa dizer isso na cara.
        assert_eq!(achado.forca, Forca::Hipotese);
        assert!(achado.evidencia.contains("momentos diferentes"));

        // 42 quadros num monitor de 60 Hz não é teto nenhum.
        let mut longe = vazia();
        longe.set(
            "display.refresh",
            Metric::measured(60.0, Unit::Hertz, "win32"),
        );
        longe.set(
            "fps.average",
            Metric::estimated(42.0, Unit::Fps, "etw", "partida"),
        );
        let d = classificar(&longe.finish(0));
        assert!(!d.achados.iter().any(|a| a.classe == Classe::TetoDeQuadros));
    }

    #[test]
    fn engasgo_e_contado_mas_a_causa_nao_e_escolhida() {
        let mut t = vazia();
        t.set(
            "frametime.stutters_per_minute",
            Metric::estimated(14.0, Unit::Count, "etw", "partida").com_idade(120_000),
        );

        let d = classificar(&t.finish(0));
        let achado = d
            .achados
            .iter()
            .find(|a| a.classe == Classe::Engasgo)
            .expect("engasgo");

        assert_eq!(achado.forca, Forca::Hipotese);
        assert!(achado.evidencia.contains("não é separável"));

        // E as duas causas possíveis continuam declaradas como não olhadas.
        let faltando: Vec<&str> = d.nao_verificado.iter().map(|n| n.classe.as_str()).collect();
        assert!(faltando.contains(&"Engasgo de shader"));
        assert!(faltando.contains(&"Streaming de assets"));
    }

    #[test]
    fn tranco_isolado_nao_vira_diagnostico() {
        let d = classificar(&com(&[("frametime.stutters_per_minute", 2.0)]).finish(0));

        assert!(
            d.achados.is_empty(),
            "dois por minuto não é o que o cliente sente"
        );
        // Mas a classe FOI avaliada: a contagem existe e estava baixa.
        let faltando: Vec<&str> = d.nao_verificado.iter().map(|n| n.classe.as_str()).collect();
        assert!(!faltando.contains(&"Engasgo"));
    }

    #[test]
    fn hardware_sobrando_na_partida_aponta_para_fora_dele() {
        let mut t = vazia();
        t.set(
            "match.cpu_usage",
            Metric::estimated(31.0, Unit::Percent, "pdh", "partida").com_idade(180_000),
        );
        t.set(
            "match.gpu_usage",
            Metric::estimated(44.0, Unit::Percent, "pdh", "partida").com_idade(180_000),
        );

        let d = classificar(&t.finish(0));
        let achado = d
            .achados
            .iter()
            .find(|a| a.classe == Classe::ForaDoHardware)
            .expect("limite fora do hardware");

        assert_eq!(achado.forca, Forca::Hipotese);
        assert!(
            achado.evidencia.contains("mesma janela"),
            "{}",
            achado.evidencia
        );
    }

    #[test]
    fn um_dos_dois_no_limite_nao_e_limite_fora_do_hardware() {
        // Placa a 95% durante a partida é gargalo DE hardware. A classe só
        // existe quando os DOIS sobram.
        let mut t = vazia();
        t.set(
            "match.cpu_usage",
            Metric::estimated(30.0, Unit::Percent, "pdh", "partida"),
        );
        t.set(
            "match.gpu_usage",
            Metric::estimated(95.0, Unit::Percent, "pdh", "partida"),
        );

        let d = classificar(&t.finish(0));
        assert!(!d.achados.iter().any(|a| a.classe == Classe::ForaDoHardware));
    }

    #[test]
    fn um_lado_so_nao_autoriza_a_conclusao() {
        // Só o processador medido na janela. Não dá para dizer nada sobre o
        // par, e a classe volta para a lista do que não foi verificado.
        let mut t = vazia();
        t.set(
            "match.cpu_usage",
            Metric::estimated(20.0, Unit::Percent, "pdh", "partida"),
        );

        let d = classificar(&t.finish(0));
        assert!(!d.achados.iter().any(|a| a.classe == Classe::ForaDoHardware));

        let faltando: Vec<&str> = d.nao_verificado.iter().map(|n| n.classe.as_str()).collect();
        assert!(faltando.contains(&"Limite fora do hardware"));
    }

    #[test]
    fn trancos_colados_no_disco_apontam_streaming() {
        let mut t = vazia();
        t.set(
            "frametime.stutters_with_disk",
            Metric::estimated(84.0, Unit::Percent, "etw+pdh", "42 trancos").com_idade(240_000),
        );

        let d = classificar(&t.finish(0));
        let achado = d
            .achados
            .iter()
            .find(|a| a.classe == Classe::StreamingDeAssets)
            .expect("streaming de assets");

        assert_eq!(achado.forca, Forca::Hipotese, "coincidência não é causa");
        assert!(achado.evidencia.contains("84"));
    }

    #[test]
    fn disco_quieto_descarta_o_disco_e_nao_prova_shader() {
        // 10% dos trancos com o disco ocupado: o disco está fora. Mas isso NÃO
        // aponta shader — memória e simulação pesada dão o mesmo buraco, e
        // escolher uma delas seria escolher a mais vendável.
        let mut t = vazia();
        t.set(
            "frametime.stutters_with_disk",
            Metric::estimated(10.0, Unit::Percent, "etw+pdh", "30 trancos"),
        );

        let d = classificar(&t.finish(0));
        assert!(!d
            .achados
            .iter()
            .any(|a| a.classe == Classe::StreamingDeAssets));

        let faltando: Vec<&str> = d.nao_verificado.iter().map(|n| n.classe.as_str()).collect();
        assert!(faltando.contains(&"Engasgo de shader"));

        // E "Streaming de assets" NÃO volta para a lista do não verificado: a
        // pergunta foi feita e respondida com não.
        assert!(!faltando.contains(&"Streaming de assets"));
    }

    #[test]
    fn sem_o_cruzamento_a_classe_volta_para_o_nao_verificado() {
        let d = classificar(&vazia().finish(0));
        let faltando: Vec<&str> = d.nao_verificado.iter().map(|n| n.classe.as_str()).collect();
        assert!(faltando.contains(&"Streaming de assets"));
    }

    #[test]
    fn jitter_e_perda_apontam_a_rede_mas_ping_alto_nao() {
        // Ping alto e ESTÁVEL: é distância, não é gargalo. Apontar isso
        // mandaria o cliente procurar conserto para a velocidade da luz.
        let mut estavel = vazia();
        estavel.set(
            "network.latency",
            Metric::estimated(190.0, Unit::Milliseconds, "icmp", "servidor").com_idade(30_000),
        );
        estavel.set(
            "network.jitter",
            Metric::estimated(3.0, Unit::Milliseconds, "icmp", "servidor").com_idade(30_000),
        );

        let d = classificar(&estavel.finish(0));
        assert!(
            !d.achados.iter().any(|a| a.classe == Classe::Rede),
            "190 ms estáveis não são gargalo de rede"
        );

        // O mesmo ping, agora pulando: é isso que produz teletransporte.
        let mut instavel = vazia();
        instavel.set(
            "network.jitter",
            Metric::estimated(55.0, Unit::Milliseconds, "icmp", "servidor").com_idade(30_000),
        );

        let d = classificar(&instavel.finish(0));
        let achado = d
            .achados
            .iter()
            .find(|a| a.classe == Classe::Rede)
            .expect("rede");
        assert_eq!(achado.forca, Forca::Hipotese);
        assert!(achado.evidencia.contains("jitter"));
    }

    #[test]
    fn perda_que_a_sonda_nao_determinou_nao_vira_zero() {
        // Servidor que filtra ICMP: a sonda diz que não sabe, e o contrato
        // entrega a perda ausente. Só o jitter responde, e ele está bom.
        let mut t = vazia();
        t.set(
            "network.jitter",
            Metric::estimated(4.0, Unit::Milliseconds, "icmp", "servidor"),
        );
        t.set(
            "network.packet_loss",
            Metric::unknown(Unit::Percent, "o servidor descarta ping"),
        );

        let d = classificar(&t.finish(0));
        assert!(!d.achados.iter().any(|a| a.classe == Classe::Rede));

        // A classe FOI avaliada — havia jitter. Não volta para o não verificado.
        let faltando: Vec<&str> = d.nao_verificado.iter().map(|n| n.classe.as_str()).collect();
        assert!(!faltando.contains(&"Limitado pela rede"));
    }

    #[test]
    fn sem_sonda_a_rede_fica_por_verificar() {
        let d = classificar(&vazia().finish(0));
        let faltando: Vec<&str> = d.nao_verificado.iter().map(|n| n.classe.as_str()).collect();
        assert!(faltando.contains(&"Limitado pela rede"));
    }

    #[test]
    fn a_cobertura_nunca_finge_estar_completa() {
        // Mesmo com tudo o que o painel consegue medir, sete classes do
        // produto continuam fora de alcance. O denominador é o prometido.
        let d = classificar(
            &com(&[
                ("cpu.throttling.thermal", 0.0),
                ("cpu.throttling.power", 0.0),
                ("cpu.usage.overall", 50.0),
                ("gpu.usage", 50.0),
                ("ram.usage", 50.0),
                ("vram.usage", 50.0),
                ("storage.busy", 50.0),
            ])
            .finish(0),
        );

        assert!(d.classes_avaliadas < d.classes_totais);
        assert!(d.nao_verificado.len() >= 7);
    }
}
