// Os três níveis: Seguro, Competitivo, Experimental
//
// O produto já tinha quatro perfis — PC fraco, Jogos, Trabalho, Privacidade. Eles
// respondem "PARA QUE VOCÊ USA A MÁQUINA", e continuam existindo.
//
// Estes três respondem outra pergunta, e ela é a que o incidente da 2.1.0
// mostrou que faltava: **O QUE VOCÊ ACEITA TROCAR.** Um cliente clicou num
// botão que não perguntou isso, e o FPS dele caiu pela metade.
//
// As duas perguntas são independentes de propósito. "Jogos" diz onde mexer;
// "Competitivo" diz até onde ir.
//
// ─────────────────────────────────────────────────────────────────────────
// OS NÍVEIS SÃO ENCAIXADOS, e isso é a regra que os torna compreensíveis
//
//   Seguro ⊂ Competitivo ⊂ Experimental
//
// Subir de nível NUNCA tira nada — só acrescenta. Sem isso, "Competitivo" seria
// um conjunto diferente e não um passo adiante, e o cliente que subisse teria
// que reler a lista inteira para saber o que perdeu. Há trava para isso.
//
// ─────────────────────────────────────────────────────────────────────────
// O QUE NENHUM DOS TRÊS FAZ
//
// **Nenhum nível troca segurança por desempenho.** Desligar o UAC, o firewall
// ou a virtualização de segurança não está em nível nenhum, nem no
// Experimental — eles continuam disponíveis item a item, com o aviso vermelho,
// porque abrir mão de proteção é decisão consciente do dono da máquina e não
// efeito colateral de escolher um nível numa lista.
//
// E o Experimental NÃO é um botão de aplicar tudo. Ele é a porta de entrada do
// protocolo A/B: os ajustes dele são justamente os que podem custar quadro, e o
// jeito de usá-los é um grupo de cada vez, com medição dos dois lados. Aplicar
// todos de uma vez é exatamente o que causou o incidente.

use serde::{Deserialize, Serialize};

use super::catalog::{entra_no_lote, OptimizationSpec, CATALOG, FORA_DO_LOTE};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Nivel {
    /// Não pode piorar nada. É o que o botão grande aplica.
    Seguro,
    /// Troca comodidade e recurso de fundo por resposta. Ainda sem nada que
    /// possa custar quadro.
    Competitivo,
    /// Aceita o que PODE custar quadro, com a condição de medir.
    Experimental,
}

impl Nivel {
    pub const TODOS: &'static [Nivel] = &[Nivel::Seguro, Nivel::Competitivo, Nivel::Experimental];

    pub fn nome(self) -> &'static str {
        match self {
            Nivel::Seguro => "Seguro",
            Nivel::Competitivo => "Avançado",
            Nivel::Experimental => "Experimental",
        }
    }

    /// O que este nível PROMETE. Sem adjetivo de marketing: o que ele faz.
    pub fn promessa(self) -> &'static str {
        match self {
            Nivel::Seguro => {
                "Só o que não pode piorar nada. Nenhum ajuste daqui tem histórico de custar \
                 quadro em máquina nenhuma, nenhum mexe em segurança, e todos voltam atrás. \
                 É o que o botão \"Otimizar agora\" aplica."
            }
            Nivel::Competitivo => {
                "Tudo do Seguro, mais o que troca comodidade por recurso. Aplicativos que \
                 param de rodar em segundo plano, indexação de busca desligada: coisas que \
                 você vai NOTAR no dia a dia, e que devolvem processador e disco para o \
                 jogo. Continua sem nada que possa custar quadro."
            }
            Nivel::Experimental => {
                "Tudo do Avançado, mais os ajustes que rendem numa máquina e custam \
                 quadro em outra. Eles não vêm com promessa: vêm com um teste. O jeito de \
                 usar este nível é um grupo de cada vez, medindo antes e depois — e foi \
                 aplicar tudo isso de uma vez que derrubou o FPS de um cliente na 2.1.0."
            }
        }
    }

    /// O que este nível EXIGE de quem escolhe. Parte do contrato, e não nota de
    /// rodapé — um nível que pede trabalho e não avisa vira reclamação.
    pub fn exigencia(self) -> &'static str {
        match self {
            Nivel::Seguro => "Nada. Aplique e siga.",
            Nivel::Competitivo => {
                "Aceitar que alguns programas parem de atualizar sozinhos em segundo plano \
                 — mensageiro, música, os aplicativos da Loja. Tudo volta atrás num clique."
            }
            Nivel::Experimental => {
                "Medir. Sem medição dos dois lados, este nível é um chute com passos extras \
                 — e um chute que já custou FPS de cliente. Use pelo painel de grupos, que \
                 aplica um por vez e compara."
            }
        }
    }
}

/// Um ajuste pertence a este nível?
///
/// **Função pura**, e a definição inteira dos níveis mora aqui. Note que ela é
/// escrita em cima de predicados que já existiam — `entra_no_lote`,
/// `FORA_DO_LOTE`, `RiscoDeFps` — em vez de uma quinta lista de identificadores
/// escrita à mão. Lista escrita à mão é o que envelhece sem ninguém perceber.
pub fn pertence(spec: &OptimizationSpec, nivel: Nivel) -> bool {
    // A REGRA QUE VALE PARA OS TRÊS: segurança não entra em nível nenhum.
    if spec.security_tradeoff || !spec.reversible {
        return false;
    }

    match nivel {
        Nivel::Seguro => entra_no_lote(spec),
        Nivel::Competitivo => entra_no_lote(spec) || FORA_DO_LOTE.contains(&spec.id),
        Nivel::Experimental => true,
    }
}

pub fn itens_do_nivel(nivel: Nivel) -> Vec<&'static str> {
    CATALOG
        .iter()
        .filter(|spec| pertence(spec, nivel))
        .map(|spec| spec.id)
        .collect()
}

/// O que ESTE nível acrescenta ao anterior.
///
/// É o que a tela mostra quando a pessoa passa de um para o outro: a lista
/// inteira não ajuda a decidir, a diferença ajuda.
pub fn acrescenta(nivel: Nivel) -> Vec<&'static str> {
    let anterior = match nivel {
        Nivel::Seguro => return itens_do_nivel(Nivel::Seguro),
        Nivel::Competitivo => Nivel::Seguro,
        Nivel::Experimental => Nivel::Competitivo,
    };

    let de_baixo = itens_do_nivel(anterior);

    itens_do_nivel(nivel)
        .into_iter()
        .filter(|id| !de_baixo.contains(id))
        .collect()
}

/// Este nível pode ser aplicado de uma vez só?
///
/// O Experimental não pode, e essa é a lição mais cara deste projeto: os
/// ajustes dele são os que rendem numa máquina e custam noutra, então aplicá-los
/// juntos produz um número e nenhuma informação sobre qual foi o culpado.
pub fn aplica_de_uma_vez(nivel: Nivel) -> bool {
    !matches!(nivel, Nivel::Experimental)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A REGRA QUE TORNA OS NÍVEIS COMPREENSÍVEIS: subir nunca tira nada.
    ///
    /// Sem ela, "Competitivo" seria um conjunto diferente e não um passo
    /// adiante, e quem subisse teria que reler a lista inteira para saber o que
    /// perdeu.
    #[test]
    fn cada_nivel_contem_o_anterior() {
        let seguro = itens_do_nivel(Nivel::Seguro);
        let competitivo = itens_do_nivel(Nivel::Competitivo);
        let experimental = itens_do_nivel(Nivel::Experimental);

        for id in &seguro {
            assert!(competitivo.contains(id), "`{id}` está no Seguro e não no Competitivo");
        }

        for id in &competitivo {
            assert!(
                experimental.contains(id),
                "`{id}` está no Competitivo e não no Experimental"
            );
        }
    }

    /// E cada um acrescenta ALGUMA COISA. Dois níveis com o mesmo conteúdo são
    /// dois nomes para a mesma escolha, e isso engana quem escolhe.
    #[test]
    fn cada_nivel_acrescenta_alguma_coisa() {
        assert!(!acrescenta(Nivel::Competitivo).is_empty(), "o Competitivo é igual ao Seguro");
        assert!(
            !acrescenta(Nivel::Experimental).is_empty(),
            "o Experimental é igual ao Competitivo"
        );
    }

    /// A AFIRMAÇÃO MAIS FORTE DESTE MÓDULO, e ela precisa continuar verdadeira:
    /// nenhum nível troca segurança por desempenho — nem o Experimental.
    #[test]
    fn nenhum_nivel_troca_seguranca_por_desempenho() {
        for nivel in Nivel::TODOS {
            for id in itens_do_nivel(*nivel) {
                let spec = CATALOG.iter().find(|s| s.id == id).unwrap();

                assert!(
                    !spec.security_tradeoff,
                    "`{}` troca segurança e entrou no nível {}",
                    id,
                    nivel.nome()
                );
            }
        }
    }

    /// O Seguro é exatamente o que o botão grande aplica. Se os dois se
    /// separarem, o cliente que escolher "Seguro" recebe uma coisa e o que
    /// clicar no botão recebe outra.
    #[test]
    fn o_seguro_e_exatamente_o_que_o_botao_grande_aplica() {
        let do_lote: Vec<&str> = CATALOG
            .iter()
            .filter(|s| entra_no_lote(s))
            .map(|s| s.id)
            .collect();

        assert_eq!(itens_do_nivel(Nivel::Seguro), do_lote);
    }

    /// Nada que possa custar quadro entra no Seguro nem no Competitivo. É a
    /// trava da 2.1.2 vista pelo outro lado.
    #[test]
    fn nada_que_pode_custar_fps_entra_abaixo_do_experimental() {
        for nivel in [Nivel::Seguro, Nivel::Competitivo] {
            for id in itens_do_nivel(nivel) {
                let spec = CATALOG.iter().find(|s| s.id == id).unwrap();

                assert!(
                    !spec.risco_de_fps.pode_custar(),
                    "`{}` pode custar FPS e entrou no nível {}",
                    id,
                    nivel.nome()
                );
            }
        }
    }

    /// E o Experimental é onde eles ficam — senão o nível não teria razão de
    /// existir.
    #[test]
    fn o_experimental_e_onde_moram_os_que_podem_custar_fps() {
        let experimental = itens_do_nivel(Nivel::Experimental);

        let arriscados: Vec<&str> = CATALOG
            .iter()
            .filter(|s| s.risco_de_fps.pode_custar())
            .map(|s| s.id)
            .collect();

        assert!(!arriscados.is_empty(), "o catálogo não tem nenhum ajuste de risco?");

        for id in arriscados {
            assert!(experimental.contains(&id), "`{id}` não está no Experimental");
        }
    }

    /// O Experimental NÃO é um botão de aplicar tudo. Aplicar todos de uma vez
    /// é literalmente o que derrubou o FPS de um cliente.
    #[test]
    fn o_experimental_nao_se_aplica_de_uma_vez() {
        assert!(aplica_de_uma_vez(Nivel::Seguro));
        assert!(aplica_de_uma_vez(Nivel::Competitivo));
        assert!(!aplica_de_uma_vez(Nivel::Experimental));
    }

    /// Promessa e exigência são contrato, não enfeite. Um nível que pede
    /// trabalho e não avisa vira reclamação.
    #[test]
    fn todo_nivel_promete_e_exige_por_escrito() {
        for nivel in Nivel::TODOS {
            assert!(nivel.promessa().len() >= 120, "{} sem promessa", nivel.nome());
            assert!(!nivel.exigencia().trim().is_empty(), "{} sem exigência", nivel.nome());
        }
    }

    /// A exigência do Experimental precisa mandar medir. Sem isso ele vira o
    /// botão perigoso que este projeto passou uma semana consertando.
    #[test]
    fn a_exigencia_do_experimental_e_medir() {
        let e = Nivel::Experimental.exigencia();

        assert!(e.contains("Medir"), "o Experimental precisa exigir medição: {e}");
        assert!(e.contains("um por vez") || e.contains("grupo"));
    }

    /// Os itens que o Competitivo acrescenta são os de `FORA_DO_LOTE` — os que
    /// mudam comportamento que a pessoa notou e escolheu.
    #[test]
    fn o_competitivo_acrescenta_o_que_muda_comportamento() {
        let novos = acrescenta(Nivel::Competitivo);

        assert!(
            novos.contains(&"background_apps_off"),
            "o item que corta aplicativos de segundo plano é o caso típico deste nível"
        );
    }
}
