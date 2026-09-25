// O veredito: recolhe os diagnósticos baratos, traduz para o mesmo vocabulário e elege UMA frase com o número
// que a sustenta (numa máquina que travava ao abrir o jogo, a tela dizia "memória sem problemas" porque cada
// pedaço morava numa aba). Disco morrendo vence memória em canal único: custa os arquivos do cliente, e otimizar
// sobre ele é trabalho perdido.

use super::achados::{
    peso_confianca, peso_severidade, Acao, Achado, Causa, Confianca, EmAchados, FindingSeverity,
    FixLocation, Lacuna, Origem,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Veredito {
    pub frase: String,
    /// O que foi medido para poder afirmar aquilo.
    pub detalhe: String,
    /// O achado que decidiu a frase. Ausente quando nada foi encontrado.
    pub principal: Option<Achado>,
    /// Outros achados da MESMA causa: junta canal único e memória prometida acima da física, de módulos diferentes.
    pub corroboracoes: Vec<Achado>,
    /// Todos os achados, já ordenados pela regra de eleição.
    pub achados: Vec<Achado>,
    /// Nunca fica escondido.
    pub lacunas: Vec<Lacuna>,
    /// O relatório de desempenho perdido (2.9): há desempenho que o hardware
    /// deveria entregar e não entrega? Com prioridade por achado.
    pub recuperacao: Recuperacao,
}

/// Prioridade de um problema. Sempre resolver P0 e P1 primeiro.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Prioridade {
    /// Erro de configuração crítico: a máquina está entregando muito menos do
    /// que deveria por um motivo que se corrige (boot limitado, paginação
    /// desligada, disco morrendo).
    P0,
    /// Oportunidade grande: memória sem XMP ou em canal único, configuração
    /// de jogo pesada, falta de memória registrada.
    P1,
    /// Oportunidade moderada.
    P2,
    /// Pequena ou deduzida só de configuração.
    P3,
}

/// Ids que são erro crítico de configuração, independente do resto.
const P0: &[&str] = &["boot_limits_present", "pagefile_off", "windows_registrou_esgotamento"];
/// Ids que são oportunidade grande quando aparecem.
const P1: &[&str] = &[
    "memory_xmp_off",
    "memory_single_channel",
    "config_do_jogo_pesada",
    "low_ram",
    "memoria_esgotada_historico",
    "pressao_recorrente",
    "sustained_decay",
    "teto_no_plano_de_energia",
];

/// A prioridade de um achado. `None` para o que está certo. **Pura.**
pub fn prioridade(a: &Achado) -> Option<Prioridade> {
    if a.severity == FindingSeverity::Ok {
        return None;
    }
    // Disco com falha é P0 pelo risco de perder arquivo, antes de desempenho.
    if P0.contains(&a.id.as_str()) || (a.causa == Causa::Armazenamento && a.severity == FindingSeverity::Critical && a.id.starts_with("disk_")) {
        return Some(Prioridade::P0);
    }
    if P1.contains(&a.id.as_str()) || a.severity == FindingSeverity::Critical {
        return Some(Prioridade::P1);
    }
    Some(if a.confianca == Confianca::Inferido { Prioridade::P3 } else { Prioridade::P2 })
}

/// Há desempenho perdido nesta máquina?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Perdido {
    /// Pelo menos um P0/P1 sustentado por medição, histórico ou declaração.
    Sim,
    /// Nada de P0/P1, e nada que deixou de ser verificado.
    Nao,
    /// Só há suspeita deduzida de configuração, ou faltou verificar coisas.
    Incerto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recuperacao {
    pub perdido: Perdido,
    /// (id do achado, prioridade), na ordem em que resolver.
    pub itens: Vec<(String, Prioridade)>,
}

/// Monta o relatório. **Pura.**
pub fn recuperacao(achados: &[Achado], lacunas: &[Lacuna]) -> Recuperacao {
    let mut itens: Vec<(String, Prioridade, u8)> = achados
        .iter()
        .filter_map(|a| prioridade(a).map(|p| (a.id.clone(), p, peso_confianca(a.confianca))))
        .collect();
    itens.sort_by_key(|(_, p, c)| (*p, *c));
    let forte = achados.iter().any(|a| {
        matches!(prioridade(a), Some(Prioridade::P0 | Prioridade::P1)) && a.confianca != Confianca::Inferido
    });
    let fraco = itens.iter().any(|(_, p, _)| *p <= Prioridade::P1);
    let perdido = if forte {
        Perdido::Sim
    } else if fraco || !lacunas.is_empty() {
        Perdido::Incerto
    } else {
        Perdido::Nao
    };
    Recuperacao { perdido, itens: itens.into_iter().map(|(id, p, _)| (id, p)).collect() }
}

/// Lexicográfica: cada critério só conta quando o anterior empata; a ordem é a decisão de produto.
fn peso(a: &Achado) -> (u8, u8, u8, u8) {
    (
        // 1. Severidade. Inegociável.
        peso_severidade(a.severity),
        // 2. Risco de perder arquivo antes de risco de perder desempenho.
        if a.causa == Causa::Armazenamento { 0 } else { 1 },
        // 3. Evidência que persiste vence a foto do instante: o cliente abre o Otimiza com o jogo fechado.
        peso_confianca(a.confianca),
        // 4. Só então onde se conserta.
        match a.fix_location {
            FixLocation::Hardware => 0,
            FixLocation::Bios => 1,
            FixLocation::Software => 2,
            FixLocation::None => 3,
        },
    )
}

/// **Pura**: testa a decisão com números de uma máquina real sem tocar em máquina nenhuma.
pub fn veredito(achados: &[Achado], lacunas: &[Lacuna]) -> Veredito {
    let mut ordenados = achados.to_vec();
    ordenados.sort_by_key(peso);

    let principal = ordenados
        .iter()
        .find(|a| a.severity != FindingSeverity::Ok)
        .cloned();

    let (frase, detalhe, corroboracoes) = match &principal {
        Some(p) => {
            let corroboracoes: Vec<Achado> = ordenados
                .iter()
                .filter(|a| {
                    a.causa == p.causa && a.id != p.id && a.severity != FindingSeverity::Ok
                })
                .cloned()
                .collect();

            (p.title.clone(), p.measured.clone(), corroboracoes)
        }
        // Não achar nada é resultado de primeira classe, com os números que sustentam.
        None => (
            "Não encontramos causa de travamento nesta máquina".to_string(),
            detalhe_de_maquina_sadia(&ordenados, lacunas),
            Vec::new(),
        ),
    };

    let recuperacao = recuperacao(&ordenados, lacunas);
    Veredito {
        frase,
        detalhe,
        principal,
        corroboracoes,
        achados: ordenados,
        lacunas: lacunas.to_vec(),
        recuperacao,
    }
}

fn detalhe_de_maquina_sadia(ordenados: &[Achado], lacunas: &[Lacuna]) -> String {
    if ordenados.is_empty() {
        return "Nenhum diagnóstico pôde ser concluído — veja o que faltou abaixo.".to_string();
    }

    // Mesmo dizendo "está tudo bem", só falamos com número medido.
    let conferidos: Vec<&str> = ordenados.iter().map(|a| a.measured.as_str()).collect();
    let mut texto = format!(
        "{} verificações passaram: {}",
        conferidos.len(),
        conferidos.join(" · ")
    );

    if !lacunas.is_empty() {
        texto.push_str(&format!(
            " — mas {} verificação(ões) não puderam ser feitas, então isto não é um atestado completo.",
            lacunas.len()
        ));
    }

    texto
}

/// Tabela explícita: um agrupamento errado inventaria uma relação. Confira linha a linha numa revisão.
fn causa_de(origem: Origem, id: &str) -> Causa {
    match origem {
        Origem::Memoria | Origem::Pressao => Causa::Memoria,
        Origem::Conflitos => Causa::Conflito,
        Origem::Termico => Causa::Refrigeracao,
        Origem::Disco => Causa::Armazenamento,

        // Saúde cobre disco e bateria. Só o disco põe arquivo em risco.
        Origem::Saude => {
            if id.starts_with("disk_") {
                Causa::Armazenamento
            } else {
                Causa::Configuracao
            }
        }

        // O canal único precisa cair em `Memoria`, junto do "prometido acima do físico".
        Origem::Firmware => {
            if id.starts_with("memory_") {
                Causa::Memoria
            } else if id.starts_with("sustained_") {
                Causa::Refrigeracao
            } else {
                Causa::Configuracao
            }
        }

        // Prontidão: a paginação em disco mecânico é assunto de armazenamento;
        // o resto é configuração do Windows.
        Origem::Prontidao => {
            if id == "trim" || id == "paginacao" {
                Causa::Armazenamento
            } else {
                Causa::Configuracao
            }
        }

        // Esgotamento de memória é declarado pelo Windows; programa que parou de responder tem várias causas e NÃO pode
        // ser agrupado sob memória.
        Origem::Esgotamento => {
            if id == "windows_registrou_esgotamento" {
                Causa::Memoria
            } else {
                Causa::Indefinida
            }
        }

        Origem::Monitor
        | Origem::PlacaDeVideo
        | Origem::ConfigDoJogo
        | Origem::Gargalo
        | Origem::Boot => Causa::Configuracao,
    }
}

/// "XMP desligado" é configuração observada, não prova: `Inferido` impede que vença um esgotamento registrado.
fn confianca_de(origem: Origem, id: &str) -> Confianca {
    match (origem, id) {
        // Marca d'água e log de eventos valem para o que já aconteceu, mesmo
        // com o PC calmo agora.
        (Origem::Memoria, "memoria_esgotada_historico") => Confianca::Historico,
        (Origem::Memoria, "pagefile_small") => Confianca::Historico,
        (Origem::Saude, _) => Confianca::Historico, // SMART é contador acumulado
        // Amostragem acumulada de dias: registro do que já aconteceu.
        (Origem::Pressao, _) => Confianca::Historico,
        // O Windows declarou o esgotamento textualmente, com data e hora. Não
        // é dedução nossa, e por isso vence a marca d'água da paginação.
        (Origem::Esgotamento, "windows_registrou_esgotamento") => Confianca::Declarado,
        // O programa travar é fato registrado, mas a causa não foi declarada.
        (Origem::Esgotamento, _) => Confianca::Historico,
        (Origem::Boot, _) => Confianca::Historico,

        // Presença de programa, chave de registro, plano de energia: são fatos
        // de configuração, não medições de efeito.
        (Origem::Conflitos, _) => Confianca::Inferido,
        (Origem::Prontidao, _) => Confianca::Inferido,
        (Origem::Firmware, "memory_xmp_off") => Confianca::Inferido,
        (Origem::Firmware, "vbs_running") => Confianca::Inferido,
        (Origem::Firmware, "vbs_sem_uso") => Confianca::Inferido,

        _ => Confianca::Medido,
    }
}

/// Curta de propósito: memória insuficiente, disco morrendo e BIOS não têm botão. Só entra comando registrado,
/// testado e reversível pelo histórico.
fn acao_de(origem: Origem, id: &str) -> Option<Acao> {
    let (comando, argumento, rotulo, exige_admin) = match (origem, id) {
        // Existia só como botão dentro de um painel, e quem mais precisava nunca chegava lá.
        (Origem::Memoria, "pagefile_off") | (Origem::Memoria, "pagefile_manual") => (
            "set_automatic_pagefile",
            None,
            "Deixar o Windows gerenciar a paginação",
            true,
        ),

        // `apply_optimization` recebe `id: String`, que casa com o `{ id: argumento }` da tela.

        // O caso mais comum segundo o `thermal.rs`: teto no plano de energia.
        (Origem::Termico, "teto_no_plano_de_energia") => (
            "apply_optimization",
            Some("plano_otimiza"),
            "Aplicar o plano de energia do Otimiza",
            true,
        ),

        (Origem::Firmware, "boot_limits_present") => (
            "apply_optimization",
            Some("clear_boot_limits"),
            "Liberar os limites de inicialização",
            true,
        ),

        // Plano de energia de terceiro — mesma cura do teto no plano.
        (Origem::Prontidao, "plano_de_terceiro") => (
            "apply_optimization",
            Some("plano_otimiza"),
            "Aplicar o plano de energia do Otimiza",
            true,
        ),

        // A mesma cura de `pagefile_off`: devolver a decisão ao Windows.
        (Origem::Memoria, "pagefile_small") => (
            "set_automatic_pagefile",
            None,
            "Deixar o Windows gerenciar a paginação",
            true,
        ),

        (Origem::Prontidao, "trim") => (
            "fix_readiness",
            Some("trim"),
            "Ligar o TRIM do SSD",
            true,
        ),

        (Origem::Prontidao, "janelas_otimizadas_desligada") => (
            "apply_optimization",
            Some("windowed_game_optimizations"),
            "Ligar as otimizações para jogos em janela",
            false,
        ),

        _ => return None,
    };

    Some(Acao {
        comando: comando.to_string(),
        argumento: argumento.map(str::to_string),
        rotulo: rotulo.to_string(),
        exige_admin,
    })
}

fn montar(
    origem: Origem,
    id: String,
    title: String,
    measured: String,
    advice: String,
    severity: FindingSeverity,
    fix_location: FixLocation,
) -> Achado {
    let causa = causa_de(origem, &id);
    let confianca = confianca_de(origem, &id);
    let acao = acao_de(origem, &id);

    Achado {
        id,
        origem,
        causa,
        title,
        measured,
        advice,
        severity,
        fix_location,
        confianca,
        acao,
    }
}

impl EmAchados for super::memory::MemoryReport {
    fn achados(&self) -> Vec<Achado> {
        self.findings
            .iter()
            .map(|f| {
                montar(
                    Origem::Memoria,
                    f.id.clone(),
                    f.title.clone(),
                    f.measured.clone(),
                    f.advice.clone(),
                    f.severity,
                    f.fix_location,
                )
            })
            .collect()
    }
}

/// Otimizações para jogos em janela desligada (Windows 11). Leitura de registro.
fn achados_da_janela() -> Result<Vec<Achado>, String> {
    Ok(match super::janelas::ligada()? {
        Some(false) => vec![montar(
            Origem::Prontidao,
            "janelas_otimizadas_desligada".to_string(),
            "Otimizações para jogos em janela desligada".to_string(),
            "A opção do Windows 11 está desligada nesta máquina.".to_string(),
            "Ligada, ela tira o atraso dos jogos em tela cheia sem borda e deixa o VRR funcionar nesse modo. Não é um ajuste para aumentar o FPS.".to_string(),
            FindingSeverity::Important,
            FixLocation::Software,
        )],
        _ => Vec::new(),
    })
}

fn achados_do_x3d() -> Vec<Achado> {
    let Some(cpu) = super::x3d::cpu().filter(|c| super::x3d::e_x3d_de_dois_blocos(c)) else { return Vec::new() };
    super::x3d::achados(&cpu, &super::x3d::ler())
        .into_iter()
        .map(|a| montar(Origem::Prontidao, a.id.to_string(), a.titulo, a.medido, a.conselho, a.severidade, a.onde))
        .collect()
}

fn achados_de_eventos_de_hardware() -> Result<Vec<Achado>, String> {
    Ok(super::eventoshw::achados(&super::eventoshw::ler()?)
        .into_iter()
        .map(|a| montar(Origem::Esgotamento, a.id.to_string(), a.titulo, a.medido, a.conselho, a.severidade, a.onde))
        .collect())
}

/// Intel de mesa da 13ª ou 14ª geração sem o microcódigo 0x12F.
fn achados_do_microcodigo() -> Result<Vec<Achado>, String> {
    Ok(match super::fichabios::defeito_de_microcodigo()? {
        Some(super::fichabios::Defeito::MicrocodigoIntelAntigo { atual }) => vec![montar(
            Origem::Firmware,
            "microcodigo_intel_antigo".to_string(),
            "BIOS sem a correção de instabilidade da Intel".to_string(),
            format!("O microcódigo carregado é o 0x{:X}; a correção da Intel é o 0x12F.", atual),
            "Processadores de mesa da 13ª e 14ª geração com o chip Raptor Lake, como este, degradam com o tempo sem essa correção. Ela chega por atualização de BIOS, na página oficial da sua placa-mãe. A aba BIOS mostra o modelo da placa.".to_string(),
            FindingSeverity::Critical,
            FixLocation::Bios,
        )],
        _ => Vec::new(),
    })
}

/// Serve ao relatório completo de firmware e ao diagnóstico rápido, que só recolhe os de memória.
fn achados_de_firmware(findings: &[super::firmware::FirmwareFinding]) -> Vec<Achado> {
    findings
        .iter()
        .map(|f| {
            montar(
                Origem::Firmware,
                f.id.clone(),
                f.title.clone(),
                f.measured.clone(),
                f.advice.clone(),
                f.severity,
                f.fix_location,
            )
        })
        .collect()
}

impl EmAchados for super::firmware::FirmwareReport {
    fn achados(&self) -> Vec<Achado> {
        achados_de_firmware(&self.findings)
    }
}

impl EmAchados for super::health::HealthReport {
    fn achados(&self) -> Vec<Achado> {
        self.findings
            .iter()
            .map(|f| {
                montar(
                    Origem::Saude,
                    f.id.clone(),
                    f.title.clone(),
                    f.measured.clone(),
                    f.advice.clone(),
                    f.severity,
                    f.fix_location,
                )
            })
            .collect()
    }
}

impl EmAchados for super::readiness::ReadinessReport {
    fn achados(&self) -> Vec<Achado> {
        self.findings
            .iter()
            .map(|f| {
                montar(
                    Origem::Prontidao,
                    f.id.clone(),
                    f.title.clone(),
                    f.measured.clone(),
                    f.advice.clone(),
                    f.severity,
                    f.fix_location,
                )
            })
            .collect()
    }
}

impl EmAchados for super::configjogo::ConfigJogoReport {
    fn achados(&self) -> Vec<Achado> {
        self.findings
            .iter()
            .map(|f| {
                montar(
                    Origem::ConfigDoJogo,
                    f.id.clone(),
                    f.title.clone(),
                    f.measured.clone(),
                    f.advice.clone(),
                    f.severity,
                    f.fix_location,
                )
            })
            .collect()
    }
}

impl EmAchados for super::gpupref::GpuPrefReport {
    fn achados(&self) -> Vec<Achado> {
        self.findings
            .iter()
            .map(|f| {
                montar(
                    Origem::PlacaDeVideo,
                    f.id.clone(),
                    f.title.clone(),
                    f.measured.clone(),
                    f.advice.clone(),
                    f.severity,
                    f.fix_location,
                )
            })
            .collect()
    }
}

impl EmAchados for super::display::DisplayReport {
    fn achados(&self) -> Vec<Achado> {
        self.findings
            .iter()
            .map(|f| {
                let mut achado = montar(
                    Origem::Monitor,
                    f.id.clone(),
                    f.title.clone(),
                    f.measured.clone(),
                    f.advice.clone(),
                    f.severity,
                    f.fix_location,
                );

                // O único achado resolvido com um clique que muda a tela na hora (quase ninguém acha o caminho no Windows). O do
                // cabo na placa-mãe se resolve com a mão.
                if !f.id.starts_with("hz_abaixo_") {
                    return achado;
                }
                achado.acao = Some(Acao {
                    comando: "set_max_refresh_rate".to_string(),
                    argumento: Some(f.dispositivo.clone()),
                    rotulo: format!("Colocar em {} Hz", f.hz_alvo),
                    exige_admin: false,
                });

                achado
            })
            .collect()
    }
}

impl EmAchados for super::pressao::PressaoReport {
    fn achados(&self) -> Vec<Achado> {
        self.findings
            .iter()
            .map(|f| {
                montar(
                    Origem::Pressao,
                    f.id.clone(),
                    f.title.clone(),
                    f.measured.clone(),
                    f.advice.clone(),
                    f.severity,
                    f.fix_location,
                )
            })
            .collect()
    }
}

impl EmAchados for super::exhaustion::EsgotamentoReport {
    fn achados(&self) -> Vec<Achado> {
        self.findings
            .iter()
            .map(|f| {
                montar(
                    Origem::Esgotamento,
                    f.id.clone(),
                    f.title.clone(),
                    f.measured.clone(),
                    f.advice.clone(),
                    f.severity,
                    f.fix_location,
                )
            })
            .collect()
    }
}

impl EmAchados for super::conflicts::ConflictReport {
    fn achados(&self) -> Vec<Achado> {
        self.conflicts
            .iter()
            .map(|c| {
                // `measured` nunca fica vazio: o número é quantos programas foram examinados.
                let medido = if c.found.is_empty() {
                    match self.programs_scanned {
                        Some(n) => format!(
                            "{} programas instalados examinados, sem dois disputando a mesma função.",
                            n
                        ),
                        None => c.explanation.clone(),
                    }
                } else {
                    format!("Encontrados: {}.", c.found.join(", "))
                };

                montar(
                    Origem::Conflitos,
                    c.id.clone(),
                    c.title.clone(),
                    medido,
                    c.advice.clone(),
                    c.severity,
                    FixLocation::Software,
                )
            })
            .collect()
    }
}

impl EmAchados for super::thermal::ThermalReport {
    fn achados(&self) -> Vec<Achado> {
        use super::thermal::Culprit;

        // Nada segurando NÃO vira achado, e `Bateria` também não: é limitação correta do Windows.
        let (id, titulo, severidade, onde) = match self.culprit {
            Culprit::Nenhum | Culprit::Bateria => return Vec::new(),

            // A ÚNICA situação em que o módulo diz "calor": problema físico que nenhum ajuste resolve.
            Culprit::Calor => (
                "throttling_termico",
                "O processador está sendo segurado por temperatura",
                FindingSeverity::Critical,
                FixLocation::Hardware,
            ),

            // Teto no plano de energia é software, e o produto conserta.
            Culprit::PlanoDeEnergia => (
                "teto_no_plano_de_energia",
                "O plano de energia está segurando o processador",
                FindingSeverity::Important,
                FixLocation::Software,
            ),

            // Limite elétrico não pode ser vendido como sujeira no cooler.
            Culprit::LimiteEletrico => (
                "limite_eletrico",
                "O processador está limitado por energia, não por temperatura",
                FindingSeverity::Important,
                FixLocation::Hardware,
            ),

            // O fato é medido, a causa não: `Indefinida`.
            Culprit::NaoIdentificado => (
                "frequencia_baixa_sem_causa",
                "O processador está abaixo do que pode, e não sei dizer por quê",
                FindingSeverity::Important,
                FixLocation::None,
            ),
        };

        vec![montar(
            Origem::Termico,
            id.to_string(),
            titulo.to_string(),
            self.summary.clone(),
            self.advice.clone(),
            severidade,
            onde,
        )]
    }
}

/// Não é "atualizar sempre" (driver novo regride): é a distância, dois anos sem as otimizações da placa.
const DIAS_QUE_TORNAM_O_DRIVER_VELHO: i64 = 365;

impl EmAchados for super::shaders::ShaderReport {
    fn achados(&self) -> Vec<Achado> {
        let Some(dias) = self.driver_age_days else {
            return Vec::new();
        };

        if dias < DIAS_QUE_TORNAM_O_DRIVER_VELHO {
            return Vec::new();
        }

        let placa = self.gpu.clone().unwrap_or_else(|| "A placa de vídeo".to_string());

        vec![montar(
            Origem::Disco,
            "driver_de_video_velho".to_string(),
            "O driver de vídeo tem mais de um ano".to_string(),
            format!(
                "{} está com o driver {} de {}, publicado há {} dias.",
                placa,
                self.driver_version.clone().unwrap_or_else(|| "?".into()),
                self.driver_date.clone().unwrap_or_else(|| "?".into()),
                dias
            ),
            "Driver mais novo costuma trazer ganho em jogo recente. Baixe pelo site \
             do fabricante da placa — não pelo Windows Update, que entrega versões \
             mais antigas."
                .to_string(),
            FindingSeverity::Important,
            // `None`: instalar driver baixa da internet e troca componente de vídeo; errar deixa sem imagem.
            FixLocation::None,
        )]
    }
}

/// Abaixo disto o Windows falha de formas que ninguém associa a disco.
const GB_LIVRES_QUE_JA_E_PROBLEMA: f64 = 10.0;

impl EmAchados for super::diskspace::DiskReport {
    fn achados(&self) -> Vec<Achado> {
        // Sem medição, sem achado: `disk_usage` devolvia (0, 0) sem volume e virava "Restam 0.0 GB" Critical. Disco
        // cheio de verdade (total > 0) continua Critical; o não medido vira lacuna na coleta.
        if self.medida_do_espaco != super::diskspace::Medida::Medido {
            return Vec::new();
        }

        let livres_gb = self.free_bytes as f64 / 1_073_741_824.0;

        if livres_gb >= GB_LIVRES_QUE_JA_E_PROBLEMA {
            return Vec::new();
        }

        // Com o quanto dá para limpar, a reclamação vira ação.
        let limpavel = self
            .findings
            .iter()
            .filter(|f| f.cleanable)
            .map(|f| f.bytes)
            .sum::<u64>() as f64
            / 1_073_741_824.0;

        vec![montar(
            Origem::Disco,
            "disco_quase_cheio".to_string(),
            "O disco do sistema está quase sem espaço".to_string(),
            format!("Restam {:.1} GB livres no disco do Windows.", livres_gb),
            if limpavel >= 1.0 {
                format!(
                    "A aba Espaço encontrou cerca de {:.1} GB de lixo que dá para \
                     apagar aqui mesmo.",
                    limpavel
                )
            } else {
                "Libere espaço apagando arquivos grandes ou desinstalando o que não \
                 usa. Abaixo de 10 GB o Windows começa a falhar de formas que \
                 ninguém associa a disco cheio."
                    .to_string()
            },
            FindingSeverity::Critical,
            FixLocation::Software,
        )]
    }
}

/// Cada diagnóstico abre um `powershell.exe` (~40 MB prometidos): quatro juntos pesariam 150 MB na máquina
/// diagnosticada por falta de memória. Três no máximo; em máquina fraca, um.
const DIAGNOSTICOS_SIMULTANEOS: usize = 3;

fn limite_de_simultaneos() -> usize {
    let nucleos = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    if nucleos <= 2 {
        1
    } else {
        DIAGNOSTICOS_SIMULTANEOS
    }
}

/// Fora os caros ou que mentem na hora errada: o gargalo leva até 30 s e, parado, devolve "sem carga".
pub fn coletar_rapido() -> (Vec<Achado>, Vec<Lacuna>) {
    type Tarefa = (Origem, fn() -> Result<Vec<Achado>, String>);

    // Os caros primeiro: a fila termina no mais demorado, não na soma. Medido em 12/08/2026: prontidão 4,8 s, saúde
    // 4,5 s, memória 1,1 s, firmware 0,9 s.
    let tarefas: Vec<Tarefa> = vec![
        (Origem::Prontidao, || Ok(super::readiness::analyze().achados())),
        (Origem::Saude, || {
            let relatorio = super::health::analyze();

            // Sem administrador o SMART não se lê: lacuna, nunca silêncio.
            if relatorio.needs_admin && relatorio.findings.is_empty() {
                return Err(
                    "Ler a saúde do disco exige executar o Otimiza como administrador."
                        .to_string(),
                );
            }

            Ok(relatorio.achados())
        }),
        // Não ler a memória vira lacuna: some junto o canal único, o achado mais valioso.
        (Origem::Memoria, || {
            let relatorio = super::memory::analyze();

            if !relatorio.medido {
                return Err("nao consegui ler a memoria desta maquina".to_string());
            }

            Ok(relatorio.achados())
        }),
        // Uma leitura de arquivo, e a única que vê o que aconteceu enquanto o cliente jogava.
        (Origem::Pressao, || Ok(super::pressao::analyze().achados())),
        (Origem::Monitor, || Ok(super::display::analyze().achados())),
        (Origem::PlacaDeVideo, || Ok(super::gpupref::analyze().achados())),
        // A maior alavanca num PC fraco. Uma leitura de arquivo.
        (Origem::ConfigDoJogo, || Ok(super::configjogo::analyze().achados())),
        // A evidência mais forte e mais barata: o Windows já anotou o esgotamento e o programa que travou.
        (Origem::Esgotamento, || {
            let relatorio = super::exhaustion::analyze();

            match relatorio.erro.clone() {
                Some(motivo) => Err(motivo),
                None => Ok(relatorio.achados()),
            }
        }),
        // `Err` na falha: `Ok(vec![])` deixava a tela calada sobre memória.
        (Origem::Firmware, || {
            super::firmware::analyze_memory_ou_lacuna().map(|f| achados_de_firmware(&f))
        }),
        // Térmico, disco, shaders e conflitos entram na eleição (medido em 31/08/2026: 1,33 s, 0,44 s, 0,13 s, 0,14 s,
        // todos abaixo do `readiness`). O disco é `scan_para_o_veredito()`, sem DISM (guarda
        // `o_diagnostico_rapido_nao_chama_quem_mede_por_segundos`). Boot e bloatware ficam fora: são higiene e empurrariam
        // a causa real para baixo.
        (Origem::Termico, || Ok(super::thermal::analyze().achados())),
        // 3.0: janela e microcódigo pelo registro; eventos e X3D pelo PowerShell (X3D só nos quatro processadores dele).
        (Origem::Prontidao, achados_da_janela),
        (Origem::Esgotamento, achados_de_eventos_de_hardware),
        (Origem::Prontidao, || Ok(achados_do_x3d())),
        (Origem::Firmware, achados_do_microcodigo),
        // Não medir o disco vira lacuna, não silêncio.
        (Origem::Disco, || {
            let relatorio = super::diskspace::scan_para_o_veredito();

            if relatorio.medida_do_espaco != super::diskspace::Medida::Medido {
                return Err("nao consegui medir o espaco do disco do sistema".to_string());
            }

            Ok(relatorio.achados())
        }),
        (Origem::Disco, || Ok(super::shaders::analyze().achados())),
        (Origem::Conflitos, || {
            let relatorio = super::conflicts::analyze();

            // Leitura falha sem nada encontrado é lacuna; conflito encontrado com leitura parcial fica.
            if relatorio.conflicts.is_empty() && !relatorio.lacunas.is_empty() {
                return Err(relatorio.lacunas.join(" · "));
            }

            Ok(relatorio.achados())
        }),
    ];

    coletar_em_paralelo(tarefas)
}

/// Separado para o teste passar uma tarefa que entra em pânico de propósito.
fn coletar_em_paralelo(
    tarefas: Vec<(Origem, fn() -> Result<Vec<Achado>, String>)>,
) -> (Vec<Achado>, Vec<Lacuna>) {
    // Fila, não lotes fixos: com lotes o custo é a soma dos mais lentos de cada lote (16 s onde 5 bastavam).
    let fila = std::sync::Mutex::new(tarefas.into_iter());
    let coletado = std::sync::Mutex::new((Vec::new(), Vec::new()));

    std::thread::scope(|escopo| {
        for _ in 0..limite_de_simultaneos() {
            escopo.spawn(|| loop {
                // `lock().unwrap()` com tranca envenenada derrubaria todos os trabalhadores e a tela inteira. Segue com o dado; a
                // tarefa que falhou vira lacuna declarada.
                let proximo = fila.lock().unwrap_or_else(|e| e.into_inner()).next();
                let Some((origem, tarefa)) = proximo else {
                    return;
                };

                // Sem `catch_unwind` o pânico atravessa o `spawn` e envenena `coletado` para todos; com ele, vira lacuna.
                let resultado = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(tarefa)) {
                    Ok(resultado) => resultado,
                    // `payload.as_ref()`: com `&payload` o `downcast_ref` confere o tipo da CAIXA e nunca casa com `&str`/`String`.
                    Err(payload) => Err(mensagem_de_panico(payload.as_ref())),
                };

                let mut guarda = coletado.lock().unwrap_or_else(|e| e.into_inner());
                let (achados, lacunas) = &mut *guarda;

                match resultado {
                    Ok(mut novos) => achados.append(&mut novos),
                    Err(motivo) => lacunas.push(Lacuna {
                        origem,
                        o_que: nome_da_origem(origem).to_string(),
                        por_que: motivo,
                    }),
                }
            });
        }
    });

    coletado.into_inner().unwrap_or_else(|e| e.into_inner())
}

/// O prefixo "(pânico)" marca que o texto é sintoma de bug, não explicação para o cliente; a severidade continua
/// vindo de `Lacuna`.
fn mensagem_de_panico(payload: &(dyn std::any::Any + Send)) -> String {
    let texto = payload
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "motivo desconhecido".to_string());

    format!("O diagnóstico travou de forma inesperada (pânico): {texto}")
}

fn nome_da_origem(origem: Origem) -> &'static str {
    match origem {
        Origem::Memoria => "Memória e paginação",
        Origem::Firmware => "Memória instalada e BIOS",
        Origem::Saude => "Saúde do disco e da bateria",
        Origem::Conflitos => "Programas em conflito",
        Origem::Prontidao => "Prontidão do sistema",
        Origem::Gargalo => "Gargalo de desempenho",
        Origem::Termico => "Limitação por temperatura",
        Origem::Boot => "Tempo de inicialização",
        Origem::Disco => "Espaço em disco",
        Origem::Esgotamento => "Registro de eventos do Windows",
        Origem::Monitor => "Monitor e taxa de atualização",
        Origem::PlacaDeVideo => "Placa de vídeo por jogo",
        Origem::ConfigDoJogo => "Configuração gráfica do jogo",
        Origem::Pressao => "Observação dos últimos dias",
    }
}

pub fn diagnostico_rapido() -> Veredito {
    let (achados, lacunas) = coletar_rapido();
    veredito(&achados, &lacunas)
}

#[cfg(test)]
mod medicao_de_tempo {
    //! Mede de novo a ordem "caros primeiro" e o limite de simultâneos: medição envelhece.
    //! `cargo test --lib -- --ignored --nocapture onde_vai_o_tempo`

    use std::time::Instant;

    fn cronometrar(nome: &str, f: impl FnOnce()) -> (String, u128) {
        let inicio = Instant::now();
        f();
        let ms = inicio.elapsed().as_millis();
        println!("  {:<28} {:>6} ms", nome, ms);
        (nome.to_string(), ms)
    }

    #[test]
    #[ignore]
    fn onde_vai_o_tempo() {
        println!("\nCADA DIAGNÓSTICO, ISOLADO\n");

        let mut tempos = vec![
            cronometrar("prontidão", || {
                let _ = super::super::readiness::analyze();
            }),
            cronometrar("saúde", || {
                let _ = super::super::health::analyze();
            }),
            cronometrar("memória", || {
                let _ = super::super::memory::analyze();
            }),
            cronometrar("pressão", || {
                let _ = super::super::pressao::analyze();
            }),
            cronometrar("monitor", || {
                let _ = super::super::display::analyze();
            }),
            cronometrar("placa de vídeo", || {
                let _ = super::super::gpupref::analyze();
            }),
            cronometrar("config do jogo", || {
                let _ = super::super::configjogo::analyze();
            }),
            cronometrar("esgotamento", || {
                let _ = super::super::exhaustion::analyze();
            }),
            cronometrar("térmico", || {
                let _ = super::super::thermal::analyze();
            }),
            cronometrar("disco", || {
                let _ = super::super::diskspace::scan_para_o_veredito();
            }),
            cronometrar("shaders", || {
                let _ = super::super::shaders::analyze();
            }),
            cronometrar("resizable bar", || {
                let _ = super::super::rbar::analyze();
            }),
            cronometrar("jogos em janela (3.0)", || {
                let _ = super::achados_da_janela();
            }),
            cronometrar("microcódigo (3.0)", || {
                let _ = super::achados_do_microcodigo();
            }),
            cronometrar("eventos de hardware (3.0)", || {
                let _ = super::achados_de_eventos_de_hardware();
            }),
            cronometrar("x3d (3.0)", || {
                let _ = super::achados_do_x3d();
            }),
        ];

        let soma: u128 = tempos.iter().map(|(_, ms)| ms).sum();

        tempos.sort_by_key(|(_, ms)| std::cmp::Reverse(*ms));

        println!("\nOS MAIS CAROS\n");
        for (nome, ms) in tempos.iter().take(4) {
            let fatia = if soma > 0 { ms * 100 / soma } else { 0 };
            println!("  {:<28} {:>6} ms   {:>2}% do total", nome, ms, fatia);
        }

        println!("\n  soma se fosse em série: {} ms", soma);

        let inicio = Instant::now();
        let veredito = super::diagnostico_rapido();
        let real = inicio.elapsed().as_millis();

        println!("  TELA INICIAL DE VERDADE: {} ms", real);
        println!(
            "  ganho da paralelização: {} ms ({} tarefas simultâneas)",
            soma.saturating_sub(real),
            super::limite_de_simultaneos()
        );
        println!(
            "  achados: {} · lacunas: {}\n",
            veredito.achados.len(),
            veredito.lacunas.len()
        );
    }
}

#[cfg(test)]
mod tests_1_9_acoes {
    use super::*;

    #[test]
    fn os_quatro_consertos_que_faltavam_ganharam_botao() {
        let esperados = [
            (Origem::Termico, "teto_no_plano_de_energia", "apply_optimization"),
            (Origem::Firmware, "boot_limits_present", "apply_optimization"),
            (Origem::Prontidao, "plano_de_terceiro", "apply_optimization"),
            (Origem::Memoria, "pagefile_small", "set_automatic_pagefile"),
        ];

        for (origem, id, comando) in esperados {
            let acao = acao_de(origem, id)
                .unwrap_or_else(|| panic!("{:?}/{} ficou sem acao", origem, id));

            assert_eq!(acao.comando, comando, "comando errado para {}", id);

            assert!(
                !acao.rotulo.trim().is_empty(),
                "{} tem botao sem rotulo: o cliente veria um botao mudo",
                id
            );
        }
    }

    /// O que se resolve comprando peça não ganha botão.
    #[test]
    fn achado_de_hardware_continua_sem_botao() {
        for (origem, id) in [
            (Origem::Firmware, "memory_single_channel"),
            (Origem::Memoria, "low_ram"),
            (Origem::Saude, "disk_wear_alto"),
            (Origem::Pressao, "pressao_recorrente"),
        ] {
            assert!(
                acao_de(origem, id).is_none(),
                "{:?}/{} ganhou um botao, e a cura dele nao e software",
                origem,
                id
            );
        }
    }
}

#[cfg(test)]
mod tests_1_8_memoria {
    use super::*;
    use crate::modules::windows::memory::MemoryReport;

    fn nao_medido() -> MemoryReport {
        MemoryReport {
            medido: false,
            total_ram_gb: 0.0,
            available_ram_gb: 0.0,
            committed_gb: 0.0,
            pagefile_automatic: false,
            pagefile_size_gb: 0.0,
            pagefile_peak_gb: 0.0,
            pagefile_location: "não foi possível ler".to_string(),
            findings: Vec::new(),
        }
    }

    /// RAM zero com paginação zero satisfazia o achado mais grave ("Nenhuma paginação, com 0.0 GB de RAM", Critical).
    #[test]
    fn memoria_nao_medida_nao_produz_achado() {
        assert!(
            nao_medido().achados().is_empty(),
            "sem ler a memoria, o produto nao pode concluir nada sobre ela"
        );
    }

    /// O achado de paginação tem botão que ESCREVE: sem medição, nem achado nem botão, travado entre os dois arquivos.
    #[test]
    fn memoria_nao_medida_nao_oferece_botao_que_escreve() {
        let com_acao: Vec<String> = nao_medido()
            .achados()
            .iter()
            .filter(|a| a.acao.is_some())
            .map(|a| a.id.clone())
            .collect();

        assert!(
            com_acao.is_empty(),
            "o produto ofereceu escrever no sistema a partir de medicao que nao \
             aconteceu: {:?}",
            com_acao
        );
    }

    #[test]
    fn paginacao_realmente_desligada_continua_com_achado_e_acao() {
        let relatorio = MemoryReport {
            medido: true,
            total_ram_gb: 8.0,
            available_ram_gb: 2.0,
            committed_gb: 6.0,
            pagefile_automatic: false,
            pagefile_size_gb: 0.0,
            pagefile_peak_gb: 0.0,
            pagefile_location: "nenhum".to_string(),
            findings: crate::modules::windows::memory::diagnosticar(
                8.0, 0.0, 0.0, false, 6.0, 5.0,
            ),
        };

        let achados = relatorio.achados();
        let off = achados
            .iter()
            .find(|a| a.id == "pagefile_off")
            .expect("paginacao desligada de verdade tem que aparecer");

        assert_eq!(off.severity, FindingSeverity::Critical);
        assert!(off.acao.is_some(), "e o botao de consertar tem que estar la");
    }
}

#[cfg(test)]
mod tests_1_8_disco {
    use super::*;
    use crate::modules::windows::diskspace::{DiskReport, Medida};

    fn relatorio(total_bytes: u64, free_bytes: u64, medida: Medida) -> DiskReport {
        let free_percent = if total_bytes > 0 {
            free_bytes as f64 / total_bytes as f64 * 100.0
        } else {
            0.0
        };

        DiskReport {
            drive: "C:".to_string(),
            total_bytes,
            free_bytes,
            free_percent,
            medida_do_espaco: medida,
            pressure: None,
            recoverable_bytes: 0,
            findings: Vec::new(),
        }
    }

    #[test]
    fn disco_nao_medido_nao_produz_achado() {
        let achados = relatorio(0, 0, Medida::NaoConsegui).achados();

        assert!(
            achados.is_empty(),
            "sem medir o disco, o produto nao pode afirmar nada sobre ele — \
             e muito menos com severidade Critical"
        );
    }

    #[test]
    fn disco_realmente_cheio_continua_sendo_achado() {
        let achados = relatorio(500_000_000_000, 0, Medida::Medido).achados();

        assert_eq!(achados.len(), 1, "disco cheio medido tem que aparecer");
        assert_eq!(achados[0].id, "disco_quase_cheio");
        assert_eq!(achados[0].severity, FindingSeverity::Critical);
    }

    #[test]
    fn disco_com_folga_nao_produz_achado() {
        let achados = relatorio(500_000_000_000, 300_000_000_000, Medida::Medido).achados();

        assert!(achados.is_empty());
    }

    #[test]
    fn abaixo_de_dez_gb_livres_o_achado_aparece() {
        let apertado = relatorio(500_000_000_000, 5_000_000_000, Medida::Medido).achados();
        let folgado = relatorio(500_000_000_000, 11_000_000_000, Medida::Medido).achados();

        assert_eq!(apertado.len(), 1);
        assert!(folgado.is_empty());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn achado(
        id: &str,
        origem: Origem,
        severity: FindingSeverity,
        fix: FixLocation,
    ) -> Achado {
        montar(
            origem,
            id.to_string(),
            format!("título de {}", id),
            format!("medida de {}", id),
            String::new(),
            severity,
            fix,
        )
    }

    #[test]
    fn todo_botao_de_conserto_aponta_para_otimizacao_que_existe() {
        // A 2.7 tinha dois botões chamando "power_high_performance", que não existia: o clique dava erro.
        let casos = [
            (Origem::Termico, "teto_no_plano_de_energia"),
            (Origem::Firmware, "boot_limits_present"),
            (Origem::Prontidao, "plano_de_terceiro"),
        ];
        for (origem, id) in casos {
            let acao = acao_de(origem, id).expect("tem botão");
            if acao.comando == "apply_optimization" {
                let alvo = acao.argumento.as_deref().unwrap();
                let spec = crate::modules::windows::catalog::find(alvo)
                    .unwrap_or_else(|| panic!("`{}` aponta para `{}`, que não existe no catálogo", id, alvo));
                assert!(!crate::modules::windows::catalog::retirado(spec.id), "`{}` aponta para item retirado", id);
            }
        }
    }

    #[test]
    fn prioridade_e_desempenho_perdido() {
        let boot = achado("boot_limits_present", Origem::Firmware, FindingSeverity::Critical, FixLocation::Software);
        let xmp = achado("memory_xmp_off", Origem::Firmware, FindingSeverity::Important, FixLocation::Bios);
        let ok = achado("memory_dual_channel", Origem::Firmware, FindingSeverity::Ok, FixLocation::None);
        assert_eq!(prioridade(&boot), Some(Prioridade::P0));
        assert_eq!(prioridade(&xmp), Some(Prioridade::P1));
        assert_eq!(prioridade(&ok), None);

        let r = recuperacao(&[xmp.clone(), boot.clone(), ok.clone()], &[]);
        assert_eq!(r.perdido, Perdido::Sim);
        assert_eq!(r.itens[0], ("boot_limits_present".to_string(), Prioridade::P0));

        assert_eq!(recuperacao(&[xmp, ok.clone()], &[]).perdido, Perdido::Incerto);
        assert_eq!(recuperacao(&[ok], &[]).perdido, Perdido::Nao);
    }

    /// Máquina do dono, 12/08/2026: 7,9 GB num único pente, 9,5 GB prometidos, pico de 8,6 GB de paginação, FiveM
    /// travando o PC, e a tela dizendo memória sem problemas.
    #[test]
    fn a_maquina_que_travava_recebe_veredito_de_memoria() {
        let achados = vec![
            achado(
                "memoria_esgotada_historico",
                Origem::Memoria,
                FindingSeverity::Critical,
                FixLocation::Hardware,
            ),
            achado(
                "over_committed",
                Origem::Memoria,
                FindingSeverity::Critical,
                FixLocation::Hardware,
            ),
            achado(
                "memory_single_channel",
                Origem::Firmware,
                FindingSeverity::Critical,
                FixLocation::Hardware,
            ),
            achado("trim", Origem::Prontidao, FindingSeverity::Ok, FixLocation::None),
        ];

        let v = veredito(&achados, &[]);
        let principal = v.principal.as_ref().unwrap();

        assert_eq!(principal.id, "memoria_esgotada_historico");
        assert_eq!(principal.causa, Causa::Memoria);

        assert!(
            v.corroboracoes.iter().any(|c| c.id == "memory_single_channel"),
            "canal único tem que aparecer junto do esgotamento: é a mesma causa"
        );
        assert!(v.corroboracoes.iter().any(|c| c.id == "over_committed"));
    }

    #[test]
    fn disco_morrendo_vence_memoria() {
        let achados = vec![
            achado(
                "memoria_esgotada_historico",
                Origem::Memoria,
                FindingSeverity::Critical,
                FixLocation::Hardware,
            ),
            achado(
                "disk_wear_0",
                Origem::Saude,
                FindingSeverity::Critical,
                FixLocation::Hardware,
            ),
        ];

        let v = veredito(&achados, &[]);
        assert_eq!(v.principal.unwrap().id, "disk_wear_0");
    }

    #[test]
    fn evidencia_historica_vence_configuracao_observada() {
        let achados = vec![
            achado(
                "memory_xmp_off",
                Origem::Firmware,
                FindingSeverity::Critical,
                FixLocation::Bios,
            ),
            achado(
                "memoria_esgotada_historico",
                Origem::Memoria,
                FindingSeverity::Critical,
                FixLocation::Hardware,
            ),
        ];

        let v = veredito(&achados, &[]);
        assert_eq!(v.principal.unwrap().id, "memoria_esgotada_historico");
    }

    #[test]
    fn critico_sempre_vence_importante() {
        let achados = vec![
            achado(
                "disk_temp_0",
                Origem::Saude,
                FindingSeverity::Important,
                FixLocation::Hardware,
            ),
            achado(
                "over_committed",
                Origem::Memoria,
                FindingSeverity::Critical,
                FixLocation::Hardware,
            ),
        ];

        assert_eq!(veredito(&achados, &[]).principal.unwrap().id, "over_committed");
    }

    #[test]
    fn maquina_sadia_admite_que_esta_tudo_bem_com_numeros() {
        let achados = vec![
            achado("pagefile_ok", Origem::Memoria, FindingSeverity::Ok, FixLocation::None),
            achado(
                "memory_dual_channel",
                Origem::Firmware,
                FindingSeverity::Ok,
                FixLocation::None,
            ),
        ];

        let v = veredito(&achados, &[]);
        assert!(v.principal.is_none());
        assert!(v.frase.contains("Não encontramos"));
        assert!(v.detalhe.contains("medida de pagefile_ok"));
    }

    #[test]
    fn atestado_de_saude_com_lacuna_diz_que_nao_e_completo() {
        let achados = vec![achado(
            "pagefile_ok",
            Origem::Memoria,
            FindingSeverity::Ok,
            FixLocation::None,
        )];
        let lacunas = vec![Lacuna {
            origem: Origem::Saude,
            o_que: "Saúde do disco".to_string(),
            por_que: "Exige executar como administrador.".to_string(),
        }];

        let v = veredito(&achados, &lacunas);
        assert!(v.detalhe.contains("não é um atestado completo"));
        assert_eq!(v.lacunas.len(), 1);
    }

    #[test]
    fn canal_unico_da_memoria_cai_na_causa_memoria() {
        assert_eq!(causa_de(Origem::Firmware, "memory_single_channel"), Causa::Memoria);
        assert_eq!(causa_de(Origem::Firmware, "vbs_running"), Causa::Configuracao);
        assert_eq!(causa_de(Origem::Saude, "disk_wear_0"), Causa::Armazenamento);
        assert_eq!(causa_de(Origem::Saude, "battery_health"), Causa::Configuracao);
    }

    #[test]
    fn nunca_dispara_diagnosticos_demais_de_uma_vez() {
        assert!(limite_de_simultaneos() <= DIAGNOSTICOS_SIMULTANEOS);
        assert!(limite_de_simultaneos() >= 1);
    }

    /// A 0.17 levou o diagnóstico de 31 s para 6 s parando de abrir PowerShell; basta um módulo que mede por segundos
    /// para voltar a meio minuto. Gargalo, rede, FiveM, navegadores e `scan()` completo ficam fora. Pelo fonte, não por
    /// cronômetro: tempo numa esteira compartilhada falha por vizinho barulhento.
    #[test]
    fn o_diagnostico_rapido_nao_chama_quem_mede_por_segundos() {
        let fonte = include_str!("veredito.rs");

        let coleta = fonte
            .split("pub fn coletar_rapido")
            .nth(1)
            .expect("coletar_rapido precisa existir");

        // Só o corpo da função: a explicação acima cita os nomes, e a guarda se acharia sozinha.
        let corpo: String = coleta
            .lines()
            .take_while(|l| !l.starts_with('}'))
            .collect::<Vec<_>>()
            .join("\n");

        // `diskspace::scan()` com parênteses: `scan_para_o_veredito()` continua liberada.
        for caro in [
            "bottleneck::",
            "network::",
            "fivem::",
            "browsers::",
            "diskspace::scan()",
        ] {
            assert!(
                !corpo.contains(caro),
                "`{}` entrou no diagnóstico rápido; ele mede por segundos e a \
                 primeira tela do produto volta a demorar",
                caro
            );
        }
    }

    #[test]
    fn diagnostica_esta_maquina_de_verdade() {
        let inicio = std::time::Instant::now();
        let v = diagnostico_rapido();
        let duracao = inicio.elapsed();

        println!("\n  VEREDITO: {}", v.frase);
        println!("  {}", v.detalhe);
        println!("  DESEMPENHO PERDIDO: {:?} {:?}", v.recuperacao.perdido, v.recuperacao.itens);
        for c in &v.corroboracoes {
            println!("    junto: {} — {}", c.title, c.measured);
        }
        for l in &v.lacunas {
            println!("    não deu para ver: {} — {}", l.o_que, l.por_que);
        }
        println!("  ({} achados em {:?})\n", v.achados.len(), duracao);

        assert!(
            !v.achados.is_empty() || !v.lacunas.is_empty(),
            "o diagnóstico não pode voltar vazio e calado"
        );
    }

    #[test]
    fn uma_tarefa_em_panico_nao_derruba_o_diagnostico() {
        // Um pânico envenenava a tranca e derrubava a primeira tela. O hook de pânico fica silenciado durante o teste,
        // senão aparece um "panicked at" numa suíte que passou.
        let hook_original = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));

        let tarefas: Vec<(Origem, fn() -> Result<Vec<Achado>, String>)> = vec![
            (Origem::Prontidao, || Ok(vec![])),
            (Origem::Conflitos, || panic!("explode de proposito")),
            (Origem::Monitor, || Ok(vec![])),
        ];

        let resultado = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            coletar_em_paralelo(tarefas)
        }));

        std::panic::set_hook(hook_original);

        let (achados, lacunas) = resultado.expect(
            "o pânico de uma tarefa não pode escapar de coletar_em_paralelo",
        );
        let _ = achados;

        assert!(
            lacunas.iter().any(|l| l.por_que.contains("explode")),
            "a tarefa que entrou em pânico não virou lacuna declarada: {:?}",
            lacunas
        );
    }

    #[test]
    fn achado_de_hardware_nunca_ganha_botao() {
        for (origem, id) in [
            (Origem::Memoria, "low_ram"),
            (Origem::Memoria, "memoria_esgotada_historico"),
            (Origem::Memoria, "over_committed"),
            (Origem::Firmware, "memory_single_channel"),
            (Origem::Firmware, "memory_xmp_off"),
            (Origem::Saude, "disk_wear_0"),
            (Origem::Esgotamento, "windows_registrou_esgotamento"),
        ] {
            assert!(
                acao_de(origem, id).is_none(),
                "{} não pode ter botão: não é software que resolve",
                id
            );
        }
    }

    #[test]
    fn todo_botao_do_diagnostico_chama_comando_que_existe() {
        // Botão com comando não registrado compila, passa em tudo e só falha no clique do cliente.
        let fonte = include_str!("veredito.rs");
        let lib = include_str!("../../lib.rs");

        let producao = fonte
            .split("#[cfg(test)]")
            .next()
            .expect("split devolve ao menos um pedaço");

        assert!(
            producao.len() < fonte.len(),
            "não achei onde a produção termina"
        );

        let mut comandos: Vec<&str> = Vec::new();

        // Duas formas: a tabela do `acao_de` (tupla depois da seta) e o achado do monitor, montado à mão. Só a tabela,
        // recortada por linha: o arquivo é CRLF e um padrão com "\n}" nunca casaria.
        let tabela: String = producao
            .lines()
            .skip_while(|l| !l.contains("fn acao_de("))
            .take_while(|l| l.trim_end() != "}")
            .collect::<Vec<_>>()
            .join("
");

        assert!(
            tabela.contains("match (origem, id)"),
            "não achei a tabela do `acao_de` — o formato mudou"
        );

        for (texto, marca) in [(tabela.as_str(), "=> ("), (producao, "comando: ")] {
            for pedaco in texto.split(marca).skip(1) {
                let Some((antes, resto)) = pedaco.split_once('"') else {
                    continue;
                };

                if !antes.trim().is_empty() {
                    continue;
                }

                if let Some(nome) = resto.split('"').next() {
                    comandos.push(nome);
                }
            }
        }

        comandos.sort_unstable();
        comandos.dedup();

        // Um piso: se a varredura parar de enxergar, o teste fica vermelho em vez de uma lista vazia passar calada.
        assert!(
            comandos.len() >= 3,
            "achei só {} comando(s) — o formato do arquivo mudou e esta guarda              parou de enxergar: {:?}",
            comandos.len(),
            comandos
        );

        for comando in comandos {
            assert!(
                lib.contains(&format!("commands::{},", comando)),
                "o diagnóstico oferece um botão que chama `{}`, e esse comando                  não está registrado em lib.rs. O clique do cliente falharia.",
                comando
            );
        }
    }

    #[test]
    fn a_paginacao_desligada_ganha_o_conserto_que_estava_escondido() {
        let acao = acao_de(Origem::Memoria, "pagefile_off").expect("tem conserto");
        assert_eq!(acao.comando, "set_automatic_pagefile");
        assert!(acao.exige_admin);
        assert!(acao.argumento.is_none());

        let trim = acao_de(Origem::Prontidao, "trim").expect("tem conserto");
        assert_eq!(trim.argumento.as_deref(), Some("trim"));
    }

    #[test]
    fn achado_ok_nunca_vira_veredito() {
        let achados = vec![achado(
            "memory_dual_channel",
            Origem::Firmware,
            FindingSeverity::Ok,
            FixLocation::None,
        )];

        assert!(veredito(&achados, &[]).principal.is_none());
    }
}
