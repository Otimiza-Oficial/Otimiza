// Contrato de otimização
//
// Tipos compartilhados entre a interface e os otimizadores de cada plataforma.
// A implementação concreta vive em `modules::windows` (e futuramente linux/macos);
// aqui fica apenas o formato de dados que a UI consome.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Category {
    System,
    Gaming,
    Network,
    Startup,
    Privacy,
}

/// Quanto de ganho real esperar. Deliberadamente conservador:
/// prometer menos e entregar o medido é o diferencial do produto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExpectedGain {
    /// Ganho mensurável em FPS ou tempo de resposta na maioria das máquinas.
    Measurable,
    /// Ganho pequeno, perceptível principalmente em PCs fracos ou casos específicos.
    Situational,
    /// Não muda FPS: melhora a sensação de resposta ou libera recursos de fundo.
    Responsiveness,
}

/// Situação de uma otimização nesta máquina.
///
/// `AlreadyOptimal` existe por honestidade comercial: quando o PC do cliente já
/// está configurado daquele jeito, o produto diz isso em vez de fingir que
/// "otimizou" e cobrar por trabalho que não houve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptimizationState {
    /// Aplicada por nós e registrada no histórico.
    Applied,
    /// O sistema já estava assim antes de existirmos.
    AlreadyOptimal,
    /// Pode ser aplicada.
    Available,
    /// Não se aplica a esta máquina (serviço inexistente, hardware sem suporte).
    Unavailable,
}

/// Descrição de uma otimização enviada para a interface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    /// O que o cliente realmente deve esperar, sem maquiagem.
    pub honest_effect: String,
    pub category: Category,
    pub expected_gain: ExpectedGain,
    pub requires_admin: bool,
    pub requires_restart: bool,
    pub reversible: bool,
    /// Troca segurança por desempenho. A interface avisa em vermelho e o lote
    /// automático não inclui.
    pub security_tradeoff: bool,
    /// Pesa muito mais nesta máquina do que na média, segundo o hardware detectado.
    /// Não é promessa de milagre — é dizer o que vale a pena AQUI.
    pub recommended: bool,
    pub state: OptimizationState,
    /// Informação medida agora nesta máquina, quando existir.
    /// Ex.: "1,4 GB de temporários para limpar".
    pub detail: Option<String>,
}

/// Resultado de aplicar ou desfazer uma otimização.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationOutcome {
    pub id: String,
    pub name: String,
    pub success: bool,
    /// Estado da otimização depois da operação.
    pub applied: bool,
    pub message: String,
    pub requires_restart: bool,
    pub changes_count: usize,
    /// O que exatamente foi alterado, em português. Aparece no registro ao vivo:
    /// o cliente vê cada mexida em vez de confiar numa barra de progresso.
    pub changes: Vec<String>,
}

impl OptimizationOutcome {
    /// Resultado de uma otimização que falhou, para que um erro isolado não
    /// interrompa o lote inteiro no modo "Otimizar agora".
    pub fn failed(id: &str, name: &str, error: String) -> Self {
        OptimizationOutcome {
            id: id.to_string(),
            name: name.to_string(),
            success: false,
            applied: false,
            message: error,
            requires_restart: false,
            changes_count: 0,
            changes: Vec::new(),
        }
    }
}

/// Um passo do lote, emitido ao vivo para a interface.
///
/// Existe para o cliente ver o que está acontecendo em vez de encarar uma barra
/// de progresso. Barra de progresso é o que os concorrentes mostram justamente
/// porque não têm nada real para exibir.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchStep {
    pub index: usize,
    pub total: usize,
    pub name: String,
    /// `started` quando começa, `finished` quando termina.
    pub stage: &'static str,
    pub message: String,
    /// O que foi alterado, item por item.
    pub changes: Vec<String>,
    pub success: bool,
}
