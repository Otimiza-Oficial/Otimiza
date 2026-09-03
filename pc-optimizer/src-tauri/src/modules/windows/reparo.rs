// As ferramentas de reparo do Windows
//
// O produto tinha 42 ajustes e nenhum reparo. Numa máquina com arquivo de
// sistema corrompido, nenhum dos 42 adianta — e é a explicação, sem supor
// nada, para "os comandos do terminal ajudaram mais que o Otimiza".
//
// REPARO NÃO É OTIMIZAÇÃO, E POR ISSO NÃO ESTÁ NO CATÁLOGO.
//
// Toda mudança do produto é reversível com o valor anterior guardado. O `sfc`
// não muda ajuste nenhum: devolve um arquivo corrompido ao original. Não há
// valor anterior a guardar, e desfazer significaria recorromper de propósito.
//
// Este módulo só descreve as ferramentas — programa, argumentos, duração
// típica, se cancelar é seguro. Ele não sabe rodar processo nem ler log; quem
// executa e quem interpreta a saída são módulos à parte.
//
// A trava do disco importa `health` (o tipo `HealthReport`) e `tarefa_longa`
// (o tipo `Desfecho`), e nada além disso. Isso não quebra a separação acima:
// ler um diagnóstico JÁ PRONTO e um desfecho JÁ ACONTECIDO para decidir se um
// argumento pode existir ainda é DESCREVER. Rodar processo e interpretar log
// continuam de fora — este arquivo não chama nenhum dos dois.

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
    /// A volta atrás do `ConsertarDisco`, enquanto a máquina ainda não
    /// reiniciou.
    DesmarcarConsertoDoDisco,
}

pub struct Receita {
    pub programa: &'static str,
    pub args: Vec<String>,
    /// Para a tela poder dizer "de 10 a 30 minutos" antes de começar. É o que
    /// impede o cliente de desistir no meio.
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
            // Interrompido, ele para. Rodar de novo recomeça do zero.
            cancelar_e_seguro: true,
            aviso: None,
        },

        Ferramenta::RepararImagem => Receita {
            programa: "DISM",
            args: args(&[
                "/Online",
                "/Cleanup-Image",
                "/RestoreHealth",
                // Saída que não muda com o idioma do Windows.
                "/English",
            ]),
            minutos_tipicos: (10, 30),
            cancelar_e_seguro: false,
            aviso: Some(
                "Fica parado em 20% por vários minutos, e isso é normal. \
                 Precisa de internet: os arquivos bons vêm do Windows Update.",
            ),
        },

        // `/scan` roda com o Windows LIGADO, em NTFS. Acha sem consertar.
        Ferramenta::VerificarDisco => Receita {
            programa: "chkdsk",
            args: args(&["C:", "/scan"]),
            minutos_tipicos: (2, 20),
            cancelar_e_seguro: true,
            aviso: None,
        },

        // POR QUE NÃO É `chkdsk C: /f`.
        //
        // No volume do sistema EM USO, o `chkdsk /f` não consegue travar o
        // volume e faz uma PERGUNTA: "Would you like to schedule this volume
        // to be checked the next time the system restarts? (Y/N)" — traduzida,
        // num Windows em português, para "(S/N)". O filho nasce com
        // `CREATE_NO_WINDOW` e sem console: não há para onde a pergunta ir, e
        // ninguém a responde. No melhor caso o `chkdsk` desiste e a tela pinta
        // "Terminou." de verde sem nada ter sido agendado; no pior ele fica
        // preso na pergunta e trava o executor. Nos dois, o cliente é
        // informado de que o conserto foi feito quando não foi.
        //
        // Responder pelo cano também não serve: a letra da resposta é
        // TRADUZIDA. Um "Y" enviado a um Windows em português não é aceito, e
        // adivinhar o idioma do console é a mesma armadilha que o `shell.rs`
        // já documenta para a página de código.
        //
        // Então não se pergunta. O que o "S" faria — marcar o volume como
        // sujo, para o `autochk` rodar o conserto completo antes de o Windows
        // abrir — é exatamente o que o `fsutil dirty set` faz direto, sem
        // pergunta nenhuma e com código de saída que dá para conferir. É o
        // MESMO mecanismo, acionado pela porta que não depende de um diálogo
        // que não existe.
        // ESTA RECEITA SOZINHA NÃO BASTA. `fsutil dirty set` marca o bit; o
        // `autochk` só olha esse bit se o volume não estiver na lista de
        // exclusão que `chkntfs /X` cria (ver `DesmarcarConsertoDoDisco`
        // abaixo). Se uma sessão anterior desmarcou um conserto, o volume
        // continua nessa lista PARA SEMPRE — `/X` não é "cancele o próximo
        // boot", é "pare de checar este volume", e nada nesta receita desfaz
        // isso. Quem chama precisa rodar `receita_reinclusao_do_disco()`
        // ANTES desta, ou o `fsutil` sai 0, a tela diz "agendado", e o
        // conserto simplesmente não acontece — a mentira exata que este
        // módulo existe para impedir. O executor (`reparo_executar`, em
        // `commands.rs`) é quem sequencia as duas: esta receita continua
        // descrevendo um comando só.
        Ferramenta::ConsertarDisco => Receita {
            programa: "fsutil",
            args: args(&["dirty", "set", "C:"]),
            // O clique volta em segundos; estes minutos são os do conserto em
            // si, que acontece na próxima inicialização — e é isso que o aviso
            // abaixo diz com todas as letras.
            minutos_tipicos: (10, 60),
            // Não dá para cancelar: fica agendado para a inicialização. Quem
            // desmarca é o `DesmarcarConsertoDoDisco`, e a tela oferece isso
            // enquanto a máquina não reiniciou.
            cancelar_e_seguro: false,
            aviso: Some(
                "Agenda o conserto para a próxima vez que você ligar o \
                 computador — ele acontece antes de o Windows abrir, e não dá \
                 para usar a máquina durante ele. Enquanto você não reiniciar, \
                 dá para desmarcar aqui mesmo.",
            ),
        },

        // A saída de emergência. O `ConsertarDisco` é a única operação do
        // produto que não dá para cancelar depois de começar; poder desmarcar
        // antes do reinício é o que impede o aviso "não dá para voltar atrás"
        // de virar uma porta trancada.
        //
        // `chkntfs /X` NÃO É "cancele o check agendado". É "acrescente este
        // volume à lista de exclusão do boot check, e deixe-o lá". A lista é
        // persistente — sobrevive ao reinício, à sessão, ao próprio produto
        // fechando — e é consultada em TODO boot daqui em diante, não só no
        // próximo. Um cliente que desmarca uma vez e, meses depois, tem o
        // `/scan` achando erro de novo: o `fsutil dirty set` roda, sai 0, a
        // tela diz "agendado" — e o `autochk` pula o volume no boot seguinte
        // porque ele nunca saiu daquela lista. É por isso que
        // `Ferramenta::ConsertarDisco` precisa reincluir o volume antes de
        // marcar: ver `receita_reinclusao_do_disco` e o comentário acima.
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
                // Mexe no WinSxS mesmo sem `/ResetBase`: uma limpeza cortada no
                // meio pode deixar o componente pela metade, do mesmo jeito que
                // o `/RestoreHealth` acima — cancelar não é de graça aqui também.
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

/// O passo que precisa rodar ANTES de `receita(&Ferramenta::ConsertarDisco)`,
/// toda vez.
///
/// `chkntfs /C C:` devolve o volume à lista de "verificar no boot" —
/// desfazendo um `/X` de qualquer sessão passada, inclusive uma de antes
/// desta função existir. NÃO é uma `Ferramenta` do catálogo: ninguém pede
/// "reincluir o disco" na tela, `reparo_disponivel` nunca a oferece, e ela
/// não tem estado próprio em `EstadoDoDisco` — é sempre um passo interno do
/// agendamento, nunca uma escolha do cliente.
///
/// NÃO É DESTRUTIVO. `/C` só restaura o comportamento padrão do Windows para
/// o volume; rodar num volume que nunca foi excluído não muda nada. Por isso
/// dá para chamar sempre, sem precisar saber se um `/X` aconteceu antes —
/// saber isso exigiria um registro que este produto não guarda hoje, e
/// exigiria confiar nesse registro sobre o que o Windows realmente tem
/// gravado no volume.
pub fn receita_reinclusao_do_disco() -> Receita {
    Receita {
        programa: "chkntfs",
        args: args(&["/C", "C:"]),
        minutos_tipicos: (0, 1),
        cancelar_e_seguro: true,
        aviso: None,
    }
}

/// Se a reinclusão deu certo — a condição que autoriza o executor a seguir
/// para o `fsutil dirty set`.
///
/// Função pura pelo mesmo motivo de `EstadoDoDisco::apos_execucao`: é a
/// regra que decide se o cliente pode ouvir "agendado" depois do
/// `ConsertarDisco`, e precisa ser conferível sem rodar `chkntfs` de
/// verdade. Só o código 0 conta — o mesmo corte que `EstadoDoDisco` já usa
/// para o `fsutil` e para o próprio `chkntfs /X`: um `Cancelada`, um
/// `NaoComecou` ou um código diferente de zero são "não sei se reincluiu", e
/// "não sei" não pode virar "pode marcar sujo".
pub fn reinclusao_deu_certo(desfecho: &Desfecho) -> bool {
    matches!(desfecho, Desfecho::Terminou { codigo: 0 })
}

/// O que se sabe sobre o disco DESTA MÁQUINA, nesta sessão.
///
/// A especificação diz: "Só se oferece `/f` DEPOIS de o `/scan` achar alguma
/// coisa. Sem achado, não há motivo para reiniciar a máquina de ninguém."
/// Essa trava não existia, e nada em lugar nenhum registrava se o
/// `VerificarDisco` tinha rodado ou o que ele tinha achado — a oferta saía só
/// da saúde do disco, e a tela ainda descrevia o botão como "corrige os erros
/// que a verificação encontrou", uma frase que afirma uma medição que nunca
/// foi feita. Num produto cuja regra fundadora é "nunca mostrar número que não
/// foi medido", isso é um achado declarado sem medição.
///
/// É UM VALOR SÓ, E NÃO UMA COLEÇÃO DE BANDEIRAS. Duas bandeiras
/// independentes ("já verificou" e "achou alguma coisa") admitiriam o estado
/// sem sentido "não verificou mas achou", e alguém teria que lembrar de nunca
/// produzi-lo. Aqui esse estado não existe para ser produzido.
///
/// Nasce em `SemVerificacao`: começar do "não sei" é o que faz a ausência de
/// medição negar por padrão, e não abrir por descuido.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EstadoDoDisco {
    /// Ninguém verificou nada nesta sessão. Nada a oferecer.
    #[default]
    SemVerificacao,
    /// O `/scan` rodou até o fim e não achou erro de estrutura.
    VerificadoSemAchado,
    /// O `/scan` rodou e achou. É o ÚNICO estado que autoriza o conserto.
    VerificadoComAchado,
    /// O conserto já está marcado para a próxima inicialização. A partir daqui
    /// o que se oferece é a volta atrás, não o conserto de novo.
    ConsertoAgendado,
}

impl EstadoDoDisco {
    /// O que o disco passa a ser depois de uma ferramenta ter rodado.
    ///
    /// Função pura de propósito: é a regra que decide se alguém pode agendar
    /// um `chkdsk` na máquina do cliente, e ela precisa ser conferível sem
    /// disco, sem reinício e sem elevação.
    ///
    /// OS CÓDIGOS DE SAÍDA DO `chkdsk`. `0` é "nenhum erro"; `1` e `2` são
    /// "achou" (o `2` é o que o `/scan` devolve quando há coisa para o
    /// conserto offline resolver); `3` é "não consegui verificar". O `3`, o
    /// cancelamento e a tarefa que nem começou levam de volta a
    /// `SemVerificacao` — não a `VerificadoSemAchado`. É a mesma regra do
    /// `NaoSei` do `cbslog`: "não consegui conferir" nunca vira "está tudo
    /// bem", e aqui isso significa que a oferta do conserto some em vez de
    /// aparecer sobre nada.
    pub fn apos_execucao(self, ferramenta: &Ferramenta, desfecho: &Desfecho) -> EstadoDoDisco {
        match ferramenta {
            Ferramenta::VerificarDisco => match desfecho {
                Desfecho::Terminou { codigo: 0 } => EstadoDoDisco::VerificadoSemAchado,
                Desfecho::Terminou { codigo: 1 } | Desfecho::Terminou { codigo: 2 } => {
                    EstadoDoDisco::VerificadoComAchado
                }
                _ => EstadoDoDisco::SemVerificacao,
            },

            // O agendamento só conta quando o `fsutil` confirma que marcou.
            // Um código diferente de zero mantém o achado de pé: o cliente
            // continua vendo a oferta e pode tentar de novo, em vez de a tela
            // afirmar que agendou algo que não agendou.
            Ferramenta::ConsertarDisco => match desfecho {
                Desfecho::Terminou { codigo: 0 } => EstadoDoDisco::ConsertoAgendado,
                _ => self,
            },

            // Desmarcado, a máquina volta a não ter medição nenhuma válida: o
            // achado que autorizava o conserto foi consumido no agendamento, e
            // oferecer o conserto de novo sem novo `/scan` seria repetir o
            // defeito que está enum existe para fechar.
            Ferramenta::DesmarcarConsertoDoDisco => match desfecho {
                Desfecho::Terminou { codigo: 0 } => EstadoDoDisco::SemVerificacao,
                _ => self,
            },

            // As ferramentas de arquivo e de componente não dizem nada sobre a
            // estrutura do disco.
            _ => self,
        }
    }

    /// Se o `chkdsk` pode ser agendado — a metade da trava que fala de
    /// MEDIÇÃO. A outra metade, que fala da SAÚDE do disco, é
    /// `consertar_disco_e_permitido`; as duas precisam valer.
    pub fn autoriza_consertar(self) -> bool {
        self == EstadoDoDisco::VerificadoComAchado
    }

    /// Se há o que desmarcar. Oferecer a volta atrás de algo que nunca foi
    /// agendado seria afirmar um estado da máquina que ninguém mediu.
    pub fn tem_conserto_agendado(self) -> bool {
        self == EstadoDoDisco::ConsertoAgendado
    }
}

/// Prova de que um disco foi lido e passou no exame — não um bool solto.
///
/// O campo é privado de propósito. Um `bool` na assinatura de
/// `consertar_disco_e_permitido` podia vir de qualquer lugar: um valor fixo, um
/// `unwrap_or(true)` esquecido, uma inversão de sinal — o compilador não via
/// diferença entre isso e uma leitura real do disco. Sem um construtor que
/// exige o `HealthReport`, não existe caminho para produzir um `DiscoSaudavel`
/// que não tenha vindo de um diagnóstico de verdade.
pub struct DiscoSaudavel(bool);

/// O prefixo de TODO achado de disco do `health.rs`.
///
/// São quatro famílias hoje — `disk_status_*`, `disk_wear_*`, `disk_errors_*`,
/// `disk_temp_*` — e um quinto informativo (`disk_hours_*`). A trava olhava só
/// a primeira, e era isso que a deixava aberta num SSD com 97% da vida
/// consumida: o Windows ainda reportava `Healthy`, então `disk_status_0` saía
/// `Ok` enquanto `disk_wear_0` e `disk_errors_0` gritavam `Critical`. O
/// produto dizia numa aba que o disco estava morrendo e noutra reescrevia a
/// estrutura dele.
///
/// Filtrar pelo prefixo curto — e não por uma lista de nomes — é de propósito:
/// uma quinta família de achado de disco criada amanhã no `health.rs` entra
/// nesta trava sozinha, sem ninguém precisar lembrar de vir aqui.
const PREFIXO_DE_DISCO: &str = "disk_";

/// O achado que prova que o Windows conseguiu falar com o disco.
const PREFIXO_DE_ESTADO: &str = "disk_status_";

impl DiscoSaudavel {
    /// Único jeito de obter um `DiscoSaudavel`: a partir do relatório real.
    ///
    /// A TRAVA RECUSA POR PADRÃO. A pergunta não é "apareceu algum motivo para
    /// recusar", é "apareceu evidência que justifique deixar o `chkdsk`
    /// reescrever estrutura por cima dos setores deste disco". Sem essa
    /// evidência, a resposta é não — e é essa inversão, e não uma checagem a
    /// mais, que fecha o buraco.
    ///
    /// São três as condições, e todas precisam valer:
    ///
    /// - `needs_admin` falso. `needs_admin` significa que a checagem não
    ///   conseguiu ler o disco. "Não sei" não pode virar "está bem" — é a
    ///   mesma regra do `NaoSei` do `cbslog`, que nunca colapsa em
    ///   `SemCorrupcao`, e do monitor de pagamento, que separa "não sei se
    ///   pagou" de "não pagou".
    ///
    /// - Existe pelo menos um achado `disk_status_*`. Esse achado é o recibo
    ///   de que o Windows leu o disco e disse algo reconhecível sobre ele:
    ///   `avaliar_estado` só devolve `Some` para `Healthy`, `Warning` e
    ///   `Unhealthy`. Um `"Unknown"` — comum em RAID, USB, máquina virtual e
    ///   controladora antiga — não produz achado nenhum, e a versão anterior
    ///   lia essa AUSÊNCIA como aprovação. O mesmo valia para uma máquina em
    ///   que o PowerShell falhou e a lista de discos voltou vazia: nenhum
    ///   achado, nenhum `needs_admin`, e a trava abria sobre nada.
    ///
    /// - NENHUM achado `disk_*` com severidade diferente de `Ok`. Não só o de
    ///   estado: desgaste, erros acumulados e temperatura são, neste produto,
    ///   a própria definição de disco morrendo. E o corte é "qualquer coisa
    ///   que não seja `Ok`", e não "só `Critical`", porque a pergunta aqui não
    ///   é "isto já é grave o bastante para preocupar o cliente" (isso é o que
    ///   a severidade mede para a TELA), é "o Windows já viu algo de errado
    ///   neste disco" — e `Important` já é isso.
    ///
    /// UMA LACUNA TOLERADA DE PROPÓSITO: `disk_errors_naosei_*` — o achado que
    /// `health.rs` emite quando `ReadErrorsTotal`/`WriteErrorsTotal` não vêm
    /// do Windows para aquele disco — sai com severidade `Ok`, então passa por
    /// este filtro e a trava continua liberando o `chkdsk /f`. Isto é decisão,
    /// não descuido: a evidência que esta trava exige é que a SAÚDE do disco
    /// tenha sido lida, e `disco_foi_lido` já comprova isso via
    /// `disk_status_*`. O contador de erros é um dado A MAIS, que boa parte
    /// dos SSDs simplesmente não publica — recusar o reparo a todo SSD sem
    /// esse contador seria negar o conserto pela falta de um dado que a
    /// maioria dos discos nunca teve. A lacuna que a trava NÃO tolera continua
    /// sendo não saber a saúde (`needs_admin`, ausência de `disk_status_*`) ou
    /// saber que algo está errado (severidade diferente de `Ok`); não saber
    /// sobre um contador específico e pouco publicado é outra categoria — mais
    /// estreita, e aceita.
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

        // Sem o recibo de que o disco foi lido, não há o que aprovar. Isto
        // cobre de uma vez os três jeitos de não ter evidência: relatório sem
        // achado nenhum, disco que só respondeu `Unknown`, e leitura que
        // trouxe desgaste ou temperatura mas nunca chegou ao estado.
        let disco_foi_lido = achados_de_disco().any(|a| a.id.starts_with(PREFIXO_DE_ESTADO));

        if !disco_foi_lido {
            return DiscoSaudavel(false);
        }

        let algum_problema = achados_de_disco().any(|a| a.severity != FindingSeverity::Ok);

        DiscoSaudavel(!algum_problema)
    }
}

/// Se `chkdsk /f` pode ser oferecido.
///
/// Num disco em más condições, o `chkdsk` é justamente o que costuma matá-lo de
/// vez: ele reescreve estrutura em setores que já estão falhando. `DiscoSaudavel`
/// só existe quando veio de um `HealthReport` de verdade, então está trava não
/// tem como ser furada por um bool inventado na chamada.
pub fn consertar_disco_e_permitido(disco: &DiscoSaudavel) -> bool {
    disco.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verificar_disco_roda_sem_reiniciar() {
        // O que se ensina por aí é `chkdsk /f`, que reinicia a máquina e prende
        // a pessoa numa tela azul por tempo indeterminado. `/scan` roda com o
        // Windows ligado e acha o problema sem consertar.
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
        // `needs_admin` é "não sei", não "está bem" — não sei não pode virar
        // sim, do mesmo jeito que o NaoSei do cbslog nunca vira SemCorrupcao.
        let relatorio = HealthReport {
            findings: Vec::new(),
            needs_admin: true,
        };

        let disco = DiscoSaudavel::a_partir_do_relatorio(&relatorio);
        assert!(!consertar_disco_e_permitido(&disco));
    }

    #[test]
    fn disco_reprovado_no_relatorio_nao_recebe_consertar() {
        // Num disco morrendo, o chkdsk é justamente o que costuma matá-lo de
        // vez — e o health.rs já sabe reconhecer esse disco.
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
        // O caminho do "não consegui conferir" virando "está tudo bem": o
        // PowerShell falhou, `discos()` voltou vazio, e `analisar_discos` só
        // marca `needs_admin` quando a lista NÃO está vazia. Resultado:
        // relatorio limpo, sem achado nenhum, numa máquina onde nada do disco
        // pode ser lido. A trava antiga abria aqui.
        assert!(!permite(Vec::new()));
    }

    #[test]
    fn estado_que_o_windows_nao_soube_dizer_nao_recebe_consertar() {
        // `avaliar_estado` devolve `None` para qualquer coisa que não seja
        // `Healthy`/`Warning`/`Unhealthy`. `"Unknown"` e comum em RAID, USB,
        // máquina virtual e controladora antiga: nenhum `disk_status_*` e
        // emitido. A trava antiga lia essa AUSÊNCIA como aprovacao.
        assert!(!permite(vec![
            achado("disk_wear_0", FindingSeverity::Ok),
            achado("disk_hours_0", FindingSeverity::Ok),
        ]));
    }

    #[test]
    fn desgaste_critico_nao_recebe_consertar_mesmo_com_o_windows_dizendo_healthy() {
        // O cenario exato do achado: SSD com 97% da vida consumida. O Windows
        // ainda reporta `Healthy`, então `disk_status_0` sai `Ok` — e a trava
        // antiga, que só olhava `disk_status_*`, abria. O produto dizia numa
        // aba que o disco estava morrendo e noutra reescrevia a estrutura.
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
        // `disk_temp_*` nasce `Important`, e não `Critical`. O corte da trava e
        // "qualquer coisa que não seja Ok" justamente para isto: a pergunta e
        // "o Windows já viu algo de errado neste disco", não "já e grave o
        // bastante para assustar o cliente".
        assert!(!permite(vec![
            achado("disk_status_0", FindingSeverity::Ok),
            achado("disk_temp_0", FindingSeverity::Important),
        ]));
    }

    #[test]
    fn achado_informativo_de_outra_area_nao_reprova_o_disco() {
        // O filtro e por prefixo `disk_`: um achado de memória ou de energia
        // marcado `Critical` no mesmo relatorio não tem nada a ver com a
        // estrutura do disco e não pode fechar a trava por tabela.
        assert!(permite(vec![
            achado("disk_status_0", FindingSeverity::Ok),
            achado("disk_hours_0", FindingSeverity::Ok),
            achado("memory_pressure", FindingSeverity::Critical),
        ]));
    }

    #[test]
    fn o_consertar_nasce_sem_autorizacao() {
        // A especificacao: "Só se oferece /f DEPOIS de o /scan achar alguma
        // coisa. Sem achado, não há motivo para reiniciar a máquina de
        // ninguém." Antes disto, um cliente com NTFS limpo abria a aba pela
        // primeira vez e já via "Consertar a estrutura do disco".
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
        // Código 3 do chkdsk e "não consegui verificar". Isso e "não sei", e
        // "não sei" nunca vira "achei" nem "está limpo".
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
        // O pior desfecho possível aqui e a tela dizer que agendou o conserto
        // sem nada ter sido agendado — foi exatamente isso que o `chkdsk /f`
        // sem resposta para a pergunta produzia.
        let depois = EstadoDoDisco::VerificadoComAchado
            .apos_execucao(&Ferramenta::ConsertarDisco, &Desfecho::Terminou { codigo: 1 });

        assert_eq!(depois, EstadoDoDisco::VerificadoComAchado);
        assert!(!depois.tem_conserto_agendado());
    }

    #[test]
    fn o_conserto_do_disco_nao_faz_pergunta_a_ninguem() {
        // `chkdsk C: /f` no volume do sistema em uso PERGUNTA se deve agendar,
        // e a pergunta e traduzida. O filho nasce sem console: ninguém
        // responde. O `fsutil dirty set` marca o mesmo bit que a resposta "S"
        // marcaria, sem diálogo nenhum.
        let r = receita(&Ferramenta::ConsertarDisco);
        assert_eq!(r.programa, "fsutil");
        assert!(
            !r.args.iter().any(|a| a == "/f"),
            "voltou a chamar o chkdsk /f, que faz pergunta: {:?}",
            r.args
        );

        // E existe volta atras enquanto a máquina não reiniciou.
        let volta = receita(&Ferramenta::DesmarcarConsertoDoDisco);
        assert_eq!(volta.programa, "chkntfs");
        assert!(volta.cancelar_e_seguro);
    }

    #[test]
    fn a_reinclusao_desfaz_exatamente_o_que_o_x_faz() {
        // `chkntfs /X` exclui o volume do boot check PARA SEMPRE, não só uma
        // vez. `/C` é o comando documentado pela Microsoft para desfazer
        // isso — e precisa ser ESTE volume (`C:`), com ESTE programa
        // (`chkntfs`), ou não reverte nada.
        let reinclusao = receita_reinclusao_do_disco();
        let desmarcar = receita(&Ferramenta::DesmarcarConsertoDoDisco);

        assert_eq!(reinclusao.programa, "chkntfs");
        assert_eq!(desmarcar.programa, "chkntfs");
        assert!(reinclusao.args.iter().any(|a| a == "/C"));
        assert!(reinclusao.args.iter().any(|a| a == "C:"));
        assert!(desmarcar.args.iter().any(|a| a == "/X"));

        // Não é a mesma receita do `DesmarcarConsertoDoDisco`: uma exclui, a
        // outra reinclui, e confundi-las apagaria o efeito uma da outra.
        assert_ne!(reinclusao.args, desmarcar.args);

        // Não é destrutiva: cancelar no meio não deixa nada pela metade,
        // então oferecer cancelamento dela nunca seria arriscado (mesmo que
        // hoje ela não seja oferecida como botão nenhum).
        assert!(reinclusao.cancelar_e_seguro);
    }

    #[test]
    fn conserto_do_disco_nao_marca_sujo_sem_reincluir_antes() {
        // A receita do `ConsertarDisco`, sozinha, continua sendo só o
        // `fsutil dirty set` — `Receita` carrega um programa só de propósito.
        // A reinclusão é um passo À PARTE, que o executor roda ANTES desta
        // receita (ver `reparo_executar` em commands.rs); este teste prova
        // que a receita e o passo de reinclusão continuam sendo comandos
        // DIFERENTES, para o dia em que alguém tentar "simplificar" fundindo
        // os dois numa `Receita` só e apagando a reinclusão sem perceber.
        let marcar = receita(&Ferramenta::ConsertarDisco);
        let reinclusao = receita_reinclusao_do_disco();

        assert_eq!(marcar.programa, "fsutil");
        assert_eq!(reinclusao.programa, "chkntfs");
        assert_ne!(marcar.programa, reinclusao.programa);
    }

    #[test]
    fn reinclusao_que_terminou_com_erro_nao_autoriza_marcar_sujo() {
        // O cenário do achado: `/C` falha (permissão, volume ocupado, o que
        // for) e o executor segue para o `fsutil` mesmo assim. O `fsutil`
        // quase sempre funciona — ele só grava um bit — então o desfecho
        // final sai 0 e a tela diria "agendado" sobre um volume que
        // continua fora do boot check. `reinclusao_deu_certo` é o freio que
        // impede o executor de seguir nesse caso.
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

        // O cliente perde a capacidade de desinstalar atualizações. Isso não
        // pode ficar só na cabeça de quem escreveu a tela.
        assert!(com.aviso.is_some(), "/ResetBase saiu sem aviso");
        assert!(sem.aviso.is_none());
    }

    #[test]
    fn cancelar_o_dism_nao_e_de_graca() {
        // Interrompido no meio de uma escrita, o DISM pode deixar uma operação
        // pendente que só se resolve rodando de novo até o fim.
        assert!(!receita(&Ferramenta::RepararImagem).cancelar_e_seguro);
        assert!(receita(&Ferramenta::VerificarArquivos).cancelar_e_seguro);
    }

    #[test]
    fn o_dism_pede_saida_estavel() {
        // `/English` dá uma saída que não muda com o idioma do Windows. As três
        // ferramentas rodam DISM, e as três precisam do argumento — uma
        // regressão que tirasse `/English` só de uma delas ficaria invisível
        // se o teste checasse uma única variante.
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
