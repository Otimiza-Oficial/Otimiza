// A limpeza, item a item: cada item diz o que se perde; tamanho que não se leu é desconhecido, e não 0 B; o que
// é do cliente (a lixeira) não vem marcado. Prefetch não está aqui (`naofazemos`); cache de shader e de navegador
// têm tela própria.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Alvo {
    pub id: &'static str,
    pub nome: &'static str,
    pub o_que_e: &'static str,
    /// Nunca vazio, nem para os inofensivos: "nada" é uma resposta.
    pub custo: &'static str,
    /// Nada que contenha arquivo do cliente vem marcado por padrão.
    pub padrao: bool,
}

pub const ALVOS: &[Alvo] = &[
    Alvo {
        id: "temporarios",
        nome: "Arquivos temporários",
        o_que_e: "O que instaladores e programas deixaram para trás nas pastas de temporários \
                  do Windows e da sua conta.",
        custo: "Nada. Se algum arquivo estiver em uso por um programa aberto, ele é pulado.",
        padrao: true,
    },
    Alvo {
        id: "windows_update",
        nome: "Cache do Windows Update",
        o_que_e: "Os pacotes de atualização que o Windows já baixou e já instalou.",
        custo: "Se o Windows precisar reinstalar uma dessas atualizações, ele baixa de novo.",
        padrao: true,
    },
    Alvo {
        id: "entregas_otimizadas",
        nome: "Cache de Entrega Otimizada",
        o_que_e: "Pedaços de atualização que o Windows guarda para compartilhar com outros \
                  PCs da mesma rede.",
        custo: "Outro PC da sua rede que fosse pegar a atualização daqui vai baixar da \
                internet. Nada além disso.",
        padrao: true,
    },
    Alvo {
        id: "miniaturas",
        nome: "Cache de miniaturas",
        o_que_e: "As miniaturas que o Explorer desenhou das suas fotos, vídeos e documentos.",
        custo: "As miniaturas são redesenhadas na primeira vez que você abrir cada pasta. \
                Numa pasta com muitas fotos, essa primeira vez fica lenta.",
        padrao: true,
    },
    Alvo {
        id: "relatorios_de_erro",
        nome: "Relatórios de erro do Windows",
        o_que_e: "Os despejos que o Windows guarda quando um programa fecha sozinho.",
        custo: "Você perde os relatórios de travamento que ainda não foram analisados — e é \
                justamente neles que um técnico olha para descobrir POR QUE o programa \
                fechou. Se está investigando um travamento, deixe para depois.",
        padrao: false,
    },
    Alvo {
        id: "lixeira",
        nome: "Lixeira",
        o_que_e: "Os arquivos que você mandou para a lixeira e ainda não esvaziou.",
        custo: "APAGA DE VEZ. É a única coisa desta lista que contém arquivo que você criou, \
                e não tem como desfazer. Confira antes de marcar.",
        padrao: false,
    },
];

pub fn alvo_por_id(id: &str) -> Option<&'static Alvo> {
    ALVOS.iter().find(|a| a.id == id)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlvoMedido {
    pub id: String,
    pub nome: String,
    pub o_que_e: String,
    pub custo: String,
    pub padrao: bool,
    /// `None` quando não se leu: "0 B" afirmaria que não há nada ali.
    pub bytes: Option<u64>,
}

pub fn marcados_por_padrao() -> Vec<String> {
    ALVOS
        .iter()
        .filter(|a| a.padrao)
        .map(|a| a.id.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nao_ha_id_repetido() {
        let mut ids: Vec<&str> = ALVOS.iter().map(|a| a.id).collect();
        let antes = ids.len();
        ids.sort_unstable();
        ids.dedup();

        assert_eq!(ids.len(), antes);
    }

    #[test]
    fn todo_alvo_diz_o_que_se_perde() {
        for a in ALVOS {
            assert!(!a.custo.is_empty(), "{} sem custo", a.id);
            assert!(!a.o_que_e.is_empty(), "{} sem explicação", a.id);
            assert!(a.custo.ends_with('.'), "{}: o custo é uma frase", a.id);
        }
    }

    /// A lixeira é a única com arquivo do cliente: marcada por padrão, apagaria arquivo de quem só queria liberar
    /// temporários.
    #[test]
    fn a_lixeira_nunca_vem_marcada() {
        let lixeira = alvo_por_id("lixeira").expect("lixeira");

        assert!(!lixeira.padrao);
        assert!(lixeira.custo.contains("APAGA DE VEZ"));
        assert!(!marcados_por_padrao().contains(&"lixeira".to_string()));
    }

    /// Os relatórios de erro são o que um técnico usa para achar a causa de um travamento.
    #[test]
    fn os_relatorios_de_erro_nao_vem_marcados() {
        assert!(!alvo_por_id("relatorios_de_erro").expect("alvo").padrao);
    }

    #[test]
    fn o_prefetch_nao_esta_na_limpeza() {
        assert!(
            !ALVOS.iter().any(|a| a.id.contains("prefetch")),
            "o prefetch é anotação de abertura rápida, não lixo — ver windows::naofazemos"
        );
    }
}
