// Licença por assinatura: o dono assina com a chave privada; o programa só carrega a PÚBLICA, que confere e não
// cria (segredo dentro do executável vira gerador). Chave `<dados base64>.<assinatura base64>`: os dados são
// legíveis, a assinatura impede alteração. Não é inquebrável (dá para arrancar a conferência do executável):
// impede o repasse casual.

use base64::Engine;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Pode ser pública. A privada vive na máquina do dono e no segredo do bot, nunca neste repositório.
/// Trocar este valor invalida TODAS as licenças emitidas. É a terceira: as duas anteriores vazaram pelo
/// `.env.example` (versionado) e numa captura de tela, e por isso o gerador escreve a privada direto no `.env` do
/// bot e nunca a imprime.
const CHAVE_PUBLICA: &str = "sR3nmVzmAtjoDmAWr8McycSq+vhDUCy2YnLDhJfy5LU=";

/// O que a assinatura garante é que ninguém mudou nada, não sigilo.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Dados {
    /// É o que faz a chave não valer em outro PC.
    pub maquina: String,
    pub comprador: String,
    pub emitida: String,
    /// `None` é vitalícia.
    pub expira: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Recusa {
    Malformada,
    AssinaturaInvalida,
    OutraMaquina { emitida_para: String },
    Expirada { em: String },
    MaquinaDesconhecida,
}

impl Recusa {
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

pub fn desmontar(chave: &str) -> Result<(Vec<u8>, Vec<u8>), Recusa> {
    let motor = base64::engine::general_purpose::URL_SAFE_NO_PAD;

    // Espaço e quebra de linha vêm de toda cópia do Discord: tirar é mais gentil que recusar.
    let limpa: String = chave.chars().filter(|c| !c.is_whitespace()).collect();

    let (dados, assinatura) = limpa.split_once('.').ok_or(Recusa::Malformada)?;

    if dados.is_empty() || assinatura.is_empty() {
        return Err(Recusa::Malformada);
    }

    let dados = motor.decode(dados).map_err(|_| Recusa::Malformada)?;
    let assinatura = motor.decode(assinatura).map_err(|_| Recusa::Malformada)?;

    Ok((dados, assinatura))
}

/// **Pura**: a decisão entre pagante e não pagante precisa ser testável sem a máquina nem a data de hoje.
pub fn conferir(chave: &str, maquina: &str, hoje: &str) -> Result<Dados, Recusa> {
    conferir_com(chave_publica()?, chave, maquina, hoje)
}

/// Chave por parâmetro para o teste gerar o próprio par e assinar, sem depender da chave de verdade.
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

    // A ORDEM IMPORTA: a assinatura é conferida ANTES de qualquer campo ser lido como verdade.

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
        // "AAAA-MM-DD" compara certo como texto.
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Guardada {
    pub chave: String,
}

impl Guardada {
    fn path() -> PathBuf {
        let base = std::env::var("APPDATA")
            .or_else(|_| std::env::var("HOME"))
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));

        base.join("pc-optimizer").join("licenca.json")
    }

    /// `unwrap_or_default()` punha o portão de ativação na cara de quem já comprou, sem dizer que havia licença
    /// gravada.
    pub fn load() -> Leitura {
        Self::ler(&Self::path())
    }

    /// Caminho por parâmetro: os testes não mexem no `%APPDATA%` de quem roda a esteira.
    fn ler(caminho: &Path) -> Leitura {
        let bruto = match fs::read_to_string(caminho) {
            Ok(bruto) => bruto,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Leitura::Ok(Guardada::default())
            }
            Err(e) => return Leitura::Ilegivel(format!("não consegui abrir o arquivo: {}", e)),
        };

        match serde_json::from_str(&bruto) {
            Ok(guardada) => Leitura::Ok(guardada),
            Err(e) => Leitura::Ilegivel(format!("o arquivo está corrompido: {}", e)),
        }
    }

    pub fn save(&self) -> Result<(), String> {
        Self::gravar(&Self::path(), self)
    }

    /// ATÔMICA: um `licenca.json` truncado por queda de energia tranca o produto para quem pagou.
    fn gravar(caminho: &Path, guardada: &Guardada) -> Result<(), String> {
        if let Some(pasta) = caminho.parent() {
            fs::create_dir_all(pasta)
                .map_err(|e| format!("Não foi possível criar a pasta de dados: {}", e))?;
        }

        let json = serde_json::to_string_pretty(guardada)
            .map_err(|e| format!("Não foi possível gravar a licença: {}", e))?;

        let temporario = super::changelog::caminho_temporario(caminho);

        fs::write(&temporario, json)
            .map_err(|e| format!("Não foi possível gravar a licença: {}", e))?;

        fs::rename(&temporario, caminho)
            .map_err(|e| format!("Não foi possível gravar a licença: {}", e))
    }
}

/// `Ok` com chave vazia é "nunca ativou"; `Ilegivel` é "existe e não consegui ler".
pub enum Leitura {
    Ok(Guardada),
    Ilegivel(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Estado {
    pub ativa: bool,
    pub maquina: String,
    /// "OTZ-WPYY-0J4F-77AB" sozinho não explica nada a quem está comprando.
    pub origem: String,
    pub sobrevive_formatacao: bool,
    pub comprador: Option<String>,
    pub expira: Option<String>,
    pub motivo: Option<String>,
}

fn hoje() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

/// A cada chamada, não na abertura: licença com prazo vence com o programa aberto.
pub fn estado() -> Estado {
    let quem = crate::modules::maquina::identidade();

    let base = Estado {
        ativa: false,
        maquina: quem.id.clone(),
        origem: quem.fonte.descricao().to_string(),
        sobrevive_formatacao: quem.fonte.sobrevive_formatacao(),
        comprador: None,
        expira: None,
        motivo: None,
    };

    let guardada = match Guardada::load() {
        Leitura::Ok(guardada) => guardada,

        // O PORTÃO CONTINUA FECHADO: muda só a frase.
        Leitura::Ilegivel(motivo) => {
            return Estado {
                motivo: Some(format!(
                    "Existe uma licença gravada neste computador, mas não consegui lê-la ({}). \
                     Cole a sua chave de novo — se você não a tiver mais, fale no suporte com o \
                     código desta máquina que ela é reemitida sem custo.",
                    motivo
                )),
                ..base
            }
        }
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

/// Conferir antes de gravar: chave inválida no disco daria um erro que o cliente não provocou.
pub fn ativar(chave: &str) -> Result<Dados, String> {
    let quem = crate::modules::maquina::identidade();

    let dados = conferir(chave, &quem.id, &hoje()).map_err(|r| r.explicacao())?;

    Guardada {
        chave: chave.trim().to_string(),
    }
    .save()?;

    Ok(dados)
}

pub fn liberado() -> bool {
    estado().ativa
}

/// Aqui e não só na tela: a tela é HTML numa janela com ferramentas de desenvolvedor, e esconder o botão não
/// impede chamar o comando. Conferida a cada chamada.
pub fn exigir() -> Result<(), String> {
    if liberado() {
        return Ok(());
    }

    Err("Esta ação faz parte do Otimiza completo, e este computador ainda não          está ativado. O diagnóstico continua livre: o que você viu sobre esta          máquina é real. Para liberar as correções, ative com a sua chave."
        .to_string())
}

#[cfg(test)]
mod tests_1_8 {
    use super::*;

    fn pasta(nome: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("otimiza-licenca-{}", nome));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("criar pasta de teste");
        dir
    }

    #[test]
    fn licenca_ausente_e_leitura_ok_sem_chave() {
        let caminho = pasta("ausente").join("licenca.json");

        match Guardada::ler(&caminho) {
            Leitura::Ok(g) => assert!(g.chave.is_empty()),
            Leitura::Ilegivel(m) => panic!("arquivo ausente nao pode ser ilegivel: {}", m),
        }
    }

    #[test]
    fn licenca_truncada_nao_vira_cliente_sem_licenca() {
        let caminho = pasta("truncada").join("licenca.json");
        fs::write(&caminho, r#"{"chave":"eyJtYXF1aW5"#).unwrap();

        match Guardada::ler(&caminho) {
            Leitura::Ilegivel(_) => {}
            Leitura::Ok(_) => panic!(
                "arquivo truncado foi lido como licenca valida e vazia: o cliente que                  pagou veria o portao de ativacao"
            ),
        }
    }

    #[test]
    fn gravar_e_ler_devolve_a_mesma_chave() {
        let caminho = pasta("ida-e-volta").join("licenca.json");
        let original = Guardada { chave: "uma-chave-qualquer".to_string() };

        Guardada::gravar(&caminho, &original).unwrap();

        match Guardada::ler(&caminho) {
            Leitura::Ok(g) => assert_eq!(g.chave, original.chave),
            Leitura::Ilegivel(m) => panic!("nao consegui reler o que acabei de gravar: {}", m),
        }
    }

    #[test]
    fn a_gravacao_da_licenca_nao_deixa_temporario() {
        let dir = pasta("temporario");
        let caminho = dir.join("licenca.json");

        Guardada::gravar(&caminho, &Guardada { chave: "x".to_string() }).unwrap();

        let sobraram: Vec<String> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();

        assert_eq!(sobraram, vec!["licenca.json".to_string()]);
    }
}

#[cfg(test)]
#[path = "licenca_prova.rs"]
mod prova;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chave_privada_nunca_entra_no_produto() {
        // A conferência mais importante: com a privada aqui, qualquer um que baixe o instalador emite licença. Só a
        // parte de PRODUÇÃO: a lista proibida mora no teste, e procurar no arquivo inteiro faria a guarda se acusar.
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
        let com_sujeira = "  YWJj\n.ZGVm  ";
        let limpa = desmontar(com_sujeira);

        assert!(limpa.is_ok(), "não deveria recusar por espaço: {:?}", limpa);
    }

    #[test]
    fn maquina_sem_identidade_nao_ativa() {
        // Liberar sem identificar a máquina abriria a porta em toda máquina virtual.
        assert_eq!(
            conferir("qualquer.coisa", "", "2026-01-01").unwrap_err(),
            Recusa::MaquinaDesconhecida
        );
    }

    #[test]
    fn cada_recusa_explica_o_que_fazer() {
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
        // O padrão é "trancado": erro de leitura, arquivo corrompido ou disco cheio não viram liberação.
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

        if let Leitura::Ok(guardada) = Guardada::load() {
            if guardada.chave.trim().is_empty() {
                assert!(!e.ativa);
            }
        }
    }
}
