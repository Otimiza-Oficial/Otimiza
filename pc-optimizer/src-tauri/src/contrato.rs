// O contrato entre o Rust e a interface nova: os tipos de `ui/src/lib/tipos.ts` saem daqui, gerados pelo tauri-specta.
// Um comando renomeado ou um campo trocado no Rust quebra este teste, e não a tela do cliente. Só existe em teste: o
// executável continua registrando os comandos pelo `generate_handler!` de `lib.rs`, sem specta nenhum.
//
// Cada comando que a interface nova passa a usar entra na lista abaixo (e ganha `#[cfg_attr(test, specta::specta)]`),
// e os tipos que ele devolve ganham `#[cfg_attr(test, derive(specta::Type))]`. Depois:
//
//     OTIMIZA_GERAR_TIPOS=1 cargo test --lib contrato

#[cfg(test)]
mod tests {
    use std::path::Path;

    use specta_typescript::Typescript;
    use tauri_specta::{collect_commands, Builder, ErrorHandlingMode};

    const DESTINO: &str = "../ui/src/lib/tipos.ts";

    const CABECALHO: &str = "// Gerado por `src-tauri/src/contrato.rs`. Não edite: rode `OTIMIZA_GERAR_TIPOS=1 cargo test --lib contrato`.";

    /// `MockRuntime`, e não `tauri::Wry`: com o runtime de verdade o binário de teste puxa o código da janela, que
    /// exige o manifesto do comctl32 6 e não abre no Windows (`STATUS_ENTRYPOINT_NOT_FOUND`). O tipo gerado é o mesmo.
    fn construtor() -> Builder<tauri::test::MockRuntime> {
        Builder::<tauri::test::MockRuntime>::new()
            .commands(collect_commands![
                crate::commands::versao_do_app,
                crate::commands::hardware_ultimo,
                crate::commands::hardware_perfil,
                crate::commands::jogo_aberto,
                crate::commands::biblioteca_de_jogos,
                crate::commands::jogos_instalados,
                crate::commands::capa_do_jogo,
                crate::commands::start_monitoring,
                crate::commands::get_performance_metrics,
                // Onda 1: Histórico e desfazer.
                crate::commands::estado_do_historico,
                crate::commands::historico_de_mudancas,
                crate::commands::revert_optimization,
                // Os que recebem `AppHandle` são genéricos no runtime: aqui, o de teste.
                crate::commands::restaurar_tudo::<tauri::test::MockRuntime>,
                crate::commands::is_elevated,
                crate::commands::relaunch_as_admin::<tauri::test::MockRuntime>,
                // Onda 1: Tela, Rede e Latência.
                crate::commands::tela_diagnostico,
                crate::commands::tela_troca_pendente,
                crate::commands::tela_trocar_hz,
                crate::commands::tela_confirmar_troca,
                crate::commands::tela_voltar_troca,
                crate::commands::rede_rota,
                crate::commands::rede_rota_do_jogo,
                crate::commands::rede_historico,
                crate::commands::rede_wifi,
                crate::commands::rede_varredura_wifi,
                crate::commands::rede_varredura_wifi_ligar,
                crate::commands::rede_varredura_wifi_desligar,
                crate::commands::dpc_capturar,
                crate::commands::dpc_cancelar,
                crate::commands::mouse_taxa,
                crate::commands::energia_calibrar,
                crate::commands::energia_calibrar_cancelar,
                crate::commands::energia_aplicar_calibrado,
                crate::commands::cs2_painel,
                crate::commands::cs2_ligar,
                crate::commands::cs2_desligar,
                crate::commands::bandeja_humor,
                crate::commands::discord_disponivel,
                crate::commands::banda_painel,
                crate::commands::banda_ligar,
                crate::commands::banda_desligar,
                // Onda 2: Turbo.
                crate::commands::turbo_painel,
                crate::commands::turbo_ligar,
                crate::commands::turbo_provar,
                crate::commands::turbo_prova_guardada,
                crate::commands::turbo_compartilhar,
                crate::commands::turbo_desligar,
                crate::commands::turbo_atraso,
                crate::commands::turbo_esquecer,
                // Onda 2: Energia.
                crate::commands::simular_plano_otimiza,
                crate::commands::aplicar_plano_otimiza,
                crate::commands::vistoriar_plano_otimiza,
                crate::commands::reparar_plano_otimiza,
                crate::commands::energia_de_jogo,
                crate::commands::energia_de_jogo_definir,
                crate::commands::diagnostico_de_energia,
                crate::commands::tetos_escondidos,
                // Onda 2: Sistema.
                crate::commands::list_optimizations,
                crate::commands::apply_optimization,
                crate::commands::list_startup,
                crate::commands::set_startup_enabled,
                crate::commands::limpeza_medir,
                crate::commands::limpeza_executar,
                crate::commands::restore_status,
                crate::commands::create_restore_point,
                // Onda 2: Prova.
                crate::commands::lab_estado,
                crate::commands::lab_medir,
                crate::commands::lab_zerar,
                crate::commands::prova_alternada_guardada,
                crate::commands::resultado_para_compartilhar,
                // Onda 2: Ajustes.
                crate::commands::get_preferences,
                crate::commands::set_preferences,
                crate::commands::licenca_estado,
                crate::commands::licenca_ativar,
                crate::commands::versao_mais_nova,
                crate::commands::relatorio_de_suporte,
                crate::commands::overlay_estado,
                crate::commands::overlay_gravar,
                crate::commands::overlay_ligar,
                // Plano 1.1: o Oti com IA.
                crate::commands::oti_perguntar,
            ])
            // `invoke` rejeita com o texto do erro, como a interface atual espera; nada de `{ status, data }`.
            .error_handling(ErrorHandlingMode::Throw)
            // Os `u64` que cruzam a ponte são carimbos de tempo e contagens: cabem com folga num `number`.
            .dangerously_cast_bigints_to_number()
    }

    fn gerar(caminho: &Path) {
        construtor()
            .export(Typescript::default().header(CABECALHO), caminho)
            .expect("o tauri-specta não conseguiu exportar os tipos");
    }

    /// Normaliza o fim de linha: o Git do Windows pode trocar LF por CRLF no arquivo guardado.
    fn ler(caminho: &Path) -> String {
        std::fs::read_to_string(caminho)
            .unwrap_or_default()
            .replace("\r\n", "\n")
    }

    #[test]
    fn os_tipos_da_interface_nova_estao_em_dia() {
        let destino = Path::new(DESTINO);

        if std::env::var_os("OTIMIZA_GERAR_TIPOS").is_some() {
            gerar(destino);
            return;
        }

        let novo = std::env::temp_dir().join(format!("otimiza-tipos-{}.ts", std::process::id()));
        gerar(&novo);
        let gerado = ler(&novo);
        let _ = std::fs::remove_file(&novo);

        assert!(
            gerado == ler(destino),
            "ui/src/lib/tipos.ts está desatualizado em relação aos comandos do Rust. Rode \
             `OTIMIZA_GERAR_TIPOS=1 cargo test --lib contrato` e confira o que mudou na interface."
        );
    }
}
