// A limpeza do sistema, item a item, com o preço de cada um escrito
//
// O QUE SEPARA ISTO DE UM "LIMPADOR DE PC"
//
// Três coisas, e nenhuma é opcional:
//
// 1. CADA ITEM DIZ O QUE SE PERDE. "Cache de miniaturas · 16 MB" não é
//    informação suficiente para alguém decidir. "As miniaturas são redesenhadas
//    na primeira vez que você abrir cada pasta" é. Um limpador que só mostra
//    tamanho está pedindo uma decisão sobre uma coisa que ele não explicou.
//
// 2. TAMANHO É MEDIDO, E AUSENTE NÃO É ZERO. Pasta que não abriu sai com
//    tamanho desconhecido, e não com "0 B" — que seria o produto afirmando que
//    não há nada ali. É a mesma regra da telemetria, e vale aqui porque o
//    número vira uma decisão de apagar.
//
// 3. O QUE É DO CLIENTE NÃO VEM MARCADO. A lixeira é a única coisa desta lista
//    que contém arquivo que o cliente criou, e apagá-la não tem volta. Ela
//    aparece, com o tamanho, e desmarcada — quem quiser marca.
//
// O QUE NÃO ESTÁ AQUI, E POR QUÊ
//
// PREFETCH. Ele aparece em toda lista de limpeza que existe, e é o contrário de
// lixo: é a anotação de que arquivos cada programa lê ao abrir, que o Windows
// usa para abrir mais rápido na próxima vez. Apagar deixa as próximas aberturas
// mais lentas, e o Windows refaz sozinho em poucos dias. Está em
// `windows::naofazemos`, com o motivo, para quem vier perguntar por que não
// está aqui.
//
// CACHE DE SHADER. Não está nesta lista porque JÁ TEM TELA PRÓPRIA — limpar o
// cache de shader depois de trocar o driver é uma das coisas mais úteis que o
// produto faz, e ela tem contexto (qual placa, qual driver, quando) que uma
// caixinha numa lista de limpeza jogaria fora.
//
// CACHE DE NAVEGADOR. Não está porque não é "lixo": é o que faz os sites que a
// pessoa usa abrirem rápido. Apagar troca alguns megabytes por navegação lenta
// pelos próximos dias, e o navegador tem a própria tela para isso.

use serde::{Deserialize, Serialize};

/// Um alvo da limpeza.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Alvo {
    pub id: &'static str,
    pub nome: &'static str,
    /// O que aquilo é, em uma frase.
    pub o_que_e: &'static str,
    /// O QUE SE PERDE ao apagar. Nunca vazio, nem para os inofensivos: "nada"
    /// é uma resposta, e o cliente precisa lê-la para saber que perguntamos.
    pub custo: &'static str,
    /// Vem marcado.
    ///
    /// Só o que não tem volta fica desmarcado. A regra é uma só: nada que
    /// contenha arquivo do cliente vem marcado por padrão.
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

/// Um alvo com o tamanho que ele tem agora.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlvoMedido {
    pub id: String,
    pub nome: String,
    pub o_que_e: String,
    pub custo: String,
    pub padrao: bool,
    /// Bytes. `None` quando a pasta não pôde ser lida.
    ///
    /// AUSENTE NÃO É ZERO. "0 B" afirma que não há nada ali; ausente diz que
    /// ninguém conseguiu olhar. A diferença decide se vale a pena rodar como
    /// administrador e tentar de novo.
    pub bytes: Option<u64>,
}

/// Os que vêm marcados de fábrica.
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

    /// A regra que separa isto de um limpador de PC: todo item diz o que se
    /// perde, inclusive os que não perdem nada.
    #[test]
    fn todo_alvo_diz_o_que_se_perde() {
        for a in ALVOS {
            assert!(!a.custo.is_empty(), "{} sem custo", a.id);
            assert!(!a.o_que_e.is_empty(), "{} sem explicação", a.id);
            assert!(a.custo.ends_with('.'), "{}: o custo é uma frase", a.id);
        }
    }

    /// A lixeira é a única coisa da lista com arquivo do cliente, e ela NÃO
    /// pode vir marcada. Um limpador que esvazia a lixeira por padrão apaga
    /// arquivo de gente que só queria liberar espaço de temporários.
    #[test]
    fn a_lixeira_nunca_vem_marcada() {
        let lixeira = alvo_por_id("lixeira").expect("lixeira");

        assert!(!lixeira.padrao);
        assert!(lixeira.custo.contains("APAGA DE VEZ"));
        assert!(!marcados_por_padrao().contains(&"lixeira".to_string()));
    }

    /// Os relatórios de erro são o que um técnico usa para achar a causa de um
    /// travamento. Apagá-los por padrão apagaria a prova junto com o lixo.
    #[test]
    fn os_relatorios_de_erro_nao_vem_marcados() {
        assert!(!alvo_por_id("relatorios_de_erro").expect("alvo").padrao);
    }

    /// O prefetch não pode voltar para esta lista por descuido.
    #[test]
    fn o_prefetch_nao_esta_na_limpeza() {
        assert!(
            !ALVOS.iter().any(|a| a.id.contains("prefetch")),
            "o prefetch é anotação de abertura rápida, não lixo — ver windows::naofazemos"
        );
    }
}
