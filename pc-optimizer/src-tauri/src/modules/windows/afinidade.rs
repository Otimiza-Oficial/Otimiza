// Afinidade é do processo aberto, não configuração do Windows: fechou o jogo, acabou. Por isso não entra no
// histórico (viraria uma linha oferecendo desfazer processo que não existe mais). A máscara é conferida antes,
// para o erro do Windows virar frase.

#![cfg(target_os = "windows")]

use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE};
use windows_sys::Win32::System::Threading::{
    GetProcessAffinityMask, OpenProcess, SetProcessAffinityMask, PROCESS_QUERY_INFORMATION,
    PROCESS_SET_INFORMATION,
};

/// Alça vazada num programa aberto a partida inteira aparece horas depois, longe de onde nasceu.
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
            // 5 é acesso negado: outro usuário, mais privilégio, ou anticheat.
            5 => "o Windows negou acesso a este processo. Jogos com anticheat costumam \
                  bloquear isso, e alguns exigem que o Otimiza rode como administrador."
                .to_string(),
            // 87 aqui é processo que fechou entre listar e abrir.
            87 => "o processo não existe mais — ele fechou entre a leitura e agora.".to_string(),
            outro => format!("não consegui abrir o processo (erro {outro})."),
        });
    }

    Ok(Alca(alca))
}

/// Com a máscara do SISTEMA: as duas iguais significam "roda em tudo", o estado normal.
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

pub fn escrever(pid: u32, mascara: u64) -> Result<(), String> {
    // Nunca com anticheat rodando. Mora AQUI para valer em todo caminho: botão, Auto CPU Set e vigia.
    if let Some(recusa) = super::anticheat::permite(
        super::anticheat::Acao::AfinidadeNoJogo,
        &super::anticheat::detectar_agora(),
    )
    .motivo()
    {
        return Err(recusa.to_string());
    }

    if mascara == 0 {
        return Err("uma máscara sem nenhum núcleo pararia o processo.".to_string());
    }

    let (_, do_sistema) = ler(pid)?;

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

/// O desfazer: a máscara do sistema É o valor original de qualquer processo. Não há estado a guardar.
pub fn soltar(pid: u32) -> Result<(), String> {
    let (_, do_sistema) = ler(pid)?;
    escrever(pid, do_sistema)
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn mascara_vazia_e_recusada_antes_de_chegar_no_windows() {
        let erro = escrever(eu(), 0).expect_err("tem de recusar");

        assert!(erro.contains("nenhum núcleo"), "{erro}");
    }

    #[test]
    fn nucleo_que_nao_existe_e_recusado_com_explicacao() {
        let (_, sistema) = ler(eu()).expect("ler");

        let impossivel = 1u64 << 63;
        if sistema & impossivel != 0 {
            println!("esta máquina tem 64 lógicos; o teste não se aplica");
            return;
        }

        let erro = escrever(eu(), impossivel).expect_err("tem de recusar");
        assert!(erro.contains("não pode usar"), "{erro}");
    }

    #[test]
    fn prender_e_soltar_funciona_no_proprio_processo() {
        let (original, sistema) = ler(eu()).expect("ler");

        escrever(eu(), 0b1).expect("prender no núcleo 0");
        let (depois, _) = ler(eu()).expect("reler");
        assert_eq!(depois, 0b1, "tinha de ficar só no núcleo 0");

        soltar(eu()).expect("soltar");
        let (solto, _) = ler(eu()).expect("reler");
        assert_eq!(solto, sistema, "soltar devolve todos os núcleos do sistema");

        escrever(eu(), original).expect("devolver o original");
    }
}
