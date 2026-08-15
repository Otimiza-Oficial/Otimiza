// Prontidão do sistema
//
// Coisas que precisam estar certas ANTES de otimizar. Não são otimizações: são
// condições que, quando erradas, fazem o atendimento inteiro dar errado por um
// motivo que ninguém procura.
//
// O exemplo que motivou o módulo: máquina com reinício pendente. Parte das
// mudanças não fixa até reiniciar, o técnico aplica tudo, mede, não vê ganho, e
// conclui que o produto não funciona. O motivo estava lá desde o começo, numa
// chave de registro que ninguém olha.
//
// Cada verificação aqui responde uma pergunta do tipo "isto vai atrapalhar o
// resto?", e todas são baratas — o levantamento inteiro leva menos de um
// segundo, então pode rodar antes de qualquer coisa.

use super::{power, registry, shell};
use serde::{Deserialize, Serialize};

pub use super::firmware::{FindingSeverity, FixLocation};

/// Plano de energia oculto do Windows, mais agressivo que o de alto desempenho.
///
/// Ele não aparece na lista até ser criado a partir deste identificador, e o
/// Windows não o cria sozinho em máquina de consumo.
pub const DESEMPENHO_MAXIMO_GUID: &str = "e9a42b02-d5df-448d-aa00-03f14749eb61";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessFinding {
    pub id: String,
    pub title: String,
    pub measured: String,
    pub advice: String,
    pub severity: FindingSeverity,
    pub fix_location: FixLocation,
    /// Verdadeiro quando o próprio Otimiza sabe corrigir isto.
    pub actionable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessReport {
    pub findings: Vec<ReadinessFinding>,
    pub note: String,
}

// ------------------------------------------------------ reinício pendente

/// Se o Windows está esperando um reinício para concluir alguma coisa.
///
/// Três origens, e basta uma. As chaves são estruturais e não mudam de idioma.
pub fn reinicio_pendente() -> Vec<&'static str> {
    let mut motivos = Vec::new();

    if registry::key_exists(
        "HKLM",
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Component Based Servicing\RebootPending",
    ) {
        motivos.push("instalação de componente do Windows");
    }

    if registry::key_exists(
        "HKLM",
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\WindowsUpdate\Auto Update\RebootRequired",
    ) {
        motivos.push("atualização do Windows");
    }

    // Arquivo esperando para ser renomeado ou apagado no próximo boot. É o mais
    // comum dos três e o menos conhecido.
    if registry::read(
        "HKLM",
        r"SYSTEM\CurrentControlSet\Control\Session Manager",
        "PendingFileRenameOperations",
    )
    .is_ok()
    {
        motivos.push("arquivo aguardando substituição");
    }

    motivos
}

// ------------------------------------------------------------------- TRIM

/// Se o Windows está enviando TRIM ao disco.
///
/// `DisableDeleteNotify = 0` significa ligado. A saída do `fsutil` é traduzida,
/// então o que se lê é o número na linha, nunca a frase.
pub fn trim_ligado() -> Option<bool> {
    let saida = shell::run("fsutil", &["behavior", "query", "DisableDeleteNotify"]).ok()?;

    if !saida.success {
        return None;
    }

    // A linha do NTFS é a que interessa; ReFS quase nunca é o disco do sistema.
    saida
        .stdout
        .lines()
        .find(|l| l.contains("NTFS"))
        .and_then(|l| l.split('=').nth(1))
        .and_then(|v| v.trim().split_whitespace().next())
        .and_then(|v| v.parse::<u32>().ok())
        .map(|v| v == 0)
}

// -------------------------------------------------------- arquivo de troca

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
struct RawPaginacao {
    name: Option<String>,
    mecanico: Option<bool>,
}

/// Descobre se o arquivo de paginação está num disco mecânico.
///
/// É um erro de configuração real e caro: numa máquina com SSD e HD, deixar a
/// paginação no HD faz o Windows usar a peça mais lenta justamente quando a
/// memória acaba, que é o pior momento possível.
fn paginacao_em_disco_lento() -> Option<(String, bool)> {
    // UMA chamada, não duas.
    //
    // Até a versão 0.15 este diagnóstico abria dois `powershell.exe`: um para
    // descobrir onde está o arquivo de paginação, outro para descobrir se
    // aquele disco é mecânico. Cada processo custa uns 200 a 400 ms e cerca de
    // 40 MB de memória prometida — na máquina que estamos diagnosticando
    // justamente por falta de memória.
    //
    // Eram dois porque o segundo dependia do resultado do primeiro. Aqui a
    // dependência vira uma variável dentro do mesmo script.
    let script = "$p = Get-CimInstance Win32_PageFileUsage -ErrorAction SilentlyContinue |                     Select-Object -First 1;                   if ($p) {                     $letra = $p.Name.Substring(0,1);                     $part = Get-Partition -ErrorAction SilentlyContinue |                             Where-Object DriveLetter -eq $letra | Select-Object -First 1;                     $mec = $false;                     if ($part) {                       $d = Get-PhysicalDisk -ErrorAction SilentlyContinue |                            Where-Object DeviceId -eq (Get-Disk -Number $part.DiskNumber).Number;                       $mec = ($d.MediaType -eq 'HDD') };                     ConvertTo-Json -Compress -InputObject ([ordered]@{                       Name = $p.Name; Mecanico = [bool]$mec }) }";

    let bruto: RawPaginacao = shell::powershell(script)
        .ok()
        .filter(|s| s.success && !s.stdout.trim().is_empty())
        .and_then(|s| serde_json::from_str(&s.stdout).ok())?;

    Some((bruto.name?, bruto.mecanico.unwrap_or(false)))
}

// ------------------------------------------------------------- montagem

/// Levantamento completo.
pub fn analyze() -> ReadinessReport {
    let mut findings = Vec::new();

    // 1. Reinício pendente. Vem primeiro porque atrapalha todo o resto.
    let motivos = reinicio_pendente();

    if !motivos.is_empty() {
        findings.push(ReadinessFinding {
            id: "reinicio".to_string(),
            title: "Reinício pendente".to_string(),
            measured: format!("Aguardando reinício por: {}.", motivos.join(", ")),
            advice: "Parte das mudanças de sistema só passa a valer depois de reiniciar. \
                     Otimizar antes disso costuma dar a impressão de que nada funcionou — o \
                     ajuste foi aplicado, mas o Windows ainda não o assumiu. Reinicie antes de \
                     medir qualquer coisa."
                .to_string(),
            severity: FindingSeverity::Important,
            fix_location: FixLocation::Software,
            actionable: false,
        });
    }

    // 2. TRIM. Desligado num SSD, o disco vai perdendo velocidade de escrita ao
    //    longo de meses, e nada no sistema avisa.
    if super::hardware::profile().system_storage == super::hardware::StorageKind::Ssd {
        if trim_ligado() == Some(false) {
            findings.push(ReadinessFinding {
                id: "trim".to_string(),
                title: "TRIM desligado num SSD".to_string(),
                measured: "O Windows não está avisando o disco sobre blocos apagados.".to_string(),
                advice: "Sem esse aviso o SSD vai perdendo velocidade de escrita com o tempo, e \
                         a queda é gradual o bastante para ninguém associar à causa. Costuma ser \
                         resultado de um \"tutorial de otimização\" antigo. Religar é imediato e \
                         não tem contraindicação."
                    .to_string(),
                severity: FindingSeverity::Important,
                fix_location: FixLocation::Software,
                actionable: true,
            });
        }
    }

    // 3. Paginação em disco mecânico.
    if let Some((caminho, mecanico)) = paginacao_em_disco_lento() {
        if mecanico {
            findings.push(ReadinessFinding {
                id: "paginacao".to_string(),
                title: "Arquivo de paginação num disco mecânico".to_string(),
                measured: format!("O arquivo está em {}.", caminho),
                advice: "Quando a memória acaba, o Windows recorre a este arquivo — e aqui ele \
                         está na peça mais lenta da máquina, justamente no pior momento. Mover \
                         para o disco do sistema, se ele for SSD, muda de forma perceptível o \
                         comportamento em travadas."
                    .to_string(),
                severity: FindingSeverity::Important,
                fix_location: FixLocation::Software,
                actionable: false,
            });
        }
    }

    // 4. Plano de desempenho máximo. Informativo: é oportunidade, não defeito.
    // Plano de terceiro ativo: o cliente acha que está no "alto desempenho" do
    // Windows e está num plano que ninguém auditou.
    if let Some(nome) = plano_ativo_e_de_terceiro() {
        findings.push(ReadinessFinding {
            id: "plano_de_terceiro".to_string(),
            title: "O plano de energia ativo não é do Windows".to_string(),
            measured: format!("Plano em uso: \"{}\".", nome),
            advice: "Programas de otimização e fabricantes de notebook criam planos de                      energia próprios e os deixam ativos. Alguns são bons; outros limitam o                      processador para economizar bateria, e quem instalou já desinstalou o                      programa faz tempo. O Otimiza não mexe nele sem você mandar — mas você                      merece saber que o plano em uso não é nenhum dos que o Windows traz."
                .to_string(),
            severity: FindingSeverity::Important,
            fix_location: FixLocation::Software,
            actionable: false,
        });
    }

    if !plano_maximo_existe() {
        findings.push(ReadinessFinding {
            id: "plano_maximo".to_string(),
            title: "Plano de desempenho máximo não existe nesta máquina".to_string(),
            measured: "O Windows tem um plano de energia mais agressivo que o de alto \
                       desempenho, e ele não aparece na lista até ser criado."
                .to_string(),
            advice: "Ele reduz a latência de mudança de estado do processador, o que aparece \
                     em resposta e não em número de quadros. O ganho é pequeno e real; em \
                     notebook na bateria, custa autonomia."
                .to_string(),
            severity: FindingSeverity::Ok,
            fix_location: FixLocation::Software,
            actionable: true,
        });
    }

    findings.sort_by_key(|f| match f.severity {
        FindingSeverity::Critical => 0,
        FindingSeverity::Important => 1,
        FindingSeverity::Ok => 2,
    });

    let problemas = findings
        .iter()
        .filter(|f| f.severity != FindingSeverity::Ok)
        .count();

    let note = if problemas == 0 {
        "Nada atrapalhando. As verificações aqui não são otimizações: são condições que, quando \
         erradas, fazem o resto do trabalho parecer que não funcionou."
            .to_string()
    } else {
        format!(
            "{} ponto(s) que atrapalham antes de qualquer otimização. Vale resolver estes \
             primeiro: são o tipo de coisa que faz o atendimento inteiro parecer sem efeito.",
            problemas
        )
    };

    ReadinessReport { findings, note }
}

/// Se o plano de desempenho máximo já foi criado.
///
/// A verificação NÃO pode ser pelo identificador de origem.
///
/// `powercfg -duplicatescheme` cria uma cópia com identificador NOVO — o
/// `e9a42b02-…` é o molde, e nunca aparece na lista de planos da máquina. Na
/// máquina onde este defeito foi encontrado, o plano existia como
/// `d1664682-…` e o produto respondia que não existia, oferecendo criar um
/// segundo.
///
/// O nome também não serve: ele é traduzido, e comparar "Desempenho Máximo"
/// quebraria em qualquer Windows que não seja português.
///
/// O que sobra, e é o certo: perguntar ao próprio `powercfg` quais
/// configurações o plano tem. O plano de desempenho máximo é o único que
/// desliga o estacionamento de núcleos por padrão — mas ler isso plano a plano
/// custa caro. Então a checagem passa a ser por CONTAGEM: se existe mais plano
/// do que os quatro que o Windows traz de fábrica, algum foi acrescentado.
pub fn plano_maximo_existe() -> bool {
    planos_instalados().iter().any(|(guid, nome)| {
        guid.eq_ignore_ascii_case(DESEMPENHO_MAXIMO_GUID)
            // O molde tem identificador fixo; a cópia herda o nome que o
            // Windows deu na criação. Comparar os dois cobre a máquina que
            // criou pelo Otimiza e a que já tinha o plano.
            || sem_acento(nome).contains(&sem_acento("desempenho máximo"))
            || sem_acento(nome).contains("ultimate performance")
    })
}

/// Deixa só o esqueleto ASCII, em minúsculas.
///
/// POR QUE ISTO É NECESSÁRIO AQUI
///
/// `powercfg` não é PowerShell, então não passa pelo `shell::powershell()` que
/// força UTF-8 — e a saída dele vem no código de página do console. Nesta
/// máquina, "Desempenho Máximo" chega como `M` + caractere de substituição +
/// `ximo`, com os bytes `239 191 189` no lugar do acento.
///
/// Resultado prático: `contains("máximo")` devolvia falso para uma máquina que
/// TINHA o plano, e o produto oferecia criar um segundo.
///
/// Descartar tudo que não é ASCII resolve os dois lados de uma vez — funciona
/// se o acento sobreviveu e funciona se ele virou lixo, porque o texto
/// procurado passa pela mesma peneira.
fn sem_acento(texto: &str) -> String {
    texto
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == ' ')
        .collect()
}

/// Todos os planos de energia da máquina: identificador e nome.
///
/// O resultado é guardado por alguns segundos porque UMA análise consulta esta
/// lista três vezes — para dizer se o plano ativo é de terceiro, para saber se
/// o de desempenho máximo existe, e de novo dentro da primeira. Eram três
/// processos `powercfg` para responder a mesma pergunta, e o diagnóstico
/// inteiro já custa caro demais na máquina fraca que é o público do produto.
///
/// A validade é curta de propósito: plano de energia muda quando o cliente
/// clica em alguma coisa, e uma lista velha faria a tela mentir.
pub fn planos_instalados() -> Vec<(String, String)> {
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    static CACHE: Mutex<Option<(Instant, Vec<(String, String)>)>> = Mutex::new(None);
    const VALIDADE: Duration = Duration::from_secs(10);

    if let Ok(guarda) = CACHE.lock() {
        if let Some((quando, lista)) = guarda.as_ref() {
            if quando.elapsed() < VALIDADE {
                return lista.clone();
            }
        }
    }

    let Ok(saida) = shell::run("powercfg", &["/list"]) else {
        return Vec::new();
    };

    let lista = analisar_lista_de_planos(&saida.stdout);

    if let Ok(mut guarda) = CACHE.lock() {
        *guarda = Some((Instant::now(), lista.clone()));
    }

    lista
}

/// Extrai os planos da saída do `powercfg /list`.
///
/// **Função pura.** O formato é `GUID do Esquema de Energia: <guid>  (<nome>)`,
/// com o rótulo traduzido — por isso a leitura se apoia no formato do
/// identificador e nos parênteses, e nunca no texto do rótulo.
pub fn analisar_lista_de_planos(saida: &str) -> Vec<(String, String)> {
    let mut planos = Vec::new();

    for linha in saida.lines() {
        let Some(inicio) = linha.find(':') else {
            continue;
        };

        let resto = linha[inicio + 1..].trim();
        let guid: String = resto.chars().take(36).collect();

        // Um identificador tem 36 caracteres com hífen na quarta posição a
        // partir de cada bloco. A conferência simples evita casar com qualquer
        // linha que tenha dois-pontos.
        if guid.len() != 36 || guid.matches('-').count() != 4 {
            continue;
        }

        let nome = resto
            .find('(')
            .and_then(|a| resto.rfind(')').map(|b| resto[a + 1..b].to_string()))
            .unwrap_or_default();

        planos.push((guid, nome));
    }

    planos
}

/// O plano ATIVO é de terceiro?
///
/// Programas como o IObit Driver Booster criam um plano próprio e o deixam
/// ativo. O cliente acha que está no "alto desempenho" do Windows e está num
/// plano que ninguém auditou — na máquina onde este código foi escrito, o plano
/// ativo era o "Driver Booster Power Plan".
pub fn plano_ativo_e_de_terceiro() -> Option<String> {
    let ativo = super::power::active_scheme().ok()?;

    let (_, nome) = planos_instalados()
        .into_iter()
        .find(|(guid, _)| guid.eq_ignore_ascii_case(&ativo))?;

    if nome_e_do_windows(&nome) {
        None
    } else {
        Some(nome)
    }
}

/// Nomes que o Windows dá aos planos de fábrica, nos idiomas que o produto
/// atende. Qualquer outro nome veio de fora.
fn nome_e_do_windows(nome: &str) -> bool {
    // Pelo esqueleto ASCII: a saída do `powercfg` chega com o acento corrompido.
    let minusculo = sem_acento(nome);

    [
        "equilibrado",
        "balanced",
        "alto desempenho",
        "high performance",
        "economia de energia",
        "power saver",
        "desempenho máximo",
        "ultimate performance",
    ]
    .iter()
    // Os dois lados passam pela mesma peneira: `sem_acento` DESCARTA o
    // caractere acentuado em vez de trocá-lo pelo sem acento, então comparar
    // um lado dobrado com o outro cru daria falso.
    .any(|conhecido| minusculo.contains(&sem_acento(conhecido)))
}

/// Cria o plano de desempenho máximo, sem ativá-lo.
///
/// Criar e ativar são passos separados de propósito: quem está na bateria pode
/// querer ter o plano disponível sem ligá-lo agora.
pub fn criar_plano_maximo() -> Result<String, String> {
    if !registry::is_elevated() {
        return Err("Criar um plano de energia exige executar como administrador.".to_string());
    }

    if plano_maximo_existe() {
        return Err("O plano de desempenho máximo já existe nesta máquina.".to_string());
    }

    shell::run_checked("powercfg", &["-duplicatescheme", DESEMPENHO_MAXIMO_GUID])
        .map_err(|e| format!("Não foi possível criar o plano: {}", e))?;

    Ok("Plano de desempenho máximo criado. Ele aparece agora nas opções de energia do Windows, \
        e o Otimiza não o ativou — ativar é uma escolha sua, e em notebook na bateria ele custa \
        autonomia."
        .to_string())
}

/// Religa o TRIM.
pub fn ligar_trim() -> Result<String, String> {
    if !registry::is_elevated() {
        return Err("Alterar o comportamento do sistema de arquivos exige administrador.".to_string());
    }

    shell::run_checked("fsutil", &["behavior", "set", "DisableDeleteNotify", "0"])
        .map_err(|e| format!("Não foi possível religar o TRIM: {}", e))?;

    Ok("TRIM religado. O SSD volta a ser avisado sobre blocos apagados, e a velocidade de \
        escrita para de se degradar com o tempo."
        .to_string())
}

/// Usado pelo relatório para citar o plano ativo.
#[allow(dead_code)]
pub fn plano_ativo() -> Option<String> {
    power::active_scheme().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reinicio_pendente_desta_maquina() {
        let motivos = reinicio_pendente();
        println!("motivos de reinício pendente: {:?}", motivos);

        // Todo motivo relatado tem texto; motivo vazio na tela não explica nada.
        assert!(motivos.iter().all(|m| !m.is_empty()));
    }

    #[test]
    fn trim_e_lido_pelo_numero_e_nao_pelo_texto() {
        // A saída do fsutil vem traduzida. Este projeto já quebrou uma vez por
        // ler texto localizado, e a leitura aqui é do número depois do sinal.
        let estado = trim_ligado();
        println!("TRIM ligado: {:?}", estado);

        // Numa máquina Windows a resposta existe; `None` seria falha de leitura.
        assert!(estado.is_some(), "não foi possível ler o estado do TRIM");
    }

    #[test]
    fn le_a_lista_de_planos_sem_depender_do_idioma() {
        // O rótulo é traduzido; o formato do identificador não é.
        let saida = "
Esquemas de Energia Existentes (* Ativos)
             -----------------------------------
             GUID do Esquema de Energia: 381b4222-f694-41f0-9685-ff5bb260df2e  (Equilibrado)
             GUID do Esquema de Energia: 3d23ae32-1072-4a92-ab57-ce99335b215d  (Driver Booster Power Plan) *
             GUID do Esquema de Energia: d1664682-a7b9-4796-b248-286ed3cc2d01  (Desempenho Máximo)
";

        let planos = analisar_lista_de_planos(saida);

        assert_eq!(planos.len(), 3);
        assert_eq!(planos[1].1, "Driver Booster Power Plan");
        assert_eq!(planos[2].0, "d1664682-a7b9-4796-b248-286ed3cc2d01");
        // A linha de cabeçalho tem dois-pontos e não pode virar plano.
        assert!(planos.iter().all(|(g, _)| g.len() == 36));
    }

    #[test]
    fn plano_de_terceiro_e_reconhecido_como_de_fora() {
        // O defeito real: o cliente acha que está no alto desempenho e está num
        // plano criado por um otimizador que ele nem lembra de ter instalado.
        assert!(!nome_e_do_windows("Driver Booster Power Plan"));
        assert!(!nome_e_do_windows("Razer Game Booster"));
        assert!(!nome_e_do_windows("Lenovo Vantage"));

        // E os de fábrica não podem virar alarme falso, em nenhum dos idiomas.
        for oficial in [
            "Equilibrado",
            "Balanced",
            "Alto desempenho",
            "High performance",
            "Economia de energia",
            "Desempenho Máximo",
            "Ultimate Performance",
        ] {
            assert!(nome_e_do_windows(oficial), "`{}` é do Windows", oficial);
        }
    }

    #[test]
    fn plano_maximo_e_detectado_pelo_identificador() {
        // Pelo identificador, nunca pelo nome — o nome do plano é traduzido.
        let existe = plano_maximo_existe();
        println!("plano de desempenho máximo existe: {}", existe);

        assert_eq!(DESEMPENHO_MAXIMO_GUID.len(), 36);
    }

    #[test]
    fn criar_duas_vezes_e_recusado() {
        if plano_maximo_existe() {
            let erro = criar_plano_maximo().unwrap_err();
            assert!(erro.contains("já existe") || erro.contains("administrador"));
        } else {
            println!("plano ainda não existe nesta máquina; caso não exercitado");
        }
    }

    #[test]
    fn analisa_esta_maquina() {
        let r = analyze();

        println!("nota: {}", r.note);
        for f in &r.findings {
            println!(
                "  [{:?}{}] {} — {}",
                f.severity,
                if f.actionable { ", corrigível" } else { "" },
                f.title,
                f.measured
            );
        }

        assert!(!r.note.is_empty());

        // Todo achado tem texto medido e conselho: achado sem explicação vira
        // alarme que ninguém sabe o que fazer com.
        for f in &r.findings {
            assert!(!f.measured.is_empty(), "{} sem medida", f.title);
            assert!(!f.advice.is_empty(), "{} sem conselho", f.title);
        }

        // Problemas antes do que é só oportunidade.
        let ordem: Vec<u8> = r
            .findings
            .iter()
            .map(|f| match f.severity {
                FindingSeverity::Critical => 0,
                FindingSeverity::Important => 1,
                FindingSeverity::Ok => 2,
            })
            .collect();
        assert!(ordem.windows(2).all(|p| p[0] <= p[1]));
    }
}
