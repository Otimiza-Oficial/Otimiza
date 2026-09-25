// Filtragem de Teclas e Teclas de Aderência ligadas sem querer (Shift segurado 8 s no jogo liga o atalho). O
// estado é um campo de bits e só o bit 0 diz LIGADO: gravar o "122" dos tutoriais apagaria as escolhas da pessoa
// sobre atalho, som e aviso. Lê, muda um bit, escreve; a reversão devolve o número exato.

use crate::modules::changelog::{ChangeRecord, PreviousValue};

/// `MouseKeys` fica de fora: não atrapalha quem não usa, e desligar não devolve desempenho.
pub const CHAVES: &[(&str, &str)] = &[
    (r"Control Panel\Accessibility\Keyboard Response", "Filtragem de Teclas"),
    (r"Control Panel\Accessibility\StickyKeys", "Teclas de Aderência"),
    (r"Control Panel\Accessibility\ToggleKeys", "Teclas Alternadas"),
];

const BIT_LIGADO: u32 = 1;

pub fn esta_ligado(flags: &str) -> Option<bool> {
    let valor: u32 = flags.trim().parse().ok()?;
    Some(valor & BIT_LIGADO != 0)
}

/// `None` quando já está desligado: não se grava o valor que já estava lá.
pub fn desligado(flags: &str) -> Option<String> {
    let valor: u32 = flags.trim().parse().ok()?;

    if valor & BIT_LIGADO == 0 {
        return None;
    }

    Some((valor & !BIT_LIGADO).to_string())
}

/// Recebe o vetor em vez de devolvê-lo: devolvendo `Result<Vec<_>>`, uma falha na segunda chave descartaria o
/// registro da primeira, já gravada, e a reversão não teria como desfazê-la.
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
        // Padrões da máquina de desenvolvimento (126, 510, 62): bit 0 zerado em todos.
        assert_eq!(esta_ligado("126"), Some(false));
        assert_eq!(esta_ligado("510"), Some(false));
        assert_eq!(esta_ligado("62"), Some(false));
    }

    #[test]
    fn o_bit_zero_ligado_e_reconhecido() {
        assert_eq!(esta_ligado("127"), Some(true));
        assert_eq!(esta_ligado("511"), Some(true));
    }

    #[test]
    fn desligar_preserva_as_preferencias_do_usuario() {
        assert_eq!(desligado("127").as_deref(), Some("126"));
        assert_eq!(desligado("511").as_deref(), Some("510"));
        assert_eq!(desligado("63").as_deref(), Some("62"));
    }

    #[test]
    fn ja_desligado_nao_gera_escrita() {
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
