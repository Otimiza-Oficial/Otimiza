// Classifica a telemetria que o painel já coletou (sem medir nada; a sessão de medição é `windows::bottleneck`),
// com a qualidade e a idade de cada número. Métrica ausente não é folgada: vai para `nao_verificado`. Hipótese não
// é causa: flag de firmware ligada agora é causa; placa a 95% de dez segundos atrás é hipótese. Não recomenda
// ajuste.

use serde::{Deserialize, Serialize};

use super::telemetry::{Quality, Telemetry};

/// Os mesmos 92% de `windows::bottleneck`: contador nenhum fica cravado em 100, e 80% ainda tem folga.
pub const SATURADO: f64 = 92.0;

/// Para o contraste: um núcleo a 99% com a placa a 35% é outra história que tudo a 95%.
pub const FOLGADO: f64 = 60.0;

/// Máquina parada não tem gargalo.
pub const CARGA_MINIMA: f64 = 15.0;

/// RAM e VRAM: o sistema pagina e despeja textura muito antes de o contador encostar no topo.
pub const MEMORIA_APERTADA: f64 = 90.0;

/// Um a cada dez segundos: deixa de ser tranco isolado e vira a experiência da partida.
pub const ENGASGOS_POR_MINUTO: f64 = 6.0;

/// Abaixo de 60%, algum tranco cai sobre atividade de disco sem relação nenhuma.
pub const TRANCOS_COM_DISCO_PCT: f64 = 60.0;

/// O quanto o ping PULA: 120 ms estável é jogável; 40 que vira 90 é o teletransporte.
pub const JITTER_QUE_ATRAPALHA: f64 = 30.0;

/// Abaixo de 2% o jogo reconstrói o que faltou sem ninguém perceber.
pub const PERDA_QUE_ATRAPALHA: f64 = 2.0;

/// Mais que o intervalo do painel: não descreve o instante que a tela mostra.
pub const IDADE_DE_CAUSA_MS: u64 = 5_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Classe {
    CpuTodosNucleos,
    CpuUmNucleo,
    Gpu,
    MemoriaRam,
    MemoriaVideo,
    Disco,
    LimiteTermico,
    LimiteEletrico,
    TetoDeQuadros,
    /// A contagem é medida; a causa NÃO está nesta classe.
    Engasgo,
    /// Diz onde o limite NÃO está (motor do jogo, teto, espera): nomear a causa pediria evidência que não há.
    ForaDoHardware,
    /// Evidência a mais que o engasgo genérico: o INSTANTE de cada tranco bateu com o disco ocupado.
    StreamingDeAssets,
    /// LATÊNCIA ALTA NÃO ENTRA: ping é distância. Variação e perda têm conserto.
    Rede,
}

impl Classe {
    /// Classes em que a resposta é hardware ou refrigeração: vender otimização aqui seria vender fumaça.
    pub fn software_nao_resolve(self) -> bool {
        matches!(self, Classe::LimiteTermico | Classe::LimiteEletrico)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Forca {
    Causa,
    Hipotese,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Achado {
    pub classe: Classe,
    pub forca: Forca,
    pub evidencia: String,
    pub idade_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NaoVerificado {
    pub classe: String,
    pub falta: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Conclusao {
    SemEvidencia,
    SemCarga,
    NadaNoLimite,
    Encontrado,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Diagnostico {
    pub conclusao: Conclusao,
    pub achados: Vec<Achado>,
    pub nao_verificado: Vec<NaoVerificado>,
    /// Impede o diagnóstico de parecer completo quando não é.
    pub classes_avaliadas: usize,
    pub classes_totais: usize,
}

/// A lista do prometido, não do alcançado hoje: é o denominador de `classes_avaliadas`.
const CLASSES_PROMETIDAS: usize = 14;

/// A pressão de VRAM depende do piso de derramamento desta máquina, que uma coleta sozinha não carrega.
pub fn classificar_com(t: &Telemetry, vram: &super::vram::Analise) -> Diagnostico {
    let mut achados = Vec::new();
    let mut nao_verificado = Vec::new();
    let mut avaliadas = 0usize;

    // Primeiro: o sistema AFIRMA o fato, e nenhum ajuste resolve. O bloco encerra os empréstimos do fecho antes de
    // o resto voltar às mesmas listas.
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

    // Só com as duas leituras: a média esconde o caso, e o pico sozinho não prova que os outros sobram.
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

    // Fora do laço: `vram.usage` alto é cache cheio, o estado normal. Pressão é derramamento acima do piso
    // (`modules::vram`).
    match vram.estado {
        super::vram::Estado::NaoAvaliado => nao_verificado.push(NaoVerificado {
            classe: "Memória de vídeo".to_string(),
            falta: if vram.falta.is_empty() {
                "a memória de vídeo não foi avaliada".to_string()
            } else {
                vram.falta.join(", ")
            },
        }),
        super::vram::Estado::Transbordando => {
            avaliadas += 1;
            achados.push(Achado {
                classe: Classe::MemoriaVideo,
                // Causa: as duas pontas são leitura direta desta coleta, e o piso é medida desta máquina.
                forca: Forca::Causa,
                evidencia: match vram.derramado_gb {
                    Some(gb) => format!("vram.shared_used: {gb:.1} GB acima do piso da máquina"),
                    None => "a placa está derramando para a memória do sistema".to_string(),
                },
                idade_ms: None,
            });
        }
        _ => avaliadas += 1,
    }

    for (id, classe, nome, teto) in [
        ("gpu.usage", Classe::Gpu, "Placa de vídeo", SATURADO),
        (
            "ram.usage",
            Classe::MemoriaRam,
            "Memória do sistema",
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

    // Hipótese, nunca causa: o FPS é da partida e a taxa do monitor é de agora (só comparáveis porque ela não muda
    // sozinha).
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

    // Separar shader, asset e memória exige frametime e disco na mesma janela: apontar "shader" sem isso seria
    // escolher a causa mais vendável.
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

    // A proporção vem de `frames::trancos_com_disco`. O corte para cima aponta o disco; para baixo só o descarta.
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

    // Só se sustenta porque os dois usos são da MESMA janela dos quadros.
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

    // Jitter e perda, NÃO latência. Perda indeterminada (servidor que filtra ICMP) chega ausente, e só o jitter
    // responde.
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

    // Classes que dependem de correlação temporal, histórico de driver ou sondagem: aparecem dizendo o que falta.
    for (nome, falta) in [
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

    // Causa antes de hipótese; dentro de cada, o que o software não resolve primeiro.
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

    // Parado é diferente de "procurei e não achei". Limite de firmware ligado é carga por si só.
    let cargas = ["cpu.usage.overall", "gpu.usage", "storage.busy"];
    let vistas: Vec<f64> = cargas.iter().filter_map(|id| t.value(id)).collect();

    if !vistas.is_empty() && vistas.iter().all(|v| *v < CARGA_MINIMA) {
        return Conclusao::SemCarga;
    }

    Conclusao::NadaNoLimite
}

#[derive(Debug, Clone, Copy)]
struct Leitura {
    valor: f64,
    qualidade: Quality,
    idade_ms: Option<u64>,
}

impl Leitura {
    /// ESTIMATED não vira causa por definição.
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

    /// Sem piso a classe de VRAM cai em "não verificada": um atalho que a desse por folgada esconderia o engano.
    fn classificar(t: &Telemetry) -> Diagnostico {
        classificar_com(
            t,
            &crate::modules::vram::avaliar(t, &crate::modules::vram::Piso::default()),
        )
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
            "vram.used" | "vram.total" | "vram.shared_used" => Unit::Gigabytes,
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

        assert!(!d.nao_verificado.is_empty());
    }

    #[test]
    fn metrica_ausente_nao_conta_como_folgada() {
        // Só a CPU medida: não pode dizer que não há gargalo.
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

        let classes: Vec<Classe> = d.achados.iter().map(|a| a.classe).collect();
        assert!(classes.contains(&Classe::CpuTodosNucleos));
        assert!(classes.contains(&Classe::Gpu));
    }

    #[test]
    fn flag_baixa_e_resposta_e_nao_ausencia() {
        let d = classificar(&com(&[("cpu.throttling.thermal", 0.0)]).finish(0));

        // Avaliada (a flag foi lida baixa) é diferente de não olhada.
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

        let mut fresca = vazia();
        fresca.set("gpu.usage", Metric::measured(97.0, Unit::Percent, "wmi"));
        let d = classificar(&fresca.finish(0));
        assert_eq!(d.achados[0].forca, Forca::Causa);
    }

    #[test]
    fn um_nucleo_no_talo_com_media_folgada() {
        // O caso do FiveM: um núcleo a 99%, sete quase parados, média 20%.
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

        assert_eq!(d.conclusao, Conclusao::NadaNoLimite);
    }

    #[test]
    fn memoria_do_sistema_aperta_antes_dos_noventa_e_dois() {
        let d = classificar(&com(&[("ram.usage", 91.0)]).finish(0));
        assert!(d.achados.iter().any(|a| a.classe == Classe::MemoriaRam));

        let d = classificar(&com(&[("gpu.usage", 91.0)]).finish(0));
        assert!(d.achados.is_empty(), "91% de placa ainda tem folga");
    }

    #[test]
    fn dedicada_cheia_sozinha_nao_acusa_memoria_de_video() {
        let d = classificar(&com(&[("vram.used", 7.8), ("vram.total", 8.0)]).finish(0));

        assert!(
            !d.achados.iter().any(|a| a.classe == Classe::MemoriaVideo),
            "cache cheio não é gargalo: {:?}",
            d.achados
        );
    }

    #[test]
    fn transbordo_medido_vira_causa_de_memoria_de_video() {
        use crate::modules::vram;

        let t = com(&[
            ("vram.used", 7.8),
            ("vram.total", 8.0),
            ("vram.shared_used", 2.0),
        ])
        .finish(0);

        let mut piso = vram::Piso::default();
        for _ in 0..vram::AMOSTRAS_PARA_PISO {
            piso.observar(Some(0.3), Some(4.0));
        }

        let d = classificar_com(&t, &vram::avaliar(&t, &piso));
        let achado = d
            .achados
            .iter()
            .find(|a| a.classe == Classe::MemoriaVideo)
            .expect("transbordo medido tem de virar achado");

        assert_eq!(achado.forca, Forca::Causa);
        assert!(achado.evidencia.contains("vram.shared_used"));
    }

    #[test]
    fn memoria_de_video_sem_medida_fica_declarada_como_nao_verificada() {
        let d = classificar(&vazia().finish(0));

        assert!(
            d.nao_verificado
                .iter()
                .any(|n| n.classe == "Memória de vídeo"),
            "{:?}",
            d.nao_verificado
        );
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

        assert_eq!(achado.forca, Forca::Hipotese);
        assert!(achado.evidencia.contains("momentos diferentes"));

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
        // O denominador é o prometido.
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
