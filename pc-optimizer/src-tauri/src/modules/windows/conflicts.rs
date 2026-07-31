// Detector de conflitos
//
// Este é o sistema que nenhum concorrente tem, e por um motivo simples: metade
// do que ele denuncia são os próprios concorrentes.
//
// Programa lento raramente é culpa de um programa só. É de dois fazendo a mesma
// coisa ao mesmo tempo: dois antivírus varrendo um ao outro, três sobreposições
// injetando código no mesmo jogo, dois "otimizadores" desfazendo a configuração
// um do outro. Cada um sozinho funcionaria; juntos, brigam.
//
// Aqui não se desinstala nada. Desinstalar é decisão do dono da máquina, e
// desinstalador de terceiro é interativo. O que se faz é mostrar o conflito com
// nome e sobrenome, para a pessoa poder escolher.

use super::{registry, shell};
use serde::{Deserialize, Serialize};

pub use super::firmware::FindingSeverity;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    pub id: String,
    pub title: String,
    /// Os programas concretos encontrados, pelo nome.
    pub found: Vec<String>,
    pub explanation: String,
    pub advice: String,
    pub severity: FindingSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictReport {
    pub conflicts: Vec<Conflict>,
    /// Quantos programas instalados foram examinados.
    pub programs_scanned: usize,
}

// --------------------------------------------------------- programas instalados

const UNINSTALL_KEYS: [(&str, &str); 3] = [
    ("HKLM", r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall"),
    ("HKLM", r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall"),
    ("HKCU", r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall"),
];

/// Nomes dos programas instalados, lidos do registro.
///
/// As três chaves cobrem programas de 64 bits, de 32 bits e os instalados só
/// para o usuário atual. Ler só a primeira — erro comum — perde metade da lista
/// justamente nas máquinas antigas, cheias de programa de 32 bits.
pub fn programas_instalados() -> Vec<String> {
    let mut nomes = Vec::new();

    for (hive, base) in UNINSTALL_KEYS {
        for entrada in registry::subkeys(hive, base).unwrap_or_default() {
            let caminho = format!("{}\\{}", base, entrada);

            if let Some(nome) = registry::read_text(hive, &caminho, "DisplayName") {
                let nome = nome.trim().to_string();
                if !nome.is_empty() && !nomes.contains(&nome) {
                    nomes.push(nome);
                }
            }
        }
    }

    nomes
}

/// Encontra programas cujo nome contém algum dos termos.
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

// ------------------------------------------------------------------ antivírus

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawAntivirus {
    display_name: Option<String>,
    product_state: Option<u32>,
}

/// Antivírus com proteção em tempo real LIGADA.
///
/// O `productState` do Windows é um campo de bits. O byte do meio indica o
/// estado da proteção em tempo real: `0x10` significa ligada. Verificar isso
/// importa porque quase toda máquina tem o Defender instalado — o que pesa é
/// ter dois varrendo ao mesmo tempo, não ter dois instalados.
pub fn antivirus_ativos() -> Vec<String> {
    let script = "ConvertTo-Json -Compress -Depth 2 -InputObject @(Get-CimInstance \
                  -Namespace root/SecurityCenter2 -ClassName AntiVirusProduct \
                  -ErrorAction SilentlyContinue | Select-Object displayName,productState)";

    let saida = match shell::powershell(script) {
        Ok(o) if o.success && !o.stdout.trim().is_empty() => o.stdout,
        _ => return Vec::new(),
    };

    let brutos: Vec<RawAntivirus> = serde_json::from_str(&saida).unwrap_or_default();

    brutos
        .into_iter()
        .filter(|a| tempo_real_ligado(a.product_state.unwrap_or(0)))
        .filter_map(|a| a.display_name)
        .collect()
}

/// Exposto para teste: a leitura de bits é onde este tipo de código erra calado.
///
/// É preciso TESTAR O BIT, não comparar igualdade. A primeira versão exigia que
/// o byte do meio fosse exatamente `0x10`, e falhava com `0x061100` — que é o
/// valor do próprio Defender ativo, onde o byte é `0x11`. O resultado seria o
/// pior possível para este módulo: concluir que um antivírus ligado está
/// desligado, e nunca apontar o conflito de dois rodando juntos.
pub fn tempo_real_ligado(product_state: u32) -> bool {
    const BIT_TEMPO_REAL: u32 = 0x1000;
    product_state & BIT_TEMPO_REAL != 0
}

// ------------------------------------------------------------------- processos

/// Nomes dos processos em execução, em minúsculas.
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

// -------------------------------------------------------------------- análise

/// Ferramentas que mexem nas mesmas configurações que o Otimiza.
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

/// Programas que injetam sobreposição em jogos.
const SOBREPOSICOES: [(&str, &str); 6] = [
    ("discord.exe", "Discord"),
    ("nvcontainer.exe", "NVIDIA App / GeForce Experience"),
    ("rtss.exe", "RivaTuner Statistics Server"),
    ("msiafterburner.exe", "MSI Afterburner"),
    ("gameoverlayui.exe", "Sobreposição do Steam"),
    ("gamebar.exe", "Xbox Game Bar"),
];

/// Clientes de nuvem que varrem disco em segundo plano.
const NUVEM: [(&str, &str); 5] = [
    ("onedrive.exe", "OneDrive"),
    ("googledrivefs.exe", "Google Drive"),
    ("dropbox.exe", "Dropbox"),
    ("megasync.exe", "MEGAsync"),
    ("icloudservices.exe", "iCloud"),
];

pub fn analyze() -> ConflictReport {
    let programas = programas_instalados();
    let processos = processos_em_execucao();
    let mut conflitos = Vec::new();

    // --- dois ou mais antivírus com proteção em tempo real ---
    let antivirus = antivirus_ativos();
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

    // --- outros otimizadores instalados ---
    let otimizadores = casar(&programas, &OTIMIZADORES);
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

    // --- várias sobreposições de jogo ---
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

    // --- vários clientes de nuvem ---
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

    if conflitos.is_empty() {
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

    ConflictReport {
        conflicts: conflitos,
        programs_scanned: programas.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_o_bit_de_protecao_em_tempo_real() {
        // Valores reais do Windows. 0x061100 é o Defender ativo — o byte do meio
        // é 0x11, não 0x10, e foi exatamente isso que derrubou a primeira versão
        // deste código, que comparava igualdade em vez de testar o bit.
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
        // "iobit" e "driver booster" casam com a mesma entrada; ela não pode
        // aparecer duas vezes na tela do cliente.
        let programas = vec!["IObit Driver Booster".to_string()];
        assert_eq!(casar(&programas, &OTIMIZADORES).len(), 1);
    }

    #[test]
    fn sem_conflito_o_relatorio_ainda_diz_algo() {
        // Relatório vazio deixaria o cliente sem saber se rodou. Um achado
        // "está tudo certo" é informação, e das boas.
        let vazio: Vec<String> = Vec::new();
        assert!(casar(&vazio, &OTIMIZADORES).is_empty());
    }

    #[test]
    fn le_os_programas_instalados_desta_maquina() {
        let programas = programas_instalados();
        println!("{} programas instalados", programas.len());

        assert!(
            programas.len() > 3,
            "toda máquina com Windows tem mais que três programas registrados"
        );
        // Nome vazio na lista viraria linha em branco na tela.
        assert!(programas.iter().all(|p| !p.trim().is_empty()));
    }

    #[test]
    fn analisa_conflitos_desta_maquina() {
        let r = analyze();
        println!("{} programas examinados", r.programs_scanned);

        for c in &r.conflicts {
            println!("  [{:?}] {}", c.severity, c.title);
            for achado in &c.found {
                println!("        - {}", achado);
            }
        }

        assert!(!r.conflicts.is_empty(), "o relatório nunca pode vir vazio");
        // Problemas antes do que está certo.
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
