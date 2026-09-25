// O vocabulário comum de achados, para juntar sintomas da mesma causa vindos de módulos diferentes (canal único
// em `firmware`, memória prometida acima da física em `memory`). Acrescenta `causa` (o problema físico) e
// `confianca` (foto do instante ou evidência que persiste).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FindingSeverity {
    Critical,
    Important,
    /// Dizer o que está certo evita vender conserto de coisa boa.
    Ok,
}

/// Para o produto não fingir que conserta o que só se resolve trocando peça ou na BIOS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FixLocation {
    Software,
    Bios,
    Hardware,
    None,
}

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
    Monitor,
    ConfigDoJogo,
    PlacaDeVideo,
    Pressao,
    /// A única evidência que o cliente confere sozinho, no Visualizador de Eventos.
    Esgotamento,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Causa {
    Memoria,
    Armazenamento,
    Refrigeracao,
    Configuracao,
    Conflito,
    /// Sintoma medido com mais de uma causa possível: mostra o fato sem inventar o motivo.
    Indefinida,
}

/// O cliente abre o Otimiza com o jogo FECHADO: quem persiste (marca d'água, log) vence a foto do instante.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Confianca {
    Declarado,
    Medido,
    Historico,
    /// Nunca sustenta sozinho uma afirmação sobre travamento.
    Inferido,
}

/// `None` na maioria: memória, disco e BIOS não são coisas que um programa conserte.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Acao {
    pub comando: String,
    pub argumento: Option<String>,
    /// Verbo no infinitivo, sem promessa de resultado.
    pub rotulo: String,
    pub exige_admin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Achado {
    pub id: String,
    pub origem: Origem,
    pub causa: Causa,
    pub title: String,
    /// Nunca vazio: afirmação sem número medido é o que este produto não faz.
    pub measured: String,
    pub advice: String,
    pub severity: FindingSeverity,
    pub fix_location: FixLocation,
    pub confianca: Confianca,
    pub acao: Option<Acao>,
}

/// "Não medimos" nunca vira "está tudo bem": silêncio é indistinguível de aprovação.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lacuna {
    pub origem: Origem,
    pub o_que: String,
    pub por_que: String,
}

/// Trait e não troca das structs: cada `*Report` já alimenta um painel e o PDF.
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
        assert!(peso_confianca(Confianca::Declarado) < peso_confianca(Confianca::Historico));
        assert!(peso_confianca(Confianca::Historico) < peso_confianca(Confianca::Medido));
        assert!(peso_confianca(Confianca::Medido) < peso_confianca(Confianca::Inferido));
    }
}

pub fn peso_severidade(s: FindingSeverity) -> u8 {
    match s {
        FindingSeverity::Critical => 0,
        FindingSeverity::Important => 1,
        FindingSeverity::Ok => 2,
    }
}

pub fn peso_confianca(c: Confianca) -> u8 {
    match c {
        Confianca::Declarado => 0,
        Confianca::Historico => 1,
        Confianca::Medido => 2,
        Confianca::Inferido => 3,
    }
}
