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
    /// Não muda desempenho nenhum.
    ///
    /// Existe porque o cliente compara lista com lista, e boa parte do que o
    /// mercado chama de "otimização" é isto: desligar sincronização,
    /// atualização automática de mapa, notificação. São ajustes legítimos de
    /// higiene e privacidade, e não devolvem um quadro por segundo.
    ///
    /// Sem este nível, esses itens teriam que entrar como `Responsiveness`, e
    /// aí o rótulo "não muda FPS" passaria a cobrir duas coisas diferentes:
    /// "libera recurso de fundo" e "não faz nada de desempenho". Um cliente que
    /// aplica trinta itens e não sente diferença precisa conseguir saber, antes
    /// de clicar, quantos deles nunca prometeram diferença.
    NoGain,
}

/// Se um ajuste pode DERRUBAR o FPS em alguma máquina.
///
/// NASCEU DE UM PREJUÍZO REAL. Na 2.1.0 um cliente aplicou tudo que o produto
/// oferece e caiu de cerca de 200 para 80-120 FPS no FiveM. Um dos culpados
/// foi um erro de valor meu, já corrigido — mas a investigação mostrou uma
/// falha maior e estrutural: **o catálogo não tinha como dizer que um ajuste
/// pode piorar o FPS.**
///
/// `ExpectedGain` responde "quanto isso ajuda" e tem `NoGain` como piso. Não
/// existe degrau abaixo de zero. Então um ajuste que troca FPS por latência, ou
/// que ajuda numa máquina e atrapalha em outra, entrava no "Otimizar agora"
/// exatamente como um que só tem a ganhar — e o cliente descobria a diferença
/// olhando o contador de quadros.
///
/// A regra que este tipo carrega: **o botão grande só aplica o que não pode
/// custar quadro.** O resto continua no catálogo, item a item, dizendo em voz
/// alta o que está trocando pelo quê. Ver `catalog::entra_no_lote`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "risco", content = "quando")]
pub enum RiscoDeFps {
    /// Não mexe no caminho que desenha o quadro, ou mexe só para o bem.
    Nenhum,
    /// Pode custar quadros em parte das máquinas. Fica FORA do lote automático.
    ///
    /// O texto diz EM QUE CASO ele custa — não "pode variar", que não ajuda
    /// ninguém a decidir. Ele vai para a tela do jeito que está escrito.
    /// `Cow` e não `&'static str` porque `OptimizationInfo` precisa ser
    /// desserializável: o catálogo constrói `Borrowed` de graça, e o que volta
    /// do outro lado do IPC vira `Owned`.
    PodeCustar(std::borrow::Cow<'static, str>),
}

impl RiscoDeFps {
    /// O jeito de escrever um risco no catálogo, sem `Cow` em toda linha.
    pub const fn custa(quando: &'static str) -> Self {
        RiscoDeFps::PodeCustar(std::borrow::Cow::Borrowed(quando))
    }

    pub fn pode_custar(&self) -> bool {
        matches!(self, RiscoDeFps::PodeCustar(_))
    }
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
    /// NÃO DEU PARA VERIFICAR o estado nesta máquina.
    ///
    /// Separado de `Unavailable` porque os dois diziam a mesma frase ao cliente
    /// — "não se aplica a esta máquina" — e só um deles era verdade. O outro era
    /// uma leitura que falhou, o que é comum justamente onde este produto mais
    /// precisa funcionar: imagem modificada, permissão negada, política de
    /// domínio.
    ///
    /// Também não é `AlreadyOptimal`: dizer "seu PC já está assim" sobre um
    /// valor que não foi lido é a mentira mais cara que um otimizador pode
    /// contar, porque o cliente para de procurar.
    Unknown,
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
    /// Se este ajuste pode DERRUBAR o FPS em alguma máquina, e em qual caso.
    /// Quando pode, ele fica fora do "Otimizar agora" e a tela diz por quê.
    pub risco_de_fps: RiscoDeFps,
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

/// O desfecho de UMA AÇÃO, com o que é preciso para reproduzir a falha numa
/// máquina que não está na sua frente.
///
/// Os campos são os que a nota de arquitetura do projeto define, e cada um tem
/// uma pergunta por trás: o que havia antes, o que pedimos, o que ficou, o que
/// o comando devolveu, e quanto demorou. Um `message` sozinho responde só a
/// última pergunta do atendimento, e nunca a primeira.
///
/// `Option` em vez de string vazia onde o dado pode não existir: um ajuste que
/// não roda comando nenhum não tem código de saída, e escrever `0` ali seria
/// afirmar que um comando deu certo.
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
    /// POR QUE não se aplica aqui. `Unsupported` sem motivo manda o cliente
    /// perguntar exatamente o que este campo responderia.
    pub unsupported_reason: Option<String>,
}

/// O que aconteceu com uma ação.
///
/// O VOCABULÁRIO É O DO PROTOCOLO DE COMPATIBILIDADE do projeto, e não um
/// inventado aqui — as notas de laboratório classificam com estes mesmos
/// termos, e duas listas diferentes de palavras para a mesma coisa é como o
/// relatório do cliente deixa de se agrupar com o do laboratório.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionStatus {
    /// Escrito E CONFERIDO relendo do Windows.
    #[default]
    Verified,
    /// A máquina já estava assim. Nada foi escrito.
    AlreadyOptimized,
    /// Este Windows ou este hardware não tem isto. NÃO É FALHA.
    Unsupported,
    /// O comando foi recusado.
    Failed,
    /// O comando foi aceito e o valor relido não é o pedido.
    ///
    /// Separado de `Failed` porque é outra conversa: quase sempre é política de
    /// domínio ou outro programa reescrevendo, e não há nada a consertar no
    /// produto.
    VerificationFailed,
    /// Escrito, e não deu para reler para confirmar.
    NotConfirmed,
    /// O valor não ficou, E esta máquina tem política de grupo aplicada.
    ///
    /// É o `VerificationFailed` com a causa provável nomeada. Só é usado quando
    /// há GPO de verdade na máquina — sem isso continua `VerificationFailed`,
    /// porque acusar política sem política seria mandar o cliente falar com um
    /// administrador que não existe.
    BlockedByPolicy,
    /// Não existe nesta máquina PORQUE QUEM MONTOU A IMAGEM TIROU.
    ///
    /// Diferente de `Unsupported`: ali o Windows não tem o recurso; aqui o
    /// Windows tem e a imagem instalada não. Pelo cliente, são duas frases
    /// muito diferentes — a segunda explica por que o mesmo PC, com um Windows
    /// normal, se comportaria de outro jeito.
    UserOrOemManaged,
    /// Não se aplica a esta máquina.
    Skipped,
}

impl ActionStatus {
    /// Esta ação deu certo do ponto de vista de quem clicou?
    ///
    /// `Unsupported` conta como certo de propósito: um ajuste que este Windows
    /// não tem não é um problema do cliente nem do produto, e pintá-lo de
    /// vermelho manda procurar defeito onde não há.
    pub fn deu_certo(&self) -> bool {
        matches!(
            self,
            ActionStatus::Verified
                | ActionStatus::AlreadyOptimized
                | ActionStatus::Unsupported
                // Pelo mesmo motivo do `Unsupported`: o recurso não está aqui
                // porque quem montou a imagem tirou, e não há o que o produto
                // ou o cliente consertem. Vermelho mandaria procurar defeito.
                | ActionStatus::UserOrOemManaged
                | ActionStatus::NotConfirmed
                | ActionStatus::Skipped
        )
    }
}

/// Resultado de aplicar ou desfazer uma otimização.
///
/// `Default` existe para que os campos novos do resultado padronizado tenham um
/// estado neutro em cada ponto de construção, e não para ser usado como
/// resultado de verdade: um resultado com `success: false` e mensagem vazia não
/// diz nada a ninguém.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OptimizationOutcome {
    pub id: String,
    pub name: String,
    pub success: bool,
    /// Estado da otimização depois da operação.
    pub applied: bool,
    pub message: String,
    pub requires_restart: bool,
    /// Exige SAIR E ENTRAR na conta, que não é a mesma coisa que reiniciar.
    ///
    /// Campo e não status, de propósito: uma otimização pode ter sido aplicada
    /// E exigir logoff, e as duas informações precisam caber juntas. Como
    /// status, uma apagaria a outra.
    #[serde(default)]
    pub requires_logoff: bool,
    /// Quanto a otimização inteira levou. É o que separa "falhou" de "ficou
    /// pendurada" quando o cliente manda o registro.
    #[serde(default)]
    pub duration_ms: u64,
    pub changes_count: usize,
    /// O que exatamente foi alterado, em português. Aparece no registro ao vivo:
    /// o cliente vê cada mexida em vez de confiar numa barra de progresso.
    pub changes: Vec<String>,
    /// Uma linha por ação, com antes, esperado, depois, código de saída e
    /// saída do comando.
    ///
    /// NÃO SUBSTITUI `changes`: aquele é a lista em português que o cliente lê
    /// no registro ao vivo; esta é a evidência que o atendimento lê. Juntar as
    /// duas faria uma das leituras piorar.
    #[serde(default)]
    pub actions: Vec<ActionResult>,
}

impl OptimizationOutcome {
    /// O esqueleto de um resultado, com os campos novos no estado neutro.
    ///
    /// Existe para que acrescentar um campo ao resultado padronizado não vire
    /// uma edição em seis lugares — que é exatamente como um deles acaba com o
    /// valor errado e ninguém percebe.
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

    /// Resultado de uma otimização que falhou, para que um erro isolado não
    /// interrompa o lote inteiro no modo "Otimizar agora".
    pub fn failed(id: &str, name: &str, error: String) -> Self {
        Self::novo(id, name, error)
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
