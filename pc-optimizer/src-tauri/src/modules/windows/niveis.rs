// Seguro, Competitivo, Experimental: O QUE VOCÊ ACEITA TROCAR (os perfis dizem para que serve a máquina).
// Encaixados: subir de nível nunca tira nada. Nenhum troca segurança por desempenho, nem o Experimental. O
// Experimental não é aplicar tudo: é a porta do protocolo A/B, um grupo de cada vez, medido (a 2.1.0 foi aplicar
// tudo).

use serde::{Deserialize, Serialize};

use super::catalog::{entra_no_lote, OptimizationSpec, CATALOG, FORA_DO_LOTE};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Nivel {
    Seguro,
    Competitivo,
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

/// Feita dos predicados que já existem (`entra_no_lote`, `FORA_DO_LOTE`, `RiscoDeFps`), e não de uma lista à
/// mão, que envelhece sem ninguém perceber.
pub fn pertence(spec: &OptimizationSpec, nivel: Nivel) -> bool {
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

/// A diferença para o anterior é o que ajuda a decidir, não a lista inteira.
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

/// O Experimental não: aplicar junto dá um número e nenhuma informação sobre o culpado.
pub fn aplica_de_uma_vez(nivel: Nivel) -> bool {
    !matches!(nivel, Nivel::Experimental)
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn cada_nivel_acrescenta_alguma_coisa() {
        assert!(!acrescenta(Nivel::Competitivo).is_empty(), "o Competitivo é igual ao Seguro");
        assert!(
            !acrescenta(Nivel::Experimental).is_empty(),
            "o Experimental é igual ao Competitivo"
        );
    }

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

    /// Se os dois se separarem, "Seguro" e o botão entregam coisas diferentes.
    #[test]
    fn o_seguro_e_exatamente_o_que_o_botao_grande_aplica() {
        let do_lote: Vec<&str> = CATALOG
            .iter()
            .filter(|s| entra_no_lote(s))
            .map(|s| s.id)
            .collect();

        assert_eq!(itens_do_nivel(Nivel::Seguro), do_lote);
    }

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

    #[test]
    fn o_experimental_nao_se_aplica_de_uma_vez() {
        assert!(aplica_de_uma_vez(Nivel::Seguro));
        assert!(aplica_de_uma_vez(Nivel::Competitivo));
        assert!(!aplica_de_uma_vez(Nivel::Experimental));
    }

    #[test]
    fn todo_nivel_promete_e_exige_por_escrito() {
        for nivel in Nivel::TODOS {
            assert!(nivel.promessa().len() >= 120, "{} sem promessa", nivel.nome());
            assert!(!nivel.exigencia().trim().is_empty(), "{} sem exigência", nivel.nome());
        }
    }

    #[test]
    fn a_exigencia_do_experimental_e_medir() {
        let e = Nivel::Experimental.exigencia();

        assert!(e.contains("Medir"), "o Experimental precisa exigir medição: {e}");
        assert!(e.contains("um por vez") || e.contains("grupo"));
    }

    #[test]
    fn o_competitivo_acrescenta_o_que_muda_comportamento() {
        let novos = acrescenta(Nivel::Competitivo);

        assert!(
            novos.contains(&"background_apps_off"),
            "o item que corta aplicativos de segundo plano é o caso típico deste nível"
        );
    }
}
