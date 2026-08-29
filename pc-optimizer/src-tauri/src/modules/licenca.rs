// A licença
//
// COMO ISTO FUNCIONA, EM UMA FRASE
//
// O dono assina uma licença com uma chave que só ele tem; o programa confere a
// assinatura com uma chave que qualquer um pode ver. Assinar e conferir são
// operações diferentes, e a chave que viaja no instalador só sabe conferir.
//
// POR QUE NÃO É UM SEGREDO DENTRO DO PROGRAMA
//
// O jeito comum de fazer chave de licença é guardar no programa o segredo que
// gera a chave, e comparar. O problema é que esse segredo viaja dentro do
// executável que o cliente baixa: quem abrir o arquivo acha o segredo e escreve
// um gerador. Foi assim que praticamente todo software dos anos 90 foi
// pirateado.
//
// Aqui o programa carrega apenas a chave PÚBLICA. Ela confere assinatura e não
// cria nenhuma. Extraí-la do executável não serve para nada.
//
// O FORMATO DA CHAVE
//
// Duas partes separadas por ponto, no mesmo espírito de um token da web:
//
//     <dados em base64>.<assinatura em base64>
//
// Os dados são legíveis por quem quiser olhar — não há segredo neles. O que
// impede alteração é a assinatura: mudar um caractere dos dados invalida.
//
// O QUE ISTO NÃO FAZ, E PRECISA ESTAR ESCRITO
//
// Licença conferida no PC do cliente pode ser contornada editando o executável
// e arrancando a conferência. Nenhuma é inquebrável. O que esta entrega é
// impedir o repasse casual — a chave do vizinho não abre aqui — e exigir
// habilidade real de quem quiser quebrar.

use base64::Engine;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// A chave pública do Otimiza, em base64.
///
/// PODE aparecer no código, no executável e em qualquer lugar público: é o que
/// confere assinatura, não o que cria. A chave privada correspondente vive na
/// máquina do dono e no segredo do bot, e nunca entra neste repositório.
///
/// Trocar este valor invalida TODAS as licenças já emitidas.
/// ATENÇÃO — ESTA É UMA CHAVE DE TESTE E PRECISA SER TROCADA ANTES DE VENDER.
///
/// A privada correspondente foi impressa num terminal durante o
/// desenvolvimento, o que a torna pública para efeitos práticos. Quem a tiver
/// consegue emitir licença.
///
/// Antes da primeira venda:
///
///     cargo run --example gerar_chave -- novo-par
///
/// e cole aqui a PÚBLICA que sair, num terminal que só você vê.
const CHAVE_PUBLICA: &str = "C0fmvRgj2Sb01AfppfzEx7VTlhc3VnvNF3qqYbq8nLA=";

/// O que a licença afirma.
///
/// Tudo aqui é público: qualquer um que tenha a chave pode ler estes campos. O
/// que a assinatura garante não é sigilo, é que ninguém mudou nada.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Dados {
    /// O identificador da máquina para a qual esta licença foi emitida.
    /// É o que faz a chave não valer em outro PC.
    pub maquina: String,
    /// Nome ou identificação do comprador. Só para o dono saber de quem é.
    pub comprador: String,
    /// Quando foi emitida, em "AAAA-MM-DD".
    pub emitida: String,
    /// Quando expira, quando expira. `None` é vitalícia.
    pub expira: Option<String>,
}

/// Por que a licença não vale.
///
/// Cada motivo tem um texto próprio porque o cliente que pagou merece saber a
/// diferença entre "digitei errado" e "esta chave é de outro PC".
#[derive(Debug, Clone, PartialEq)]
pub enum Recusa {
    /// Não tem o formato de uma chave.
    Malformada,
    /// A assinatura não confere. Ou foi alterada, ou não saiu daqui.
    AssinaturaInvalida,
    /// A chave é válida, mas foi emitida para outra máquina.
    OutraMaquina { emitida_para: String },
    /// A chave venceu.
    Expirada { em: String },
    /// Não foi possível identificar esta máquina.
    MaquinaDesconhecida,
}

impl Recusa {
    /// O que o cliente lê na tela.
    pub fn explicacao(&self) -> String {
        match self {
            Recusa::Malformada => "Esta chave não está completa. Confira se copiou tudo, \
                 do começo ao fim, sem espaço sobrando."
                .to_string(),

            Recusa::AssinaturaInvalida => "Esta chave não foi reconhecida. Ou faltou um \
                 pedaço na cópia, ou ela não foi emitida pelo Otimiza."
                .to_string(),

            Recusa::OutraMaquina { .. } => "Esta chave foi emitida para outro computador. \
                 Cada chave vale em uma máquina só — é o que impede que ela seja repassada. \
                 Se você trocou de PC ou de placa-mãe, fale no Discord com o código desta \
                 máquina que a gente emite outra."
                .to_string(),

            Recusa::Expirada { em } => format!("Esta chave venceu em {}.", em),

            Recusa::MaquinaDesconhecida => "Não foi possível identificar este computador, \
                 então não dá para conferir a chave. Isso costuma acontecer em máquina \
                 virtual."
                .to_string(),
        }
    }
}

/// Separa a chave nas duas partes e devolve os bytes.
///
/// **Função pura.** Não confere assinatura — só desmonta.
pub fn desmontar(chave: &str) -> Result<(Vec<u8>, Vec<u8>), Recusa> {
    let motor = base64::engine::general_purpose::URL_SAFE_NO_PAD;

    // Espaço e quebra de linha entram sempre que alguém copia de uma mensagem
    // do Discord. Tirar é mais gentil que recusar.
    let limpa: String = chave.chars().filter(|c| !c.is_whitespace()).collect();

    let (dados, assinatura) = limpa.split_once('.').ok_or(Recusa::Malformada)?;

    if dados.is_empty() || assinatura.is_empty() {
        return Err(Recusa::Malformada);
    }

    let dados = motor.decode(dados).map_err(|_| Recusa::Malformada)?;
    let assinatura = motor.decode(assinatura).map_err(|_| Recusa::Malformada)?;

    Ok((dados, assinatura))
}

/// Confere a chave contra uma máquina e uma data.
///
/// **Função pura**, e é de propósito: a decisão que separa cliente pagante de
/// não pagante precisa ser testável sem depender da máquina de quem roda os
/// testes nem da data de hoje.
pub fn conferir(chave: &str, maquina: &str, hoje: &str) -> Result<Dados, Recusa> {
    conferir_com(chave_publica()?, chave, maquina, hoje)
}

/// O mesmo de [`conferir`], mas recebendo a chave pública em vez de usar a
/// constante.
///
/// Existe por causa dos testes. Um teste que prove que a assinatura está sendo
/// conferida de verdade precisa ASSINAR alguma coisa, e assinar exige a chave
/// privada — que não pode viver neste arquivo. Amarrar o teste à
/// [`CHAVE_PUBLICA`] também não serve: no dia em que ela for trocada, os testes
/// quebrariam sem nada de errado ter acontecido.
///
/// Com a chave como parâmetro, o teste gera o próprio par, assina, confere, e
/// não depende de nenhuma chave de verdade.
pub fn conferir_com(
    publica: VerifyingKey,
    chave: &str,
    maquina: &str,
    hoje: &str,
) -> Result<Dados, Recusa> {
    if maquina.is_empty() {
        return Err(Recusa::MaquinaDesconhecida);
    }

    let (dados_bytes, assinatura_bytes) = desmontar(chave)?;

    // A ORDEM IMPORTA. A assinatura é conferida ANTES de qualquer campo ser
    // lido como verdade. Ler primeiro e conferir depois deixaria o programa
    // tomar decisão com dado que ainda não se sabe se é legítimo.

    let assinatura: [u8; 64] = assinatura_bytes
        .try_into()
        .map_err(|_| Recusa::Malformada)?;

    publica
        .verify(&dados_bytes, &Signature::from_bytes(&assinatura))
        .map_err(|_| Recusa::AssinaturaInvalida)?;

    let dados: Dados =
        serde_json::from_slice(&dados_bytes).map_err(|_| Recusa::AssinaturaInvalida)?;

    if dados.maquina != maquina {
        return Err(Recusa::OutraMaquina {
            emitida_para: dados.maquina.clone(),
        });
    }

    if let Some(vencimento) = &dados.expira {
        // Data em "AAAA-MM-DD" compara certo como texto, e não precisa de
        // biblioteca de calendário para isso.
        if hoje > vencimento.as_str() {
            return Err(Recusa::Expirada {
                em: vencimento.clone(),
            });
        }
    }

    Ok(dados)
}

fn chave_publica() -> Result<VerifyingKey, Recusa> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(CHAVE_PUBLICA)
        .map_err(|_| Recusa::AssinaturaInvalida)?;

    let bytes: [u8; 32] = bytes.try_into().map_err(|_| Recusa::AssinaturaInvalida)?;

    VerifyingKey::from_bytes(&bytes).map_err(|_| Recusa::AssinaturaInvalida)
}

// ------------------------------------------------------------------- em disco

/// A licença guardada, do jeito que fica em disco.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Guardada {
    pub chave: String,
}

impl Guardada {
    /// Mesmo padrão dos outros cinco arquivos do produto.
    fn path() -> PathBuf {
        let base = std::env::var("APPDATA")
            .or_else(|_| std::env::var("HOME"))
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));

        base.join("pc-optimizer").join("licenca.json")
    }

    pub fn load() -> Self {
        fs::read_to_string(Self::path())
            .ok()
            .and_then(|bruto| serde_json::from_str(&bruto).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        let caminho = Self::path();

        if let Some(pasta) = caminho.parent() {
            fs::create_dir_all(pasta)
                .map_err(|e| format!("Não foi possível criar a pasta de dados: {}", e))?;
        }

        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Não foi possível gravar a licença: {}", e))?;

        fs::write(&caminho, json).map_err(|e| format!("Não foi possível gravar a licença: {}", e))
    }
}

/// O estado da licença desta máquina, para a tela.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Estado {
    pub ativa: bool,
    /// O código desta máquina, que o cliente manda no Discord.
    pub maquina: String,
    /// De onde o código foi tirado, em português. A tela mostra isso porque
    /// "OTZ-WPYY-0J4F-77AB" sozinho não explica nada a quem está comprando.
    pub origem: String,
    /// Se este código sobrevive a uma formatação.
    pub sobrevive_formatacao: bool,
    pub comprador: Option<String>,
    pub expira: Option<String>,
    /// Por que não está ativa, em português, quando não está.
    pub motivo: Option<String>,
}

fn hoje() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

/// Lê a licença guardada e diz se ela vale AGORA.
///
/// É conferida a cada chamada, e não uma vez na abertura: uma licença com prazo
/// vence enquanto o programa está aberto, e guardar a resposta faria o produto
/// continuar liberado depois do vencimento.
pub fn estado() -> Estado {
    let quem = crate::modules::maquina::identidade();
    let guardada = Guardada::load();

    let base = Estado {
        ativa: false,
        maquina: quem.id.clone(),
        origem: quem.fonte.descricao().to_string(),
        sobrevive_formatacao: quem.fonte.sobrevive_formatacao(),
        comprador: None,
        expira: None,
        motivo: None,
    };

    if guardada.chave.trim().is_empty() {
        return base;
    }

    match conferir(&guardada.chave, &quem.id, &hoje()) {
        Ok(dados) => Estado {
            ativa: true,
            comprador: Some(dados.comprador),
            expira: dados.expira,
            ..base
        },
        Err(recusa) => Estado {
            motivo: Some(recusa.explicacao()),
            ..base
        },
    }
}

/// Guarda uma chave, depois de conferir que ela vale.
///
/// Gravar primeiro e conferir depois deixaria o disco com chave inválida, e a
/// próxima abertura mostraria um erro que o cliente não provocou.
pub fn ativar(chave: &str) -> Result<Dados, String> {
    let quem = crate::modules::maquina::identidade();

    let dados = conferir(chave, &quem.id, &hoje()).map_err(|r| r.explicacao())?;

    Guardada {
        chave: chave.trim().to_string(),
    }
    .save()?;

    Ok(dados)
}

/// O produto está liberado?
///
/// É esta função que os comandos que ALTERAM o sistema consultam.
pub fn liberado() -> bool {
    estado().ativa
}

/// A guarda dos comandos que ALTERAM o sistema.
///
/// Devolve erro em português quando não há licença, e esse erro sobe até a
/// tela do jeito que qualquer outro erro sobe — sem tratamento especial no
/// caminho.
///
/// POR QUE AQUI E NÃO SÓ NA TELA
///
/// A tela é HTML rodando dentro de uma janela que tem ferramentas de
/// desenvolvedor. Esconder um botão não impede ninguém de chamar o comando por
/// trás dele. Esta função é o ponto onde a decisão realmente acontece.
///
/// É conferida a cada chamada, e não uma vez na abertura, porque uma licença
/// com prazo vence enquanto o programa está aberto.
pub fn exigir() -> Result<(), String> {
    if liberado() {
        return Ok(());
    }

    Err("Esta ação faz parte do Otimiza completo, e este computador ainda não          está ativado. O diagnóstico continua livre: o que você viu sobre esta          máquina é real. Para liberar as correções, ative com a sua chave."
        .to_string())
}

/// As provas de ponta a ponta da assinatura. Só existe em compilação de teste.
#[cfg(test)]
#[path = "licenca_prova.rs"]
mod prova;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chave_privada_nunca_entra_no_produto() {
        // A conferência mais importante deste arquivo. Se a chave privada
        // aparecer aqui um dia, qualquer pessoa que baixe o instalador passa a
        // conseguir emitir licença — e o sistema inteiro deixa de valer.
        //
        // A busca é só na parte de PRODUÇÃO do arquivo, e a razão é dupla:
        // código de teste não entra no executável do cliente, e a própria
        // lista de palavras proibidas mora no teste — procurar no arquivo
        // inteiro faria a guarda se encontrar e reprovar sozinha.
        let fonte = include_str!("licenca.rs");

        let producao = fonte
            .split("#[cfg(test)]")
            .next()
            .expect("split sempre devolve ao menos um pedaço");

        assert!(
            producao.len() < fonte.len(),
            "não achei onde termina a produção e começa o teste; a guarda              estaria conferindo o arquivo errado"
        );

        for proibido in [
            concat!("Signing", "Key"),
            concat!("PRIVATE", " KEY"),
            concat!("CHAVE_", "PRIVADA"),
            concat!("Secret", "Key"),
            concat!("to_keypair", "_bytes"),
        ] {
            assert!(
                !producao.contains(proibido),
                "`{}` sugere chave privada num arquivo que vai para o cliente",
                proibido
            );
        }
    }

    #[test]
    fn chave_sem_ponto_e_malformada() {
        assert_eq!(desmontar("abcdef").unwrap_err(), Recusa::Malformada);
        assert_eq!(desmontar("").unwrap_err(), Recusa::Malformada);
        assert_eq!(desmontar(".").unwrap_err(), Recusa::Malformada);
        assert_eq!(desmontar("abc.").unwrap_err(), Recusa::Malformada);
        assert_eq!(desmontar(".xyz").unwrap_err(), Recusa::Malformada);
    }

    #[test]
    fn espaco_e_quebra_de_linha_sao_perdoados() {
        // Copiar de uma mensagem do Discord traz espaço e quebra de linha
        // junto. Recusar por isso seria transformar um problema nosso em
        // suporte para o dono.
        let com_sujeira = "  YWJj\n.ZGVm  ";
        let limpa = desmontar(com_sujeira);

        assert!(limpa.is_ok(), "não deveria recusar por espaço: {:?}", limpa);
    }

    #[test]
    fn maquina_sem_identidade_nao_ativa() {
        // Sem identificar a máquina não há licença presa a ela, e liberar
        // assim mesmo seria abrir a porta em toda máquina virtual.
        assert_eq!(
            conferir("qualquer.coisa", "", "2026-01-01").unwrap_err(),
            Recusa::MaquinaDesconhecida
        );
    }

    #[test]
    fn cada_recusa_explica_o_que_fazer() {
        // O cliente que pagou merece saber a diferença entre "digitei errado" e
        // "esta chave é de outro PC" — a segunda tem solução, a primeira não.
        let de_outra = Recusa::OutraMaquina {
            emitida_para: "OTZ-AAAA-BBBB-CCCC".to_string(),
        };

        assert!(de_outra.explicacao().contains("outro computador"));
        assert!(de_outra.explicacao().contains("Discord"));

        assert!(Recusa::Malformada.explicacao().contains("copiou"));
        assert!(Recusa::Expirada { em: "2026-01-01".to_string() }
            .explicacao()
            .contains("2026-01-01"));
    }

    #[test]
    fn a_data_em_texto_compara_certo() {
        // Formato "AAAA-MM-DD" ordena como texto exatamente como ordena no
        // calendário. É por isso que não há biblioteca de data aqui.
        assert!("2026-01-02" > "2026-01-01");
        assert!("2027-01-01" > "2026-12-31");
        assert!("2026-10-01" > "2026-09-30");
    }

    #[test]
    fn o_arquivo_de_licenca_segue_o_padrao_do_produto() {
        let caminho = Guardada::path();

        assert!(caminho.ends_with("licenca.json"));
        assert!(caminho.to_string_lossy().contains("pc-optimizer"));
    }

    #[test]
    fn sem_licenca_o_produto_fica_bloqueado() {
        // O estado padrão precisa ser "trancado". Um erro de leitura, um
        // arquivo corrompido ou um disco cheio não podem virar liberação.
        let vazia = Guardada::default();
        assert!(vazia.chave.is_empty());
    }

    #[test]
    fn mostra_o_estado_desta_maquina() {
        let e = estado();

        println!("  máquina : {}", e.maquina);
        println!("  ativa   : {}", e.ativa);
        println!("  formatar mantém a chave: {}", e.sobrevive_formatacao);

        if let Some(motivo) = &e.motivo {
            println!("  motivo  : {}", motivo);
        }

        // Sem chave gravada, o produto tem que estar trancado.
        if Guardada::load().chave.trim().is_empty() {
            assert!(!e.ativa);
        }
    }
}
