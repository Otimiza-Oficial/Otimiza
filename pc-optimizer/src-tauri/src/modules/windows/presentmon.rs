// Medição de quadros pelo PresentMon (Intel, MIT, embutido no instalador). O canal antigo (`frames.rs`) só ouvia o
// início do Present: não sabia se o quadro chegou à tela, se era gerado, nem cobria Vulkan e OpenGL. Aqui cada
// quadro vem com o tempo até o próximo, o tempo na tela, o tipo (do jogo ou gerado), o modo de apresentação e quanto
// CPU e GPU trabalharam nele. Quadro gerado nunca entra no FPS do jogo.

use serde::{Deserialize, Serialize};

/// Com o nome da versão testada: outra versão pode mudar coluna, e a leitura procura pelo cabeçalho.
pub const VERSAO: &str = "2.6.0";

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "tipo", content = "qual")]
pub enum TipoDeQuadro {
    /// Desenhado pelo jogo.
    Jogo,
    /// Repetição do anterior (a tela atualizou sem quadro novo).
    Repetido,
    /// Gerado por tecnologia que se identifica ao PresentMon (XeFG, AFMF). DLSS FG ainda não se identifica: os
    /// quadros dele aparecem como do jogo, e a tela diz isso.
    Gerado(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Quadro {
    pub cadeia: String,
    pub tipo: TipoDeQuadro,
    pub modo: String,
    /// Relógio de alta precisão do Windows (`--qpc_time`): o mesmo dos eventos de disco, para cruzar os trancos.
    pub inicio_qpc: Option<i64>,
    pub intervalo_ms: Option<f64>,
    pub cpu_ocupada_ms: Option<f64>,
    pub gpu_ocupada_ms: Option<f64>,
    /// `None`: o quadro nunca chegou à tela (descartado).
    pub na_tela_ms: Option<f64>,
    pub ate_a_tela_ms: Option<f64>,
}

fn numero(campo: Option<&&str>) -> Option<f64> {
    campo.and_then(|c| c.trim().parse::<f64>().ok()).filter(|v| v.is_finite())
}

/// Lê o CSV `--v2_metrics`. Colunas pelo nome, nunca pela posição. Sem as colunas essenciais, `Err` (versão diferente
/// ou saída de erro), nunca uma lista vazia fingindo "zero quadros".
pub fn ler_csv(texto: &str) -> Result<Vec<Quadro>, String> {
    let mut linhas = texto.lines().filter(|l| !l.trim().is_empty());
    let cabecalho: Vec<&str> = linhas
        .next()
        .ok_or("o PresentMon não devolveu nada")?
        .split(',')
        .map(str::trim)
        .collect();
    let coluna = |nome: &str| cabecalho.iter().position(|c| *c == nome);

    let (Some(c_cadeia), Some(c_modo), Some(c_intervalo)) = (
        coluna("SwapChainAddress"),
        coluna("PresentMode"),
        coluna("FrameTime"),
    ) else {
        return Err(format!(
            "a saída do PresentMon não tem as colunas esperadas (versão testada: {}): {}",
            VERSAO,
            cabecalho.join(",").chars().take(200).collect::<String>()
        ));
    };
    let c_qpc = coluna("CPUStartQPC");
    let c_tipo = coluna("FrameType");
    let c_cpu = coluna("CPUBusy");
    let c_gpu = coluna("GPUBusy");
    let c_tela = coluna("DisplayedTime");
    let c_latencia = coluna("DisplayLatency");

    let mut quadros = Vec::new();
    for linha in linhas {
        let campos: Vec<&str> = linha.split(',').collect();
        if campos.len() < cabecalho.len() {
            continue;
        }
        let tipo = match c_tipo.and_then(|c| campos.get(c)).map(|t| t.trim()) {
            None | Some("Application") | Some("NA") | Some("NotSet") | Some("Unspecified") | Some("") => TipoDeQuadro::Jogo,
            Some("Repeated") => TipoDeQuadro::Repetido,
            Some(outro) => TipoDeQuadro::Gerado(outro.to_string()),
        };
        quadros.push(Quadro {
            cadeia: campos[c_cadeia].trim().to_string(),
            tipo,
            modo: campos[c_modo].trim().to_string(),
            inicio_qpc: c_qpc.and_then(|c| campos.get(c)).and_then(|v| v.trim().parse::<i64>().ok()),
            intervalo_ms: numero(campos.get(c_intervalo)).filter(|v| *v > 0.0),
            cpu_ocupada_ms: c_cpu.and_then(|c| numero(campos.get(c))),
            gpu_ocupada_ms: c_gpu.and_then(|c| numero(campos.get(c))),
            na_tela_ms: c_tela.and_then(|c| numero(campos.get(c))),
            ate_a_tela_ms: c_latencia.and_then(|c| numero(campos.get(c))),
        });
    }
    Ok(quadros)
}

/// Por onde o quadro chega à tela. A tela escolhe a frase por aqui, nunca comparando o texto do PresentMon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Apresentacao {
    /// "Hardware: Independent Flip", "Hardware: Legacy Flip", "Hardware Composed: Independent Flip": direto na tela.
    Direta,
    /// "Composed: Flip": o Windows compõe antes de mostrar.
    Composta,
    /// "Composed: Copy with GPU/CPU GDI": o Windows copia cada quadro, o caminho com mais atraso.
    Copiada,
    #[default]
    Desconhecida,
}

/// Pura.
pub fn classificar_modo(modo: &str) -> Apresentacao {
    if modo.starts_with("Hardware") && modo.contains("Flip") {
        Apresentacao::Direta
    } else if modo == "Composed: Flip" {
        Apresentacao::Composta
    } else if modo.starts_with("Composed: Copy") {
        Apresentacao::Copiada
    } else {
        Apresentacao::Desconhecida
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GargaloProvavel {
    /// A GPU trabalhou quase o quadro inteiro.
    Gpu,
    /// A CPU (a thread que monta o quadro) trabalhou quase o quadro inteiro.
    Cpu,
    /// Nenhum dos dois: limite de FPS, V-Sync, espera do jogo ou do disco.
    Espera,
    NaoDeuParaSaber,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Resumo {
    pub segundos: f64,
    /// Só os quadros que o jogo desenhou.
    pub quadros_do_jogo: usize,
    pub quadros_gerados: usize,
    /// Do jogo, que nunca chegaram à tela.
    pub quadros_descartados: usize,
    pub fps_do_jogo_medio: f64,
    pub fps_do_jogo_mediana: f64,
    /// Tudo o que chegou à tela por segundo, gerados incluídos. Sempre ao lado do FPS do jogo, nunca no lugar dele.
    pub fps_exibido: f64,
    /// Média do 1% pior, em FPS: a mesma definição de `frames::estatistica` (uma só no produto). `None` com
    /// menos de `fluidez::AMOSTRA_PARA_1PCT` quadros (mil).
    pub low_1pct: Option<f64>,
    /// Média do 0,1% pior. `None` com menos de `fluidez::AMOSTRA_PARA_01PCT` (dez mil).
    pub low_01pct: Option<f64>,
    pub quadro_medio_ms: f64,
    pub quadro_p95_ms: f64,
    pub quadro_p99_ms: f64,
    pub quadro_desvio_ms: f64,
    pub engasgos_por_minuto: f64,
    pub modo_de_apresentacao: String,
    /// `Desconhecida` em medição gravada antes deste campo.
    #[serde(default)]
    pub apresentacao: Apresentacao,
    pub cpu_ocupada_media_ms: Option<f64>,
    pub gpu_ocupada_media_ms: Option<f64>,
    pub gargalo: GargaloProvavel,
    pub ate_a_tela_media_ms: Option<f64>,
    /// Tecnologias de geração que se identificaram (ex.: "AMD AFMF").
    pub geradores: Vec<String>,
    /// Onde caíram os trancos, no relógio QPC, para o cruzamento com o disco. Fica fora da tela.
    #[serde(skip, default)]
    pub trancos_qpc: Vec<i64>,
}

/// Percentil por posição mais próxima em lista já ordenada.
fn percentil(ordenados: &[f64], p: f64) -> f64 {
    let i = ((p / 100.0) * ordenados.len() as f64).ceil() as usize;
    ordenados[i.clamp(1, ordenados.len()) - 1]
}

/// Média da fração pior (1% → 0.01) de uma lista ordenada, em FPS.
fn media_dos_piores(ordenados: &[f64], fracao: f64) -> f64 {
    let n = ((ordenados.len() as f64 * fracao) as usize).max(1);
    let piores = &ordenados[ordenados.len() - n..];
    1000.0 / (piores.iter().sum::<f64>() / n as f64)
}

fn media(v: &[f64]) -> Option<f64> {
    (!v.is_empty()).then(|| v.iter().sum::<f64>() / v.len() as f64)
}

/// Da cadeia de apresentação com mais quadros (o jogo; um lançador ou sobreposição tem a sua). Pura.
pub fn resumir(quadros: &[Quadro]) -> Result<Resumo, String> {
    let cadeia = cadeia_principal(quadros)
        .ok_or("nenhum quadro capturado: o jogo estava minimizado, pausado ou fechado")?;
    let da_cadeia: Vec<&Quadro> = quadros.iter().filter(|q| q.cadeia == cadeia).collect();

    let do_jogo: Vec<&Quadro> = da_cadeia.iter().copied().filter(|q| q.tipo == TipoDeQuadro::Jogo).collect();
    let mut intervalos: Vec<f64> = do_jogo.iter().filter_map(|q| q.intervalo_ms).collect();
    if intervalos.len() < 20 {
        return Err(format!(
            "só {} quadros do jogo capturados: pouco para medir. Meça com o jogo em partida, fora do menu.",
            intervalos.len()
        ));
    }
    intervalos.sort_by(|a, b| a.total_cmp(b));

    // Pela soma dos intervalos do jogo: não depende do formato do relógio da saída.
    let segundos = (intervalos.iter().sum::<f64>() / 1000.0).max(0.001);

    let quadro_medio = media(&intervalos).unwrap_or(0.0);
    let mediana = percentil(&intervalos, 50.0);
    let desvio = (intervalos.iter().map(|v| (v - quadro_medio).powi(2)).sum::<f64>() / intervalos.len() as f64).sqrt();
    let limite_de_engasgo = (mediana * 2.0).max(50.0);
    let engasgos = intervalos.iter().filter(|v| **v > limite_de_engasgo).count();
    let trancos_qpc: Vec<i64> = do_jogo
        .iter()
        .filter(|q| q.intervalo_ms.is_some_and(|v| v > limite_de_engasgo))
        .filter_map(|q| q.inicio_qpc)
        .collect();

    let exibidos = da_cadeia.iter().filter(|q| q.na_tela_ms.is_some()).count();
    let gerados: Vec<&&Quadro> = da_cadeia.iter().filter(|q| matches!(q.tipo, TipoDeQuadro::Gerado(_))).collect();
    let mut geradores: Vec<String> = gerados
        .iter()
        .filter_map(|q| match &q.tipo {
            TipoDeQuadro::Gerado(nome) => Some(nome.clone()),
            _ => None,
        })
        .collect();
    geradores.sort();
    geradores.dedup();

    let mut modos: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for q in &do_jogo {
        *modos.entry(q.modo.as_str()).or_default() += 1;
    }
    let modo = modos.into_iter().max_by_key(|(_, n)| *n).map(|(m, _)| m.to_string()).unwrap_or_default();

    let cpu = media(&do_jogo.iter().filter_map(|q| q.cpu_ocupada_ms).collect::<Vec<_>>());
    let gpu = media(&do_jogo.iter().filter_map(|q| q.gpu_ocupada_ms).collect::<Vec<_>>());
    let gargalo = match (cpu, gpu) {
        (_, Some(g)) if g >= 0.9 * quadro_medio => GargaloProvavel::Gpu,
        (Some(c), _) if c >= 0.9 * quadro_medio => GargaloProvavel::Cpu,
        (Some(_), Some(_)) => GargaloProvavel::Espera,
        _ => GargaloProvavel::NaoDeuParaSaber,
    };

    Ok(Resumo {
        segundos,
        quadros_do_jogo: do_jogo.len(),
        quadros_gerados: gerados.len(),
        quadros_descartados: do_jogo.iter().filter(|q| q.na_tela_ms.is_none()).count(),
        fps_do_jogo_medio: 1000.0 / quadro_medio,
        fps_do_jogo_mediana: 1000.0 / mediana,
        fps_exibido: exibidos as f64 / segundos,
        low_1pct: (intervalos.len() >= crate::core::fluidez::AMOSTRA_PARA_1PCT)
            .then(|| media_dos_piores(&intervalos, 0.01)),
        low_01pct: (intervalos.len() >= crate::core::fluidez::AMOSTRA_PARA_01PCT)
            .then(|| media_dos_piores(&intervalos, 0.001)),
        quadro_medio_ms: quadro_medio,
        quadro_p95_ms: percentil(&intervalos, 95.0),
        quadro_p99_ms: percentil(&intervalos, 99.0),
        quadro_desvio_ms: desvio,
        engasgos_por_minuto: engasgos as f64 / (segundos / 60.0),
        apresentacao: classificar_modo(&modo),
        modo_de_apresentacao: modo,
        cpu_ocupada_media_ms: cpu,
        gpu_ocupada_media_ms: gpu,
        gargalo,
        ate_a_tela_media_ms: media(&do_jogo.iter().filter_map(|q| q.ate_a_tela_ms).collect::<Vec<_>>()),
        geradores,
        trancos_qpc,
    })
}

/// Ao lado do Otimiza no instalado; na pasta `binaries` em desenvolvimento e nos testes.
#[cfg(target_os = "windows")]
pub fn executavel() -> Option<std::path::PathBuf> {
    let junto = std::env::current_exe().ok()?.parent()?.join("PresentMon.exe");
    if junto.is_file() {
        return Some(junto);
    }
    let fonte = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("binaries")
        .join("PresentMon-x86_64-pc-windows-msvc.exe");
    fonte.is_file().then_some(fonte)
}

/// No formato da medição antiga, para quem já consome `frames::MedicaoCrua` (medição automática, prova, portão):
/// os mesmos cálculos sobre os quadros DO JOGO que o PresentMon separou. O resumo novo vem junto.
#[cfg(target_os = "windows")]
pub fn medir_como_antes(pid: u32, nome: &str, segundos: u32) -> Result<(super::frames::MedicaoCrua, Resumo), String> {
    let quadros = capturar(pid, segundos)?;
    let resumo = resumir(&quadros)?;
    let cadeia = cadeia_principal(&quadros).unwrap_or_default();
    let intervalos: Vec<f64> = quadros
        .iter()
        .filter(|q| q.cadeia == cadeia && q.tipo == TipoDeQuadro::Jogo)
        .filter_map(|q| q.intervalo_ms)
        .collect();
    let crua = super::frames::montar(nome, pid, intervalos.len() as u64, resumo.segundos, intervalos, resumo.trancos_qpc.clone());
    Ok((crua, resumo))
}

fn cadeia_principal(quadros: &[Quadro]) -> Option<String> {
    let mut contagem: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for q in quadros {
        *contagem.entry(q.cadeia.as_str()).or_default() += 1;
    }
    contagem.into_iter().max_by_key(|(_, n)| *n).map(|(c, _)| c.to_string())
}

/// Os quadros crus, antes do resumo.
#[cfg(target_os = "windows")]
pub fn capturar(pid: u32, segundos: u32) -> Result<Vec<Quadro>, String> {
    let exe = executavel().ok_or("o PresentMon não está junto do Otimiza: reinstale o programa")?;
    let pid = pid.to_string();
    let tempo = segundos.to_string();
    let saida = super::shell::run_com_prazo(
        &exe.to_string_lossy(),
        &[
            "--process_id", &pid,
            "--timed", &tempo,
            "--terminate_after_timed",
            "--output_stdout",
            "--no_console_stats",
            "--v2_metrics",
            "--track_frame_type",
            "--qpc_time",
            // Nome próprio: não derruba uma captura do PresentMon que a pessoa tenha aberta.
            "--session_name", "Otimiza",
            "--stop_existing_session",
        ],
        std::time::Duration::from_secs(segundos as u64 + 20),
    )?;
    if !saida.success && saida.stdout.trim().is_empty() {
        return Err(format!("o PresentMon não conseguiu medir: {}", saida.stderr.trim()));
    }
    ler_csv(&saida.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Cabeçalho real do PresentMon 2.6.0 com `--track_frame_type --qpc_time` (captura do dwm.exe nesta máquina).
    const CABECALHO: &str = "Application,ProcessID,SwapChainAddress,PresentRuntime,SyncInterval,PresentFlags,AllowsTearing,PresentMode,FrameType,CPUStartQPC,FrameTime,CPUBusy,CPUWait,GPULatency,GPUTime,GPUBusy,GPUWait,DisplayLatency,DisplayedTime,AnimationError,AnimationTime,MsFlipDelay,AllInputToPhotonLatency,ClickToPhotonLatency";

    fn linha(tipo: &str, inicio: f64, intervalo: f64, cpu: f64, gpu: f64, na_tela: Option<f64>) -> String {
        let qpc = (inicio * 10_000.0) as i64;
        format!(
            "jogo.exe,100,0xAAA,DXGI,0,512,1,Hardware: Independent Flip,{tipo},{qpc},{intervalo:.4},{cpu:.4},0.1,0.5,{gpu:.4},{gpu:.4},0.1,12.0,{},NA,{inicio:.4},NA,NA,NA",
            na_tela.map(|v| format!("{v:.4}")).unwrap_or_else(|| "NA".into())
        )
    }

    fn csv(linhas: &[String]) -> String {
        std::iter::once(CABECALHO.to_string()).chain(linhas.iter().cloned()).collect::<Vec<_>>().join("\n")
    }

    #[test]
    fn le_a_saida_real_do_presentmon_2_6() {
        let sem_qpc = CABECALHO.replace("CPUStartQPC", "CPUStartTime");
        let real = format!(
            "{sem_qpc}\n{}\n{}",
            "dwm.exe,1060,0x157C882C8F0,DXGI,1,0,0,Hardware: Legacy Flip,Application,446.4983,133.0706,132.8957,0.1749,0.0000,133.0797,0.0717,133.0080,138.1134,272.2096,NA,446.4983,NA,NA,NA",
            "dwm.exe,1060,0x157C882C8F0,DXGI,1,0,0,Hardware: Legacy Flip,Application,579.5689,272.2641,272.0841,0.1800,0.0000,272.2615,0.1196,272.1419,277.2524,272.2301,-139.1390,579.5689,NA,NA,NA"
        );
        let q = ler_csv(&real).unwrap();
        assert_eq!(q.len(), 2);
        assert_eq!(q[0].tipo, TipoDeQuadro::Jogo);
        assert_eq!(q[0].modo, "Hardware: Legacy Flip");
        assert_eq!(q[1].intervalo_ms, Some(272.2641));
        assert_eq!(q[0].na_tela_ms, Some(272.2096));
    }

    #[test]
    fn o_modo_de_apresentacao_vira_tipo_fixo() {
        assert_eq!(classificar_modo("Hardware: Independent Flip"), Apresentacao::Direta);
        assert_eq!(classificar_modo("Hardware: Legacy Flip"), Apresentacao::Direta);
        assert_eq!(classificar_modo("Hardware Composed: Independent Flip"), Apresentacao::Direta);
        assert_eq!(classificar_modo("Composed: Flip"), Apresentacao::Composta);
        assert_eq!(classificar_modo("Composed: Copy with GPU GDI"), Apresentacao::Copiada);
        assert_eq!(classificar_modo("Composed: Copy with CPU GDI"), Apresentacao::Copiada);
        assert_eq!(classificar_modo("Other"), Apresentacao::Desconhecida);
    }

    #[test]
    fn saida_sem_as_colunas_e_erro_e_nao_zero_quadros() {
        assert!(ler_csv("Erro: precisa de administrador").is_err());
        assert!(ler_csv("").is_err());
    }

    #[test]
    fn quadro_gerado_nao_entra_no_fps_do_jogo() {
        // 60 FPS do jogo, com um quadro AFMF entre cada dois: 120 na tela.
        let mut linhas = Vec::new();
        for i in 0..600 {
            let t = i as f64 * 16.6667;
            linhas.push(linha("Application", t, 16.6667, 6.0, 15.5, Some(8.33)));
            linhas.push(linha("AMD AFMF", t + 8.33, 16.6667, 0.0, 0.0, Some(8.33)));
        }
        let r = resumir(&ler_csv(&csv(&linhas)).unwrap()).unwrap();
        assert_eq!(r.quadros_do_jogo, 600);
        assert_eq!(r.quadros_gerados, 600);
        assert!((r.fps_do_jogo_medio - 60.0).abs() < 0.5, "FPS do jogo: {}", r.fps_do_jogo_medio);
        assert!((r.fps_exibido - 120.0).abs() < 1.5, "exibido: {}", r.fps_exibido);
        assert_eq!(r.geradores, vec!["AMD AFMF".to_string()]);
        assert_eq!(r.gargalo, GargaloProvavel::Gpu);
    }

    #[test]
    fn o_1pct_pior_e_os_engasgos_saem_do_ritmo() {
        // 1000 quadros a 10 ms e 20 travadas de 80 ms (2% do total: o percentil 99 cai numa travada).
        let mut linhas: Vec<String> = (0..1000).map(|i| linha("Application", i as f64 * 10.0, 10.0, 9.5, 4.0, Some(10.0))).collect();
        for k in 0..20 {
            linhas.push(linha("Application", 10_000.0 + k as f64 * 80.0, 80.0, 79.0, 4.0, Some(80.0)));
        }
        let r = resumir(&ler_csv(&csv(&linhas)).unwrap()).unwrap();
        assert!((r.fps_do_jogo_mediana - 100.0).abs() < 0.01);
        assert!((r.low_1pct.unwrap() - 12.5).abs() < 0.01, "1% low: {:?}", r.low_1pct);
        assert_eq!(r.low_01pct, None, "1020 quadros não bastam para o 0,1% pior (regra: dez mil)");
        assert!(r.engasgos_por_minuto > 0.0);
        assert_eq!(r.trancos_qpc.len(), 20, "cada travada leva o seu instante para o cruzamento com o disco");
        assert_eq!(r.gargalo, GargaloProvavel::Cpu);
    }

    #[test]
    fn quadro_que_nao_chegou_a_tela_conta_como_descartado() {
        let mut linhas: Vec<String> = (0..50).map(|i| linha("Application", i as f64 * 5.0, 5.0, 1.0, 1.0, Some(16.7))).collect();
        linhas.extend((50..60).map(|i| linha("Application", i as f64 * 5.0, 5.0, 1.0, 1.0, None)));
        let r = resumir(&ler_csv(&csv(&linhas)).unwrap()).unwrap();
        assert_eq!(r.quadros_descartados, 10);
        assert_eq!(r.low_1pct, None, "60 quadros não bastam para o 1% pior");
        assert_eq!(r.gargalo, GargaloProvavel::Espera);
    }

    #[test]
    fn a_cadeia_do_jogo_ganha_da_sobreposicao() {
        let mut linhas: Vec<String> = (0..200).map(|i| linha("Application", i as f64 * 8.0, 8.0, 3.0, 7.5, Some(8.0))).collect();
        linhas.extend((0..30).map(|i| linha("Application", i as f64 * 33.0, 33.0, 1.0, 1.0, Some(33.0)).replace("0xAAA", "0xBBB")));
        let r = resumir(&ler_csv(&csv(&linhas)).unwrap()).unwrap();
        assert_eq!(r.quadros_do_jogo, 200);
    }

    #[test]
    fn poucos_quadros_e_recusa_e_nao_numero() {
        let linhas: Vec<String> = (0..5).map(|i| linha("Application", i as f64 * 10.0, 10.0, 1.0, 1.0, Some(10.0))).collect();
        assert!(resumir(&ler_csv(&csv(&linhas)).unwrap()).is_err());
    }

    /// Mede o dwm.exe de verdade: prova que o executável embutido roda e que a leitura casa com a saída real.
    #[cfg(target_os = "windows")]
    #[test]
    #[ignore = "precisa de administrador e lê esta máquina"]
    fn mede_o_compositor_do_windows() {
        let pid = super::super::frames::encontrar_processo("dwm").expect("o dwm sempre roda").0;
        // A área de trabalho parada apresenta pouco: aqui vale provar que o executável roda e a leitura casa.
        let q = capturar(pid, 3).expect("o PresentMon roda e a saída é lida");
        println!("{} quadros do dwm em 3 s; primeiro: {:?}", q.len(), q.first());
        assert!(!q.is_empty());
    }
}
