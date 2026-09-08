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
    /// A ressalva da categoria, em um de dois usos:
    ///
    /// 1. **O que se perde ao limpar** — o caso comum (`browser_cache`,
    ///    `store_cache`): o cliente precisa saber ANTES de clicar.
    /// 2. **Por que não limpamos por aqui e para onde ir** — quando
    ///    `cleanable` é `false` (`winsxs`, `windows_old`): não há perda a
    ///    avisar, e o que falta ao cliente é o caminho da ferramenta certa.
    ///
    /// Os dois usos convivem de propósito: a regra que o teste
    /// `categoria_perigosa_avisa_e_nao_e_limpa_por_aqui` prende é "toda
    /// categoria não limpável explica o motivo", e ela mora neste campo.
    /// `None` só quando não há nem perda nem motivo — nada a dizer.
    pub warning: Option<String>,
    /// O `bytes` acima foi medido, ou não deu para medir? Ver `Medida`.
    pub medida: Medida,
}

/// O que o número de `bytes` É — para a tela não precisar adivinhar.
///
/// CAMPO TIPADO, E NÃO O PRÓPRIO `bytes`. Sem isto, "não consegui medir" chega
/// à tela como `bytes: 0`, indistinguível de "medi e deu zero" — e a tela
/// pintava um selo verde "vazio" bem ao lado do texto que dizia justamente que
/// não foi possível estimar. É o mesmo defeito que este módulo evita no
/// backend (`estimativa_do_winsxs` devolve `None`, nunca `Some(0)`) escapando
/// uma camada acima, porque `u64` não carrega a distinção.
///
/// E não é caso raro: o `DISM /AnalyzeComponentStore` exige elevação, então
/// TODO cliente que não abrir o Otimiza como administrador cai aqui.
///
/// Tipado e não frase: este projeto reprova o build quando a interface decide
/// comparando prosa vinda do backend — ver a guarda em `commands.rs` e a
/// `Natureza` do `foldermap.rs`, que resolveu o mesmo problema do mesmo jeito.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tipo")]
pub enum Medida {
    /// O número veio de uma medição de verdade. Zero aqui quer dizer zero.
    Medido,
    /// Não deu para medir agora. O `bytes` é 0 por falta de opção, e a tela
    /// NÃO pode tratá-lo como "vazio".
    NaoConsegui,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskReport {
    pub drive: String,
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub free_percent: f64,
    /// O espaço do disco foi medido, ou os zeros acima são falta de opção?
    ///
    /// Sem este campo, quem lê o relatório não tem como saber — e o veredito
    /// chegou a anunciar disco cheio por causa disso. Mesma `Medida` que as
    /// categorias já usavam; a honestidade estava aplicada ao tamanho de cada
    /// pasta e faltava no número principal.
    pub medida_do_espaco: Medida,
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

/// Os ids das categorias que ESTE produto limpa de verdade.
///
/// Existe para o `foldermap.rs` poder conferir, em teste, que cada pasta em
/// que o mapa oferece o botão "Limpar no liberador" chega aqui e encontra uma
/// categoria com botão. Sem esta junção as duas telas divergem em silêncio:
/// marcar uma categoria como `cleanable: false` aqui não reprovava nada lá, e
/// o mapa seguia prometendo uma limpeza que a segunda tela não entrega.
pub fn ids_que_o_liberador_limpa() -> Vec<&'static str> {
    CATEGORIES
        .iter()
        .filter(|c| c.cleanable)
        .map(|c| c.id)
        .collect()
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
        // O `warning` aqui NÃO é "o que se perde" — não se perde nada limpando
        // o que o DISM já marcou como recuperável, e ao contrário do
        // Windows.old não há rollback para perder. É o outro uso do campo (ver
        // `SpaceFinding::warning`): dizer por que a limpeza não acontece por
        // aqui e apontar para onde ela acontece. Sem ele, `winsxs` seria uma
        // categoria não limpável e calada — o que o teste
        // `categoria_perigosa_avisa_e_nao_e_limpa_por_aqui` proíbe.
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
///
/// `None` quando o volume do sistema não aparece na enumeração — drive
/// mapeado, `SystemDrive` apontando para outro lugar, volume sem letra, ou a
/// própria enumeração falhando.
///
/// ANTES ISSO DEVOLVIA `(0, 0)`, e o custo estava na primeira tela: o veredito
/// não tinha como distinguir "não achei o disco" de "o disco está cheio", e
/// anunciava "Restam 0.0 GB livres no disco do Windows" com severidade
/// Critical. Um número inventado, no lugar mais visível do produto, sobre uma
/// máquina que podia estar com meio terabyte livre.
fn disk_usage() -> Option<(u64, u64)> {
    let drive = system_drive();
    let disks = sysinfo::Disks::new_with_refreshed_list();

    for disk in &disks {
        let ponto = disk.mount_point().to_string_lossy().to_uppercase();
        if ponto.starts_with(&drive.to_uppercase()) {
            return Some((disk.total_space(), disk.available_space()));
        }
    }

    None
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
/// — e a tela do liberador precisa responder rápido. Por isso a chamada tem
/// prazo: se o DISM não respondeu a tempo, a categoria diz honestamente que não
/// deu para estimar agora (`Medida::NaoConsegui`), em vez de deixar a tela
/// inteira esperando. Quem quiser o número de qualquer jeito tem a ferramenta
/// de reparo, que roda a mesma análise como tarefa longa com progresso.
///
/// DUAS COISAS QUE ESTA FUNÇÃO PRECISA FAZER ALÉM DE ESPERAR:
///
/// 1. **Matar o DISM ao desistir.** Desistir de esperar não fazia o comando
///    parar: ele seguia até o fim, de 1 a 5 minutos, consumindo disco e CPU de
///    uma máquina que por definição é o "PC fraco" que este produto existe para
///    ajudar. Quem desiste da resposta desiste do trabalho junto.
/// 2. **Não repetir a análise a cada varredura.** Cada clique em "Limpar"
///    refaz o `scan()`, e sem memória disso o cliente acumulava um `Dism.exe`
///    por clique. A última análise vale por `VALIDADE_DA_ANALISE` — o
///    repositório de componentes não muda de tamanho em minutos.
fn saida_do_dism_analyze_component_store() -> Option<String> {
    if let Some(lembrada) = analise_lembrada() {
        return lembrada;
    }

    let saida = match shell::spawn_capturando(
        "Dism",
        &["/Online", "/Cleanup-Image", "/AnalyzeComponentStore"],
    ) {
        Ok(mut filho) => {
            let resultado = saida_com_prazo(&mut filho, PRAZO_DO_DISM);
            // Sem `wait` o processo morto vira zumbi até o Otimiza fechar.
            let _ = filho.wait();
            resultado
        }
        Err(_) => None,
    };

    lembrar_analise(saida.clone());
    saida
}

/// Espera a saída de um processo por um prazo — e, se o prazo estourar, MATA o
/// processo em vez de deixá-lo rodando sozinho.
///
/// Recebe o `Child` emprestado (e não por valor) de propósito: assim quem
/// chamou continua dono do processo e pode conferir, no teste, que ele de fato
/// morreu. É o que prende o conserto — sem o `kill`, o `try_wait` do teste
/// ainda encontra o processo vivo.
///
/// A leitura roda numa thread à parte porque ler o cano até o fim bloqueia até
/// o processo terminar; matar o processo fecha o cano, a leitura termina e a
/// thread morre junto — nada fica pendurado.
fn saida_com_prazo(filho: &mut std::process::Child, prazo: Duration) -> Option<String> {
    // Sem cano não há o que esperar — mas o processo está VIVO. Devolver `None`
    // aqui sem matar seria a única saída da função que não honra o nome dela: o
    // chamador cai no `filho.wait()` logo depois e trava de 1 a 5 minutos
    // esperando o DISM inteiro, que é pior que o defeito que este prazo existe
    // para consertar. Hoje `spawn_capturando` sempre canaliza o stdout, então
    // este ramo não acontece; o dia em que acontecer, ele mata igual.
    let cano = match filho.stdout.take() {
        Some(cano) => cano,
        None => {
            let _ = filho.kill();
            return None;
        }
    };
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let mut bruto = Vec::new();
        let lido = std::io::Read::read_to_end(&mut std::io::BufReader::new(cano), &mut bruto);
        let _ = tx.send(lido.map(|_| String::from_utf8_lossy(&bruto).to_string()));
    });

    match rx.recv_timeout(prazo) {
        // A leitura só termina quando o processo fecha o cano, então neste
        // ponto ele já saiu: `wait` responde na hora e diz se deu certo.
        Ok(Ok(texto)) => match filho.wait() {
            Ok(status) if status.success() => Some(texto),
            _ => None,
        },
        _ => {
            let _ = filho.kill();
            None
        }
    }
}

/// Quanto o liberador espera pelo DISM antes de desistir e matar.
const PRAZO_DO_DISM: Duration = Duration::from_secs(3);

/// Por quanto tempo a última análise do DISM continua valendo.
///
/// O repositório de componentes só muda quando o Windows aplica ou remove
/// atualização — não em minutos, e não por causa de um clique em "Limpar".
const VALIDADE_DA_ANALISE: Duration = Duration::from_secs(600);

/// A última análise e quando ela foi feita. `Option<String>` por dentro porque
/// a FALHA também se lembra: sem admin o DISM falha sempre, e repetir a falha a
/// cada clique é o mesmo desperdício com outro nome.
static ULTIMA_ANALISE: std::sync::Mutex<Option<(std::time::Instant, Option<String>)>> =
    std::sync::Mutex::new(None);

/// `true` enquanto uma análise feita em `quando` ainda vale para `agora`.
///
/// Função à parte, e não um `if` solto lá dentro, para o prazo ser testável sem
/// esperar dez minutos de relógio.
fn ainda_vale(quando: std::time::Instant, agora: std::time::Instant) -> bool {
    agora.duration_since(quando) < VALIDADE_DA_ANALISE
}

/// A análise lembrada, se ainda valer. `None` externo = precisa rodar o DISM.
fn analise_lembrada() -> Option<Option<String>> {
    let guarda = ULTIMA_ANALISE.lock().ok()?;
    let (quando, saida) = guarda.as_ref()?;

    if ainda_vale(*quando, std::time::Instant::now()) {
        Some(saida.clone())
    } else {
        None
    }
}

fn lembrar_analise(saida: Option<String>) {
    if let Ok(mut guarda) = ULTIMA_ANALISE.lock() {
        *guarda = Some((std::time::Instant::now(), saida));
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

    let (bytes, medida, explanation, formatted) = match estimativa {
        Some(bytes) => (
            bytes,
            Medida::Medido,
            c.explanation.to_string(),
            format_size(bytes),
        ),
        None => (
            0,
            // O `bytes: 0` daqui é falta de opção, não medição. Sem este
            // `NaoConsegui` a tela lê o zero e conclui "vazio" — ver `Medida`.
            Medida::NaoConsegui,
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
        medida,
    }
}

/// Monta o achado do WinSxS rodando o DISM de verdade. Fininha de propósito:
/// toda a lógica testável está em `finding_do_winsxs_a_partir_da_saida`.
fn finding_do_winsxs(c: &Category) -> SpaceFinding {
    let saida = saida_do_dism_analyze_component_store();
    finding_do_winsxs_a_partir_da_saida(c, saida.as_deref())
}

/// Varre todas as categorias, com a análise do DISM. Não apaga nada.
///
/// É o que a TELA do liberador chama: quem abriu a aba Espaço quer o número do
/// WinSxS e aceita esperar por ele.
pub fn scan() -> DiskReport {
    varrer(true)
}

/// A mesma varredura, sem chamar o DISM. É o que o veredito geral usa.
///
/// O `diagnostico_rapido()` roda SOZINHO na abertura do app, sem o cliente
/// pedir nada — e do relatório de disco ele só olha o espaço livre e o que é
/// LIMPÁVEL (ver `impl EmAchados for DiskReport`). O `winsxs` não é limpável,
/// então a análise do DISM não muda uma vírgula do veredito: era um comando de
/// minutos rodando na abertura para alimentar um número que ninguém lia.
pub fn scan_para_o_veredito() -> DiskReport {
    varrer(false)
}

fn varrer(medir_o_winsxs: bool) -> DiskReport {
    let findings: Vec<SpaceFinding> = CATEGORIES
        .iter()
        .map(|c| {
            if c.id == "winsxs" {
                return if medir_o_winsxs {
                    finding_do_winsxs(c)
                } else {
                    // Sem DISM não há estimativa — e o achado diz isso, em vez
                    // de inventar um zero.
                    finding_do_winsxs_a_partir_da_saida(c, None)
                };
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
                // Somar pastas sempre dá um número: aqui zero é zero de
                // verdade, e a tela pode dizer "vazio" sem mentir.
                medida: Medida::Medido,
            }
        })
        .collect();

    let medido = disk_usage();
    let (total_bytes, free_bytes) = medido.unwrap_or((0, 0));
    let medida_do_espaco = if medido.is_some() {
        Medida::Medido
    } else {
        Medida::NaoConsegui
    };
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
        medida_do_espaco,
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

    /// TRAVA SÓ DO LADO DO TESTE — NÃO É PRODUÇÃO.
    ///
    /// `ULTIMA_ANALISE` é um `static` de produção, compartilhado por todo
    /// teste que chama `scan()` (que lê e escreve nele) ou mexe nele direto.
    /// `cargo test` roda em threads paralelas por padrão, então sem isto há
    /// uma corrida de verdade: um teste pode ler a memória que outro acabou
    /// de plantar ou apagar. Um laço de tentativas (como havia aqui antes)
    /// não elimina essa corrida, só reduz a chance dela aparecer — e um teste
    /// que falha uma vez a cada tantas é pior que um que nunca passa: todo
    /// mundo aprende a reexecutar sem investigar. A trava é só `std::sync`,
    /// sem dependência nova, e não move nada de `ULTIMA_ANALISE` nem de
    /// `saida_do_dism_analyze_component_store` — a produção continua igual.
    static TRAVA_ULTIMA_ANALISE: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Se um teste entrar em pânico segurando `TRAVA_ULTIMA_ANALISE`, o
    /// `Mutex` fica envenenado e os testes seguintes falhariam por
    /// "poisoned" em vez de pelo motivo real deles. Aqui a trava não guarda
    /// dado nenhum (é um `()`) — só serializa acesso —, então é seguro pegar
    /// a guarda de dentro do erro e seguir em frente.
    fn trava_ultima_analise() -> std::sync::MutexGuard<'static, ()> {
        TRAVA_ULTIMA_ANALISE
            .lock()
            .unwrap_or_else(|envenenada| envenenada.into_inner())
    }

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
        // Trava porque `scan()` lê/escreve `ULTIMA_ANALISE`, memória
        // compartilhada com as outras provas que também chamam `scan()`.
        let _trava = trava_ultima_analise();

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
        // Trava porque `scan()` lê/escreve `ULTIMA_ANALISE`, memória
        // compartilhada com as outras provas que também chamam `scan()`.
        let _trava = trava_ultima_analise();

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
    fn winsxs_sem_estimativa_chega_na_tela_como_nao_medido_e_nao_como_vazio() {
        // O ZERO PRECISA CHEGAR ROTULADO NA TELA.
        //
        // `estimativa_do_winsxs` já devolve `None` em vez de `Some(0)` — mas
        // `bytes: u64` não carrega essa distinção até a interface, e a tela
        // acabava mostrando um selo verde "vazio" ao lado do texto que dizia
        // "não foi possível estimar". O `Medida` é o que atravessa.
        let categoria = CATEGORIES
            .iter()
            .find(|c| c.id == "winsxs")
            .expect("categoria winsxs existe");

        let sem_dism = finding_do_winsxs_a_partir_da_saida(categoria, None);
        assert_eq!(sem_dism.bytes, 0);
        assert_eq!(
            sem_dism.medida,
            Medida::NaoConsegui,
            "sem a análise do DISM o achado precisa dizer que NÃO MEDIU"
        );

        let com_dism = finding_do_winsxs_a_partir_da_saida(categoria, Some("Recuperável : 2.34 GB
"));
        assert_eq!(com_dism.medida, Medida::Medido);
    }

    #[test]
    fn as_outras_categorias_medem_de_verdade() {
        // Somar pastas sempre dá um número: se alguma categoria comum chegasse
        // como `NaoConsegui`, a tela esconderia um "vazio" legítimo.
        for f in scan_para_o_veredito().findings.iter().filter(|f| f.id != "winsxs") {
            assert_eq!(f.medida, Medida::Medido, "{} não é medição do disco", f.id);
        }
    }

    /// A TELA NÃO PODE VOLTAR A DECIDIR "VAZIO" OLHANDO SÓ O `bytes`.
    ///
    /// Guarda por leitura do fonte, como a da prosa em `commands.rs`: o defeito
    /// não estava no Rust, estava em `renderSpaceFinding` — e um teste de Rust
    /// que só olhasse o `Medida` passaria com a tela quebrada do mesmo jeito.
    ///
    /// A regra que ela prende: nem o rótulo (`state-label`) nem a severidade
    /// (`data-severity`) saem de `bytes`, e a função consulta `medida`.
    ///
    /// MAS ELA SOZINHA NÃO BASTA, e isto não é teoria: dá para passar por ela
    /// guardando a decisão errada numa variável intermediária (`const vazio =
    /// semNada;`) e citando `medida` para qualquer coisa cosmética. Quem prende
    /// o comportamento é
    /// `a_tela_do_liberador_renderiza_diferente_o_nao_medido_e_o_zero_medido`,
    /// que RODA a função. Esta continua aqui porque é barata e pega a
    /// regressão literal antes de subir um Node.
    #[test]
    fn a_tela_do_liberador_nao_chama_de_vazio_o_que_nao_foi_medido() {
        let caminho = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("src")
            .join("main.ts");
        let fonte = std::fs::read_to_string(&caminho)
            .unwrap_or_else(|e| panic!("não consegui ler {:?}: {}", caminho, e));

        let corpo: String = fonte
            .split("function renderSpaceFinding")
            .nth(1)
            .expect("renderSpaceFinding precisa existir")
            .lines()
            .take_while(|l| !l.starts_with('}'))
            .collect::<Vec<_>>()
            .join("
");

        assert!(
            corpo.contains("medida"),
            "renderSpaceFinding não olha `medida`: voltou a tratar todo zero como vazio"
        );

        for linha in corpo.lines() {
            let decide = linha.contains("state-label") || linha.contains("data-severity");
            assert!(
                !(decide && linha.contains("bytes")),
                "o rótulo/severidade do liberador voltou a sair de `bytes`:
  {}",
                linha.trim()
            );
        }
    }

    #[test]
    fn desistir_de_esperar_mata_o_processo_em_vez_de_deixar_rodando() {
        // O DEFEITO: o `recv_timeout` devolvia o controle, mas o `Dism.exe`
        // seguia até o fim — de 1 a 5 minutos de disco e CPU numa máquina que,
        // por definição, é o "PC fraco" que este produto existe para ajudar. E
        // como cada clique em "Limpar" refazia a varredura, eles empilhavam.
        //
        // `ping -n 30` no lugar do DISM: um processo que demora muito mais que
        // o prazo, sem precisar de administrador nem mexer no sistema.
        let mut filho = shell::spawn_capturando("ping", &["-n", "30", "127.0.0.1"])
            .expect("o ping do Windows sobe");

        let inicio = std::time::Instant::now();
        let saida = saida_com_prazo(&mut filho, Duration::from_millis(300));

        assert!(saida.is_none(), "o prazo estourou; não podia vir saída");
        assert!(
            inicio.elapsed() < Duration::from_secs(10),
            "não desistiu no prazo: esperou {:?}",
            inicio.elapsed()
        );

        // A prova: o processo precisa estar MORTO agora. Sem o `kill`, ele
        // continuaria vivo aqui pelos ~30 s do ping.
        let mut morreu = false;
        for _ in 0..100 {
            if matches!(filho.try_wait(), Ok(Some(_))) {
                morreu = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let _ = filho.kill();

        assert!(
            morreu,
            "desistimos de esperar e o processo continuou rodando sozinho"
        );
    }

    #[test]
    fn a_analise_do_dism_e_lembrada_e_nao_refeita_a_cada_clique() {
        // Cada clique em "Limpar" refaz o `scan()`. Sem memória da última
        // análise, era um `Dism.exe` por clique.
        let agora = std::time::Instant::now();
        assert!(ainda_vale(agora, agora), "a análise recém-feita vale");

        let velha = agora
            .checked_sub(VALIDADE_DA_ANALISE + Duration::from_secs(1))
            .expect("o relógio da máquina tem passado suficiente");
        assert!(
            !ainda_vale(velha, agora),
            "análise mais velha que a validade precisa ser refeita"
        );
    }

    /// O TESTE ACIMA SÓ OLHA O RELÓGIO — ESTE OLHA O CONSERTO.
    ///
    /// `ainda_vale` é comparação pura de `Instant`: apagar o early-return de
    /// `saida_do_dism_analyze_component_store` inteiro deixava a suíte verde,
    /// com um `warning: function analise_lembrada is never used` como único
    /// sinal — e warning não reprova build. Ou seja, o conserto que impede um
    /// `Dism.exe` por clique podia ser removido sem nada notar.
    ///
    /// A prova é por SENTINELA: planta na memória uma saída que o DISM de
    /// verdade nunca produziria e chama a função. Se ela devolver a sentinela,
    /// consultou a memória; se devolver qualquer outra coisa, subiu o processo.
    #[test]
    fn a_analise_lembrada_volta_da_memoria_sem_subir_o_dism_de_novo() {
        const SENTINELA: &str = "SENTINELA DA MEMORIA (nao veio de Dism.exe)\nRecuperável : 1.00 GB\n";

        // Trava porque este teste planta na `ULTIMA_ANALISE` compartilhada e
        // depende de ninguém mais mexer nela entre plantar e ler — as provas
        // que chamam `scan()` fazem exatamente isso. Sem a trava havia uma
        // corrida de verdade (não só teórica) e um laço de tentativas em cima
        // dela, que reduzia a chance de pegar a corrida sem eliminá-la: um
        // teste flaky é o mesmo problema que este projeto já levou a sério
        // demais para tolerar — a suíte para de ser prova.
        let _trava = trava_ultima_analise();

        *ULTIMA_ANALISE
            .lock()
            .expect("a memória da análise não está envenenada") =
            Some((std::time::Instant::now(), Some(SENTINELA.to_string())));

        let inicio = std::time::Instant::now();
        let saida = saida_do_dism_analyze_component_store();
        let gasto = inicio.elapsed();

        assert_eq!(
            saida.as_deref(),
            Some(SENTINELA),
            "a análise lembrada foi ignorada: a função subiu o DISM de novo em vez de \
             devolver o que já estava na memória — é um Dism.exe por clique em Limpar",
        );

        // E precisa voltar NA HORA: se tivesse esperado o prazo do DISM, teria
        // rodado o comando e a economia não existiria.
        assert!(
            gasto < PRAZO_DO_DISM,
            "a resposta lembrada levou {:?} — esperou pelo DISM em vez de lembrar",
            gasto
        );

        // Não deixa a sentinela para as outras provas: `varre_esta_maquina`
        // imprime o número do WinSxS, e ele tem de ser o desta máquina.
        *ULTIMA_ANALISE
            .lock()
            .expect("a memória da análise não está envenenada") = None;
    }

    /// UMA LINHA DE ATRIBUTO É O CONTRATO INTEIRO COM A TELA.
    ///
    /// Sem o `#[serde(tag = "tipo")]` do `Medida`, o enum serializa como a
    /// string `"NaoConsegui"`, `item.medida.tipo` vira `undefined` na tela, o
    /// `naoMedido` vira `false` — e o achado não medido volta a ser um selo
    /// verde "vazio". É o Crítico 1 idêntico, ressuscitado por uma linha, com
    /// `cargo test` e `tsc` limpos. Este teste afirma a FORMA do JSON.
    #[test]
    fn a_medida_chega_na_tela_como_objeto_com_campo_tipo() {
        assert_eq!(
            serde_json::to_string(&Medida::NaoConsegui).expect("Medida serializa"),
            r#"{"tipo":"NaoConsegui"}"#
        );
        assert_eq!(
            serde_json::to_string(&Medida::Medido).expect("Medida serializa"),
            r#"{"tipo":"Medido"}"#
        );

        // E dentro do achado, que é o que a tela recebe de verdade.
        let categoria = CATEGORIES
            .iter()
            .find(|c| c.id == "winsxs")
            .expect("categoria winsxs existe");
        let json = serde_json::to_string(&finding_do_winsxs_a_partir_da_saida(categoria, None))
            .expect("o achado serializa");
        assert!(
            json.contains(r#""medida":{"tipo":"NaoConsegui"}"#),
            "o achado não chega com `medida.tipo` na tela: {}",
            json
        );
    }

    /// A GUARDA POR LEITURA DO FONTE FOI BURLADA — ESTA EXECUTA A TELA.
    ///
    /// `a_tela_do_liberador_nao_chama_de_vazio_o_que_nao_foi_medido` lê o texto
    /// do `main.ts`, e o revisor passou por ela em quatro linhas: guardou a
    /// decisão errada numa variável intermediária (`const vazio = semNada;`),
    /// usou `medida` para algo cosmético e pronto — guarda verde, `tsc` limpo,
    /// defeito inteiro de volta na tela.
    ///
    /// Aqui não tem texto para enganar: extrai `renderSpaceFinding` do
    /// `main.ts`, transpila com o próprio TypeScript do projeto, RODA a função
    /// com dois achados idênticos exceto pela `medida` e olha o HTML que sai.
    /// Reescrever a decisão de qualquer jeito que produza a tela errada reprova.
    #[test]
    fn a_tela_do_liberador_renderiza_diferente_o_nao_medido_e_o_zero_medido() {
        let raiz = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
        let main_ts = raiz.join("src").join("main.ts");
        let typescript = raiz.join("node_modules").join("typescript");
        assert!(
            typescript.is_dir(),
            "esta prova roda a tela de verdade e precisa do TypeScript do projeto: \
             rode `npm ci` em pc-optimizer ({:?} não existe)",
            typescript
        );

        // O laboratório: só o `escapeHtml` é dublê (o de verdade usa `document`,
        // que não existe no Node). A decisão de rótulo e severidade é a do
        // `main.ts`, sem uma linha reescrita aqui.
        let laboratorio = r#"
const fs = require("fs");
const ts = require(process.argv[3]);

const fonte = fs.readFileSync(process.argv[2], "utf8");
const depois = fonte.split("function renderSpaceFinding")[1];
if (depois === undefined) throw new Error("renderSpaceFinding nao existe no main.ts");

const corpo = [];
for (const linha of depois.split(/\r?\n/)) {
  if (linha.startsWith("}")) break;
  corpo.push(linha);
}
const trecho = "function renderSpaceFinding" + corpo.join("\n") + "\n}\n";
const js = ts.transpileModule(trecho, { compilerOptions: { target: "ES2020" } }).outputText;

const escapeHtml = (v) =>
  String(v).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
const render = new Function("escapeHtml", js + "\nreturn renderSpaceFinding;")(escapeHtml);

// Os dois achados são IGUAIS em tudo — inclusive `bytes: 0` e o mesmo texto
// formatado. A única diferença é a `medida`. Qualquer diferença no HTML só
// pode ter vindo dela.
const base = {
  id: "winsxs",
  name: "Repositorio de componentes",
  explanation: "explicacao",
  bytes: 0,
  formatted: "--",
  cleanable: false,
  requires_admin: true,
  warning: null,
};

console.log(
  JSON.stringify({
    naoMedido: render(Object.assign({}, base, { medida: { tipo: "NaoConsegui" } })),
    medido: render(Object.assign({}, base, { medida: { tipo: "Medido" } })),
  })
);
"#;

        let script = std::env::temp_dir().join("otimiza_render_space_finding.cjs");
        std::fs::write(&script, laboratorio).expect("escreve o laboratório no temporário");

        let saida = std::process::Command::new("node")
            .arg(&script)
            .arg(&main_ts)
            .arg(&typescript)
            .output()
            .expect("esta prova roda a tela de verdade e precisa do Node no PATH");
        let _ = std::fs::remove_file(&script);

        assert!(
            saida.status.success(),
            "não deu para rodar `renderSpaceFinding`: {}",
            String::from_utf8_lossy(&saida.stderr)
        );

        let telas: serde_json::Value =
            serde_json::from_slice(&saida.stdout).expect("o laboratório imprime JSON");
        let nao_medido = telas["naoMedido"].as_str().expect("html do não medido");
        let medido = telas["medido"].as_str().expect("html do zero medido");

        assert_ne!(
            nao_medido, medido,
            "a tela pinta IGUAL o que não foi medido e o zero medido — o cliente \
             não tem como saber a diferença"
        );

        // O zero medido é o caso resolvido: verde e "vazio".
        assert!(
            medido.contains(r#"data-severity="Ok""#),
            "zero MEDIDO é assunto resolvido e precisa sair em Ok:\n{}",
            medido
        );
        assert!(
            medido.contains(">vazio<"),
            "zero MEDIDO é vazio de verdade e precisa dizer isso:\n{}",
            medido
        );

        // E o não medido é assunto pendente: nem verde, nem "vazio".
        assert!(
            !nao_medido.contains(r#"data-severity="Ok""#),
            "o que não foi medido saiu com selo verde de resolvido:\n{}",
            nao_medido
        );
        assert!(
            !nao_medido.contains("vazio"),
            "o que não foi medido saiu como \"vazio\" — o produto afirmando o que \
             não mediu:\n{}",
            nao_medido
        );
    }

    #[test]
    fn a_estimativa_entende_virgula_decimal_do_windows_em_portugues() {
        // O DISM de um Windows em português escreve "2,34 GB", com VÍRGULA.
        // Todos os outros testes usam ponto — sem este, um refactor que
        // apagasse o `replace(',', ".")` passaria batido e a categoria sumiria
        // justamente para o cliente brasileiro, que é o público do produto.
        let com_virgula = estimativa_do_winsxs("Recuperável : 2,34 GB
")
            .expect("a saída em português usa vírgula decimal");
        let com_ponto = estimativa_do_winsxs("Recuperável : 2.34 GB
").expect("e a com ponto");

        assert_eq!(com_virgula, com_ponto, "vírgula e ponto medem o mesmo espaço");
        assert!(com_virgula > 2_400_000_000, "veio {}", com_virgula);
    }

    #[test]
    fn varre_esta_maquina() {
        // Trava porque `scan()` lê/escreve `ULTIMA_ANALISE`, memória
        // compartilhada com as outras provas que também chamam `scan()`.
        let _trava = trava_ultima_analise();

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
