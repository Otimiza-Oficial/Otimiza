// A PARTIDA PELO OTIMIZA (Plano "Partida de respeito", fase 2). O botão "Jogar" abre o jogo e, quando o vigia de sessão
// (`lib.rs`) vê ESSE jogo abrir, liga a receita que o Turbo já lembra dele. Abrindo por aqui, o que só vale com o jogo
// aberto DEPOIS de aplicar (o teto do driver, a base do Frame Boost) já vale nesta partida.
//
// O jogo abre pelo `explorer.exe`, nunca como filho do Otimiza: o Otimiza costuma rodar como administrador (o PresentMon
// exige), e o jogo herdaria isso. Jogo elevado não recebe a sobreposição da Steam nem do Discord, e alguns anticheats
// recusam. Pelo Explorer ele abre como a pessoa, igual a um clique no atalho.
//
// Só abre e lembra o pedido: aplicar é com o vigia, que já sabe qual processo é o jogo.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::modules::windows::jogos::{JogoInstalado, Origem};

/// Um jogo grande (FiveM baixando atualização, Steam abrindo pela primeira vez) leva minutos até a janela aparecer.
const VALIDADE_DO_PEDIDO: Duration = Duration::from_secs(8 * 60);

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
pub enum Caminho {
    /// `steam://rungameid/<appid>`: a Steam abre com as opções de lançamento da pessoa.
    Steam,
    /// O executável que a loja registrou.
    Executavel,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct Lancamento {
    pub jogo: String,
    pub caminho: Caminho,
}

struct Pedido {
    chave: String,
    executavel: Option<String>,
    quando: Instant,
}

static PEDIDO: Mutex<Option<Pedido>> = Mutex::new(None);

/// **Pura.** O mesmo jogo com nome de outra fonte vira a mesma chave ("Roblox Player" e "Roblox"), como na tela
/// (`ui/src/lib/capas.ts`, `chaveDoJogo`).
pub fn chave(nome: &str) -> String {
    let c: String = nome.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_ascii_lowercase();
    match c.as_str() {
        "robloxplayer" | "robloxplayerbeta" => "roblox".to_string(),
        "fivemclient" | "citizenfxfivem" => "fivem".to_string(),
        _ => c,
    }
}

/// **Pura.** Como abrir este jogo: a Steam quando ele é dela (respeita as opções de lançamento), senão o executável.
pub fn caminho(j: &JogoInstalado) -> Option<(Caminho, String)> {
    match (&j.origem, j.appid, &j.executavel) {
        (Origem::Steam, Some(appid), _) => Some((Caminho::Steam, format!("steam://rungameid/{}", appid))),
        (_, _, Some(exe)) => Some((Caminho::Executavel, exe.to_string_lossy().to_string())),
        _ => None,
    }
}

/// **Pura.** O jogo que abriu é o que o "Jogar" pediu? Pelo executável quando a loja o deu (o mais certo), senão pelo
/// nome. Pedido vencido não conta: quem abriu o jogo uma hora depois abriu por conta própria.
pub fn e_o_pedido(
    pedido_chave: &str,
    pedido_exe: Option<&str>,
    idade: Duration,
    nome_aberto: &str,
    exe_aberto: &str,
) -> bool {
    if idade > VALIDADE_DO_PEDIDO {
        return false;
    }
    let arquivo = |c: &str| {
        std::path::Path::new(c)
            .file_name()
            .map(|n| n.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default()
    };
    if let Some(exe) = pedido_exe {
        if !exe.is_empty() && arquivo(exe) == arquivo(exe_aberto) {
            return true;
        }
    }
    let aberto = chave(nome_aberto);
    !pedido_chave.is_empty() && (aberto == pedido_chave || aberto.contains(pedido_chave) || pedido_chave.contains(&aberto))
}

/// Abre o jogo de nome `nome` (o da lista de instalados) e guarda o pedido.
#[cfg(target_os = "windows")]
pub fn lancar(nome: &str) -> Result<Lancamento, String> {
    let biblioteca = crate::modules::windows::jogos::varrer();
    let procurado = chave(nome);
    let jogo = biblioteca
        .jogos
        .iter()
        .find(|j| j.nome.eq_ignore_ascii_case(nome))
        .or_else(|| biblioteca.jogos.iter().find(|j| chave(&j.nome) == procurado))
        .ok_or_else(|| format!("Não achei o {} entre os jogos instalados.", nome))?;
    let (como, alvo) = caminho(jogo).ok_or_else(|| {
        format!("Não sei como abrir o {} por aqui: abra pelo atalho dele. O Otimiza reconhece o jogo do mesmo jeito.", jogo.nome)
    })?;

    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    std::process::Command::new("explorer.exe")
        .arg(&alvo)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| format!("O Windows não abriu o {}: {}", jogo.nome, e))?;

    *PEDIDO.lock().unwrap_or_else(|e| e.into_inner()) = Some(Pedido {
        chave: chave(&jogo.nome),
        executavel: jogo.executavel.as_ref().map(|p| p.to_string_lossy().to_string()),
        quando: Instant::now(),
    });
    crate::utils::Logger::info(&format!("jogar: abrindo {} ({:?})", jogo.nome, como));
    Ok(Lancamento { jogo: jogo.nome.clone(), caminho: como })
}

/// O jogo que acabou de abrir veio do "Jogar"? Se veio, o pedido é consumido: a receita liga uma vez por partida.
pub fn tomar_se_for(nome_aberto: &str, exe_aberto: &str) -> bool {
    let mut guarda = PEDIDO.lock().unwrap_or_else(|e| e.into_inner());
    let Some(p) = guarda.as_ref() else {
        return false;
    };
    let veio = e_o_pedido(&p.chave, p.executavel.as_deref(), p.quando.elapsed(), nome_aberto, exe_aberto);
    if veio || p.quando.elapsed() > VALIDADE_DO_PEDIDO {
        *guarda = None;
    }
    veio
}

/// O que o vigia conta para a tela quando o jogo que o "Jogar" abriu aparece (`partida:preparada`).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct Preparada {
    pub jogo: String,
    /// Os passos que o Turbo aplicou, como ele mesmo descreve.
    pub passos: Vec<String>,
    /// Sem receita guardada para este jogo: o Turbo ainda não mediu ele.
    pub sem_receita: bool,
    /// O Turbo não ligou (prova rodando, receita reprovada, sem licença): a frase dele.
    pub recusa: Option<String>,
}

/// O que o vigia conta quando o jogo fecha (`partida:terminou`): a tela monta o cartão da partida com o relatório.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct Terminada {
    pub jogo: String,
    pub executavel: String,
    /// O caminho do executável, lido quando o jogo abriu (o perfil do driver pode ser pelo caminho, como no FiveM).
    pub caminho: Option<String>,
    /// Do jogo aberto ao fechado, como o vigia viu (a cada 4 s).
    pub segundos: u64,
    pub pelo_jogar: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn jogo(origem: Origem, appid: Option<u32>, exe: Option<&str>) -> JogoInstalado {
        JogoInstalado {
            nome: "Jogo".into(),
            origem,
            pasta: PathBuf::new(),
            executavel: exe.map(PathBuf::from),
            ultima_vez: 0,
            bytes: 0,
            appid,
        }
    }

    #[test]
    fn steam_abre_pela_steam_e_o_resto_pelo_executavel() {
        assert_eq!(
            caminho(&jogo(Origem::Steam, Some(730), Some(r"C:\cs2.exe"))),
            Some((Caminho::Steam, "steam://rungameid/730".into()))
        );
        assert_eq!(
            caminho(&jogo(Origem::Instalado, None, Some(r"C:\FiveM\FiveM.exe"))),
            Some((Caminho::Executavel, r"C:\FiveM\FiveM.exe".into()))
        );
        // Steam sem número: o executável, se houver.
        assert_eq!(caminho(&jogo(Origem::Steam, None, None)), None);
    }

    #[test]
    fn o_jogo_que_abriu_e_o_pedido_pelo_executavel_ou_pelo_nome() {
        let pouco = Duration::from_secs(30);
        assert!(e_o_pedido("fivem", Some(r"C:\Users\x\AppData\Local\FiveM\FiveM.exe"), pouco, "FiveM", r"D:\outro\FiveM.exe"));
        // O FiveM abre o processo do jogo com outro nome: o nome do jogo ainda casa.
        assert!(e_o_pedido("fivem", Some(r"C:\FiveM\FiveM.exe"), pouco, "FiveM", r"C:\FiveM\FiveM_GTAProcess.exe"));
        assert!(e_o_pedido("roblox", None, pouco, "Roblox Player", r"C:\Roblox\RobloxPlayerBeta.exe"));
        assert!(!e_o_pedido("fivem", None, pouco, "Counter-Strike 2", r"C:\cs2.exe"));
        // Uma hora depois não é mais o pedido.
        assert!(!e_o_pedido("fivem", None, Duration::from_secs(3600), "FiveM", r"C:\FiveM.exe"));
    }

    #[test]
    fn a_chave_junta_os_nomes_do_mesmo_jogo() {
        assert_eq!(chave("Roblox Player"), "roblox");
        assert_eq!(chave("Grand Theft Auto V"), "grandtheftautov");
    }
}
