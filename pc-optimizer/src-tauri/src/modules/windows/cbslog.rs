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

use std::collections::HashSet;
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

/// O quão preocupante é o resultado — decidido AQUI, a partir do dado
/// estruturado, e nunca por uma frase que a tela teria que reconhecer por
/// prefixo.
///
/// Existe porque a tela chegou a colorir `CorrigiuEmParte` de verde: a frase
/// dele também começa com "Corrigiu ", igual à de `Corrigiu`, e um
/// `startsWith("Corrigiu ")` no frontend não via diferença entre "corrigiu
/// tudo" e "corrigiu metade e sobrou corrupção". É exatamente o defeito que
/// este arquivo já evita dentro do Rust — só que ele tinha voltado a
/// acontecer do lado de fora, na tela, porque o dado que chegava lá era
/// texto solto, e texto solto convida a ser lido por prefixo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severidade {
    Ok,
    Atencao,
    Erro,
}

impl ResultadoSfc {
    /// `CorrigiuEmParte` e `NaoConseguiu` SIGNIFICAM A MESMA COISA — sobrou
    /// corrupção na máquina — mas diferem em progresso: `CorrigiuEmParte`
    /// corrigiu ao menos um arquivo, `NaoConseguiu` corrigiu zero. É essa
    /// diferença de progresso, e não a coincidência de vocabulário nas duas
    /// frases, que separa `Atencao` de `Erro` abaixo. Sem progresso nenhum, o
    /// próximo passo (reparar a imagem do Windows) deixa de ser sugestão e
    /// passa a ser o único caminho.
    pub fn severidade(&self) -> Severidade {
        match self {
            ResultadoSfc::SemCorrupcao | ResultadoSfc::Corrigiu { .. } => Severidade::Ok,
            ResultadoSfc::CorrigiuEmParte { .. } => Severidade::Atencao,
            ResultadoSfc::NaoConseguiu { .. } => Severidade::Erro,
            // "Não sei" não é "está ruim" — é incerteza. Recebe o mesmo tom
            // intermediário do parcial, nunca o pior: a mesma regra que
            // impede `NaoSei` de colapsar em `SemCorrupcao` também impede que
            // ele colapse no tom mais grave sem prova nenhuma disso.
            ResultadoSfc::NaoSei { .. } => Severidade::Atencao,
        }
    }
}

pub fn caminho_do_log() -> PathBuf {
    let raiz = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
    PathBuf::from(raiz).join("Logs").join("CBS").join("CBS.log")
}

/// A marca que o CBS escreve no INÍCIO de cada passagem do `sfc`.
///
/// Como as demais marcas `[SR]`, ela não é traduzida.
const INICIO_DA_PASSAGEM: &str = "Beginning Verify and Repair transaction";

/// A ÚLTIMA execução do `sfc`, e não o arquivo inteiro.
///
/// O CBS.log só cresce: ele guarda todas as execuções, através de rotações.
/// Contar as marcas `[SR]` do arquivo todo somava execuções diferentes umas
/// nas outras — um cliente que rodou o `sfc` há três meses (dois arquivos sem
/// conserto), depois o `DISM`, e depois o `sfc` de novo (tudo consertado)
/// recebia contagens que não correspondiam a execução nenhuma. E a tela
/// chamava isso de "a última verificação".
///
/// Recortar pela marca de início resolve isso sem guardar estado: a última
/// passagem do arquivo é sempre a última que rodou, tenha ela sido disparada
/// por este produto, pelo prompt do cliente ou por outra ferramenta.
fn ultima_passagem(conteudo: &str) -> Option<&str> {
    conteudo.rfind(INICIO_DA_PASSAGEM).map(|inicio| &conteudo[inicio..])
}

/// Extrai o nome do arquivo de uma linha "Cannot repair member file
/// [l:NN]'nome.dll'". O nome vem entre o ÚLTIMO par de aspas simples da
/// linha.
///
/// Devolve `None` quando a linha não tem esse par de aspas — e nesse caso
/// quem chama NÃO deve inventar um nome nem contar a linha como arquivo
/// distinto. Subcontar é melhor que fabricar: um nome inventado poderia por
/// coincidência colidir com outro arquivo real e esconder uma falha, ou
/// nunca colidir e inflar `restantes` para sempre.
fn nome_do_arquivo_nao_reparado(linha: &str) -> Option<&str> {
    let fim = linha.rfind('\'')?;
    let inicio = linha[..fim].rfind('\'')?;
    let nome = &linha[inicio + 1..fim];
    (!nome.is_empty()).then_some(nome)
}

/// Extrai o nome do arquivo de uma linha "Repairing corrupted file
/// \\??\\C:\\Windows\\nome.dll" — o último trecho depois da última barra
/// (invertida ou não; o CBS.log usa barra invertida, mas nada garante isso
/// em todas as versões).
///
/// Mesma regra da função acima: sem um trecho final não vazio, `None`, e a
/// linha não conta.
fn nome_do_arquivo_reparado(linha: &str) -> Option<&str> {
    let nome = linha.trim_end().rsplit(['\\', '/']).next()?;
    (!nome.is_empty()).then_some(nome)
}

pub fn interpretar(conteudo: &str) -> ResultadoSfc {
    let Some(passagem) = ultima_passagem(conteudo) else {
        // Duas causas, e a frase precisa nomear a certa: um log que nunca viu
        // um `sfc` e um log em que não dá para saber onde a última execução
        // começa são situações diferentes para quem lê a tela. As duas são
        // `NaoSei` — nenhuma delas pode virar `SemCorrupcao`.
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

    // Deduplicado por NOME DE ARQUIVO, não por linha. O `sfc` pode registrar
    // "Cannot repair member file" duas vezes para o MESMO arquivo antes de
    // consertá-lo por uma fonte secundária — contar marcas brutas relatava
    // como quebrado um arquivo que já tinha sido corrigido. Linhas de onde o
    // nome não pôde ser extraído com segurança são descartadas (ver as duas
    // funções de extração acima) em vez de contadas por marca ou de receber
    // um nome inventado.
    let nao_reparados_set: HashSet<&str> = marcas
        .iter()
        .filter(|l| l.contains("Cannot repair member file"))
        .filter_map(|l| nome_do_arquivo_nao_reparado(l))
        .collect();

    if nao_reparados_set.is_empty() {
        return ResultadoSfc::SemCorrupcao;
    }

    // "Repairing corrupted file" é o registro de que o conserto aconteceu. Sem
    // ele, o que ficou foi só a lista do que não deu para consertar.
    let reparados_set: HashSet<&str> = marcas
        .iter()
        .filter(|l| l.contains("Repairing corrupted file"))
        .filter_map(|l| nome_do_arquivo_reparado(l))
        .collect();

    // `reparados` é a interseção: arquivos que apareceram como "não
    // consegui" E depois como "reparando" — o mesmo arquivo, consertado.
    // `restantes` é o que sobrou em `nao_reparados_set` sem esse par, e por
    // isso nunca precisa de `saturating_sub`: interseção nunca é maior que o
    // conjunto que a contém.
    let reparados = nao_reparados_set.intersection(&reparados_set).count();
    let restantes = nao_reparados_set.len() - reparados;

    if reparados == 0 {
        ResultadoSfc::NaoConseguiu {
            quantos: nao_reparados_set.len(),
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

    /// Duas famílias de arquivos diferentes: kernel32/gdi32 completam o ciclo
    /// "Cannot repair" seguido de "Repairing corrupted" (foram consertados),
    /// e user32 fica só com o "Cannot repair" (não sobrou registro de
    /// conserto). O resultado não pode ser `Corrigiu` — sobrou corrupção.
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
    fn duas_execucoes_no_mesmo_log_contam_so_a_ultima() {
        // O CBS.log só cresce. O cliente rodou o `sfc` há três meses e sobrou
        // corrupção; rodou o `DISM`; rodou o `sfc` de novo e desta vez o
        // conserto foi completo. Somando o arquivo inteiro, a tela reportava
        // uma contagem que não correspondia a execução nenhuma — e chamava
        // isso de "a última verificação".
        let historico = format!("{}\n{}", NAO_CONSEGUIU, CORRIGIU);

        assert_eq!(
            interpretar(&historico),
            ResultadoSfc::Corrigiu { quantos: 1 },
            "somou a execucao de tres meses atras na de agora"
        );
    }

    #[test]
    fn a_passagem_anterior_nao_contamina_a_atual_no_sentido_contrario() {
        // E o inverso também: uma execução limpa NÃO pode apagar o fato de a
        // última ter deixado corrupção para trás.
        let historico = format!("{}\n{}", CORRIGIU, NAO_CONSEGUIU);

        assert_eq!(
            interpretar(&historico),
            ResultadoSfc::NaoConseguiu { quantos: 2 }
        );
    }

    #[test]
    fn log_com_marcas_mas_sem_inicio_de_passagem_e_nao_sei() {
        // Sem a marca de início não da para saber a que execução as linhas
        // pertencem. Isso e "não sei", e a frase nomeia a causa certa em vez
        // de dizer que não houve verificação nenhuma.
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
        // "não consegui ler" virando "está tudo bem" é o mesmo defeito que o
        // vigia de pagamento evita ao separar "não sei" de "não pago".
        assert!(matches!(interpretar(""), ResultadoSfc::NaoSei { .. }));
    }

    #[test]
    fn corrigiu_em_parte_nunca_recebe_o_tom_de_sucesso() {
        // Este é o defeito que voltou na tela: a frase de `CorrigiuEmParte`
        // também começa com "Corrigiu ", e um `startsWith` no frontend
        // pintava os dois de verde. Aqui a severidade não vem de texto — vem
        // do variante em si, e o teste prende exatamente essa garantia.
        let parcial = ResultadoSfc::CorrigiuEmParte {
            corrigidos: 2,
            restantes: 1,
        };

        assert_ne!(parcial.severidade(), Severidade::Ok);
        assert_eq!(parcial.severidade(), Severidade::Atencao);
    }

    #[test]
    fn nao_conseguiu_e_mais_grave_que_corrigiu_em_parte() {
        // Os dois significam "sobrou corrupção" — mas `NaoConseguiu`
        // corrigiu zero arquivos, e é essa ausência de progresso que separa
        // `Erro` de `Atencao`, não a semelhança de vocabulário entre as duas
        // frases.
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
        // "Não sei" é incerteza, não "está ruim" — não pode colapsar no tom
        // mais grave sem nenhuma prova de que a máquina está mal.
        let ns = ResultadoSfc::NaoSei {
            motivo: "teste".into(),
        };

        assert_eq!(ns.severidade(), Severidade::Atencao);
    }

    #[test]
    fn o_mesmo_arquivo_falhando_duas_vezes_conta_uma() {
        // O `sfc` pode registrar duas tentativas falhas do MESMO arquivo antes de
        // consertá-lo de outra fonte. Contando marcas brutas, `restantes` reportava
        // como quebrado um arquivo que foi consertado.
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
        // Uma linha "Cannot repair member file" sem o par de aspas (log
        // truncado, formato inesperado) não pode virar um arquivo fantasma.
        // Subcontar é o comportamento seguro; inventar um nome poderia tanto
        // esconder uma falha real quanto inflar `restantes` para sempre.
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
