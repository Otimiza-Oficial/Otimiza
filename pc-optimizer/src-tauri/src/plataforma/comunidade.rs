// COMUNIDADE E NUVEM (Plano 1.1, Onda 5). Fala com a API do bot (o mesmo endereço do atestado, `OTIMIZA_API_LICENCA`).
//
// Comunidade (opt-in, desligada por padrão): depois de cada partida medida, manda o nome do processador e da placa, o
// jogo e o FPS/1% pior; depois de uma calibração com jogo, a receita que ganhou e o ganho. O servidor guarda só a
// impressão da licença e devolve "em PCs iguais ao seu, o que mais ganhou" — só com 3 PCs ou mais. Desligar apaga lá.
//
// Nuvem: um pacote com o histórico e as preferências (lista fechada de arquivos, até 256 KB), guardado pela licença.
// Fica de fora o que é estado DESTE PC (as mudanças aplicadas, a licença, o log): restaurar isso noutro PC mentiria.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Os arquivos de `%APPDATA%\pc-optimizer` que vão para a nuvem.
pub const ARQUIVOS_DA_NUVEM: &[&str] = &[
    "medicoes.json",
    "rota-historico.json",
    "preferences.json",
    "overlay.json",
    "cores-perfis.json",
    "lab.json",
];
pub const NUVEM_MAXIMA: usize = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pacote {
    pub versao: u32,
    pub feito_em: u64,
    pub arquivos: BTreeMap<String, String>,
}

/// **Pura.** Monta o pacote com os arquivos lidos (nome → conteúdo). Arquivo que não é JSON, fora da lista ou que
/// estouraria o tamanho fica de fora e é dito.
pub fn montar_pacote(lidos: &[(String, String)], agora: u64) -> (Pacote, Vec<String>) {
    let mut arquivos = BTreeMap::new();
    let mut fora = Vec::new();
    let mut total = 200usize;
    for (nome, conteudo) in lidos {
        if !ARQUIVOS_DA_NUVEM.contains(&nome.as_str()) || serde_json::from_str::<serde_json::Value>(conteudo).is_err() {
            fora.push(nome.clone());
            continue;
        }
        // Estimativa do JSON com escape: o conteúdo cresce um pouco ao virar texto dentro de texto.
        let custo = conteudo.len() + conteudo.len() / 8 + nome.len() + 8;
        if total + custo > NUVEM_MAXIMA {
            fora.push(nome.clone());
            continue;
        }
        total += custo;
        arquivos.insert(nome.clone(), conteudo.clone());
    }
    (Pacote { versao: 1, feito_em: agora, arquivos }, fora)
}

/// **Pura.** O que restaurar de um pacote: só nomes da lista e só JSON válido (o pacote veio de fora do PC).
pub fn a_restaurar(p: &Pacote) -> Vec<(String, String)> {
    p.arquivos
        .iter()
        .filter(|(n, c)| ARQUIVOS_DA_NUVEM.contains(&n.as_str()) && serde_json::from_str::<serde_json::Value>(c).is_ok())
        .map(|(n, c)| (n.clone(), c.clone()))
        .collect()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct ReceitaDaComunidade {
    pub receita: String,
    pub pcs: u32,
    pub ganho_fps_pct: Option<f64>,
    pub ganho_low_pct: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct PcsParecidos {
    pub processador: String,
    pub placa: String,
    pub jogo: String,
    pub receitas: Vec<ReceitaDaComunidade>,
    /// Quantos PCs uma receita precisa para aparecer.
    pub minimo: u32,
}

/// **Pura.** "energia:Resposta máxima" vira a frase da tela.
pub fn frase_da_receita(receita: &str) -> String {
    match receita.split_once(':') {
        Some(("energia", r)) => format!("Plano de energia \"{r}\" (calibração)"),
        Some(("turbo", r)) => format!("Receita do Turbo \"{r}\""),
        _ => receita.to_string(),
    }
}

pub fn pasta_de_dados() -> PathBuf {
    std::env::var("APPDATA")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("pc-optimizer")
}

/// Lê os arquivos da lista que existem em `pasta`.
pub fn ler_arquivos(pasta: &Path) -> Vec<(String, String)> {
    ARQUIVOS_DA_NUVEM
        .iter()
        .filter_map(|n| std::fs::read_to_string(pasta.join(n)).ok().map(|c| (n.to_string(), c)))
        .collect()
}

/// Grava os arquivos restaurados, guardando a versão de antes como `<nome>.antes-da-nuvem`.
pub fn gravar_arquivos(pasta: &Path, arquivos: &[(String, String)]) -> Result<usize, String> {
    std::fs::create_dir_all(pasta).map_err(|e| format!("não consegui criar a pasta de dados: {e}"))?;
    for (nome, conteudo) in arquivos {
        let destino = pasta.join(nome);
        if destino.exists() {
            let _ = std::fs::copy(&destino, pasta.join(format!("{nome}.antes-da-nuvem")));
        }
        std::fs::write(&destino, conteudo).map_err(|e| format!("não consegui gravar {nome}: {e}"))?;
    }
    Ok(arquivos.len())
}

/// Endereço, licença e máquina para falar com o servidor; a frase certa quando falta algo.
pub fn credenciais() -> Result<(String, String, String), String> {
    let base = crate::modules::licenca_online::endereco()
        .ok_or("Esta versão não fala com o servidor do Otimiza.")?;
    let crate::modules::licenca::Leitura::Ok(guardada) = crate::modules::licenca::Guardada::load() else {
        return Err("Precisa da licença ativada (Ajustes).".to_string());
    };
    let chave = guardada.chave.trim().to_string();
    if chave.is_empty() {
        return Err("Precisa da licença ativada (Ajustes).".to_string());
    }
    Ok((base.trim_end_matches('/').to_string(), chave, crate::modules::maquina::identidade().id))
}

pub fn cliente() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .user_agent(concat!("Otimiza/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|_| "Não consegui preparar a conexão.".to_string())
}

/// **Pura.** O código de erro do servidor vira frase.
pub fn frase_do_erro(status: u16, erro: Option<&str>) -> String {
    match (status, erro) {
        (404, Some("sem_backup")) => "Ainda não há backup na nuvem para esta licença.".to_string(),
        (403, Some("licenca_revogada")) => "Esta licença foi desativada. Fale com o suporte.".to_string(),
        (403, _) => "O servidor não reconheceu a licença desta máquina.".to_string(),
        (400, Some("backup_invalido")) => "O backup passou do tamanho que a nuvem guarda.".to_string(),
        (429, _) => "Muitos pedidos seguidos. Espere um minuto.".to_string(),
        _ => "Não consegui falar com o servidor do Otimiza agora.".to_string(),
    }
}

/// Manda um envio para a comunidade, se a pessoa participa. Falha calada: é contribuição, não pode atrapalhar.
pub async fn enviar(corpo: serde_json::Value) {
    if !crate::modules::preferences::Preferences::load().comunidade {
        return;
    }
    let (Ok((base, chave, maquina)), Ok(c)) = (credenciais(), cliente()) else { return };
    let perfil = crate::plataforma::hardware::ultimo_conhecido();
    let Some(perfil) = perfil else { return };
    let mut corpo = corpo;
    corpo["chave"] = chave.into();
    corpo["maquina"] = maquina.into();
    corpo["cpu"] = perfil.processador.into();
    corpo["gpu"] = perfil.placa.nome.into();
    let _ = c.post(format!("{base}/v1/comunidade/envio")).json(&corpo).send().await;
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn o_pacote_so_leva_a_lista_e_json_valido() {
        let lidos = vec![
            ("medicoes.json".to_string(), "[1,2]".to_string()),
            ("changes.json".to_string(), "[]".to_string()),
            ("lab.json".to_string(), "não é json".to_string()),
        ];
        let (p, fora) = montar_pacote(&lidos, 10);
        assert_eq!(p.arquivos.keys().collect::<Vec<_>>(), vec!["medicoes.json"]);
        assert_eq!(fora, vec!["changes.json", "lab.json"]);
    }

    #[test]
    fn arquivo_grande_demais_fica_de_fora_e_e_dito() {
        let grande = format!("[\"{}\"]", "x".repeat(NUVEM_MAXIMA));
        let (p, fora) = montar_pacote(&[("medicoes.json".into(), grande), ("lab.json".into(), "{}".into())], 0);
        assert!(p.arquivos.contains_key("lab.json"));
        assert_eq!(fora, vec!["medicoes.json"]);
        assert!(serde_json::to_string(&p).unwrap().len() < NUVEM_MAXIMA);
    }

    #[test]
    fn restaurar_ignora_o_que_nao_e_da_lista() {
        let mut arquivos = BTreeMap::new();
        arquivos.insert("licenca.json".to_string(), "{}".to_string());
        arquivos.insert("../../Windows/x.json".to_string(), "{}".to_string());
        arquivos.insert("preferences.json".to_string(), "{\"a\":1}".to_string());
        let r = a_restaurar(&Pacote { versao: 1, feito_em: 0, arquivos });
        assert_eq!(r, vec![("preferences.json".to_string(), "{\"a\":1}".to_string())]);
    }

    #[test]
    fn gravar_guarda_a_versao_de_antes() {
        let pasta = std::env::temp_dir().join(format!("otimiza-nuvem-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&pasta);
        gravar_arquivos(&pasta, &[("lab.json".into(), "{\"v\":1}".into())]).unwrap();
        gravar_arquivos(&pasta, &[("lab.json".into(), "{\"v\":2}".into())]).unwrap();
        assert_eq!(std::fs::read_to_string(pasta.join("lab.json.antes-da-nuvem")).unwrap(), "{\"v\":1}");
        assert_eq!(std::fs::read_to_string(pasta.join("lab.json")).unwrap(), "{\"v\":2}");
        let _ = std::fs::remove_dir_all(&pasta);
    }

    #[test]
    fn receita_vira_frase() {
        assert_eq!(frase_da_receita("energia:Adaptativo"), "Plano de energia \"Adaptativo\" (calibração)");
        assert_eq!(frase_do_erro(404, Some("sem_backup")), "Ainda não há backup na nuvem para esta licença.");
    }
}
