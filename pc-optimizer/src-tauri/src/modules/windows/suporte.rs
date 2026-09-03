// O relatório que o cliente cola no atendimento
//
// A queixa que originou este módulo: um cliente pagante escreveu dizendo que
// os programas não abriam mais. O dono não tinha nenhum jeito de ver aquela
// máquina e ofereceu AnyDesk — dar acesso remoto ao próprio computador para
// alguém resolver o que deveria caber numa mensagem. E foi preciso escrever
// um script de PowerShell à mão para diagnosticar uma máquina que JÁ TINHA O
// PRODUTO instalado.
//
// O Otimiza estava sentado naquela máquina. Ele sabia a versão, sabia o que
// estava congelado, sabia quantas mudanças tinha aplicado, e as leituras de
// disco e térmico já existiam nos módulos `health` e `thermal`. Ele só não
// tinha como contar nada disso a ninguém.
//
// Este módulo é esse botão: monta um bloco de texto curto que o cliente cola
// no Discord ou no WhatsApp do atendimento, substituindo a chamada de AnyDesk
// na maioria dos casos.
//
// TRÊS REGRAS GOVERNAM O QUE VAI AQUI DENTRO — cada uma com uma história:
//
// 1. CABE NUMA MENSAGEM. Um relatório que não cabe vira anexo, e ninguém
//    anexa arquivo no meio de uma conversa de atendimento — o cliente cola
//    texto, não sobe arquivo. Ver `LIMITE_DE_CARACTERES` abaixo para o número
//    e a justificativa.
//
// 2. NADA QUE IDENTIFIQUE A PESSOA. A tabela de compras do produto guarda só
//    o Discord id e o código da placa-mãe — de propósito, sem nome nem
//    documento. Este relatório segue a mesma regra: sem nome de usuário do
//    Windows, sem caminho de perfil, nada que amarre o relatório a uma
//    pessoa em vez de a uma máquina.
//
// 3. A LACUNA APARECE. A mesma regra das leituras de saúde corrigidas na
//    1.3 (`health::ErrosDoDisco::NaoSei`, `thermal::LimitesDoProcessador::NaoSei`):
//    não conseguir medir não é o mesmo que estar bem. Um relatório de suporte
//    que omite silenciosamente uma leitura que falhou manda o atendimento
//    procurar o problema no lugar errado — o mesmo defeito que motivou aquelas
//    duas correções, só que agora na porta de saída em vez de na leitura.

use super::{display, health, memory, shell, suspend, thermal};
use serde::Deserialize;

/// Limite de caracteres do relatório.
///
/// O produto é vendido no Brasil e o atendimento acontece em Discord e
/// WhatsApp. O WhatsApp aceita mensagens bem mais longas; quem manda é o
/// Discord, cujo limite de mensagem comum é 2000 caracteres. `1900` deixa uma
/// margem de cem caracteres para o cliente colar o relatório JUNTO de uma
/// frase própria ("aqui, olha isso:") sem estourar o limite do Discord —
/// exatamente o cenário real, porque ninguém cola um bloco de texto sozinho
/// sem introduzi-lo.
pub const LIMITE_DE_CARACTERES: usize = 1900;

/// Tudo que o relatório precisa para ser montado — nenhum campo aqui faz
/// leitura de sistema. Isolar a montagem de texto da coleta é o que permite
/// testar as três regras acima sem hardware, e sem que o teste dependa do
/// estado da máquina que roda a esteira.
pub struct Entrada {
    pub versao: String,
    pub windows: String,
    pub ram_gb: u32,
    pub monitores: usize,
    /// Nomes apresentáveis do que está congelado agora (`Suspenso::visivel`).
    pub congelados: Vec<String>,
    pub mudancas_aplicadas: usize,
    /// Resumo curto do disco, já decidido (ex.: "saudável", "crítico").
    pub disco: String,
    /// Resumo curto do térmico, já decidido (ex.: "sem limite ativo").
    pub termico: String,
    /// O que não deu para ler. Vazio quando tudo foi lido — nunca omitido.
    pub lacunas: Vec<String>,
}

/// Monta o texto que vai para a área de transferência.
///
/// Pura: só formata o que `Entrada` já traz decidido. A decisão de "o que
/// está crítico" e "o que não deu para ler" mora em `resumir_disco` e
/// `resumir_termico`, não aqui — esta função só tem que caber e não vazar
/// dado pessoal, e ambas as regras são mais fáceis de garantir formatando
/// texto pronto do que decidindo em cima de structs inteiros.
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

    let congelados = if entrada.congelados.is_empty() {
        "nenhum".to_string()
    } else {
        entrada.congelados.join(", ")
    };
    linhas.push(format!("Congelados agora: {}", congelados));

    linhas.push(format!("Mudanças aplicadas: {}", entrada.mudancas_aplicadas));
    linhas.push(format!("Disco: {} · Térmico: {}", entrada.disco, entrada.termico));

    // Regra 3: a lacuna aparece. Sem este `if`, uma lista vazia de lacunas e
    // uma leitura que falhou silenciosamente ficariam indistinguíveis para
    // quem lê o relatório — por isso a linha só some quando `lacunas` está
    // de fato vazia, nunca por engano.
    if !entrada.lacunas.is_empty() {
        linhas.push(format!("Não consegui ler: {}", entrada.lacunas.join(", ")));
    }

    cortar_no_limite(linhas.join("\n"))
}

/// Rede de segurança da Regra 1, não só o teste.
///
/// `cabe_numa_mensagem_e_nao_leva_dado_pessoal` prova que o conteúdo de HOJE
/// cabe — mas a lista de congelados (`suspend::SUSPENSIVEIS`) pode crescer no
/// futuro, e nada além deste corte impediria o relatório de um dia estourar o
/// limite do Discord sem que a mensagem chegasse. Corta em vez de simplesmente
/// afirmar: preferir um relatório truncado, com `…` avisando o corte, a uma
/// mensagem que o Discord recusa silenciosamente.
fn cortar_no_limite(texto: String) -> String {
    if texto.len() <= LIMITE_DE_CARACTERES {
        return texto;
    }

    // `LIMITE_DE_CARACTERES` é contagem de bytes (a mesma conta que o teste
    // usa em `texto.len()`), e um corte no meio de um caractere acentuado
    // quebraria o UTF-8. Reserva os bytes do próprio "…" (3, não 1 — não é
    // ASCII) antes de recuar até a fronteira de caractere mais próxima, para
    // o resultado final — texto cortado MAIS reticências — não passar do
    // limite que ele existe para respeitar.
    let reserva = '…'.len_utf8();
    let mut fim = LIMITE_DE_CARACTERES.saturating_sub(reserva);
    while fim > 0 && !texto.is_char_boundary(fim) {
        fim -= 1;
    }

    format!("{}…", &texto[..fim])
}

/// Ordem de gravidade entre achados, para escolher o pior sem comparar texto.
fn rank(severidade: health::FindingSeverity) -> u8 {
    match severidade {
        health::FindingSeverity::Ok => 0,
        health::FindingSeverity::Important => 1,
        health::FindingSeverity::Critical => 2,
    }
}

/// Resume o relatório de saúde de disco numa frase curta, e separa o que não
/// deu para ler.
///
/// Não compara texto: decide pelo `id` (que é vocabulário interno, não prosa
/// de tela — `disk_errors_naosei_*` é a mesma convenção que `health.rs` já
/// usa para marcar o achado de leitura ausente) e pela `severity` tipada de
/// cada achado.
fn resumir_disco(relatorio: &health::HealthReport) -> (String, Vec<String>) {
    let mut lacunas = Vec::new();
    let mut pior: Option<health::FindingSeverity> = None;

    for achado in &relatorio.findings {
        if achado.id.contains("naosei") {
            lacunas.push("contador de erros do disco".to_string());
            continue;
        }

        if achado.severity != health::FindingSeverity::Ok {
            pior = Some(match pior {
                Some(atual) if rank(atual) >= rank(achado.severity) => atual,
                _ => achado.severity,
            });
        }
    }

    // `needs_admin` cobre um caso mais largo que o achado `naosei`: nem
    // sequer conseguimos ler o contador de confiabilidade de nenhum disco,
    // não só o de erros de um disco específico.
    if relatorio.needs_admin {
        lacunas.push("leitura completa de disco (sem administrador)".to_string());
    }

    let resumo = match pior {
        Some(health::FindingSeverity::Critical) => "crítico",
        Some(health::FindingSeverity::Important) => "atenção",
        _ => "saudável",
    };

    (resumo.to_string(), lacunas)
}

/// Resume o relatório térmico numa frase curta, e aponta a lacuna quando o
/// contador de limite não foi lido.
///
/// Usa `ThermalReport::medido` — e não compara `summary` — pela mesma razão
/// de `resumir_disco`: este projeto trava o build se a UI decidir comparando
/// prosa do backend, e este módulo segue a mesma disciplina mesmo não sendo
/// UI, para o campo continuar sendo a única fonte da verdade.
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

    (resumo.to_string(), Vec::new())
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
struct RawWindows {
    caption: Option<String>,
    build_number: Option<String>,
}

/// Lê edição e build do Windows. `None` em qualquer um dos dois vira lacuna —
/// nunca um texto genérico fingindo que leu.
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
            // "Microsoft Windows 11 Pro" vira "Windows 11 Pro": o "Microsoft"
            // não ajuda o atendimento a diagnosticar nada.
            let nome = caption.trim().replace("Microsoft ", "");
            (format!("{} {}", nome, build.trim()), Vec::new())
        }
        _ => (
            "Windows (versão não lida)".to_string(),
            vec!["versão do Windows".to_string()],
        ),
    }
}

/// Coleta tudo de verdade e monta `Entrada`. Não é pura de propósito — é a
/// única função deste módulo que toca o sistema, e a fronteira existe para
/// que `montar` e os `resumir_*` continuem testáveis sem hardware.
pub fn gerar() -> Entrada {
    let versao = env!("CARGO_PKG_VERSION").to_string();
    let (windows, mut lacunas) = ler_versao_do_windows();

    let ram_gb = memory::analyze().total_ram_gb.round().max(0.0) as u32;
    let monitores = display::monitores().len();

    let congelados = suspend::congelados()
        .into_iter()
        .map(|s| s.visivel)
        .collect();

    let mudancas_aplicadas = crate::modules::changelog::ChangeLog::load().applied().len();

    let (disco, lacunas_disco) = resumir_disco(&health::analyze());
    lacunas.extend(lacunas_disco);

    let (termico, lacunas_termico) = resumir_termico(&thermal::analyze());
    lacunas.extend(lacunas_termico);

    Entrada {
        versao,
        windows,
        ram_gb,
        monitores,
        congelados,
        mudancas_aplicadas,
        disco,
        termico,
        lacunas,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl Entrada {
        /// O maior relatório plausível: todo campo no seu valor mais longo
        /// realista. `LIMITE_DE_CARACTERES` só significa alguma coisa se for
        /// testado contra ISTO, não contra um relatório vazio — um relatório
        /// vazio cabe em qualquer limite e não prova nada sobre o caso real
        /// que importa, o cliente com o PC mais bagunçado possível.
        fn exemplo_cheia() -> Self {
            Entrada {
                versao: "1.5.0".to_string(),
                windows: "Windows 11 Pro 26200.5074".to_string(),
                ram_gb: 128,
                // Seis monitores é um posto de streaming, o topo do realista.
                monitores: 6,
                // Todo `SUSPENSIVEIS` de `suspend.rs`, congelado ao mesmo
                // tempo — o teto real de quanto essa lista pode crescer.
                congelados: vec![
                    "Discord".to_string(),
                    "Google Chrome".to_string(),
                    "Microsoft Edge".to_string(),
                    "Firefox".to_string(),
                    "Opera".to_string(),
                    "Brave".to_string(),
                    "Arc".to_string(),
                    "Spotify".to_string(),
                    "Slack".to_string(),
                    "Microsoft Teams".to_string(),
                    "WhatsApp".to_string(),
                    "Telegram".to_string(),
                ],
                mudancas_aplicadas: 999,
                disco: "crítico".to_string(),
                termico: "limitado (causa não identificada)".to_string(),
                lacunas: vec![
                    "contador de erros do disco".to_string(),
                    "leitura completa de disco (sem administrador)".to_string(),
                    "limite do processador".to_string(),
                    "versão do Windows".to_string(),
                ],
            }
        }

        /// Só o suficiente para exercitar a Regra 3 — uma leitura falhou e
        /// precisa aparecer.
        fn com_leitura_falha() -> Self {
            Entrada {
                versao: "1.5.0".to_string(),
                windows: "Windows 11 Pro 26200".to_string(),
                ram_gb: 16,
                monitores: 1,
                congelados: Vec::new(),
                mudancas_aplicadas: 3,
                disco: "saudável".to_string(),
                termico: "sem limite ativo".to_string(),
                lacunas: vec!["contador de erros do disco".to_string()],
            }
        }
    }

    #[test]
    fn cabe_numa_mensagem_e_nao_leva_dado_pessoal() {
        // CABER É REQUISITO, não estética: um relatório que não cabe numa mensagem
        // vira anexo, e ninguém anexa arquivo no meio de um atendimento.
        let texto = montar(&Entrada::exemplo_cheia());

        assert!(
            texto.len() <= LIMITE_DE_CARACTERES,
            "relatório com {} caracteres",
            texto.len()
        );

        // O produto guarda só o código da placa-mãe na tabela de compras. Este
        // relatório segue a mesma regra: nada que identifique a PESSOA.
        let usuario = std::env::var("USERNAME").unwrap_or_default();
        if !usuario.is_empty() {
            assert!(
                !texto.contains(&usuario),
                "o relatório carrega o nome do usuário do Windows"
            );
        }
    }

    #[test]
    fn o_que_nao_deu_para_ler_aparece_como_nao_sei() {
        // Mesma regra das leituras de saúde: não conseguir medir não é o mesmo que
        // estar bem, e um relatório de suporte que omite a lacuna manda o
        // atendimento procurar no lugar errado.
        let texto = montar(&Entrada::com_leitura_falha());
        assert!(
            texto.to_lowercase().contains("não consegui"),
            "a lacuna não aparece no relatório: {}",
            texto
        );
    }

    #[test]
    fn sem_lacuna_nenhuma_a_linha_de_lacuna_some() {
        // O espelho do teste acima: quando tudo foi lido, a linha "Não
        // consegui ler" não deve aparecer — senão o relatório mentiria sobre
        // uma lacuna que não existe.
        let texto = montar(&Entrada::com_leitura_falha());
        let mut cheia = Entrada::com_leitura_falha();
        cheia.lacunas.clear();
        let texto_sem_lacuna = montar(&cheia);

        assert!(texto.to_lowercase().contains("não consegui"));
        assert!(!texto_sem_lacuna.to_lowercase().contains("não consegui"));
    }

    #[test]
    fn congelados_vazio_diz_nenhum() {
        let texto = montar(&Entrada::com_leitura_falha());
        assert!(texto.contains("Congelados agora: nenhum"));
    }

    #[test]
    fn congelados_com_gente_lista_os_nomes() {
        let mut entrada = Entrada::com_leitura_falha();
        entrada.congelados = vec!["Discord".to_string(), "Google Chrome".to_string()];
        let texto = montar(&entrada);
        assert!(texto.contains("Congelados agora: Discord, Google Chrome"));
    }

    #[test]
    fn um_relatorio_hipoteticamente_maior_que_o_limite_ainda_cabe() {
        // `exemplo_cheia` já cabe folgado hoje — este teste é o que garante
        // que a Regra 1 continua valendo se a lista de congelados crescer no
        // futuro, sem depender de ninguém lembrar de revisar o teste acima.
        // Sintetiza um cenário maior que qualquer congelamento real seria
        // capaz de produzir hoje.
        let mut entrada = Entrada::exemplo_cheia();
        entrada.congelados = (0..200).map(|n| format!("Programa Hipotético Número {}", n)).collect();

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
    fn resumir_termico_sem_medicao_vira_lacuna_e_nao_sei() {
        // Constrói o `ThermalReport` do jeito que `montar_relatorio` monta no
        // caso `NaoSei` — `culprit == Nenhum` mas `medido == false` — para
        // provar que este módulo lê o campo `medido`, e não o `culprit`
        // sozinho, que sozinho confundiria isto com "processador livre".
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
        };

        let (resumo, lacunas) = resumir_termico(&relatorio);
        assert_eq!(resumo, "não sei");
        assert_eq!(lacunas, vec!["limite do processador".to_string()]);
    }
}
