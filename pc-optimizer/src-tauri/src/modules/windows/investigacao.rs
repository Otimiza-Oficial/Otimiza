// Investigação e teste ativo da travada: a casca Windows dos motores puros `causadatravada` e `testedatravada`
// (docs/planos/PIP-ENGENHARIA.md, seção 10). Quadros pelo PresentMon (nunca pelo canal antigo, que dobra no FiveM) e
// contadores do Windows, carimbados no MESMO relógio (QPC). O custo do próprio Otimiza e do PresentMon é medido em cada
// investigação e vai junto do resultado.
//
// O teste ativo só existe para uma hipótese que a última investigação deste jogo apontou (Moderada ou Forte), com
// travadas suficientes para o teste enxergar. Mexe só na prioridade e no EcoQoS do programa suspeito (o mesmo
// `acalmar` do modo jogo), nunca no jogo; cada troca é anotada em disco antes de acontecer a próxima, e a abertura
// seguinte do Otimiza devolve o que um fechamento no meio deixou.

use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use super::governador::{self, Acalmado, Candidato};
use crate::core::telemetria::Amostra;
use crate::modules::causadatravada::{self as motor, Confianca, Hipotese, Investigacao, QuadroDoJogo, Teste};
use crate::modules::testedatravada::{self as teste, Decisao, Janela, Julgamento, LadoDoTeste, Registro};

static OCUPADO: AtomicBool = AtomicBool::new(false);
/// Só o TESTE mexe no Windows; a investigação só mede e não impede devolver nada.
static TESTANDO: AtomicBool = AtomicBool::new(false);

/// A medição automática e o "Restaurar meu PC" ficam de fora enquanto isto roda.
pub fn em_andamento() -> bool {
    OCUPADO.load(Ordering::SeqCst)
}

/// O modo jogo não acalma nada durante o teste (o lado "normal" precisa ser normal) nem o programa que o teste manteve.
pub fn reservado_pelo_teste(nome: &str) -> bool {
    TESTANDO.load(Ordering::SeqCst)
        || anotacao()
            .and_then(|a| governador::ler_de(&a).ok())
            .is_some_and(|l| l.iter().any(|x| x.nome.eq_ignore_ascii_case(nome)))
}

struct Testando;
impl Testando {
    fn marcar() -> Testando {
        TESTANDO.store(true, Ordering::SeqCst);
        Testando
    }
}
impl Drop for Testando {
    fn drop(&mut self) {
        TESTANDO.store(false, Ordering::SeqCst);
    }
}

struct Vez;
impl Vez {
    fn pegar() -> Result<Vez, String> {
        OCUPADO
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map(|_| Vez)
            .map_err(|_| "Uma investigação ou um teste de travada já está rodando.".to_string())
    }
}
impl Drop for Vez {
    fn drop(&mut self) {
        OCUPADO.store(false, Ordering::SeqCst);
    }
}

/// Sem APPDATA não há onde anotar: o teste recusa (anotar na pasta atual seria perder o registro na abertura seguinte).
fn pasta() -> Option<PathBuf> {
    std::env::var("APPDATA").ok().map(|a| PathBuf::from(a).join("pc-optimizer"))
}

/// O que o teste deixou acalmado: existe só enquanto algo está acalmado.
fn anotacao() -> Option<PathBuf> {
    pasta().map(|p| p.join("teste_da_travada.json"))
}

fn memoria() -> Option<PathBuf> {
    pasta().map(|p| p.join("travadas_testadas.json"))
}

const SEM_APPDATA: &str = "Sem a pasta de dados do usuário (APPDATA), o Otimiza não tem onde anotar o que muda: o teste não roda.";

/// O custo de observar, medido na própria investigação. Percentual de UM núcleo, média da janela.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Custo {
    pub otimiza_pct_de_um_nucleo: Option<f64>,
    pub presentmon_pct_de_um_nucleo: Option<f64>,
    pub amostras: usize,
    /// Tempo do motor puro (detecção + hipóteses + testes), em milissegundos.
    pub analise_ms: f64,
}

pub struct Captura {
    pub quadros: Vec<QuadroDoJogo>,
    pub amostras: Vec<(i64, Amostra)>,
    pub frequencia: i64,
    pub vram_total_mb: Option<f64>,
    pub custo: Custo,
    pub erro_dos_quadros: Option<String>,
}

/// Segundos de CPU (usuário + núcleo) do processo do Otimiza até agora.
fn cpu_do_otimiza_s() -> Option<f64> {
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};
    let zero = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
    let (mut c, mut s, mut k, mut u) = (zero, zero, zero, zero);
    let ok = unsafe { GetProcessTimes(GetCurrentProcess(), &mut c, &mut s, &mut k, &mut u) } != 0;
    let seg = |f: FILETIME| ((f.dwHighDateTime as u64) << 32 | f.dwLowDateTime as u64) as f64 / 1e7;
    ok.then(|| seg(k) + seg(u))
}

/// Os quadros do jogo na cadeia principal (a mesma regra da medição automática), no formato do motor.
pub fn quadros_do_jogo(quadros: &[super::presentmon::Quadro]) -> Vec<QuadroDoJogo> {
    use super::presentmon::TipoDeQuadro;
    let mut contagem: std::collections::HashMap<&str, usize> = Default::default();
    for q in quadros.iter().filter(|q| q.tipo == TipoDeQuadro::Jogo) {
        *contagem.entry(q.cadeia.as_str()).or_default() += 1;
    }
    let Some(principal) = contagem.into_iter().max_by_key(|(_, n)| *n).map(|(c, _)| c.to_string()) else {
        return Vec::new();
    };
    let mut v: Vec<QuadroDoJogo> = quadros
        .iter()
        .filter(|q| q.cadeia == principal && q.tipo == TipoDeQuadro::Jogo)
        .filter_map(|q| {
            Some(QuadroDoJogo {
                qpc: q.inicio_qpc?,
                intervalo_ms: q.intervalo_ms.filter(|x| x.is_finite() && *x > 0.0)?,
                cpu_ms: q.cpu_ocupada_ms,
                gpu_ms: q.gpu_ocupada_ms,
            })
        })
        .collect();
    v.sort_by_key(|q| q.qpc);
    v
}

/// Uma captura gravada: roda de novo no motor, offline (laboratório, sessões de referência).
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct CapturaGravada {
    pub quadros: Vec<QuadroDoJogo>,
    pub amostras: Vec<(i64, Amostra)>,
    pub frequencia: i64,
    pub vram_total_mb: Option<f64>,
}

/// Captura sincronizada: PresentMon numa thread, contadores a cada 500 ms carimbados com o QPC na outra.
pub fn capturar(pid_do_jogo: u32, segundos: u32) -> Result<Captura, String> {
    use crate::core::telemetria::Coletor;
    let frequencia = super::frames::frequencia_qpc().ok_or("o relógio de alta precisão do Windows não respondeu")?;
    let mut coletor = Coletor::novo().ok_or("Os contadores de desempenho do Windows não abriram nesta máquina.")?;
    coletor.acompanhar_processos(Some(pid_do_jogo));

    let cpu_antes = cpu_do_otimiza_s();
    let relogio = Instant::now();
    let medicao = std::thread::spawn(move || super::presentmon::capturar(pid_do_jogo, segundos));

    let mut amostras = Vec::new();
    let mut pm = sysinfo::System::new();
    let mut pm_pids: Vec<sysinfo::Pid> = Vec::new();
    let mut pm_preparado = false;
    let mut pm_soma = 0.0;
    let mut pm_leituras = 0usize;
    let limite = Instant::now() + Duration::from_secs(segundos as u64 + 25);
    while !medicao.is_finished() && Instant::now() < limite {
        std::thread::sleep(Duration::from_millis(500));
        let amostra = coletor.amostra();
        if let Some(q) = super::frames::agora_qpc() {
            amostras.push((q, amostra));
        }
        // O PresentMon pelo nome, achado uma vez; depois só ele é relido (reler todos os processos a cada meio
        // segundo entraria no custo que se quer medir). `cpu_usage` é em % de um núcleo; a primeira leitura é zero.
        if pm_pids.is_empty() {
            pm.refresh_processes_specifics(sysinfo::ProcessesToUpdate::All, true, sysinfo::ProcessRefreshKind::nothing());
            pm_pids = pm
                .processes()
                .iter()
                .filter(|(_, p)| p.name().to_string_lossy().to_lowercase().starts_with("presentmon"))
                .map(|(pid, _)| *pid)
                .collect();
        }
        if !pm_pids.is_empty() {
            pm.refresh_processes_specifics(
                sysinfo::ProcessesToUpdate::Some(&pm_pids),
                true,
                sysinfo::ProcessRefreshKind::nothing().with_cpu(),
            );
            if pm_preparado {
                pm_soma += pm_pids.iter().filter_map(|p| pm.process(*p)).map(|p| p.cpu_usage() as f64).sum::<f64>();
                pm_leituras += 1;
            }
            pm_preparado = true;
        }
    }
    let decorrido = relogio.elapsed().as_secs_f64();
    let cpu_depois = cpu_do_otimiza_s();

    let (quadros, erro_dos_quadros) = match medicao.join() {
        Ok(Ok(q)) => {
            let v = quadros_do_jogo(&q);
            let erro = v.is_empty().then(|| "o PresentMon não viu nenhum quadro do jogo".to_string());
            (v, erro)
        }
        Ok(Err(e)) => (Vec::new(), Some(e)),
        Err(_) => (Vec::new(), Some("a medição de quadros parou no meio".to_string())),
    };
    Ok(Captura {
        quadros,
        amostras,
        frequencia,
        vram_total_mb: coletor.placa().map(|p| p.vram_total_mb),
        custo: Custo {
            otimiza_pct_de_um_nucleo: match (cpu_antes, cpu_depois) {
                (Some(a), Some(b)) if decorrido > 0.0 => Some((b - a) / decorrido * 100.0),
                _ => None,
            },
            presentmon_pct_de_um_nucleo: (pm_leituras > 0).then(|| pm_soma / pm_leituras as f64),
            amostras: 0,
            analise_ms: 0.0,
        },
        erro_dos_quadros,
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct ResultadoDaInvestigacao {
    pub jogo: String,
    pub investigacao: Investigacao,
    pub custo: Custo,
    /// O programa que o teste ativo pode acalmar, quando a política deixa.
    pub teste_possivel: Option<String>,
    /// Por que não há teste ativo, quando havia hipótese para ele.
    pub motivo_sem_teste: Option<String>,
}

/// A última investigação: o teste ativo só roda sobre uma hipótese que ela apontou.
static ULTIMA: Mutex<Option<(String, Instant, Investigacao)>> = Mutex::new(None);
const VALIDADE_DA_INVESTIGACAO: Duration = Duration::from_secs(30 * 60);

/// Existe e não se lê é ERRO: "nunca testado" apagaria um negativo já medido, e o teste voltaria a ser oferecido.
pub fn ler_memoria() -> Result<Vec<Registro>, String> {
    let arquivo = memoria().ok_or(SEM_APPDATA)?;
    ler_memoria_de(&arquivo)
}

fn ler_memoria_de(arquivo: &std::path::Path) -> Result<Vec<Registro>, String> {
    match std::fs::read_to_string(arquivo) {
        Ok(t) => serde_json::from_str(&t).map_err(|e| format!("a memória dos testes de travada está ilegível ({})", e)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(format!("não consegui ler a memória dos testes de travada ({})", e)),
    }
}

/// Com a memória ilegível, a cópia vai para o lado (não é sobrescrita) e a lista recomeça.
fn guardar_na_memoria_em(arquivo: &std::path::Path, r: Registro) -> Result<Option<String>, String> {
    let (mut lista, aviso) = match ler_memoria_de(arquivo) {
        Ok(l) => (l, None),
        Err(e) => {
            let guardado = arquivo.with_extension(format!("ilegivel-{}.json", crate::modules::changelog::now_timestamp()));
            std::fs::rename(arquivo, &guardado).map_err(|x| format!("{}; e não consegui guardá-la à parte ({})", e, x))?;
            (Vec::new(), Some(format!("{}; a cópia foi guardada em {}", e, guardado.display())))
        }
    };
    lista.push(r);
    let excesso = lista.len().saturating_sub(teste::REGISTROS_GUARDADOS);
    lista.drain(..excesso);
    if let Some(p) = arquivo.parent() {
        let _ = std::fs::create_dir_all(p);
    }
    let json = serde_json::to_string_pretty(&lista).map_err(|e| e.to_string())?;
    std::fs::write(arquivo, json).map_err(|e| format!("não gravei a memória dos testes: {}", e))?;
    Ok(aviso)
}

fn contexto_agora() -> (Option<String>, Option<String>) {
    (crate::core::telemetria::versao_do_driver(), crate::core::telemetria::build_do_windows())
}

pub fn investigar(segundos: u32) -> Result<ResultadoDaInvestigacao, String> {
    let (jogo, pid) = super::gamemode::jogo_aberto_com_pid()
        .ok_or("Abra o jogo e entre numa partida (fora do menu) antes de investigar.")?;
    investigar_em(&jogo, pid, segundos)
}

/// A investigação sobre um processo dado (o jogo detectado, ou o alvo do laboratório).
pub fn investigar_em(jogo: &str, pid: u32, segundos: u32) -> Result<ResultadoDaInvestigacao, String> {
    let jogo = jogo.to_string();
    let _vez = Vez::pegar()?;
    if crate::modules::provaalternada::em_andamento() {
        return Err("A prova do Otimizar está rodando: investigue depois que ela terminar.".to_string());
    }
    let mut c = capturar(pid, segundos)?;
    if let Some(e) = &c.erro_dos_quadros {
        return Err(format!(
            "Não deu para medir os quadros do jogo pelo PresentMon ({}). A investigação não usa o medidor antigo: no \
             FiveM ele conta o dobro dos quadros.",
            e
        ));
    }
    let inicio = Instant::now();
    let mut inv = motor::investigar(&c.quadros, &c.amostras, c.frequencia, c.vram_total_mb, &governador::protegido);
    let chave = super::gamemode::chave_do_processo(&jogo);
    let (driver, windows) = contexto_agora();
    let memoria_ilegivel = match ler_memoria() {
        Ok(m) => {
            teste::anotar_com_memoria(&mut inv, &chave, &m, driver.as_deref(), windows.as_deref());
            None
        }
        Err(e) => Some(e),
    };
    c.custo.analise_ms = inicio.elapsed().as_secs_f64() * 1000.0;
    c.custo.amostras = c.amostras.len();

    let candidato = inv.avaliacoes.iter().find_map(|a| match (&a.teste, a.confianca >= Confianca::Moderada) {
        (Teste::AcalmarProcesso { processo }, true) => Some(processo.clone()),
        _ => None,
    });
    let (teste_possivel, motivo_sem_teste) = match (candidato, memoria_ilegivel) {
        (None, _) => (None, None),
        // Sem a memória, o Otimiza poderia repetir um teste que já falhou aqui: não oferece.
        (Some(_), Some(e)) => (None, Some(format!("Sem teste ativo agora: {}. Sem ela o Otimiza poderia repetir um teste que já não funcionou nesta máquina.", e))),
        (Some(p), None) => match teste::pode_testar(inv.por_minuto) {
            Ok(()) => (Some(p), None),
            Err(motivo) => (None, Some(motivo)),
        },
    };
    crate::utils::Logger::info(&format!(
        "investigação de travadas em {}: {} quadros, {} travadas ({:.1}/min), custo Otimiza {:?}% PresentMon {:?}% de um núcleo",
        jogo,
        inv.quadros,
        inv.episodios.len(),
        inv.por_minuto,
        c.custo.otimiza_pct_de_um_nucleo.map(|x| (x * 10.0).round() / 10.0),
        c.custo.presentmon_pct_de_um_nucleo.map(|x| (x * 10.0).round() / 10.0),
    ));
    *ULTIMA.lock().unwrap_or_else(|e| e.into_inner()) = Some((chave, Instant::now(), inv.clone()));
    Ok(ResultadoDaInvestigacao { jogo, investigacao: inv, custo: c.custo, teste_possivel, motivo_sem_teste })
}

#[derive(Debug, Clone, Serialize)]
pub struct ResultadoDoTeste {
    pub processo: String,
    pub julgamento: Julgamento,
    /// O programa ficou acalmado (até o jogo fechar ou "Devolver agora").
    pub mantido: bool,
    /// Falhou devolver algum processo: a tela diz, e a abertura seguinte tenta de novo.
    pub aviso: Option<String>,
}

/// Os processos com esse nome que o modo jogo também aceitaria acalmar (fora do jogo, da pasta dele e dos protegidos).
fn alvos(processo: &str, pid_do_jogo: u32) -> Vec<u32> {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
    let mut s = System::new();
    s.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing().with_exe(sysinfo::UpdateKind::OnlyIfNotSet));
    let caminho_do_jogo = s.process(sysinfo::Pid::from_u32(pid_do_jogo)).and_then(|p| p.exe().map(|e| e.to_path_buf()));
    let candidatos: Vec<Candidato> = s
        .processes()
        .iter()
        .filter(|(_, p)| p.name().to_string_lossy().eq_ignore_ascii_case(processo))
        .map(|(pid, p)| Candidato {
            pid: pid.as_u32(),
            nome: p.name().to_string_lossy().to_string(),
            caminho: p.exe().map(|e| e.to_path_buf()),
            uso: 1.0,
        })
        .collect();
    governador::escolher(&candidatos, pid_do_jogo, caminho_do_jogo.as_deref(), std::process::id())
}

/// Estado do teste: o que está acalmado agora, sempre espelhado no arquivo antes da próxima troca. Qualquer saída
/// (erro, `?`, pânico) devolve tudo no `Drop`, a não ser que a causa tenha sido demonstrada e o programa deva ficar.
struct Estado {
    acalmados: Vec<Acalmado>,
    manter: bool,
    arquivo: PathBuf,
}

impl Drop for Estado {
    fn drop(&mut self) {
        if !self.manter {
            if let Some(aviso) = self.devolver() {
                crate::utils::Logger::warn(&format!("teste da travada: {}", aviso));
            }
        }
    }
}

impl Estado {
    fn acalmar(&mut self, pids: &[u32]) -> Result<(), String> {
        let mut recusas = Vec::new();
        for pid in pids {
            match governador::acalmar_um(*pid) {
                Ok(a) => {
                    self.acalmados.push(a);
                    // Anotado ANTES de seguir: morrer aqui deixa o registro para a abertura seguinte devolver.
                    governador::gravar_em(&self.arquivo, &self.acalmados)?;
                }
                Err(codigo) => recusas.push(codigo),
            }
        }
        if self.acalmados.is_empty() {
            return Err(if recusas.contains(&5) {
                "O Windows recusou mexer nesse programa (acesso negado): ele roda como administrador e o Otimiza não."
                    .to_string()
            } else {
                "O programa fechou antes de ser acalmado.".to_string()
            });
        }
        Ok(())
    }

    /// Devolve tudo; o que falhar fica anotado para a próxima abertura.
    fn devolver(&mut self) -> Option<String> {
        if self.acalmados.is_empty() {
            return None;
        }
        let d = governador::devolver_estes(&self.acalmados);
        self.acalmados = d.falharam.clone();
        let anotado = governador::gravar_em(&self.arquivo, &self.acalmados).err();
        match (d.falharam.is_empty(), anotado) {
            (true, None) => None,
            (true, Some(e)) => Some(format!("Tudo voltou, mas não consegui apagar a anotação do teste ({}).", e)),
            (false, _) => Some(format!(
                "Não consegui devolver {} processo(s) agora; fica anotado e o Otimiza tenta de novo na próxima abertura.",
                d.falharam.len()
            )),
        }
    }
}

/// O teste ativo inteiro. `avisar`: uma frase por janela, para a tela.
pub fn testar(processo: &str, avisar: &dyn Fn(String)) -> Result<ResultadoDoTeste, String> {
    let (jogo, pid_do_jogo) =
        super::gamemode::jogo_aberto_com_pid().ok_or("O jogo fechou. Abra o jogo e investigue de novo antes de testar.")?;
    testar_em(&jogo, pid_do_jogo, processo, avisar)
}

/// O teste sobre um processo dado (o jogo detectado, ou o alvo do laboratório).
pub fn testar_em(jogo: &str, pid_do_jogo: u32, processo: &str, avisar: &dyn Fn(String)) -> Result<ResultadoDoTeste, String> {
    let _vez = Vez::pegar()?;
    let arquivo = anotacao().ok_or(SEM_APPDATA)?;
    let chave = super::gamemode::chave_do_processo(jogo);

    // A hipótese precisa ter vindo da última investigação DESTE jogo, recente.
    let taxa = {
        let ultima = ULTIMA.lock().unwrap_or_else(|e| e.into_inner());
        let (c, quando, inv) = ultima.as_ref().ok_or("Investigue as travadas antes: o teste só existe para uma suspeita medida.")?;
        if *c != chave || quando.elapsed() > VALIDADE_DA_INVESTIGACAO {
            return Err("A investigação é de outro jogo ou tem mais de 30 minutos: investigue de novo.".to_string());
        }
        let apontou = inv.avaliacoes.iter().any(|a| {
            a.confianca >= Confianca::Moderada
                && a.hipotese == Hipotese::Processo { nome: processo.to_string() }
                && matches!(a.teste, Teste::AcalmarProcesso { .. })
        });
        if !apontou {
            return Err(format!("A última investigação não apontou o {} como suspeito.", processo));
        }
        inv.por_minuto
    };
    teste::pode_testar(taxa)?;
    // Antes de conferir o modo jogo: dali em diante o governador não acalma nada novo (Drop desmarca em qualquer saída).
    let _testando = Testando::marcar();
    // A memória ilegível tira o teste da tela; uma chamada direta também não roda (poderia repetir um negativo).
    ler_memoria()?;
    if crate::modules::provaalternada::em_andamento() {
        return Err("A prova do Otimizar está rodando: teste depois que ela terminar.".to_string());
    }
    if crate::utils::diagnostico::modo_seguro() {
        return Err("O Otimiza está em modo seguro: nenhum teste que mexe no Windows roda até ele sair.".to_string());
    }
    if governador::protegido(processo) {
        return Err(format!("O {} é um programa que o Otimiza não mexe.", processo));
    }
    if governador::acalmados_pelo_modo_jogo()?.iter().any(|n| n.eq_ignore_ascii_case(processo)) {
        return Err(format!(
            "O modo jogo já está acalmando o {}: o lado \"normal\" do teste não existiria. Desligue o modo jogo para testar.",
            processo
        ));
    }
    if !governador::ler_de(&arquivo)?.is_empty() {
        return Err("Há um programa acalmado por um teste anterior. Use \"Devolver agora\" antes de testar de novo.".to_string());
    }
    let pids = alvos(processo, pid_do_jogo);
    if pids.is_empty() {
        return Err(format!("O {} não está aberto agora (ou é da pasta do jogo).", processo));
    }
    let frequencia = super::frames::frequencia_qpc().ok_or("o relógio de alta precisão do Windows não respondeu")?;

    let mut estado = Estado { acalmados: Vec::new(), manter: false, arquivo };
    let mut janelas: Vec<Janela> = Vec::new();
    let mut quadros: Vec<QuadroDoJogo> = Vec::new();
    let mut julgamento: Option<Julgamento> = None;
    let mut aviso: Option<String> = None;
    let total = teste::JANELAS_POR_OLHADA * teste::OLHADAS;
    crate::utils::Logger::info(&format!("teste da travada: {} ({} processo(s)) em {}", processo, pids.len(), jogo));

    'olhadas: for olhada in 1..=teste::OLHADAS {
        // Três segundos para o PresentMon abrir a sessão, oito janelas, dois de folga.
        let duracao = 3 + teste::JANELAS_POR_OLHADA as u32 * teste::SEGUNDOS_POR_JANELA as u32 + 2;
        let medicao = std::thread::spawn(move || super::presentmon::capturar(pid_do_jogo, duracao));
        std::thread::sleep(Duration::from_secs(3));
        for w in 0..teste::JANELAS_POR_OLHADA {
            let i = (olhada - 1) * teste::JANELAS_POR_OLHADA + w;
            let lado = teste::lado_da_janela(i);
            let troca = match lado {
                LadoDoTeste::Acalmado if estado.acalmados.is_empty() => estado.acalmar(&pids).err(),
                LadoDoTeste::Normal => {
                    aviso = estado.devolver();
                    None
                }
                _ => None,
            };
            if let Some(e) = troca {
                let devolucao = estado.devolver().or(aviso.take());
                let _ = medicao.join();
                return Err(match devolucao {
                    Some(d) => format!("O teste parou: {} {}", e, d),
                    None => format!("O teste parou: {} Nada ficou mudado.", e),
                });
            }
            let Some(inicio) = super::frames::agora_qpc() else { break 'olhadas };
            std::thread::sleep(Duration::from_secs(teste::SEGUNDOS_POR_JANELA));
            let Some(fim) = super::frames::agora_qpc() else { break 'olhadas };
            janelas.push(Janela { lado, inicio_qpc: inicio, fim_qpc: fim });
            avisar(format!(
                "Janela {} de até {}: {}.",
                i + 1,
                total,
                if lado == LadoDoTeste::Acalmado { format!("{} acalmado", processo) } else { "tudo normal".to_string() }
            ));
            if medicao.is_finished() {
                // O PresentMon parou antes da hora: o jogo fechou.
                break;
            }
        }
        aviso = estado.devolver();
        match medicao.join() {
            Ok(Ok(q)) => quadros.extend(quadros_do_jogo(&q)),
            Ok(Err(e)) => crate::utils::Logger::warn(&format!("teste da travada: PresentMon falhou na olhada {}: {}", olhada, e)),
            Err(_) => crate::utils::Logger::warn("teste da travada: a medição parou no meio"),
        }
        quadros.sort_by_key(|q| q.qpc);
        let resumos = teste::resumir_janelas(&quadros, &janelas, frequencia);
        let j = teste::julgar(&resumos, processo, olhada);
        let parar = j.decisao != Decisao::Continuar || resumos.iter().rev().take(teste::JANELAS_POR_OLHADA).all(|r| !r.valida);
        julgamento = Some(j);
        if parar {
            break;
        }
    }
    // Qualquer caminho que saiu do laço antes do fim de uma olhada chega aqui com tudo devolvido.
    aviso = estado.devolver();
    let mut julgamento = julgamento.ok_or("O teste não chegou a medir nenhuma janela.")?;
    if julgamento.decisao == Decisao::Continuar {
        // Parou por janelas inválidas antes da última olhada: julgamento final com o que houve.
        let resumos = teste::resumir_janelas(&quadros, &janelas, frequencia);
        julgamento = teste::julgar(&resumos, processo, teste::OLHADAS);
    }

    let mut mantido = false;
    if julgamento.decisao == Decisao::Manter && !estado.acalmados.is_empty() {
        // Algo não voltou: acalmar de novo gravaria como "anterior" o estado do próprio teste. Não mantém; o aviso
        // da devolução já diz o que ficou anotado.
        crate::utils::Logger::warn("teste da travada: causa demonstrada, mas não mantido porque a devolução falhou");
    } else if julgamento.decisao == Decisao::Manter {
        match estado.acalmar(&alvos(processo, pid_do_jogo)) {
            Ok(()) => {
                mantido = true;
                estado.manter = true;
                vigiar_o_fim_do_jogo(pid_do_jogo);
            }
            Err(e) => aviso = Some(format!("A causa foi demonstrada, mas não consegui deixar o programa acalmado: {}", e)),
        }
    }
    let (driver, windows) = contexto_agora();
    match memoria().ok_or(SEM_APPDATA.to_string()).and_then(|m| guardar_na_memoria_em(&m, teste::registro(&chave, processo, &julgamento, driver, windows))) {
        Ok(None) => {}
        Ok(Some(a)) => {
            crate::utils::Logger::warn(&a);
            aviso = Some(aviso.map(|x| format!("{} {}", x, a)).unwrap_or(a));
        }
        Err(e) => crate::utils::Logger::warn(&e),
    }
    crate::utils::Logger::info(&format!("teste da travada: {:?} · {}", julgamento.decisao, julgamento.frase));
    Ok(ResultadoDoTeste { processo: processo.to_string(), julgamento, mantido, aviso })
}

/// Com a causa demonstrada, o programa fica acalmado até o jogo fechar; então volta sozinho.
fn vigiar_o_fim_do_jogo(pid_do_jogo: u32) {
    std::thread::spawn(move || {
        let alvo = sysinfo::Pid::from_u32(pid_do_jogo);
        let mut s = sysinfo::System::new();
        loop {
            std::thread::sleep(Duration::from_secs(5));
            s.refresh_processes_specifics(sysinfo::ProcessesToUpdate::Some(&[alvo]), true, sysinfo::ProcessRefreshKind::nothing());
            if s.process(alvo).is_none() {
                break;
            }
            if anotacao().and_then(|a| governador::ler_de(&a).ok()).is_some_and(|l| l.is_empty()) {
                return; // já devolvido pela pessoa
            }
        }
        // Tenta até conseguir (um teste novo rodando recusa por alguns minutos); o que o Windows recusar fica anotado
        // para a abertura seguinte.
        for _ in 0..120 {
            match devolver_mantido() {
                Ok(Some(frase)) => {
                    crate::utils::Logger::info(&format!("teste da travada, o jogo fechou: {}", frase));
                    return;
                }
                Ok(None) => return,
                Err(e) if TESTANDO.load(Ordering::SeqCst) => {
                    crate::utils::Logger::info(&format!("teste da travada, o jogo fechou: {}; tento de novo", e));
                    std::thread::sleep(Duration::from_secs(10));
                }
                Err(e) => {
                    crate::utils::Logger::warn(&format!("teste da travada, o jogo fechou: {}", e));
                    return;
                }
            }
        }
    });
}

/// Há algo acalmado pelo teste (mantido, ou deixado por um fechamento no meio).
pub fn ha_acalmado() -> bool {
    anotacao().map(|a| governador::ler_de(&a).map(|l| !l.is_empty()).unwrap_or(true)).unwrap_or(false)
}

/// "Devolver agora", o jogo fechando, o "Restaurar meu PC" e a abertura seguinte. `Ok(None)`: nada anotado.
pub fn devolver_mantido() -> Result<Option<String>, String> {
    if TESTANDO.load(Ordering::SeqCst) {
        return Err("O teste da travada está rodando; ele devolve tudo ao terminar.".to_string());
    }
    match anotacao() {
        Some(a) => devolver_de(&a),
        None => Ok(None),
    }
}

/// Anotação ilegível é guardada à parte (não se apaga o único registro do que ficou acalmado) e dita como erro: os
/// programas voltam sozinhos quando fecham ou quando o Windows reinicia, porque a prioridade vive só na memória.
fn devolver_de(arquivo: &std::path::Path) -> Result<Option<String>, String> {
    let lista = match governador::ler_de(arquivo) {
        Ok(l) => l,
        Err(e) => {
            let guardado = arquivo.with_extension(format!("ilegivel-{}.json", crate::modules::changelog::now_timestamp()));
            let onde = std::fs::rename(arquivo, &guardado).map(|_| guardado.display().to_string());
            return Err(format!(
                "A anotação do teste da travada está ilegível ({}){}. O que ela guardava volta sozinho quando o programa \
                 fechar ou o Windows reiniciar.",
                e,
                onde.map(|g| format!("; guardada em {}", g)).unwrap_or_default()
            ));
        }
    };
    if lista.is_empty() {
        return Ok(None);
    }
    let d = governador::devolver_estes(&lista);
    governador::gravar_em(arquivo, &d.falharam)?;
    if d.falharam.is_empty() {
        Ok(Some(format!(
            "Prioridade devolvida a {} processo(s){}.",
            d.devolvidos,
            if d.sumidos > 0 { format!(" ({} já tinham fechado)", d.sumidos) } else { String::new() }
        )))
    } else {
        Err(format!("Não consegui devolver {} processo(s); fica anotado para a próxima abertura.", d.falharam.len()))
    }
}

/// Na abertura: um teste interrompido (app fechado, queda, desligamento) deixa a anotação.
pub fn recuperar_na_abertura() -> Result<Option<String>, String> {
    devolver_mantido()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::windows::presentmon::{Quadro, TipoDeQuadro};

    fn q(cadeia: &str, tipo: TipoDeQuadro, qpc: Option<i64>, ms: Option<f64>) -> Quadro {
        Quadro {
            cadeia: cadeia.into(),
            tipo,
            modo: "Hardware: Independent Flip".into(),
            runtime: "DXGI".into(),
            inicio_qpc: qpc,
            intervalo_ms: ms,
            cpu_ocupada_ms: Some(3.0),
            gpu_ocupada_ms: Some(4.0),
            na_tela_ms: ms,
            ate_a_tela_ms: None,
        }
    }

    #[test]
    fn so_a_cadeia_principal_e_so_quadro_do_jogo() {
        // O caso do FiveM: uma segunda cadeia (interface) intercalada dobraria a contagem.
        let mut v = Vec::new();
        for i in 0..100 {
            v.push(q("0xJOGO", TipoDeQuadro::Jogo, Some(i * 100), Some(10.0)));
            if i % 2 == 0 {
                v.push(q("0xUI", TipoDeQuadro::Jogo, Some(i * 100 + 50), Some(20.0)));
            }
        }
        v.push(q("0xJOGO", TipoDeQuadro::Gerado("XeFG".into()), Some(10_050), Some(5.0)));
        v.push(q("0xJOGO", TipoDeQuadro::Jogo, None, Some(10.0)));
        v.push(q("0xJOGO", TipoDeQuadro::Jogo, Some(10_100), Some(f64::NAN)));
        let quadros = quadros_do_jogo(&v);
        assert_eq!(quadros.len(), 100);
        assert!(quadros.windows(2).all(|w| w[0].qpc < w[1].qpc));
    }

    fn pasta_de_teste(nome: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("otimiza-investigacao-{}-{}", nome, std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn processo_que_sumiu_nao_e_acalmado_nem_anotado() {
        let arquivo = pasta_de_teste("sumiu").join("teste_da_travada.json");
        let mut e = Estado { acalmados: Vec::new(), manter: false, arquivo: arquivo.clone() };
        let erro = e.acalmar(&[u32::MAX - 7]).unwrap_err();
        assert!(erro.contains("fechou"), "{}", erro);
        assert!(!arquivo.exists());
    }

    #[test]
    fn anotacao_ilegivel_e_guardada_a_parte_e_dita() {
        let pasta = pasta_de_teste("ilegivel");
        let arquivo = pasta.join("teste_da_travada.json");
        std::fs::write(&arquivo, "{ isto não é json").unwrap();
        let erro = devolver_de(&arquivo).unwrap_err();
        assert!(erro.contains("ilegível") && erro.contains("guardada em"), "{}", erro);
        assert!(!arquivo.exists());
        let guardados = std::fs::read_dir(&pasta).unwrap().count();
        assert_eq!(guardados, 1, "a cópia ilegível fica, não some");
        assert_eq!(devolver_de(&arquivo).unwrap(), None, "depois disso, nada anotado");
    }

    #[test]
    fn na_abertura_processo_que_ja_fechou_limpa_a_anotacao() {
        let arquivo = pasta_de_teste("abertura").join("teste_da_travada.json");
        let fantasma = Acalmado {
            pid: u32::MAX - 9,
            nome: "rajada.exe".into(),
            inicio: 1,
            prioridade_anterior: 0x20,
            ecoqos: governador::Eco::NaoMexido,
        };
        governador::gravar_em(&arquivo, &[fantasma]).unwrap();
        let frase = devolver_de(&arquivo).unwrap().unwrap();
        assert!(frase.contains("já tinham fechado"), "{}", frase);
        assert!(!arquivo.exists());
    }

    fn prioridade(pid: u32) -> u32 {
        use windows_sys::Win32::System::Threading::{GetPriorityClass, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
        unsafe {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            let p = GetPriorityClass(h);
            windows_sys::Win32::Foundation::CloseHandle(h);
            p
        }
    }

    /// Falha no meio (pânico): o `Drop` devolve a prioridade e apaga a anotação. Só um processo filho do teste.
    #[test]
    #[ignore = "abre um processo filho e mexe na prioridade dele"]
    fn panico_no_meio_do_teste_devolve_a_prioridade() {
        let mut filho = std::process::Command::new("cmd").args(["/C", "ping -n 60 127.0.0.1 >nul"]).spawn().unwrap();
        std::thread::sleep(Duration::from_millis(500));
        let pid = filho.id();
        let arquivo = pasta_de_teste("panico").join("teste_da_travada.json");
        assert_eq!(prioridade(pid), 0x20);
        let a = arquivo.clone();
        let r = std::panic::catch_unwind(move || {
            let mut e = Estado { acalmados: Vec::new(), manter: false, arquivo: a.clone() };
            e.acalmar(&[pid]).unwrap();
            assert_eq!(prioridade(pid), 0x4000, "acalmado");
            assert!(a.exists(), "anotado antes de seguir");
            panic!("falha injetada no meio do teste");
        });
        assert!(r.is_err());
        assert_eq!(prioridade(pid), 0x20, "o Drop devolveu");
        assert!(!arquivo.exists(), "e apagou a anotação");
        let _ = filho.kill();
    }

    /// O Otimiza morre com a causa demonstrada e o programa acalmado: a abertura seguinte devolve pela anotação.
    #[test]
    #[ignore = "abre um processo filho e mexe na prioridade dele"]
    fn app_morto_com_programa_mantido_a_abertura_devolve() {
        let mut filho = std::process::Command::new("cmd").args(["/C", "ping -n 60 127.0.0.1 >nul"]).spawn().unwrap();
        std::thread::sleep(Duration::from_millis(500));
        let pid = filho.id();
        let arquivo = pasta_de_teste("morto").join("teste_da_travada.json");
        {
            let mut e = Estado { acalmados: Vec::new(), manter: true, arquivo: arquivo.clone() };
            e.acalmar(&[pid]).unwrap();
        } // "morre" sem devolver: `manter`
        assert_eq!(prioridade(pid), 0x4000);
        assert!(arquivo.exists());
        let frase = devolver_de(&arquivo).unwrap().unwrap();
        assert!(frase.contains("devolvida a 1"), "{}", frase);
        assert_eq!(prioridade(pid), 0x20);
        assert!(!arquivo.exists());
        let _ = filho.kill();
    }

    #[test]
    fn memoria_ilegivel_e_erro_e_nao_e_sobrescrita() {
        let pasta = pasta_de_teste("memoria");
        let arquivo = pasta.join("travadas_testadas.json");
        std::fs::write(&arquivo, "[{ quebrado").unwrap();
        assert!(ler_memoria_de(&arquivo).is_err(), "ilegível não é \"nunca testado\"");
        let registro = Registro {
            quando: 1,
            jogo: "fivem_".into(),
            processo: "X.exe".into(),
            decisao: Decisao::SemMelhoriaConfiavel,
            razao: None,
            ic: None,
            travadas_normal: 0,
            minutos_normal: 0.0,
            travadas_acalmado: 0,
            minutos_acalmado: 0.0,
            variacao_do_fps: None,
            versao_do_otimiza: "t".into(),
            driver: None,
            windows: None,
        };
        let aviso = guardar_na_memoria_em(&arquivo, registro).unwrap().unwrap();
        assert!(aviso.contains("guardada em"), "{}", aviso);
        assert_eq!(ler_memoria_de(&arquivo).unwrap().len(), 1);
        let copias: Vec<_> = std::fs::read_dir(&pasta).unwrap().flatten().filter(|e| e.file_name().to_string_lossy().contains("ilegivel")).collect();
        assert_eq!(copias.len(), 1, "o histórico ilegível fica guardado");
        assert_eq!(std::fs::read_to_string(copias[0].path()).unwrap(), "[{ quebrado");
    }

    #[test]
    fn sem_quadro_do_jogo_nao_ha_cadeia() {
        assert!(quadros_do_jogo(&[q("0xA", TipoDeQuadro::Repetido, Some(1), Some(1.0))]).is_empty());
    }

    /// Replay offline: roda o motor sobre uma captura gravada. `OTIMIZA_REPLAY=arquivo.json`.
    #[test]
    #[ignore = "replay de uma captura gravada"]
    fn replay_de_captura() {
        let arquivo = std::env::var("OTIMIZA_REPLAY").expect("OTIMIZA_REPLAY");
        let c: CapturaGravada = serde_json::from_str(&std::fs::read_to_string(arquivo).unwrap()).unwrap();
        let inv = motor::investigar(&c.quadros, &c.amostras, c.frequencia, c.vram_total_mb, &governador::protegido);
        println!("{}", inv.conclusao);
        for a in inv.avaliacoes.iter().take(6) {
            println!("{:?} {:?} k={} n={} p0={:.2} lift={:?} p={:?}", a.hipotese, a.confianca, a.episodios_com_sinal, a.episodios_cobertos, a.taxa_de_base, a.lift, a.p_corrigido);
        }
    }

    /// Laboratório: investiga o jogo aberto agora. Exige administrador (PresentMon).
    /// `cargo test --release --lib investigacao_real -- --ignored --nocapture`
    #[test]
    #[ignore = "laboratório: precisa do jogo aberto e de administrador"]
    fn investigacao_real() {
        let r = investigar(60).expect("investigação");
        println!("{}", serde_json::to_string_pretty(&r).unwrap());
    }

    /// Laboratório com a verdade conhecida (docs/planos/PIP-RESULTADO.md): OTIMIZA_LAB_PID = o processo que apresenta
    /// os quadros; OTIMIZA_LAB_CARGA = o nome do processo que dispara rajadas (a causa plantada); OTIMIZA_LAB_SAIDA =
    /// arquivo JSON do resultado. Roda a investigação e, se ela apontar a carga, o teste ativo inteiro.
    #[test]
    #[ignore = "laboratório: precisa de administrador, da fonte de quadros e da carga"]
    fn laboratorio_causa_plantada() {
        let pid: u32 = std::env::var("OTIMIZA_LAB_PID").expect("OTIMIZA_LAB_PID").parse().unwrap();
        let carga = std::env::var("OTIMIZA_LAB_CARGA").expect("OTIMIZA_LAB_CARGA");
        let saida = std::env::var("OTIMIZA_LAB_SAIDA").expect("OTIMIZA_LAB_SAIDA");
        let segundos: u32 = std::env::var("OTIMIZA_LAB_SEGUNDOS").ok().and_then(|s| s.parse().ok()).unwrap_or(60);
        let mut relatorio = serde_json::Map::new();
        // Primeiro uma captura crua, gravada, para o motor rodar de novo offline sobre os mesmos dados.
        if let Ok(caminho) = std::env::var("OTIMIZA_LAB_CAPTURA") {
            match capturar(pid, segundos) {
                Ok(c) => {
                    let gravada = CapturaGravada { quadros: c.quadros, amostras: c.amostras, frequencia: c.frequencia, vram_total_mb: c.vram_total_mb };
                    let inv = motor::investigar(&gravada.quadros, &gravada.amostras, gravada.frequencia, gravada.vram_total_mb, &governador::protegido);
                    println!("captura crua: {} quadros, {} amostras · {}", gravada.quadros.len(), gravada.amostras.len(), inv.conclusao);
                    std::fs::write(&caminho, serde_json::to_string(&gravada).unwrap()).unwrap();
                }
                Err(e) => println!("captura crua falhou: {}", e),
            }
        }
        let inv = investigar_em("laboratorio.exe", pid, segundos);
        println!("investigação: {:?}", inv.as_ref().map(|r| (&r.investigacao.conclusao, &r.custo, &r.teste_possivel)));
        relatorio.insert("investigacao".into(), match &inv {
            Ok(r) => serde_json::to_value(r).unwrap(),
            Err(e) => serde_json::Value::String(e.clone()),
        });
        let apontou = inv.as_ref().ok().and_then(|r| r.teste_possivel.clone());
        if apontou.as_deref().is_some_and(|p| p.eq_ignore_ascii_case(&carga)) {
            let r = testar_em("laboratorio.exe", pid, &carga, &|frase| println!("{}", frase));
            println!("teste: {:?}", r.as_ref().map(|x| (&x.julgamento.frase, x.mantido)));
            relatorio.insert("teste".into(), match &r {
                Ok(x) => serde_json::to_value(x).unwrap(),
                Err(e) => serde_json::Value::String(e.clone()),
            });
            // O laboratório não deixa nada acalmado.
            println!("devolução: {:?}", devolver_mantido());
        }
        relatorio.insert("sobrou_anotado".into(), serde_json::Value::Bool(ha_acalmado()));
        std::fs::write(&saida, serde_json::to_string_pretty(&relatorio).unwrap()).unwrap();
    }
}
