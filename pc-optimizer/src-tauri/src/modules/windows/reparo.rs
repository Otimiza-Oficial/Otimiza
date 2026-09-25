// Descrição das ferramentas de reparo do Windows (programa, argumentos, duração, se cancelar é seguro); quem
// executa e interpreta são outros módulos. Reparo não está no catálogo: não há valor anterior a guardar, e
// desfazer seria recorromper. Ler `health` e `tarefa_longa` já prontos para decidir um argumento ainda é descrever.

use super::health::{FindingSeverity, HealthReport};
use super::tarefa_longa::Desfecho;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ferramenta {
    VerificarArquivos,
    RepararImagem,
    VerificarDisco,
    ConsertarDisco,
    AnalisarWinSxS,
    LimparWinSxS { resetar_base: bool },
    DesmarcarConsertoDoDisco,
}

pub struct Receita {
    pub programa: &'static str,
    pub args: Vec<String>,
    /// Dizer "de 10 a 30 minutos" antes impede o cliente de desistir no meio.
    pub minutos_tipicos: (u32, u32),
    pub cancelar_e_seguro: bool,
    pub aviso: Option<&'static str>,
}

fn args(lista: &[&str]) -> Vec<String> {
    lista.iter().map(|s| s.to_string()).collect()
}

pub fn receita(f: &Ferramenta) -> Receita {
    match f {
        Ferramenta::VerificarArquivos => Receita {
            programa: "sfc",
            args: args(&["/scannow"]),
            minutos_tipicos: (5, 15),
            cancelar_e_seguro: true,
            aviso: None,
        },

        Ferramenta::RepararImagem => Receita {
            programa: "DISM",
            args: args(&[
                "/Online",
                "/Cleanup-Image",
                "/RestoreHealth",
                "/English",
            ]),
            minutos_tipicos: (10, 30),
            cancelar_e_seguro: false,
            aviso: Some(
                "Fica parado em 20% por vários minutos, e isso é normal. \
                 Precisa de internet: os arquivos bons vêm do Windows Update.",
            ),
        },

        Ferramenta::VerificarDisco => Receita {
            programa: "chkdsk",
            args: args(&["C:", "/scan"]),
            minutos_tipicos: (2, 20),
            cancelar_e_seguro: true,
            aviso: None,
        },

        // Não é `chkdsk C: /f`: no volume em uso ele PERGUNTA se agenda ("(S/N)" traduzido), o filho não tem console, e
        // a tela dizia "Terminou" sem nada agendado. `fsutil dirty set` marca o mesmo bit, sem diálogo e com código de
        // saída conferível. Sozinho não basta: com o volume na lista de exclusão de um `chkntfs /X` antigo, o `autochk`
        // o pula para sempre; o executor (`reparo_executar`) roda `receita_reinclusao_do_disco()` ANTES.
        Ferramenta::ConsertarDisco => Receita {
            programa: "fsutil",
            args: args(&["dirty", "set", "C:"]),
            // O clique volta em segundos; os minutos são do conserto na próxima inicialização.
            minutos_tipicos: (10, 60),
            // Agendado não se cancela; a tela oferece `DesmarcarConsertoDoDisco` até o reinício.
            cancelar_e_seguro: false,
            aviso: Some(
                "Este clique volta em segundos — só marca o agendamento. Os \
                 minutos aí em cima são de outra hora: a próxima vez que você \
                 ligar o computador, o conserto roda antes de o Windows abrir, \
                 e a máquina fica indisponível enquanto isso dura. Enquanto \
                 você não reiniciar, dá para desmarcar aqui mesmo.",
            ),
        },

        // A saída de emergência do único reparo que não se cancela. `chkntfs /X` não cancela o próximo boot: põe o
        // volume numa lista de exclusão PERSISTENTE. Por isso o `ConsertarDisco` reinclui antes de marcar.
        Ferramenta::DesmarcarConsertoDoDisco => Receita {
            programa: "chkntfs",
            args: args(&["/X", "C:"]),
            minutos_tipicos: (0, 1),
            cancelar_e_seguro: true,
            aviso: None,
        },

        Ferramenta::AnalisarWinSxS => Receita {
            programa: "DISM",
            args: args(&[
                "/Online",
                "/Cleanup-Image",
                "/AnalyzeComponentStore",
                "/English",
            ]),
            minutos_tipicos: (1, 5),
            cancelar_e_seguro: true,
            aviso: None,
        },

        Ferramenta::LimparWinSxS { resetar_base } => {
            let mut lista = vec![
                "/Online".to_string(),
                "/Cleanup-Image".to_string(),
                "/StartComponentCleanup".to_string(),
                "/English".to_string(),
            ];

            if *resetar_base {
                lista.push("/ResetBase".to_string());
            }

            Receita {
                programa: "DISM",
                args: lista,
                minutos_tipicos: (5, 25),
                // Mexe no WinSxS mesmo sem `/ResetBase`: cortada no meio, pode deixar o componente pela metade.
                cancelar_e_seguro: false,
                aviso: if *resetar_base {
                    Some(
                        "Libera mais espaço, e em troca você perde a capacidade \
                         de desinstalar qualquer atualização já aplicada. Não dá \
                         para voltar atrás depois.",
                    )
                } else {
                    None
                },
            }
        }
    }
}

/// `chkntfs /C C:` desfaz um `/X` de qualquer sessão passada. Passo interno, nunca oferecido na tela. Não é
/// destrutivo: num volume nunca excluído não muda nada, então roda sempre, sem precisar de registro.
pub fn receita_reinclusao_do_disco() -> Receita {
    Receita {
        programa: "chkntfs",
        args: args(&["/C", "C:"]),
        minutos_tipicos: (0, 1),
        cancelar_e_seguro: true,
        aviso: None,
    }
}

/// Pura. Só o código 0 autoriza o `fsutil dirty set`: cancelada, não começou ou código diferente é "não sei".
pub fn reinclusao_deu_certo(desfecho: &Desfecho) -> bool {
    matches!(desfecho, Desfecho::Terminou { codigo: 0 })
}

/// Um valor só (não bandeiras, que admitiriam "não verificou mas achou"): o `/f` só é oferecido DEPOIS de o
/// `/scan` achar algo. Nasce em `SemVerificacao`, que nega por padrão.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EstadoDoDisco {
    #[default]
    SemVerificacao,
    VerificadoSemAchado,
    /// O ÚNICO estado que autoriza o conserto.
    VerificadoComAchado,
    ConsertoAgendado,
}

impl EstadoDoDisco {
    /// Pura. Códigos do `chkdsk`: 0 nenhum erro; 1 e 2 achou; 3 não conseguiu verificar. O 3, o cancelamento e o que
    /// nem começou voltam a `SemVerificacao`, nunca a `VerificadoSemAchado`.
    pub fn apos_execucao(self, ferramenta: &Ferramenta, desfecho: &Desfecho) -> EstadoDoDisco {
        match ferramenta {
            Ferramenta::VerificarDisco => match desfecho {
                Desfecho::Terminou { codigo: 0 } => EstadoDoDisco::VerificadoSemAchado,
                Desfecho::Terminou { codigo: 1 } | Desfecho::Terminou { codigo: 2 } => {
                    EstadoDoDisco::VerificadoComAchado
                }
                _ => EstadoDoDisco::SemVerificacao,
            },

            // Só conta com o `fsutil` confirmando; senão o achado fica de pé e a oferta continua.
            Ferramenta::ConsertarDisco => match desfecho {
                Desfecho::Terminou { codigo: 0 } => EstadoDoDisco::ConsertoAgendado,
                _ => self,
            },

            // O achado foi consumido no agendamento: oferecer de novo exige novo `/scan`.
            Ferramenta::DesmarcarConsertoDoDisco => match desfecho {
                Desfecho::Terminou { codigo: 0 } => EstadoDoDisco::SemVerificacao,
                _ => self,
            },

            _ => self,
        }
    }

    /// A metade da trava que fala de MEDIÇÃO; a da SAÚDE é `consertar_disco_e_permitido`. As duas precisam valer.
    pub fn autoriza_consertar(self) -> bool {
        self == EstadoDoDisco::VerificadoComAchado
    }

    pub fn tem_conserto_agendado(self) -> bool {
        self == EstadoDoDisco::ConsertoAgendado
    }
}

/// Campo privado: um `bool` na assinatura podia vir de `unwrap_or(true)`; só um `HealthReport` real produz isto.
pub struct DiscoSaudavel(bool);

/// O prefixo de todo achado de disco: olhando só `disk_status_*`, um SSD com 97% de vida gasta (ainda `Healthy`)
/// liberava o `chkdsk`. Por prefixo, uma família nova entra sozinha.
const PREFIXO_DE_DISCO: &str = "disk_";

const PREFIXO_DE_ESTADO: &str = "disk_status_";

impl DiscoSaudavel {
    /// RECUSA POR PADRÃO: exige `needs_admin` falso, ao menos um `disk_status_*` (o recibo de que o disco foi lido;
    /// `Unknown` e lista vazia não produzem nenhum) e NENHUM `disk_*` fora de `Ok`. `disk_errors_naosei_*` sai `Ok` e
    /// passa de propósito: a maioria dos SSDs não publica o contador, e a saúde já foi lida.
    pub fn a_partir_do_relatorio(relatorio: &HealthReport) -> DiscoSaudavel {
        if relatorio.needs_admin {
            return DiscoSaudavel(false);
        }

        let achados_de_disco = || {
            relatorio
                .findings
                .iter()
                .filter(|achado| achado.id.starts_with(PREFIXO_DE_DISCO))
        };

        // Cobre relatório vazio, disco `Unknown` e leitura que trouxe desgaste sem chegar ao estado.
        let disco_foi_lido = achados_de_disco().any(|a| a.id.starts_with(PREFIXO_DE_ESTADO));

        if !disco_foi_lido {
            return DiscoSaudavel(false);
        }

        let algum_problema = achados_de_disco().any(|a| a.severity != FindingSeverity::Ok);

        DiscoSaudavel(!algum_problema)
    }
}

/// Num disco em más condições o `chkdsk` reescreve estrutura em setores que já falham.
pub fn consertar_disco_e_permitido(disco: &DiscoSaudavel) -> bool {
    disco.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verificar_disco_roda_sem_reiniciar() {
        // `/scan` roda com o Windows ligado e acha sem consertar, sem prender a pessoa no boot.
        let r = receita(&Ferramenta::VerificarDisco);
        assert!(r.args.iter().any(|a| a == "/scan"), "args: {:?}", r.args);
        assert!(!r.args.iter().any(|a| a == "/f"), "ofereceu /f na verificação");
        assert!(r.cancelar_e_seguro, "o /scan só lê; cancelar tem que ser seguro");
    }

    fn achado(id: &str, severidade: FindingSeverity) -> super::super::health::HealthFinding {
        use super::super::health::FixLocation;

        super::super::health::HealthFinding {
            id: id.to_string(),
            title: "Disco: Teste".to_string(),
            measured: "SSD de 500 GB, estado relatado pelo Windows.".to_string(),
            advice: "achado de teste".to_string(),
            severity: severidade,
            fix_location: if severidade == FindingSeverity::Ok {
                FixLocation::None
            } else {
                FixLocation::Hardware
            },
        }
    }

    fn achado_de_disco(severidade: FindingSeverity) -> super::super::health::HealthFinding {
        achado("disk_status_0", severidade)
    }

    fn permite(findings: Vec<super::super::health::HealthFinding>) -> bool {
        let relatorio = HealthReport {
            findings,
            needs_admin: false,
        };

        consertar_disco_e_permitido(&DiscoSaudavel::a_partir_do_relatorio(&relatorio))
    }

    #[test]
    fn relatorio_sem_permissao_nao_recebe_consertar() {
        let relatorio = HealthReport {
            findings: Vec::new(),
            needs_admin: true,
        };

        let disco = DiscoSaudavel::a_partir_do_relatorio(&relatorio);
        assert!(!consertar_disco_e_permitido(&disco));
    }

    #[test]
    fn disco_reprovado_no_relatorio_nao_recebe_consertar() {
        let relatorio = HealthReport {
            findings: vec![achado_de_disco(FindingSeverity::Critical)],
            needs_admin: false,
        };

        let disco = DiscoSaudavel::a_partir_do_relatorio(&relatorio);
        assert!(!consertar_disco_e_permitido(&disco));
    }

    #[test]
    fn disco_limpo_e_legivel_recebe_consertar() {
        let relatorio = HealthReport {
            findings: vec![achado_de_disco(FindingSeverity::Ok)],
            needs_admin: false,
        };

        let disco = DiscoSaudavel::a_partir_do_relatorio(&relatorio);
        assert!(consertar_disco_e_permitido(&disco));
    }

    #[test]
    fn sem_evidencia_nenhuma_o_disco_nao_recebe_consertar() {
        assert!(!permite(Vec::new()));
    }

    #[test]
    fn estado_que_o_windows_nao_soube_dizer_nao_recebe_consertar() {
        assert!(!permite(vec![
            achado("disk_wear_0", FindingSeverity::Ok),
            achado("disk_hours_0", FindingSeverity::Ok),
        ]));
    }

    #[test]
    fn desgaste_critico_nao_recebe_consertar_mesmo_com_o_windows_dizendo_healthy() {
        assert!(!permite(vec![
            achado("disk_status_0", FindingSeverity::Ok),
            achado("disk_wear_0", FindingSeverity::Critical),
        ]));
    }

    #[test]
    fn erros_acumulados_nao_recebem_consertar() {
        assert!(!permite(vec![
            achado("disk_status_0", FindingSeverity::Ok),
            achado("disk_errors_0", FindingSeverity::Critical),
        ]));
    }

    #[test]
    fn disco_quente_nao_recebe_consertar() {
        assert!(!permite(vec![
            achado("disk_status_0", FindingSeverity::Ok),
            achado("disk_temp_0", FindingSeverity::Important),
        ]));
    }

    #[test]
    fn achado_informativo_de_outra_area_nao_reprova_o_disco() {
        // Achado `Critical` de memória ou energia não fecha a trava do disco.
        assert!(permite(vec![
            achado("disk_status_0", FindingSeverity::Ok),
            achado("disk_hours_0", FindingSeverity::Ok),
            achado("memory_pressure", FindingSeverity::Critical),
        ]));
    }

    #[test]
    fn o_consertar_nasce_sem_autorizacao() {
        assert!(!EstadoDoDisco::default().autoriza_consertar());
        assert!(!EstadoDoDisco::default().tem_conserto_agendado());
    }

    #[test]
    fn scan_limpo_nao_autoriza_o_conserto() {
        let depois = EstadoDoDisco::default().apos_execucao(
            &Ferramenta::VerificarDisco,
            &Desfecho::Terminou { codigo: 0 },
        );

        assert_eq!(depois, EstadoDoDisco::VerificadoSemAchado);
        assert!(!depois.autoriza_consertar());
    }

    #[test]
    fn scan_com_achado_autoriza_o_conserto() {
        for codigo in [1, 2] {
            let depois = EstadoDoDisco::default()
                .apos_execucao(&Ferramenta::VerificarDisco, &Desfecho::Terminou { codigo });

            assert!(
                depois.autoriza_consertar(),
                "codigo {} do /scan nao abriu a oferta",
                codigo
            );
        }
    }

    #[test]
    fn scan_que_nao_conseguiu_verificar_nao_autoriza_nada() {
        for desfecho in [
            Desfecho::Terminou { codigo: 3 },
            Desfecho::Cancelada,
            Desfecho::NaoComecou {
                motivo: "Ja existe uma tarefa em andamento.".into(),
            },
        ] {
            let depois = EstadoDoDisco::VerificadoComAchado
                .apos_execucao(&Ferramenta::VerificarDisco, &desfecho);

            assert_eq!(
                depois,
                EstadoDoDisco::SemVerificacao,
                "desfecho {:?} deixou autorizacao de pe",
                desfecho
            );
        }
    }

    #[test]
    fn o_agendamento_consome_o_achado_e_abre_a_volta_atras() {
        let com_achado = EstadoDoDisco::VerificadoComAchado;

        let agendado = com_achado
            .apos_execucao(&Ferramenta::ConsertarDisco, &Desfecho::Terminou { codigo: 0 });
        assert_eq!(agendado, EstadoDoDisco::ConsertoAgendado);
        assert!(!agendado.autoriza_consertar(), "ofereceu agendar duas vezes");
        assert!(agendado.tem_conserto_agendado());

        let desmarcado = agendado.apos_execucao(
            &Ferramenta::DesmarcarConsertoDoDisco,
            &Desfecho::Terminou { codigo: 0 },
        );
        assert_eq!(desmarcado, EstadoDoDisco::SemVerificacao);
        assert!(!desmarcado.tem_conserto_agendado());
    }

    #[test]
    fn agendamento_que_falhou_nao_e_contado_como_feito() {
        let depois = EstadoDoDisco::VerificadoComAchado
            .apos_execucao(&Ferramenta::ConsertarDisco, &Desfecho::Terminou { codigo: 1 });

        assert_eq!(depois, EstadoDoDisco::VerificadoComAchado);
        assert!(!depois.tem_conserto_agendado());
    }

    #[test]
    fn o_conserto_do_disco_nao_faz_pergunta_a_ninguem() {
        let r = receita(&Ferramenta::ConsertarDisco);
        assert_eq!(r.programa, "fsutil");
        assert!(
            !r.args.iter().any(|a| a == "/f"),
            "voltou a chamar o chkdsk /f, que faz pergunta: {:?}",
            r.args
        );

        let volta = receita(&Ferramenta::DesmarcarConsertoDoDisco);
        assert_eq!(volta.programa, "chkntfs");
        assert!(volta.cancelar_e_seguro);
    }

    #[test]
    fn consertar_disco_diz_de_quem_e_o_tempo() {
        let r = receita(&Ferramenta::ConsertarDisco);
        let aviso = r.aviso.unwrap_or_default().to_lowercase();

        assert!(
            aviso.contains("segundos") && aviso.contains("reinici"),
            "o aviso não diz que ESTE clique volta em segundos e que os \
             minutos são do próximo boot: {}",
            aviso
        );
    }

    #[test]
    fn a_reinclusao_desfaz_exatamente_o_que_o_x_faz() {
        let reinclusao = receita_reinclusao_do_disco();
        let desmarcar = receita(&Ferramenta::DesmarcarConsertoDoDisco);

        assert_eq!(reinclusao.programa, "chkntfs");
        assert_eq!(desmarcar.programa, "chkntfs");
        assert!(reinclusao.args.iter().any(|a| a == "/C"));
        assert!(reinclusao.args.iter().any(|a| a == "C:"));
        assert!(desmarcar.args.iter().any(|a| a == "/X"));

        // Uma exclui, a outra reinclui: confundi-las anularia uma a outra.
        assert_ne!(reinclusao.args, desmarcar.args);

        assert!(reinclusao.cancelar_e_seguro);
    }

    #[test]
    fn conserto_do_disco_nao_marca_sujo_sem_reincluir_antes() {
        // `Receita` carrega um programa só: este teste impede que alguém funda os dois e apague a reinclusão.
        let marcar = receita(&Ferramenta::ConsertarDisco);
        let reinclusao = receita_reinclusao_do_disco();

        assert_eq!(marcar.programa, "fsutil");
        assert_eq!(reinclusao.programa, "chkntfs");
        assert_ne!(marcar.programa, reinclusao.programa);
    }

    #[test]
    fn reinclusao_que_terminou_com_erro_nao_autoriza_marcar_sujo() {
        // Com o `/C` falho e o `fsutil` bem-sucedido, a tela diria "agendado" com o volume fora do boot check.
        for desfecho in [
            Desfecho::Terminou { codigo: 1 },
            Desfecho::Cancelada,
            Desfecho::NaoComecou {
                motivo: "Já existe uma tarefa em andamento.".into(),
            },
        ] {
            assert!(
                !reinclusao_deu_certo(&desfecho),
                "desfecho {:?} autorizou marcar sujo sem reincluir de verdade",
                desfecho
            );
        }
    }

    #[test]
    fn reinclusao_que_terminou_limpa_autoriza_marcar_sujo() {
        assert!(reinclusao_deu_certo(&Desfecho::Terminou { codigo: 0 }));
    }

    #[test]
    fn resetar_base_muda_os_argumentos_e_avisa() {
        let sem = receita(&Ferramenta::LimparWinSxS { resetar_base: false });
        let com = receita(&Ferramenta::LimparWinSxS { resetar_base: true });

        assert!(!sem.args.iter().any(|a| a == "/ResetBase"));
        assert!(com.args.iter().any(|a| a == "/ResetBase"));

        // O cliente perde a capacidade de desinstalar atualizações.
        assert!(com.aviso.is_some(), "/ResetBase saiu sem aviso");
        assert!(sem.aviso.is_none());
    }

    #[test]
    fn cancelar_o_dism_nao_e_de_graca() {
        assert!(!receita(&Ferramenta::RepararImagem).cancelar_e_seguro);
        assert!(receita(&Ferramenta::VerificarArquivos).cancelar_e_seguro);
    }

    #[test]
    fn o_dism_pede_saida_estavel() {
        // As três rodam DISM e as três precisam de `/English`.
        for ferramenta in [
            Ferramenta::RepararImagem,
            Ferramenta::AnalisarWinSxS,
            Ferramenta::LimparWinSxS { resetar_base: false },
        ] {
            let r = receita(&ferramenta);
            assert!(
                r.args.iter().any(|a| a == "/English"),
                "{:?} saiu sem /English: {:?}",
                ferramenta,
                r.args
            );
        }
    }
}
