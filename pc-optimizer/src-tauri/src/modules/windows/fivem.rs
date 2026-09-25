// FiveM: o cache de servidores chega a 10 GB e é descartável (o servidor reenvia). A armadilha é `game-storage`
// (perfil e sessão da Rockstar): apagar desloga da Social Club. Cada pasta é classificada uma a uma; tamanho não
// decide nada. Não injeta nada no jogo (risco de ban) nem "libera memória" antes de abrir.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FiveMFolder {
    pub id: String,
    pub name: String,
    pub path: String,
    pub bytes: u64,
    pub formatted: String,
    pub cleanable: bool,
    pub explanation: String,
    /// Nunca é omitido.
    pub tradeoff: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FiveMReport {
    pub installed: bool,
    pub running: bool,
    pub game_running: bool,
    pub folders: Vec<FiveMFolder>,
    pub cleanable_bytes: u64,
    pub protected_bytes: u64,
    pub note: String,
}

pub fn format_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let b = bytes as f64;

    if b >= GB {
        format!("{:.1} GB", b / GB)
    } else if b >= MB {
        format!("{:.0} MB", b / MB)
    } else if b >= KB {
        format!("{:.0} KB", b / KB)
    } else {
        format!("{} B", bytes)
    }
}

pub fn pasta_do_fivem() -> Option<PathBuf> {
    let local = std::env::var("LOCALAPPDATA").ok()?;
    let caminho = PathBuf::from(local).join("FiveM").join("FiveM.app");

    caminho.is_dir().then_some(caminho)
}

/// Caminho, nome, se pode apagar, explicação e preço. Cada pasta foi aberta e olhada numa instalação real.
fn catalogo() -> Vec<(&'static str, &'static str, bool, &'static str, Option<&'static str>)> {
    vec![
        (
            "data/server-cache-priv",
            "Cache dos servidores",
            true,
            "Tudo que o FiveM baixou de cada servidor em que você entrou: mapas, scripts, \
             sons, texturas. Numa instalação com muito tempo de uso esta é, de longe, a maior \
             pasta — e nada dela é necessário, porque o servidor reenvia na próxima conexão.",
            Some(
                "Na primeira vez que você entrar em cada servidor depois da limpeza, ele vai \
                 baixar tudo de novo. Em servidor de RP grande isso pode levar vários minutos.",
            ),
        ),
        (
            "data/server-cache",
            "Cache de servidores (público)",
            true,
            "Mesma finalidade da pasta anterior, para conteúdo público de servidor.",
            Some("Também é rebaixado na próxima conexão."),
        ),
        (
            "data/cache",
            "Cache interno do FiveM",
            true,
            "Arquivos temporários do próprio cliente: lista de servidores, atalhos de \
             inicialização e dados de subprocesso. O FiveM refaz sozinho.",
            None,
        ),
        (
            "crashes",
            "Relatórios de travamento",
            true,
            "Despejos de memória gravados quando o jogo fechou sozinho. Servem para o \
             desenvolvedor investigar e não têm utilidade depois disso.",
            None,
        ),
        (
            "logs",
            "Registros de execução",
            true,
            "Histórico do que o cliente fez em cada sessão. Só serve para diagnóstico.",
            None,
        ),
        (
            "data/game-storage",
            "Perfil do jogo e sessão da Rockstar",
            false,
            "Parece cache pelo tamanho, e não é. Aqui ficam a sua configuração do jogo, o \
             perfil do jogador e os dados de sessão da Rockstar Social Club. Apagar desloga \
             você da sua conta e joga fora os seus ajustes. O Otimiza mede e mostra, e não \
             oferece limpeza.",
            None,
        ),
        (
            "data/nui-storage",
            "Dados das interfaces dos servidores",
            false,
            "Armazenamento das telas que os servidores desenham dentro do jogo — inventário, \
             celular, menus. Vários servidores guardam preferência de personagem aqui. Apagar \
             não quebra o jogo, mas apaga essas escolhas.",
            None,
        ),
        (
            "citizen",
            "Arquivos do cliente",
            false,
            "O próprio FiveM. Apagar exige reinstalar.",
            None,
        ),
        (
            "plugins",
            "Complementos",
            false,
            "Complementos instalados por você ou por servidores.",
            None,
        ),
        (
            "mods",
            "Modificações locais",
            false,
            "Modificações que você mesmo colocou.",
            None,
        ),
    ]
}

fn somar(dir: &Path) -> u64 {
    let Ok(entradas) = std::fs::read_dir(dir) else {
        return 0;
    };

    entradas
        .flatten()
        .filter_map(|e| {
            let meta = e.metadata().ok()?;

            if meta.is_dir() {
                Some(somar(&e.path()))
            } else {
                Some(meta.len())
            }
        })
        .sum()
}

/// O jogo tem processo próprio com o número da compilação no nome: comparação por prefixo.
pub fn processos_abertos() -> (bool, bool) {
    use sysinfo::System;

    let mut sistema = System::new();
    sistema.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    let mut lancador = false;
    let mut jogo = false;

    for processo in sistema.processes().values() {
        let nome = processo.name().to_string_lossy().to_lowercase();

        if !nome.starts_with("fivem") {
            continue;
        }

        lancador = true;

        if nome.contains("gtaprocess") {
            jogo = true;
        }
    }

    (lancador, jogo)
}

pub fn montar_nota(limpavel: u64, protegido: u64, aberto: bool, instalado: bool) -> String {
    if !instalado {
        return "O FiveM não está instalado nesta máquina.".to_string();
    }

    let mut nota = String::new();

    if limpavel > 0 {
        nota.push_str(&format!(
            "{} podem ser recuperados. A maior parte é conteúdo que os servidores reenviam \
             sozinhos na próxima vez que você entrar. ",
            format_size(limpavel)
        ));
    }

    if protegido > 0 {
        nota.push_str(&format!(
            "Outros {} aparecem na lista como protegidos: são o seu perfil do jogo, a sua \
             sessão da Rockstar e as preferências que os servidores guardaram. Parece cache \
             pelo tamanho e não é — apagar desloga você da conta e apaga seus ajustes. ",
            format_size(protegido)
        ));
    }

    if aberto {
        nota.push_str("Feche o FiveM por completo antes de limpar: com ele aberto os arquivos ficam travados.");
    }

    if nota.is_empty() {
        nota.push_str("Nada a recuperar na instalação do FiveM.");
    }

    nota
}

pub fn analyze() -> FiveMReport {
    let Some(base) = pasta_do_fivem() else {
        return FiveMReport {
            installed: false,
            running: false,
            game_running: false,
            folders: Vec::new(),
            cleanable_bytes: 0,
            protected_bytes: 0,
            note: montar_nota(0, 0, false, false),
        };
    };

    let (running, game_running) = processos_abertos();

    let mut folders = Vec::new();
    let mut cleanable_bytes = 0u64;
    let mut protected_bytes = 0u64;

    for (relativo, nome, cleanable, explicacao, tradeoff) in catalogo() {
        let caminho = base.join(relativo.replace('/', "\\"));

        if !caminho.is_dir() {
            continue;
        }

        let bytes = somar(&caminho);

        if bytes == 0 && cleanable {
            continue;
        }

        if cleanable {
            cleanable_bytes += bytes;
        } else {
            protected_bytes += bytes;
        }

        folders.push(FiveMFolder {
            id: relativo.to_string(),
            name: nome.to_string(),
            path: caminho.to_string_lossy().to_string(),
            formatted: format_size(bytes),
            bytes,
            cleanable,
            explanation: explicacao.to_string(),
            tradeoff: tradeoff.map(|t| t.to_string()),
        });
    }

    folders.sort_by(|a, b| b.cleanable.cmp(&a.cleanable).then(b.bytes.cmp(&a.bytes)));

    FiveMReport {
        note: montar_nota(cleanable_bytes, protected_bytes, running, true),
        installed: true,
        running,
        game_running,
        folders,
        cleanable_bytes,
        protected_bytes,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanOutcome {
    pub freed_mb: f64,
    pub message: String,
}

/// Três travas: no catálogo e descartável, FiveM fechado, e só o conteúdo é apagado.
pub fn limpar(id: &str) -> Result<CleanOutcome, String> {
    let base = pasta_do_fivem().ok_or("O FiveM não está instalado nesta máquina.")?;

    // Refeita aqui: o comando vem por IPC, e tela com dado velho não pode apagar o perfil de alguém.
    let (relativo, nome, cleanable, _, _) = catalogo()
        .into_iter()
        .find(|(r, _, _, _, _)| *r == id)
        .ok_or_else(|| format!("`{}` não é uma pasta conhecida do FiveM.", id))?;

    if !cleanable {
        return Err(format!(
            "`{}` guarda dados seus e não é apagada pelo Otimiza.",
            nome
        ));
    }

    let (running, _) = processos_abertos();
    if running {
        return Err(
            "O FiveM está aberto. Feche-o por completo antes de limpar — com ele rodando os \
             arquivos ficam travados e a limpeza sairia pela metade."
                .to_string(),
        );
    }

    let caminho = base.join(relativo.replace('/', "\\"));
    let antes = somar(&caminho);

    esvaziar(&caminho);

    // O que se relata é o que sumiu de verdade.
    let liberado = antes.saturating_sub(somar(&caminho));

    Ok(CleanOutcome {
        freed_mb: liberado as f64 / 1_048_576.0,
        message: format!("{} liberados de {}.", format_size(liberado), nome),
    })
}

fn esvaziar(dir: &Path) {
    let Ok(entradas) = std::fs::read_dir(dir) else {
        return;
    };

    for entrada in entradas.flatten() {
        let caminho = entrada.path();

        let _ = if caminho.is_dir() {
            std::fs::remove_dir_all(&caminho)
        } else {
            std::fs::remove_file(&caminho)
        };
    }
}

// `priorizar_jogo` saiu na 2.9: prioridade cega, sem ganho medido, e a ação mais visível para um anticheat.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perfil_do_jogo_nunca_e_apagavel() {
        let protegidas = ["data/game-storage", "data/nui-storage", "citizen", "plugins", "mods"];

        for id in protegidas {
            let (_, _, cleanable, explicacao, _) = catalogo()
                .into_iter()
                .find(|(r, _, _, _, _)| *r == id)
                .unwrap_or_else(|| panic!("`{}` sumiu do catálogo", id));

            assert!(!cleanable, "`{}` foi marcada como apagável", id);
            assert!(!explicacao.is_empty(), "`{}` sem explicação", id);
        }
    }

    #[test]
    fn cache_de_servidor_e_apagavel_e_declara_o_preco() {
        let (_, _, cleanable, _, tradeoff) = catalogo()
            .into_iter()
            .find(|(r, _, _, _, _)| *r == "data/server-cache-priv")
            .unwrap();

        assert!(cleanable);
        let preco = tradeoff.expect("a maior limpeza do módulo precisa declarar o preço");
        assert!(preco.contains("baixar tudo de novo"));
    }

    #[test]
    fn limpar_recusa_pasta_protegida() {
        let erro = limpar("data/game-storage").unwrap_err();
        assert!(
            erro.contains("guarda dados seus") || erro.contains("não está instalado"),
            "recusa inesperada: {}",
            erro
        );
    }

    #[test]
    fn limpar_recusa_pasta_desconhecida() {
        let erro = limpar("data/../../../Windows").unwrap_err();
        assert!(
            erro.contains("não é uma pasta conhecida") || erro.contains("não está instalado"),
            "recusa inesperada: {}",
            erro
        );
    }

    #[test]
    fn nota_explica_o_protegido_em_vez_de_so_listar() {
        let nota = montar_nota(10_737_418_240, 3_407_872_000, true, true);

        assert!(nota.contains("10.0 GB"));
        assert!(nota.contains("protegidos"));
        assert!(nota.contains("desloga você da conta"));
        assert!(nota.contains("Feche o FiveM"));
    }

    #[test]
    fn sem_fivem_a_nota_diz_isso() {
        let nota = montar_nota(0, 0, false, false);
        assert!(nota.contains("não está instalado"));
    }

    #[test]
    fn tamanho_sai_na_unidade_certa() {
        assert_eq!(format_size(2048), "2 KB");
        assert_eq!(format_size(5 * 1024 * 1024), "5 MB");
        assert_eq!(format_size(3 * 1024 * 1024 * 1024), "3.0 GB");
    }

    /// Sem este corte os guards se acusariam: os nomes proibidos aparecem nas próprias asserções.
    fn codigo_de_producao() -> &'static str {
        let fonte = include_str!("fivem.rs");
        fonte.split("#[cfg(test)]").next().unwrap()
    }

    #[test]
    fn o_fivem_nao_mexe_mais_na_prioridade_do_jogo() {
        let fonte = codigo_de_producao();

        for proibido in ["PriorityClass = 'High'", "PriorityClass = 'RealTime'"] {
            assert!(!fonte.contains(proibido), "`{}` voltou ao módulo do FiveM", proibido);
        }
    }

    #[test]
    fn nao_ha_injecao_nem_liberacao_de_memoria() {
        let fonte = codigo_de_producao();

        for proibido in ["EmptyWorkingSet", "SetWindowsHookEx", "CreateRemoteThread"] {
            assert!(
                !fonte.contains(proibido),
                "`{}` apareceu no módulo do FiveM",
                proibido
            );
        }
    }

    #[test]
    fn analisa_esta_maquina() {
        let r = analyze();

        println!("instalado: {} | aberto: {}", r.installed, r.running);
        println!("nota: {}", r.note);

        for f in &r.folders {
            println!(
                "  [{}] {:>9}  {}",
                if f.cleanable { "limpa " } else { "PROTEG" },
                f.formatted,
                f.name
            );
        }

        assert!(!r.note.is_empty());

        if r.installed {
            for f in &r.folders {
                assert!(!f.explanation.is_empty(), "{} sem explicação", f.name);
            }

            let primeira_protegida = r.folders.iter().position(|f| !f.cleanable);
            let ultima_limpavel = r.folders.iter().rposition(|f| f.cleanable);

            if let (Some(p), Some(l)) = (primeira_protegida, ultima_limpavel) {
                assert!(p > l, "pasta protegida apareceu antes de uma apagável");
            }
        }
    }
}
