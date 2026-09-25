// Diário de intenção: sobreviver a morrer no meio. O histórico grava de forma atômica, mas não protege a janela
// entre mexer no sistema e anotar (ou, ao reverter, entre tirar o registro e devolver o valor). Antes de tocar
// no sistema a intenção vai para um arquivo; terminado, ele some. Sobrou, é prova de interrupção, com o que é
// preciso para terminar. Não conserta sozinho: mostra a pendência e oferece.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::changelog::ChangeRecord;

/// Versão desconhecida é erro explícito, nunca "não há pendência".
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Intencao {
    Aplicar,
    Reverter,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pendencia {
    pub schema_version: u32,
    pub id: String,
    pub nome: String,
    pub intencao: Intencao,
    pub quando: u64,
    /// Os valores ANTERIORES: sem eles, saber que algo ficou pela metade não ajudaria ninguém.
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

/// Sem `Drop` que apaga: o desmonte de uma pane removeria a prova junto com o problema.
#[must_use = "um diário aberto e não concluído fica como pendência na próxima abertura"]
pub struct Diario {
    caminho: PathBuf,
}

impl Diario {
    /// Chamar ANTES de tocar no sistema. Falhar aqui é falhar a operação: sem diário, não se aplica.
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

        // O `sync_all` é o módulo inteiro: um diário ainda no cache quando a energia cai não existiu.
        arquivo
            .sync_all()
            .map_err(|e| format!("não consegui confirmar o diário em disco: {e}"))?;

        Ok(Diario {
            caminho: caminho.to_path_buf(),
        })
    }

    /// Falha TRATADA também fecha: o diário é sobre o processo morrer, não sobre a operação dar errado.
    pub fn concluir(self) -> Result<(), String> {
        match std::fs::remove_file(&self.caminho) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(format!("não consegui fechar o diário: {e}")),
        }
    }
}

/// Arquivo ilegível é `Err`, nunca `None`: ele só existe porque algo foi interrompido.
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

/// Deixar como está é escolha do cliente: por isso o nome não disfarça, DESCARTAR.
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

        let diario = Diario::abrir_em(&arquivo, &exemplo(Intencao::Reverter)).expect("abre");
        std::mem::forget(diario);

        let p = pendente_em(&arquivo).expect("lê").expect("há pendência");

        assert_eq!(p.intencao, Intencao::Reverter);
        assert_eq!(p.id, "mouse_precision");
        assert!(p.explicacao().contains("desfazia"));

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

        descartar_em(&arquivo).expect("idempotente");
    }

    #[test]
    fn abrir_por_cima_substitui_a_pendencia_anterior() {
        // Uma operação por vez (o histórico fica atrás de um cadeado).
        let arquivo = pasta("substitui").join("pendente.json");

        let primeiro = Diario::abrir_em(&arquivo, &exemplo(Intencao::Aplicar)).expect("abre");
        std::mem::forget(primeiro);

        let segundo = Diario::abrir_em(&arquivo, &exemplo(Intencao::Reverter)).expect("abre");
        std::mem::forget(segundo);

        let p = pendente_em(&arquivo).expect("lê").expect("há pendência");
        assert_eq!(p.intencao, Intencao::Reverter);
    }
}
