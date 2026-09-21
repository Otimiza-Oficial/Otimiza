// O que o Otimiza NÃO faz, e por quê
//
// O cliente compara lista com lista. Ele abre um vídeo de "50 tweaks para
// aumentar FPS", vê trinta coisas que o Otimiza não faz, e conclui que o
// produto é fraco. A conclusão é razoável do ponto de vista dele: ninguém tem
// como saber, olhando de fora, que metade daquela lista não faz nada e que um
// quarto dela piora a máquina.
//
// Então o produto passa a dizer. Esta é a lista dos ajustes famosos que o
// Otimiza SE RECUSA a fazer, cada um com o motivo — e o motivo é sempre um
// fato, nunca "não recomendamos".
//
// ─────────────────────────────────────────────────────────────────────────
// AS TRÊS CATEGORIAS, e por que elas não são a mesma coisa
//
// PLACEBO — o ajuste faz exatamente nada. A chave não existe, ou existe e o
//   Windows ignora, ou ela já está naquele valor em toda máquina. Não é
//   perigoso; é perda de tempo vendida como ganho.
//
// REDUNDANTE — o Windows já faz isso sozinho, e melhor. Mexer só tira a
//   decisão de quem tem mais informação que nós.
//
// PREJUDICIAL — o ajuste faz alguma coisa, e a coisa é ruim. É a categoria que
//   mais aparece em lista de internet, porque o efeito costuma ser invisível
//   no dia seguinte e só dói semanas depois.
//
// ─────────────────────────────────────────────────────────────────────────
// A REGRA DESTA LISTA
//
// Entrar aqui exige ser AFIRMÁVEL. Não basta eu achar que não funciona: o
// motivo escrito precisa ser uma coisa que o cliente consegue conferir, ou uma
// consequência que se explica sozinha. "Placebo" sem explicação é só o nosso
// palpite contra o do vídeo — e aí o cliente está escolhendo entre duas
// opiniões, que é exatamente onde ele já estava.
//
// E há uma trava disso logo abaixo: todo item precisa de um motivo com tamanho
// de explicação, não de rótulo.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Natureza {
    /// Não faz nada. Perda de tempo vendida como ganho.
    Placebo,
    /// O Windows já faz, e melhor.
    Redundante,
    /// Faz alguma coisa, e a coisa é ruim.
    Prejudicial,
}

impl Natureza {
    pub fn rotulo(self) -> &'static str {
        match self {
            Natureza::Placebo => "não faz nada",
            Natureza::Redundante => "o Windows já faz",
            Natureza::Prejudicial => "piora",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NaoFazemos {
    pub id: &'static str,
    /// Como o ajuste é chamado nos vídeos e nas listas.
    pub nome: &'static str,
    pub natureza: Natureza,
    /// O motivo, em fato. Nunca "não recomendamos".
    pub porque: &'static str,
}

pub static LISTA: &[NaoFazemos] = &[
    // --- retirados do próprio catálogo na 2.9 (`catalog::RETIRADOS`) ---
    NaoFazemos {
        id: "desligar_uac_firewall",
        nome: "Desligar o Controle de Conta de Usuário (UAC) e o Firewall \"para ganhar FPS\"",
        natureza: Natureza::Prejudicial,
        porque: "Nenhum dos dois trabalha durante o jogo: o UAC só aparece quando um \
                 programa pede para ser administrador, e o Firewall filtra conexões novas \
                 sem custo que o contador de quadros consiga ver. Desligar não devolve um \
                 quadro e deixa qualquer programa mudar o sistema sem perguntar. O Otimiza \
                 tinha os dois no catálogo, marcados como \"sem ganho\", e tirou na 2.9.",
    },
    NaoFazemos {
        id: "nagle",
        nome: "Desligar o algoritmo de Nagle para \"baixar o ping\"",
        natureza: Natureza::Placebo,
        porque: "Nagle junta pedaços pequenos de dados em conexões TCP. A maioria dos jogos \
                 de ação manda a partida por UDP, onde ele não existe, e o ping é distância \
                 até o servidor mais o caminho da internet — nada disso muda no seu PC. O \
                 que o Otimiza faz no lugar é medir a perda de pacote até o servidor do jogo, \
                 que é o que o jogador sente como travada de rede.",
    },
    NaoFazemos {
        id: "servicos_xbox",
        nome: "Desativar os serviços do Xbox",
        natureza: Natureza::Prejudicial,
        porque: "Esses serviços são sob demanda: ficam parados até um jogo da Microsoft \
                 pedir. Parados, não gastam processador nem memória, então desativar não \
                 muda FPS. O que muda é que o Game Pass, as conquistas e o salvamento na \
                 nuvem de vários jogos param de funcionar.",
    },
    NaoFazemos {
        id: "telemetria_por_fps",
        nome: "Desligar a telemetria do Windows para ganhar desempenho",
        natureza: Natureza::Placebo,
        porque: "O serviço de telemetria usa uma fração de processador que nenhum contador \
                 de quadros consegue separar do ruído, e quase sempre fora da partida. É uma \
                 escolha de privacidade, legítima, mas não de desempenho — e o Otimiza é uma \
                 ferramenta de desempenho. Os ajustes de telemetria, Copilot, busca web do \
                 Iniciar, mapas, sincronização e assistência remota saíram na 2.9 por isso.",
    },
    NaoFazemos {
        id: "dns_para_ping",
        nome: "Trocar o DNS para \"diminuir o ping\" no jogo",
        natureza: Natureza::Placebo,
        porque: "O DNS só é consultado para descobrir o endereço do servidor, uma vez, antes \
                 de conectar. Durante a partida o jogo fala direto com o endereço, e o DNS \
                 não participa de nenhum pacote. Um DNS mais rápido encurta o carregamento \
                 de uma página de internet; o ping do jogo continua o mesmo.",
    },
    NaoFazemos {
        id: "hpet_forcado",
        nome: "Forçar o relógio de alta precisão (HPET) pela linha de comando",
        natureza: Natureza::Prejudicial,
        porque: "O Windows escolhe sozinho o temporizador mais barato que a máquina tem, e \
                 o HPET é o mais CARO de ler. Forçá-lo obriga o sistema a pagar esse preço \
                 milhares de vezes por segundo, e o sintoma é engasgo — o contrário do que \
                 a dica promete. O Otimiza faz o inverso: quando encontra o HPET forçado \
                 por outra ferramenta, ele oferece REMOVER, e guarda o valor anterior.",
    },
    NaoFazemos {
        id: "limpador_de_memoria",
        nome: "Limpador de memória em espera, rodando o tempo todo",
        natureza: Natureza::Prejudicial,
        porque: "A memória \"em espera\" não está ocupada: é cache do que o Windows já leu \
                 do disco, e ela é entregue na hora para qualquer programa que peça. \
                 Esvaziá-la a cada poucos segundos joga fora esse cache e obriga o jogo a \
                 reler do disco — que é engasgo de carregamento, exatamente o que o \
                 programa dizia estar consertando. O número no gerenciador de tarefas fica \
                 bonito; a máquina fica pior.",
    },
    NaoFazemos {
        id: "prioridade_tempo_real",
        nome: "Colocar o jogo em prioridade Tempo Real",
        natureza: Natureza::Prejudicial,
        porque: "Tempo real põe o jogo ACIMA de partes do próprio Windows, incluindo o que \
                 atende teclado, mouse e rede. Quando o jogo satura o processador — que é \
                 quando isso supostamente ajudaria — a máquina para de responder. O Otimiza \
                 usa prioridade Alta, que é o degrau abaixo, e tem uma trava no código \
                 impedindo o valor de tempo real de voltar.",
    },
    NaoFazemos {
        id: "desligar_paginacao",
        nome: "Desligar o arquivo de paginação",
        natureza: Natureza::Prejudicial,
        porque: "Com o arquivo de paginação desligado, o programa que pedir memória quando \
                 ela acabar simplesmente fecha — sem aviso, no meio da partida. E o Windows \
                 usa esse arquivo mesmo com RAM sobrando, para tirar do caminho o que \
                 ninguém está usando. A dica troca um engasgo raro por um fechamento.",
    },
    NaoFazemos {
        id: "desativar_todos_os_servicos",
        nome: "Listas de \"desative estes 60 serviços\"",
        natureza: Natureza::Prejudicial,
        porque: "Essas listas circulam há dez anos e não distinguem uma máquina da outra. \
                 Entre os nomes que aparecem nelas estão o Plug and Play, o Agendador de \
                 Tarefas e os serviços de criptografia — sem eles, programa não abre, \
                 impressora some e o Windows para de atualizar. O Otimiza mexe em dois \
                 serviços (SysMain e a indexação de busca), nomeados na tela, e se recusa a tocar no Windows Update e \
                 no BITS por teste do próprio produto.",
    },
    NaoFazemos {
        id: "registry_cleaner",
        nome: "Limpeza de registro",
        natureza: Natureza::Placebo,
        porque: "O registro do Windows é um banco de dados indexado: chaves órfãs não são \
                 lidas e não custam desempenho nenhum. A própria Microsoft não publica \
                 nenhuma ferramenta de limpeza de registro, e a documentação dela não \
                 reconhece o problema. O que essas ferramentas fazem de concreto é apagar \
                 chaves que algum programa ainda usava.",
    },
    NaoFazemos {
        id: "desfragmentar_ssd",
        nome: "Desfragmentar o SSD",
        natureza: Natureza::Prejudicial,
        porque: "Em SSD não existe cabeça de leitura para percorrer distância, então \
                 desfragmentar não acelera nada — e cada passada escreve o disco inteiro de \
                 novo, gastando vida útil à toa. O Windows já trata SSD de outro jeito \
                 sozinho, com TRIM, e o botão \"Otimizar\" dele faz isso e não \
                 desfragmentação.",
    },
    NaoFazemos {
        id: "core_parking_por_terceiro",
        nome: "Ferramentas que \"desligam o estacionamento de núcleos\" fora do plano de energia",
        natureza: Natureza::Redundante,
        porque: "Isso é uma configuração do plano de energia, e o Windows a expõe na árvore \
                 de energia como qualquer outra. O Otimiza a escreve no plano dele, onde ela \
                 pode ser desfeita reativando o plano anterior. Ferramenta de terceiro que \
                 grava direto no registro do sistema deixa a máquina num estado que nem ela \
                 mesma sabe reverter.",
    },
    NaoFazemos {
        id: "desligar_defender_para_fps",
        nome: "Desligar o antivírus para ganhar FPS",
        natureza: Natureza::Prejudicial,
        porque: "O ganho existe e é pequeno: a varredura pesa no carregamento, não no \
                 quadro. O custo é a máquina ficar sem proteção — num público que baixa mod \
                 e executável de servidor. O Otimiza MEDE o impacto do antivírus e mostra, \
                 e nunca cria exclusão nem desliga proteção: a própria Microsoft diz que a \
                 ferramenta de análise dela não serve para sugerir exclusão.",
    },
    NaoFazemos {
        id: "gpu_tweaks_sem_documentacao",
        nome: "Valores \"secretos\" do driver de vídeo tirados de fórum",
        natureza: Natureza::Placebo,
        porque: "O Otimiza escreve cinco ajustes do driver NVIDIA, e todos os cinco estão no \
                 cabeçalho público que a NVIDIA distribui — com o nome conferido no driver \
                 antes de qualquer escrita, e só aceita ajuste que o driver saiba devolver \
                 ao padrão. Valor sem documentação não tem como ser conferido nem revertido, \
                 e um driver que não reconhece o número simplesmente ignora.",
    },
];

/// Quantos de cada natureza. Serve para a tela poder resumir sem contar no
/// TypeScript — contagem no lado da tela é como dois lugares passam a discordar.
pub fn contar(natureza: Natureza) -> usize {
    LISTA.iter().filter(|n| n.natureza == natureza).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A trava da lista: motivo é EXPLICAÇÃO, não rótulo. Sem isso o cliente
    /// está escolhendo entre duas opiniões — a nossa e a do vídeo —, que é
    /// exatamente onde ele já estava antes de abrir o Otimiza.
    #[test]
    fn todo_item_explica_em_vez_de_so_rotular() {
        for n in LISTA {
            assert!(
                n.porque.len() >= 200,
                "`{}` tem motivo curto demais para ser explicação",
                n.id
            );
            assert!(!n.nome.trim().is_empty(), "`{}` sem nome", n.id);
        }
    }

    /// "Não recomendamos" é a frase que não diz nada. Se ela aparecer aqui, a
    /// lista virou opinião.
    #[test]
    fn nenhum_motivo_e_apelo_a_autoridade() {
        for n in LISTA {
            let baixo = n.porque.to_lowercase();

            assert!(
                !baixo.contains("não recomendamos") && !baixo.contains("nao recomendamos"),
                "`{}` apela para a nossa autoridade em vez de dar o fato",
                n.id
            );
            assert!(
                !baixo.contains("todo mundo sabe"),
                "`{}` apela para senso comum",
                n.id
            );
        }
    }

    #[test]
    fn nenhum_id_repetido() {
        let mut ids: Vec<&str> = LISTA.iter().map(|n| n.id).collect();
        let antes = ids.len();
        ids.sort();
        ids.dedup();

        assert_eq!(ids.len(), antes, "há id repetido na lista");
    }

    /// As três naturezas precisam existir de verdade na lista. Uma lista só de
    /// "prejudicial" viraria alarmismo, e uma só de "placebo" esconderia o que
    /// realmente machuca a máquina.
    #[test]
    fn as_tres_naturezas_aparecem() {
        assert!(contar(Natureza::Placebo) > 0);
        assert!(contar(Natureza::Redundante) > 0);
        assert!(contar(Natureza::Prejudicial) > 0);
    }

    #[test]
    fn cada_natureza_tem_rotulo_proprio() {
        let mut rotulos = [
            Natureza::Placebo.rotulo(),
            Natureza::Redundante.rotulo(),
            Natureza::Prejudicial.rotulo(),
        ];
        let antes = rotulos.len();
        rotulos.sort();

        let mut v = rotulos.to_vec();
        v.dedup();
        assert_eq!(v.len(), antes);
    }

    /// COERÊNCIA COM O PRODUTO: se a lista diz que não fazemos uma coisa, o
    /// catálogo não pode estar fazendo. Esta é a trava que impede a página de
    /// marketing de mentir sobre o próprio programa.
    #[test]
    fn o_que_dizemos_que_nao_fazemos_nao_esta_no_catalogo() {
        use crate::modules::windows::catalog::CATALOG;

        // A prioridade de tempo real é o caso concreto e o mais perigoso: há
        // uma trava separada em `gamemode.rs` sobre o mesmo assunto.
        let proibidos = ["Tempo Real", "desfragment"];

        for spec in CATALOG {
            for proibido in proibidos {
                assert!(
                    !spec.name.contains(proibido),
                    "`{}` faz o que a lista diz que não fazemos",
                    spec.id
                );
            }
        }
    }
}
