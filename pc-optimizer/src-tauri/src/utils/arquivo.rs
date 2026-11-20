// Escrita atômica dos arquivos de estado (1.0.7, Onda 6). O desfazer, a recuperação e o portão leem estes arquivos na
// abertura seguinte: um `fs::write` cortado no meio (energia caindo, disco cheio, o app morto) deixava um JSON pela
// metade, e o estado era lido como vazio (o teto do jogo "sem dono", o registro de devolução perdido).
//
// Escreve num arquivo ao lado (`.tmp`), força para o disco e troca pelo nome certo com `rename`: quem lê vê o arquivo
// velho inteiro ou o novo inteiro, nunca a mistura. Vários módulos tinham a própria cópia disto; este é o lugar único.

use std::io::Write;
use std::path::Path;

/// Grava `bytes` em `caminho` de uma vez, criando a pasta se faltar. O erro diz o arquivo.
pub fn escrever_atomico(caminho: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(pasta) = caminho.parent() {
        std::fs::create_dir_all(pasta).map_err(|e| format!("não criei a pasta de {}: {}", caminho.display(), e))?;
    }
    let tmp = caminho.with_extension(match caminho.extension().and_then(|e| e.to_str()) {
        Some(ext) => format!("{}.tmp", ext),
        None => "tmp".to_string(),
    });
    let gravar = || -> std::io::Result<()> {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()
    };
    if let Err(e) = gravar() {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!("não gravei {}: {}", caminho.display(), e));
    }
    std::fs::rename(&tmp, caminho).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("não troquei {}: {}", caminho.display(), e)
    })
}

/// `escrever_atomico` de um valor em JSON legível.
pub fn escrever_json<T: serde::Serialize>(caminho: &Path, valor: &T) -> Result<(), String> {
    let json = serde_json::to_string_pretty(valor).map_err(|e| format!("não montei {}: {}", caminho.display(), e))?;
    escrever_atomico(caminho, json.as_bytes())
}

/// Grava e, se falhar, deixa no log em vez de sumir (o `let _ = fs::write(...)` de antes). Para estado que não impede a
/// ação de seguir, mas que precisa deixar rastro quando não ficou registrado.
pub fn escrever_json_ou_avisar<T: serde::Serialize>(caminho: &Path, valor: &T) {
    if let Err(e) = escrever_json(caminho, valor) {
        crate::utils::Logger::warn(&e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pasta_de_teste(nome: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("otimiza-arquivo-{}-{}", nome, std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        p
    }

    #[test]
    fn grava_inteiro_e_sem_sobra_do_temporario() {
        let pasta = pasta_de_teste("inteiro");
        let alvo = pasta.join("sub").join("estado.json");
        escrever_json(&alvo, &serde_json::json!({ "teto": 72 })).unwrap();
        escrever_json(&alvo, &serde_json::json!({ "teto": 141 })).unwrap();
        let lido: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&alvo).unwrap()).unwrap();
        assert_eq!(lido["teto"], 141, "o último grava por cima, inteiro");
        let sobras: Vec<_> = std::fs::read_dir(alvo.parent().unwrap())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|x| x == "tmp"))
            .collect();
        assert!(sobras.is_empty(), "o temporário não fica para trás");
        let _ = std::fs::remove_dir_all(&pasta);
    }

    /// 1.0.12 (A2): o estado que a abertura seguinte lê para devolver o PC (marcas de teste em andamento, energia de
    /// jogo, Wi-Fi, banda, FOV, motor de energia, preferências, provas) só passa por aqui. Um `fs::write` direto
    /// voltando a um destes arquivos quebra este teste.
    #[test]
    fn o_estado_da_recuperacao_so_grava_atomico() {
        let fontes = [
            ("provaalternada.rs", include_str!("../modules/provaalternada.rs")),
            ("prova.rs", include_str!("../modules/prova.rs")),
            ("preferences.rs", include_str!("../modules/preferences.rs")),
            ("motorenergia_maquina.rs", include_str!("../modules/windows/motorenergia_maquina.rs")),
            ("cpuset.rs", include_str!("../modules/windows/cpuset.rs")),
            ("pressao.rs", include_str!("../modules/windows/pressao.rs")),
            ("energia_jogo.rs", include_str!("../plataforma/energia_jogo.rs")),
            ("banda.rs", include_str!("../plataforma/banda.rs")),
            ("wifi_varredura.rs", include_str!("../plataforma/wifi_varredura.rs")),
            ("fov_fivem.rs", include_str!("../plataforma/fov_fivem.rs")),
            ("fb_por_jogo.rs", include_str!("../plataforma/fb_por_jogo.rs")),
            ("turbo.rs", include_str!("../plataforma/turbo.rs")),
        ];
        for (nome, fonte) in fontes {
            let producao = fonte.split("#[cfg(test)]").next().unwrap_or(fonte);
            assert!(
                !producao.contains("fs::write("),
                "{nome} voltou a gravar estado com `fs::write` (use `utils::arquivo::escrever_atomico`)"
            );
        }
    }

    #[test]
    fn falha_diz_o_arquivo_e_mantem_o_antigo() {
        let pasta = pasta_de_teste("falha");
        let alvo = pasta.join("estado.json");
        escrever_atomico(&alvo, b"{\"ok\":1}").unwrap();
        // Uma PASTA com o nome do temporário: a escrita falha antes de tocar no arquivo bom.
        std::fs::create_dir_all(pasta.join("estado.json.tmp")).unwrap();
        let erro = escrever_atomico(&alvo, b"{\"ok\":2}").unwrap_err();
        assert!(erro.contains("estado.json"), "{erro}");
        assert_eq!(std::fs::read_to_string(&alvo).unwrap(), "{\"ok\":1}", "o arquivo bom ficou inteiro");
        let _ = std::fs::remove_dir_all(&pasta);
    }
}
