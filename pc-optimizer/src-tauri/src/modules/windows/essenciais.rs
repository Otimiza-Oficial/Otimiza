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
    Essencial {
        servico: "PlugPlay",
        padrao: "demand",
    },
    Essencial {
        servico: "AppXSvc",
        padrao: "demand",
    },
    Essencial {
        servico: "ClipSVC",
        padrao: "demand",
    },
    Essencial {
        servico: "LicenseManager",
        padrao: "demand",
    },
    Essencial {
        servico: "StateRepository",
        padrao: "demand",
    },
    Essencial {
        servico: "AppReadiness",
        padrao: "demand",
    },
    Essencial {
        servico: "KeyIso",
        padrao: "demand",
    },
    Essencial {
        servico: "CryptSvc",
        padrao: "auto",
    },
    Essencial {
        servico: "SamSs",
        padrao: "auto",
    },
    Essencial {
        servico: "TimeBrokerSvc",
        padrao: "demand",
    },
    Essencial {
        servico: "TokenBroker",
        padrao: "demand",
    },
];

/// UM WINDOWS MODIFICADO ("lite", "anti-lag", "gamer OS"): a imagem se apresenta no próprio registro, no dono e na
/// organização registrados e na ficha do fabricante (`OEMInformation`). No PC do dono (11/10/2026): "SnyX OS 11 25H2
/// v1.4.5 by Team AntiLag", organização "https://teamantilag.com". Nele a captura nova do Windows
/// (Windows.Graphics.Capture) é recusada a qualquer programa, com as permissões concedidas: peça removida da imagem.
/// Dizer isso evita que a culpa de um recurso faltando caia no Otimiza.
#[derive(Debug, Clone, PartialEq, Serialize, serde::Deserialize)]
pub struct WindowsModificado {
    /// Como a imagem se chama ("SnyX OS 11 25H2 v1.4.5").
    pub nome: String,
    /// Quem fez, quando ela diz ("Team AntiLag").
    pub autor: Option<String>,
}

/// Nomes de imagens modificadas conhecidas, em minúsculas. Lista curta: o nome sozinho já basta para estas.
const IMAGENS_CONHECIDAS: &[&str] = &[
    "antilag", "atlasos", "atlas os", "revios", "revi os", "ghost spectre", "ghostspectre", "tiny11", "tiny10",
    "x-lite", "superlite", "kernelos", "windows lite", "gamer os", "spectre os",
];

/// Pura: a imagem é um Windows modificado? Pelo dono e organização registrados e pelo fabricante e modelo da ficha.
/// Um PC de marca também preenche a ficha (Dell, Lenovo), mas nunca com site no nome do dono, nem "… OS … by …".
pub fn identificar_windows_modificado(
    dono: Option<&str>,
    organizacao: Option<&str>,
    fabricante: Option<&str>,
    modelo: Option<&str>,
) -> Option<WindowsModificado> {
    let campos: Vec<&str> = [dono, organizacao, fabricante, modelo]
        .into_iter()
        .flatten()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .collect();
    let minusculo = |t: &str| t.to_lowercase();
    let conhecida = campos
        .iter()
        .any(|t| IMAGENS_CONHECIDAS.iter().any(|n| minusculo(t).contains(n)));
    let site_no_registro = [dono, organizacao]
        .into_iter()
        .flatten()
        .any(|t| { let t = minusculo(t); t.contains("http://") || t.contains("https://") || t.contains("www.") });
    // "SnyX OS 11 25H2 v1.4.5 by Team AntiLag": um sistema com nome próprio, versão do Windows e assinatura.
    let assinada = |t: &str| {
        let t = format!(" {} ", minusculo(t));
        t.contains(" os ") && t.contains(" by ") && (t.contains(" 10") || t.contains(" 11"))
    };
    if !(conhecida || site_no_registro || campos.iter().any(|t| assinada(t))) {
        return None;
    }
    // O nome: o campo que parece um sistema ("… OS …"), senão o modelo, senão o dono.
    let com_nome = campos
        .iter()
        .find(|t| format!(" {} ", minusculo(t)).contains(" os "))
        .or(modelo.filter(|m| !m.trim().is_empty()).as_ref())
        .or(dono.as_ref())
        .map(|t| t.trim().to_string())?;
    let (nome, autor_no_nome) = match com_nome.to_lowercase().find(" by ") {
        Some(i) => (com_nome[..i].trim().to_string(), Some(com_nome[i + 4..].trim().to_string())),
        None => (com_nome.clone(), None),
    };
    let autor = autor_no_nome
        .filter(|a| !a.is_empty())
        .or_else(|| fabricante.map(|f| f.trim().to_string()).filter(|f| !f.is_empty() && !nome.contains(f.as_str())));
    Some(WindowsModificado { nome, autor })
}

/// Lê o registro e identifica.
pub fn windows_modificado() -> Option<WindowsModificado> {
    let nt = |nome: &str| {
        registry::read_text("HKLM", r"SOFTWARE\Microsoft\Windows NT\CurrentVersion", nome)
            .ok()
            .flatten()
    };
    let oem = |nome: &str| registry::read_text("HKLM", OEM_KEY, nome).ok().flatten();
    identificar_windows_modificado(
        nt("RegisteredOwner").as_deref(),
        nt("RegisteredOrganization").as_deref(),
        oem("Manufacturer").as_deref(),
        oem("Model").as_deref(),
    )
}

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

    /// `cargo test --lib essenciais::tests::mostra_o_windows_deste_pc -- --ignored --nocapture`
    #[cfg(target_os = "windows")]
    #[test]
    #[ignore]
    fn mostra_o_windows_deste_pc() {
        println!("windows modificado: {:?}", windows_modificado());
    }

    #[test]
    fn reconhece_o_windows_modificado_do_dono() {
        let w = identificar_windows_modificado(
            Some("SnyX OS 11 25H2 v1.4.5 by Team AntiLag"),
            Some("https://teamantilag.com"),
            Some("Team AntiLag"),
            Some("SnyX OS 11 25H2 v1.4.5"),
        )
        .unwrap();
        assert_eq!(w.nome, "SnyX OS 11 25H2 v1.4.5");
        assert_eq!(w.autor.as_deref(), Some("Team AntiLag"));
    }

    #[test]
    fn pc_de_marca_e_windows_comum_nao_sao_modificados() {
        assert_eq!(
            identificar_windows_modificado(Some("Usuário"), None, Some("Dell Inc."), Some("Inspiron 15 3511")),
            None
        );
        assert_eq!(identificar_windows_modificado(Some("joao"), Some(""), None, None), None);
        assert_eq!(
            identificar_windows_modificado(Some("Windows User"), Some("Lenovo"), Some("LENOVO"), Some("82K2")),
            None
        );
    }

    #[test]
    fn imagens_conhecidas_e_site_no_registro_bastam() {
        let a = identificar_windows_modificado(Some("user"), None, Some("AtlasOS"), Some("AtlasOS v0.4")).unwrap();
        assert_eq!(a.nome, "AtlasOS v0.4");
        let s = identificar_windows_modificado(Some("Gamer"), Some("www.algumaiso.net"), None, Some("Windows Turbo")).unwrap();
        assert_eq!(s.nome, "Windows Turbo");
    }

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
                        !ESSENCIAIS
                            .iter()
                            .any(|e| e.servico.eq_ignore_ascii_case(name)),
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
        println!(
            "fabricante: {:?} · modelo: {:?}",
            checagem.fabricante, checagem.modelo
        );

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
