// FiveM
//
// O público que motivou este módulo joga FiveM, e o problema dele não é o
// mesmo do PC de escritório lento: é travada no meio da partida, crash ao
// entrar num servidor, e disco cheio sem explicação.
//
// A causa mais comum das três é a mesma, e é medível: o FiveM guarda em disco
// tudo que baixa de cada servidor em que você entra. Numa instalação real
// medida durante o desenvolvimento, essa pasta tinha 10 GB divididos em 21.584
// arquivos. Nada disso é necessário — o servidor manda de novo na próxima
// conexão.
//
// A ARMADILHA, QUE É A MESMA DO NAVEGADOR
//
// A segunda maior pasta da instalação NÃO pode ser apagada, e é justamente o
// alvo óbvio de quem varre por tamanho. `game-storage` tinha 3,2 GB e guarda o
// perfil do jogo e os dados de sessão da Rockstar. Apagar desloga a pessoa da
// Social Club e joga fora a configuração do jogo dela.
//
// Por isso aqui, como no resto do produto, cada pasta é classificada uma a uma,
// com o motivo escrito. Tamanho não decide nada.
//
// O QUE ESTE MÓDULO NÃO FAZ
//
// Não injeta nada no jogo, e isso não é limitação técnica — é decisão. Overlay
// de FPS dentro do FiveM exige injetar código no processo, e o anticheat trata
// injeção como ameaça. O ganho seria um número bonito na tela; o risco é o
// cliente tomar ban na conta dele.
//
// Também não "libera memória" antes de abrir o jogo. Esvaziar o conjunto de
// trabalho dos processos melhora o gráfico e piora o desempenho real, porque
// tudo precisa ser relido do disco. Já está na lista de recusas do produto, e
// não passa a valer só porque o assunto agora é jogo.

use super::shell;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Uma pasta da instalação, classificada.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FiveMFolder {
    pub id: String,
    pub name: String,
    pub path: String,
    pub bytes: u64,
    pub formatted: String,
    /// Se pode ser apagada com segurança.
    pub cleanable: bool,
    pub explanation: String,
    /// O preço de apagar, quando existe. Nunca é omitido.
    pub tradeoff: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FiveMReport {
    pub installed: bool,
    /// O FiveM está aberto agora. Com ele aberto não dá para apagar nada.
    pub running: bool,
    /// O jogo em si, e não só o lançador.
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

/// Onde o FiveM instala.
pub fn pasta_do_fivem() -> Option<PathBuf> {
    let local = std::env::var("LOCALAPPDATA").ok()?;
    let caminho = PathBuf::from(local).join("FiveM").join("FiveM.app");

    caminho.is_dir().then_some(caminho)
}

/// Classificação de cada pasta conhecida da instalação.
///
/// A ordem dos campos é: caminho relativo, nome visível, se pode apagar,
/// explicação, e o preço de apagar.
///
/// Nada aqui foi deduzido pelo nome da pasta. Cada uma foi aberta e olhada numa
/// instalação real antes de ser classificada.
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

/// Soma recursiva de uma pasta, sem seguir link.
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

/// Processos do FiveM em execução.
///
/// Devolve (lançador aberto, jogo aberto). O jogo roda num processo próprio,
/// cujo nome carrega o número da compilação — por isso a comparação é por
/// prefixo e não por nome exato.
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

        // `FiveM_b3570_GTAProcess.exe` é o jogo; `FiveM.exe` é só o lançador.
        if nome.contains("gtaprocess") {
            jogo = true;
        }
    }

    (lancador, jogo)
}

/// Monta a frase de resumo.
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

/// Levantamento completo da instalação.
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

        // Pasta vazia não vira linha: encheria a tela sem dizer nada.
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

    // O que dá para recuperar primeiro; dentro de cada grupo, do maior.
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

/// Apaga uma pasta descartável da instalação.
///
/// Três travas: a pasta precisa estar no catálogo e marcada como descartável,
/// o FiveM precisa estar fechado, e o conteúdo é apagado sem remover a pasta.
pub fn limpar(id: &str) -> Result<CleanOutcome, String> {
    let base = pasta_do_fivem().ok_or("O FiveM não está instalado nesta máquina.")?;

    // A checagem é refeita aqui e não confiada à interface: o comando é exposto
    // por IPC, e uma tela com dado velho não pode virar permissão para apagar o
    // perfil do jogo de alguém.
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

    // O que se relata é o que sumiu de verdade, e não o que se esperava apagar.
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

/// Coloca o processo do jogo em prioridade alta no processador.
///
/// DUAS COISAS QUE PRECISAM ESTAR DITAS
///
/// A prioridade é do processo, e some quando ele fecha. Não é ajuste
/// permanente, e prometer o contrário seria mentira: tem que ser aplicado a
/// cada sessão.
///
/// E é "alta", nunca "tempo real". Tempo real coloca o jogo acima do próprio
/// sistema operacional, incluindo as tarefas que cuidam de som, mouse e teclado
/// — o resultado prático costuma ser travar a máquina inteira. É uma das
/// "dicas de FPS" mais repetidas e mais perigosas da internet.
pub fn priorizar_jogo() -> Result<String, String> {
    let (_, jogo_aberto) = processos_abertos();

    if !jogo_aberto {
        return Err(
            "O jogo não está aberto. Entre no FiveM primeiro — a prioridade é dada ao \
             processo do jogo, e ele só existe com o jogo rodando."
                .to_string(),
        );
    }

    let script = "$n = 0; \
                  foreach ($p in @(Get-Process -ErrorAction SilentlyContinue | \
                    Where-Object { $_.Name -like 'FiveM*GTAProcess*' })) { \
                    try { $p.PriorityClass = 'High'; $n++ } catch {} }; \
                  $n";

    let saida = shell::powershell(script)?;
    let quantos: u32 = saida.stdout.trim().parse().unwrap_or(0);

    if quantos == 0 {
        return Err(
            "Não foi possível alterar a prioridade do jogo. Reabra o Otimiza como \
             administrador e tente de novo."
                .to_string(),
        );
    }

    Ok("Jogo em prioridade alta no processador. Vale para esta sessão: ao fechar o FiveM a \
        prioridade volta ao normal, e isto precisa ser aplicado de novo na próxima vez."
        .to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perfil_do_jogo_nunca_e_apagavel() {
        // A confusão mais cara possível deste módulo, e a mesma que o navegador
        // já ensinou: a segunda maior pasta é a que não se pode tocar.
        // `game-storage` tinha 3,2 GB numa instalação real e guarda a sessão da
        // Rockstar. Apagar desloga a pessoa da conta dela.
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
        // Limpar não é ganho puro: o servidor reenvia tudo na próxima conexão,
        // e em servidor de RP isso demora. O usuário precisa saber antes.
        let preco = tradeoff.expect("a maior limpeza do módulo precisa declarar o preço");
        assert!(preco.contains("baixar tudo de novo"));
    }

    #[test]
    fn limpar_recusa_pasta_protegida() {
        // O comando é exposto por IPC. Uma tela com dado velho, ou qualquer
        // chamada direta, não pode virar permissão para apagar o perfil.
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
        // A parte que impede o cliente de achar que estamos escondendo espaço.
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

    /// Só o código de produção, sem os testes.
    ///
    /// Os guards abaixo procuram nomes de API proibidos no módulo. Sem este
    /// corte eles se acusariam sozinhos, porque os nomes aparecem dentro das
    /// próprias asserções.
    fn codigo_de_producao() -> &'static str {
        let fonte = include_str!("fivem.rs");
        fonte.split("#[cfg(test)]").next().unwrap()
    }

    #[test]
    fn prioridade_e_alta_e_nunca_tempo_real() {
        // "Tempo real" coloca o jogo acima do sistema operacional, incluindo o
        // que cuida de som, mouse e teclado. É uma das dicas de FPS mais
        // repetidas da internet e trava a máquina inteira.
        let fonte = codigo_de_producao();

        assert!(fonte.contains("PriorityClass = 'High'"));
        assert!(
            !fonte.contains("PriorityClass = 'RealTime'"),
            "prioridade de tempo real nunca pode entrar aqui"
        );
    }

    #[test]
    fn nao_ha_injecao_nem_liberacao_de_memoria() {
        // As duas recusas do cabeçalho, travadas em teste: injetar overlay é
        // risco de ban para o cliente, e esvaziar memória piora o desempenho
        // real enquanto melhora o gráfico.
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
            // Toda pasta listada tem explicação, e toda pasta apagável com
            // preço declara qual é.
            for f in &r.folders {
                assert!(!f.explanation.is_empty(), "{} sem explicação", f.name);
            }

            // As protegidas vêm depois das apagáveis.
            let primeira_protegida = r.folders.iter().position(|f| !f.cleanable);
            let ultima_limpavel = r.folders.iter().rposition(|f| f.cleanable);

            if let (Some(p), Some(l)) = (primeira_protegida, ultima_limpavel) {
                assert!(p > l, "pasta protegida apareceu antes de uma apagável");
            }
        }
    }
}
