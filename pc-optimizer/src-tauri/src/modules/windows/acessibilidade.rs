// Teclas de acessibilidade ligadas sem querer
//
// O PROBLEMA
//
// A Filtragem de Teclas ignora toques breves e repetidos. Ela existe para quem
// tem tremor nas mãos, e para essa pessoa é essencial. Para quem ligou sem
// querer, é um PC que "come letra" e demora a responder — na máquina de
// desenvolvimento o `DelayBeforeAcceptance` está em 1000 ms, ou seja, um segundo
// inteiro antes de a tecla ser aceita.
//
// E liga sozinha: segurar o Shift direito por oito segundos aciona o atalho. Em
// jogo, segurar Shift é agachar, correr, andar devagar. O cliente aperta Shift
// por oito segundos sem perceber, aparece uma caixa que ele fecha no reflexo, e
// a partir dali o teclado responde errado — sem que ele jamais associe uma coisa
// à outra.
//
// As Teclas de Aderência (StickyKeys) são o mesmo caso, com o Shift apertado
// cinco vezes seguidas.
//
// POR QUE NÃO DÁ PARA ESCREVER UM NÚMERO FIXO
//
// O estado mora num campo de bits, e só o bit 0 diz se o recurso está LIGADO. Os
// outros guardam preferências do usuário: se o atalho existe, se toca som, se
// mostra aviso. Gravar o "122" que os tutoriais repetem desligaria o recurso e
// apagaria junto as escolhas da pessoa.
//
// Então aqui é leitura, mudança de um bit e escrita de volta — e a reversão
// devolve o número exato que estava lá.

use crate::modules::changelog::{ChangeRecord, PreviousValue};

/// As três chaves que o atalho de teclado liga sem querer.
///
/// `MouseKeys` fica de fora de propósito: mover o ponteiro pelo teclado numérico
/// não atrapalha quem não usa, e desligar não devolve desempenho nenhum.
pub const CHAVES: &[(&str, &str)] = &[
    (r"Control Panel\Accessibility\Keyboard Response", "Filtragem de Teclas"),
    (r"Control Panel\Accessibility\StickyKeys", "Teclas de Aderência"),
    (r"Control Panel\Accessibility\ToggleKeys", "Teclas Alternadas"),
];

/// O bit que diz se o recurso está em uso. Vale para as três chaves.
const BIT_LIGADO: u32 = 1;

/// Se o campo de bits indica recurso ligado.
pub fn esta_ligado(flags: &str) -> Option<bool> {
    let valor: u32 = flags.trim().parse().ok()?;
    Some(valor & BIT_LIGADO != 0)
}

/// O mesmo campo de bits com o recurso desligado, preservando o resto.
///
/// Devolve `None` quando já está desligado — não se escreve no registro do
/// cliente para gravar o valor que já estava lá.
pub fn desligado(flags: &str) -> Option<String> {
    let valor: u32 = flags.trim().parse().ok()?;

    if valor & BIT_LIGADO == 0 {
        return None;
    }

    Some((valor & !BIT_LIGADO).to_string())
}

/// Desliga as três, acumulando no histórico do chamador o que já foi escrito.
///
/// **Recebe o vetor em vez de devolvê-lo de propósito.** Devolvendo
/// `Result<Vec<_>>`, uma falha na segunda chave descartaria o registro da
/// primeira — que JÁ foi gravada no registro do cliente — e a reversão
/// automática não teria como desfazê-la. O sistema ficaria pela metade sem
/// rastro, exatamente o que a regra nº 2 do `mod.rs` proíbe.
///
/// Chave ausente ou ilegível é pulada em silêncio: instalações do Windows
/// variam, e não conseguir ler uma delas não é motivo para falhar as outras.
pub fn desligar(mudancas: &mut Vec<ChangeRecord>) -> Result<(), String> {
    for (caminho, _) in CHAVES {
        let Ok(PreviousValue::Text(atual)) = super::registry::read("HKCU", caminho, "Flags") else {
            continue;
        };

        let Some(novo) = desligado(&atual) else {
            continue;
        };

        let anterior = super::registry::set_string("HKCU", caminho, "Flags", &novo)?;

        mudancas.push(ChangeRecord::RegistryValue {
            hive: "HKCU".to_string(),
            path: caminho.to_string(),
            name: "Flags".to_string(),
            previous: anterior,
        });
    }

    Ok(())
}

/// Quais das três estão ligadas agora.
pub fn ligadas() -> Vec<&'static str> {
    CHAVES
        .iter()
        .filter(|(caminho, _)| {
            matches!(
                super::registry::read("HKCU", caminho, "Flags"),
                Ok(PreviousValue::Text(flags)) if esta_ligado(&flags) == Some(true)
            )
        })
        .map(|(_, nome)| *nome)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn o_padrao_do_windows_esta_desligado() {
        // Os três valores padrão, lidos da máquina de desenvolvimento: 126, 510
        // e 62. Em todos o bit 0 está zerado. Um produto que "consertasse" isto
        // estaria inventando problema para vender solução.
        assert_eq!(esta_ligado("126"), Some(false));
        assert_eq!(esta_ligado("510"), Some(false));
        assert_eq!(esta_ligado("62"), Some(false));
    }

    #[test]
    fn o_bit_zero_ligado_e_reconhecido() {
        // 127 é o 126 com o recurso em uso: o estado de quem segurou Shift por
        // oito segundos sem querer.
        assert_eq!(esta_ligado("127"), Some(true));
        assert_eq!(esta_ligado("511"), Some(true));
    }

    #[test]
    fn desligar_preserva_as_preferencias_do_usuario() {
        // A regra que mais importa aqui. Gravar o "122" dos tutoriais desligaria
        // o recurso e apagaria junto as escolhas da pessoa sobre atalho, som e
        // aviso. Só o bit 0 pode mudar.
        assert_eq!(desligado("127").as_deref(), Some("126"));
        assert_eq!(desligado("511").as_deref(), Some("510"));
        assert_eq!(desligado("63").as_deref(), Some("62"));
    }

    #[test]
    fn ja_desligado_nao_gera_escrita() {
        // Escrever para gravar o valor que já estava lá deixaria rastro nosso no
        // registro do cliente sem mudar nada.
        assert_eq!(desligado("126"), None);
        assert_eq!(desligado("510"), None);
    }

    #[test]
    fn valor_ilegivel_nao_vira_palpite() {
        assert_eq!(esta_ligado("nao é numero"), None);
        assert_eq!(desligado(""), None);
    }

    #[test]
    fn le_esta_maquina() {
        let ligadas = ligadas();
        println!("teclas de acessibilidade ligadas agora: {:?}", ligadas);

        for (caminho, nome) in CHAVES {
            let flags = super::super::registry::read("HKCU", caminho, "Flags");
            println!("  {} → {:?}", nome, flags);
        }
    }
}
