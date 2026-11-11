// O TRÁFEGO DO JOGO MARCADO COMO PRIORIDADE (Plano de refino, R5), como experimento. Com o interruptor ligado, o jogo
// aberto ganha uma política QoS do Windows que marca os pacotes dele com DSCP 46 ("encaminhamento expresso", a marca
// de voz e jogo). Roteador que respeita a marca (com QoS/SQM ligado) passa o jogo na frente do download; a maioria
// dos roteadores de operadora ignora, e a tela diz isso. Não muda nada no PC além da marca.
//
// A política fica só na memória do Windows (`ActiveStore`): some ao reiniciar. O Otimiza cria quando o jogo abre e
// tira quando fecha, e limpa resto na abertura. Nome com o prefixo `OtimizaJogo-`, diferente do `Otimiza-` da banda
// de fundo (`banda.rs`), que remove as dela e não pode levar esta junto.

/// Prefixo das políticas daqui.
pub const PREFIXO: &str = "OtimizaJogo-";
/// Encaminhamento expresso (RFC 3246).
pub const DSCP: u8 = 46;

/// **Pura.** O nome da política do executável: só letras, números, ponto e hífen.
pub fn nome_da_politica(exe: &str) -> String {
    let limpo: String = exe
        .to_lowercase()
        .trim_end_matches(".exe")
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '-' { c } else { '-' })
        .collect();
    format!("{}{}", PREFIXO, limpo)
}

/// **Pura.** O executável só passa se for um nome de arquivo comum (vai entre aspas no PowerShell).
pub fn exe_valido(exe: &str) -> bool {
    !exe.is_empty()
        && exe.len() <= 80
        && exe.to_lowercase().ends_with(".exe")
        && exe.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | ' '))
}

#[cfg(target_os = "windows")]
mod windows {
    use super::*;
    use crate::modules::windows::shell;

    fn aspas(s: &str) -> String {
        format!("'{}'", s.replace('\'', "''"))
    }

    pub fn ligado() -> bool {
        super::super::flags::Flags::load().prioridade_na_rede
    }

    /// O jogo abriu: marca o tráfego dele (se o interruptor estiver ligado e o Otimiza for administrador).
    pub fn ao_abrir(exe: &str) {
        if !ligado() || !exe_valido(exe) || !crate::modules::windows::registry::is_elevated() {
            return;
        }
        let script = format!(
            "New-NetQosPolicy -Name {} -AppPathNameMatchCondition {} -DSCPAction {} -NetworkProfile All -PolicyStore ActiveStore -ErrorAction Stop | Out-Null",
            aspas(&nome_da_politica(exe)),
            aspas(exe),
            DSCP
        );
        match shell::powershell_checked(&script) {
            Ok(_) => crate::utils::Logger::info(&format!("prioridade na rede: {} marcado com DSCP {}", exe, DSCP)),
            Err(e) => crate::utils::Logger::warn(&format!("prioridade na rede: não marquei {}: {}", exe, e)),
        }
    }

    /// Tira todas as políticas daqui (jogo fechou, interruptor desligado, abertura do Otimiza).
    pub fn limpar() {
        if !crate::modules::windows::registry::is_elevated() {
            return;
        }
        let script = format!(
            "Get-NetQosPolicy -PolicyStore ActiveStore -ErrorAction SilentlyContinue | Where-Object {{ $_.Name -like {} }} | Remove-NetQosPolicy -PolicyStore ActiveStore -Confirm:$false -ErrorAction SilentlyContinue",
            aspas(&format!("{}*", PREFIXO))
        );
        let _ = shell::powershell(&script);
    }

    /// Liga ou desliga. Ligando com o jogo aberto, já marca; desligando, tira na hora.
    pub fn definir(ligar: bool) -> Result<bool, String> {
        let mut f = super::super::flags::Flags::load();
        f.prioridade_na_rede = ligar;
        f.save()?;
        if ligar {
            if let Some(j) = super::super::sessao::atual() {
                ao_abrir(&j.executavel);
            }
        } else {
            limpar();
        }
        Ok(ligar)
    }
}

#[cfg(target_os = "windows")]
pub use windows::{ao_abrir, definir, ligado, limpar};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn o_nome_nao_colide_com_a_banda_de_fundo() {
        let n = nome_da_politica("FiveM_GTAProcess.exe");
        assert_eq!(n, "OtimizaJogo-fivem-gtaprocess");
        assert!(!crate::plataforma::banda::e_nossa(&n), "a limpeza da banda levaria esta junto");
    }

    #[test]
    fn so_executavel_comum_vira_politica() {
        assert!(exe_valido("cs2.exe"));
        assert!(exe_valido("FiveM_GTAProcess.exe"));
        assert!(!exe_valido("a';Remove-Item C:\\.exe"));
        assert!(!exe_valido("jogo.bat"));
    }
}
