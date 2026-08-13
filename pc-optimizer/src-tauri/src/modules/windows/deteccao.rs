// Reconhecer um jogo sem lista de nomes
//
// POR QUE A LISTA NÃO SERVE
//
// Até a versão 0.13 o Otimiza reconhecia jogo comparando o nome do processo com
// uma lista de cinco entradas, três delas da família GTA. Todo jogo fora dela —
// Fortnite, Minecraft, LoL, qualquer lançamento novo — era invisível: o modo
// jogo nunca ligava, o segundo plano nunca era pausado, o plano de energia
// nunca entrava.
//
// Uma lista maior adia o problema em vez de resolver. Jogo novo sai toda
// semana, e o cliente que comprou o produto não vai esperar a próxima versão
// para o programa perceber que ele está jogando.
//
// OS QUATRO SINAIS, E POR QUE OS QUATRO JUNTOS
//
// A tentação é dar pontos e somar. Pontuação é exatamente o que faz detector
// genérico errar, porque deixa dois sinais fracos valerem por um forte. Aqui os
// quatro precisam valer AO MESMO TEMPO:
//
//   1. A janela em primeiro plano cobre o monitor
//   2. O MESMO processo está consumindo o motor 3D da placa
//   3. Está aberto há tempo suficiente
//   4. Não está na lista de recusa
//
// O par 1+2 é o que carrega o peso, e é sutil: num navegador, quem desenha é um
// processo auxiliar (`--type=gpu-process`) que NÃO tem janela; o processo que
// tem a janela consome praticamente nada de 3D. A conjunção nunca fecha. Isso
// cobre Chrome, Edge, Discord, Spotify, Teams — tudo que é feito em Electron.
//
// Vídeo em tela cheia também não fecha: reprodução consome o motor de DECODE,
// não o 3D, e o filtro é por `engtype_3D`.
//
// O QUE ESTA DETECÇÃO NÃO AUTORIZA
//
// Ela decide se o modo jogo liga. Ela NÃO autoriza escrever em IFEO — para
// isso, além de ser jogo, o executável precisa estar dentro de uma biblioteca
// de jogo de verdade (ver `jogos::dentro_de_biblioteca`). Detector heurístico
// não pode virar autoridade de segurança.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Quanto da área do monitor a janela precisa cobrir.
///
/// Não é 100% porque a janela de um jogo em tela cheia sem bordas costuma
/// medir alguns pixels a mais ou a menos que o monitor, e porque a barra de
/// tarefas às vezes entra na conta.
const COBERTURA_MINIMA: f64 = 0.90;

/// Uso do motor 3D que separa jogo de programa que só desenha a interface.
const USO_3D_MINIMO: f64 = 20.0;

/// Programa aberto há menos que isto ainda está carregando, e a medição de GPU
/// no primeiro instante não vale nada.
const SEGUNDOS_MINIMOS: u64 = 20;

/// Programas que fecham os quatro sinais e NÃO são jogo.
///
/// Cada entrada tem o motivo escrito, e o motivo é sempre o mesmo tipo de
/// coisa: são programas que legitimamente desenham em 3D e ocupam a tela
/// inteira. Ligar o modo jogo porque alguém abriu o OBS para gravar, ou o
/// editor de vídeo para trabalhar, seria mexer na energia da máquina pelo
/// motivo errado.
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

/// O motivo pelo qual o programa foi recusado, se foi.
pub fn recusado_por(executavel: &str) -> Option<&'static str> {
    let nome = executavel.trim().to_lowercase();

    NAO_E_JOGO
        .iter()
        .find(|(exe, _)| nome == *exe)
        .map(|(_, motivo)| *motivo)
}

/// O que sustentou a decisão. Vai para a tela: o cliente merece saber por que o
/// programa resolveu que aquilo era um jogo.
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
    /// Nome bonito quando o jogo é conhecido; o nome do executável quando não.
    pub nome: String,
    /// Verdadeiro quando o nome veio do catálogo, e não do arquivo.
    pub conhecido: bool,
    pub sinais: Vec<Sinal>,
}

/// O que foi observado de um processo, pronto para a decisão.
///
/// Existe separado da leitura para a regra poder ser testada sem abrir jogo
/// nenhum — que é o único jeito de testar isto numa esteira de integração.
#[derive(Debug, Clone)]
pub struct Observacao {
    pub pid: u32,
    pub executavel: String,
    pub caminho: Option<PathBuf>,
    /// Fração do monitor coberta pela janela em primeiro plano, de 0 a 1.
    /// `None` quando este processo não é o dono da janela em primeiro plano.
    pub cobertura: Option<f64>,
    /// Uso do motor 3D, em porcentagem. `None` quando não foi possível medir —
    /// e isso é diferente de zero.
    pub uso_3d: Option<f64>,
    pub segundos_aberto: u64,
}

/// A regra. **Função pura.**
pub fn decidir(obs: &Observacao) -> Option<JogoDetectado> {
    if recusado_por(&obs.executavel).is_some() {
        return None;
    }

    // Contador ausente NÃO é zero. Se a máquina não expõe o contador de GPU,
    // o produto não sabe — e "não sei" nunca pode virar "não é jogo" nem "é".
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

/// Transforma `r5apex.exe` em `R5apex` para um jogo que não conhecemos.
///
/// Não é bonito, e é de propósito: inventar um nome comercial a partir do
/// arquivo produziria coisas erradas. O nome do executável é um fato.
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

// ------------------------------------------------------------------- leitura

/// O processo dono da janela em primeiro plano, e quanto do monitor ela cobre.
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

        // `rcMonitor` e não `rcWork`: a área de trabalho desconta a barra de
        // tarefas, e um jogo em tela cheia cobre a barra também.
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

/// Uso do motor 3D por processo.
///
/// Custa uma chamada ao WMI, que passa de um segundo. Por isso quem chama só
/// deve pedir DEPOIS que os sinais baratos passaram — ver `procurar`.
#[cfg(target_os = "windows")]
pub fn uso_3d_do_processo(pid: u32) -> Option<f64> {
    // As instâncias do contador têm a forma `pid_1234_luid_..._engtype_3D`. O
    // filtro por PID precisa do sublinhado dos dois lados, senão o PID 123
    // casaria com 1234.
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

    // Vazio significa "o contador não existe nesta máquina", e isso NÃO é
    // zero. Devolver zero aqui faria o detector nunca disparar numa máquina
    // sem os contadores, e a tela mentiria dizendo que nenhum jogo abriu.
    if texto.is_empty() {
        return None;
    }

    texto.parse::<f64>().ok()
}

#[cfg(not(target_os = "windows"))]
pub fn uso_3d_do_processo(_pid: u32) -> Option<f64> {
    None
}

/// Procura um jogo rodando agora.
///
/// A ordem das verificações é a economia do módulo: janela e tempo de vida são
/// leituras de memória, e custam microssegundos. O uso de GPU custa mais de um
/// segundo. Então a GPU só é consultada quando um único candidato já passou por
/// tudo mais — no caso comum, em que ninguém está em tela cheia, esta função
/// não gasta nada.
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

    decidir(&observacao)
}

/// Quanto tempo um "não é jogo" continua valendo para o mesmo processo.
///
/// Sem isto, alguém assistindo a um vídeo em tela cheia faria o Otimiza abrir
/// um `powershell.exe` a cada seis segundos, para sempre — o otimizador virando
/// o programa que mais pesa na máquina. Uma vez por minuto responde igual,
/// porque programa que não é jogo não vira jogo no minuto seguinte.
const SEGUNDOS_DE_FOLGA: u64 = 60;

/// Consulta a GPU, mas no máximo uma vez por minuto para o mesmo processo.
fn uso_3d_com_folga(pid: u32) -> Option<f64> {
    use std::sync::Mutex;
    use std::time::Instant;

    static ULTIMA: Mutex<Option<(u32, Instant, Option<f64>)>> = Mutex::new(None);

    let Ok(mut guarda) = ULTIMA.lock() else {
        return uso_3d_do_processo(pid);
    };

    if let Some((anterior, quando, valor)) = *guarda {
        // Só reaproveita quando o processo é o MESMO e a resposta foi "não é
        // jogo". Um processo que passou no teste precisa continuar sendo
        // medido: se o jogo fechar, o modo tem que desligar sem esperar.
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

/// Nome, caminho e há quanto tempo o processo está aberto.
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
        // O ponto inteiro do módulo: um jogo que ninguém cadastrou continua
        // sendo reconhecido, e ganha o nome do executável em vez de um nome
        // comercial inventado.
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
        // O caso que derruba todo detector ingênuo. Num navegador quem desenha
        // é um processo auxiliar sem janela; o processo que tem a janela
        // consome quase nada de 3D. Vale para tudo feito em Electron —
        // Discord, Spotify, Teams.
        let mut obs = jogo_tipico();
        obs.executavel = "chrome.exe".to_string();
        obs.uso_3d = Some(3.0);

        assert!(decidir(&obs).is_none());
    }

    #[test]
    fn video_em_tela_cheia_nao_e_jogo() {
        // Reprodução de vídeo consome o motor de decodificação, não o 3D. O
        // filtro por `engtype_3D` já exclui, e aqui isso aparece como uso
        // baixo do motor que interessa.
        let mut obs = jogo_tipico();
        obs.executavel = "vlc.exe".to_string();
        obs.uso_3d = Some(1.5);

        assert!(decidir(&obs).is_none());
    }

    #[test]
    fn programa_que_desenha_em_3d_mas_nao_e_jogo_e_recusado_pelo_nome() {
        // O OBS é o pior caso: usa 3D pesado, apresenta continuamente, e o
        // projetor em tela cheia fecha todos os sinais medidos. Só a lista
        // resolve — e recusar é o certo: ligar o modo jogo porque alguém
        // começou a gravar seria mexer na máquina pelo motivo errado.
        for exe in ["obs64.exe", "blender.exe", "resolve.exe", "unrealeditor.exe"] {
            let mut obs = jogo_tipico();
            obs.executavel = exe.to_string();

            assert!(decidir(&obs).is_none(), "{} passou como jogo", exe);
            assert!(recusado_por(exe).is_some(), "{} sem motivo escrito", exe);
        }
    }

    #[test]
    fn contador_ausente_nao_e_zero() {
        // Se a máquina não expõe o contador de GPU, o produto NÃO SABE. Tratar
        // isso como zero faria o detector nunca disparar e a tela afirmaria
        // que nenhum jogo está aberto — uma afirmação que ninguém mediu.
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
        // Nos primeiros segundos o jogo está carregando e a medição de GPU não
        // vale nada — pode estar em 100% compilando shader, ou em 0% lendo
        // disco.
        let mut obs = jogo_tipico();
        obs.segundos_aberto = 4;

        assert!(decidir(&obs).is_none());
    }

    #[test]
    fn a_decisao_mostra_o_que_a_sustentou() {
        // O cliente merece saber por que o programa resolveu que aquilo era um
        // jogo — ainda mais porque essa decisão muda o plano de energia da
        // máquina dele.
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
