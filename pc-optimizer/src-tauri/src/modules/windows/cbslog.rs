// A leitura do resultado do `sfc`
//
// POR QUE NÃO LER A SAÍDA DO CONSOLE
//
// O `sfc` escreve "não encontrou nenhuma violação de integridade" — em
// português, num Windows em português. Comparar essa frase quebraria em
// qualquer outro idioma, e é exatamente o defeito que `readiness.rs` já
// documenta para o `powercfg`.
//
// As marcas `[SR]` do CBS.log não são traduzidas. Elas são o mesmo texto em
// qualquer instalação do Windows, e é por isso que a leitura vem daqui.

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResultadoSfc {
    SemCorrupcao,
    Corrigiu { quantos: usize },
    NaoConseguiu { quantos: usize },
    /// Log ausente, vazio ou ilegível.
    ///
    /// NUNCA vira `SemCorrupcao`. "Não consegui conferir" virando "está tudo
    /// bem" é o mesmo defeito que o vigia de pagamento evita ao separar "não
    /// sei" de "não pago".
    NaoSei { motivo: String },
}

pub fn caminho_do_log() -> PathBuf {
    let raiz = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
    PathBuf::from(raiz).join("Logs").join("CBS").join("CBS.log")
}

pub fn interpretar(conteudo: &str) -> ResultadoSfc {
    let marcas: Vec<&str> = conteudo
        .lines()
        .filter(|l| l.contains("[SR]"))
        .collect();

    if marcas.is_empty() {
        return ResultadoSfc::NaoSei {
            motivo: "o registro do Windows não trouxe nenhuma verificação".into(),
        };
    }

    let nao_reparados = marcas
        .iter()
        .filter(|l| l.contains("Cannot repair member file"))
        .count();

    if nao_reparados == 0 {
        return ResultadoSfc::SemCorrupcao;
    }

    // "Repairing corrupted file" é o registro de que o conserto aconteceu. Sem
    // ele, o que ficou foi só a lista do que não deu para consertar.
    let reparados = marcas
        .iter()
        .filter(|l| l.contains("Repairing corrupted file"))
        .count();

    if reparados > 0 {
        ResultadoSfc::Corrigiu { quantos: reparados }
    } else {
        ResultadoSfc::NaoConseguiu {
            quantos: nao_reparados,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trecho no formato real: as marcas `[SR]` do CBS.log NÃO são traduzidas,
    /// mesmo num Windows em português. É por isso que a leitura vem daqui e
    /// não do texto do console.
    const SEM_CORRUPCAO: &str = "\
2026-08-31 12:00:01, Info CSI 00000001 [SR] Verifying 100 (0x00000064) components
2026-08-31 12:00:02, Info CSI 00000002 [SR] Verify complete
2026-08-31 12:00:03, Info CSI 00000003 [SR] Repairing 0 components";

    const CORRIGIU: &str = "\
2026-08-31 12:00:01, Info CSI 00000001 [SR] Cannot repair member file [l:20]'ntdll.dll'
2026-08-31 12:00:02, Info CSI 00000002 [SR] Repairing corrupted file \\??\\C:\\Windows\\ntdll.dll
2026-08-31 12:00:03, Info CSI 00000003 [SR] Repair complete";

    const NAO_CONSEGUIU: &str = "\
2026-08-31 12:00:01, Info CSI 00000001 [SR] Cannot repair member file [l:20]'ntdll.dll'
2026-08-31 12:00:02, Info CSI 00000002 [SR] Cannot repair member file [l:18]'user32.dll'";

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
    fn log_vazio_e_nao_sei_e_nunca_sem_corrupcao() {
        // "não consegui ler" virando "está tudo bem" é o mesmo defeito que o
        // vigia de pagamento evita ao separar "não sei" de "não pago".
        assert!(matches!(interpretar(""), ResultadoSfc::NaoSei { .. }));
    }

    #[test]
    fn nenhuma_frase_traduzida_e_comparada() {
        // A saída do console do `sfc` é traduzida; a do CBS.log não é. Comparar
        // texto em português quebraria em qualquer Windows de outro idioma.
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
