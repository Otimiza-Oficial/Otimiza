// Tipos compartilhados entre a interface e os otimizadores de cada plataforma (`modules::windows`).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Category {
    System,
    Gaming,
    Network,
    Startup,
    Privacy,
}

/// Conservador de propósito: prometer menos e entregar o medido.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExpectedGain {
    Measurable,
    Situational,
    Responsiveness,
    /// Não muda desempenho (higiene e privacidade). Separado de `Responsiveness` para o cliente saber, antes de
    /// clicar, quantos itens nunca prometeram diferença.
    NoGain,
}

/// Se um ajuste pode DERRUBAR o FPS em alguma máquina. Na 2.1.0 um cliente caiu de ~200 para 80-120 FPS: o
/// catálogo não tinha como dizer que um ajuste pode piorar. O botão grande só aplica o que não pode custar
/// quadro (ver `catalog::entra_no_lote`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "risco")]
pub enum RiscoDeFps {
    Nenhum,
    /// Fica FORA do lote automático. `o_que` e `quando` juntos permitem decidir: "pode variar" não ajuda ninguém.
    /// `Cow` porque `OptimizationInfo` precisa ser desserializável.
    PodeCustar {
        o_que: OQuePodeCustar,
        quando: std::borrow::Cow<'static, str>,
    },
}

/// Quatro coisas diferentes: tratá-las como uma só entregou "otimização" que derrubou FPS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OQuePodeCustar {
    FpsMedio,
    /// Pode cair com a média intacta, e é o que a pessoa SENTE.
    UmPorCentoPior,
    Engasgo,
    /// Só aparece em carga longa: o mais difícil de perceber.
    Boost,
}

impl OQuePodeCustar {
    /// Vocabulário de produto: mora aqui porque tem teste.
    pub fn rotulo(self) -> &'static str {
        match self {
            OQuePodeCustar::FpsMedio => "pode custar FPS",
            OQuePodeCustar::UmPorCentoPior => "pode piorar os quadros ruins",
            OQuePodeCustar::Engasgo => "pode causar engasgo",
            OQuePodeCustar::Boost => "pode segurar o turbo do processador",
        }
    }
}

impl RiscoDeFps {
    pub const fn custa(o_que: OQuePodeCustar, quando: &'static str) -> Self {
        RiscoDeFps::PodeCustar {
            o_que,
            quando: std::borrow::Cow::Borrowed(quando),
        }
    }

    pub fn pode_custar(&self) -> bool {
        matches!(self, RiscoDeFps::PodeCustar { .. })
    }
}

/// O rótulo sai do Rust JÁ ESCOLHIDO: montá-lo na tela pelo nome da variante é o que
/// `a_tela_nao_decide_cor_comparando_texto_do_backend` impede, e um texto fixo rotularia errado o ajuste que
/// ataca o engasgo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "risco")]
pub enum RiscoNaTela {
    Nenhum,
    PodeCustar {
        o_que: OQuePodeCustar,
        rotulo: String,
        quando: String,
    },
}

impl From<&RiscoDeFps> for RiscoNaTela {
    fn from(risco: &RiscoDeFps) -> Self {
        match risco {
            RiscoDeFps::Nenhum => RiscoNaTela::Nenhum,
            RiscoDeFps::PodeCustar { o_que, quando } => RiscoNaTela::PodeCustar {
                o_que: *o_que,
                rotulo: o_que.rotulo().to_string(),
                quando: quando.to_string(),
            },
        }
    }
}

/// `AlreadyOptimal`: quando já está assim, o produto diz isso em vez de fingir que otimizou.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptimizationState {
    Applied,
    AlreadyOptimal,
    Available,
    Unavailable,
    /// NÃO DEU PARA VERIFICAR. Nem `Unavailable` ("não se aplica" seria mentira sobre uma leitura falha) nem
    /// `AlreadyOptimal` ("já está assim" sem ler faz o cliente parar de procurar).
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub honest_effect: String,
    pub category: Category,
    pub expected_gain: ExpectedGain,
    /// Quando pode, fica fora do "Otimizar agora" e a tela diz por quê.
    pub risco_de_fps: RiscoNaTela,
    pub requires_admin: bool,
    pub requires_restart: bool,
    pub reversible: bool,
    /// Troca segurança por desempenho: aviso em vermelho e fora do lote automático.
    pub security_tradeoff: bool,
    pub recommended: bool,
    /// Retirado na 2.9: só aparece enquanto aplicado, e só se desfaz. Ver `catalog::RETIRADOS`.
    #[serde(default)]
    pub retirado: bool,
    #[serde(default)]
    pub expert: bool,
    #[serde(default)]
    pub condicao: Option<String>,
    pub state: OptimizationState,
    pub detail: Option<String>,
}

/// O desfecho de UMA ação, para reproduzir a falha à distância: antes, pedido, depois, saída do comando e duração.
/// `Option` onde o dado pode não existir: `0` de código de saída afirmaria que um comando deu certo.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActionResult {
    pub name: String,
    pub status: ActionStatus,
    pub message: String,
    pub before_value: Option<String>,
    pub expected_value: Option<String>,
    pub after_value: Option<String>,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    pub unsupported_reason: Option<String>,
}

/// Vocabulário do protocolo de compatibilidade, o mesmo das notas de laboratório, para os relatórios se agruparem.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionStatus {
    /// Escrito E CONFERIDO relendo do Windows.
    #[default]
    Verified,
    AlreadyOptimized,
    /// Este Windows ou hardware não tem isto. NÃO É FALHA.
    Unsupported,
    Failed,
    /// Separado de `Failed`: quase sempre é política de domínio ou outro programa reescrevendo.
    VerificationFailed,
    NotConfirmed,
    /// Só com GPO de verdade na máquina: acusar política sem política mandaria procurar um administrador que não existe.
    BlockedByPolicy,
    /// O Windows tem o recurso e quem montou a imagem tirou (diferente de `Unsupported`).
    UserOrOemManaged,
    Skipped,
}

impl ActionStatus {
    /// `Unsupported` conta como certo: pintar de vermelho manda procurar defeito onde não há.
    pub fn deu_certo(&self) -> bool {
        matches!(
            self,
            ActionStatus::Verified
                | ActionStatus::AlreadyOptimized
                | ActionStatus::Unsupported
                | ActionStatus::UserOrOemManaged
                | ActionStatus::NotConfirmed
                | ActionStatus::Skipped
        )
    }
}

/// `Default` é o estado neutro dos campos novos em cada ponto de construção, não um resultado de verdade.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OptimizationOutcome {
    pub id: String,
    pub name: String,
    pub success: bool,
    pub applied: bool,
    pub message: String,
    pub requires_restart: bool,
    /// Campo e não status: aplicada E exige logoff precisam caber juntas.
    #[serde(default)]
    pub requires_logoff: bool,
    /// Separa "falhou" de "ficou pendurada".
    #[serde(default)]
    pub duration_ms: u64,
    pub changes_count: usize,
    pub changes: Vec<String>,
    /// NÃO substitui `changes` (o que o cliente lê ao vivo): esta é a evidência que o atendimento lê.
    #[serde(default)]
    pub actions: Vec<ActionResult>,
}

impl OptimizationOutcome {
    /// Para um campo novo não virar edição em seis lugares.
    pub fn novo(id: &str, name: &str, message: String) -> Self {
        OptimizationOutcome {
            id: id.to_string(),
            name: name.to_string(),
            success: false,
            applied: false,
            message,
            requires_restart: false,
            requires_logoff: false,
            duration_ms: 0,
            changes_count: 0,
            changes: Vec::new(),
            actions: Vec::new(),
        }
    }

    /// Um erro isolado não interrompe o lote do "Otimizar agora".
    pub fn failed(id: &str, name: &str, error: String) -> Self {
        Self::novo(id, name, error)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchStep {
    pub index: usize,
    pub total: usize,
    pub name: String,
    pub stage: &'static str,
    pub message: String,
    pub changes: Vec<String>,
    pub success: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cada_coisa_que_pode_ser_custada_tem_rotulo_proprio() {
        let todos = [
            OQuePodeCustar::FpsMedio,
            OQuePodeCustar::UmPorCentoPior,
            OQuePodeCustar::Engasgo,
            OQuePodeCustar::Boost,
        ];

        let mut rotulos: Vec<&str> = todos.iter().map(|o| o.rotulo()).collect();
        let antes = rotulos.len();
        rotulos.sort();
        rotulos.dedup();

        assert_eq!(rotulos.len(), antes, "dois deles dizem a mesma frase ao cliente");

        for o in todos {
            assert!(!o.rotulo().trim().is_empty(), "{o:?} sem rótulo");
        }
    }

    /// Se `pode_custar` deixar de reconhecer a variante, o botão grande volta a aplicar o que derrubou o FPS na 2.1.0.
    #[test]
    fn todo_risco_declarado_e_reconhecido_como_risco() {
        assert!(RiscoDeFps::custa(OQuePodeCustar::Boost, "x").pode_custar());
        assert!(RiscoDeFps::custa(OQuePodeCustar::Engasgo, "x").pode_custar());
        assert!(!RiscoDeFps::Nenhum.pode_custar());
    }
}
