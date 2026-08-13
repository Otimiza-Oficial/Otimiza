// Vocabulário comum de achados
//
// Cada módulo de diagnóstico do Otimiza nasceu sozinho e responde bem à sua
// própria pergunta. O problema apareceu quando o produto foi testado de verdade:
// numa máquina que travava, `firmware` sabia que a memória estava em canal
// único e `memory` sabia que os programas pediam mais do que existe. As duas
// coisas são a MESMA causa, e o cliente nunca viu as duas juntas — cada uma
// morava numa aba diferente, atrás de um botão diferente.
//
// Este arquivo é o vocabulário único que permite juntá-las. Ele não substitui
// nenhum módulo: cada um continua com sua struct e seu painel. O que ele
// acrescenta são dois campos que faltavam para poder comparar achados de
// origens diferentes:
//
//   `causa`     — agrupa achados que apontam para o mesmo problema físico.
//   `confianca` — separa foto do instante de evidência que persiste.
//
// `FindingSeverity` e `FixLocation` moravam em `firmware.rs` e eram importados
// de lá por meia dúzia de módulos. Foram movidos para cá porque agora servem ao
// produto inteiro, e não mais ao diagnóstico de firmware em particular —
// `firmware.rs` continua reexportando os dois, então nada quebra.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FindingSeverity {
    /// Custa desempenho de forma grande e comprovada.
    Critical,
    /// Vale corrigir, com ganho menor ou dependente do caso.
    Important,
    /// Está correto. Dizer o que está certo evita vender conserto de coisa boa.
    Ok,
}

/// Onde o problema se resolve. Serve para o produto não fingir que conserta o
/// que só se resolve trocando peça ou entrando na BIOS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FixLocation {
    /// Dá para corrigir por software, aqui mesmo.
    Software,
    /// Só na configuração da BIOS/UEFI, na mão, com o PC reiniciando.
    Bios,
    /// Só trocando ou acrescentando peça.
    Hardware,
    /// Nada a corrigir.
    None,
}

/// De qual diagnóstico o achado veio. Serve para o cliente poder conferir, e
/// para o produto saber o que recolher de novo quando algo muda.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Origem {
    Memoria,
    Firmware,
    Saude,
    Conflitos,
    Prontidao,
    Gargalo,
    Termico,
    Boot,
    Disco,
    /// Monitor: resolução e taxa de atualização.
    Monitor,
    /// Qual placa de vídeo cada jogo usa. Só existe em máquina com duas placas.
    PlacaDeVideo,
    /// A janela de observação contínua dos últimos dias, amostrada pelo próprio
    /// Otimiza enquanto fica aberto.
    Pressao,
    /// O registro de eventos do próprio Windows. Origem separada porque é a
    /// única evidência do produto que o cliente confere sozinho, no
    /// Visualizador de Eventos, sem depender da nossa palavra.
    Esgotamento,
}

/// O problema físico por trás do achado.
///
/// É este campo que junta na mesma tela o canal único (que mora em `firmware`)
/// e a memória prometida acima da física (que mora em `memory`). Sem ele, dois
/// sintomas da mesma causa continuam sendo duas linhas soltas em abas
/// diferentes — que foi exatamente como o produto falhou.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Causa {
    Memoria,
    Armazenamento,
    Refrigeracao,
    Configuracao,
    Conflito,
    /// O sintoma é real e medido, mas a causa tem mais de uma explicação
    /// possível e não temos como escolher entre elas.
    ///
    /// Existe para o produto poder mostrar um fato sem inventar o motivo. Um
    /// programa que parou de responder, por exemplo, pode ser falta de memória,
    /// disco lento ou defeito do próprio programa — agrupá-lo sob "memória"
    /// afirmaria uma relação que ninguém mediu.
    Indefinida,
}

/// Quanto peso a evidência aguenta.
///
/// A distinção existe por um motivo concreto: o cliente abre o Otimiza com o
/// jogo FECHADO. Uma medição do instante do clique não vê o travamento de
/// ontem à noite; uma marca d'água desde o boot vê. Quando as duas discordam,
/// quem persiste ganha.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Confianca {
    /// O próprio Windows declarou o problema, com data e hora, no registro de
    /// eventos. É o nível mais alto que existe, e o único que o cliente pode
    /// conferir sozinho sem depender da palavra do Otimiza.
    Declarado,
    /// Lido agora, direto do sistema. Vale para o estado atual.
    Medido,
    /// Registro do que já aconteceu: marca d'água, log de eventos, amostragem
    /// acumulada. Não depende de o problema estar acontecendo na hora.
    Historico,
    /// Deduzido de uma configuração, sem medir o efeito. Nunca sustenta sozinho
    /// uma afirmação sobre travamento.
    Inferido,
}

/// O conserto, quando o próprio Otimiza sabe fazer.
///
/// Sem isto, o veredito é um diagnóstico que manda o cliente procurar o botão
/// certo em alguma das cinco abas — que é metade do problema que este trabalho
/// veio resolver. Com isto, o que dá para consertar se conserta ali mesmo.
///
/// Fica `None` de propósito na maioria dos achados: acrescentar memória, trocar
/// disco ou entrar na BIOS não são coisas que um programa faça, e inventar um
/// botão para elas seria prometer o que não se cumpre.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Acao {
    /// Comando que a interface invoca.
    pub comando: String,
    /// Argumento único, quando o comando pede um.
    pub argumento: Option<String>,
    /// O que o botão diz. Verbo no infinitivo, sem promessa de resultado.
    pub rotulo: String,
    pub exige_admin: bool,
}

/// Achado normalizado, comparável entre módulos.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Achado {
    pub id: String,
    pub origem: Origem,
    pub causa: Causa,
    pub title: String,
    /// O que foi medido nesta máquina, com números. Nunca pode ficar vazio:
    /// afirmação sem número medido é a coisa que este produto não faz.
    pub measured: String,
    pub advice: String,
    pub severity: FindingSeverity,
    pub fix_location: FixLocation,
    pub confianca: Confianca,
    /// Preenchido só quando o Otimiza sabe consertar isto sozinho.
    pub acao: Option<Acao>,
}

/// O que não deu para saber.
///
/// Existe para que "não medimos" nunca vire "está tudo bem". Um diagnóstico que
/// falhou por falta de administrador precisa aparecer na tela dizendo isso —
/// silêncio, aqui, é indistinguível de aprovação.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lacuna {
    pub origem: Origem,
    /// O que não pôde ser verificado, em português para o cliente.
    pub o_que: String,
    /// Por que não deu, e o que fazer para dar.
    pub por_que: String,
}

/// Conversão para o vocabulário comum.
///
/// É um trait e não uma troca das structs de propósito: cada `*Report` já é
/// consumido por um painel próprio em `main.ts` e pelo relatório em PDF.
/// Trocar as structs públicas quebraria frontend e relatório de uma vez; o
/// trait é aditivo e cada módulo entra no seu tempo.
pub trait EmAchados {
    fn achados(&self) -> Vec<Achado>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severidade_ordena_do_pior_para_o_melhor() {
        assert!(peso_severidade(FindingSeverity::Critical) < peso_severidade(FindingSeverity::Important));
        assert!(peso_severidade(FindingSeverity::Important) < peso_severidade(FindingSeverity::Ok));
    }

    #[test]
    fn evidencia_que_persiste_pesa_mais_que_foto_do_instante() {
        // O cliente abre o Otimiza com o jogo fechado. Se o instantâneo
        // ganhasse do histórico, o produto voltaria a dizer "sem problemas"
        // para a máquina que trava à noite. E o que o próprio Windows declarou,
        // com data e hora, vence o que nós deduzimos da marca d'água.
        assert!(peso_confianca(Confianca::Declarado) < peso_confianca(Confianca::Historico));
        assert!(peso_confianca(Confianca::Historico) < peso_confianca(Confianca::Medido));
        assert!(peso_confianca(Confianca::Medido) < peso_confianca(Confianca::Inferido));
    }
}

/// Menor é pior. Usado pelo veredito para ordenar.
pub fn peso_severidade(s: FindingSeverity) -> u8 {
    match s {
        FindingSeverity::Critical => 0,
        FindingSeverity::Important => 1,
        FindingSeverity::Ok => 2,
    }
}

/// Menor aguenta mais peso.
pub fn peso_confianca(c: Confianca) -> u8 {
    match c {
        Confianca::Declarado => 0,
        Confianca::Historico => 1,
        Confianca::Medido => 2,
        Confianca::Inferido => 3,
    }
}
