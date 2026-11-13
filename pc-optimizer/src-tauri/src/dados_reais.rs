// DADOS REAIS PARA A FUMAÇA (Plano de refino 2.1.2). A demonstração escondeu defeitos que só apareciam com a resposta de
// verdade do Rust; aqui se gravam as respostas reais DESTA máquina dos comandos que só leem, e a fumaça da interface
// roda com elas (`OTIMIZA_DADOS_REAIS=ui/testes/dados-reais node scripts/fumaca-nova.mjs`).
//
// Teste manual (`--ignored`): grava hardware, jogos, rede e licença de quem roda, então a pasta fica fora do Git.
//
//     cargo test --lib -- --ignored dados_reais --nocapture

#[cfg(test)]
mod tests {
    use std::path::Path;

    fn gravar<T: serde::Serialize>(pasta: &Path, comando: &str, valor: Result<T, String>) {
        match valor {
            Ok(v) => {
                let json = serde_json::to_string_pretty(&v).expect("a resposta vira JSON");
                std::fs::write(pasta.join(format!("{comando}.json")), json).expect("grava a resposta");
                println!("gravado: {comando}");
            }
            Err(e) => println!("sem resposta de {comando}: {e}"),
        }
    }

    #[test]
    #[ignore]
    fn gravar_dados_reais() {
        let pasta = Path::new(env!("CARGO_MANIFEST_DIR")).join("../ui/testes/dados-reais");
        std::fs::create_dir_all(&pasta).expect("cria a pasta");
        use crate::commands as c;
        gravar(&pasta, "versao_do_app", Ok(c::versao_do_app()));
        gravar(&pasta, "hardware_perfil", c::hardware_perfil(false));
        gravar(&pasta, "hardware_ultimo", c::hardware_ultimo());
        gravar(&pasta, "jogos_instalados", Ok(c::jogos_instalados()));
        gravar(&pasta, "biblioteca_de_jogos", c::biblioteca_de_jogos());
        gravar(&pasta, "tela_diagnostico", Ok(c::tela_diagnostico()));
        gravar(&pasta, "jogo_aberto", c::jogo_aberto());
        tauri::async_runtime::block_on(async {
            gravar(&pasta, "relatorio_da_ultima_partida", c::relatorio_da_ultima_partida().await);
            gravar(&pasta, "rede_wifi", c::rede_wifi().await);
            gravar(&pasta, "rede_historico", c::rede_historico().await);
            gravar(&pasta, "energia_painel", c::energia_painel().await);
        });
    }
}
