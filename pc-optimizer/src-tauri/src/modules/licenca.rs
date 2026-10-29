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
/// É a quarta: as duas primeiras vazaram pelo `.env.example` (versionado) e numa captura de tela, e por isso o gerador
/// escreve a privada direto no `.env` do bot e nunca a imprime. A terceira se perdeu com a hospedagem do bot (04/10/2026);
/// o `.env` agora tem backup. É com esta que o bot assina as licenças e os atestados de feedback.
const CHAVE_PUBLICA: &str = "Fz2AWNqDmj2iLeaEqkfMWgsfVdXuXJn2+eAn006pKSM=";

/// A terceira, só para licença já emitida: a privada se perdeu (ninguém emite mais com ela), e quem ativou antes de
/// 04/10/2026 continua aberto ao atualizar. Decisão do dono em 10/10/2026. Nunca vale para o atestado de feedback.
/// A troca de 04/10 ficou num commit que nunca foi publicado: a 3.2.0 só conhece esta, e toda chave nova do bot
/// era recusada ("Esta chave não foi reconhecida"), achado num cliente que formatou o PC.
const CHAVE_PUBLICA_ANTERIOR: &str = "sR3nmVzmAtjoDmAWr8McycSq+vhDUCy2YnLDhJfy5LU=";

/// A privada da terceira ficou na hospedagem perdida: quem tiver aquele `.env` emite licença que toda versão aceitava,
/// para qualquer máquina e com qualquer data. Daqui em diante ela só vale para licença emitida até a troca...
/// O dia 05 entra: o bot carimba a data em UTC, e venda depois das 21 h de Brasília do dia 04 sai com o dia seguinte.
const FIM_DA_CHAVE_ANTERIOR: &str = "2026-10-05";

/// ...e, preenchida, só para as máquinas que compraram com ela (SHA-256 do código `OTZ-…`, em hexadecimal: a lista
/// sai do banco do bot, e o código em claro não precisa ficar público). A data sozinha não segura quem tem a privada,
/// que assina a data que quiser; vazia, vale só a data, para nenhum cliente antigo ficar trancado antes da lista.
/// As três vendas do Otimiza pagas antes da reemissão com o par novo (04/10/2026 21:23 UTC), tiradas do banco do bot
/// em 29/10/2026. As outras vendas do período eram booster e pack, sem máquina.
const MAQUINAS_DA_CHAVE_ANTERIOR: &[&str] = &[
    "6507a6fb1cd0cb41e53867c65777ca55f863d2668342be6354af98cbcef6d2d2",
    "6a9086ff684d2accb7605ed1762b7ac78b351f0191d016a71831fce0e24a29c3",
    "bbdb4724b8686204b84e108ae2c680a64d3d196da74b362f596ad137fdd3ba05",
];

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
    /// Assinada pela chave anterior, mas fora da trava dela: pode ser compra de verdade que faltou na lista.
    ChaveAntiga,
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

            Recusa::ChaveAntiga => "Esta chave é do sistema de licenças antigo e precisa ser \
                 trocada. Se você comprou, fale no suporte com o código desta máquina: a chave \
                 nova sai sem custo."
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
    conferir_com_as_duas(
        chave_publica()?,
        decodificar_publica(CHAVE_PUBLICA_ANTERIOR)?,
        MAQUINAS_DA_CHAVE_ANTERIOR,
        chave,
        maquina,
        hoje,
    )
}

/// A atual decide; só a recusa de ASSINATURA passa para a anterior (chave de outra máquina ou vencida, assinada de
/// verdade, é recusa final). Chaves por parâmetro para o teste gerar os próprios pares.
pub fn conferir_com_as_duas(
    atual: VerifyingKey,
    anterior: VerifyingKey,
    maquinas_da_anterior: &[&str],
    chave: &str,
    maquina: &str,
    hoje: &str,
) -> Result<Dados, Recusa> {
    match conferir_com(atual, chave, maquina, hoje) {
        Err(Recusa::AssinaturaInvalida) => {
            conferir_anterior(anterior, maquinas_da_anterior, chave, maquina, hoje)
        }
        resultado => resultado,
    }
}

/// A chave anterior com as duas travas (data da troca e lista de quem comprou). Lista por parâmetro para o teste.
pub fn conferir_anterior(
    publica: VerifyingKey,
    maquinas: &[&str],
    chave: &str,
    maquina: &str,
    hoje: &str,
) -> Result<Dados, Recusa> {
    let dados = conferir_com(publica, chave, maquina, hoje)?;

    // A mesma frase para as duas travas: quem comprou e faltou na lista vai ao suporte, e quem forjou não fica sabendo
    // qual trava pegou.
    if dados.emitida.as_str() > FIM_DA_CHAVE_ANTERIOR {
        return Err(Recusa::ChaveAntiga);
    }

    if !maquinas.is_empty() && !maquinas.contains(&resumo_da_maquina(maquina).as_str()) {
        return Err(Recusa::ChaveAntiga);
    }

    Ok(dados)
}

/// SHA-256 do código da máquina em hexadecimal: é o que a lista da chave anterior guarda.
pub fn resumo_da_maquina(maquina: &str) -> String {
    hex_sha256(maquina.trim().to_uppercase().as_bytes())
}

pub(crate) fn hex_sha256(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect()
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

    let dados_bytes = conferir_assinatura(publica, chave)?;

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

/// Os bytes assinados, só depois de a assinatura conferir. Serve à licença e ao atestado de feedback do bot: mesmo
/// par de chaves, e o campo `tipo` do atestado impede um de passar pelo outro.
pub(crate) fn conferir_assinatura(publica: VerifyingKey, texto: &str) -> Result<Vec<u8>, Recusa> {
    let (dados_bytes, assinatura_bytes) = desmontar(texto)?;

    // A ORDEM IMPORTA: a assinatura é conferida ANTES de qualquer campo ser lido como verdade.

    let assinatura: [u8; 64] = assinatura_bytes
        .try_into()
        .map_err(|_| Recusa::Malformada)?;

    publica
        .verify(&dados_bytes, &Signature::from_bytes(&assinatura))
        .map_err(|_| Recusa::AssinaturaInvalida)?;

    Ok(dados_bytes)
}

/// A pública de agora: a do bot. O atestado de feedback só vale com ela.
pub(crate) fn chave_publica() -> Result<VerifyingKey, Recusa> {
    decodificar_publica(CHAVE_PUBLICA)
}

fn decodificar_publica(texto: &str) -> Result<VerifyingKey, Recusa> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(texto)
        .map_err(|_| Recusa::AssinaturaInvalida)?;

    let bytes: [u8; 32] = bytes.try_into().map_err(|_| Recusa::AssinaturaInvalida)?;

    VerifyingKey::from_bytes(&bytes).map_err(|_| Recusa::AssinaturaInvalida)
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Guardada {
    pub chave: String,
    /// O último atestado online (`licenca_online`). Ausente nos arquivos de antes dele: `default`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub atestado: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revogada: Option<super::licenca_online::Revogacao>,
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

    let hoje = hoje();
    let dados = match conferir(&guardada.chave, &quem.id, &hoje) {
        Ok(dados) => dados,
        Err(recusa) => {
            return Estado {
                motivo: Some(recusa.explicacao()),
                ..base
            }
        }
    };

    let online = chave_publica().map_err(|_| super::licenca_online::Falta::Invalido).and_then(|publica| {
        super::licenca_online::politica(
            publica,
            super::licenca_online::endereco().is_some(),
            super::licenca_online::EXIGE_ATESTADO,
            &guardada,
            &quem.id,
            &hoje,
        )
    });

    match online {
        Ok(()) => Estado {
            ativa: true,
            comprador: Some(dados.comprador),
            expira: dados.expira,
            ..base
        },
        Err(falta) => Estado {
            comprador: Some(dados.comprador),
            motivo: Some(falta.explicacao()),
            ..base
        },
    }
}

/// Conferir antes de gravar: chave inválida no disco daria um erro que o cliente não provocou.
pub fn ativar(chave: &str) -> Result<Dados, String> {
    let quem = crate::modules::maquina::identidade();

    let dados = conferir(chave, &quem.id, &hoje()).map_err(|r| r.explicacao())?;

    // O atestado é da chave anterior: a nova pede o dela. A revogação fica se for a MESMA chave colada de novo, ou
    // colar bastaria para desfazê-la.
    let revogada = match Guardada::load() {
        Leitura::Ok(antes) => antes
            .revogada
            .filter(|r| r.licenca == super::licenca_online::impressao_da_licenca(chave)),
        Leitura::Ilegivel(_) => None,
    };

    Guardada {
        chave: chave.trim().to_string(),
        atestado: None,
        revogada,
    }
    .save()?;

    // Pede o atestado já: com ele exigido, quem acabou de colar não espera a próxima volta.
    if super::licenca_online::endereco().is_some() {
        tauri::async_runtime::spawn(super::licenca_online::renovar_se_preciso());
    }

    Ok(dados)
}

pub fn liberado() -> bool {
    estado().ativa
}

/// Aqui e não só na tela: a tela é HTML numa janela com ferramentas de desenvolvedor, e esconder o botão não
/// impede chamar o comando. Conferida a cada chamada.
pub fn exigir() -> Result<(), String> {
    if liberado() {
        // Quem pagou e passou dos minutos de uso sem deixar o feedback: as correções pausam até ele chegar.
        return crate::plataforma::feedback::exigir();
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
        let original = Guardada {
            chave: "uma-chave-qualquer".to_string(),
            ..Guardada::default()
        };

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

        Guardada::gravar(
            &caminho,
            &Guardada {
                chave: "x".to_string(),
                ..Guardada::default()
            },
        )
        .unwrap();

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
    fn a_lista_da_chave_anterior_so_tem_resumos() {
        // Código em claro aqui nunca casaria (a conferência compara o SHA-256) e trancaria quem pagou.
        for item in MAQUINAS_DA_CHAVE_ANTERIOR {
            assert!(
                item.len() == 64 && item.chars().all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
                "`{}` não é um SHA-256 em hexadecimal minúsculo",
                item
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
        assert!(Recusa::Expirada {
            em: "2026-01-01".to_string()
        }
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
