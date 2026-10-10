// Rastro de falha para o suporte. Sem isto, um programa que fechava sozinho no cliente não deixava nada: o
// Relatório de Erros do Windows costuma estar desligado em PC "otimizado", e o log parava no meio da frase.
// Só grava na pasta de dados do Otimiza; nada sai da máquina.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// Duas quedas seguidas: uma pode ser acaso; a segunda, com as mesmas tarefas automáticas rodando, é suspeita delas.
pub const QUEDAS_PARA_MODO_SEGURO: u32 = 2;

static MODO_SEGURO: AtomicBool = AtomicBool::new(false);
static QUEDAS: AtomicU32 = AtomicU32::new(0);
static SAIDA_POR_FALHA: AtomicBool = AtomicBool::new(false);

/// A janela vai reabrir por uma falha (WebView2): o fechamento que vem é queda, não fim normal. Sem isto, uma falha em
/// laço reabriria para sempre sem nunca chegar ao modo seguro.
pub fn sair_por_falha() {
    SAIDA_POR_FALHA.store(true, Ordering::SeqCst);
}

/// Modo seguro: as tarefas que agem sozinhas (o governador do modo jogo, modo dinâmico de energia, Auto CPU Set, medição automática)
/// ficam paradas nesta sessão. O que DEVOLVE coisa (recuperações da abertura, desfazer) continua rodando.
pub fn modo_seguro() -> bool {
    MODO_SEGURO.load(Ordering::SeqCst)
}

pub fn quedas_seguidas() -> u32 {
    QUEDAS.load(Ordering::SeqCst)
}

/// A pessoa escolheu sair: zera a contagem no disco e religa as tarefas automáticas já nesta sessão.
pub fn sair_do_modo_seguro() {
    MODO_SEGURO.store(false, Ordering::SeqCst);
    QUEDAS.store(0, Ordering::SeqCst);
    let Some(arquivo) = pasta().map(|p| p.join("sessao.json")) else {
        return;
    };
    if let Some(mut s) = std::fs::read_to_string(&arquivo)
        .ok()
        .and_then(|t| serde_json::from_str::<Sessao>(&t).ok())
    {
        s.quedas_seguidas = 0;
        if let Ok(json) = serde_json::to_string(&s) {
            let _ = std::fs::write(&arquivo, json);
        }
    }
}

fn pasta() -> Option<PathBuf> {
    if cfg!(test) {
        return None;
    }
    let base = std::env::var("APPDATA").ok()?;
    Some(PathBuf::from(base).join("pc-optimizer"))
}

/// Pânico em qualquer thread vira um arquivo `falhas\falha-AAAAMMDD-HHMMSS.txt` e uma linha no log, com versão,
/// thread, local e pilha. Chama o gancho anterior depois, para não mudar o que o processo faz em seguida.
pub fn instalar_gancho_de_panico() {
    let anterior = std::panic::take_hook();

    std::panic::set_hook(Box::new(move |info| {
        let thread = std::thread::current();
        let texto = relatorio_de_panico(
            thread.name().unwrap_or("sem nome"),
            &info
                .location()
                .map(|l| format!("{}:{}", l.file(), l.line()))
                .unwrap_or_default(),
            &mensagem_do_panico(info.payload()),
            &std::backtrace::Backtrace::force_capture().to_string(),
        );

        super::Logger::error(&format!(
            "PÂNICO: {}",
            texto.lines().take(4).collect::<Vec<_>>().join(" | ")
        ));

        if let Some(pasta) = pasta().map(|p| p.join("falhas")) {
            let _ = std::fs::create_dir_all(&pasta);
            let nome = format!("falha-{}.txt", chrono::Local::now().format("%Y%m%d-%H%M%S"));
            let _ = std::fs::write(pasta.join(nome), &texto);
        }

        anterior(info);
    }));
}

fn mensagem_do_panico(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "pânico sem mensagem".to_string())
}

/// Pura, para o teste: o que vai para o arquivo.
fn relatorio_de_panico(thread: &str, local: &str, mensagem: &str, pilha: &str) -> String {
    format!(
        "Otimiza {} — falha\nquando: {}\nthread: {}\nlocal: {}\nmensagem: {}\n\npilha:\n{}\n",
        env!("CARGO_PKG_VERSION"),
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
        thread,
        local,
        mensagem,
        pilha
    )
}

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
struct Sessao {
    versao: String,
    aberta_em: String,
    pid: u32,
    fechou_normalmente: bool,
    /// Sessões seguidas que terminaram sem fechar normalmente (esta não conta).
    #[serde(default)]
    quedas_seguidas: u32,
    /// Segundos desde 1970; 0 em sessão gravada antes deste campo.
    #[serde(default)]
    aberta_em_epoch: u64,
    /// A impressão do executável que rodava (gravação e tamanho); vazio em sessão gravada antes deste campo.
    #[serde(default)]
    exe: String,
}

/// Pura. Windows que reiniciou depois da sessão aberta não é queda do Otimiza: o desligamento pode não ter dado
/// tempo de marcar o fim (o fim de sessão marca, mas não sempre). Sessão anterior ainda VIVA (segunda cópia, ou a
/// cópia que pediu administrador e ainda está fechando) também não: herda a contagem dela sem somar. Executável
/// diferente do que a sessão rodava (`exe_agora`) é atualização: o instalador encerra o Otimiza aberto para trocar o
/// arquivo. No PC do dono, 6 das 7 "quedas" seguidas de 09/10 eram instalações, e o modo seguro parou o automático.
/// Pela impressão e não pela data: o instalador preserva a data da compilação, que pode ser anterior à sessão.
fn quedas_agora(anterior: Option<&Sessao>, boot_epoch: u64, anterior_viva: bool, exe_agora: &str) -> u32 {
    match anterior {
        None => 0,
        Some(s) if anterior_viva => s.quedas_seguidas,
        Some(s) if s.fechou_normalmente => 0,
        Some(s) if s.aberta_em_epoch > 0 && boot_epoch > s.aberta_em_epoch => 0,
        Some(s) if !s.exe.is_empty() && !exe_agora.is_empty() && s.exe != exe_agora => s.quedas_seguidas,
        Some(s) => s.quedas_seguidas + 1,
    }
}

/// A impressão do executável que está rodando: gravação e tamanho. Vazia sem leitura.
fn impressao_do_exe() -> String {
    std::env::current_exe()
        .and_then(std::fs::metadata)
        .ok()
        .and_then(|m| {
            let quando = m.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?;
            Some(format!("{}:{}", quando.as_secs(), m.len()))
        })
        .unwrap_or_default()
}

/// Ao abrir: se a sessão anterior não marcou fim normal, o log diz. Depois anota a sessão nova.
pub fn abrir_sessao() {
    let Some(arquivo) = pasta().map(|p| p.join("sessao.json")) else {
        return;
    };

    let anterior: Option<Sessao> = std::fs::read_to_string(&arquivo)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok());

    let viva = anterior
        .as_ref()
        .is_some_and(|s| processo_do_otimiza_vivo(s.pid));
    let quedas = quedas_agora(
        anterior.as_ref(),
        sysinfo::System::boot_time(),
        viva,
        &impressao_do_exe(),
    );

    if let Some(frase) = anterior
        .as_ref()
        .filter(|_| quedas > 0)
        .and_then(aviso_da_sessao_anterior)
    {
        super::Logger::warn(&frase);
    }
    QUEDAS.store(quedas, Ordering::SeqCst);
    if quedas >= QUEDAS_PARA_MODO_SEGURO {
        MODO_SEGURO.store(true, Ordering::SeqCst);
        super::Logger::warn(&format!(
            "modo seguro: {} sessões seguidas terminaram sem fechar normalmente; as tarefas automáticas ficam paradas",
            quedas
        ));
    }

    let nova = Sessao {
        versao: env!("CARGO_PKG_VERSION").to_string(),
        aberta_em: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        pid: std::process::id(),
        fechou_normalmente: false,
        quedas_seguidas: quedas,
        aberta_em_epoch: chrono::Utc::now().timestamp().max(0) as u64,
        exe: impressao_do_exe(),
    };
    if let Ok(json) = serde_json::to_string(&nova) {
        let _ = std::fs::create_dir_all(arquivo.parent().unwrap_or(&arquivo));
        let _ = std::fs::write(&arquivo, json);
    }
}

/// Pelo nome do executável também: o pid de um processo morto pode ter sido reaproveitado por outro programa.
fn processo_do_otimiza_vivo(pid: u32) -> bool {
    if pid == std::process::id() {
        return false;
    }
    let Some(nosso) = std::env::current_exe()
        .ok()
        .and_then(|c| c.file_name().map(|n| n.to_owned()))
    else {
        return false;
    };
    let alvo = sysinfo::Pid::from_u32(pid);
    let mut s = sysinfo::System::new();
    s.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[alvo]), true);
    s.process(alvo)
        .is_some_and(|p| p.name().eq_ignore_ascii_case(&nosso))
}

fn aviso_da_sessao_anterior(s: &Sessao) -> Option<String> {
    (!s.fechou_normalmente).then(|| {
        format!(
            "A sessão anterior (versão {}, aberta em {}, processo {}) não fechou normalmente: o programa caiu, travou \
             e foi encerrado, ou o Windows desligou com ele aberto.",
            s.versao, s.aberta_em, s.pid
        )
    })
}

/// No fechamento normal.
pub fn fechar_sessao() {
    if SAIDA_POR_FALHA.load(Ordering::SeqCst) {
        return;
    }
    let Some(arquivo) = pasta().map(|p| p.join("sessao.json")) else {
        return;
    };

    let Some(mut s) = std::fs::read_to_string(&arquivo)
        .ok()
        .and_then(|t| serde_json::from_str::<Sessao>(&t).ok())
    else {
        return;
    };

    // Só a própria sessão: uma segunda cópia aberta não marca a da outra.
    if s.pid == std::process::id() && !s.fechou_normalmente {
        s.fechou_normalmente = true;
        if let Ok(json) = serde_json::to_string(&s) {
            let _ = std::fs::write(&arquivo, json);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sessao_que_nao_fechou_vira_aviso_e_a_que_fechou_nao() {
        let mut s = Sessao {
            versao: "3.1.0".into(),
            aberta_em: "2026-09-25 18:27:13".into(),
            pid: 11028,
            fechou_normalmente: false,
            quedas_seguidas: 0,
            aberta_em_epoch: 0,
            exe: String::new(),
        };
        let aviso = aviso_da_sessao_anterior(&s).expect("sessão aberta vira aviso");
        assert!(aviso.contains("3.1.0") && aviso.contains("18:27:13") && aviso.contains("11028"));

        s.fechou_normalmente = true;
        assert_eq!(aviso_da_sessao_anterior(&s), None);
    }

    #[test]
    fn quedas_seguidas_contam_so_o_que_foi_queda() {
        let sessao = |fechou, quedas, aberta| Sessao {
            versao: "3.2.0".into(),
            aberta_em: String::new(),
            pid: 1,
            fechou_normalmente: fechou,
            quedas_seguidas: quedas,
            aberta_em_epoch: aberta,
            exe: String::new(),
        };
        assert_eq!(quedas_agora(None, 100, false, ""), 0);
        assert_eq!(
            quedas_agora(Some(&sessao(true, 5, 50)), 10, false, ""),
            0,
            "fechou normalmente zera"
        );
        assert_eq!(quedas_agora(Some(&sessao(false, 0, 50)), 10, false, ""), 1);
        assert_eq!(
            quedas_agora(Some(&sessao(false, 1, 50)), 10, false, ""),
            QUEDAS_PARA_MODO_SEGURO
        );
        assert_eq!(
            quedas_agora(Some(&sessao(false, 3, 50)), 100, false, ""),
            0,
            "o Windows reiniciou depois: não foi queda"
        );
        assert_eq!(
            quedas_agora(Some(&sessao(false, 0, 0)), 100, false, ""),
            1,
            "sessão antiga, sem o instante: conta"
        );
        assert_eq!(
            quedas_agora(Some(&sessao(false, 1, 50)), 10, true, ""),
            1,
            "a anterior ainda está aberta: herda, não soma"
        );
        let mut com_exe = sessao(false, 1, 50);
        com_exe.exe = "1000:25723904".into();
        assert_eq!(
            quedas_agora(Some(&com_exe), 10, false, "900:25835008"),
            1,
            "o executável mudou (mesmo com data de compilação mais antiga): atualização, não queda"
        );
        assert_eq!(
            quedas_agora(Some(&com_exe), 10, false, "1000:25723904"),
            2,
            "o mesmo executável: queda"
        );
    }

    #[test]
    fn o_relatorio_de_panico_leva_o_que_o_suporte_precisa() {
        let r = relatorio_de_panico("main", "src/commands.rs:726", "índice fora", "0: pilha");
        for trecho in [
            env!("CARGO_PKG_VERSION"),
            "thread: main",
            "src/commands.rs:726",
            "índice fora",
            "0: pilha",
        ] {
            assert!(r.contains(trecho), "faltou {trecho:?} em {r}");
        }
    }

    #[test]
    fn a_mensagem_sai_de_str_e_de_string() {
        assert_eq!(mensagem_do_panico(&"texto"), "texto");
        assert_eq!(mensagem_do_panico(&String::from("dono")), "dono");
        assert_eq!(mensagem_do_panico(&42u8), "pânico sem mensagem");
    }
}
