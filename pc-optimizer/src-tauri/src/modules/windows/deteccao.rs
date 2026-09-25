// Reconhecer um jogo sem lista de nomes (a lista de cinco deixava Fortnite, LoL e todo lançamento invisíveis).
// Quatro sinais AO MESMO TEMPO, sem pontuação (que deixa dois fracos valerem um forte): janela cobrindo o
// monitor, o MESMO processo no motor 3D, aberto há tempo, fora da recusa. Navegador e Electron não fecham (quem
// desenha é um processo sem janela), vídeo também (usa decode, não 3D). Não autoriza IFEO: aquilo exige
// `jogos::dentro_de_biblioteca`.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Não é 100%: janela sem bordas mede alguns pixels a mais ou a menos, e a barra de tarefas às vezes conta.
const COBERTURA_MINIMA: f64 = 0.90;

const USO_3D_MINIMO: f64 = 20.0;

/// No primeiro instante a medição de GPU não vale nada.
const SEGUNDOS_MINIMOS: u64 = 20;

/// Desenham em 3D e ocupam a tela inteira sem ser jogo: ligar o modo jogo porque abriram o OBS mexeria na
/// energia pelo motivo errado.
pub const NAO_E_JOGO: &[(&str, &str)] = &[
    ("obs64.exe", "grava a tela: usa o motor 3D em tela cheia igual a um jogo"),
    ("obs32.exe", "grava a tela"),
    ("streamlabs obs.exe", "grava a tela"),
    ("xsplit.core.exe", "grava a tela"),
    ("adobe premiere pro.exe", "a prévia em tela cheia usa o motor 3D"),
    ("afterfx.exe", "a prévia em tela cheia usa o motor 3D"),
    ("resolve.exe", "a prévia em tela cheia usa o motor 3D"),
    ("vegas180.exe", "a prévia em tela cheia usa o motor 3D"),
    ("blender.exe", "a janela de trabalho em 3D ocupa a tela inteira"),
    ("unity.exe", "editor de jogo não é jogo"),
    ("unityhub.exe", "editor de jogo não é jogo"),
    ("unrealeditor.exe", "editor de jogo não é jogo"),
    ("ue4editor.exe", "editor de jogo não é jogo"),
    ("godot.exe", "editor de jogo não é jogo"),
    ("dwm.exe", "é o próprio compositor de janelas do Windows"),
    ("explorer.exe", "é a área de trabalho"),
];

pub fn recusado_por(executavel: &str) -> Option<&'static str> {
    let nome = executavel.trim().to_lowercase();

    NAO_E_JOGO
        .iter()
        .find(|(exe, _)| nome == *exe)
        .map(|(_, motivo)| *motivo)
}

/// Vai para a tela: essa decisão muda o plano de energia do cliente.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Sinal {
    JanelaCobrindoMonitor(u8),
    MotorGrafico(f64),
    AbertoHa(u64),
    NomeConhecido(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JogoDetectado {
    pub pid: u32,
    pub executavel: String,
    pub caminho: Option<PathBuf>,
    pub nome: String,
    pub conhecido: bool,
    pub sinais: Vec<Sinal>,
}

/// Separado da leitura para a regra ser testável sem abrir jogo.
#[derive(Debug, Clone)]
pub struct Observacao {
    pub pid: u32,
    pub executavel: String,
    pub caminho: Option<PathBuf>,
    pub cobertura: Option<f64>,
    /// `None` quando não foi possível medir, que é diferente de zero.
    pub uso_3d: Option<f64>,
    pub segundos_aberto: u64,
}

pub fn decidir(obs: &Observacao) -> Option<JogoDetectado> {
    if recusado_por(&obs.executavel).is_some() {
        return None;
    }

    // Contador ausente NÃO é zero: "não sei" nunca vira "não é jogo" nem "é".
    let uso = obs.uso_3d?;
    let cobertura = obs.cobertura?;

    if cobertura < COBERTURA_MINIMA || uso < USO_3D_MINIMO || obs.segundos_aberto < SEGUNDOS_MINIMOS
    {
        return None;
    }

    let conhecido = super::gamemode::nome_do_jogo(&obs.executavel);

    let mut sinais = vec![
        Sinal::JanelaCobrindoMonitor((cobertura * 100.0).round() as u8),
        Sinal::MotorGrafico((uso * 10.0).round() / 10.0),
        Sinal::AbertoHa(obs.segundos_aberto),
    ];

    if let Some(nome) = conhecido {
        sinais.push(Sinal::NomeConhecido(nome.to_string()));
    }

    Some(JogoDetectado {
        pid: obs.pid,
        executavel: obs.executavel.clone(),
        caminho: obs.caminho.clone(),
        nome: conhecido
            .map(str::to_string)
            .unwrap_or_else(|| nome_apresentavel(&obs.executavel)),
        conhecido: conhecido.is_some(),
        sinais,
    })
}

/// De propósito: inventar nome comercial a partir do arquivo erraria; o executável é um fato.
fn nome_apresentavel(executavel: &str) -> String {
    let sem_extensao = executavel
        .rsplit_once('.')
        .map(|(base, _)| base)
        .unwrap_or(executavel);

    let mut letras = sem_extensao.chars();

    match letras.next() {
        Some(primeira) => primeira.to_uppercase().collect::<String>() + letras.as_str(),
        None => executavel.to_string(),
    }
}

#[cfg(target_os = "windows")]
pub fn janela_em_primeiro_plano() -> Option<(u32, f64)> {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowRect, GetWindowThreadProcessId, IsWindowVisible,
    };

    unsafe {
        let janela = GetForegroundWindow();

        if janela.is_null() || IsWindowVisible(janela) == 0 {
            return None;
        }

        let mut pid: u32 = 0;
        GetWindowThreadProcessId(janela, &mut pid);

        if pid == 0 {
            return None;
        }

        let mut retangulo: RECT = std::mem::zeroed();
        if GetWindowRect(janela, &mut retangulo) == 0 {
            return None;
        }

        let monitor = MonitorFromWindow(janela, MONITOR_DEFAULTTONEAREST);
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;

        if GetMonitorInfoW(monitor, &mut info) == 0 {
            return None;
        }

        let area = |r: &RECT| {
            let largura = (r.right - r.left).max(0) as f64;
            let altura = (r.bottom - r.top).max(0) as f64;
            largura * altura
        };

        // `rcMonitor`, não `rcWork`: a área de trabalho desconta a barra, e o jogo em tela cheia cobre a barra.
        let area_monitor = area(&info.rcMonitor);

        if area_monitor <= 0.0 {
            return None;
        }

        Some((pid, (area(&retangulo) / area_monitor).min(1.0)))
    }
}

#[cfg(not(target_os = "windows"))]
pub fn janela_em_primeiro_plano() -> Option<(u32, f64)> {
    None
}

/// Mais de um segundo pelo WMI: só depois dos sinais baratos (ver `procurar`).
#[cfg(target_os = "windows")]
pub fn uso_3d_do_processo(pid: u32) -> Option<f64> {
    // Sublinhado dos dois lados, senão o PID 123 casaria com 1234.
    let script = format!(
        "$i = Get-CimInstance Win32_PerfFormattedData_GPUPerformanceCounters_GPUEngine \
           -ErrorAction SilentlyContinue | \
           Where-Object {{ $_.Name -like 'pid_{}_*' -and $_.Name -like '*engtype_3D*' }}; \
         if ($null -eq $i) {{ '' }} else {{ \
           [math]::Min(100, ($i | Measure-Object -Property UtilizationPercentage -Sum).Sum) }}",
        pid
    );

    let saida = super::shell::powershell(&script).ok()?;

    if !saida.success {
        return None;
    }

    let texto = saida.stdout.trim();

    // Vazio é "o contador não existe", não zero: senão o detector nunca dispararia nessa máquina.
    if texto.is_empty() {
        return None;
    }

    texto.parse::<f64>().ok()
}

#[cfg(not(target_os = "windows"))]
pub fn uso_3d_do_processo(_pid: u32) -> Option<f64> {
    None
}

/// Janela e tempo de vida custam microssegundos; a GPU só é consultada para um único candidato que passou em tudo.
pub fn procurar() -> Option<JogoDetectado> {
    let (pid, cobertura) = janela_em_primeiro_plano()?;

    if cobertura < COBERTURA_MINIMA {
        return None;
    }

    let (executavel, caminho, segundos) = dados_do_processo(pid)?;

    if recusado_por(&executavel).is_some() || segundos < SEGUNDOS_MINIMOS {
        return None;
    }

    let observacao = Observacao {
        pid,
        executavel,
        caminho,
        cobertura: Some(cobertura),
        uso_3d: uso_3d_com_folga(pid),
        segundos_aberto: segundos,
    };

    let detectado = decidir(&observacao)?;

    if let Some(caminho) = &detectado.caminho {
        super::jogos::registrar_visto(&detectado.nome, caminho);
    }
    Some(detectado)
}

/// Sem isto, vídeo em tela cheia abriria um `powershell.exe` a cada seis segundos, para sempre.
const SEGUNDOS_DE_FOLGA: u64 = 60;

fn uso_3d_com_folga(pid: u32) -> Option<f64> {
    use std::sync::Mutex;
    use std::time::Instant;

    static ULTIMA: Mutex<Option<(u32, Instant, Option<f64>)>> = Mutex::new(None);

    let Ok(mut guarda) = ULTIMA.lock() else {
        return uso_3d_do_processo(pid);
    };

    if let Some((anterior, quando, valor)) = *guarda {
        // Só reaproveita "não é jogo" do MESMO processo: quem passou continua medido, para o modo desligar quando fechar.
        let ainda_vale = anterior == pid
            && quando.elapsed().as_secs() < SEGUNDOS_DE_FOLGA
            && valor.map(|v| v < USO_3D_MINIMO).unwrap_or(true);

        if ainda_vale {
            return valor;
        }
    }

    let medido = uso_3d_do_processo(pid);
    *guarda = Some((pid, Instant::now(), medido));
    medido
}

fn dados_do_processo(pid: u32) -> Option<(String, Option<PathBuf>, u64)> {
    use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

    let mut sistema = System::new();
    let alvo = Pid::from_u32(pid);

    sistema.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[alvo]),
        true,
        ProcessRefreshKind::nothing(),
    );

    let processo = sistema.process(alvo)?;
    let agora = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    Some((
        processo.name().to_string_lossy().to_string(),
        processo.exe().map(PathBuf::from),
        agora.saturating_sub(processo.start_time()),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jogo_tipico() -> Observacao {
        Observacao {
            pid: 4242,
            executavel: "palworld.exe".to_string(),
            caminho: Some(PathBuf::from(r"D:\SteamLibrary\steamapps\common\Palworld\Palworld.exe")),
            cobertura: Some(1.0),
            uso_3d: Some(78.0),
            segundos_aberto: 600,
        }
    }

    #[test]
    fn jogo_desconhecido_e_reconhecido_mesmo_assim() {
        let d = decidir(&jogo_tipico()).expect("é jogo");

        assert_eq!(d.nome, "Palworld");
        assert!(!d.conhecido);
        assert_eq!(d.pid, 4242);
    }

    #[test]
    fn jogo_conhecido_ganha_o_nome_bonito() {
        let mut obs = jogo_tipico();
        obs.executavel = "cs2.exe".to_string();

        let d = decidir(&obs).expect("é jogo");

        assert_eq!(d.nome, "Counter-Strike 2");
        assert!(d.conhecido);
        assert!(d.sinais.contains(&Sinal::NomeConhecido("Counter-Strike 2".to_string())));
    }

    #[test]
    fn navegador_em_tela_cheia_nao_e_jogo() {
        let mut obs = jogo_tipico();
        obs.executavel = "chrome.exe".to_string();
        obs.uso_3d = Some(3.0);

        assert!(decidir(&obs).is_none());
    }

    #[test]
    fn video_em_tela_cheia_nao_e_jogo() {
        let mut obs = jogo_tipico();
        obs.executavel = "vlc.exe".to_string();
        obs.uso_3d = Some(1.5);

        assert!(decidir(&obs).is_none());
    }

    #[test]
    fn programa_que_desenha_em_3d_mas_nao_e_jogo_e_recusado_pelo_nome() {
        // O OBS fecha todos os sinais medidos (3D pesado, projetor em tela cheia): só a lista resolve.
        for exe in ["obs64.exe", "blender.exe", "resolve.exe", "unrealeditor.exe"] {
            let mut obs = jogo_tipico();
            obs.executavel = exe.to_string();

            assert!(decidir(&obs).is_none(), "{} passou como jogo", exe);
            assert!(recusado_por(exe).is_some(), "{} sem motivo escrito", exe);
        }
    }

    #[test]
    fn contador_ausente_nao_e_zero() {
        let mut obs = jogo_tipico();
        obs.uso_3d = None;

        assert!(decidir(&obs).is_none());
    }

    #[test]
    fn janela_pequena_nao_e_jogo() {
        let mut obs = jogo_tipico();
        obs.cobertura = Some(0.55);

        assert!(decidir(&obs).is_none());
    }

    #[test]
    fn recem_aberto_ainda_nao_conta() {
        let mut obs = jogo_tipico();
        obs.segundos_aberto = 4;

        assert!(decidir(&obs).is_none());
    }

    #[test]
    fn a_decisao_mostra_o_que_a_sustentou() {
        let d = decidir(&jogo_tipico()).expect("é jogo");

        assert!(d.sinais.iter().any(|s| matches!(s, Sinal::JanelaCobrindoMonitor(_))));
        assert!(d.sinais.iter().any(|s| matches!(s, Sinal::MotorGrafico(_))));
        assert!(d.sinais.iter().any(|s| matches!(s, Sinal::AbertoHa(_))));
    }

    #[test]
    fn nenhum_recusado_esta_na_lista_de_jogos_conhecidos() {
        // Guarda contra contradição entre os dois catálogos do produto.
        for (exe, _) in NAO_E_JOGO {
            assert!(
                super::super::gamemode::nome_do_jogo(exe).is_none(),
                "{} está nas duas listas",
                exe
            );
        }
    }

    #[test]
    fn observa_esta_maquina() {
        match janela_em_primeiro_plano() {
            Some((pid, cobertura)) => {
                println!("  janela em primeiro plano: pid {} cobrindo {:.0}% do monitor", pid, cobertura * 100.0);
                assert!((0.0..=1.0).contains(&cobertura));
            }
            None => println!("  nenhuma janela em primeiro plano"),
        }

        match procurar() {
            Some(j) => println!("  jogo detectado: {} ({}) — {:?}", j.nome, j.executavel, j.sinais),
            None => println!("  nenhum jogo rodando agora"),
        }
    }
}
