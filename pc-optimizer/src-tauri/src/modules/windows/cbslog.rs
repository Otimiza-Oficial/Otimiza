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
    /// Reparo misto: parte dos arquivos corrompidos foi consertada, mas
    /// sobrou corrupção sem conserto.
    ///
    /// Existe porque devolver `Corrigiu` aqui contaria um resultado que não
    /// foi alcançado — a mesma regra que o módulo de comparação já aplica ao
    /// se recusar a chamar ruído de medição de ganho. Um cliente que recebe
    /// "corrigido" quando ainda há corrupção na máquina não sabe que precisa
    /// seguir para o reparo da imagem do Windows; `CorrigiuEmParte` deixa
    /// isso visível.
    CorrigiuEmParte { corrigidos: usize, restantes: usize },
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

    // `nao_reparados` conta toda linha "Cannot repair member file", e essa
    // linha aparece SEMPRE que o CBS tenta primeiro a fonte local — inclusive
    // para os arquivos que depois são consertados por uma fonte secundária
    // (é o que a linha "Repairing corrupted file" registra na sequência). Ou
    // seja, um arquivo que termina reparado também soma em `nao_reparados`;
    // por isso `nao_reparados - reparados` é a contagem de quem ficou para
    // trás, não um dobro de quem foi consertado — não é uma subtração ingênua
    // que dobra a contagem, é a leitura correta do formato do log.
    let restantes = nao_reparados.saturating_sub(reparados);

    if reparados == 0 {
        ResultadoSfc::NaoConseguiu {
            quantos: nao_reparados,
        }
    } else if restantes == 0 {
        ResultadoSfc::Corrigiu { quantos: reparados }
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

    /// Duas famílias de arquivos diferentes: kernel32/gdi32 completam o ciclo
    /// "Cannot repair" seguido de "Repairing corrupted" (foram consertados),
    /// e user32 fica só com o "Cannot repair" (não sobrou registro de
    /// conserto). O resultado não pode ser `Corrigiu` — sobrou corrupção.
    const CORRIGIU_PARCIAL: &str = "\
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
        // Contar "corrigido" quando ainda sobra corrupção é o mesmo defeito
        // que o módulo de comparação evita ao se recusar a chamar ruído de
        // medição de ganho: reportar um resultado que não foi alcançado.
        assert_eq!(
            interpretar(CORRIGIU_PARCIAL),
            ResultadoSfc::CorrigiuEmParte {
                corrigidos: 2,
                restantes: 1
            }
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
