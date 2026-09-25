// Por que o FPS está baixo: suspeitos conhecidos, cada um com uma leitura que o confirma (nasceu de duas
// investigações feitas à mão). Regras: sem leitura, não é suspeito (vira lacuna, nem absolvição nem silêncio);
// ordem pela força da evidência; cada causa diz como confirmar. Correlação com ajuste aplicado é com `regressao.rs`.

use serde::{Deserialize, Serialize};

use super::achados::Confianca;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Suspeito {
    pub id: String,
    pub titulo: String,
    /// Nunca vazio: afirmação sem número medido é o que este produto não faz.
    pub medido: String,
    pub porque: String,
    pub como_confirmar: String,
    pub confianca: Confianca,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Investigacao {
    /// Vazio quando nada foi encontrado, e a tela diz isso com palavras.
    pub suspeitos: Vec<Suspeito>,
    /// Suspeito ausente NÃO significa máquina limpa.
    pub lacunas: Vec<String>,
}

/// 4 GB: o piso em que FiveM de servidor movimentado para de caber (o FPS muda conforme o lugar do mapa).
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

/// Vem primeiro quando existe, fora da ordem por evidência: se o Otimiza pode ser a causa, o cliente lê isso
/// antes dos defeitos da máquina dele.
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

/// Não é "está tudo bem": é "não achei nenhuma destas seis coisas".
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

/// A única função que toca o sistema. `regressao` vem de fora: depende do histórico que vive no estado do app.
#[cfg(target_os = "windows")]
pub fn investigar(regressao: Option<(String, f64)>) -> Investigacao {
    let mut suspeitos = Vec::new();
    let mut lacunas = Vec::new();

    if let Some((jogo, queda)) = &regressao {
        if let Some(s) = suspeito_do_proprio_otimiza(Some((jogo, *queda))) {
            suspeitos.push(s);
        }
    }

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

    let discos = super::discodojogo::analisar();
    let no_hd: Vec<String> = super::discodojogo::em_disco_mecanico(&discos.jogos)
        .iter()
        .map(|o| o.jogo.clone())
        .collect();

    if let Some(s) = suspeito_do_disco(&no_hd) {
        suspeitos.push(s);
    }
    lacunas.extend(discos.lacunas);

    let faixas = super::pcie::analisar();
    if let super::pcie::Faixas::NaoDeuParaLer { .. } = faixas {
        lacunas.push(super::pcie::explicar(&faixas));
    } else if let Some(s) = suspeito_das_faixas(&faixas) {
        suspeitos.push(s);
    }

    match super::firmware::analyze_memory_ou_lacuna() {
        Ok(achados) => {
            let canal_unico = achados.iter().any(|a| a.id == "memory_single_channel");
            let abaixo = achados
                .iter()
                .find(|a| a.id == "memory_xmp_off")
                .map(|a| a.measured.clone());

            // Reusa o texto de `firmware`: duas leituras do mesmo fato acabam dando dois números.
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

    let termico = super::thermal::analyze();
    // `percent_of_max` `None` é não medido (lacuna); `NaoIdentificado` é medido e baixo sem causa (achado de `thermal`).
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

    /// Contra o Windows desta máquina: confere que as seis leituras respondem aqui.
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

    #[test]
    fn faixas_completas_nao_viram_suspeito() {
        assert!(suspeito_das_faixas(&super::super::pcie::Faixas::Completo { largura: 16 }).is_none());
        assert!(suspeito_das_faixas(&super::super::pcie::Faixas::NaoDeuParaLer {
            motivo: "sem driver".to_string()
        })
        .is_none());
    }
}
