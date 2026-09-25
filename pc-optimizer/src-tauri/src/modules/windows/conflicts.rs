// Dois programas fazendo a mesma coisa: antivírus varrendo um ao outro, sobreposições no mesmo jogo,
// otimizadores desfazendo um ao outro. Aqui não se desinstala nada: mostra o conflito pelo nome, e a pessoa
// escolhe.

use super::{registry, shell};
use serde::{Deserialize, Serialize};

pub use super::firmware::FindingSeverity;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    pub id: String,
    pub title: String,
    pub found: Vec<String>,
    pub explanation: String,
    pub advice: String,
    pub severity: FindingSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictReport {
    pub conflicts: Vec<Conflict>,
    /// `None` quando a lista não se leu: "0 examinados" seria um número falso.
    pub programs_scanned: Option<usize>,
    /// Com qualquer coisa aqui, a ausência de conflito não é afirmada.
    pub lacunas: Vec<String>,
}

const UNINSTALL_KEYS: [(&str, &str); 3] = [
    ("HKLM", r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall"),
    ("HKLM", r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall"),
    ("HKCU", r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall"),
];

/// As três chaves (64 bits, 32 bits e só deste usuário): ler só a primeira perde metade da lista nas máquinas
/// antigas. `Err` quando uma não abre.
pub fn programas_instalados() -> Result<Vec<String>, String> {
    let mut nomes = Vec::new();

    for (hive, base) in UNINSTALL_KEYS {
        for entrada in registry::subkeys(hive, base)? {
            let caminho = format!("{}\\{}", base, entrada);

            if let Ok(Some(nome)) = registry::read_text(hive, &caminho, "DisplayName") {
                let nome = nome.trim().to_string();
                if !nome.is_empty() && !nomes.contains(&nome) {
                    nomes.push(nome);
                }
            }
        }
    }

    Ok(nomes)
}

fn casar(programas: &[String], termos: &[&str]) -> Vec<String> {
    let mut achados: Vec<String> = programas
        .iter()
        .filter(|p| {
            let baixo = p.to_lowercase();
            termos.iter().any(|t| baixo.contains(t))
        })
        .cloned()
        .collect();

    achados.sort();
    achados.dedup();
    achados
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawAntivirus {
    display_name: Option<String>,
    product_state: Option<u32>,
}

/// Tempo real LIGADO é o que pesa: quase toda máquina tem o Defender instalado. `Err` quando a Central de
/// Segurança não responde (imagem "lite"): não saber não é ter só um.
pub fn antivirus_ativos() -> Result<Vec<String>, String> {
    let script = "ConvertTo-Json -Compress -Depth 2 -InputObject @(Get-CimInstance \
                  -Namespace root/SecurityCenter2 -ClassName AntiVirusProduct \
                  -ErrorAction Stop | Select-Object displayName,productState)";

    let brutos: Vec<RawAntivirus> = shell::json_da_saida(
        shell::powershell(script),
        "os antivírus registrados no Windows",
    )?;

    // Estado ilegível propaga erro: virar 0 fazia o antivírus sumir calado, justamente com dois varrendo juntos.
    let mut ativos = Vec::new();

    for a in brutos {
        let Some(estado) = a.product_state else {
            return Err(
                "A Central de Segurança do Windows respondeu sem o estado de um dos \
                 antivírus, então não dá para dizer quantos estão varrendo ao mesmo \
                 tempo nesta máquina."
                    .to_string(),
            );
        };

        if tempo_real_ligado(estado) {
            if let Some(nome) = a.display_name {
                ativos.push(nome);
            }
        }
    }

    Ok(ativos)
}

/// TESTA O BIT, não igualdade: o Defender ativo é 0x061100, byte do meio 0x11.
pub fn tempo_real_ligado(product_state: u32) -> bool {
    const BIT_TEMPO_REAL: u32 = 0x1000;
    product_state & BIT_TEMPO_REAL != 0
}

fn processos_em_execucao() -> Vec<String> {
    let mut sistema = sysinfo::System::new();
    sistema.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    let mut nomes: Vec<String> = sistema
        .processes()
        .values()
        .map(|p| p.name().to_string_lossy().to_lowercase())
        .collect();

    nomes.sort();
    nomes.dedup();
    nomes
}

fn casar_processos(processos: &[String], alvos: &[(&str, &str)]) -> Vec<String> {
    let mut achados: Vec<String> = alvos
        .iter()
        .filter(|(executavel, _)| processos.iter().any(|p| p == executavel))
        .map(|(_, rotulo)| rotulo.to_string())
        .collect();

    achados.sort();
    achados.dedup();
    achados
}

const OTIMIZADORES: [&str; 10] = [
    "driver booster",
    "iobit",
    "advanced systemcare",
    "ccleaner",
    "avast cleanup",
    "avg tuneup",
    "wise care",
    "glary utilities",
    "razer cortex",
    "itop",
];

const SOBREPOSICOES: [(&str, &str); 6] = [
    ("discord.exe", "Discord"),
    ("nvcontainer.exe", "NVIDIA App / GeForce Experience"),
    ("rtss.exe", "RivaTuner Statistics Server"),
    ("msiafterburner.exe", "MSI Afterburner"),
    ("gameoverlayui.exe", "Sobreposição do Steam"),
    ("gamebar.exe", "Xbox Game Bar"),
];

const NUVEM: [(&str, &str); 5] = [
    ("onedrive.exe", "OneDrive"),
    ("googledrivefs.exe", "Google Drive"),
    ("dropbox.exe", "Dropbox"),
    ("megasync.exe", "MEGAsync"),
    ("icloudservices.exe", "iCloud"),
];

pub fn analyze() -> ConflictReport {
    let mut lacunas = Vec::new();

    let programas = match programas_instalados() {
        Ok(programas) => Some(programas),
        Err(erro) => {
            lacunas.push(format!("Programas instalados: {}", erro));
            None
        }
    };
    let processos = processos_em_execucao();
    let mut conflitos = Vec::new();

    let antivirus = match antivirus_ativos() {
        Ok(antivirus) => antivirus,
        Err(erro) => {
            lacunas.push(erro);
            Vec::new()
        }
    };
    if antivirus.len() > 1 {
        conflitos.push(Conflict {
            id: "antivirus".to_string(),
            title: "Mais de um antivírus com proteção em tempo real".to_string(),
            found: antivirus,
            explanation: "Cada antivírus verifica todo arquivo aberto. Com dois ligados, cada \
                          leitura de disco é verificada duas vezes — e um passa a inspecionar o \
                          outro, porque ambos mexem em arquivos o tempo todo."
                .to_string(),
            advice: "Escolha um e desinstale o outro pelo Painel de Controle. É a mudança que \
                     mais devolve desempenho num PC nessa situação, e nenhum ajuste de sistema \
                     substitui. Manter dois não protege mais: eles atrapalham um ao outro."
                .to_string(),
            severity: FindingSeverity::Critical,
        });
    }

    let otimizadores = programas
        .as_deref()
        .map(|programas| casar(programas, &OTIMIZADORES))
        .unwrap_or_default();
    if !otimizadores.is_empty() {
        conflitos.push(Conflict {
            id: "optimizers".to_string(),
            title: "Outro programa de otimização instalado".to_string(),
            found: otimizadores,
            explanation: "Duas ferramentas mexendo nas mesmas configurações desfazem o trabalho \
                          uma da outra. Várias delas também instalam serviço próprio, tarefa \
                          agendada e aviso de renovação — que consomem justamente o que \
                          prometem liberar."
                .to_string(),
            advice: "Não dá para os dois gerenciarem o mesmo PC. Escolha um. Se ficar com o \
                     Otimiza, desinstale o outro para que o plano de energia e os ajustes de \
                     sistema parem de ser revertidos pelas costas."
                .to_string(),
            severity: FindingSeverity::Important,
        });
    }

    let sobreposicoes = casar_processos(&processos, &SOBREPOSICOES);
    if sobreposicoes.len() > 2 {
        conflitos.push(Conflict {
            id: "overlays".to_string(),
            title: "Várias sobreposições ativas ao mesmo tempo".to_string(),
            found: sobreposicoes,
            explanation: "Cada sobreposição injeta código dentro do jogo para desenhar por cima \
                          dele. Uma custa pouco; três disputam o mesmo ponto de entrada e é uma \
                          causa conhecida de engasgo e de fechamento inesperado."
                .to_string(),
            advice: "Deixe ligada a que você realmente usa e desligue as outras nas opções de \
                     cada programa. Não precisa desinstalar — basta desativar a sobreposição."
                .to_string(),
            severity: FindingSeverity::Important,
        });
    }

    let nuvem = casar_processos(&processos, &NUVEM);
    if nuvem.len() > 1 {
        conflitos.push(Conflict {
            id: "cloud".to_string(),
            title: "Mais de um sincronizador de nuvem rodando".to_string(),
            found: nuvem,
            explanation: "Cada cliente de nuvem vigia pastas e lê disco continuamente. Em PC com \
                          disco mecânico ou com pouca memória, dois ou três somam bastante."
                .to_string(),
            advice: "Mantenha rodando o que você usa de verdade e feche os demais na inicialização. \
                     A aba Sistema mostra quais deles sobem com o Windows."
                .to_string(),
            severity: FindingSeverity::Important,
        });
    }

    ConflictReport {
        conflicts: fechar(conflitos, &lacunas),
        programs_scanned: programas.as_ref().map(|programas| programas.len()),
        lacunas,
    }
}

/// O "nenhum conflito" só entra quando tudo foi lido: fabricado com a leitura falha, ele inflava a contagem de
/// verificações aprovadas.
pub fn fechar(mut conflitos: Vec<Conflict>, lacunas: &[String]) -> Vec<Conflict> {
    if conflitos.is_empty() && lacunas.is_empty() {
        conflitos.push(Conflict {
            id: "none".to_string(),
            title: "Nenhum conflito entre programas".to_string(),
            found: Vec::new(),
            explanation: "Não há dois programas disputando a mesma função nesta máquina."
                .to_string(),
            advice: String::new(),
            severity: FindingSeverity::Ok,
        });
    }

    conflitos.sort_by_key(|c| match c.severity {
        FindingSeverity::Critical => 0,
        FindingSeverity::Important => 1,
        FindingSeverity::Ok => 2,
    });

    conflitos
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estado_de_antivirus_ilegivel_nao_vira_antivirus_desligado() {
        let producao = include_str!("conflicts.rs").split("#[cfg(test)]").next().unwrap();
        let corpo = producao
            .split("pub fn antivirus_ativos")
            .nth(1)
            .expect("a função continua existindo");

        assert!(
            !corpo.contains("product_state.unwrap_or"),
            "um `productState` ilegível voltou a ser tratado como antivírus desligado"
        );
    }

    #[test]
    fn nenhum_conflito_so_aparece_quando_tudo_foi_lido() {
        let tudo_lido = fechar(Vec::new(), &[]);
        assert_eq!(tudo_lido.len(), 1);
        assert_eq!(tudo_lido[0].id, "none");

        let com_lacuna = fechar(
            Vec::new(),
            &["Programas instalados: acesso negado".to_string()],
        );
        assert!(com_lacuna.is_empty(), "leitura falha virou achado verde");
    }

    #[test]
    fn le_o_bit_de_protecao_em_tempo_real() {
        assert!(tempo_real_ligado(0x061100), "Defender ativo");
        assert!(tempo_real_ligado(0x041000), "antivírus de terceiro ativo");

        assert!(!tempo_real_ligado(0x060000), "instalado, tempo real desligado");
        assert!(!tempo_real_ligado(0x040000), "instalado, sem proteção ativa");
        assert!(!tempo_real_ligado(0));
    }

    #[test]
    fn casar_encontra_por_pedaco_do_nome() {
        let programas = vec![
            "IObit Driver Booster 12".to_string(),
            "Microsoft Edge".to_string(),
            "CCleaner".to_string(),
        ];

        let achados = casar(&programas, &OTIMIZADORES);
        assert!(achados.iter().any(|p| p.contains("Driver Booster")));
        assert!(achados.iter().any(|p| p == "CCleaner"));
        assert!(!achados.iter().any(|p| p.contains("Edge")));
    }

    #[test]
    fn casar_nao_repete_o_mesmo_programa() {
        // "iobit" e "driver booster" casam com a mesma entrada; ela não pode aparecer duas vezes.
        let programas = vec!["IObit Driver Booster".to_string()];
        assert_eq!(casar(&programas, &OTIMIZADORES).len(), 1);
    }

    #[test]
    fn sem_conflito_o_relatorio_ainda_diz_algo() {
        let vazio: Vec<String> = Vec::new();
        assert!(casar(&vazio, &OTIMIZADORES).is_empty());
    }

    #[test]
    fn le_os_programas_instalados_desta_maquina() {
        let programas = programas_instalados()
            .expect("os programas instalados desta máquina precisam ser legíveis");
        println!("{} programas instalados", programas.len());

        assert!(
            programas.len() > 3,
            "toda máquina com Windows tem mais que três programas registrados"
        );
        assert!(programas.iter().all(|p| !p.trim().is_empty()));
    }

    #[test]
    fn analisa_conflitos_desta_maquina() {
        let r = analyze();
        println!("{:?} programas examinados", r.programs_scanned);

        for c in &r.conflicts {
            println!("  [{:?}] {}", c.severity, c.title);
            for achado in &c.found {
                println!("        - {}", achado);
            }
        }
        for lacuna in &r.lacunas {
            println!("  não li: {}", lacuna);
        }

        assert!(
            !r.conflicts.is_empty() || !r.lacunas.is_empty(),
            "o relatório não pode vir sem conflito e sem lacuna"
        );
        let ordem: Vec<u8> = r
            .conflicts
            .iter()
            .map(|c| match c.severity {
                FindingSeverity::Critical => 0,
                FindingSeverity::Important => 1,
                FindingSeverity::Ok => 2,
            })
            .collect();
        assert!(ordem.windows(2).all(|p| p[0] <= p[1]));
    }
}
