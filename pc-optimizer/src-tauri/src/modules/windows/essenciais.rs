// Serviços essenciais desativados, típico de imagem "lite" do Windows (no PC do dono, "SnyX OS" com ~180
// desativados): programa não abre com ou sem otimização, e a culpa cai no Otimiza. Lê uma lista CURTA e FIXA do
// que a Microsoft descreve como necessário. O fabricante registrado é evidência, nunca decide. Religar volta ao
// tipo de início PADRÃO do Windows, e entra no histórico.

use super::registry;
use crate::modules::changelog::PreviousValue;
use serde::Serialize;

const SERVICES_KEY: &str = r"SYSTEM\CurrentControlSet\Services";
const OEM_KEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\OEMInformation";

pub struct Essencial {
    pub servico: &'static str,
    pub padrao: &'static str,
}

/// Padrões da tabela da Microsoft "Security guidelines for system services in Windows Server 2016 with Desktop
/// Experience"; `TokenBroker` segue o Manual do Windows 10 e 11. Fora de propósito: o que a imagem "lite" desliga
/// sem impedir programa de abrir, e o que o Windows não deixa desligar.
pub const ESSENCIAIS: &[Essencial] = &[
    Essencial { servico: "PlugPlay", padrao: "demand" },
    Essencial { servico: "AppXSvc", padrao: "demand" },
    Essencial { servico: "ClipSVC", padrao: "demand" },
    Essencial { servico: "LicenseManager", padrao: "demand" },
    Essencial { servico: "StateRepository", padrao: "demand" },
    Essencial { servico: "AppReadiness", padrao: "demand" },
    Essencial { servico: "KeyIso", padrao: "demand" },
    Essencial { servico: "CryptSvc", padrao: "auto" },
    Essencial { servico: "SamSs", padrao: "auto" },
    Essencial { servico: "TimeBrokerSvc", padrao: "demand" },
    Essencial { servico: "TokenBroker", padrao: "demand" },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Inicio {
    Desativado,
    Ativo,
    NaoExiste,
    /// Não deu para ler: NÃO conta como desativado.
    NaoConsegui,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServicoEssencial {
    pub servico: &'static str,
    pub inicio: Inicio,
}

#[derive(Debug, Clone, Serialize)]
pub struct Checagem {
    pub servicos: Vec<ServicoEssencial>,
    pub desativados: usize,
    pub fabricante: Option<String>,
    pub modelo: Option<String>,
}

pub fn classificar(start: Result<PreviousValue, String>) -> Inicio {
    match start {
        Ok(PreviousValue::AbsentKey) => Inicio::NaoExiste,
        Ok(PreviousValue::Dword(4)) => Inicio::Desativado,
        Ok(PreviousValue::Dword(_)) => Inicio::Ativo,
        // Leitura negada ou valor estranho não é "desativado".
        _ => Inicio::NaoConsegui,
    }
}

pub fn inicio_de(servico: &str) -> Inicio {
    classificar(registry::read(
        "HKLM",
        &format!("{}\\{}", SERVICES_KEY, servico),
        "Start",
    ))
}

pub fn checar() -> Checagem {
    let servicos: Vec<ServicoEssencial> = ESSENCIAIS
        .iter()
        .map(|essencial| ServicoEssencial {
            servico: essencial.servico,
            inicio: inicio_de(essencial.servico),
        })
        .collect();

    let desativados = servicos
        .iter()
        .filter(|s| s.inicio == Inicio::Desativado)
        .count();

    let ler_oem = |nome: &str| {
        registry::read_text("HKLM", OEM_KEY, nome)
            .ok()
            .flatten()
            .filter(|texto| !texto.trim().is_empty())
    };

    Checagem {
        servicos,
        desativados,
        fabricante: ler_oem("Manufacturer"),
        modelo: ler_oem("Model"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::windows::catalog::{Action, CATALOG};
    use std::collections::HashSet;

    #[test]
    fn so_o_valor_4_e_desativado() {
        assert_eq!(classificar(Ok(PreviousValue::Dword(4))), Inicio::Desativado);
        assert_eq!(classificar(Ok(PreviousValue::Dword(2))), Inicio::Ativo);
        assert_eq!(classificar(Ok(PreviousValue::Dword(3))), Inicio::Ativo);
        assert_eq!(classificar(Ok(PreviousValue::AbsentKey)), Inicio::NaoExiste);
    }

    #[test]
    fn nao_conseguir_ler_nunca_vira_desativado() {
        // Senão uma leitura negada ofereceria "religar" um serviço que pode estar ligado.
        assert_eq!(
            classificar(Err("acesso negado".to_string())),
            Inicio::NaoConsegui
        );
        assert_eq!(classificar(Ok(PreviousValue::Absent)), Inicio::NaoConsegui);
    }

    #[test]
    fn religar_so_volta_para_um_padrao_do_windows() {
        // Religar para qualquer outro estado deixaria o serviço num estado que o Windows não traz.
        for essencial in ESSENCIAIS {
            assert!(
                matches!(essencial.padrao, "auto" | "demand"),
                "`{}` religaria para `{}`",
                essencial.servico,
                essencial.padrao
            );
        }
    }

    #[test]
    fn a_lista_nao_repete_servico() {
        let unicos: HashSet<&str> = ESSENCIAIS.iter().map(|e| e.servico).collect();
        assert_eq!(unicos.len(), ESSENCIAIS.len());
    }

    #[test]
    fn o_catalogo_nunca_desliga_um_essencial() {
        // O produto não pode chamar um serviço de essencial numa tela e desligá-lo noutra.
        for spec in CATALOG {
            for action in spec.actions {
                if let Action::DisableService { name } = action {
                    assert!(
                        !ESSENCIAIS.iter().any(|e| e.servico.eq_ignore_ascii_case(name)),
                        "`{}` desliga o essencial `{}`",
                        spec.id,
                        name
                    );
                }
            }
        }
    }

    #[test]
    fn le_os_essenciais_desta_maquina() {
        let checagem = checar();

        for servico in &checagem.servicos {
            println!("{:<18} {:?}", servico.servico, servico.inicio);
        }
        println!("fabricante: {:?} · modelo: {:?}", checagem.fabricante, checagem.modelo);

        assert_eq!(checagem.servicos.len(), ESSENCIAIS.len());
        assert_eq!(
            checagem.desativados,
            checagem
                .servicos
                .iter()
                .filter(|s| s.inicio == Inicio::Desativado)
                .count()
        );
    }
}
