// Em que núcleos um processo pode rodar
//
// AFINIDADE É PROPRIEDADE DO PROCESSO, NÃO CONFIGURAÇÃO DO WINDOWS
//
// Isso muda tudo sobre como ela pode ser vendida. Ela vale para o processo que
// está aberto AGORA; fechou o jogo, acabou. Não há chave de registro, não há
// "aplicar no boot", e um produto que a oferece como ajuste permanente está
// oferecendo uma coisa que some sozinha.
//
// A consequência prática é boa, aliás: é a mudança mais reversível que existe
// neste produto. Fechar e abrir o jogo já desfaz.
//
// POR QUE ISTO NÃO ENTRA NO HISTÓRICO DE MUDANÇAS
//
// O histórico existe para o Desfazer encontrar o valor anterior de coisas que
// PERSISTEM. Uma afinidade anotada lá viraria uma linha que o cliente vê para
// sempre, oferecendo desfazer um processo que já não existe. A afinidade tem o
// próprio botão, ao lado, enquanto o jogo está aberto — e é o lugar certo.
//
// O QUE O WINDOWS RECUSA, E POR QUÊ IMPORTA
//
// `SetProcessAffinityMask` falha quando a máscara pede um núcleo que o processo
// não pode usar — por exemplo, um que está fora do grupo dele. O erro do
// Windows não explica nada, então quem chama daqui recebe uma frase que
// explica.

#![cfg(target_os = "windows")]

use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE};
use windows_sys::Win32::System::Threading::{
    GetProcessAffinityMask, OpenProcess, SetProcessAffinityMask, PROCESS_QUERY_INFORMATION,
    PROCESS_SET_INFORMATION,
};

/// Fecha a alça sozinho.
///
/// Sem isto, todo caminho de erro precisaria lembrar de fechar — e alça vazada
/// num programa que fica aberto durante a partida inteira é vazamento que
/// aparece depois de horas, longe de onde nasceu.
struct Alca(HANDLE);

impl Drop for Alca {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0) };
    }
}

fn abrir(pid: u32, direitos: u32) -> Result<Alca, String> {
    let alca = unsafe { OpenProcess(direitos, 0, pid) };

    if alca.is_null() {
        let erro = unsafe { GetLastError() };
        return Err(match erro {
            // 5 é acesso negado: processo de outro usuário, ou com mais
            // privilégio que o nosso. Anticheat também costuma bloquear.
            5 => "o Windows negou acesso a este processo. Jogos com anticheat costumam \
                  bloquear isso, e alguns exigem que o Otimiza rode como administrador."
                .to_string(),
            // 87 é parâmetro inválido, que aqui significa processo que já
            // fechou entre listar e abrir.
            87 => "o processo não existe mais — ele fechou entre a leitura e agora.".to_string(),
            outro => format!("não consegui abrir o processo (erro {outro})."),
        });
    }

    Ok(Alca(alca))
}

/// A máscara que este processo está usando agora.
///
/// Devolve também a máscara do SISTEMA — os núcleos que existem para ele. As
/// duas juntas são o que diz se o processo está limitado: iguais significa
/// "roda em tudo", e é o estado normal.
pub fn ler(pid: u32) -> Result<(u64, u64), String> {
    let alca = abrir(pid, PROCESS_QUERY_INFORMATION)?;

    let mut do_processo: usize = 0;
    let mut do_sistema: usize = 0;

    let ok = unsafe { GetProcessAffinityMask(alca.0, &mut do_processo, &mut do_sistema) };

    if ok == 0 {
        return Err(format!("não consegui ler a afinidade (erro {}).", unsafe {
            GetLastError()
        }));
    }

    Ok((do_processo as u64, do_sistema as u64))
}

/// Prende o processo nos núcleos da máscara.
///
/// A máscara é conferida ANTES de ser enviada: ela precisa ter pelo menos um
/// núcleo e não pode pedir nenhum que o sistema não ofereça. Mandar assim para
/// o Windows devolve um erro que não explica nada, e o cliente leria isso como
/// defeito do Otimiza.
pub fn escrever(pid: u32, mascara: u64) -> Result<(), String> {
    if mascara == 0 {
        return Err("uma máscara sem nenhum núcleo pararia o processo.".to_string());
    }

    let (_, do_sistema) = ler(pid)?;

    // O que o processo não pode usar não pode ser pedido. Conferir aqui troca
    // um erro numérico do Windows por uma frase.
    if mascara & !do_sistema != 0 {
        return Err(
            "a seleção inclui núcleo que este processo não pode usar. Em máquina com mais de \
             64 núcleos lógicos, o Windows divide os processadores em grupos e um processo só \
             enxerga o grupo dele."
                .to_string(),
        );
    }

    let alca = abrir(pid, PROCESS_SET_INFORMATION | PROCESS_QUERY_INFORMATION)?;
    let ok = unsafe { SetProcessAffinityMask(alca.0, mascara as usize) };

    if ok == 0 {
        return Err(format!("o Windows recusou a mudança (erro {}).", unsafe {
            GetLastError()
        }));
    }

    Ok(())
}

/// Solta o processo: ele volta a poder usar todos os núcleos.
///
/// É o desfazer, e ele não depende de ninguém ter anotado o valor anterior — a
/// máscara do sistema É o valor original de qualquer processo que ninguém
/// mexeu. Não há estado a guardar.
pub fn soltar(pid: u32) -> Result<(), String> {
    let (_, do_sistema) = ler(pid)?;
    escrever(pid, do_sistema)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// O processo de teste é o próprio: sempre existe, e a permissão é nossa.
    fn eu() -> u32 {
        std::process::id()
    }

    #[test]
    fn ler_a_propria_afinidade_responde() {
        let (processo, sistema) = ler(eu()).expect("ler a própria afinidade");

        println!("processo: {processo:#b}");
        println!("sistema:  {sistema:#b}");

        assert_ne!(sistema, 0, "o sistema tem de oferecer algum núcleo");
        assert_eq!(
            processo & !sistema,
            0,
            "o processo não pode estar em núcleo que o sistema não tem"
        );
    }

    /// Máscara vazia é recusada AQUI, com frase, e não pelo Windows com um
    /// número.
    #[test]
    fn mascara_vazia_e_recusada_antes_de_chegar_no_windows() {
        let erro = escrever(eu(), 0).expect_err("tem de recusar");

        assert!(erro.contains("nenhum núcleo"), "{erro}");
    }

    /// Núcleo que o sistema não oferece também é recusado aqui.
    #[test]
    fn nucleo_que_nao_existe_e_recusado_com_explicacao() {
        let (_, sistema) = ler(eu()).expect("ler");

        // Um bit que o sistema certamente não tem: o mais alto possível.
        let impossivel = 1u64 << 63;
        if sistema & impossivel != 0 {
            println!("esta máquina tem 64 lógicos; o teste não se aplica");
            return;
        }

        let erro = escrever(eu(), impossivel).expect_err("tem de recusar");
        assert!(erro.contains("não pode usar"), "{erro}");
    }

    /// Escrever e soltar, no próprio processo.
    ///
    /// Prende no núcleo 0, confere, e devolve tudo. Rodar isto no processo de
    /// teste é seguro: a afinidade some quando o processo termina, e o
    /// `soltar` no fim devolve antes disso.
    #[test]
    fn prender_e_soltar_funciona_no_proprio_processo() {
        let (original, sistema) = ler(eu()).expect("ler");

        escrever(eu(), 0b1).expect("prender no núcleo 0");
        let (depois, _) = ler(eu()).expect("reler");
        assert_eq!(depois, 0b1, "tinha de ficar só no núcleo 0");

        soltar(eu()).expect("soltar");
        let (solto, _) = ler(eu()).expect("reler");
        assert_eq!(solto, sistema, "soltar devolve todos os núcleos do sistema");

        // E devolve como estava, para não deixar o processo de teste preso
        // caso ele já viesse com afinidade de fora.
        escrever(eu(), original).expect("devolver o original");
    }
}
