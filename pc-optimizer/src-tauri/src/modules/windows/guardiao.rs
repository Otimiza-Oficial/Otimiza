// Guardião (MASTER-PLAN, item 5): relê na máquina cada ajuste do catálogo que o histórico diz estar aplicado. Até a
// 3.2 a tela confiava no histórico (`inspect` devolve `Applied` sem ler), e um ajuste desfeito por atualização do
// Windows, por política da empresa, por hardware novo, por outro programa ou pelo próprio cliente continuava
// aparecendo como aplicado.
//
// Só lê. Voltar a valer é "Refazer": DESFAZER pelo histórico (devolve o valor de antes do Otimiza) e APLICAR de
// novo pelo caminho normal, que grava o registro e o diário. Escrever por fora do histórico deixaria mudanças sem
// volta (achado da revisão: chave que já estava no alvo, limite novo do bcdedit, placa nova).
//
// Fora da vistoria, sem fingir que foram conferidos: tudo o que não é do catálogo (perfil de jogo, NVIDIA, Hz) e o
// plano OTIMIZA, que o próprio Otimiza troca (modo dinâmico, prova alternada) e tem vistoria própria
// (`planoenergia::vistoriar`).

use serde::Serialize;

use super::catalog::{self, Action};
use super::{ActionState, WindowsOptimizer};
use crate::modules::changelog::ChangeLog;
use crate::modules::optimizer::OptimizationOutcome;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Estado {
    /// Tudo o que o Otimiza escreveu continua lá.
    Intacto,
    /// Pelo menos uma escrita não está mais como o Otimiza deixou.
    MudouPorFora,
    /// Não deu para ler (permissão, política, sem administrador). NÃO é "intacto" nem "mudou".
    NaoLeu,
    /// O que ele mexia não existe mais (serviço desinstalado, por exemplo).
    NaoSeAplicaMais,
}

#[derive(Debug, Clone, Serialize)]
pub struct Item {
    pub id: String,
    pub nome: String,
    pub estado: Estado,
    /// Retirado do catálogo: o caminho é desfazer, não refazer.
    pub retirado: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Vistoria {
    pub itens: Vec<Item>,
    /// Aplicadas que o guardião não relê, pelo nome.
    pub fora_da_vistoria: Vec<String>,
    /// Política de grupo nesta máquina: o que ela desfaz, ela desfaz de novo depois de refeito.
    pub com_politica_de_grupo: bool,
}

/// **Pura.** A ordem é a regra: uma escrita desfeita vence uma leitura falha (senão o aviso sumiria atrás de um
/// "não li"), e a leitura falha vence "intacto".
pub(super) fn classificar(estados: &[ActionState]) -> Estado {
    if estados.iter().any(|s| *s == ActionState::Pending) {
        Estado::MudouPorFora
    } else if estados.iter().any(|s| *s == ActionState::Desconhecido) {
        Estado::NaoLeu
    } else if !estados.is_empty() && estados.iter().all(|s| *s == ActionState::NotApplicable) {
        Estado::NaoSeAplicaMais
    } else {
        Estado::Intacto
    }
}

/// **Pura.** Sem administrador, `inspect_action` devolve `Pending` para o que nem conseguiu ler (serve para a lista
/// oferecer o item); para a vistoria isso é "não li", não "mudou".
pub(super) fn leitura_da_vistoria(estado: ActionState, precisa_de_admin_para_ler: bool, elevado: bool) -> ActionState {
    if precisa_de_admin_para_ler && !elevado {
        ActionState::Desconhecido
    } else {
        estado
    }
}

fn precisa_de_admin_para_ler(action: &Action) -> bool {
    matches!(
        action,
        Action::ClearBootLimits | Action::ReservedStorage { .. } | Action::DisableHypervisor | Action::RemoveForcedPlatformClock
    )
}

/// O plano OTIMIZA fica de fora: o próprio Otimiza o troca e ele tem vistoria própria.
fn fora_por_regra(spec: &catalog::OptimizationSpec) -> bool {
    spec.actions.iter().any(|a| matches!(a, Action::PlanoOtimiza))
}

impl WindowsOptimizer {
    /// `aplicadas`: `(id, nome)` copiados do histórico, para ler a máquina sem segurar o histórico.
    pub fn vistoriar(&self, aplicadas: &[(String, String)]) -> Vistoria {
        let elevado = super::registry::is_elevated();
        let mut itens = Vec::new();
        let mut fora_da_vistoria = Vec::new();
        for (id, nome) in aplicadas {
            match catalog::find(id) {
                Some(spec) if !fora_por_regra(spec) => {
                    let estados: Vec<ActionState> = spec
                        .actions
                        .iter()
                        .map(|a| leitura_da_vistoria(self.inspect_action(a), precisa_de_admin_para_ler(a), elevado))
                        .collect();
                    itens.push(Item {
                        id: spec.id.to_string(),
                        nome: spec.name.to_string(),
                        estado: classificar(&estados),
                        retirado: catalog::retirado(spec.id),
                    });
                }
                _ => fora_da_vistoria.push(nome.clone()),
            }
        }
        let mudaram: Vec<&str> = itens.iter().filter(|i| i.estado == Estado::MudouPorFora).map(|i| i.id.as_str()).collect();
        if !mudaram.is_empty() {
            crate::utils::Logger::warn(&format!("guardião: {} ajuste(s) não estão como o Otimiza deixou: {}", mudaram.len(), mudaram.join(", ")));
        }
        Vistoria { itens, fora_da_vistoria, com_politica_de_grupo: super::governanca().com_politica_de_grupo }
    }

    /// Desfaz pelo histórico e aplica de novo pelo caminho normal: os dois gravam registro e diário. Se aplicar falhar
    /// depois de desfazer, o item fica desfeito, com os valores de antes do Otimiza: estado conhecido, não meio a meio.
    pub fn refazer(&self, id: &str, log: &mut ChangeLog) -> Result<OptimizationOutcome, String> {
        let spec = catalog::find(id).ok_or_else(|| format!("`{}` não é do catálogo", id))?;
        if !log.is_applied(id) {
            return Err(format!("`{}` não está aplicado; use Otimizar.", spec.name));
        }
        if catalog::retirado(id) {
            return Err(format!("`{}` foi retirado do Otimiza por não mudar o jogo: desfaça em vez de refazer.", spec.name));
        }
        if fora_por_regra(spec) {
            return Err("O plano OTIMIZA tem conferência própria, na área Sistema.".to_string());
        }
        if crate::modules::provaalternada::em_andamento() {
            return Err("A prova do Otimizar está rodando. Espere ela terminar.".to_string());
        }
        if spec.requires_admin && !super::registry::is_elevated() {
            return Err(format!("`{}` exige executar o programa como administrador.", spec.name));
        }
        // Como a lista: a condição medida agora falsa esconde o item, e refazer o poria de volta sem motivo.
        if let catalog::Classe::Condicional(c) = catalog::classe(id) {
            if super::condicao_atendida_sem_esperar(c) == Some(false) {
                return Err(format!(
                    "A condição que fazia `{}` valer neste PC não vale mais. Desfaça em vez de refazer.",
                    spec.name
                ));
            }
        }

        // Desfazer falho devolve Err e põe o registro de volta: nada é aplicado depois.
        self.revert(id, log)?;
        let refeito = self.apply(id, log)?;
        crate::utils::Logger::info(&format!("guardião: refazer `{}`: {}", id, refeito.success));
        if refeito.success {
            Ok(OptimizationOutcome { message: format!("{} voltou a valer.", spec.name), ..refeito })
        } else {
            Ok(OptimizationOutcome {
                message: format!(
                    "{} foi desfeito (voltou ao valor de antes do Otimiza), mas aplicar de novo falhou: {}",
                    spec.name, refeito.message
                ),
                ..refeito
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ActionState::*;

    #[test]
    fn escrita_desfeita_vence_leitura_falha() {
        assert_eq!(classificar(&[Satisfied, Desconhecido, Pending]), Estado::MudouPorFora);
    }

    #[test]
    fn nao_ler_nao_e_intacto() {
        assert_eq!(classificar(&[Satisfied, Desconhecido]), Estado::NaoLeu);
    }

    #[test]
    fn tudo_no_lugar_e_intacto_e_o_que_sumiu_nao_conta_contra() {
        assert_eq!(classificar(&[Satisfied, Satisfied]), Estado::Intacto);
        assert_eq!(classificar(&[Satisfied, NotApplicable]), Estado::Intacto);
        assert_eq!(classificar(&[NotApplicable, NotApplicable]), Estado::NaoSeAplicaMais);
    }

    #[test]
    fn sem_administrador_o_que_nao_se_le_nao_vira_mudou() {
        assert_eq!(leitura_da_vistoria(Pending, true, false), Desconhecido);
        assert_eq!(leitura_da_vistoria(Pending, true, true), Pending);
        assert_eq!(leitura_da_vistoria(Pending, false, false), Pending, "o que se lê sem admin continua valendo");
    }

    #[test]
    fn o_plano_otimiza_fica_fora_da_vistoria() {
        let plano = catalog::find("plano_otimiza").expect("o plano está no catálogo");
        assert!(fora_por_regra(plano));
    }
}
