// Os serviços essenciais do Windows, e o Windows que vem sem eles
//
// POR QUE ISTO EXISTE
//
// Em 10 e 11/09/2026, no PC do dono, clicar em "Otimizar agora" foi seguido de
// "os programas não abriam". O PC não rodava um Windows original: o registro do
// fabricante dizia "Team AntiLag · SnyX OS", uma imagem "lite" que chega com
// cerca de 180 serviços desativados — entre eles o Plug and Play, as licenças
// da Microsoft Store e o Gerente de Contas de Segurança. Nenhum deles é
// desligado pelo Otimiza, e o produto não tinha como saber que já estavam
// desligados antes do clique.
//
// Imagem desse tipo é comum no público de FiveM. Com esses serviços desligados,
// programa trava ou não abre com ou sem otimização — e o que acontece depois do
// clique vira culpa do Otimiza.
//
// O QUE ESTE MÓDULO FAZ, E O QUE NÃO FAZ
//
// Lê o tipo de início de uma lista CURTA e FIXA de serviços e diz quais estão
// desativados. Não é "todo serviço que otimizador ruim desliga": é só o que a
// própria Microsoft descreve como necessário para programa e aplicativo
// funcionarem, e cujo tipo de início padrão ela publica.
//
// O fabricante registrado vai junto como evidência, não como acusação:
// fabricante de verdade (Dell, Lenovo) também escreve ali. Quem decide o aviso é
// o estado dos serviços, nunca o nome.
//
// Religar (em `WindowsOptimizer::religar_essenciais`) volta cada serviço
// desativado ao tipo de início PADRÃO do Windows — nunca "automático" por
// conveniência — e entra no histórico: o "Desfazer" os devolve a desligados,
// como estavam.
//
// O Rust manda o ESTADO (qual serviço, desativado ou não); a tela escolhe a
// frase.

use super::registry;
use crate::modules::changelog::PreviousValue;
use serde::Serialize;

const SERVICES_KEY: &str = r"SYSTEM\CurrentControlSet\Services";
const OEM_KEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\OEMInformation";

/// Um serviço que a checagem confere, e o tipo de início que o Windows traz de
/// fábrica.
pub struct Essencial {
    pub servico: &'static str,
    /// No formato do `sc config`: `auto` ou `demand`.
    pub padrao: &'static str,
}

/// A lista, com o padrão de cada um.
///
/// Os padrões vêm da tabela da Microsoft "Security guidelines for system
/// services in Windows Server 2016 with Desktop Experience" — a fonte oficial
/// que publica o tipo de início serviço a serviço. O `TokenBroker` não está
/// nela e segue o padrão Manual do Windows 10 e 11.
///
/// Ficam de fora, de propósito, o que imagem "lite" desliga sem impedir programa
/// de abrir (Temas, Cache de Fontes, Notificações) e o que o Windows não deixa
/// desligar pelo caminho comum (RPC, DCOM, Log de Eventos).
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
    /// Esta instalação não tem o serviço. Não é defeito: edições do Windows variam.
    NaoExiste,
    /// Não deu para ler. NÃO conta como desativado: não sabemos.
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
    /// Quantos estão desativados — é o que decide o aviso.
    pub desativados: usize,
    /// O que o Windows diz de quem o montou. Evidência lida da máquina; nunca
    /// decide nada sozinha.
    pub fabricante: Option<String>,
    pub modelo: Option<String>,
}

/// O tipo de início a partir da leitura do valor `Start` do serviço.
///
/// Função pura, separada da leitura, para cada caso ser testável sem uma máquina
/// com o serviço desligado.
pub fn classificar(start: Result<PreviousValue, String>) -> Inicio {
    match start {
        Ok(PreviousValue::AbsentKey) => Inicio::NaoExiste,
        Ok(PreviousValue::Dword(4)) => Inicio::Desativado,
        Ok(PreviousValue::Dword(_)) => Inicio::Ativo,
        // Leitura negada, valor ausente ou de outro tipo: nada disso é
        // "desativado", e dizer que é seria o chute que este produto não dá.
        _ => Inicio::NaoConsegui,
    }
}

/// Lê o tipo de início de um serviço no registro — o mesmo número em qualquer
/// idioma do Windows, ao contrário do texto do `sc qc`.
pub fn inicio_de(servico: &str) -> Inicio {
    classificar(registry::read(
        "HKLM",
        &format!("{}\\{}", SERVICES_KEY, servico),
        "Start",
    ))
}

/// Confere a lista inteira. Só leitura: não precisa de administrador.
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
        registry::read_text("HKLM", OEM_KEY, nome).filter(|texto| !texto.trim().is_empty())
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
        // Se virasse, uma leitura negada abriria o aviso e ofereceria "religar"
        // um serviço que pode estar ligado.
        assert_eq!(
            classificar(Err("acesso negado".to_string())),
            Inicio::NaoConsegui
        );
        assert_eq!(classificar(Ok(PreviousValue::Absent)), Inicio::NaoConsegui);
    }

    #[test]
    fn religar_so_volta_para_um_padrao_do_windows() {
        // "auto" e "demand" são os dois únicos padrões desta lista. Religar
        // para qualquer outra coisa — "disabled" por engano, ou "delayed-auto"
        // inventado — deixaria o serviço num estado que o Windows não traz.
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
        // O produto não pode chamar um serviço de essencial numa tela e
        // desligá-lo noutra.
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
