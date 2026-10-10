// O contador do Otimiza no lugar do do FiveM, sem tocar no jogo: o FiveM guarda o `cl_drawFPS` no arquivo de
// configuração dele (`%APPDATA%\CitizenFX\fivem.cfg`, linha `seta "cl_drawFPS" "true"`). O Otimiza troca só essa
// linha, guarda o valor de antes para devolver, e NUNCA mexe com o FiveM aberto: o FiveM regrava o arquivo inteiro ao
// fechar, com o que estiver na memória dele, e a mudança se perderia. Com o jogo aberto, espera fechar.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const CHAVE: &str = "cl_drawFPS";

/// Pura: a linha é `seta "<chave>" ...`? (Sem diferenciar maiúsculas: o FiveM grava `cl_drawFPS`.)
fn e_a_linha(linha: &str, chave: &str) -> bool {
    let mut p = linha.split('"');
    let comando = p.next().unwrap_or("").trim();
    let nome = p.next().unwrap_or("");
    comando.eq_ignore_ascii_case("seta") && nome.eq_ignore_ascii_case(chave)
}

fn quebra_de(conteudo: &str) -> &'static str {
    if conteudo.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

/// Pura: o valor de uma variável `seta` do `fivem.cfg` (`None` se a linha não existe).
pub fn valor_da(conteudo: &str, chave: &str) -> Option<String> {
    conteudo.lines().find_map(|l| {
        let mut partes = l.split('"');
        partes.next()?;
        partes.next()?;
        partes.next()?;
        let valor = partes.next()?;
        e_a_linha(l, chave).then(|| valor.to_string())
    })
}

/// Pura: o arquivo com a variável no valor pedido. Troca só a linha dela (mantendo o jeito de quebrar linha do
/// arquivo); sem a linha, acrescenta no fim.
pub fn com_a_linha(conteudo: &str, chave: &str, valor: &str) -> String {
    let quebra = quebra_de(conteudo);
    let nova = format!("seta \"{}\" \"{}\"", chave, valor);
    let mut achou = false;
    let mut linhas: Vec<String> = conteudo
        .split(quebra)
        .map(|l| {
            if e_a_linha(l, chave) && !achou {
                achou = true;
                nova.clone()
            } else {
                l.to_string()
            }
        })
        .collect();
    if !achou {
        // Antes da última linha vazia, se o arquivo termina com quebra.
        if linhas.last().is_some_and(|l| l.is_empty()) {
            let ultima = linhas.len() - 1;
            linhas.insert(ultima, nova);
        } else {
            linhas.push(nova);
        }
    }
    linhas.join(quebra)
}

/// Pura: o arquivo sem a linha da variável (para devolver um arquivo que não a tinha).
pub fn sem_a_linha(conteudo: &str, chave: &str) -> String {
    let quebra = quebra_de(conteudo);
    conteudo
        .split(quebra)
        .filter(|l| !e_a_linha(l, chave))
        .collect::<Vec<_>>()
        .join(quebra)
}

/// Pura: o valor de `cl_drawFPS` no arquivo (`None` se a linha não existe).
pub fn valor_atual(conteudo: &str) -> Option<String> {
    valor_da(conteudo, CHAVE)
}

/// Pura: o arquivo com `cl_drawFPS` no valor pedido.
pub fn com_valor(conteudo: &str, valor: &str) -> String {
    com_a_linha(conteudo, CHAVE, valor)
}

/// Pura: o arquivo sem a linha de `cl_drawFPS`.
pub fn sem_linha(conteudo: &str) -> String {
    sem_a_linha(conteudo, CHAVE)
}

/// O que o Otimiza lembra: se a pessoa quer o contador dele no lugar do do FiveM e o valor de antes, para devolver.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Estado {
    pub ligado: bool,
    /// O `cl_drawFPS` que estava no arquivo antes da primeira troca (`None` = a linha não existia).
    pub antes: Option<Option<String>>,
    /// A troca pedida ainda não foi feita porque o FiveM estava aberto.
    pub pendente: bool,
}

pub(crate) fn arquivo_do_fivem() -> PathBuf {
    std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("CitizenFX")
        .join("fivem.cfg")
}

fn arquivo_do_estado() -> PathBuf {
    std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("pc-optimizer")
        .join("contador-fivem.json")
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
    fs::write(
        &p,
        serde_json::to_string_pretty(e).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

/// O FiveM está aberto? (Qualquer processo `FiveM*`: o lançador também regrava o arquivo.)
#[cfg(target_os = "windows")]
pub(crate) fn fivem_aberto() -> bool {
    use sysinfo::{ProcessesToUpdate, System};
    let mut s = System::new();
    s.refresh_processes(ProcessesToUpdate::All, true);
    let aberto = s.processes().values().any(|p| {
        p.name()
            .to_string_lossy()
            .to_lowercase()
            .starts_with("fivem")
    });
    aberto
}

/// Pede a troca (ligar = contador do Otimiza; desligar = devolve o do FiveM). Feita agora se o FiveM estiver
/// fechado; senão fica pendente e `cuidar` faz quando ele fechar.
#[cfg(target_os = "windows")]
pub fn pedir(ligar: bool) -> Result<Estado, String> {
    let mut e = estado();
    e.ligado = ligar;
    e.pendente = true;
    gravar_estado(&e)?;
    cuidar();
    Ok(estado())
}

/// Faz a troca pendente, se o FiveM estiver fechado. Chamada a cada volta da sessão (barato: lê um arquivo só quando
/// há algo pendente).
#[cfg(target_os = "windows")]
pub fn cuidar() {
    let mut e = estado();
    if !e.pendente || fivem_aberto() {
        return;
    }
    let caminho = arquivo_do_fivem();
    let Ok(conteudo) = fs::read_to_string(&caminho) else {
        // Sem o arquivo (FiveM nunca aberto aqui): nada a trocar.
        e.pendente = false;
        let _ = gravar_estado(&e);
        return;
    };
    let novo = if e.ligado {
        if e.antes.is_none() {
            e.antes = Some(valor_atual(&conteudo));
        }
        com_valor(&conteudo, "false")
    } else {
        match e.antes.take() {
            Some(Some(v)) => com_valor(&conteudo, &v),
            // A linha não existia: tira a que o Otimiza pôs.
            Some(None) => sem_linha(&conteudo),
            None => conteudo.clone(),
        }
    };
    if novo != conteudo {
        if gravar_arquivo_do_fivem(&caminho, &novo).is_err() {
            crate::utils::Logger::warn("contador do FiveM: não consegui gravar o fivem.cfg");
            return;
        }
        crate::utils::Logger::info(&format!(
            "contador do FiveM: cl_drawFPS {} no fivem.cfg",
            if e.ligado {
                "desligado (vale o do Otimiza)"
            } else {
                "devolvido"
            }
        ));
    }
    e.pendente = false;
    let _ = gravar_estado(&e);
}

/// Grava por um arquivo ao lado e troca de uma vez: o FiveM nunca lê um arquivo pela metade.
pub(crate) fn gravar_arquivo_do_fivem(caminho: &std::path::Path, novo: &str) -> std::io::Result<()> {
    let tmp = caminho.with_extension("cfg.otimiza");
    fs::write(&tmp, novo).and_then(|_| fs::rename(&tmp, caminho))
}

#[cfg(test)]
mod tests {
    use super::*;

    const CFG: &str = "// generated by CitizenFX\r\nunbindall\r\nseta \"cl_discordRichPresence\" \"enabled\"\r\nseta \"cl_drawCpuUsage\" \"true\"\r\nseta \"cl_drawFPS\" \"true\"\r\nseta \"cl_drawGpuMemoryUsage\" \"true\"\r\n";

    #[test]
    fn le_o_valor_de_cl_drawfps() {
        assert_eq!(valor_atual(CFG), Some("true".to_string()));
        assert_eq!(
            valor_atual("seta \"cl_drawPerf\" \"true\"\n"),
            None,
            "outra variável não conta"
        );
    }

    #[test]
    fn troca_so_a_linha_dele_e_mantem_o_resto_igual() {
        let novo = com_valor(CFG, "false");
        assert_eq!(valor_atual(&novo), Some("false".to_string()));
        assert_eq!(
            novo.replace(
                "seta \"cl_drawFPS\" \"false\"",
                "seta \"cl_drawFPS\" \"true\""
            ),
            CFG
        );
        assert!(novo.contains("\r\n"), "mantém o CRLF do arquivo");
    }

    #[test]
    fn sem_a_linha_acrescenta_no_fim() {
        let cfg = "unbindall\nseta \"cl_drawPing\" \"true\"\n";
        let novo = com_valor(cfg, "false");
        assert_eq!(
            novo,
            "unbindall\nseta \"cl_drawPing\" \"true\"\nseta \"cl_drawFPS\" \"false\"\n"
        );
    }

    #[test]
    fn arquivo_sem_a_linha_volta_sem_a_linha() {
        let cfg = "unbindall\nseta \"cl_drawPing\" \"true\"\n";
        assert_eq!(sem_linha(&com_valor(cfg, "false")), cfg);
    }

    #[test]
    fn devolver_o_valor_de_antes_e_a_volta_exata() {
        let ida = com_valor(CFG, "false");
        let volta = com_valor(&ida, &valor_atual(CFG).unwrap());
        assert_eq!(volta, CFG);
    }
}
