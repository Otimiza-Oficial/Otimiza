// Liberador de espaço
//
// Num PC fraco, disco cheio é o problema que mais se disfarça de "PC lento".
// Windows abaixo de 10% de espaço livre para de conseguir gerenciar o arquivo de
// paginação com folga, o Explorer engasga e as atualizações falham — e o dono da
// máquina jura que o problema é o processador.
//
// Este módulo faz o que a Limpeza de Disco do Windows deveria fazer: mostra
// CATEGORIA POR CATEGORIA quanto dá para recuperar, explica o que cada uma é, e
// deixa o usuário escolher. Sem barra de progresso genérica e sem prometer
// "otimizar" o que ele não pode conferir.

use super::shell;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceFinding {
    pub id: String,
    pub name: String,
    /// O que é aquele espaço, em português claro.
    pub explanation: String,
    pub bytes: u64,
    pub formatted: String,
    /// Se o Otimiza consegue limpar isto por aqui.
    pub cleanable: bool,
    pub requires_admin: bool,
    /// O que se perde ao limpar. Vazio quando não se perde nada.
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskReport {
    pub drive: String,
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub free_percent: f64,
    /// Aviso quando o espaço livre já é baixo o bastante para atrapalhar o Windows.
    pub pressure: Option<String>,
    pub recoverable_bytes: u64,
    pub findings: Vec<SpaceFinding>,
}

/// Uma categoria de espaço recuperável.
struct Category {
    id: &'static str,
    name: &'static str,
    explanation: &'static str,
    warning: Option<&'static str>,
    requires_admin: bool,
    /// `false` quando a remoção é arriscada demais para fazermos por aqui.
    cleanable: bool,
    paths: fn() -> Vec<PathBuf>,
}

fn local_appdata() -> Option<PathBuf> {
    std::env::var("LOCALAPPDATA").ok().map(PathBuf::from)
}

fn windows_dir() -> Option<PathBuf> {
    std::env::var("SystemRoot").ok().map(PathBuf::from)
}

fn program_data() -> Option<PathBuf> {
    std::env::var("ProgramData").ok().map(PathBuf::from)
}

fn system_drive() -> String {
    std::env::var("SystemDrive").unwrap_or_else(|_| "C:".to_string())
}

static CATEGORIES: &[Category] = &[
    Category {
        id: "temp",
        name: "Arquivos temporários",
        explanation: "Sobras de instaladores e de programas que abriram arquivos temporários e não os apagaram.",
        warning: None,
        requires_admin: false,
        cleanable: true,
        paths: || {
            let mut p = Vec::new();
            if let Ok(temp) = std::env::var("TEMP") {
                p.push(PathBuf::from(temp));
            }
            if let Some(win) = windows_dir() {
                p.push(win.join("Temp"));
            }
            p
        },
    },
    Category {
        id: "update_cache",
        name: "Instaladores de atualizações",
        explanation: "Os instaladores que o Windows guarda depois de aplicar cada atualização. Não são necessários para o sistema funcionar.",
        warning: None,
        requires_admin: true,
        cleanable: true,
        paths: || {
            windows_dir()
                .map(|w| vec![w.join("SoftwareDistribution").join("Download")])
                .unwrap_or_default()
        },
    },
    Category {
        id: "windows_old",
        name: "Instalação anterior do Windows",
        explanation: "Cópia da versão antiga do Windows, guardada depois de uma atualização grande. Costuma ser a maior sobra do disco.",
        warning: Some(
            "Apagar remove a possibilidade de voltar para a versão anterior do Windows. \
             Esta pasta pertence ao sistema e resiste a remoção comum — use a Limpeza de \
             Disco do Windows, opção \"Instalações anteriores do Windows\".",
        ),
        requires_admin: true,
        // Deliberadamente NÃO limpamos: a pasta é do TrustedInstaller e a remoção
        // comum falha no meio, deixando lixo pela metade. Prometer e entregar
        // metade é pior que apontar a ferramenta certa.
        cleanable: false,
        paths: || vec![PathBuf::from(format!("{}\\Windows.old", system_drive()))],
    },
    Category {
        id: "error_reports",
        name: "Relatórios de erro",
        explanation: "Despejos de memória que o Windows salva quando um programa trava, para enviar à Microsoft.",
        warning: None,
        requires_admin: true,
        cleanable: true,
        paths: || {
            let mut p = Vec::new();
            if let Some(pd) = program_data() {
                p.push(pd.join("Microsoft").join("Windows").join("WER"));
            }
            if let Some(la) = local_appdata() {
                p.push(la.join("Microsoft").join("Windows").join("WER"));
            }
            p
        },
    },
    Category {
        id: "delivery_optimization",
        name: "Cache de compartilhamento de atualizações",
        explanation: "Pedaços de atualizações que o Windows guardou para distribuir a outros computadores.",
        warning: None,
        requires_admin: true,
        cleanable: true,
        paths: || {
            program_data()
                .map(|pd| vec![pd.join("Microsoft").join("Network").join("Downloader")])
                .unwrap_or_default()
        },
    },
    Category {
        id: "update_logs",
        name: "Registros de atualização",
        explanation: "Arquivos de log que o Windows escreve a cada atualização. Só servem para diagnóstico.",
        warning: None,
        requires_admin: true,
        cleanable: true,
        paths: || {
            windows_dir()
                .map(|w| vec![w.join("Logs").join("CBS")])
                .unwrap_or_default()
        },
    },
    Category {
        id: "winsxs",
        name: "Componentes antigos do Windows",
        explanation: "Cópias antigas de componentes do Windows, guardadas para permitir desinstalar \
             atualizações já aplicadas. O DISM decide quanto disso já não faz falta.",
        // Sem aviso porque nada de valor se perde limpando o que o DISM já marcou
        // como recuperável — ao contrário do Windows.old, aqui não há rollback
        // para perder.
        warning: Some(
            "A limpeza de verdade (`DISM /StartComponentCleanup`) leva de 5 a 25 minutos \
             e mexe no repositório de componentes do sistema — não cabe num clique rápido \
             daqui. Use a ferramenta de reparo (Analisar e limpar WinSxS) para fazer isso \
             com acompanhamento de progresso e sem risco de interrupção pela metade.",
        ),
        requires_admin: true,
        // Deliberadamente NÃO limpamos por aqui: a operação de verdade é longa
        // (minutos) e mexe em componentes do sistema — igual ao Windows.old, uma
        // limpeza cortada no meio é pior que apontar a ferramenta certa.
        cleanable: false,
        // Sem pasta própria: o tamanho vem do DISM, nunca de `directory_size`.
        paths: || vec![],
    },
    Category {
        id: "browser_cache",
        name: "Cache dos navegadores",
        explanation: "Páginas, imagens e scripts que Chrome, Edge e Firefox guardam no disco para \
             abrir os mesmos sites mais rápido depois.",
        warning: Some("Os sites vão carregar uma vez mais devagar."),
        requires_admin: false,
        cleanable: true,
        paths: || {
            let mut p = Vec::new();
            if let Some(la) = local_appdata() {
                // Edge é baseado no Chromium, então herda a mesma estrutura de
                // pastas do Chrome ("User Data\Default\Cache").
                p.push(
                    la.join("Google")
                        .join("Chrome")
                        .join("User Data")
                        .join("Default")
                        .join("Cache"),
                );
                p.push(
                    la.join("Microsoft")
                        .join("Edge")
                        .join("User Data")
                        .join("Default")
                        .join("Cache"),
                );

                // O Firefox guarda o cache dentro de uma pasta de perfil com nome
                // aleatório ("xxxxxxxx.default-release"), então é preciso varrer
                // o diretório de perfis em vez de apontar para um caminho fixo.
                let perfis = la.join("Mozilla").join("Firefox").join("Profiles");
                if let Ok(entradas) = fs::read_dir(&perfis) {
                    for entrada in entradas.filter_map(|e| e.ok()) {
                        p.push(entrada.path().join("cache2"));
                    }
                }
            }
            p
        },
    },
    Category {
        id: "crash_dumps",
        name: "Despejos de memória",
        explanation: "Arquivos que o Windows grava quando o sistema trava (tela azul), usados só \
             para diagnóstico técnico. Ninguém abre isso no dia a dia.",
        warning: None,
        requires_admin: true,
        cleanable: true,
        paths: || {
            windows_dir()
                .map(|w| vec![w.join("MEMORY.DMP"), w.join("Minidump")])
                .unwrap_or_default()
        },
    },
    Category {
        id: "store_cache",
        name: "Cache da Microsoft Store",
        explanation: "Dados temporários que a Microsoft Store guarda para listar e abrir aplicativos \
             mais rápido.",
        warning: Some("A Store vai demorar um pouco mais para abrir na primeira vez."),
        requires_admin: false,
        cleanable: true,
        paths: || {
            local_appdata()
                .map(|la| {
                    vec![la
                        .join("Packages")
                        .join("Microsoft.WindowsStore_8wekyb3d8bbwe")
                        .join("LocalCache")]
                })
                .unwrap_or_default()
        },
    },
];

/// Tamanho de um caminho: se for arquivo — como o `MEMORY.DMP` da categoria de
/// despejos de memória —, o tamanho é o do próprio arquivo; se for pasta, soma
/// tudo que houver dentro. Caminho inacessível conta zero — nunca derruba a
/// varredura.
fn directory_size(dir: &std::path::Path) -> u64 {
    if let Ok(meta) = fs::metadata(dir) {
        if meta.is_file() {
            return meta.len();
        }
    }

    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return 0,
    };

    entries
        .filter_map(|entry| entry.ok())
        .map(|entry| match entry.metadata() {
            Ok(meta) if meta.is_dir() => directory_size(&entry.path()),
            Ok(meta) => meta.len(),
            Err(_) => 0,
        })
        .sum()
}

pub fn format_size(bytes: u64) -> String {
    const GB: f64 = 1_073_741_824.0;
    const MB: f64 = 1_048_576.0;
    let b = bytes as f64;

    if bytes == 0 {
        "vazio".to_string()
    } else if b >= GB {
        format!("{:.1} GB", b / GB)
    } else if b >= MB {
        format!("{:.0} MB", b / MB)
    } else {
        format!("{:.0} KB", b / 1024.0)
    }
}

/// Espaço total e livre do disco do sistema.
fn disk_usage() -> (u64, u64) {
    let drive = system_drive();
    let disks = sysinfo::Disks::new_with_refreshed_list();

    for disk in &disks {
        let ponto = disk.mount_point().to_string_lossy().to_uppercase();
        if ponto.starts_with(&drive.to_uppercase()) {
            return (disk.total_space(), disk.available_space());
        }
    }

    (0, 0)
}

/// Quanto o DISM diz que dá para recuperar do repositório de componentes.
///
/// PURA, PARA SER TESTÁVEL SEM RODAR O DISM. A análise real
/// (`DISM /Online /Cleanup-Image /AnalyzeComponentStore`) leva minutos e exige
/// administrador; a leitura da resposta não precisa de nenhum dos dois.
///
/// `None` quando a linha não veio — e `None` NÃO é zero. "Não consegui
/// estimar" e "não há nada para recuperar" são coisas diferentes, e confundir
/// as duas já foi o defeito deste produto em quatro módulos.
///
/// Lê português e inglês porque o DISM responde no idioma do sistema, e um
/// cliente com Windows em inglês veria a categoria sumir sem explicação.
pub fn estimativa_do_winsxs(saida_do_dism: &str) -> Option<u64> {
    for linha in saida_do_dism.lines() {
        let minuscula = linha.to_lowercase();

        if !(minuscula.contains("recuperável")
            || minuscula.contains("recuperavel")
            || minuscula.contains("reclaimable"))
        {
            continue;
        }

        // `continue`, não `?`: uma linha malformada só descarta ELA, não a
        // busca inteira. Com `?` aqui, "Reclaimable Packages : 12" (sem
        // unidade) abortava a função antes mesmo de chegar na linha
        // "Reclaimable : 2.34 GB" logo depois — a saída em inglês do DISM
        // manda as duas, nessa ordem, e a função nunca lia a segunda.
        let depois = match linha.split_once(':') {
            Some((_, depois)) => depois,
            None => continue,
        };
        let bruto = depois.trim();

        // "2.34 GB" — o número e a unidade. Um contador de pacotes
        // ("Reclaimable Packages : 12") não tem unidade e é descartado aqui.
        let (numero, unidade) = match bruto.split_once(' ') {
            Some(par) => par,
            None => continue,
        };
        let valor: f64 = match numero.replace(',', ".").parse() {
            Ok(v) => v,
            Err(_) => continue,
        };

        let multiplicador: u64 = match unidade.trim().to_uppercase().as_str() {
            "KB" => 1024,
            "MB" => 1024 * 1024,
            "GB" => 1024 * 1024 * 1024,
            "TB" => 1024u64 * 1024 * 1024 * 1024,
            _ => continue,
        };

        return Some((valor * multiplicador as f64) as u64);
    }

    None
}

/// Tenta obter a saída do `DISM /AnalyzeComponentStore` sem travar a varredura.
///
/// A análise verdadeira pode levar de 1 a 5 minutos (ver `estimativa_do_winsxs`)
/// — e este `scan()` precisa responder rápido, porque também é chamado pelo
/// veredito geral da máquina. Por isso a chamada roda numa thread à parte: se o
/// DISM não respondeu dentro do prazo curto, a categoria diz honestamente que
/// não deu para estimar agora, em vez de deixar a tela inteira esperando por
/// uma análise de minutos. Quem quiser o número de qualquer jeito tem a
/// ferramenta de reparo, que roda a mesma análise como tarefa longa com
/// progresso.
fn saida_do_dism_analyze_component_store() -> Option<String> {
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let saida = shell::run("Dism", &["/Online", "/Cleanup-Image", "/AnalyzeComponentStore"]);
        let _ = tx.send(saida);
    });

    match rx.recv_timeout(Duration::from_secs(3)) {
        Ok(Ok(saida)) if saida.success => Some(saida.stdout),
        _ => None,
    }
}

/// Monta o achado do WinSxS a partir da estimativa do DISM — nunca do tamanho
/// da pasta, que mente por causa dos hard links com `C:\Windows` (ver módulo).
///
/// Separada de `finding_do_winsxs` para ser testável sem rodar o DISM de
/// verdade: o teste passa a saída já pronta e confere que o `bytes` do achado
/// é exatamente o que `estimativa_do_winsxs` devolveu para aquela saída — nunca
/// um tamanho lido do disco.
fn finding_do_winsxs_a_partir_da_saida(c: &Category, saida_do_dism: Option<&str>) -> SpaceFinding {
    let estimativa = saida_do_dism.and_then(estimativa_do_winsxs);

    let (bytes, explanation, formatted) = match estimativa {
        Some(bytes) => (bytes, c.explanation.to_string(), format_size(bytes)),
        None => (
            0,
            "Não foi possível estimar quanto dá para recuperar do repositório de \
             componentes (WinSxS) agora. A pasta aparece grande no Explorer por causa \
             de hard links com o próprio `C:\\Windows` — o número real só vem da análise \
             do DISM, que pode ser rodada pela ferramenta de reparo."
                .to_string(),
            // NUNCA "vazio": "vazio" diria que não há nada para recuperar, e a
            // verdade aqui é que não conseguimos medir — são coisas diferentes.
            "não consegui estimar".to_string(),
        ),
    };

    SpaceFinding {
        id: c.id.to_string(),
        name: c.name.to_string(),
        explanation,
        bytes,
        formatted,
        cleanable: false,
        requires_admin: c.requires_admin,
        warning: c.warning.map(|w| w.to_string()),
    }
}

/// Monta o achado do WinSxS rodando o DISM de verdade. Fininha de propósito:
/// toda a lógica testável está em `finding_do_winsxs_a_partir_da_saida`.
fn finding_do_winsxs(c: &Category) -> SpaceFinding {
    let saida = saida_do_dism_analyze_component_store();
    finding_do_winsxs_a_partir_da_saida(c, saida.as_deref())
}

/// Varre todas as categorias. Não apaga nada.
pub fn scan() -> DiskReport {
    let findings: Vec<SpaceFinding> = CATEGORIES
        .iter()
        .map(|c| {
            if c.id == "winsxs" {
                return finding_do_winsxs(c);
            }

            let bytes: u64 = (c.paths)()
                .iter()
                .filter(|p| p.exists())
                .map(|p| directory_size(p))
                .sum();

            SpaceFinding {
                id: c.id.to_string(),
                name: c.name.to_string(),
                explanation: c.explanation.to_string(),
                bytes,
                formatted: format_size(bytes),
                cleanable: c.cleanable && bytes > 0,
                requires_admin: c.requires_admin,
                warning: c.warning.map(|w| w.to_string()),
            }
        })
        .collect();

    let (total_bytes, free_bytes) = disk_usage();
    let free_percent = if total_bytes > 0 {
        free_bytes as f64 / total_bytes as f64 * 100.0
    } else {
        0.0
    };

    // Abaixo de 10% o Windows perde folga para paginação e atualização. É o
    // ponto em que "PC lento" costuma ser, na verdade, disco cheio.
    let pressure = if total_bytes > 0 && free_percent < 10.0 {
        Some(format!(
            "Só {:.0}% de espaço livre. Abaixo de 10% o Windows perde folga para o \
             arquivo de paginação e para atualizações, e o PC fica lento por causa \
             disso — não por causa do processador.",
            free_percent
        ))
    } else {
        None
    };

    // O recuperável conta só o que dá para limpar por aqui: somar o que não
    // limpamos seria prometer espaço que o usuário não vai ver.
    let recoverable_bytes = findings.iter().filter(|f| f.cleanable).map(|f| f.bytes).sum();

    let mut findings = findings;
    findings.sort_by(|a, b| b.bytes.cmp(&a.bytes));

    DiskReport {
        drive: system_drive(),
        total_bytes,
        free_bytes,
        free_percent,
        pressure,
        recoverable_bytes,
        findings,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanOutcome {
    pub id: String,
    pub freed_bytes: u64,
    pub message: String,
}

/// Limpa uma categoria. Só as marcadas como limpáveis.
pub fn clean(id: &str) -> Result<CleanOutcome, String> {
    let categoria = CATEGORIES
        .iter()
        .find(|c| c.id == id)
        .ok_or_else(|| format!("Categoria desconhecida: {}", id))?;

    if !categoria.cleanable {
        return Err(format!(
            "`{}` não é limpo por aqui. {}",
            categoria.name,
            categoria.warning.unwrap_or("")
        ));
    }

    if categoria.requires_admin && !super::registry::is_elevated() {
        return Err(format!(
            "Limpar `{}` exige executar o Otimiza como administrador.",
            categoria.name
        ));
    }

    // A Store é um caso à parte: o pacote UWP tem permissões diferentes de uma
    // pasta comum, e apagar `LocalCache` na unha arrisca corromper o registro
    // do aplicativo. O `wsreset.exe` é a própria ferramenta do Windows para
    // isto — ele encerra a Store, limpa o cache dela e a reabre.
    if id == "store_cache" {
        let liberado: u64 = (categoria.paths)()
            .iter()
            .filter(|p| p.exists())
            .map(|p| directory_size(p))
            .sum();

        shell::run("wsreset.exe", &[])
            .map_err(|_| "Não foi possível limpar o cache da Microsoft Store.".to_string())?;

        return Ok(CleanOutcome {
            id: id.to_string(),
            freed_bytes: liberado,
            message: format!("{} liberados de {}.", format_size(liberado), categoria.name),
        });
    }

    // Parar os serviços de atualização antes de mexer no que é deles evita
    // apagar pela metade e confundir uma atualização em andamento.
    let mexe_com_update = matches!(id, "update_cache" | "delivery_optimization" | "update_logs");
    let mut estavam_rodando = Vec::new();
    let servicos = ["wuauserv", "bits", "dosvc"];

    if mexe_com_update {
        for servico in servicos {
            let rodando = super::services::is_running(servico);
            estavam_rodando.push(rodando);
            if rodando {
                let _ = super::services::stop(servico);
            }
        }
    }

    let mut liberado = 0u64;
    let mut pulados = 0usize;

    for caminho in (categoria.paths)().iter().filter(|p| p.exists()) {
        let (bytes, ignorados) = limpar_caminho(caminho);
        liberado += bytes;
        pulados += ignorados;
    }

    if mexe_com_update {
        for (servico, estava) in servicos.iter().zip(estavam_rodando) {
            if estava {
                let _ = super::services::start(servico);
            }
        }
    }

    let mut message = format!("{} liberados de {}.", format_size(liberado), categoria.name);
    if pulados > 0 {
        message.push_str(&format!(" {} itens em uso foram pulados.", pulados));
    }

    Ok(CleanOutcome {
        id: id.to_string(),
        freed_bytes: liberado,
        message,
    })
}

/// Libera um caminho: se for arquivo — como o `MEMORY.DMP` da categoria de
/// despejos de memória —, apaga o arquivo em si; se for pasta, apaga só o
/// conteúdo (ver `limpar_conteudo`).
fn limpar_caminho(caminho: &std::path::Path) -> (u64, usize) {
    let meta = match fs::metadata(caminho) {
        Ok(meta) => meta,
        Err(_) => return (0, 0),
    };

    if meta.is_file() {
        let tamanho = meta.len();
        return match fs::remove_file(caminho) {
            Ok(()) => (tamanho, 0),
            Err(_) => (0, 1),
        };
    }

    limpar_conteudo(caminho)
}

/// Apaga o conteúdo de uma pasta, preservando a pasta em si.
/// Item em uso é pulado: travar a limpeza porque um arquivo está aberto seria
/// pior que deixar esse arquivo para trás.
fn limpar_conteudo(dir: &std::path::Path) -> (u64, usize) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return (0, 0),
    };

    let mut liberado = 0u64;
    let mut pulados = 0usize;

    for entry in entries.filter_map(|e| e.ok()) {
        let caminho = entry.path();

        let meta = match entry.metadata() {
            Ok(meta) => meta,
            Err(_) => {
                pulados += 1;
                continue;
            }
        };

        if meta.is_dir() {
            let tamanho = directory_size(&caminho);
            match fs::remove_dir_all(&caminho) {
                Ok(()) => liberado += tamanho,
                Err(_) => pulados += 1,
            }
        } else {
            match fs::remove_file(&caminho) {
                Ok(()) => liberado += meta.len(),
                Err(_) => pulados += 1,
            }
        }
    }

    (liberado, pulados)
}

/// Esvazia a Lixeira. Fica fora das categorias porque não é uma pasta que se
/// varre: o Windows tem chamada própria para isso, e usá-la respeita as regras
/// dele em vez de sair apagando `$Recycle.Bin` na unha.
pub fn empty_recycle_bin() -> Result<String, String> {
    shell::powershell_checked("Clear-RecycleBin -Force -ErrorAction Stop")
    .map_err(|_| "Não foi possível esvaziar a Lixeira (ela pode já estar vazia).".to_string())?;

    Ok("Lixeira esvaziada.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toda_categoria_tem_explicacao_em_portugues() {
        for c in CATEGORIES {
            assert!(!c.explanation.trim().is_empty(), "{} sem explicação", c.id);
            assert!(!c.name.trim().is_empty());
        }
    }

    #[test]
    fn categoria_perigosa_avisa_e_nao_e_limpa_por_aqui() {
        // Windows.old resiste a remoção comum e apagar metade é pior que não
        // apagar. A regra: o que não é limpável precisa dizer o porquê.
        for c in CATEGORIES.iter().filter(|c| !c.cleanable) {
            assert!(
                c.warning.is_some(),
                "{} não é limpável e não explica o motivo",
                c.id
            );
        }
    }

    #[test]
    fn ids_sao_unicos() {
        let mut ids: Vec<&str> = CATEGORIES.iter().map(|c| c.id).collect();
        ids.sort();
        let total = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), total, "id repetido torna a limpeza ambígua");
    }

    #[test]
    fn todo_caminho_fica_dentro_de_pasta_conhecida() {
        // Nenhuma categoria pode apontar para pasta de documentos do usuário.
        // Este teste é a barreira contra alguém acrescentar uma categoria que
        // apague algo que importa.
        let permitidos = [
            "temp",
            "softwaredistribution",
            "wer",
            "downloader",
            "logs",
            "windows.old",
            "chrome",
            "edge",
            "firefox",
            "memory.dmp",
            "minidump",
            "microsoft.windowsstore",
        ];

        for c in CATEGORIES {
            for caminho in (c.paths)() {
                let p = caminho.to_string_lossy().to_lowercase();
                assert!(
                    permitidos.iter().any(|permitido| p.contains(permitido)),
                    "categoria {} aponta para fora das pastas conhecidas: {}",
                    c.id,
                    p
                );
            }
        }
    }

    #[test]
    fn formata_tamanhos_para_leitura() {
        assert_eq!(format_size(0), "vazio");
        assert_eq!(format_size(524_288_000), "500 MB");
        assert_eq!(format_size(2_147_483_648), "2.0 GB");
    }

    #[test]
    fn nao_promete_espaco_que_nao_vai_entregar() {
        // O recuperável não pode incluir o que a gente não limpa.
        let relatorio = scan();
        let soma_limpavel: u64 = relatorio
            .findings
            .iter()
            .filter(|f| f.cleanable)
            .map(|f| f.bytes)
            .sum();

        assert_eq!(relatorio.recoverable_bytes, soma_limpavel);
    }

    #[test]
    fn o_winsxs_usa_a_estimativa_do_dism_e_nunca_o_tamanho_da_pasta() {
        // O WINSXS MENTE SOBRE O TAMANHO, e o produto nao pode repetir a mentira.
        //
        // A pasta aparece com 11,5 GB na maquina do dono, mas usa hard links:
        // boa parte daquilo sao os MESMOS arquivos de `C:\Windows`, contados de
        // novo. O que o DISM libera de verdade costuma ser 1 a 5 GB. Mostrar
        // 11,5 GB e prometer o que nao vai acontecer.
        let saida = "\
Versão: 10.0.26100.1

Imagem : C:\\

Tamanho do Repositório de Componentes Reportável ao Windows Explorer : 11.49 GB
Tamanho Real do Repositório de Componentes : 8.21 GB
Recuperável : 2.34 GB
Limpeza do Repositório de Componentes Recomendada : Sim
A operação foi concluída com êxito.";

        let bytes = estimativa_do_winsxs(saida).expect("a linha Recuperavel existe nesta saida");
        // 2,34 GB, com folga de arredondamento.
        assert!(bytes > 2_400_000_000 && bytes < 2_600_000_000, "veio {}", bytes);
    }

    #[test]
    fn o_achado_do_winsxs_usa_o_numero_do_dism_e_nao_o_tamanho_do_disco() {
        // Regra do modulo: o achado NUNCA pode vir de `directory_size` da pasta
        // WinSxS — so da estimativa do DISM. Este teste passa a saida do DISM
        // ja pronta (sem rodar o comando de verdade) e confere que o `bytes`
        // do achado bate exatamente com `estimativa_do_winsxs` para essa saida.
        // Se alguem trocar a fonte por um `directory_size(...)`, o valor nao
        // vai mais bater e este teste reprova.
        let categoria = CATEGORIES.iter().find(|c| c.id == "winsxs").expect("categoria winsxs existe");
        let saida = "Recuperável : 2.34 GB\n";

        let achado = finding_do_winsxs_a_partir_da_saida(categoria, Some(saida));
        let esperado = estimativa_do_winsxs(saida).expect("saida de teste tem a linha Recuperavel");

        assert_eq!(achado.bytes, esperado);
    }

    #[test]
    fn winsxs_sem_a_linha_de_recuperavel_vira_nao_sei_e_nao_zero() {
        // "NAO CONSEGUI ESTIMAR" e diferente de "nao ha nada para recuperar". A
        // primeira e honesta; a segunda seria o produto afirmando o que nao mediu.
        assert_eq!(estimativa_do_winsxs("A operação falhou. Erro: 0x800f0954"), None);
        assert_eq!(estimativa_do_winsxs(""), None);
    }

    #[test]
    fn a_estimativa_entende_o_dism_em_ingles_tambem() {
        // O Windows do cliente pode estar em ingles, e o DISM responde no idioma
        // do sistema. Ler so o portugues faria a categoria sumir para esse
        // cliente, sem explicacao.
        let saida = "Reclaimable Packages : 12\nReclaimable : 2.34 GB\n";
        assert!(estimativa_do_winsxs(saida).is_some(), "nao leu a saida em ingles");
    }

    #[test]
    fn toda_categoria_nova_avisa_o_que_se_perde_quando_ha_o_que_perder() {
        // O cache do navegador apagado desloga de nada, mas faz o primeiro
        // carregamento de cada site ficar mais lento uma vez. O cliente precisa
        // saber ANTES de clicar -- e nao descobrir depois achando que quebrou.
        let relatorio = scan();

        for id in ["browser_cache", "store_cache"] {
            if let Some(f) = relatorio.findings.iter().find(|f| f.id == id) {
                assert!(f.warning.is_some(), "{} nao diz o que se perde", id);
            }
        }
    }

    #[test]
    fn varre_esta_maquina() {
        let r = scan();
        println!(
            "{} — {} livres de {} ({:.0}%)",
            r.drive,
            format_size(r.free_bytes),
            format_size(r.total_bytes),
            r.free_percent
        );

        for f in &r.findings {
            println!("  {:<40} {}", f.name, f.formatted);
        }
        println!("recuperável: {}", format_size(r.recoverable_bytes));

        assert_eq!(r.findings.len(), CATEGORIES.len());
        // Vem ordenado do maior para o menor: o que mais devolve espaço primeiro.
        let tamanhos: Vec<u64> = r.findings.iter().map(|f| f.bytes).collect();
        assert!(tamanhos.windows(2).all(|par| par[0] >= par[1]));
    }
}
