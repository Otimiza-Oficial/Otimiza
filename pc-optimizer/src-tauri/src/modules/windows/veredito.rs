// O veredito
//
// POR QUE ESTE ARQUIVO EXISTE
//
// O Otimiza foi testado em duas máquinas reais e falhou nas duas. Não por falta
// de diagnóstico: os módulos tinham medido tudo certo. Falhou porque o cliente
// precisava clicar em dezessete botões espalhados por cinco abas para ver os
// pedaços, e nenhum deles dizia qual era O problema. Numa máquina que travava o
// PC inteiro ao abrir o jogo, a tela dizia "memória e paginação sem problemas".
//
// Este módulo faz a coisa que faltava: recolhe os diagnósticos baratos, traduz
// todos para o mesmo vocabulário, e elege UMA frase. Não um placar, não uma
// nota de 0 a 100 — uma frase com o número que a sustenta.
//
// A REGRA DE ELEIÇÃO, E POR QUE ELA É ASSIM
//
// A ordem não é a intuitiva. Um disco morrendo vence a memória em canal único,
// e não é porque hardware vence software (os dois são hardware). É porque falta
// de memória custa desempenho, e disco morrendo custa os arquivos do cliente —
// e porque toda otimização vendida em cima de um disco que está morrendo é
// trabalho cobrado e perdido. Dizer primeiro a coisa que o produto NÃO pode
// vender é o que sustenta a promessa comercial dele.

use super::achados::{
    peso_confianca, peso_severidade, Acao, Achado, Causa, Confianca, EmAchados, FindingSeverity,
    FixLocation, Lacuna, Origem,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Veredito {
    /// A frase única. É isto que o cliente lê primeiro.
    pub frase: String,
    /// O que foi medido para poder afirmar aquilo.
    pub detalhe: String,
    /// O achado que decidiu a frase. Ausente quando nada foi encontrado.
    pub principal: Option<Achado>,
    /// Outros achados da MESMA causa. É o que junta na tela o canal único e a
    /// memória prometida acima da física, que hoje moram em módulos diferentes.
    pub corroboracoes: Vec<Achado>,
    /// Todos os achados, já ordenados pela regra de eleição.
    pub achados: Vec<Achado>,
    /// O que não deu para verificar. Nunca fica escondido.
    pub lacunas: Vec<Lacuna>,
}

// ------------------------------------------------------------------ eleição

/// Chave de ordenação. Menor vem primeiro; o primeiro é o veredito.
///
/// Lexicográfica de propósito: cada critério só é consultado quando o anterior
/// empata, e a ordem dos critérios é a decisão de produto deste arquivo.
fn peso(a: &Achado) -> (u8, u8, u8, u8) {
    (
        // 1. Severidade. Inegociável.
        peso_severidade(a.severity),
        // 2. Risco de perder arquivo antes de risco de perder desempenho.
        if a.causa == Causa::Armazenamento { 0 } else { 1 },
        // 3. Evidência que persiste vence foto do instante do clique — o
        //    cliente abre o Otimiza com o jogo fechado.
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

/// Elege a frase. **Função pura**: é o que permite testar a decisão com os
/// números de uma máquina real sem tocar em máquina nenhuma.
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
        // Não achar nada é um resultado de primeira classe, com os números que
        // sustentam a afirmação. Admitir que o ganho é zero é regra da casa —
        // inventar um problema para justificar a compra é o oposto do produto.
        None => (
            "Não encontramos causa de travamento nesta máquina".to_string(),
            detalhe_de_maquina_sadia(&ordenados, lacunas),
            Vec::new(),
        ),
    };

    Veredito {
        frase,
        detalhe,
        principal,
        corroboracoes,
        achados: ordenados,
        lacunas: lacunas.to_vec(),
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

// ------------------------------------------- conversão dos módulos existentes

/// A causa por trás de cada achado, por identificador.
///
/// Tabela explícita e não heurística de propósito: é ela que decide o que
/// aparece agrupado na tela, e um agrupamento errado inventa uma relação que
/// não existe. Vale conferir linha a linha numa revisão.
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

        // Firmware cobre memória, arranque, VBS e queda de desempenho sob carga.
        // O canal único da memória precisa cair em `Memoria` — é justamente ele
        // que tem de aparecer junto do "prometido acima do físico".
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

        // Do registro de eventos vêm duas coisas muito diferentes: o
        // esgotamento de memória, que o Windows declarou textualmente, e o
        // programa que parou de responder, que pode ter várias causas. O
        // segundo NÃO pode ser agrupado sob memória — isso afirmaria uma
        // relação que ninguém mediu.
        Origem::Esgotamento => {
            if id == "windows_registrou_esgotamento" {
                Causa::Memoria
            } else {
                Causa::Indefinida
            }
        }

        Origem::Gargalo | Origem::Boot => Causa::Configuracao,
    }
}

/// Achados que vêm de leitura de configuração, sem medir efeito.
///
/// A distinção importa: "XMP desligado" é uma configuração observada, e não a
/// prova de que a máquina trava. Marcar como `Inferido` impede que ele vença um
/// esgotamento de memória registrado pelo próprio Windows.
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

/// O conserto que o Otimiza sabe fazer para cada achado, quando existe.
///
/// A tabela é curta de propósito, e vai continuar curta: a maior parte dos
/// achados deste produto aponta para memória insuficiente, disco morrendo ou
/// configuração de BIOS — nenhum desses tem botão, e fingir que tem seria
/// exatamente a promessa que o Otimiza existe para não fazer.
///
/// O que entra aqui é só o que já existe como comando registrado, testado, e
/// reversível pelo histórico de mudanças.
fn acao_de(origem: Origem, id: &str) -> Option<Acao> {
    let (comando, argumento, rotulo, exige_admin) = match (origem, id) {
        // Devolver a paginação ao Windows. Era o conserto mais escondido do
        // produto: existia como botão dentro de um painel de aba, e o cliente
        // com paginação desligada — que é quem mais precisa dele — nunca
        // chegava lá.
        (Origem::Memoria, "pagefile_off") | (Origem::Memoria, "pagefile_manual") => (
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

        (Origem::Prontidao, "plano_maximo") => (
            "fix_readiness",
            Some("plano_maximo"),
            "Criar o plano de desempenho máximo",
            true,
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

/// Usada tanto pelo relatório completo de firmware quanto pelo diagnóstico
/// rápido da tela inicial, que só recolhe os achados de memória.
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
                // Conflito não tem `fix_location` próprio: dois antivírus se
                // resolvem desinstalando um, que é software.
                montar(
                    Origem::Conflitos,
                    c.id.clone(),
                    c.title.clone(),
                    format!("Encontrados: {}.", c.found.join(", ")),
                    c.advice.clone(),
                    c.severity,
                    FixLocation::Software,
                )
            })
            .collect()
    }
}

// ------------------------------------------------------------ coleta rápida

/// Quantos diagnósticos podem rodar ao mesmo tempo.
///
/// NÃO é um número escolhido por velocidade. Cada diagnóstico dispara um
/// `powershell.exe`, que custa uns 40 MB de memória prometida. Disparar os
/// quatro de uma vez acrescentaria mais de 150 MB de pressão **na máquina que
/// estamos diagnosticando justamente por pressão de memória** — o produto se
/// desmentiria na própria medição. Três é o teto; em máquina fraca, um só.
const DIAGNOSTICOS_SIMULTANEOS: usize = 3;

fn limite_de_simultaneos() -> usize {
    // Mesmo critério que a interface já usa para reduzir animação em máquina
    // fraca: até dois núcleos, nada roda em paralelo.
    let nucleos = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    if nucleos <= 2 {
        1
    } else {
        DIAGNOSTICOS_SIMULTANEOS
    }
}

/// O grupo barato: o que pode rodar sozinho ao abrir o programa.
///
/// Ficam de fora, de propósito, os diagnósticos que custam caro ou que mentem
/// quando rodados na hora errada — o analisador de gargalo, por exemplo, leva
/// de quatro a trinta segundos e, com o PC parado na área de trabalho, devolve
/// "sem carga". Rodá-lo ao abrir produziria lixo com aparência de diagnóstico.
pub fn coletar_rapido() -> (Vec<Achado>, Vec<Lacuna>) {
    type Tarefa = (Origem, fn() -> Result<Vec<Achado>, String>);

    // Ordem deliberada: os caros primeiro. Com um limite de três simultâneos, a
    // fila termina em torno do mais demorado se ele começar cedo, e em torno da
    // soma se ele começar por último. Medido nesta máquina em 12/08/2026:
    // prontidão 4,8 s · saúde 4,5 s · memória 1,1 s · firmware 0,9 s.
    let tarefas: Vec<Tarefa> = vec![
        (Origem::Prontidao, || Ok(super::readiness::analyze().achados())),
        (Origem::Saude, || {
            let relatorio = super::health::analyze();

            // Sem administrador o SMART não é legível. Isso precisa virar
            // lacuna visível, e não silêncio — silêncio aqui é indistinguível
            // de "seu disco está bem".
            if relatorio.needs_admin && relatorio.findings.is_empty() {
                return Err(
                    "Ler a saúde do disco exige executar o Otimiza como administrador."
                        .to_string(),
                );
            }

            Ok(relatorio.achados())
        }),
        (Origem::Memoria, || Ok(super::memory::analyze().achados())),
        // A janela dos últimos dias. Custa uma leitura de arquivo — é o
        // diagnóstico mais barato do produto, e o único que vê o que aconteceu
        // enquanto o cliente jogava, com o Otimiza aberto em segundo plano.
        (Origem::Pressao, || Ok(super::pressao::analyze().achados())),
        // A evidência mais forte que temos, e a mais barata: o Windows já
        // anotou o esgotamento de memória e o programa que travou. Fica no
        // grupo automático porque é justamente o que responde a pergunta do
        // cliente — "por que o PC inteiro congela quando abro o jogo".
        (Origem::Esgotamento, || {
            let relatorio = super::exhaustion::analyze();

            match relatorio.erro.clone() {
                Some(motivo) => Err(motivo),
                None => Ok(relatorio.achados()),
            }
        }),
        (Origem::Firmware, || {
            Ok(achados_de_firmware(&super::firmware::analyze_memory_only()))
        }),
    ];

    // Fila de trabalho, e não lotes fixos: com lotes, um módulo rápido esperaria
    // o lote inteiro terminar antes de o próximo começar, e o diagnóstico
    // passaria a custar a SOMA dos mais lentos de cada lote em vez do mais lento
    // de todos. A primeira versão deste arquivo cometia esse erro e levava 16 s
    // onde 5 bastavam.
    let fila = std::sync::Mutex::new(tarefas.into_iter());
    let coletado = std::sync::Mutex::new((Vec::new(), Vec::new()));

    std::thread::scope(|escopo| {
        for _ in 0..limite_de_simultaneos() {
            escopo.spawn(|| loop {
                let Some((origem, tarefa)) = fila.lock().unwrap().next() else {
                    return;
                };

                let resultado = tarefa();
                let (achados, lacunas) = &mut *coletado.lock().unwrap();

                match resultado {
                    Ok(mut novos) => achados.append(&mut novos),
                    // Diagnóstico que falhou vira lacuna visível, nunca ausência
                    // silenciosa. É a mesma regra que o relatório em PDF segue.
                    Err(motivo) => lacunas.push(Lacuna {
                        origem,
                        o_que: nome_da_origem(origem).to_string(),
                        por_que: motivo,
                    }),
                }
            });
        }
    });

    coletado.into_inner().unwrap()
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
        Origem::Pressao => "Observação dos últimos dias",
    }
}

/// O diagnóstico completo da tela inicial: recolhe e já elege a frase.
pub fn diagnostico_rapido() -> Veredito {
    let (achados, lacunas) = coletar_rapido();
    veredito(&achados, &lacunas)
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

    /// O caso que motivou este arquivo inteiro.
    ///
    /// Máquina do dono, 12/08/2026: 7,9 GB num único pente, 9,5 GB prometidos,
    /// pico de 8,6 GB de paginação. O FiveM travava o PC inteiro. A tela dizia
    /// que a memória estava sem problemas, porque o achado de memória morava
    /// numa aba e o de canal único em outra, e nenhum dos dois era "o" veredito.
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

        // Vence o histórico: é o único que não depende de o jogo estar aberto.
        assert_eq!(principal.id, "memoria_esgotada_historico");
        assert_eq!(principal.causa, Causa::Memoria);

        // E o canal único, que mora em OUTRO módulo, aparece junto. É esta
        // linha que representa o conserto do produto.
        assert!(
            v.corroboracoes.iter().any(|c| c.id == "memory_single_channel"),
            "canal único tem que aparecer junto do esgotamento: é a mesma causa"
        );
        assert!(v.corroboracoes.iter().any(|c| c.id == "over_committed"));
    }

    #[test]
    fn disco_morrendo_vence_memoria() {
        // Não porque hardware vence software — os dois são hardware. Porque
        // falta de memória custa FPS e disco morrendo custa os arquivos do
        // cliente, e otimização vendida sobre disco morrendo é trabalho perdido.
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
        // "XMP desligado" é configuração lida, não prova de travamento. Não
        // pode passar na frente de um esgotamento que já aconteceu.
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
        // Inventar problema para justificar a compra é o oposto do produto.
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
        // A afirmação de que está tudo bem também vem com o que foi medido.
        assert!(v.detalhe.contains("medida de pagefile_ok"));
    }

    #[test]
    fn atestado_de_saude_com_lacuna_diz_que_nao_e_completo() {
        // Silêncio não pode ser indistinguível de aprovação.
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
        // Se este teste quebrar, o canal único volta a aparecer sozinho numa
        // aba de firmware, longe do achado de memória — que foi o defeito.
        assert_eq!(causa_de(Origem::Firmware, "memory_single_channel"), Causa::Memoria);
        assert_eq!(causa_de(Origem::Firmware, "vbs_running"), Causa::Configuracao);
        assert_eq!(causa_de(Origem::Saude, "disk_wear_0"), Causa::Armazenamento);
        assert_eq!(causa_de(Origem::Saude, "battery_health"), Causa::Configuracao);
    }

    #[test]
    fn nunca_dispara_diagnosticos_demais_de_uma_vez() {
        // Se este limite subir sem querer, o diagnóstico passa a acrescentar
        // centenas de MB de pressão na máquina que está sendo diagnosticada
        // por pressão de memória.
        assert!(limite_de_simultaneos() <= DIAGNOSTICOS_SIMULTANEOS);
        assert!(limite_de_simultaneos() >= 1);
    }

    #[test]
    fn diagnostica_esta_maquina_de_verdade() {
        let inicio = std::time::Instant::now();
        let v = diagnostico_rapido();
        let duracao = inicio.elapsed();

        println!("\n  VEREDITO: {}", v.frase);
        println!("  {}", v.detalhe);
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
    fn achado_de_hardware_nunca_ganha_botao() {
        // Nenhum programa acrescenta um pente de memória nem troca um disco. Um
        // botão nesses achados seria prometer o que o produto não cumpre — que
        // é o defeito que ele existe para não ter.
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
