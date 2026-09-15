// Ajustes que brigam entre si
//
// `conflicts.rs` já existe e procura outra coisa: dois PROGRAMAS disputando a
// mesma função — dois antivírus varrendo, três sobreposições injetando no mesmo
// jogo, dois otimizadores desfazendo a configuração um do outro.
//
// Este módulo procura o problema de dentro de casa: **dois AJUSTES do próprio
// Otimiza que se anulam, se somam mal, ou que ficam sem sentido juntos.**
//
// ─────────────────────────────────────────────────────────────────────────
// POR QUE ISSO IMPORTA, e não é firula
//
// O protocolo A/B testa grupo por grupo e responde "este grupo rendeu aqui". A
// resposta só vale se os grupos forem independentes. Quando dois ajustes de
// grupos diferentes brigam, o teste do segundo mede o efeito do primeiro, e o
// veredito sai trocado — que é pior que não ter veredito nenhum.
//
// E há o caso mais simples e mais caro: o ajuste que só faz sentido se o outro
// estiver de um certo jeito. Aplicado sozinho, ele não faz nada, e o cliente
// pagou por um item da lista que nunca teve efeito.
//
// ─────────────────────────────────────────────────────────────────────────
// A REGRA DE ENTRADA
//
// Um conflito só entra aqui quando dá para DIZER O MECANISMO. "Esses dois
// podem interferir" é palpite; "o segundo escreve na mesma chave que o
// primeiro" é fato. Há trava exigindo mecanismo escrito em todos.
//
// E este módulo NÃO impede nada. Ele avisa, e a decisão continua de quem usa —
// há casos legítimos de querer os dois. Bloquear seria o produto decidindo no
// lugar da pessoa sobre a máquina dela.

use serde::{Deserialize, Serialize};

use super::grupos::{grupo_de, Grupo};

/// Que tipo de briga é.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tipo {
    /// Os dois escrevem no mesmo lugar. O último a rodar ganha, e o resultado
    /// depende da ORDEM — que é a pior espécie de ajuste, porque muda sozinho.
    MesmoLugar,
    /// Um desfaz o efeito do outro. Aplicar os dois é aplicar nenhum.
    SeAnulam,
    /// O segundo só faz sentido com o primeiro. Sozinho, não faz nada.
    DependeDoOutro,
    /// Juntos custam mais do que rendem — cada um por si é defensável.
    JuntosCustamCaro,
}

impl Tipo {
    pub fn rotulo(self) -> &'static str {
        match self {
            Tipo::MesmoLugar => "escrevem no mesmo lugar",
            Tipo::SeAnulam => "um anula o outro",
            Tipo::DependeDoOutro => "um depende do outro",
            Tipo::JuntosCustamCaro => "juntos custam caro",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Conflito {
    pub id: &'static str,
    /// Os dois ajustes, por identificador do catálogo.
    pub um: &'static str,
    pub outro: &'static str,
    pub tipo: Tipo,
    /// O MECANISMO. Não "podem interferir": o que exatamente acontece.
    pub mecanismo: &'static str,
    /// O que fazer. Nunca "escolha um": qual escolher, e por quê.
    pub conselho: &'static str,
}

pub static CONHECIDOS: &[Conflito] = &[
    Conflito {
        id: "hpet_e_boot",
        um: "remove_forced_hpet",
        outro: "clear_boot_limits",
        tipo: Tipo::MesmoLugar,
        mecanismo: "Os dois escrevem na configuração de inicialização pelo `bcdedit`, e os \
                    dois exigem reiniciar. Aplicados na mesma sessão, o resultado final \
                    depende de qual rodou por último — e como os dois só valem depois do \
                    reinício, uma medição feita antes dele mede a máquina de antes, sem \
                    nenhum dos dois.",
        conselho: "Não é problema aplicar os dois: eles escrevem valores diferentes e não \
                   se sobrepõem. O cuidado é com a MEDIÇÃO — reinicie antes de medir, senão \
                   o teste do grupo H vai comparar duas vezes a mesma configuração.",
    },
    Conflito {
        id: "compressao_e_sysmain",
        um: "disable_memory_compression",
        outro: "disable_sysmain",
        tipo: Tipo::JuntosCustamCaro,
        mecanismo: "Os dois tiram muletas de memória ao mesmo tempo. Sem a compressão, o \
                    que não cabe vai para o disco; sem o SysMain, o Windows para de \
                    adiantar leitura do que costuma ser usado. Cada um sozinho é uma troca \
                    defensável em máquina com folga — juntos, numa máquina apertada, \
                    somam-se e o sintoma é engasgo de carregamento no meio da partida.",
        conselho: "Com 16 GB ou menos, aplique no máximo um dos dois e meça. Se o jogo está \
                   num disco mecânico, NÃO desligue o SysMain: é justamente nesse disco que \
                   ele adianta leitura.",
    },
    Conflito {
        id: "prioridade_e_mmcss",
        um: "foreground_priority",
        outro: "mmcss_games",
        tipo: Tipo::MesmoLugar,
        mecanismo: "Os dois mexem em quem ganha processador, por caminhos diferentes: o \
                    primeiro muda a fatia da janela em primeiro plano, e o segundo eleva a \
                    prioridade das tarefas de jogo pelo agendador de multimídia. Aplicados \
                    juntos, o efeito de um esconde o do outro numa medição — e se o FPS \
                    piorar, não dá para saber qual dos dois foi.",
        conselho: "Os dois estão no MESMO grupo do protocolo (B) justamente por isso: eles \
                   são testados juntos ou nenhum, e o veredito é do par. Testar um de cada \
                   vez aqui daria uma resposta que não se confirma.",
    },
    Conflito {
        id: "hags_e_msi",
        um: "gpu_hardware_scheduling",
        outro: "gpu_msi_mode",
        tipo: Tipo::MesmoLugar,
        mecanismo: "Os dois mudam como o trabalho chega à placa de vídeo — um passa a fila \
                    para a própria GPU gerenciar, o outro muda como ela avisa o processador \
                    de que terminou. Os dois exigem reiniciar e os dois são dos que mais \
                    variam entre máquinas. Aplicados juntos e o FPS caindo, o culpado é um \
                    dos dois e não há como saber qual sem desfazer um.",
        conselho: "Estão no mesmo grupo (C) e são testados juntos. Se o grupo C piorar, \
                   desfaça o agendamento por hardware primeiro: ele é o que tem o histórico \
                   de custar quadro em combinação de placa e driver.",
    },
    Conflito {
        id: "plano_e_throttling",
        um: "plano_otimiza",
        outro: "disable_power_throttling",
        tipo: Tipo::DependeDoOutro,
        mecanismo: "A limitação de energia por processo age sobre programas que o Windows \
                    considera de segundo plano. Com o plano OTIMIZA ativo — estado mínimo \
                    em 100% e preferência de desempenho no máximo —, boa parte do que ela \
                    faria já não acontece. Desligá-la sem o plano ativo tem efeito bem \
                    maior do que com ele.",
        conselho: "Aplique o plano primeiro e meça. Se o grupo A já resolveu, a limitação \
                   por processo vira um ajuste sem efeito visível nesta máquina — e saber \
                   disso é melhor que aplicar e achar que ajudou.",
    },
];

/// Os conflitos entre uma lista de ajustes aplicados ou selecionados.
///
/// **Função pura.** Recebe os ids e devolve o que briga entre eles. Não lê nada.
pub fn entre(ids: &[String]) -> Vec<&'static Conflito> {
    CONHECIDOS
        .iter()
        .filter(|c| {
            ids.iter().any(|id| id == c.um) && ids.iter().any(|id| id == c.outro)
        })
        .collect()
}

/// Os conflitos que atravessam grupos do protocolo.
///
/// SÃO OS QUE MAIS IMPORTAM, e por isso têm função própria: o protocolo testa
/// grupo por grupo e afirma "este grupo rendeu". Essa afirmação só vale se os
/// grupos forem independentes — dois ajustes que brigam e moram em grupos
/// diferentes fazem o teste do segundo medir o efeito do primeiro, e o veredito
/// sai trocado.
///
/// Conflito DENTRO do mesmo grupo não tem esse problema: o grupo é aplicado
/// inteiro, e o veredito é do conjunto.
pub fn atravessam_grupos() -> Vec<&'static Conflito> {
    CONHECIDOS
        .iter()
        .filter(|c| {
            let (a, b) = (grupo_de(c.um), grupo_de(c.outro));
            a.is_some() && b.is_some() && a != b
        })
        .collect()
}

/// O par de grupos que um conflito liga, quando ele atravessa.
pub fn grupos_ligados(conflito: &Conflito) -> Option<(Grupo, Grupo)> {
    let a = grupo_de(conflito.um)?;
    let b = grupo_de(conflito.outro)?;

    (a != b).then_some((a, b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::windows::catalog::CATALOG;

    /// Um conflito que aponta para um id que não existe é um aviso que nunca
    /// aparece — e ninguém descobre, porque a ausência de aviso parece paz.
    #[test]
    fn todo_conflito_aponta_para_ajustes_que_existem() {
        for c in CONHECIDOS {
            for id in [c.um, c.outro] {
                assert!(
                    CATALOG.iter().any(|s| s.id == id),
                    "o conflito `{}` cita `{id}`, que não existe no catálogo",
                    c.id
                );
            }
        }
    }

    /// A REGRA DE ENTRADA: mecanismo, não palpite. "Podem interferir" é o que o
    /// mercado escreve quando não sabe.
    #[test]
    fn todo_conflito_diz_o_mecanismo_e_nao_um_palpite() {
        for c in CONHECIDOS {
            assert!(
                c.mecanismo.len() >= 180,
                "`{}` não explica o mecanismo, só rotula",
                c.id
            );

            let baixo = c.mecanismo.to_lowercase();
            assert!(
                !baixo.contains("podem interferir") && !baixo.contains("pode interferir"),
                "`{}` usa a frase que se escreve quando não se sabe o mecanismo",
                c.id
            );
        }
    }

    /// "Escolha um" não é conselho: o cliente já sabia que tinha que escolher.
    #[test]
    fn todo_conselho_diz_qual_escolher_e_por_que() {
        for c in CONHECIDOS {
            assert!(c.conselho.len() >= 100, "`{}` sem conselho de verdade", c.id);

            let baixo = c.conselho.to_lowercase();
            assert!(
                !baixo.starts_with("escolha um"),
                "`{}` devolve a decisão sem ajudar a tomá-la",
                c.id
            );
        }
    }

    #[test]
    fn nenhum_conflito_e_de_um_ajuste_com_ele_mesmo() {
        for c in CONHECIDOS {
            assert_ne!(c.um, c.outro, "`{}` é um ajuste brigando consigo", c.id);
        }
    }

    #[test]
    fn nenhum_par_esta_repetido() {
        let mut pares: Vec<(&str, &str)> = CONHECIDOS
            .iter()
            .map(|c| {
                if c.um < c.outro {
                    (c.um, c.outro)
                } else {
                    (c.outro, c.um)
                }
            })
            .collect();

        let antes = pares.len();
        pares.sort();
        pares.dedup();

        assert_eq!(pares.len(), antes, "há par repetido, com dois conselhos diferentes");
    }

    #[test]
    fn acha_o_conflito_numa_selecao() {
        let selecao = vec![
            "disable_memory_compression".to_string(),
            "disable_sysmain".to_string(),
            "disable_transparency".to_string(),
        ];

        let achados = entre(&selecao);

        assert_eq!(achados.len(), 1);
        assert_eq!(achados[0].id, "compressao_e_sysmain");
    }

    /// Um dos dois sozinho não é conflito. Avisar aí seria alarme falso, e
    /// alarme falso gasta a confiança que o aviso de verdade precisa ter.
    #[test]
    fn um_dos_dois_sozinho_nao_e_conflito() {
        let so_um = vec!["disable_memory_compression".to_string()];
        assert!(entre(&so_um).is_empty());
    }

    #[test]
    fn selecao_vazia_nao_inventa_conflito() {
        assert!(entre(&[]).is_empty());
    }

    /// Os que atravessam grupos são os que quebram o protocolo A/B. Este teste
    /// documenta quais são hoje — se a lista mudar, o texto que explica o
    /// protocolo ao cliente precisa mudar junto.
    ///
    /// Eu tinha escrito DOIS aqui, contando o par `plano_otimiza` +
    /// `disable_power_throttling`. O teste corrigiu: os dois moram no mesmo
    /// grupo A, então o protocolo os aplica juntos e o veredito é do par. A
    /// divisão em grupos já estava certa, e é isso que este teste confirma —
    /// de cinco conflitos conhecidos, só UM escapa dos grupos.
    #[test]
    fn so_um_conflito_atravessa_grupos() {
        let ids: Vec<&str> = atravessam_grupos().iter().map(|c| c.id).collect();

        assert_eq!(
            ids,
            vec!["compressao_e_sysmain"],
            "a lista de conflitos que atravessam grupos mudou; se foi de propósito, o \
             texto que explica o protocolo ao cliente precisa mudar junto"
        );
    }

    /// E o de energia NÃO atravessa, porque a divisão em grupos já o resolveu.
    #[test]
    fn o_par_de_energia_esta_no_mesmo_grupo() {
        let par = CONHECIDOS
            .iter()
            .find(|c| c.id == "plano_e_throttling")
            .expect("continua na lista");

        assert_eq!(grupo_de(par.um), Some(Grupo::Energia));
        assert_eq!(grupo_de(par.outro), Some(Grupo::Energia));
        assert!(grupos_ligados(par).is_none());
    }

    /// E os que ficam dentro do mesmo grupo NÃO atravessam — o grupo é aplicado
    /// inteiro e o veredito é do conjunto, então ali não há veredito trocado.
    #[test]
    fn conflito_dentro_do_mesmo_grupo_nao_atravessa() {
        let dentro = CONHECIDOS
            .iter()
            .find(|c| c.id == "hags_e_msi")
            .expect("o par da placa de vídeo continua na lista");

        assert!(grupos_ligados(dentro).is_none(), "os dois são do grupo C");
        assert_eq!(grupo_de(dentro.um), Some(Grupo::Video));
        assert_eq!(grupo_de(dentro.outro), Some(Grupo::Video));
    }

    #[test]
    fn cada_tipo_tem_rotulo_proprio() {
        let mut rotulos = [
            Tipo::MesmoLugar.rotulo(),
            Tipo::SeAnulam.rotulo(),
            Tipo::DependeDoOutro.rotulo(),
            Tipo::JuntosCustamCaro.rotulo(),
        ]
        .to_vec();

        let antes = rotulos.len();
        rotulos.sort();
        rotulos.dedup();

        assert_eq!(rotulos.len(), antes);
    }
}
