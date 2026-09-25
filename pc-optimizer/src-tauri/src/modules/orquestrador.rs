// Qual perfil de configuração de jogo o que foi medido autoriza ("nenhum ajuste genérico antes da classificação
// do gargalo"). O valor está nas RECUSAS, onde baixar gráfico é placebo: gargalo no processador (a placa já sobra),
// limite de firmware (físico) e esperando o disco. Teto de quadros (`SemTeto`) é o caso sem custo visual. Não
// aplica nada: devolve um plano, com `exige_baseline` sempre que muda algo.

use serde::{Deserialize, Serialize};

use super::gargalo::{Classe, Conclusao, Diagnostico};
use super::streaming::{Analise as Streaming, Veredito};
use super::vram::{Analise as Vram, Estado};

/// Espelha `windows::configjogo::Perfil` em vez de importar: a decisão precisa ser testável em qualquer sistema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Perfil {
    SemTeto,
    Equilibrado,
    Competitivo,
}

impl Perfil {
    /// O plano devolve o nome que o comando de aplicar espera, sem tradução na tela; o teste
    /// `o_plano_fala_a_lingua_de_quem_aplica` (`commands.rs`) confere os três.
    pub fn nome(self) -> &'static str {
        match self {
            Perfil::SemTeto => "sem_teto",
            Perfil::Equilibrado => "equilibrado",
            Perfil::Competitivo => "competitivo",
        }
    }

    /// Só no teste: o plano escolhe UM, e uma lista pública viraria de novo uma fileira de botões.
    #[cfg(test)]
    pub const TODOS: [Perfil; 3] = [Perfil::SemTeto, Perfil::Equilibrado, Perfil::Competitivo];
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Decisao {
    SemEvidencia,
    NaoResolveAqui,
    Aplicar(Perfil),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Plano {
    pub decisao: Decisao,
    pub porque: Vec<String>,
    /// Sai junto da recomendação: metade do "não adiantou" é um ganho real que não era o esperado.
    pub contra: Vec<String>,
    pub nao_verificado: Vec<String>,
    /// Sempre que o plano muda algo: sem o retrato de antes, não há como responder "melhorou?".
    pub exige_baseline: bool,
}

/// Sem arquivo de configuração (`tem_arquivo`) não há alavanca, e recomendar seria recomendar o impossível.
pub fn planejar(
    gargalo: &Diagnostico,
    vram: &Vram,
    streaming: &Streaming,
    tem_arquivo: bool,
) -> Plano {
    let mut porque = Vec::new();
    let mut contra = Vec::new();

    let nao_verificado: Vec<String> = gargalo
        .nao_verificado
        .iter()
        .map(|n| format!("{}: {}", n.classe, n.falta))
        .collect();

    let vazio = |decisao, porque, contra| Plano {
        decisao,
        porque,
        contra,
        nao_verificado: nao_verificado.clone(),
        exige_baseline: false,
    };

    if !tem_arquivo {
        return vazio(
            Decisao::SemEvidencia,
            vec!["o arquivo de configuração do jogo não foi encontrado nesta máquina".to_string()],
            Vec::new(),
        );
    }

    // Sem medição, nada de mexer em arquivo do cliente.
    match gargalo.conclusao {
        Conclusao::SemEvidencia => {
            return vazio(
                Decisao::SemEvidencia,
                vec![
                    "nenhum recurso foi medido; sem classificação de gargalo não há ajuste a \
                     recomendar"
                        .to_string(),
                ],
                Vec::new(),
            )
        }
        Conclusao::SemCarga => {
            return vazio(
                Decisao::SemEvidencia,
                vec![
                    "a máquina está parada; medir com o jogo aberto é o que diz qual ajuste cabe"
                        .to_string(),
                ],
                Vec::new(),
            )
        }
        Conclusao::NadaNoLimite | Conclusao::Encontrado => {}
    }

    let tem = |c: Classe| gargalo.achados.iter().any(|a| a.classe == c);

    // Antes de tudo: é o único caso em que nem a melhor configuração muda o resultado.
    if tem(Classe::LimiteTermico) || tem(Classe::LimiteEletrico) {
        return vazio(
            Decisao::NaoResolveAqui,
            vec![
                "o Windows está segurando o processador por temperatura ou por energia; o limite \
                 é físico e nenhuma linha de arquivo de configuração muda refrigeração"
                    .to_string(),
            ],
            vec![
                "Enquanto o firmware estiver segurando, qualquer ganho de configuração some \
                 dentro do mesmo limite."
                    .to_string(),
            ],
        );
    }

    if streaming.veredito == Veredito::AssetsDeMidiaLenta {
        return vazio(
            Decisao::NaoResolveAqui,
            vec![
                "os trancos medidos caem junto com atividade de disco, e o disco está entregando \
                 devagar; o jogo está esperando o arquivo chegar"
                    .to_string(),
            ],
            vec![
                "Sombra mais baixa não faz o asset chegar mais rápido: o jogo continua esperando \
                 o mesmo arquivo. Veja o laboratório de streaming."
                    .to_string(),
            ],
        );
    }

    // Antes da recusa do processador: o teto é um número num arquivo, não um limite de hardware.
    if tem(Classe::TetoDeQuadros) {
        porque.push(
            "os quadros estão colados na taxa do monitor, o que é sinal de teto escrito no \
             arquivo do jogo ou de sincronia ligada"
                .to_string(),
        );
        contra.push(
            "Se a sincronia com a tela estiver ligada dentro do jogo, o teto volta: essa é a \
             única parte que o arquivo não controla."
                .to_string(),
        );

        return Plano {
            decisao: Decisao::Aplicar(Perfil::SemTeto),
            porque,
            contra,
            nao_verificado,
            exige_baseline: true,
        };
    }

    if tem(Classe::CpuTodosNucleos) || tem(Classe::CpuUmNucleo) {
        let mut porque_cpu =
            vec!["o processador é que está no limite, e não a placa de vídeo".to_string()];
        if tem(Classe::CpuUmNucleo) {
            porque_cpu.push(
                "um núcleo no talo com os outros sobrando é o retrato de simulação pesada, que \
                 nenhuma configuração gráfica alivia"
                    .to_string(),
            );
        }

        return vazio(
            Decisao::NaoResolveAqui,
            porque_cpu,
            vec![
                "Baixar textura, sombra e reflexo tira trabalho da placa de vídeo. Com a placa já \
                 sobrando, o jogo fica mais feio e o número não sai do lugar."
                    .to_string(),
            ],
        );
    }

    // O único caso em que baixar textura se apoia numa medição direta.
    if vram.estado == Estado::Transbordando {
        porque.push(match vram.derramado_gb {
            Some(gb) => format!(
                "a memória da placa acabou e {gb:.1} GB de textura foram parar na memória do \
                 sistema (vram.shared_used)"
            ),
            None => "a memória da placa está transbordando para a memória do sistema".to_string(),
        });
        contra.push(
            "O ganho aqui é o fim dos trancos ao entrar em área nova, e não um número maior de \
             quadros na média."
                .to_string(),
        );

        return Plano {
            decisao: Decisao::Aplicar(Perfil::Equilibrado),
            porque,
            contra,
            nao_verificado,
            exige_baseline: true,
        };
    }

    if tem(Classe::Gpu) {
        porque.push(
            "a placa de vídeo está no limite medido, e é dela que a configuração gráfica tira \
             trabalho"
                .to_string(),
        );
        contra.push(
            "Quanto isso rende nesta máquina não dá para saber antes de medir. Guarde a linha de \
             base, aplique e compare com repetição."
                .to_string(),
        );

        return Plano {
            decisao: Decisao::Aplicar(Perfil::Competitivo),
            porque,
            contra,
            nao_verificado,
            exige_baseline: true,
        };
    }

    vazio(
        Decisao::NaoResolveAqui,
        vec![
            "há carga e nenhum recurso medido encostou no limite; não há gargalo que uma \
             configuração gráfica resolva"
                .to_string(),
        ],
        vec![
            "Isto não é o mesmo que estar tudo bem: veja o que não foi verificado antes de \
             concluir."
                .to_string(),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::gargalo::{Achado, Forca, NaoVerificado};
    use crate::modules::streaming;
    use crate::modules::vram;

    fn diagnostico(conclusao: Conclusao, classes: &[Classe]) -> Diagnostico {
        Diagnostico {
            conclusao,
            achados: classes
                .iter()
                .map(|c| Achado {
                    classe: *c,
                    forca: Forca::Causa,
                    evidencia: "teste".to_string(),
                    idade_ms: None,
                })
                .collect(),
            nao_verificado: vec![NaoVerificado {
                classe: "Engasgo de shader".to_string(),
                falta: "não há sinal que separe shader de memória".to_string(),
            }],
            classes_avaliadas: 11,
            classes_totais: 14,
        }
    }

    fn vram_de(estado: vram::Estado) -> Vram {
        Vram {
            estado,
            dedicada_pct: None,
            folga_gb: None,
            derramado_gb: (estado == Estado::Transbordando).then_some(1.4),
            piso_gb: None,
            falta: Vec::new(),
            explicacao: String::new(),
            conselho: None,
        }
    }

    fn streaming_de(veredito: streaming::Veredito) -> Streaming {
        Streaming {
            veredito,
            engasgos_por_minuto: None,
            coincidencia_pct: None,
            latencia_ms: None,
            memoria_pct: None,
            midia: None,
            falta: Vec::new(),
            explicacao: String::new(),
            proximo_passo: None,
        }
    }

    fn calmo() -> (Vram, Streaming) {
        (
            vram_de(vram::Estado::Folgada),
            streaming_de(streaming::Veredito::SemEngasgos),
        )
    }

    #[test]
    fn sem_medicao_nao_recomenda_nada() {
        let (v, s) = calmo();
        let p = planejar(&diagnostico(Conclusao::SemEvidencia, &[]), &v, &s, true);

        assert_eq!(p.decisao, Decisao::SemEvidencia);
        assert!(!p.exige_baseline, "nada a aplicar, nada a medir antes");
    }

    #[test]
    fn maquina_parada_nao_autoriza_mexer_no_arquivo() {
        let (v, s) = calmo();
        let p = planejar(&diagnostico(Conclusao::SemCarga, &[]), &v, &s, true);

        assert_eq!(p.decisao, Decisao::SemEvidencia);
    }

    #[test]
    fn sem_o_arquivo_do_jogo_nao_ha_alavanca() {
        let (v, s) = calmo();
        let p = planejar(
            &diagnostico(Conclusao::Encontrado, &[Classe::Gpu]),
            &v,
            &s,
            false,
        );

        assert_eq!(p.decisao, Decisao::SemEvidencia);
    }

    #[test]
    fn gargalo_no_processador_recusa_o_ajuste_grafico() {
        let (v, s) = calmo();
        let p = planejar(
            &diagnostico(Conclusao::Encontrado, &[Classe::CpuUmNucleo]),
            &v,
            &s,
            true,
        );

        assert_eq!(p.decisao, Decisao::NaoResolveAqui);
        assert!(
            p.contra.iter().any(|c| c.contains("mais feio")),
            "{:?}",
            p.contra
        );
    }

    #[test]
    fn limite_de_firmware_ganha_de_tudo() {
        let (v, s) = calmo();
        let p = planejar(
            &diagnostico(
                Conclusao::Encontrado,
                &[Classe::LimiteTermico, Classe::Gpu],
            ),
            &v,
            &s,
            true,
        );

        assert_eq!(p.decisao, Decisao::NaoResolveAqui);
        assert!(
            p.porque.iter().any(|x| x.contains("físico")),
            "{:?}",
            p.porque
        );
    }

    #[test]
    fn esperando_o_disco_nao_se_resolve_com_sombra_mais_baixa() {
        let p = planejar(
            &diagnostico(Conclusao::Encontrado, &[Classe::Gpu]),
            &vram_de(vram::Estado::Folgada),
            &streaming_de(streaming::Veredito::AssetsDeMidiaLenta),
            true,
        );

        assert_eq!(p.decisao, Decisao::NaoResolveAqui);
        assert!(
            p.contra.iter().any(|c| c.contains("streaming")),
            "{:?}",
            p.contra
        );
    }

    #[test]
    fn teto_de_quadros_vale_ate_com_o_processador_no_limite() {
        let (v, s) = calmo();
        let p = planejar(
            &diagnostico(
                Conclusao::Encontrado,
                &[Classe::TetoDeQuadros, Classe::CpuTodosNucleos],
            ),
            &v,
            &s,
            true,
        );

        assert_eq!(p.decisao, Decisao::Aplicar(Perfil::SemTeto));
        assert!(p.exige_baseline);
    }

    #[test]
    fn transbordo_de_memoria_de_video_pede_o_equilibrado() {
        let p = planejar(
            &diagnostico(Conclusao::Encontrado, &[]),
            &vram_de(vram::Estado::Transbordando),
            &streaming_de(streaming::Veredito::SemEngasgos),
            true,
        );

        assert_eq!(p.decisao, Decisao::Aplicar(Perfil::Equilibrado));
        assert!(
            p.porque.iter().any(|x| x.contains("1.4 GB")),
            "{:?}",
            p.porque
        );
        assert!(
            p.contra.iter().any(|c| c.contains("média")),
            "{:?}",
            p.contra
        );
    }

    /// Placa cheia sem transbordo NÃO pede ajuste: é cache, e é normal.
    #[test]
    fn cache_cheio_nao_pede_ajuste_de_textura() {
        let p = planejar(
            &diagnostico(Conclusao::NadaNoLimite, &[]),
            &vram_de(vram::Estado::CacheCheio),
            &streaming_de(streaming::Veredito::SemEngasgos),
            true,
        );

        assert_eq!(p.decisao, Decisao::NaoResolveAqui);
    }

    #[test]
    fn placa_no_limite_e_o_caso_do_ajuste_grafico() {
        let (v, s) = calmo();
        let p = planejar(
            &diagnostico(Conclusao::Encontrado, &[Classe::Gpu]),
            &v,
            &s,
            true,
        );

        assert_eq!(p.decisao, Decisao::Aplicar(Perfil::Competitivo));
        assert!(p.exige_baseline);
        assert!(
            p.contra.iter().any(|c| c.contains("medir")),
            "{:?}",
            p.contra
        );
    }

    #[test]
    fn o_plano_carrega_o_que_nao_foi_verificado() {
        let (v, s) = calmo();
        let p = planejar(
            &diagnostico(Conclusao::Encontrado, &[Classe::Gpu]),
            &v,
            &s,
            true,
        );

        assert!(
            p.nao_verificado.iter().any(|n| n.contains("shader")),
            "{:?}",
            p.nao_verificado
        );
    }
}
