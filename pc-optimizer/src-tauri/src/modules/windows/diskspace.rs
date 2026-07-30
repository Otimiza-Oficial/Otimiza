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
];

/// Tamanho de uma pasta, somando tudo que houver dentro.
/// Pasta inacessível conta zero — nunca derruba a varredura.
fn directory_size(dir: &std::path::Path) -> u64 {
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

/// Varre todas as categorias. Não apaga nada.
pub fn scan() -> DiskReport {
    let findings: Vec<SpaceFinding> = CATEGORIES
        .iter()
        .map(|c| {
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
        let (bytes, ignorados) = limpar_conteudo(caminho);
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
    shell::run_checked(
        "powershell",
        &["-NoProfile", "-Command", "Clear-RecycleBin -Force -ErrorAction Stop"],
    )
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
        let permitidos = ["temp", "softwaredistribution", "wer", "downloader", "logs", "windows.old"];

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
