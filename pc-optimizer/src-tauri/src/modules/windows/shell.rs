// Comandos do sistema. Todo comando roda com CREATE_NO_WINDOW (sem console piscando) e TEM PRAZO: sem limite,
// um `powercfg` travado prendia o "Otimizar agora" em "Aplicando…" sem rastro. Estourado, o processo é
// encerrado, vira erro com o nome do comando e vai para o log.

use std::io::Read;
use std::os::windows::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Um minuto: essas ferramentas respondem em menos de um segundo, e o que importa é ser finito.
pub const PRAZO_PADRAO: Duration = Duration::from_secs(60);

/// Maior: WMI e log de eventos passam de dez segundos em PC fraco. Quem demora mais passa o próprio prazo.
pub const PRAZO_DO_POWERSHELL: Duration = Duration::from_secs(120);

pub struct CommandOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    /// O número guarda o que o booleano joga fora. `None` quando encerrado por sinal (o prazo estourado).
    pub codigo: Option<i32>,
}

pub fn run(program: &str, args: &[&str]) -> Result<CommandOutput, String> {
    run_com_prazo(program, args, PRAZO_PADRAO)
}

pub fn run_com_prazo(
    program: &str,
    args: &[&str],
    prazo: Duration,
) -> Result<CommandOutput, String> {
    let rotulo = if args.is_empty() {
        program.to_string()
    } else {
        format!("{} {}", program, args.join(" "))
    };

    executar_com_rotulo(program, args, &resumir(&rotulo), prazo)
}

fn executar_com_rotulo(
    program: &str,
    args: &[&str],
    rotulo: &str,
    prazo: Duration,
) -> Result<CommandOutput, String> {
    let mut filho = Command::new(program)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to run `{}`: {}", program, e))?;

    esperar_com_prazo(&mut filho, rotulo, prazo)
}

/// Devolve o processo VIVO, com a saída num cano, para quem precisa dele na mão (a análise do DISM).
pub fn spawn_capturando(program: &str, args: &[&str]) -> Result<std::process::Child, String> {
    Command::new(program)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to spawn `{}`: {}", program, e))
}

/// Encerra no prazo em vez de deixar rodando. `Child` emprestado: o teste confere que morreu. Canos em threads
/// à parte (processo que escreve muito sem leitor trava). Sempre com `wait`, senão fica zumbi.
pub fn esperar_com_prazo(
    filho: &mut Child,
    rotulo: &str,
    prazo: Duration,
) -> Result<CommandOutput, String> {
    let limite = Instant::now() + prazo;
    let (tx, rx) = mpsc::channel::<(bool, Vec<u8>)>();

    ler_em_segundo_plano(filho.stdout.take(), true, tx.clone());
    ler_em_segundo_plano(filho.stderr.take(), false, tx);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    for _ in 0..2 {
        match rx.recv_timeout(limite.saturating_duration_since(Instant::now())) {
            Ok((true, bruto)) => stdout = bruto,
            Ok((false, bruto)) => stderr = bruto,
            Err(mpsc::RecvTimeoutError::Timeout) => return Err(encerrar(filho, rotulo, prazo)),
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                let _ = filho.kill();
                let _ = filho.wait();
                return Err(format!("perdi a saída de `{}` no meio da leitura", rotulo));
            }
        }
    }

    // Um programa pode fechar a saída e seguir vivo: `wait` sem prazo aqui seria o defeito que a função conserta.
    let status = loop {
        match filho.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < limite => std::thread::sleep(Duration::from_millis(5)),
            Ok(None) => return Err(encerrar(filho, rotulo, prazo)),
            Err(e) => {
                let _ = filho.kill();
                let _ = filho.wait();
                return Err(format!("não consegui acompanhar `{}`: {}", rotulo, e));
            }
        }
    };

    Ok(CommandOutput {
        success: status.success(),
        codigo: status.code(),
        stdout: String::from_utf8_lossy(&stdout).to_string(),
        stderr: String::from_utf8_lossy(&stderr).to_string(),
    })
}

fn ler_em_segundo_plano<R: Read + Send + 'static>(
    cano: Option<R>,
    e_saida: bool,
    tx: mpsc::Sender<(bool, Vec<u8>)>,
) {
    let Some(mut cano) = cano else {
        let _ = tx.send((e_saida, Vec::new()));
        return;
    };

    std::thread::spawn(move || {
        let mut bruto = Vec::new();
        let _ = cano.read_to_end(&mut bruto);
        let _ = tx.send((e_saida, bruto));
    });
}

fn encerrar(filho: &mut Child, rotulo: &str, prazo: Duration) -> String {
    let _ = filho.kill();
    let _ = filho.wait();

    let mensagem = format!(
        "`{}` não terminou em {} e foi encerrado",
        rotulo,
        descrever_prazo(prazo)
    );
    crate::utils::Logger::warn(&mensagem);
    mensagem
}

fn descrever_prazo(prazo: Duration) -> String {
    if prazo.as_millis() < 1000 {
        format!("{} ms", prazo.as_millis())
    } else {
        format!("{} s", prazo.as_secs())
    }
}

/// Por caractere: cortar no meio de um "ç" derrubaria o programa.
fn resumir(texto: &str) -> String {
    const MAXIMO: usize = 160;

    let limpo = texto.split_whitespace().collect::<Vec<_>>().join(" ");

    if limpo.chars().count() <= MAXIMO {
        limpo
    } else {
        format!("{}…", limpo.chars().take(MAXIMO).collect::<String>())
    }
}

/// Sem isto o PowerShell escreve em CP850 e "Serviço" vira "Servi?o" em nomes de terceiros. Resolver na origem:
/// a página de código muda com o idioma do Windows.
const FORCAR_UTF8: &str = "[Console]::OutputEncoding=[System.Text.Encoding]::UTF8;";

/// Todo PowerShell passa por aqui: `run("powershell", …)` direto devolve acento quebrado.
pub fn powershell(script: &str) -> Result<CommandOutput, String> {
    powershell_com_prazo(script, PRAZO_DO_POWERSHELL)
}

pub fn powershell_com_prazo(script: &str, prazo: Duration) -> Result<CommandOutput, String> {
    // Sessão viva, ou o processo avulso. Prazo estourado NÃO cai para o avulso: dobraria a espera e repetiria a
    // escrita.
    if let Some(resposta) = sessao::executar(script, prazo) {
        return resposta;
    }

    powershell_avulso(script, prazo)
}

fn powershell_avulso(script: &str, prazo: Duration) -> Result<CommandOutput, String> {
    let completo = format!("{} {}", FORCAR_UTF8, script);
    let rotulo = resumir(&format!("powershell: {}", script));

    executar_com_rotulo("powershell", &["-NoProfile", "-Command", &completo], &rotulo, prazo)
}

/// Abrir um `powershell.exe` vazio custa 2,26 s (medido), e a abertura abria dez: um processo vivo paga uma vez,
/// sem mudar os chamadores. Cada resposta termina numa marca com número próprio e o sucesso do script. Sessão
/// morta cai no avulso; travada estoura o prazo e é derrubada.
mod sessao {
    use super::{CommandOutput, CREATE_NO_WINDOW, FORCAR_UTF8};
    use std::io::{BufRead, BufReader, Write};
    use std::os::windows::process::CommandExt;
    use std::process::{Child, ChildStdin, Command, Stdio};
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::{mpsc, Mutex};
    use std::time::{Duration, Instant};

    pub(super) struct Viva {
        processo: Child,
        entrada: ChildStdin,
        /// `read_line` sem limite prendia quem chamou: pelo canal, a espera tem prazo.
        linhas: mpsc::Receiver<String>,
    }

    impl Viva {
        pub(super) fn encerrar(&mut self) {
            let _ = self.processo.kill();
            let _ = self.processo.wait();
        }
    }

    pub(super) enum Resposta {
        Respondeu(CommandOutput),
        Morreu,
        Estourou,
    }

    static SESSAO: Mutex<Option<Viva>> = Mutex::new(None);
    static CONTADOR: AtomicU64 = AtomicU64::new(0);

    /// Sessão que morreu ou travou no meio não é reaproveitável.
    static DESISTIMOS: AtomicBool = AtomicBool::new(false);

    pub(super) fn abrir() -> Option<Viva> {
        let mut processo = Command::new("powershell")
            .args(["-NoProfile", "-NoLogo", "-NonInteractive", "-Command", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .ok()?;

        let entrada = processo.stdin.take()?;
        let mut saida = BufReader::new(processo.stdout.take()?);
        let (tx, linhas) = mpsc::channel();

        std::thread::spawn(move || loop {
            let mut linha = String::new();

            match saida.read_line(&mut linha) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    if tx.send(linha).is_err() {
                        break;
                    }
                }
            }
        });

        let mut viva = Viva {
            processo,
            entrada,
            linhas,
        };

        viva.entrada.write_all(FORCAR_UTF8.as_bytes()).ok()?;
        viva.entrada.write_all(b"\n").ok()?;
        viva.entrada.flush().ok()?;

        Some(viva)
    }

    /// `None` = use o avulso. `Some(Err)` = prazo estourado, que não se repete.
    pub fn executar(script: &str, prazo: Duration) -> Option<Result<CommandOutput, String>> {
        if DESISTIMOS.load(Ordering::Relaxed) {
            return None;
        }

        // SCRIPT COM ACENTO NÃO PASSA POR AQUI: o PowerShell lê a entrada padrão na página do console ("Ação" chega
        // "A├º├úo") antes de o script rodar. No avulso o script vai na linha de comando. Fora do ASCII, caminho lento.
        if !script.is_ascii() {
            return None;
        }

        let mut guarda = SESSAO.lock().ok()?;

        if guarda.is_none() {
            *guarda = abrir();
        }

        let viva = guarda.as_mut()?;

        match conversar(viva, script, prazo) {
            Resposta::Respondeu(saida) => Some(Ok(saida)),
            Resposta::Morreu => {
                derrubar(&mut guarda);
                None
            }
            Resposta::Estourou => {
                derrubar(&mut guarda);

                let mensagem = format!(
                    "`powershell: {}` não terminou em {} e foi encerrado",
                    super::resumir(script),
                    super::descrever_prazo(prazo)
                );
                crate::utils::Logger::warn(&mensagem);
                Some(Err(mensagem))
            }
        }
    }

    /// Separada para o teste travar uma sessão PRÓPRIA sem derrubar a global.
    pub(super) fn conversar(viva: &mut Viva, script: &str, prazo: Duration) -> Resposta {
        let limite = Instant::now() + prazo;
        let marca = format!(
            "<<<OTIMIZA-FIM-{}>>>",
            CONTADOR.fetch_add(1, Ordering::Relaxed)
        );

        // `$LASTEXITCODE` não serve (nem todo script chama programa externo): o `try/catch` diz se o script LANÇOU erro.
        let bloco = format!(
            "$ErrorActionPreference='Continue'; $__ok=$true; try {{ {} }} catch {{ $__ok=$false }}; Write-Output ('{}' + $__ok)\n",
            script, marca
        );

        if viva.entrada.write_all(bloco.as_bytes()).is_err() || viva.entrada.flush().is_err() {
            return Resposta::Morreu;
        }

        let mut coletado = String::new();

        loop {
            let linha = match viva
                .linhas
                .recv_timeout(limite.saturating_duration_since(Instant::now()))
            {
                Ok(linha) => linha,
                Err(mpsc::RecvTimeoutError::Timeout) => return Resposta::Estourou,
                Err(mpsc::RecvTimeoutError::Disconnected) => return Resposta::Morreu,
            };

            if let Some(resto) = linha.trim_end().strip_prefix(marca.as_str()) {
                return Resposta::Respondeu(CommandOutput {
                    success: !resto.trim().eq_ignore_ascii_case("False"),
                    // Não há código de saída numa sessão viva: inventar um faria o log afirmar o que o Windows nunca devolveu.
                    codigo: None,
                    stdout: coletado,
                    stderr: String::new(),
                });
            }

            coletado.push_str(&linha);
        }
    }

    fn derrubar(guarda: &mut Option<Viva>) {
        if let Some(mut viva) = guarda.take() {
            viva.encerrar();
        }

        // Sessão que caiu costuma cair de novo.
        DESISTIMOS.store(true, Ordering::Relaxed);
    }
}

/// JSON de PowerShell sem achatar falha em vazio (quatro módulos pintavam "nada encontrado" de verde): não rodou
/// ou erro → `Err`; saída vazia → `Err` (todo script embrulha em `@(…)`, que dá `[]`); tipo errado → `Err`. Na
/// sessão viva, a consulta precisa de `-ErrorAction Stop` para uma falha chegar como `success: false`.
pub fn json_da_saida<T: serde::de::DeserializeOwned>(
    saida: Result<CommandOutput, String>,
    o_que: &str,
) -> Result<T, String> {
    let saida = saida.map_err(|e| format!("Não consegui ler {}: {}", o_que, e))?;

    if !saida.success {
        let detalhe = if !saida.stderr.trim().is_empty() {
            saida.stderr.trim()
        } else if !saida.stdout.trim().is_empty() {
            saida.stdout.trim()
        } else {
            "o Windows recusou a consulta"
        };

        return Err(format!("Não consegui ler {}: {}", o_que, resumir(detalhe)));
    }

    let texto = saida.stdout.trim();

    if texto.is_empty() {
        return Err(format!(
            "Não consegui ler {}: a consulta não devolveu resposta",
            o_que
        ));
    }

    serde_json::from_str(texto).map_err(|e| {
        format!(
            "Não consegui ler {}: a resposta veio num formato inesperado ({})",
            o_que, e
        )
    })
}

pub fn powershell_checked(script: &str) -> Result<String, String> {
    powershell_checked_com_prazo(script, PRAZO_DO_POWERSHELL)
}

pub fn powershell_checked_com_prazo(script: &str, prazo: Duration) -> Result<String, String> {
    let saida = powershell_com_prazo(script, prazo)?;

    if saida.success {
        Ok(saida.stdout)
    } else {
        let detalhe = if saida.stderr.trim().is_empty() {
            saida.stdout.trim().to_string()
        } else {
            saida.stderr.trim().to_string()
        };
        Err(detalhe)
    }
}

pub fn run_checked(program: &str, args: &[&str]) -> Result<String, String> {
    run_checked_com_prazo(program, args, PRAZO_PADRAO)
}

pub fn run_checked_com_prazo(
    program: &str,
    args: &[&str],
    prazo: Duration,
) -> Result<String, String> {
    let output = run_com_prazo(program, args, prazo)?;

    if output.success {
        Ok(output.stdout)
    } else {
        let detail = if output.stderr.trim().is_empty() {
            output.stdout.trim().to_string()
        } else {
            output.stderr.trim().to_string()
        };
        Err(format!("`{} {}` failed: {}", program, args.join(" "), detail))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saida_do_powershell_chega_com_acento_intacto() {
        // ç, ã e á: os que quebram em CP850.
        let saida = powershell("Write-Output 'Serviço de Configuração Básica'")
            .expect("o PowerShell precisa rodar");

        assert!(
            saida.stdout.contains("Serviço de Configuração Básica"),
            "acento corrompido na saída: {:?}",
            saida.stdout.trim()
        );
        assert!(!saida.stdout.contains('\u{FFFD}'));
    }

    #[test]
    fn json_com_acento_sobrevive_a_desserializacao() {
        let saida = powershell(
            "ConvertTo-Json -Compress -InputObject @{ nome = 'Ação de Manutenção' }",
        )
        .expect("o PowerShell precisa rodar");

        let valor: serde_json::Value =
            serde_json::from_str(saida.stdout.trim()).expect("JSON válido");

        assert_eq!(valor["nome"], "Ação de Manutenção");
    }

    #[test]
    fn script_que_falha_devolve_erro() {
        let erro = powershell_checked("throw 'falhou de proposito'");
        assert!(erro.is_err());
    }

    #[test]
    fn a_sessao_viva_recusa_script_com_acento() {
        // Silencioso: acento pela sessão não FALHA, devolve errado.
        assert!(super::sessao::executar("Write-Output 'Ação'", PRAZO_DO_POWERSHELL).is_none());

        let saida = powershell("Write-Output 'Ação de Manutenção'").unwrap();
        assert!(saida.stdout.contains("Ação de Manutenção"), "veio: {}", saida.stdout);
    }

    #[test]
    fn a_sessao_viva_devolve_o_mesmo_que_o_processo_avulso() {
        // Um script ASCII precisa dar o mesmo resultado pelos dois caminhos.
        let script = "ConvertTo-Json -Compress -InputObject ([ordered]@{ a = 1; b = 'dois' })";

        let pela_sessao = super::sessao::executar(script, PRAZO_DO_POWERSHELL)
            .expect("script ASCII usa a sessão")
            .expect("a sessão respondeu no prazo");
        let avulso = super::powershell_avulso(script, PRAZO_DO_POWERSHELL).unwrap();

        assert_eq!(pela_sessao.stdout.trim(), avulso.stdout.trim());
        assert_eq!(pela_sessao.success, avulso.success);
    }

    #[test]
    fn script_que_lanca_erro_e_reportado_como_falha_pela_sessao() {
        let saida = super::sessao::executar("throw 'quebrou'", PRAZO_DO_POWERSHELL)
            .expect("script ASCII usa a sessão")
            .expect("a sessão respondeu no prazo");
        assert!(!saida.success);
    }

    #[test]
    fn a_sessao_que_trava_e_encerrada_no_prazo() {
        // Sessão própria: estourar o prazo derruba a sessão, e a global faria os outros testes caírem no avulso.
        let mut viva = super::sessao::abrir().expect("o PowerShell sobe");

        // Antes de travar, ela responde: senão o estouro poderia ser uma sessão que nunca funcionou.
        let normal = super::sessao::conversar(&mut viva, "Write-Output 'ok'", PRAZO_DO_POWERSHELL);
        let respondeu = matches!(
            &normal,
            super::sessao::Resposta::Respondeu(saida) if saida.stdout.trim() == "ok"
        );

        let inicio = Instant::now();
        let travada = super::sessao::conversar(
            &mut viva,
            "Start-Sleep -Seconds 60",
            Duration::from_millis(1500),
        );
        let esperou = inicio.elapsed();
        viva.encerrar();

        assert!(respondeu, "a sessão própria não respondeu nem ao script simples");
        assert!(
            matches!(travada, super::sessao::Resposta::Estourou),
            "um script de 60 s com prazo de 1,5 s não foi dado como estourado"
        );
        assert!(
            esperou < Duration::from_secs(10),
            "não desistiu no prazo: esperou {:?}",
            esperou
        );
    }

    #[test]
    fn desistir_de_esperar_mata_o_processo_em_vez_de_deixar_rodando() {
        // O `recv_timeout` devolvia o controle e o `Dism.exe` seguia minutos, empilhando a cada clique. `ping -n 30` faz
        // o papel sem administrador.
        let mut filho = spawn_capturando("ping", &["-n", "30", "127.0.0.1"])
            .expect("o ping do Windows sobe");

        let inicio = Instant::now();
        let saida = esperar_com_prazo(&mut filho, "ping", Duration::from_millis(300));

        assert!(saida.is_err(), "o prazo estourou; não podia vir saída");
        assert!(
            inicio.elapsed() < Duration::from_secs(10),
            "não desistiu no prazo: esperou {:?}",
            inicio.elapsed()
        );

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
    fn um_comando_rapido_devolve_saida_erro_e_codigo_como_antes() {
        // O prazo não pode custar a saída: os módulos de serviço leem 1056/1062 do `sc`.
        let saida = run("cmd", &["/c", "echo saida& echo erro 1>&2& exit 3"])
            .expect("o cmd do Windows roda");

        assert!(!saida.success, "saiu com código 3 e veio como sucesso");
        assert!(saida.stdout.contains("saida"), "stdout: {:?}", saida.stdout);
        assert!(saida.stderr.contains("erro"), "stderr: {:?}", saida.stderr);
    }

    #[test]
    fn run_com_prazo_encerra_o_programa_que_nao_termina() {
        let inicio = Instant::now();
        let erro = run_com_prazo("ping", &["-n", "30", "127.0.0.1"], Duration::from_millis(500))
            .err()
            .expect("um ping de 30 s com prazo de meio segundo precisa falhar");

        assert!(inicio.elapsed() < Duration::from_secs(10));
        assert!(erro.contains("ping -n 30"), "o erro não diz o comando: {}", erro);
    }

    #[test]
    fn rotulo_longo_e_cortado_sem_quebrar_acento() {
        let longo = "ç".repeat(400);
        let rotulo = resumir(&longo);

        assert_eq!(rotulo.chars().count(), 161);
        assert!(rotulo.ends_with('…'));
        assert_eq!(resumir("  Get-Thing \n  -Flag  "), "Get-Thing -Flag");
    }

    fn saida(success: bool, stdout: &str, stderr: &str) -> Result<CommandOutput, String> {
        Ok(CommandOutput {
            success,
            codigo: if success { Some(0) } else { Some(1) },
            stdout: stdout.to_string(),
            stderr: stderr.to_string(),
        })
    }

    #[test]
    fn leitura_que_falha_nunca_vira_lista_vazia() {
        let nao_rodou: Result<Vec<u32>, String> =
            json_da_saida(Err("não subiu".to_string()), "a lista");
        let saiu_com_erro: Result<Vec<u32>, String> =
            json_da_saida(saida(false, "", "Acesso negado"), "a lista");
        let sem_resposta: Result<Vec<u32>, String> = json_da_saida(saida(true, "  \n", ""), "a lista");
        let formato_errado: Result<Vec<u32>, String> =
            json_da_saida(saida(true, "isto não é json", ""), "a lista");

        assert!(nao_rodou.is_err());
        assert!(saiu_com_erro.as_ref().is_err_and(|e| e.contains("Acesso negado")));
        assert!(sem_resposta.is_err());
        assert!(formato_errado.is_err());
    }

    #[test]
    fn lista_vazia_de_verdade_continua_sendo_vazia() {
        // `@()` vazio vira `[]`, e isso é "não há": continua lista vazia.
        let vazia: Result<Vec<u32>, String> = json_da_saida(saida(true, "[]\r\n", ""), "a lista");
        let cheia: Result<Vec<u32>, String> = json_da_saida(saida(true, "[1,2]", ""), "a lista");

        assert_eq!(vazia, Ok(Vec::new()));
        assert_eq!(cheia, Ok(vec![1, 2]));
    }

    #[test]
    fn as_quatro_consultas_param_no_erro() {
        // Com `-ErrorAction SilentlyContinue`, o erro chega como SUCESSO com `[]`.
        let consultas = [
            ("tasks.rs", "Get-ScheduledTask -ErrorAction Stop"),
            ("servicesaudit.rs", "Win32_Service -ErrorAction Stop"),
            ("bloatware.rs", "-ErrorAction Stop | Select-Object Name,PackageFullName"),
            ("conflicts.rs", "-ErrorAction Stop | Select-Object displayName,productState"),
        ];

        for (arquivo, consulta) in consultas {
            let fonte = std::fs::read_to_string(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("src/modules/windows")
                    .join(arquivo),
            )
            .expect("o módulo existe");

            assert!(
                fonte.contains(consulta),
                "{} voltou a consultar sem parar no erro (esperava `{}`)",
                arquivo,
                consulta
            );
        }
    }

    #[test]
    fn ninguem_chama_o_powershell_por_fora_do_helper() {
        // `run("powershell", …)` direto devolve acento quebrado, calado.
        let raiz = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut infratores = Vec::new();

        fn varrer(dir: &std::path::Path, achados: &mut Vec<String>) {
            let Ok(entradas) = std::fs::read_dir(dir) else { return };

            for entrada in entradas.flatten() {
                let caminho = entrada.path();

                if caminho.is_dir() {
                    varrer(&caminho, achados);
                    continue;
                }

                let nome = caminho.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !nome.ends_with(".rs") || nome == "shell.rs" {
                    continue;
                }

                let Ok(conteudo) = std::fs::read_to_string(&caminho) else { continue };

                for (numero, linha) in conteudo.lines().enumerate() {
                    // Reabrir como administrador: não lê saída, e precisa de `-WindowStyle Hidden`.
                    if linha.contains("\"powershell\"") && !conteudo.contains("Start-Process -FilePath") {
                        achados.push(format!("{}:{}", nome, numero + 1));
                    }
                }
            }
        }

        varrer(&raiz, &mut infratores);

        assert!(
            infratores.is_empty(),
            "estes pontos chamam o PowerShell sem forçar UTF-8 e vão devolver \
             acento quebrado — use shell::powershell(…): {:?}",
            infratores
        );
    }
}
