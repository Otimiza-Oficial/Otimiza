// Por que o FPS está baixo nesta máquina
//
// ESTE MÓDULO EXISTE PORQUE EU FIZ ISTO À MÃO DUAS VEZES NUMA SEMANA.
//
// Um cliente caiu de 200 para 80-120 FPS depois de otimizar, e eu abri o
// registro do Windows, li a árvore de definições de energia e achei um valor
// que o produto gravava errado. Outro cliente pegava 15 a 20 FPS "dependendo de
// onde eu fico", e eu olhei as especificações, vi 2 GB de memória de vídeo e um
// HD mecânico ao lado de um SSD, e apontei as duas causas.
//
// As duas investigações foram a mesma investigação: uma lista curta de suspeitos
// conhecidos, cada um com um jeito de CONFIRMAR, em ordem de probabilidade. Isso
// é código, e enquanto não for código quem faz é uma pessoa — e essa pessoa não
// escala para cem clientes.
//
// ─────────────────────────────────────────────────────────────────────────
// AS QUATRO REGRAS
//
// 1. CADA SUSPEITO PRECISA DE UMA LEITURA. Nada entra nesta lista por ser
//    "comum" ou por estar num vídeo. Se o produto não consegue medir, o
//    suspeito não aparece — e a ausência dele é dita como lacuna, não como
//    absolvição.
//
// 2. A ORDEM É POR FORÇA DA EVIDÊNCIA, não por gravidade. Um achado que o
//    Windows declarou vale mais que um que o Otimiza inferiu, e o cliente
//    precisa ver primeiro o que ele consegue conferir sozinho.
//
// 3. CADA CAUSA DIZ COMO CONFIRMAR. "Pode ser a memória de vídeo" sem o próximo
//    passo é o que o mercado vende. Aqui cada suspeito carrega a frase do que
//    fazer para ter certeza.
//
// 4. O QUE NÃO SE MEDE NÃO VIRA SUSPEITO E NEM VIRA SILÊNCIO. Vira lacuna, com
//    o motivo — pelo mesmo princípio de todo o resto do produto.
//
// ─────────────────────────────────────────────────────────────────────────
// O QUE NÃO ESTÁ AQUI, e por quê
//
// Não há "causa provável" tirada de correlação. O módulo não diz "seu FPS caiu
// PORQUE você aplicou X": isso quem responde é `regressao.rs`, comparando
// medição com medição. Aqui só entra o que é um fato lido da máquina agora.

use serde::{Deserialize, Serialize};

use super::achados::Confianca;

/// O que se sabe fazer para confirmar um suspeito.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Suspeito {
    /// Identificador estável, para a tela e para o relatório de suporte.
    pub id: String,
    pub titulo: String,
    /// O que foi LIDO desta máquina. Nunca vazio — afirmação sem número medido
    /// é a coisa que este produto não faz.
    pub medido: String,
    /// Por que isso derruba quadro.
    pub porque: String,
    /// Como a pessoa confirma que é isto, sem depender da palavra do Otimiza.
    pub como_confirmar: String,
    pub confianca: Confianca,
}

/// O resultado da investigação.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Investigacao {
    /// Do mais provável para o menos. Vazio quando nada foi encontrado — e aí
    /// a tela precisa dizer isso com essas palavras, não ficar em branco.
    pub suspeitos: Vec<Suspeito>,
    /// O que não deu para verificar. A ausência de suspeito aqui NÃO significa
    /// que a máquina esteja limpa.
    pub lacunas: Vec<String>,
}

// ─── As regras, todas puras ──────────────────────────────────────────────
//
// Cada uma recebe o que foi lido e devolve um suspeito ou nada. Separadas da
// coleta de propósito: é assim que a decisão de produto pode ser provada sem
// máquina, e é assim que eu consigo escrever o teste com os números do cliente
// de verdade em vez de com números inventados.

/// Quanta memória de vídeo o jogo pede num servidor de RP com asset custom.
///
/// Quatro gigabytes. Não é chute de fórum: é o piso em que o FiveM de servidor
/// movimentado para de caber, e abaixo dele o sintoma é sempre o mesmo — o FPS
/// muda conforme o lugar do mapa, porque o que estoura é o que está carregado
/// naquele pedaço da cidade.
pub const VRAM_MINIMA_PARA_RP_GB: f64 = 4.0;

pub fn suspeito_da_vram(vram_gb: Option<f64>) -> Option<Suspeito> {
    let gb = vram_gb?;

    if gb <= 0.0 || gb >= VRAM_MINIMA_PARA_RP_GB {
        return None;
    }

    Some(Suspeito {
        id: "vram_curta".to_string(),
        titulo: "A memória da placa de vídeo é curta para servidor de RP".to_string(),
        medido: format!("{gb:.0} GB de memória de vídeo nesta placa."),
        porque: "Um servidor de RP carrega carro, roupa e prédio que não vêm com o jogo. \
                 Quando isso não cabe na placa, o jogo passa a buscar na memória do \
                 sistema, que é muito mais lenta — e o resultado não é perder alguns \
                 quadros, é engasgo. É a explicação mais comum para \"o FPS muda \
                 dependendo de onde eu fico\": num lugar vazio cabe, no centro não."
            .to_string(),
        como_confirmar: "Abra o jogo, vá a um lugar movimentado e olhe a memória de vídeo \
                         em uso pelo Gerenciador de Tarefas, aba Desempenho, na placa de \
                         vídeo. Se a \"memória de GPU dedicada\" estiver no talo, é isto. \
                         O que resolve é baixar a qualidade de textura — o perfil de jogo \
                         do Otimiza já faz isso em placa abaixo de 6 GB."
            .to_string(),
        confianca: Confianca::Medido,
    })
}

/// O jogo num disco mecânico.
pub fn suspeito_do_disco(jogos_no_hd: &[String]) -> Option<Suspeito> {
    if jogos_no_hd.is_empty() {
        return None;
    }

    Some(Suspeito {
        id: "jogo_em_disco_mecanico".to_string(),
        titulo: "O jogo está num disco mecânico".to_string(),
        medido: format!("Em disco de prato: {}.", jogos_no_hd.join(", ")),
        porque: "O jogo lê textura e modelo o tempo todo enquanto a pessoa anda pelo mapa. \
                 Um disco de prato não entrega isso na velocidade que o jogo pede, e o \
                 sintoma é travada ao virar a esquina. Nenhum ajuste de software conserta \
                 — o limite é o disco."
            .to_string(),
        como_confirmar: "Com o jogo aberto, abra o Gerenciador de Tarefas na aba Desempenho \
                         e olhe o disco onde o jogo está: se ele fica em 100% de tempo \
                         ativo durante as travadas, é isto. O que resolve é passar o jogo \
                         para o SSD."
            .to_string(),
        confianca: Confianca::Medido,
    })
}

/// A placa com menos faixas do que suporta.
pub fn suspeito_das_faixas(faixas: &super::pcie::Faixas) -> Option<Suspeito> {
    let super::pcie::Faixas::Estreito { atual, maxima } = faixas else {
        return None;
    };

    Some(Suspeito {
        id: "pcie_estreito".to_string(),
        titulo: "A placa de vídeo está com menos faixas do que suporta".to_string(),
        medido: format!("{atual} faixas de PCI Express em uso, de {maxima} que a placa tem."),
        porque: "As faixas são o caminho entre a placa e o processador. Com menos faixas, \
                 tudo que o jogo manda para a placa passa por um cano mais estreito. É \
                 físico: não há ajuste de software que resolva."
            .to_string(),
        como_confirmar: "Confira no manual da placa-mãe quais slots dividem faixas com os \
                         encaixes de SSD M.2 — é a causa mais comum. Depois disso, verifique \
                         se a placa está no slot de cima e bem encostada, e se não há cabo \
                         de extensão no caminho."
            .to_string(),
        confianca: Confianca::Medido,
    })
}

/// A memória abaixo da velocidade do próprio pente, ou em canal único.
pub fn suspeito_da_memoria(canal_unico: bool, abaixo_do_nominal: Option<(u32, u32)>) -> Option<Suspeito> {
    let mut partes = Vec::new();

    if let Some((atual, nominal)) = abaixo_do_nominal {
        partes.push(format!("a memória roda a {atual} MHz e os pentes são de {nominal} MHz"));
    }

    if canal_unico {
        partes.push("a memória está em canal único".to_string());
    }

    if partes.is_empty() {
        return None;
    }

    Some(Suspeito {
        id: "memoria_abaixo_do_que_tem".to_string(),
        titulo: "A memória não está entregando o que ela tem".to_string(),
        medido: format!("Nesta máquina, {}.", partes.join(" e ")),
        porque: "Jogo preso no processador — e o FiveM é o caso extremo disso — depende da \
                 velocidade com que a memória responde. Canal único corta essa entrega pela \
                 metade, e memória abaixo da velocidade nominal é desempenho comprado e não \
                 usado."
            .to_string(),
        como_confirmar: "A velocidade se liga na BIOS, no perfil XMP (Intel) ou EXPO (AMD) — \
                         e é uma opção só. O canal único se resolve mudando os pentes de \
                         encaixe, seguindo o manual da placa-mãe: em placa de quatro \
                         encaixes, costuma ser o segundo e o quarto. Nada disso é \
                         atualização de BIOS, e nada disso o Otimiza faz sozinho."
            .to_string(),
        confianca: Confianca::Medido,
    })
}

/// O processador ou a placa segurados por temperatura ou por energia.
pub fn suspeito_do_limite(causa: Option<&str>) -> Option<Suspeito> {
    let causa = causa?;

    Some(Suspeito {
        id: "limite_ativo".to_string(),
        titulo: "Alguma coisa está segurando a frequência".to_string(),
        medido: format!("O Windows registra um limite ativo: {causa}."),
        porque: "Com um limite ativo, a peça não chega à frequência que ela alcança. O FPS \
                 fica abaixo do que a máquina daria, e piora conforme ela esquenta — por \
                 isso o jogo começa bom e vai caindo."
            .to_string(),
        como_confirmar: "Jogue por vinte minutos e veja se o FPS cai com o tempo. Se cair, é \
                         temperatura: limpeza do cooler e pasta térmica resolvem mais que \
                         qualquer ajuste de software. O Otimiza NUNCA desliga proteção \
                         térmica — a proteção é o que impede a peça de se danificar."
            .to_string(),
        confianca: Confianca::Declarado,
    })
}

/// O que o produto acabou de fazer, quando ele mesmo é o suspeito.
///
/// ESTE VEM PRIMEIRO QUANDO EXISTE, e é a única regra de ordem que não segue a
/// força da evidência. O motivo é comercial e é honesto: se o Otimiza pode ter
/// sido a causa, o cliente precisa ler isso antes de qualquer outra coisa, e não
/// depois de uma lista de defeitos da máquina dele.
pub fn suspeito_do_proprio_otimiza(piorou: Option<(&str, f64)>) -> Option<Suspeito> {
    let (jogo, queda_pct) = piorou?;

    Some(Suspeito {
        id: "foi_o_otimiza".to_string(),
        titulo: "Pode ter sido o próprio Otimiza".to_string(),
        medido: format!(
            "O {jogo} está {:.0}% abaixo do que era antes das otimizações, medido nesta \
             máquina.",
            queda_pct.abs()
        ),
        porque: "As medições de antes e de depois foram feitas aqui, e a de depois é pior. \
                 Elas vêm de sessões diferentes, então isso não é prova — mas é forte o \
                 bastante para ser o primeiro suspeito, e não o último."
            .to_string(),
        como_confirmar: "Em Otimizações, clique em \"Voltar ao meu plano\" e depois em \
                         \"Desfazer tudo\", jogue de novo e compare. Se o número voltar, \
                         fomos nós — e aí dá para religar um grupo de cada vez para achar \
                         qual ajuste foi."
            .to_string(),
        confianca: Confianca::Historico,
    })
}

/// Monta a lista na ordem certa.
///
/// **Função pura**, e a ordem é a decisão de produto: o Otimiza primeiro quando
/// ele é suspeito, depois o que é físico e a pessoa consegue conferir, depois o
/// resto.
pub fn ordenar(suspeitos: Vec<Suspeito>) -> Vec<Suspeito> {
    let peso = |s: &Suspeito| match s.id.as_str() {
        "foi_o_otimiza" => 0,
        "limite_ativo" => 1,
        "vram_curta" => 2,
        "jogo_em_disco_mecanico" => 3,
        "pcie_estreito" => 4,
        "memoria_abaixo_do_que_tem" => 5,
        _ => 9,
    };

    let mut ordenados = suspeitos;
    ordenados.sort_by_key(peso);
    ordenados
}

/// A frase de quando não se achou nada.
///
/// NÃO é "está tudo bem": é "não achei nenhuma destas seis coisas". A diferença
/// importa porque a lista é curta de propósito, e o cliente precisa saber o que
/// foi procurado.
pub fn nada_encontrado(lacunas: usize) -> String {
    let ressalva = if lacunas > 0 {
        format!(
            " E {lacunas} verificação{} não pôde ser feita nesta máquina, então a lista \
             acima não é completa.",
            if lacunas == 1 { "" } else { "ões" }
        )
    } else {
        String::new()
    };

    format!(
        "Nenhuma das causas conhecidas de FPS baixo foi encontrada aqui: memória de vídeo \
         curta, jogo em disco mecânico, faixas de PCI Express estreitas, memória abaixo da \
         velocidade nominal ou em canal único, e limite de temperatura ou energia ativo. \
         Isso não quer dizer que o FPS esteja bom — quer dizer que o motivo não é nenhum \
         destes cinco.{ressalva}"
    )
}

/// Junta tudo, lendo a máquina.
///
/// A ÚNICA função deste módulo que toca o sistema. Todas as regras acima são
/// puras de propósito: é assim que elas podem ser provadas com os números de um
/// cliente de verdade, sem depender de a esteira ter a máquina dele.
///
/// `regressao` entra de fora em vez de ser lida aqui porque ela depende do
/// histórico de mudanças, que vive atrás do estado do aplicativo — e porque o
/// comando que chama isto já tem esse número na mão.
#[cfg(target_os = "windows")]
pub fn investigar(regressao: Option<(String, f64)>) -> Investigacao {
    let mut suspeitos = Vec::new();
    let mut lacunas = Vec::new();

    if let Some((jogo, queda)) = &regressao {
        if let Some(s) = suspeito_do_proprio_otimiza(Some((jogo, *queda))) {
            suspeitos.push(s);
        }
    }

    // --- memória de vídeo
    let vram = super::bottleneck::vram_total_gb();
    if vram <= 0.0 {
        lacunas.push(
            "Não deu para ler quanta memória a placa de vídeo tem, então a causa mais \
             comum de FPS que muda conforme o lugar do mapa não pôde ser verificada."
                .to_string(),
        );
    } else if let Some(s) = suspeito_da_vram(Some(vram)) {
        suspeitos.push(s);
    }

    // --- o jogo no disco mecânico
    let discos = super::discodojogo::analisar();
    let no_hd: Vec<String> = super::discodojogo::em_disco_mecanico(&discos.jogos)
        .iter()
        .map(|o| o.jogo.clone())
        .collect();

    if let Some(s) = suspeito_do_disco(&no_hd) {
        suspeitos.push(s);
    }
    lacunas.extend(discos.lacunas);

    // --- faixas do PCI Express
    let faixas = super::pcie::analisar();
    if let super::pcie::Faixas::NaoDeuParaLer { .. } = faixas {
        lacunas.push(super::pcie::explicar(&faixas));
    } else if let Some(s) = suspeito_das_faixas(&faixas) {
        suspeitos.push(s);
    }

    // --- memória do sistema
    match super::firmware::analyze_memory_ou_lacuna() {
        Ok(achados) => {
            let canal_unico = achados.iter().any(|a| a.id == "memory_single_channel");
            let abaixo = achados
                .iter()
                .find(|a| a.id == "memory_xmp_off")
                .map(|a| a.measured.clone());

            // O número vem do texto que `firmware` já montou, e não de uma
            // segunda leitura: duas leituras da mesma coisa é como o produto
            // acaba dizendo dois números diferentes para o mesmo fato.
            let par = abaixo.as_deref().and_then(dois_numeros);

            if let Some(s) = suspeito_da_memoria(canal_unico, par) {
                suspeitos.push(s);
            }
        }
        Err(erro) => lacunas.push(format!(
            "Não deu para ler a memória desta máquina, então canal único e velocidade \
             abaixo do nominal não foram verificados: {erro}"
        )),
    }

    // --- limite térmico ou elétrico
    let termico = super::thermal::analyze();
    // `percent_of_max` em `None` é NÃO CONSEGUI MEDIR, e é isso que vira
    // lacuna. `Culprit::NaoIdentificado` é outra coisa: a frequência foi medida,
    // está baixa, e nenhuma causa conhecida explica — isso é um achado, e ele
    // continua com `thermal`, que tem os números para descrevê-lo.
    if termico.percent_of_max.is_none() {
        lacunas.push(
            "Não deu para medir a frequência do processador nesta máquina, então não foi \
             possível verificar se temperatura ou energia estão segurando ela."
                .to_string(),
        );
    }

    let causa = match termico.culprit {
        super::thermal::Culprit::Calor => Some("limite de temperatura"),
        super::thermal::Culprit::LimiteEletrico => Some("limite elétrico"),
        super::thermal::Culprit::PlanoDeEnergia => Some("teto no plano de energia"),
        _ => None,
    };

    if let Some(s) = suspeito_do_limite(causa) {
        suspeitos.push(s);
    }

    Investigacao { suspeitos: ordenar(suspeitos), lacunas }
}

#[cfg(not(target_os = "windows"))]
pub fn investigar(_regressao: Option<(String, f64)>) -> Investigacao {
    Investigacao::default()
}

/// Os dois primeiros números de um texto, na ordem em que aparecem.
///
/// **Função pura.** Serve para reaproveitar a frase que `firmware` já montou —
/// "Rodando a 2133 MHz; o pente é de 3200 MHz" — em vez de ler o hardware de
/// novo. Duas leituras da mesma coisa é como um produto acaba mostrando dois
/// números diferentes para o mesmo fato em duas telas.
pub fn dois_numeros(texto: &str) -> Option<(u32, u32)> {
    let mut achados = texto
        .split(|c: char| !c.is_ascii_digit())
        .filter(|p| !p.is_empty())
        .filter_map(|p| p.parse::<u32>().ok());

    Some((achados.next()?, achados.next()?))
}

#[cfg(test)]
mod tests {
    use super::*;


    /// Roda a investigação inteira contra o Windows desta máquina. Não é teste
    /// de lógica — é a conferência de que as seis leituras respondem aqui.
    #[test]
    #[ignore = "toca o Windows desta máquina"]
    fn investiga_esta_maquina_de_verdade() {
        let r = investigar(None);

        println!("--- suspeitos: {}", r.suspeitos.len());
        for s in &r.suspeitos {
            println!("[{}] {}", s.id, s.titulo);
            println!("    medido: {}", s.medido);
            println!("    confirmar: {}", s.como_confirmar);
        }

        println!("--- lacunas: {}", r.lacunas.len());
        for l in &r.lacunas {
            println!("  · {l}");
        }

        if r.suspeitos.is_empty() {
            println!("{}", nada_encontrado(r.lacunas.len()));
        }
    }

    #[test]
    fn le_os_dois_numeros_da_frase_do_firmware() {
        assert_eq!(
            dois_numeros("Rodando a 2133 MHz; o pente é de 3200 MHz."),
            Some((2133, 3200))
        );
        assert_eq!(dois_numeros("sem número nenhum"), None);
        assert_eq!(dois_numeros("só 1600"), None);
    }

    /// Os números do cliente de verdade: GTX 770 de 2 GB.
    #[test]
    fn dois_gigas_de_vram_viram_suspeito() {
        let s = suspeito_da_vram(Some(2.0)).expect("2 GB precisa virar suspeito");

        assert_eq!(s.id, "vram_curta");
        assert!(s.medido.contains('2'));
        assert!(
            s.como_confirmar.contains("Gerenciador de Tarefas"),
            "o cliente precisa conseguir conferir sozinho"
        );
    }

    #[test]
    fn placa_com_folga_nao_vira_suspeito() {
        assert!(suspeito_da_vram(Some(8.0)).is_none());
        assert!(suspeito_da_vram(Some(4.0)).is_none());
    }

    /// Não conseguir ler a memória de vídeo não vira acusação NEM absolvição:
    /// vira nada, e a lacuna é dita em outro lugar.
    #[test]
    fn vram_ilegivel_nao_vira_suspeito() {
        assert!(suspeito_da_vram(None).is_none());
        assert!(suspeito_da_vram(Some(0.0)).is_none());
    }

    #[test]
    fn a_memoria_junta_os_dois_problemas_numa_frase_so() {
        let s = suspeito_da_memoria(true, Some((2133, 3200))).expect("dois problemas");

        assert!(s.medido.contains("2133") && s.medido.contains("3200"));
        assert!(s.medido.contains("canal único"));
        assert!(s.como_confirmar.contains("XMP") && s.como_confirmar.contains("EXPO"));
    }

    #[test]
    fn memoria_no_lugar_nao_vira_suspeito() {
        assert!(suspeito_da_memoria(false, None).is_none());
    }

    /// A ordem é a decisão de produto mais importante do módulo: se o Otimiza
    /// pode ter sido a causa, o cliente lê isso ANTES da lista de defeitos da
    /// máquina dele.
    #[test]
    fn quando_o_otimiza_e_suspeito_ele_vem_primeiro() {
        let lista = ordenar(vec![
            suspeito_da_vram(Some(2.0)).unwrap(),
            suspeito_do_disco(&["GTA V".to_string()]).unwrap(),
            suspeito_do_proprio_otimiza(Some(("FiveM.exe", -45.0))).unwrap(),
        ]);

        assert_eq!(lista[0].id, "foi_o_otimiza");
    }

    #[test]
    fn o_suspeito_do_otimiza_manda_desfazer_e_medir_de_novo() {
        let s = suspeito_do_proprio_otimiza(Some(("FiveM.exe", -45.0))).unwrap();

        assert!(s.medido.contains("45"));
        assert!(s.como_confirmar.contains("Desfazer tudo"));
        assert!(
            s.porque.contains("isso não é prova"),
            "a ressalva precisa estar lá: as medições vêm de sessões diferentes"
        );
    }

    #[test]
    fn sem_regressao_o_otimiza_nao_se_acusa() {
        assert!(suspeito_do_proprio_otimiza(None).is_none());
    }

    /// O limite térmico nunca pode virar conselho de desligar proteção.
    #[test]
    fn o_limite_termico_nunca_manda_desligar_protecao() {
        let s = suspeito_do_limite(Some("limite de temperatura")).unwrap();

        assert!(s.como_confirmar.contains("NUNCA desliga proteção térmica"));
        assert!(!s.como_confirmar.to_lowercase().contains("desative o limite"));
    }

    /// "Nada encontrado" não pode soar como "está tudo bem".
    #[test]
    fn nada_encontrado_nao_e_atestado_de_saude() {
        let frase = nada_encontrado(0);

        assert!(frase.contains("não quer dizer que o FPS esteja bom"));
        assert!(frase.contains("memória de vídeo"), "precisa dizer o que foi procurado");
    }

    #[test]
    fn com_lacuna_a_lista_se_declara_incompleta() {
        let frase = nada_encontrado(2);
        assert!(frase.contains("não é completa"));
    }

    /// Toda causa precisa dizer como confirmar. Sem isso é o que o mercado
    /// vende: um alarme sem próximo passo.
    #[test]
    fn toda_causa_diz_como_confirmar_e_o_que_foi_medido() {
        let todos = vec![
            suspeito_da_vram(Some(2.0)).unwrap(),
            suspeito_do_disco(&["GTA V".to_string()]).unwrap(),
            suspeito_das_faixas(&super::super::pcie::Faixas::Estreito { atual: 4, maxima: 16 })
                .unwrap(),
            suspeito_da_memoria(true, None).unwrap(),
            suspeito_do_limite(Some("limite de energia")).unwrap(),
            suspeito_do_proprio_otimiza(Some(("x.exe", -30.0))).unwrap(),
        ];

        for s in todos {
            assert!(!s.medido.trim().is_empty(), "`{}` sem nada medido", s.id);
            assert!(
                s.como_confirmar.len() >= 60,
                "`{}` não diz como confirmar",
                s.id
            );
            assert!(!s.porque.trim().is_empty(), "`{}` não diz por que custa quadro", s.id);
        }
    }

    /// As faixas completas não viram suspeito — nem a geração baixa em repouso,
    /// que `pcie::julgar` já trata.
    #[test]
    fn faixas_completas_nao_viram_suspeito() {
        assert!(suspeito_das_faixas(&super::super::pcie::Faixas::Completo { largura: 16 }).is_none());
        assert!(suspeito_das_faixas(&super::super::pcie::Faixas::NaoDeuParaLer {
            motivo: "sem driver".to_string()
        })
        .is_none());
    }
}
