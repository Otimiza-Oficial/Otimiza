// Texto curto que o cliente cola no Discord do atendimento, no lugar de um AnyDesk (nasceu de um cliente com
// "os programas não abrem mais"). Regras: cabe numa mensagem; nada que identifique a PESSOA (descreve uma
// máquina); a lacuna aparece (não conseguir medir não é estar bem).

use super::{display, health, memory, shell, thermal};
use serde::Deserialize;

/// O limite do Discord é 2000; 1900 deixa espaço para a frase que introduz o relatório.
pub const LIMITE_DE_CARACTERES: usize = 1900;

/// Nenhum campo lê o sistema: as três regras ficam testáveis sem hardware.
pub struct Entrada {
    pub versao: String,
    pub windows: String,
    pub ram_gb: u32,
    pub monitores: usize,
    pub mudancas_aplicadas: usize,
    /// Sem a lista, "apliquei tudo e o FPS caiu" (2.1.0) não se diagnostica à distância. Por identificador: curto e
    /// não muda com o texto da tela.
    pub aplicadas: Vec<String>,
    pub disco: String,
    pub termico: String,
    /// Vazio quando tudo foi lido; nunca omitido.
    pub lacunas: Vec<String>,
    pub abertura_ms: Option<u64>,
    /// A3.2: sessões seguidas que terminaram sem fechar normalmente, e se o modo seguro está ligado.
    pub quedas_seguidas: u32,
    pub modo_seguro: bool,
    /// Relatórios de pânico em `falhas\` e a primeira linha útil do mais recente.
    pub falhas: usize,
    pub ultima_falha: Option<String>,
    /// "10 crashes, 3× bacon-vegan-carbon": o jogo, pelos relatórios do FiveM.
    pub crashes_do_jogo: Option<String>,
    /// A última partida medida: "FiveM 70,9 FPS, limita o processador".
    pub partida: Option<String>,
    /// As últimas linhas de aviso e erro do log, já sem caminho pessoal.
    pub ultimos_passos: Vec<String>,
}

/// Pura: tira do texto o que identifica a pessoa (a pasta do usuário e o nome dele), deixando o resto.
pub fn anonimizar(linha: &str, pasta_do_usuario: &str, usuario: &str) -> String {
    let mut saida = linha.to_string();
    if !pasta_do_usuario.is_empty() {
        saida = saida.replace(pasta_do_usuario, "%USERPROFILE%");
        saida = saida.replace(&pasta_do_usuario.replace('\\', "/"), "%USERPROFILE%");
    }
    if usuario.len() >= 3 {
        saida = saida.replace(usuario, "<usuário>");
    }
    sem_sid(&saida)
}

/// Pura: o identificador da conta do Windows (S-1-5-21-…) aparece em nome de tarefa agendada e identifica a conta.
pub fn sem_sid(texto: &str) -> String {
    let mut saida = String::with_capacity(texto.len());
    let mut resto = texto;
    while let Some(i) = resto.find("S-1-5-21-") {
        saida.push_str(&resto[..i]);
        let depois = &resto[i..];
        let fim = depois
            .char_indices()
            .skip(1)
            .find(|(_, ch)| !(ch.is_ascii_digit() || *ch == '-'))
            .map(|(j, _)| j)
            .unwrap_or(depois.len());
        saida.push_str("<SID>");
        resto = &depois[fim..];
    }
    saida.push_str(resto);
    saida
}

/// Pura: das linhas do log, as de aviso, erro e pânico, as `n` mais recentes, sem o carimbo de data longo.
pub fn passos_do_log(log: &str, n: usize) -> Vec<String> {
    let relevantes: Vec<&str> = log
        .lines()
        .filter(|l| l.contains("[WARN]") || l.contains("[ERROR]") || l.contains("PÂNICO"))
        .collect();
    relevantes
        .iter()
        .rev()
        .take(n)
        .rev()
        .map(|l| {
            // "[2026-09-26 13:17:23.761] [WARN] texto" → "09-26 13:17 WARN texto", e no máximo 160 caracteres.
            let curta = match (l.get(6..17), l.find("] [").and_then(|i| l.get(i + 3..))) {
                (Some(quando), Some(resto)) => format!("{} {}", quando, resto.replacen(']', "", 1)),
                _ => l.to_string(),
            };
            curta.chars().take(160).collect()
        })
        .collect()
}

/// Pura: só formata. O que é crítico e o que não se leu é decidido nos `resumir_*`.
pub fn montar(entrada: &Entrada) -> String {
    let mut linhas = Vec::new();

    linhas.push(format!("Otimiza {}", entrada.versao));
    linhas.push(format!(
        "{} · {} GB · {} monitor{}",
        entrada.windows,
        entrada.ram_gb,
        entrada.monitores,
        if entrada.monitores == 1 { "" } else { "es" }
    ));

    linhas.push(format!("Mudanças aplicadas: {}", entrada.mudancas_aplicadas));
    linhas.push(format!("Disco: {} · Térmico: {}", entrada.disco, entrada.termico));
    linhas.push(match entrada.abertura_ms {
        Some(ms) => format!("Abertura: {} ms", ms),
        None => "Abertura: não medida".to_string(),
    });
    if entrada.quedas_seguidas > 0 || entrada.modo_seguro {
        linhas.push(format!(
            "Quedas seguidas: {}{}",
            entrada.quedas_seguidas,
            if entrada.modo_seguro { " · MODO SEGURO ligado" } else { "" }
        ));
    }
    if entrada.falhas > 0 {
        linhas.push(format!(
            "Falhas do Otimiza gravadas: {}{}",
            entrada.falhas,
            entrada.ultima_falha.as_deref().map(|f| format!(" · última: {}", f)).unwrap_or_default()
        ));
    }
    if let Some(p) = &entrada.partida {
        linhas.push(format!("Última partida: {}", p));
    }
    if let Some(c) = &entrada.crashes_do_jogo {
        linhas.push(format!("Crashes do jogo: {}", c));
    }

    // A linha só some quando `lacunas` está de fato vazia.
    if !entrada.lacunas.is_empty() {
        linhas.push(format!("Não consegui ler: {}", entrada.lacunas.join(", ")));
    }

    if !entrada.ultimos_passos.is_empty() {
        linhas.push("Últimos avisos do log:".to_string());
        linhas.extend(entrada.ultimos_passos.iter().map(|p| format!("  {}", p)));
    }

    // Por último: se estourar, o corte come os identificadores (que se pedem de novo), não as lacunas.
    if !entrada.aplicadas.is_empty() {
        linhas.push(format!("Aplicadas: {}", entrada.aplicadas.join(" ")));
    }

    cortar_no_limite(linhas.join("\n"))
}

/// Corta com `…` em vez de deixar o Discord recusar a mensagem calado.
fn cortar_no_limite(texto: String) -> String {
    if texto.len() <= LIMITE_DE_CARACTERES {
        return texto;
    }

    // `LIMITE_DE_CARACTERES` é em bytes; reserva os 3 bytes do "…" e recua até a fronteira de caractere.
    let reserva = '…'.len_utf8();
    let mut fim = LIMITE_DE_CARACTERES.saturating_sub(reserva);
    while fim > 0 && !texto.is_char_boundary(fim) {
        fim -= 1;
    }

    format!("{}…", &texto[..fim])
}

fn rank(severidade: health::FindingSeverity) -> u8 {
    match severidade {
        health::FindingSeverity::Ok => 0,
        health::FindingSeverity::Important => 1,
        health::FindingSeverity::Critical => 2,
    }
}

/// Pelo `id` e pela `severity`, nunca por texto. Só achado com prefixo `disk_`: `health::analyze()` devolve
/// também a bateria, e bateria a 55% fazia "Disco: crítico" com SSD perfeito. Nenhum achado de disco é "não
/// consegui ler", nunca "saudável".
fn resumir_disco(relatorio: &health::HealthReport) -> (String, Vec<String>) {
    let mut lacunas = Vec::new();
    let mut pior: Option<health::FindingSeverity> = None;
    let mut leu_algum_disco = false;

    for achado in &relatorio.findings {
        if !achado.id.starts_with("disk_") {
            continue;
        }

        if achado.id.starts_with("disk_errors_naosei") {
            lacunas.push("contador de erros do disco".to_string());
            continue;
        }

        leu_algum_disco = true;

        if achado.severity != health::FindingSeverity::Ok {
            pior = Some(match pior {
                Some(atual) if rank(atual) >= rank(achado.severity) => atual,
                _ => achado.severity,
            });
        }
    }

    // `needs_admin` é mais largo que `naosei`: nenhum contador de nenhum disco foi lido.
    if relatorio.needs_admin {
        lacunas.push("leitura completa de disco (sem administrador)".to_string());
    }

    let resumo = match pior {
        Some(health::FindingSeverity::Critical) => "crítico",
        Some(health::FindingSeverity::Important) => "atenção",
        _ if leu_algum_disco => "saudável",
        _ => "não consegui ler",
    };

    if !leu_algum_disco {
        lacunas.push("saúde do disco (o Windows não expôs nada nesta máquina)".to_string());
    }

    (resumo.to_string(), lacunas)
}

/// Por `ThermalReport::medido`, não pelo `summary`.
fn resumir_termico(relatorio: &thermal::ThermalReport) -> (String, Vec<String>) {
    if !relatorio.medido {
        return (
            "não sei".to_string(),
            vec!["limite do processador".to_string()],
        );
    }

    let resumo = match relatorio.culprit {
        thermal::Culprit::Nenhum => "sem limite ativo",
        thermal::Culprit::Bateria => "limitado (bateria)",
        thermal::Culprit::PlanoDeEnergia => "limitado (plano de energia)",
        thermal::Culprit::Calor => "limitado (calor)",
        thermal::Culprit::LimiteEletrico => "limitado (elétrico)",
        thermal::Culprit::NaoIdentificado => "limitado (causa não identificada)",
    };

    let lacunas = if relatorio.eventos_lidos { Vec::new() } else { vec!["eventos térmicos do Windows".to_string()] };
    (resumo.to_string(), lacunas)
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
struct RawWindows {
    caption: Option<String>,
    build_number: Option<String>,
}

/// `None` vira lacuna, nunca texto genérico.
fn ler_versao_do_windows() -> (String, Vec<String>) {
    let script = "$os = Get-CimInstance Win32_OperatingSystem; \
                  ConvertTo-Json -Compress -InputObject ([ordered]@{ \
                    Caption = $os.Caption; BuildNumber = $os.BuildNumber })";

    let bruto: RawWindows = shell::powershell(script)
        .ok()
        .filter(|saida| saida.success && !saida.stdout.trim().is_empty())
        .and_then(|saida| serde_json::from_str(saida.stdout.trim()).ok())
        .unwrap_or_default();

    match (bruto.caption, bruto.build_number) {
        (Some(caption), Some(build)) if !caption.trim().is_empty() && !build.trim().is_empty() => {
            let nome = caption.trim().replace("Microsoft ", "");
            (format!("{} {}", nome, build.trim()), Vec::new())
        }
        _ => (
            "Windows (versão não lida)".to_string(),
            vec!["versão do Windows".to_string()],
        ),
    }
}

/// A única função que toca o sistema.
pub fn gerar() -> Entrada {
    let versao = env!("CARGO_PKG_VERSION").to_string();
    let (windows, mut lacunas) = ler_versao_do_windows();

    let ram_gb = memory::analyze().total_ram_gb.round().max(0.0) as u32;
    let monitores = display::monitores().len();

    let historico = crate::modules::changelog::ChangeLog::load();
    let mudancas_aplicadas = historico.applied().len();
    let aplicadas: Vec<String> = historico
        .applied()
        .iter()
        .map(|a| sem_sid(&a.optimization_id))
        .collect();

    let (disco, lacunas_disco) = resumir_disco(&health::analyze());
    lacunas.extend(lacunas_disco);

    let (termico, lacunas_termico) = resumir_termico(&thermal::analyze());
    lacunas.extend(lacunas_termico);

    let (falhas, ultima_falha) = ler_falhas();
    let pasta_do_usuario = std::env::var("USERPROFILE").unwrap_or_default();
    let usuario = std::env::var("USERNAME").unwrap_or_default();
    let ultimos_passos = crate::utils::Logger::caminho()
        .and_then(|c| std::fs::read(c).ok())
        .map(|b| passos_do_log(&String::from_utf8_lossy(&b), 8))
        .unwrap_or_default()
        .into_iter()
        .map(|l| anonimizar(&l, &pasta_do_usuario, &usuario))
        .collect();

    let mudancas: Vec<(u64, String)> = historico.applied().iter().map(|a| (a.timestamp, a.optimization_id.clone())).collect();
    let jogo = super::crashes::relatorio(&mudancas);
    let crashes_do_jogo = (!jogo.sem_fivem && !jogo.crashes.is_empty()).then(|| {
        let mut texto = format!("{} no FiveM", jogo.crashes.len());
        let mut por_hash: std::collections::BTreeMap<&str, usize> = Default::default();
        for h in jogo.crashes.iter().filter_map(|c| c.hash.as_deref()) {
            *por_hash.entry(h).or_default() += 1;
        }
        if let Some((h, n)) = por_hash.iter().max_by_key(|(_, n)| **n) {
            texto.push_str(&format!(", {}× {}", n, h));
        }
        texto
    });

    let partida = crate::modules::medicoes::ler().ok().and_then(|m| {
        let ultima = m.iter().filter(|x| x.presentmon.is_some()).max_by_key(|x| x.quando)?;
        let r = ultima.presentmon.as_ref()?;
        Some(format!("{} {:.1} FPS do jogo, gargalo {:?}, modo {}", ultima.jogo, r.fps_do_jogo_medio, r.gargalo, r.modo_de_apresentacao))
    });

    Entrada {
        versao,
        windows,
        ram_gb,
        monitores,
        mudancas_aplicadas,
        aplicadas,
        disco,
        termico,
        lacunas,
        abertura_ms: crate::modules::abertura::medida(),
        quedas_seguidas: crate::utils::diagnostico::quedas_seguidas(),
        modo_seguro: crate::utils::diagnostico::modo_seguro(),
        falhas,
        ultima_falha,
        crashes_do_jogo,
        partida,
        ultimos_passos,
    }
}

/// Quantos relatórios de pânico há em `%APPDATA%\pc-optimizer\falhas` e a mensagem do mais recente.
fn ler_falhas() -> (usize, Option<String>) {
    let Ok(pasta) = std::env::var("APPDATA").map(|a| std::path::PathBuf::from(a).join("pc-optimizer").join("falhas")) else {
        return (0, None);
    };
    let mut arquivos: Vec<std::path::PathBuf> = std::fs::read_dir(pasta)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "txt"))
        .collect();
    arquivos.sort();
    let ultima = arquivos.last().and_then(|p| std::fs::read_to_string(p).ok()).and_then(|t| {
        t.lines().find_map(|l| l.strip_prefix("mensagem: ").map(|m| m.chars().take(120).collect::<String>()))
    });
    (arquivos.len(), ultima)
}

#[cfg(test)]
mod tests {
    use super::*;

    impl Entrada {
        /// O limite só significa algo testado contra o pior caso realista, não um relatório vazio.
        fn exemplo_cheia() -> Self {
            Entrada {
                versao: "1.5.0".to_string(),
                windows: "Windows 11 Pro 26200.5074".to_string(),
                ram_gb: 128,
                monitores: 6,
                mudancas_aplicadas: 999,
                // O catálogo inteiro aplicado, com os ids mais longos de hoje.
                aplicadas: (0..44)
                    .map(|i| format!("gpu_hardware_scheduling_{i}"))
                    .collect(),
                disco: "crítico".to_string(),
                termico: "limitado (causa não identificada)".to_string(),
                lacunas: vec![
                    "contador de erros do disco".to_string(),
                    "leitura completa de disco (sem administrador)".to_string(),
                    "limite do processador".to_string(),
                    "versão do Windows".to_string(),
                ],
                abertura_ms: Some(1234),
                quedas_seguidas: 0,
                modo_seguro: false,
                falhas: 0,
                ultima_falha: None,
                crashes_do_jogo: None,
                partida: None,
                ultimos_passos: Vec::new(),
            }
        }

        fn com_leitura_falha() -> Self {
            Entrada {
                versao: "1.5.0".to_string(),
                windows: "Windows 11 Pro 26200".to_string(),
                ram_gb: 16,
                monitores: 1,
                mudancas_aplicadas: 3,
                aplicadas: vec![
                    "plano_otimiza".to_string(),
                    "disable_gamedvr".to_string(),
                    "mmcss_games".to_string(),
                ],
                disco: "saudável".to_string(),
                termico: "sem limite ativo".to_string(),
                lacunas: vec!["contador de erros do disco".to_string()],
                abertura_ms: Some(1234),
                quedas_seguidas: 0,
                modo_seguro: false,
                falhas: 0,
                ultima_falha: None,
                crashes_do_jogo: None,
                partida: None,
                ultimos_passos: Vec::new(),
            }
        }
    }

    #[test]
    fn o_nome_do_usuario_nao_sai_no_relatorio() {
        let linha = r"09-26 13:17 WARN não abri C:\Users\Fulano\AppData\Roaming\x.json (Fulano)";
        let limpa = anonimizar(linha, r"C:\Users\Fulano", "Fulano");
        assert!(!limpa.contains("Fulano"), "{limpa}");
        assert!(limpa.contains(r"%USERPROFILE%\AppData"), "{limpa}");
    }

    #[test]
    fn o_sid_da_conta_nao_sai() {
        let id = r"task:\update-S-1-5-21-1111111111-222222222-3333333333-1000 service:X";
        assert_eq!(sem_sid(id), r"task:\update-<SID> service:X");
        assert_eq!(sem_sid("nada aqui"), "nada aqui");
    }

    #[test]
    fn do_log_saem_so_avisos_e_erros_recentes_e_curtos() {
        let log = "[2026-09-26 13:17:23.761] [INFO] tudo certo\n\
                   [2026-09-26 13:17:24.100] [WARN] primeiro aviso\n\
                   [2026-09-26 13:18:00.000] [ERROR] PÂNICO: índice fora\n\
                   [2026-09-26 13:19:00.000] [WARN] segundo aviso";
        let passos = passos_do_log(log, 2);
        assert_eq!(passos.len(), 2);
        assert!(passos[0].contains("PÂNICO") && passos[1].contains("segundo aviso"), "{passos:?}");
        assert!(passos[1].starts_with("09-26 13:19"), "{passos:?}");
        assert!(!passos.iter().any(|p| p.contains("tudo certo")));
    }

    #[test]
    fn quedas_falhas_e_crashes_entram_quando_existem() {
        let mut e = Entrada::com_leitura_falha();
        e.quedas_seguidas = 2;
        e.modo_seguro = true;
        e.falhas = 1;
        e.ultima_falha = Some("índice fora".into());
        e.crashes_do_jogo = Some("10 no FiveM, 3× bacon-vegan-carbon".into());
        let texto = montar(&e);
        for trecho in ["MODO SEGURO", "índice fora", "bacon-vegan-carbon"] {
            assert!(texto.contains(trecho), "faltou {trecho}:\n{texto}");
        }
        assert!(!montar(&Entrada::com_leitura_falha()).contains("Quedas seguidas"), "sem queda, a linha não aparece");
    }

    #[test]
    fn o_relatorio_diz_quais_otimizacoes_foram_aplicadas() {
        let texto = montar(&Entrada::com_leitura_falha());

        assert!(
            texto.contains("plano_otimiza"),
            "a contagem sozinha não diagnostica nada; falta a lista:\n{texto}"
        );
        assert!(texto.contains("mmcss_games"), "a lista veio incompleta:\n{texto}");
    }

    #[test]
    fn o_corte_come_a_lista_e_nunca_a_lacuna() {
        let texto = montar(&Entrada::exemplo_cheia());

        assert!(
            texto.len() <= LIMITE_DE_CARACTERES,
            "estourou o limite do Discord: {} bytes",
            texto.len()
        );
        assert!(
            texto.contains("Não consegui ler:"),
            "a lacuna foi cortada — é ela que manda o atendimento para o lugar \
             certo, e some calada:\n{texto}"
        );
    }
    #[test]
    fn cabe_numa_mensagem_e_nao_leva_dado_pessoal() {
        let texto = montar(&Entrada::exemplo_cheia());

        assert!(
            texto.len() <= LIMITE_DE_CARACTERES,
            "relatório com {} caracteres",
            texto.len()
        );

        // Hoje é garantido POR CONSTRUÇÃO (`resumir_disco` não lê `title` nem `measured`, monitores só como contagem);
        // as três checagens pegam quem acrescentar nome de máquina ou caminho de perfil a `Entrada`.
        let usuario = std::env::var("USERNAME").unwrap_or_default();
        if !usuario.is_empty() {
            assert!(
                !texto.contains(&usuario),
                "o relatório carrega o nome do usuário do Windows"
            );
        }

        // Muita gente batiza o PC com o próprio nome.
        let maquina = std::env::var("COMPUTERNAME").unwrap_or_default();
        if !maquina.is_empty() {
            assert!(
                !texto.contains(&maquina),
                "o relatório carrega o nome do computador"
            );
        }

        // `C:\Users\<nome>` vaza o nome mesmo sem ele aparecer sozinho.
        assert!(
            !texto.contains(r"C:\Users\"),
            "o relatório carrega um caminho de perfil (contrabarra)"
        );
        assert!(
            !texto.contains("C:/Users/"),
            "o relatório carrega um caminho de perfil (barra normal)"
        );

        // Sem regex de número de série de propósito (formatos variam e pegaria o build e os GB): a garantia é nenhuma
        // função daqui ler `SerialNumber`/`ProductId`.
    }

    #[test]
    fn o_que_nao_deu_para_ler_aparece_como_nao_sei() {
        let texto = montar(&Entrada::com_leitura_falha());
        assert!(
            texto.to_lowercase().contains("não consegui"),
            "a lacuna não aparece no relatório: {}",
            texto
        );
    }

    #[test]
    fn sem_lacuna_nenhuma_a_linha_de_lacuna_some() {
        let texto = montar(&Entrada::com_leitura_falha());
        let mut cheia = Entrada::com_leitura_falha();
        cheia.lacunas.clear();
        let texto_sem_lacuna = montar(&cheia);

        assert!(texto.to_lowercase().contains("não consegui"));
        assert!(!texto_sem_lacuna.to_lowercase().contains("não consegui"));
    }

    #[test]
    fn o_relatorio_nao_fala_mais_de_congelados() {
        // O congelamento saiu na 2.0, e a linha "Congelados agora" com ele.
        let texto = montar(&Entrada::exemplo_cheia()).to_lowercase();

        assert!(
            !texto.contains("congelad"),
            "o relatório ainda fala de congelados: {}",
            texto
        );
    }

    #[test]
    fn um_relatorio_hipoteticamente_maior_que_o_limite_ainda_cabe() {
        // Garante a Regra 1 se a lista de lacunas crescer.
        let mut entrada = Entrada::exemplo_cheia();
        entrada.lacunas = (0..200).map(|n| format!("Leitura Hipotética Número {}", n)).collect();

        let texto = montar(&entrada);

        assert!(
            texto.len() <= LIMITE_DE_CARACTERES,
            "o corte de segurança não segurou: {} caracteres",
            texto.len()
        );
        assert!(texto.ends_with('…'), "relatório cortado não avisa o corte");
    }

    #[test]
    fn resumir_disco_relata_o_pior_achado_e_a_lacuna() {
        use health::{FindingSeverity, FixLocation, HealthFinding, HealthReport};

        let relatorio = HealthReport {
            needs_admin: false,
            findings: vec![
                HealthFinding {
                    id: "disk_status_0".to_string(),
                    title: "Disco: NVMe".to_string(),
                    measured: String::new(),
                    advice: String::new(),
                    severity: FindingSeverity::Ok,
                    fix_location: FixLocation::None,
                },
                HealthFinding {
                    id: "disk_wear_1".to_string(),
                    title: "Desgaste".to_string(),
                    measured: String::new(),
                    advice: String::new(),
                    severity: FindingSeverity::Critical,
                    fix_location: FixLocation::Hardware,
                },
                HealthFinding {
                    id: "disk_errors_naosei_1".to_string(),
                    title: "Erros".to_string(),
                    measured: String::new(),
                    advice: String::new(),
                    severity: FindingSeverity::Ok,
                    fix_location: FixLocation::None,
                },
            ],
        };

        let (resumo, lacunas) = resumir_disco(&relatorio);
        assert_eq!(resumo, "crítico");
        assert_eq!(lacunas, vec!["contador de erros do disco".to_string()]);
    }

    #[test]
    fn bateria_ruim_nao_e_apresentada_como_disco_ruim() {
        use health::{FindingSeverity, FixLocation, HealthFinding, HealthReport};

        let relatorio = HealthReport {
            needs_admin: false,
            findings: vec![
                HealthFinding {
                    id: "disk_wear_0".to_string(),
                    title: "Desgaste".to_string(),
                    measured: "3% de desgaste.".to_string(),
                    advice: String::new(),
                    severity: FindingSeverity::Ok,
                    fix_location: FixLocation::None,
                },
                HealthFinding {
                    id: "battery_health".to_string(),
                    title: "Saúde da bateria".to_string(),
                    measured: "55% da capacidade de fábrica.".to_string(),
                    advice: String::new(),
                    severity: FindingSeverity::Critical,
                    fix_location: FixLocation::Hardware,
                },
            ],
        };

        let (resumo, lacunas) = resumir_disco(&relatorio);
        assert_eq!(resumo, "saudável");
        assert!(lacunas.is_empty(), "não há lacuna: o disco foi lido");
    }

    #[test]
    fn sem_achado_de_disco_o_resumo_nao_diz_saudavel() {
        use health::{FindingSeverity, FixLocation, HealthFinding, HealthReport};

        let relatorio = HealthReport {
            needs_admin: false,
            findings: vec![HealthFinding {
                id: "no_data".to_string(),
                title: "Sem dados de saúde disponíveis".to_string(),
                measured: "O Windows não expôs informação de saúde para este hardware."
                    .to_string(),
                advice: String::new(),
                severity: FindingSeverity::Ok,
                fix_location: FixLocation::None,
            }],
        };

        let (resumo, lacunas) = resumir_disco(&relatorio);
        assert_eq!(resumo, "não consegui ler");
        assert!(
            lacunas.iter().any(|l| l.contains("saúde do disco")),
            "a lacuna precisa aparecer no relatório: {:?}",
            lacunas
        );
    }

    #[test]
    fn resumir_termico_sem_medicao_vira_lacuna_e_nao_sei() {
        // `culprit == Nenhum` com `medido == false`: o `culprit` sozinho confundiria com "processador livre".
        let relatorio = thermal::ThermalReport {
            culprit: thermal::Culprit::Nenhum,
            summary: "Não foi possível medir se o processador está sendo limitado agora."
                .to_string(),
            advice: String::new(),
            percent_of_max: None,
            power_cap_percent: None,
            on_battery: false,
            thermal_events: 0,
            last_thermal_event: None,
            medido: false,
            eventos_lidos: true,
        };

        let (resumo, lacunas) = resumir_termico(&relatorio);
        assert_eq!(resumo, "não sei");
        assert_eq!(lacunas, vec!["limite do processador".to_string()]);
    }
}

/// O relatório DESTA máquina, para conferir o que sai: `cargo test --lib -- --ignored --nocapture relatorio_desta_maquina`.
#[cfg(test)]
mod desta_maquina {
    #[test]
    #[ignore]
    fn relatorio_desta_maquina() {
        println!("{}", super::montar(&super::gerar()));
    }
}
