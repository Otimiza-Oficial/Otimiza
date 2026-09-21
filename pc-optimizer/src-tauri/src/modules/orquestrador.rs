// Orquestrador de renderização: qual ajuste o que foi medido autoriza
//
// A REGRA DO PRODUTO QUE ESTE MÓDULO TORNA EXECUTÁVEL
//
// "Nenhum ajuste genérico antes da classificação do gargalo." Até aqui isso era
// uma frase no cabeçalho de `gargalo.rs`. Na prática, os três perfis de
// configuração de jogo (`windows::configjogo::Perfil`) ficavam lado a lado na
// tela com um botão cada, e quem escolhia era o cliente — que não tem como
// saber qual deles o problema DELE pede.
//
// Este módulo escolhe, e escolhe a partir do que foi medido. Ele não inventa
// nenhuma alavanca nova: as que existem já estão escritas, testadas e com
// caminho de volta. O que ele acrescenta é a decisão — e, principalmente, a
// RECUSA.
//
// AS RECUSAS SÃO A PARTE VALIOSA
//
// Três casos em que baixar configuração gráfica é placebo, e em que um produto
// que só sabe recomendar acabaria recomendando:
//
//   1. GARGALO NO PROCESSADOR. Baixar textura, sombra e reflexo tira trabalho
//      da PLACA. Num jogo preso no processador — que é o caso clássico de
//      simulação pesada, e o do FiveM — a placa já está sobrando, e aliviá-la
//      mais não devolve um quadro sequer. O cliente mexe em tudo, vê o jogo
//      ficar feio e o número não sair do lugar.
//   2. LIMITE DE FIRMWARE. Com o Windows segurando o processador por
//      temperatura ou por energia, o limite é físico. Nenhuma linha de arquivo
//      de configuração muda refrigeração.
//   3. ESPERANDO O DISCO. Tranco de asset chegando devagar não melhora com
//      sombra mais baixa: o jogo continua esperando o mesmo arquivo.
//
// E O CASO EM QUE O AJUSTE É O CERTO E NINGUÉM FALA DELE
//
// Teto de quadros. Um jogo entregando exatamente a taxa do monitor pode estar
// preso num número escrito num arquivo, e tirar aquilo devolve o que a máquina
// já era capaz de fazer — sem custo visual nenhum. É o perfil `SemTeto`, e é o
// único que este módulo recomenda sem pedir nada em troca.
//
// NADA É APLICADO AQUI
//
// O módulo devolve um PLANO. Quem aplica é o caminho que já existe, com diário
// de intenção e desfazer (`transacao`, `changelog`). E o plano sai com
// `exige_baseline` ligado sempre que muda alguma coisa: aplicar sem o retrato
// de antes é abrir mão de poder responder "melhorou?" depois.

use serde::{Deserialize, Serialize};

use super::gargalo::{Classe, Conclusao, Diagnostico};
use super::streaming::{Analise as Streaming, Veredito};
use super::vram::{Analise as Vram, Estado};

/// O perfil de configuração de jogo que o plano pede.
///
/// Espelha `windows::configjogo::Perfil` em vez de importá-lo, pela mesma razão
/// de `streaming::Midia`: a decisão é aritmética sobre diagnósticos e precisa
/// ser testável em qualquer sistema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Perfil {
    /// Só tira os tetos. Não muda como o jogo se parece.
    SemTeto,
    /// Tira os tetos e desliga o que é caro e pouco visível.
    Equilibrado,
    /// Derruba tudo que custa quadro.
    Competitivo,
}

impl Perfil {
    /// O nome que o comando de aplicar espera.
    ///
    /// O plano devolve ESTE texto, e não um enum que a tela teria de traduzir:
    /// uma tradução na tela é uma tabela a mais para sair do lugar sem ninguém
    /// perceber. O teste `o_plano_fala_a_lingua_de_quem_aplica`, em
    /// `commands.rs`, exige que os três nomes continuem sendo aceitos lá.
    pub fn nome(self) -> &'static str {
        match self {
            Perfil::SemTeto => "sem_teto",
            Perfil::Equilibrado => "equilibrado",
            Perfil::Competitivo => "competitivo",
        }
    }

    /// Os três, para a guarda que confere os nomes contra quem aplica.
    ///
    /// Só existe no teste de propósito: no produto ninguém varre os perfis —
    /// o plano escolhe UM. Deixá-la pública fora do teste seria oferecer uma
    /// varredura que não tem uso e que alguém acabaria usando para montar uma
    /// lista de botões, que é exatamente o que este módulo veio substituir.
    #[cfg(test)]
    pub const TODOS: [Perfil; 3] = [Perfil::SemTeto, Perfil::Equilibrado, Perfil::Competitivo];
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Decisao {
    /// Não há medição que autorize mexer em nada.
    SemEvidencia,
    /// O que foi medido diz que o ajuste gráfico não resolve este caso.
    NaoResolveAqui,
    /// Há um perfil que o que foi medido justifica.
    Aplicar(Perfil),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Plano {
    pub decisao: Decisao,
    /// A medida que sustenta a decisão, em português, com o id da métrica.
    pub porque: Vec<String>,
    /// O que este ajuste NÃO vai resolver, mesmo quando é o certo.
    ///
    /// Sai junto da recomendação, e não escondido: metade das reclamações de
    /// "não adiantou nada" vem de um ganho real que não era o que a pessoa
    /// esperava.
    pub contra: Vec<String>,
    /// Classes que ninguém pôde avaliar e que mudariam esta decisão.
    pub nao_verificado: Vec<String>,
    /// Guardar a linha de base antes de aplicar.
    ///
    /// Verdadeiro sempre que o plano muda alguma coisa. Aplicar sem o retrato
    /// de antes é abrir mão de responder "melhorou?" depois — e "melhorou?" é a
    /// única pergunta que o cliente realmente faz.
    pub exige_baseline: bool,
}

/// Decide o que fazer com a configuração do jogo.
///
/// `tem_arquivo` diz se o arquivo de configuração do jogo foi encontrado. Sem
/// ele não há alavanca nenhuma, e recomendar um perfil seria recomendar uma
/// ação impossível.
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

    // ---- sem classificação não há ajuste
    //
    // A regra do produto, aplicada literalmente. Máquina parada ou sem medição
    // não autoriza mexer em arquivo do cliente.
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

    // ---- recusa 1: limite de firmware
    //
    // Vem antes de tudo porque é o único caso em que nem a melhor configuração
    // possível muda o resultado: o limite é físico.
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

    // ---- recusa 2: esperando o disco
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

    // ---- o caso sem custo: teto de quadros
    //
    // Antes das recusas de processador de propósito. Tirar um teto devolve
    // quadros mesmo num jogo preso no processador — o teto é um número escrito
    // num arquivo, e não um limite de hardware.
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

    // ---- recusa 3: gargalo no processador
    //
    // A mais importante das três, porque é a que o mercado erra todo dia.
    // Baixar textura, sombra e reflexo tira trabalho da PLACA; num jogo preso
    // no processador a placa já está sobrando.
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

    // ---- memória de vídeo transbordando: o ajuste certo, pela medida certa
    //
    // O único caso em que baixar textura está apoiado numa medição direta, e
    // não numa suposição sobre o que costuma pesar.
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

    // ---- placa no limite: aqui a configuração gráfica é a alavanca certa
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

    // ---- há carga e nada encostou no limite
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

    /// A regra do produto, aplicada: sem classificação, nenhum ajuste.
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

    /// A recusa que o mercado erra todo dia.
    ///
    /// Jogo preso no processador: baixar configuração gráfica tira trabalho da
    /// placa, que já está sobrando. O cliente mexe em tudo, o jogo fica feio e
    /// o número não sai do lugar.
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
                // Placa no limite TAMBÉM: ainda assim o firmware manda.
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

    /// O teto vale mesmo com o processador no limite: é um número num arquivo,
    /// não um limite de hardware.
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

    /// Transbordo medido é o único caso em que baixar textura está apoiado em
    /// medição direta.
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
        // E o que o ajuste NÃO entrega sai junto.
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
        // Sem promessa de número: o ganho se mede depois.
        assert!(
            p.contra.iter().any(|c| c.contains("medir")),
            "{:?}",
            p.contra
        );
    }

    /// O que não foi verificado acompanha toda decisão, inclusive as boas.
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
