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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ferramenta {
    VerificarArquivos,
    RepararImagem,
    VerificarDisco,
    ConsertarDisco,
    AnalisarWinSxS,
    LimparWinSxS { resetar_base: bool },
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

        Ferramenta::ConsertarDisco => Receita {
            programa: "chkdsk",
            args: args(&["C:", "/f"]),
            minutos_tipicos: (10, 60),
            // Não dá para cancelar: fica agendado para a inicialização. Quem
            // desmarca é o `chkntfs /x`, e a tela oferece isso enquanto a
            // máquina não reiniciou.
            cancelar_e_seguro: false,
            aviso: Some(
                "Exige reiniciar o computador. O conserto acontece antes de o \
                 Windows abrir, e não dá para usar a máquina durante ele.",
            ),
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

/// Se `chkdsk /f` pode ser oferecido.
///
/// Num disco em más condições, o `chkdsk` é justamente o que costuma matá-lo de
/// vez: ele reescreve estrutura em setores que já estão falhando. O `health.rs`
/// já sabe reconhecer esse disco, e esta é a trava que usa essa leitura.
pub fn consertar_disco_e_permitido(disco_saudavel: bool) -> bool {
    disco_saudavel
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

    #[test]
    fn disco_reprovado_nao_recebe_consertar() {
        // Num disco morrendo, o chkdsk é justamente o que costuma matá-lo de
        // vez — e o health.rs já sabe reconhecer esse disco.
        assert!(!consertar_disco_e_permitido(false));
        assert!(consertar_disco_e_permitido(true));
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
        // `/English` dá uma saída que não muda com o idioma do Windows.
        let r = receita(&Ferramenta::RepararImagem);
        assert!(r.args.iter().any(|a| a == "/English"), "args: {:?}", r.args);
    }
}
