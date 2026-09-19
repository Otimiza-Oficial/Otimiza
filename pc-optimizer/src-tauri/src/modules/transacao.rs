// Diário de intenção: sobreviver a morrer no meio
//
// POR QUE ISTO EXISTE
//
// `changelog.rs` guarda o valor anterior de cada mudança, e grava esse
// histórico de forma atômica. Isso protege o ARQUIVO. Não protege a JANELA
// entre mexer no sistema e anotar que mexeu.
//
// Aplicar é: escrever no registro, depois anotar. Morrer entre as duas deixa a
// mudança aplicada e o histórico dizendo que não está — "Desfazer tudo" não
// acha nada para desfazer, e o cliente fica com um ajuste que o produto não
// sabe que fez.
//
// Reverter é pior. O `take` do histórico tira o registro E GRAVA EM DISCO
// antes de a reversão rodar. Morrer durante a reversão perde o valor anterior
// para sempre: a mudança continua aplicada e não existe mais no mundo o número
// que estava lá antes. O código já devolve o registro quando a reversão falha,
// mas uma falha é diferente de um processo morto — não há `catch` para a
// tomada sendo puxada.
//
// COMO ISTO FECHA
//
// Antes de tocar no sistema, a intenção vai para um arquivo próprio: o que vai
// ser feito, em quê, e os valores anteriores. Terminado o trabalho, o arquivo
// some. Um arquivo que sobreviveu é a prova de que a máquina foi interrompida
// no meio — e ele carrega tudo o que é preciso para terminar o serviço.
//
// O QUE ESTE MÓDULO NÃO FAZ, DE PROPÓSITO
//
// Não conserta sozinho na próxima abertura. Completar uma reversão sem
// perguntar é decidir pelo cliente sobre a máquina dele, com base num arquivo
// que já provou que algo deu errado. O produto MOSTRA a pendência, com o que
// ficou pela metade, e oferece. Quem escolhe é quem é dono da máquina.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::changelog::ChangeRecord;

/// Versão do formato. Pendência de versão desconhecida é erro explícito, e
/// nunca "não há pendência": o arquivo existe porque algo foi interrompido, e
/// ignorá-lo seria esconder exatamente o que ele veio contar.
pub const SCHEMA_VERSION: u32 = 1;

/// O que estava sendo feito quando o mundo parou.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Intencao {
    /// Estava aplicando uma otimização.
    Aplicar,
    /// Estava desfazendo uma otimização.
    Reverter,
}

/// Uma operação que começou e não se sabe se terminou.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pendencia {
    pub schema_version: u32,
    /// Id da otimização, para casar com o catálogo e com o histórico.
    pub id: String,
    /// Nome em português, para a frase que o cliente lê.
    pub nome: String,
    pub intencao: Intencao,
    pub quando: u64,
    /// Os valores ANTERIORES, os mesmos que iriam para o histórico.
    ///
    /// É isto que transforma o arquivo de aviso em conserto: sem os valores
    /// anteriores, saber que algo ficou pela metade não ajudaria ninguém.
    pub mudancas: Vec<ChangeRecord>,
}

impl Pendencia {
    pub fn nova(
        id: impl Into<String>,
        nome: impl Into<String>,
        intencao: Intencao,
        quando: u64,
        mudancas: Vec<ChangeRecord>,
    ) -> Self {
        Pendencia {
            schema_version: SCHEMA_VERSION,
            id: id.into(),
            nome: nome.into(),
            intencao,
            quando,
            mudancas,
        }
    }

    /// A frase que a tela mostra.
    pub fn explicacao(&self) -> String {
        format!(
            "O Otimiza foi interrompido enquanto {} `{}`. {} mudança(s) podem ter ficado \
             pela metade, e os valores anteriores estão guardados.",
            match self.intencao {
                Intencao::Aplicar => "aplicava",
                Intencao::Reverter => "desfazia",
            },
            self.nome,
            self.mudancas.len()
        )
    }
}

/// O diário aberto. Fechar é `concluir`.
///
/// NÃO tem `Drop` que apaga sozinho. Seria a coisa errada: o processo morrendo
/// é justamente o caso que este módulo cobre, e um `Drop` que apagasse o
/// arquivo durante o desmonte de uma pane removeria a prova junto com o
/// problema. O arquivo só some quando alguém diz, em código, que terminou.
#[must_use = "um diário aberto e não concluído fica como pendência na próxima abertura"]
pub struct Diario {
    caminho: PathBuf,
}

impl Diario {
    /// Abre o diário. Chamar ANTES de tocar no sistema.
    ///
    /// Falhar aqui é falhar a operação inteira: sem o diário, a mudança
    /// seguinte não teria como ser recuperada, e aplicar sem rede é o que este
    /// produto não faz.
    pub fn abrir_em(caminho: &Path, pendencia: &Pendencia) -> Result<Self, String> {
        use std::io::Write;

        if let Some(pasta) = caminho.parent() {
            std::fs::create_dir_all(pasta)
                .map_err(|e| format!("não consegui criar a pasta de dados: {e}"))?;
        }

        let bruto = serde_json::to_string(pendencia)
            .map_err(|e| format!("não consegui preparar a pendência: {e}"))?;

        let mut arquivo = std::fs::File::create(caminho)
            .map_err(|e| format!("não consegui abrir o diário: {e}"))?;

        arquivo
            .write_all(bruto.as_bytes())
            .map_err(|e| format!("não consegui gravar o diário: {e}"))?;

        // O `sync_all` aqui é o módulo inteiro. Um diário que ainda está no
        // cache do sistema quando a energia cai não existiu — e a operação que
        // ele ia proteger acontece mesmo assim.
        arquivo
            .sync_all()
            .map_err(|e| format!("não consegui confirmar o diário em disco: {e}"))?;

        Ok(Diario {
            caminho: caminho.to_path_buf(),
        })
    }

    /// Fecha o diário: o trabalho terminou, com sucesso ou com falha tratada.
    ///
    /// Falha TRATADA também fecha. O diário é sobre o processo morrer no meio,
    /// não sobre a operação dar errado — quando ela dá errado e o código
    /// percebe, ele já sabe o que fazer, e `changelog.rs` já devolve o registro
    /// ao histórico.
    pub fn concluir(self) -> Result<(), String> {
        match std::fs::remove_file(&self.caminho) {
            Ok(()) => Ok(()),
            // Já não estava lá. O objetivo era não existir, e ele não existe.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(format!("não consegui fechar o diário: {e}")),
        }
    }
}

/// A pendência guardada, se houver.
///
/// `Ok(None)` é "não há pendência". Arquivo ilegível é `Err`, e nunca `None`:
/// esse arquivo só existe porque algo foi interrompido, e tratá-lo como
/// ausência seria apagar o aviso justamente no caso em que ele mais importa.
pub fn pendente_em(caminho: &Path) -> Result<Option<Pendencia>, String> {
    let bruto = match std::fs::read_to_string(caminho) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("não consegui ler o diário: {e}")),
    };

    let p: Pendencia = serde_json::from_str(&bruto)
        .map_err(|e| format!("há um diário interrompido, mas ele está ilegível: {e}"))?;

    if p.schema_version != SCHEMA_VERSION {
        return Err(format!(
            "há um diário interrompido no formato {}, que esta versão não sabe ler",
            p.schema_version
        ));
    }

    Ok(Some(p))
}

/// Descarta a pendência sem completar nada.
///
/// Existe para quando o cliente olha o que ficou pela metade e decide deixar
/// como está. É uma escolha dele, e por isso é uma função separada e com nome
/// que não disfarça: não "resolver", DESCARTAR.
pub fn descartar_em(caminho: &Path) -> Result<(), String> {
    match std::fs::remove_file(caminho) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("não consegui descartar a pendência: {e}")),
    }
}

fn caminho_padrao() -> PathBuf {
    let base = std::env::var("APPDATA")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));

    base.join("pc-optimizer").join("operacao-pendente.json")
}

pub fn abrir(pendencia: &Pendencia) -> Result<Diario, String> {
    Diario::abrir_em(&caminho_padrao(), pendencia)
}

pub fn pendente() -> Result<Option<Pendencia>, String> {
    pendente_em(&caminho_padrao())
}

pub fn descartar() -> Result<(), String> {
    descartar_em(&caminho_padrao())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::changelog::PreviousValue;

    fn pasta(nome: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("otimiza-transacao-{nome}"));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).expect("pasta de teste");
        p
    }

    fn mudanca() -> ChangeRecord {
        ChangeRecord::RegistryValue {
            hive: "HKCU".into(),
            path: r"Control Panel\Mouse".into(),
            name: "MouseSpeed".into(),
            previous: PreviousValue::Text("1".into()),
        }
    }

    fn exemplo(intencao: Intencao) -> Pendencia {
        Pendencia::nova(
            "mouse_precision",
            "Precisão do mouse",
            intencao,
            1_700_000_000,
            vec![mudanca()],
        )
    }

    #[test]
    fn o_ciclo_normal_nao_deixa_rastro() {
        let arquivo = pasta("normal").join("pendente.json");

        let diario = Diario::abrir_em(&arquivo, &exemplo(Intencao::Aplicar)).expect("abre");
        assert!(
            arquivo.exists(),
            "o diário existe enquanto o trabalho corre"
        );

        diario.concluir().expect("fecha");
        assert!(!arquivo.exists());
        assert_eq!(pendente_em(&arquivo).expect("lê"), None);
    }

    #[test]
    fn morrer_no_meio_deixa_a_pendencia_com_os_valores_anteriores() {
        let arquivo = pasta("morte").join("pendente.json");

        // Abre e NÃO conclui: é o processo sendo morto.
        let diario = Diario::abrir_em(&arquivo, &exemplo(Intencao::Reverter)).expect("abre");
        std::mem::forget(diario);

        let p = pendente_em(&arquivo).expect("lê").expect("há pendência");

        assert_eq!(p.intencao, Intencao::Reverter);
        assert_eq!(p.id, "mouse_precision");
        assert!(p.explicacao().contains("desfazia"));

        // O que torna isto útil: o valor anterior sobreviveu.
        match &p.mudancas[0] {
            ChangeRecord::RegistryValue { previous, name, .. } => {
                assert_eq!(name, "MouseSpeed");
                assert_eq!(previous, &PreviousValue::Text("1".into()));
            }
            outro => panic!("esperava valor de registro, veio {outro:?}"),
        }
    }

    #[test]
    fn diario_ilegivel_e_erro_e_nao_ausencia() {
        // O caso mais perigoso: o arquivo só existe porque algo foi
        // interrompido. Lê-lo como "não há pendência" apagaria o aviso
        // justamente quando ele mais importa.
        let arquivo = pasta("ilegivel").join("pendente.json");
        std::fs::write(&arquivo, "{isto não é json}").expect("escreve");

        let erro = pendente_em(&arquivo).expect_err("ilegível");
        assert!(erro.contains("interrompido"), "{erro}");
    }

    #[test]
    fn formato_desconhecido_tambem_e_erro() {
        let arquivo = pasta("versao").join("pendente.json");
        let mut p = exemplo(Intencao::Aplicar);
        p.schema_version = 99;
        std::fs::write(&arquivo, serde_json::to_string(&p).unwrap()).expect("escreve");

        let erro = pendente_em(&arquivo).expect_err("versão desconhecida");
        assert!(erro.contains("99"), "{erro}");
    }

    #[test]
    fn descartar_e_uma_escolha_separada_de_concluir() {
        let arquivo = pasta("descarte").join("pendente.json");

        let diario = Diario::abrir_em(&arquivo, &exemplo(Intencao::Aplicar)).expect("abre");
        std::mem::forget(diario);
        assert!(pendente_em(&arquivo).expect("lê").is_some());

        descartar_em(&arquivo).expect("descarta");
        assert_eq!(pendente_em(&arquivo).expect("lê"), None);

        // Descartar de novo não é erro: o objetivo era não existir.
        descartar_em(&arquivo).expect("idempotente");
    }

    #[test]
    fn abrir_por_cima_substitui_a_pendencia_anterior() {
        // Só existe uma operação por vez — o histórico fica atrás de um
        // cadeado. Se houvesse duas, a segunda esconderia a primeira, e é
        // melhor que isso seja explícito aqui do que uma surpresa depois.
        let arquivo = pasta("substitui").join("pendente.json");

        let primeiro = Diario::abrir_em(&arquivo, &exemplo(Intencao::Aplicar)).expect("abre");
        std::mem::forget(primeiro);

        let segundo = Diario::abrir_em(&arquivo, &exemplo(Intencao::Reverter)).expect("abre");
        std::mem::forget(segundo);

        let p = pendente_em(&arquivo).expect("lê").expect("há pendência");
        assert_eq!(p.intencao, Intencao::Reverter);
    }
}
