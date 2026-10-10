// FOV DO FIVEM. O campo de visão em primeira pessoa é uma opção do menu do jogo (Configurações → Câmera), e o FiveM a
// guarda no próprio arquivo de configuração: `seta "profile_fpsFieldOfView" "5"` no `%APPDATA%\CitizenFX\fivem.cfg`
// (lido no PC do dono em 09/10/2026). O Otimiza troca só essa linha, com o FiveM fechado (ele regrava o arquivo inteiro
// ao sair), guarda o valor de antes e devolve exato. É a mesma opção do menu: nada que o jogo não ofereça, e servidor
// que fixa o FOV vence.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use super::contador_fivem::{com_a_linha, sem_a_linha, valor_da};

pub const CHAVE: &str = "profile_fpsFieldOfView";
/// As posições do controle do menu: 0 (mais fechado) a 10 (mais aberto). O dono tinha 5.
pub const MAXIMO: u8 = 10;

/// O que o Otimiza lembra para devolver.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Estado {
    /// A posição pedida; `None` = devolver a de antes.
    pub pedido: Option<u8>,
    /// A linha que estava no arquivo antes da primeira troca (`Some(None)` = não existia).
    pub antes: Option<Option<String>>,
    /// A troca pedida ainda não foi feita porque o FiveM estava aberto.
    pub pendente: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Painel {
    /// A posição no arquivo agora (`None` sem o arquivo ou sem a linha: o jogo usa o padrão dele).
    pub atual: Option<u8>,
    pub pedido: Option<u8>,
    pub pendente: bool,
    pub fivem_aberto: bool,
    /// O Otimiza trocou e ainda guarda o de antes.
    pub alterado: bool,
    pub maximo: u8,
}

/// Pura: a posição lida (só um número inteiro dentro do controle do menu conta).
pub fn posicao(conteudo: &str) -> Option<u8> {
    valor_da(conteudo, CHAVE)?
        .trim()
        .parse::<u8>()
        .ok()
        .filter(|v| *v <= MAXIMO)
}

/// Pura: o arquivo e o estado depois de cumprir o pedido. Sem pedido, devolve o de antes e esquece.
pub fn cumprir(conteudo: &str, mut e: Estado) -> (String, Estado) {
    let novo = match e.pedido {
        Some(v) => {
            if e.antes.is_none() {
                e.antes = Some(valor_da(conteudo, CHAVE));
            }
            com_a_linha(conteudo, CHAVE, &v.min(MAXIMO).to_string())
        }
        None => match e.antes.take() {
            Some(Some(v)) => com_a_linha(conteudo, CHAVE, &v),
            Some(None) => sem_a_linha(conteudo, CHAVE),
            None => conteudo.to_string(),
        },
    };
    e.pendente = false;
    (novo, e)
}

fn arquivo_do_estado() -> PathBuf {
    std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("pc-optimizer")
        .join("fov-fivem.json")
}

pub fn estado() -> Estado {
    fs::read_to_string(arquivo_do_estado())
        .ok()
        .and_then(|b| serde_json::from_str(&b).ok())
        .unwrap_or_default()
}

fn gravar_estado(e: &Estado) -> Result<(), String> {
    let p = arquivo_do_estado();
    if let Some(d) = p.parent() {
        fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    fs::write(&p, serde_json::to_string_pretty(e).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

#[cfg(target_os = "windows")]
pub fn painel() -> Painel {
    let e = estado();
    let atual = fs::read_to_string(super::contador_fivem::arquivo_do_fivem())
        .ok()
        .and_then(|c| posicao(&c));
    Painel {
        atual,
        pedido: e.pedido,
        pendente: e.pendente,
        fivem_aberto: super::contador_fivem::fivem_aberto(),
        alterado: e.antes.is_some(),
        maximo: MAXIMO,
    }
}

/// Pede a posição (ou `None` para devolver a de antes). Feita agora se o FiveM estiver fechado; senão fica pendente e
/// `cuidar` faz quando ele fechar.
#[cfg(target_os = "windows")]
pub fn pedir(posicao: Option<u8>) -> Result<Painel, String> {
    if posicao.is_some_and(|v| v > MAXIMO) {
        return Err(format!("O FOV vai de 0 a {}, como no menu do jogo.", MAXIMO));
    }
    let mut e = estado();
    if posicao.is_none() && e.antes.is_none() {
        // Nada trocado pelo Otimiza: não há o que devolver.
        e.pedido = None;
        e.pendente = false;
        gravar_estado(&e)?;
        return Ok(painel());
    }
    e.pedido = posicao;
    e.pendente = true;
    gravar_estado(&e)?;
    cuidar();
    Ok(painel())
}

/// Faz a troca pendente, se o FiveM estiver fechado. Chamada a cada volta da sessão (só lê o arquivo com pendência).
#[cfg(target_os = "windows")]
pub fn cuidar() {
    let e = estado();
    if !e.pendente || super::contador_fivem::fivem_aberto() {
        return;
    }
    let caminho = super::contador_fivem::arquivo_do_fivem();
    let Ok(conteudo) = fs::read_to_string(&caminho) else {
        // Sem o arquivo (FiveM nunca aberto aqui): nada a trocar.
        let _ = gravar_estado(&Estado::default());
        return;
    };
    let pedido = e.pedido;
    let (novo, depois) = cumprir(&conteudo, e);
    if novo != conteudo {
        if super::contador_fivem::gravar_arquivo_do_fivem(&caminho, &novo).is_err() {
            crate::utils::Logger::warn("FOV do FiveM: não consegui gravar o fivem.cfg");
            return;
        }
        crate::utils::Logger::info(&match pedido {
            Some(v) => format!("FOV do FiveM: {} no fivem.cfg", v),
            None => "FOV do FiveM: devolvido ao de antes".to_string(),
        });
    }
    let _ = gravar_estado(&depois);
}

/// Para o "Desfazer tudo": pede a volta do FOV de antes (na hora, ou quando o FiveM fechar).
#[cfg(target_os = "windows")]
pub fn desfazer() -> Result<Option<String>, String> {
    if estado().antes.is_none() {
        return Ok(None);
    }
    let p = pedir(None)?;
    Ok(Some(if p.pendente {
        "FOV do FiveM volta ao de antes quando o FiveM fechar.".to_string()
    } else {
        "FOV do FiveM voltou ao de antes.".to_string()
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Trecho do fivem.cfg do dono (09/10/2026), com a quebra de linha do Windows.
    const CFG: &str = "seta \"profile_fpsDefaultAimType\" \"0\"\r\nseta \"profile_fpsFieldOfView\" \"5\"\r\nseta \"profile_fpsHeadbob\" \"1\"\r\n";

    #[test]
    fn le_a_posicao_do_menu() {
        assert_eq!(posicao(CFG), Some(5));
        assert_eq!(posicao("seta \"profile_fpsFieldOfView\" \"11\"\n"), None, "fora do controle não conta");
        assert_eq!(posicao("seta \"profile_fpsHeadbob\" \"1\"\n"), None);
    }

    #[test]
    fn troca_so_a_linha_do_fov_e_guarda_a_de_antes() {
        let (novo, e) = cumprir(
            CFG,
            Estado {
                pedido: Some(10),
                ..Default::default()
            },
        );
        assert_eq!(posicao(&novo), Some(10));
        assert_eq!(e.antes, Some(Some("5".to_string())));
        assert!(!e.pendente);
        assert_eq!(
            novo.replace("\"profile_fpsFieldOfView\" \"10\"", "\"profile_fpsFieldOfView\" \"5\""),
            CFG,
            "o resto do arquivo fica byte a byte"
        );
    }

    #[test]
    fn trocar_duas_vezes_e_devolver_volta_ao_original() {
        let (a, e) = cumprir(CFG, Estado { pedido: Some(8), ..Default::default() });
        let (b, e) = cumprir(&a, Estado { pedido: Some(2), ..e });
        assert_eq!(e.antes, Some(Some("5".to_string())), "o de antes é o da primeira troca");
        let (volta, e) = cumprir(&b, Estado { pedido: None, ..e });
        assert_eq!(volta, CFG);
        assert_eq!(e.antes, None, "devolvido, nada mais a lembrar");
    }

    /// Só lê: `cargo test --lib fov_fivem::tests::na_maquina -- --ignored --nocapture`.
    #[cfg(target_os = "windows")]
    #[test]
    #[ignore]
    fn na_maquina() {
        println!("{:?}", painel());
    }

    #[test]
    fn arquivo_sem_a_linha_volta_sem_a_linha() {
        let cfg = "unbindall\nseta \"cl_drawPing\" \"true\"\n";
        let (a, e) = cumprir(cfg, Estado { pedido: Some(7), ..Default::default() });
        assert_eq!(posicao(&a), Some(7));
        let (volta, _) = cumprir(&a, Estado { pedido: None, ..e });
        assert_eq!(volta, cfg);
    }
}
