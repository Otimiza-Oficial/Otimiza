// Resultado do `sfc` pelas marcas `[SR]` do CBS.log, que não são traduzidas; a saída do console é, e compará-la
// quebraria em outro idioma.

use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResultadoSfc {
    SemCorrupcao,
    Corrigiu { quantos: usize },
    /// Sobrou corrupção sem conserto: `Corrigiu` esconderia que é preciso reparar a imagem do Windows.
    CorrigiuEmParte { corrigidos: usize, restantes: usize },
    NaoConseguiu { quantos: usize },
    /// Os nomes legíveis foram consertados, mas houve linhas de falha sem nome: não se sabe se sobrou corrupção nelas.
    CorrigiuComRessalva { quantos: usize, linhas_ilegiveis: usize },
    /// NUNCA vira `SemCorrupcao`.
    NaoSei { motivo: String },
}

/// Decidido AQUI, pelo dado: a tela chegou a pintar `CorrigiuEmParte` de verde com `startsWith("Corrigiu ")`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severidade {
    Ok,
    Atencao,
    Erro,
}

impl ResultadoSfc {
    /// Os dois deixaram corrupção; separa-os o progresso (zero corrigidos é `Erro`: reparar a imagem vira o único
    /// caminho).
    pub fn severidade(&self) -> Severidade {
        match self {
            ResultadoSfc::SemCorrupcao | ResultadoSfc::Corrigiu { .. } => Severidade::Ok,
            ResultadoSfc::CorrigiuEmParte { .. } => Severidade::Atencao,
            ResultadoSfc::NaoConseguiu { .. } => Severidade::Erro,
            // "Não sei" é incerteza: tom intermediário, nunca o pior sem prova.
            ResultadoSfc::NaoSei { .. } => Severidade::Atencao,
            // Incerteza também: nunca o `Ok` que a contagem legível sozinha sugeriria.
            ResultadoSfc::CorrigiuComRessalva { .. } => Severidade::Atencao,
        }
    }
}

pub fn caminho_do_log() -> PathBuf {
    let raiz = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
    PathBuf::from(raiz).join("Logs").join("CBS").join("CBS.log")
}

const INICIO_DA_PASSAGEM: &str = "Beginning Verify and Repair transaction";

/// A ÚLTIMA execução: o CBS.log guarda todas, e somar o arquivo dava contagens de execução nenhuma sob o rótulo
/// "a última verificação".
fn ultima_passagem(conteudo: &str) -> Option<&str> {
    conteudo.rfind(INICIO_DA_PASSAGEM).map(|inicio| &conteudo[inicio..])
}

/// `None` sem o par de aspas: inventar nome poderia esconder uma falha ou inflar `restantes` para sempre.
fn nome_do_arquivo_nao_reparado(linha: &str) -> Option<&str> {
    let fim = linha.rfind('\'')?;
    let inicio = linha[..fim].rfind('\'')?;
    let nome = &linha[inicio + 1..fim];
    (!nome.is_empty()).then_some(nome)
}

/// Barra invertida ou não: nada garante o formato em todas as versões.
fn nome_do_arquivo_reparado(linha: &str) -> Option<&str> {
    let nome = linha.trim_end().rsplit(['\\', '/']).next()?;
    (!nome.is_empty()).then_some(nome)
}

pub fn interpretar(conteudo: &str) -> ResultadoSfc {
    let Some(passagem) = ultima_passagem(conteudo) else {
        // Log sem `sfc` e log sem início identificável são causas diferentes; as duas são `NaoSei`.
        let motivo = if conteudo.contains("[SR]") {
            "o registro do Windows não marca onde a última verificação começou"
        } else {
            "o registro do Windows não trouxe nenhuma verificação"
        };

        return ResultadoSfc::NaoSei {
            motivo: motivo.into(),
        };
    };

    let marcas: Vec<&str> = passagem
        .lines()
        .filter(|l| l.contains("[SR]"))
        .collect();

    if marcas.is_empty() {
        return ResultadoSfc::NaoSei {
            motivo: "o registro do Windows não trouxe nenhuma verificação".into(),
        };
    }

    // Guardadas à parte: "há linha de falha?" e "quantos arquivos ela nomeia?" divergem quando o nome não sai.
    let falhas_brutas: Vec<&&str> = marcas
        .iter()
        .filter(|l| l.contains("Cannot repair member file"))
        .collect();

    // Por NOME, não por linha: o `sfc` registra a mesma falha duas vezes antes de consertar por outra fonte.
    let nao_reparados_set: HashSet<&str> = falhas_brutas
        .iter()
        .filter_map(|l| nome_do_arquivo_nao_reparado(l))
        .collect();

    if falhas_brutas.is_empty() {
        return ResultadoSfc::SemCorrupcao;
    }

    // Falha no log sem nenhum nome legível: não é "sem corrupção", é "não consegui nomear". O caso misto segue abaixo.
    if nao_reparados_set.is_empty() {
        return ResultadoSfc::NaoSei {
            motivo: "o registro do Windows mostra arquivos sem reparo, mas nenhuma linha veio no formato esperado para dizer qual".into(),
        };
    }

    let reparados_set: HashSet<&str> = marcas
        .iter()
        .filter(|l| l.contains("Repairing corrupted file"))
        .filter_map(|l| nome_do_arquivo_reparado(l))
        .collect();

    // Interseção: nunca maior que o conjunto, dispensa `saturating_sub`.
    let reparados = nao_reparados_set.intersection(&reparados_set).count();
    let restantes = nao_reparados_set.len() - reparados;

    // Linhas sem nome ficam fora de `restantes`, mas não podem ficar mudas quando a conta fecha em "tudo consertado".
    let linhas_ilegiveis = falhas_brutas
        .iter()
        .filter(|l| nome_do_arquivo_nao_reparado(l).is_none())
        .count();

    if reparados == 0 {
        ResultadoSfc::NaoConseguiu {
            quantos: nao_reparados_set.len(),
        }
    } else if restantes == 0 {
        if linhas_ilegiveis > 0 {
            ResultadoSfc::CorrigiuComRessalva {
                quantos: reparados,
                linhas_ilegiveis,
            }
        } else {
            ResultadoSfc::Corrigiu { quantos: reparados }
        }
    } else {
        ResultadoSfc::CorrigiuEmParte {
            corrigidos: reparados,
            restantes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEM_CORRUPCAO: &str = "\
2026-08-31 11:59:59, Info CSI 00000000 [SR] Beginning Verify and Repair transaction
2026-08-31 12:00:01, Info CSI 00000001 [SR] Verifying 100 (0x00000064) components
2026-08-31 12:00:02, Info CSI 00000002 [SR] Verify complete
2026-08-31 12:00:03, Info CSI 00000003 [SR] Repairing 0 components";

    const CORRIGIU: &str = "\
2026-08-31 11:59:59, Info CSI 00000000 [SR] Beginning Verify and Repair transaction
2026-08-31 12:00:01, Info CSI 00000001 [SR] Cannot repair member file [l:20]'ntdll.dll'
2026-08-31 12:00:02, Info CSI 00000002 [SR] Repairing corrupted file \\??\\C:\\Windows\\ntdll.dll
2026-08-31 12:00:03, Info CSI 00000003 [SR] Repair complete";

    const NAO_CONSEGUIU: &str = "\
2026-08-31 11:59:59, Info CSI 00000000 [SR] Beginning Verify and Repair transaction
2026-08-31 12:00:01, Info CSI 00000001 [SR] Cannot repair member file [l:20]'ntdll.dll'
2026-08-31 12:00:02, Info CSI 00000002 [SR] Cannot repair member file [l:18]'user32.dll'";

    const CORRIGIU_PARCIAL: &str = "\
2026-08-31 11:59:59, Info CSI 00000000 [SR] Beginning Verify and Repair transaction
2026-08-31 12:00:01, Info CSI 00000001 [SR] Cannot repair member file [l:20]'kernel32.dll'
2026-08-31 12:00:02, Info CSI 00000002 [SR] Repairing corrupted file \\??\\C:\\Windows\\kernel32.dll
2026-08-31 12:00:03, Info CSI 00000003 [SR] Cannot repair member file [l:18]'gdi32.dll'
2026-08-31 12:00:04, Info CSI 00000004 [SR] Repairing corrupted file \\??\\C:\\Windows\\gdi32.dll
2026-08-31 12:00:05, Info CSI 00000005 [SR] Cannot repair member file [l:18]'user32.dll'";

    #[test]
    fn log_limpo_e_sem_corrupcao() {
        assert_eq!(interpretar(SEM_CORRUPCAO), ResultadoSfc::SemCorrupcao);
    }

    #[test]
    fn reparo_concluido_e_corrigiu() {
        assert_eq!(interpretar(CORRIGIU), ResultadoSfc::Corrigiu { quantos: 1 });
    }

    #[test]
    fn sem_reparo_concluido_e_nao_conseguiu() {
        assert_eq!(
            interpretar(NAO_CONSEGUIU),
            ResultadoSfc::NaoConseguiu { quantos: 2 }
        );
    }

    #[test]
    fn reparo_misto_nunca_vira_corrigiu_total() {
        assert_eq!(
            interpretar(CORRIGIU_PARCIAL),
            ResultadoSfc::CorrigiuEmParte {
                corrigidos: 2,
                restantes: 1
            }
        );
    }

    #[test]
    fn duas_execucoes_no_mesmo_log_contam_so_a_ultima() {
        let historico = format!("{}\n{}", NAO_CONSEGUIU, CORRIGIU);

        assert_eq!(
            interpretar(&historico),
            ResultadoSfc::Corrigiu { quantos: 1 },
            "somou a execucao de tres meses atras na de agora"
        );
    }

    #[test]
    fn a_passagem_anterior_nao_contamina_a_atual_no_sentido_contrario() {
        // Uma execução limpa antiga não apaga a corrupção deixada pela última.
        let historico = format!("{}\n{}", CORRIGIU, NAO_CONSEGUIU);

        assert_eq!(
            interpretar(&historico),
            ResultadoSfc::NaoConseguiu { quantos: 2 }
        );
    }

    #[test]
    fn log_com_marcas_mas_sem_inicio_de_passagem_e_nao_sei() {
        let orfao = "2026-08-31 12:00:01, Info CSI 00000001 [SR] Verify complete";

        match interpretar(orfao) {
            ResultadoSfc::NaoSei { motivo } => {
                assert!(motivo.contains("começou"), "motivo errado: {}", motivo);
            }
            outro => panic!("esperava NaoSei, veio {:?}", outro),
        }
    }

    #[test]
    fn log_vazio_e_nao_sei_e_nunca_sem_corrupcao() {
        assert!(matches!(interpretar(""), ResultadoSfc::NaoSei { .. }));
    }

    #[test]
    fn corrigiu_em_parte_nunca_recebe_o_tom_de_sucesso() {
        let parcial = ResultadoSfc::CorrigiuEmParte {
            corrigidos: 2,
            restantes: 1,
        };

        assert_ne!(parcial.severidade(), Severidade::Ok);
        assert_eq!(parcial.severidade(), Severidade::Atencao);
    }

    #[test]
    fn nao_conseguiu_e_mais_grave_que_corrigiu_em_parte() {
        assert_eq!(
            ResultadoSfc::NaoConseguiu { quantos: 3 }.severidade(),
            Severidade::Erro
        );
    }

    #[test]
    fn sucesso_total_e_ausencia_de_corrupcao_recebem_o_tom_de_sucesso() {
        assert_eq!(ResultadoSfc::SemCorrupcao.severidade(), Severidade::Ok);
        assert_eq!(
            ResultadoSfc::Corrigiu { quantos: 4 }.severidade(),
            Severidade::Ok
        );
    }

    #[test]
    fn nao_sei_recebe_o_tom_intermediario_e_nao_o_pior() {
        let ns = ResultadoSfc::NaoSei {
            motivo: "teste".into(),
        };

        assert_eq!(ns.severidade(), Severidade::Atencao);
    }

    #[test]
    fn o_mesmo_arquivo_falhando_duas_vezes_conta_uma() {
        let log = "\
2026-09-02 10:00:00, Info CSI 00000001 [SR] Beginning Verify and Repair transaction
2026-09-02 10:00:01, Info CSI 00000002 [SR] Cannot repair member file [l:10]'ntdll.dll'
2026-09-02 10:00:02, Info CSI 00000003 [SR] Cannot repair member file [l:10]'ntdll.dll'
2026-09-02 10:00:03, Info CSI 00000004 [SR] Repairing corrupted file \\??\\C:\\Windows\\ntdll.dll
2026-09-02 10:00:04, Info CSI 00000005 [SR] Repair complete";

        assert_eq!(
            interpretar(log),
            ResultadoSfc::Corrigiu { quantos: 1 },
            "duas tentativas do mesmo arquivo viraram um arquivo ainda quebrado"
        );
    }

    #[test]
    fn linha_sem_nome_extraivel_nao_e_contada_nem_inventada() {
        let log = "\
2026-09-02 10:00:00, Info CSI 00000001 [SR] Beginning Verify and Repair transaction
2026-09-02 10:00:01, Info CSI 00000002 [SR] Cannot repair member file sem aspas nenhuma
2026-09-02 10:00:02, Info CSI 00000003 [SR] Cannot repair member file [l:20]'ntdll.dll'";

        assert_eq!(
            interpretar(log),
            ResultadoSfc::NaoConseguiu { quantos: 1 },
            "linha sem nome extraivel foi contada como arquivo distinto"
        );
    }

    #[test]
    fn falha_sem_nenhum_nome_extraivel_nunca_vira_sem_corrupcao() {
        let log = "\
2026-09-02 10:00:00, Info CSI 00000001 [SR] Beginning Verify and Repair transaction
2026-09-02 10:00:01, Info CSI 00000002 [SR] Cannot repair member file sem aspas nenhuma
2026-09-02 10:00:02, Info CSI 00000003 [SR] Cannot repair member file tambem sem aspas";

        match interpretar(log) {
            ResultadoSfc::NaoSei { motivo } => {
                assert!(!motivo.is_empty(), "motivo vazio nao ajuda o cliente");
            }
            outro => panic!("esperava NaoSei, veio {:?} — log com falha virou 'sem corrupcao'", outro),
        }
    }

    #[test]
    fn reparo_com_falhas_ilegiveis_nao_le_como_sucesso() {
        let log = "\
[SR] Beginning Verify and Repair transaction
[SR] Cannot repair member file [l:10]'ntdll.dll'
[SR] Cannot repair member file
[SR] Cannot repair member file
[SR] Cannot repair member file
[SR] Cannot repair member file
[SR] Repairing corrupted file \\??\\C:\\Windows\\ntdll.dll";

        let r = interpretar(log);
        assert!(
            !matches!(r, ResultadoSfc::Corrigiu { .. }),
            "quatro falhas ilegíveis viraram sucesso limpo: {:?}",
            r
        );
    }

    #[test]
    fn nenhuma_frase_traduzida_e_comparada() {
        let fonte = include_str!("cbslog.rs");
        for proibida in ["não encontrou", "violação", "nenhuma viola"] {
            assert!(
                !fonte.contains(&format!("contains(\"{}", proibida)),
                "está comparando texto traduzido: {}",
                proibida
            );
        }
    }
}
