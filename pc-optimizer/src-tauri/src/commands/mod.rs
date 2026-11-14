use crate::core::PlatformDetector;
use crate::modules::benchmark::{
    self, BaselineResult, BaselineStore, Benchmark, BenchmarkComparison, BenchmarkSnapshot,
};
use crate::modules::changelog::ChangeLog;
#[cfg(target_os = "windows")]
use crate::modules::optimizer::BatchStep;
#[cfg(target_os = "windows")]
use crate::modules::optimizer::{OptimizationInfo, OptimizationOutcome};
use crate::modules::preferences::Preferences;
#[cfg(target_os = "windows")]
use crate::modules::windows::bloatware::BloatReport;
#[cfg(target_os = "windows")]
use crate::modules::windows::bottleneck::BottleneckReport;
#[cfg(target_os = "windows")]
use crate::modules::windows::citizenfx::CitizenFxReport;
#[cfg(target_os = "windows")]
#[cfg(target_os = "windows")]
use crate::modules::windows::essenciais::Checagem;
#[cfg(target_os = "windows")]
#[cfg(target_os = "windows")]
#[cfg(target_os = "windows")]
#[cfg(target_os = "windows")]
#[cfg(target_os = "windows")]
#[cfg(target_os = "windows")]
#[cfg(target_os = "windows")]
#[cfg(target_os = "windows")]
use crate::modules::windows::fivem::{CleanOutcome as FiveMCleanOutcome, FiveMReport};
#[cfg(target_os = "windows")]
use crate::modules::windows::framegen::{
    Artefato, Deteccao, Intensidade, Perfil, ResultadoDaRodada, Rodada, Tecnologia,
};
#[cfg(target_os = "windows")]
#[cfg(target_os = "windows")]
use crate::modules::windows::frames::FrameMeasurement;
#[cfg(target_os = "windows")]
use crate::modules::windows::gamemode::GameModeStatus;
#[cfg(target_os = "windows")]
use crate::modules::windows::gpupref::GpuPrefReport;
#[cfg(target_os = "windows")]
use crate::modules::windows::nvdriver::PainelDoDriver;
#[cfg(target_os = "windows")]
use crate::modules::windows::processes::ProcessImpact;
#[cfg(target_os = "windows")]
use crate::modules::windows::restore::RestoreStatus;
#[cfg(target_os = "windows")]
use crate::modules::windows::servicesaudit::ServiceEntry;
#[cfg(target_os = "windows")]
use crate::modules::windows::shaders::{CleanOutcome as ShaderCleanOutcome, ShaderReport};
#[cfg(target_os = "windows")]
use crate::modules::windows::startup::StartupEntry;
#[cfg(target_os = "windows")]
use crate::modules::windows::tasks::ScheduledTask;
#[cfg(target_os = "windows")]
#[cfg(target_os = "windows")]
use crate::modules::windows::veredito::Veredito;
use crate::modules::{PerformanceMetrics, PerformanceMonitor};
use tauri::{Emitter, Manager};

#[cfg(not(target_os = "windows"))]
const UNSUPPORTED_PLATFORM: &str =
    "Otimizações ainda não implementadas para este sistema operacional.";
use serde::Serialize;
use tauri::State;
use tokio::sync::Mutex;

// `tokio::sync::Mutex`: os guards atravessam `.await`, e um `std::sync::MutexGuard` não é Send.
pub struct AppState {
    pub monitor: Mutex<PerformanceMonitor>,
    pub changes: Mutex<ChangeLog>,
    /// Mantido: uso de CPU por processo só existe comparando duas leituras.
    #[cfg(target_os = "windows")]
    pub processes: Mutex<crate::modules::windows::processes::ProcessMonitor>,
}

// Um arquivo por área da tela, e não um de seis mil linhas: quem mexe no Turbo abre `turbo.rs` e enxerga só o Turbo.
// Os `pub use` mantêm cada comando em `crate::commands::<nome>`, o caminho que o `generate_handler!` de `lib.rs` e o
// contrato (`contrato.rs`) já usam: trocar o comando de arquivo não troca o nome que a tela chama. Cada área faz
// `use super::*` e enxerga daqui os `use` acima, o `AppState` e o `UNSUPPORTED_PLATFORM`.
mod calibracao;
mod comunidade;
mod driver;
mod energia;
mod frame_boost;
mod hardware;
mod jogos;
mod latencia;
mod licenca;
mod monitor;
mod nucleos;
mod oti;
mod otimizacoes;
mod overlay;
mod prova;
mod rede;
mod sistema;
mod tela;
mod turbo;

pub use calibracao::*;
pub use comunidade::*;
pub use driver::*;
pub use energia::*;
pub use frame_boost::*;
pub use hardware::*;
pub use jogos::*;
pub use latencia::*;
pub use licenca::*;
pub use monitor::*;
pub use nucleos::*;
pub use oti::*;
pub use otimizacoes::*;
pub use overlay::*;
pub use prova::*;
pub use rede::*;
pub use sistema::*;
pub use tela::*;
pub use turbo::*;

/// Comando novo sem classificação REPROVA O BUILD: a falha provável não é quebrar a assinatura, é esquecer a
/// linha da guarda.
#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "mede esta máquina por 6 s"]
    fn diagnostico_ao_vivo_nesta_maquina() {
        let d = super::diagnostico_na_janela(6, Default::default()).expect("diagnóstico");
        println!(
            "placa {:?} jogo {:?} erro {:?}",
            d.placa, d.jogo, d.quadros_erro
        );
        println!("saude {:#?}", d.saude);
        println!("gargalo {:#?}", d.gargalo);
        println!("{}", serde_json::to_string(&d).unwrap().len());
        assert!(d.amostras.len() >= 10);
    }

    /// O botão genérico NUNCA muda a aparência do jogo: no lote, só o teto de quadros sai do arquivo. Trava de FORMA
    /// sobre o corpo da função.
    #[test]
    fn o_lote_automatico_nao_tira_mais_o_limite_do_jogo() {
        // Medido no FiveM do dono (preso no processador): tirar o limite de FPS (`SemTeto`) aumenta a oscilação do tempo
        // de quadro, o tranco que o cliente sente. O "Otimizar" não deve mais aplicar nenhum perfil que mexa no jogo.
        // Só a produção (`producao`): procurando no arquivo inteiro, o texto procurado achava a si mesmo aqui no teste,
        // e a trava conferia este bloco em vez do lote.
        let corpo = producao()
            .split("let resultados = crate::modules::windows::WindowsOptimizer::new().apply_selection(")
            .nth(1)
            .and_then(|c| c.split("Ok(resultados)").next())
            .expect("não achei o corpo do lote automático");
        assert!(corpo.contains("optimize:step"), "o corpo achado não é o do lote");
        for proibido in [
            "aplicar_teto_do_jogo(&app",
            "Perfil::SemTeto",
            "Perfil::Equilibrado",
        ] {
            assert!(
                !corpo.contains(proibido),
                "o lote automático voltou a mexer na configuração do jogo (`{}`)",
                proibido
            );
        }
    }

    /// O nome que o orquestrador devolve é o que a tela manda para `apply_game_profile`: tabelas divergentes dariam
    /// "perfil desconhecido" depois de recomendar.
    #[cfg(target_os = "windows")]
    #[test]
    fn o_plano_fala_a_lingua_de_quem_aplica() {
        use crate::modules::orquestrador::Perfil;

        for p in Perfil::TODOS {
            super::perfil_por_nome(p.nome())
                .unwrap_or_else(|e| panic!("o perfil {:?} saiu do lugar: {e}", p));
        }
    }

    /// Rodam sem licença: leitura, medição e o desfazer (licença vencida não pode deixar o PC sem volta).
    const LIVRES: &[&str] = &[
        "oti_perguntar",
        "feedback_estado",
        "feedback_verificar",
        "flags_ler",
        "flags_gravar",
        "hardware_perfil",
        "hardware_ultimo",
        "compatibilidade",
        "driver_amd",
        "driver_intel",
        "jogo_aberto",
        "catalogo_de_jogos",
        "biblioteca_de_jogos",
        "lab_estado",
        "lab_medir",
        "lab_zerar",
        "capa_do_jogo",
        "capa_larga_do_jogo",
        "jogos_instalados",
        "jogar",
        "fundos_listar",
        "limpeza_medir",
        "fundos_adicionar",
        "fundos_limpar",
        "anotar_erro_da_tela",
        "mira_atualizar",
        "mira_desligar",
        "mira_estado",
        "mira_cs2",
        "cores_estado",
        "frame_boost_painel",
        "frame_boost_desligar",
        "turbo_painel",
        "turbo_receitas",
        "turbo_desligar",
        "turbo_esquecer",
        "turbo_atraso",
        "turbo_perfil_do_jogo",
        "config_motor_painel",
        "escala_pedir_comparacao",
        "escala_comparacao",
        "turbo_prova_guardada",
        "turbo_compartilhar",
        "frame_boost_destravar_base",
        "frame_boost_medir",
        "jogo_liso_medir",
        "memoria_para_o_jogo",
        "energia_de_jogo",
        "overlay_estado",
        "overlay_gravar",
        "contador_fivem_estado",
        "contador_fivem_desligar",
        "fov_fivem",
        "fov_fivem_devolver",
        "cores_restaurar",
        "cores_salvar_perfil",
        "investigar_travadas",
        "devolver_teste_da_travada",
        "vistoriar_mudancas",
        "zerar_modo_jogo",
        "placa_de_video",
        "memoria_instalada",
        "monitores",
        "analyze_game_config",
        "preview_game_profile",
        "medir_antes",
        "medir_depois",
        "prova_guardada",
        "prova_alternada_guardada",
        "modo_seguro",
        "crashes_do_jogo",
        "vrr_dos_monitores",
        "sair_do_modo_seguro",
        "get_platform_info",
        "get_performance_metrics",
        "capturar_baseline",
        "capturar_baseline_repetido",
        "protocolo_do_perfil",
        "laboratorio_de_streaming",
        "plano_de_renderizacao",
        "historico_de_desempenho",
        "caminho_do_mouse",
        "nucleos_da_maquina",
        "recuperacao_pendente",
        "descartar_pendencia",
        "concluir_recuperacao",
        "comparar_com_baseline",
        "start_monitoring",
        "stop_monitoring",
        "measure_baseline",
        "get_baseline",
        "measure_and_compare",
        "is_elevated",
        "diagnostico_de_energia",
        "simular_plano_otimiza",
        "relatorio_de_compatibilidade",
        "vistoriar_plano_otimiza",
        "relaunch_as_admin",
        "get_hardware_profile",
        "top_processes",
        "get_preferences",
        "set_preferences",
        "analyze_bloatware",
        "open_apps_settings",
        "analyze_shaders",
        "diagnostico_rapido",
        "analyze_gpu_preference",
        "running_game_executable",
        "analyze_bottleneck",
        "game_mode_status",
        "measure_frames",
        "framegen_detectar",
        "framegen_medir",
        "framegen_comparar",
        "framegen_melhor",
        "gerador_desligar",
        "gerador_estado",
        "energia_painel",
        "energia_vizinhos",
        "diagnostico_ao_vivo",
        "tetos_escondidos",
        "pronto_para_jogar",
        "energia_medir_atual",
        "energia_escolher",
        "energia_restaurar_anterior",
        "energia_restaurar_windows",
        "energia_salvar_perfil_de_jogo",
        "energia_remover_perfil_de_jogo",
        "analyze_fivem",
        "analyze_citizenfx",
        "export_report",
        "exportar_alteracoes",
        "salvar_ficha_da_bios",
        "cpuset_resultados",
        "cpuset_esquecer",
        "analyze_rbar",
        "list_profiles",
        "list_third_party_services",
        "list_scheduled_tasks",
        "restore_status",
        "list_startup",
        "list_optimizations",
        "estado_do_historico",
        "historico_de_mudancas",
        "convite_do_discord",
        "revert_optimization",
        "restaurar_tudo",
        "licenca_estado",
        "licenca_ativar",
        "relatorio_de_suporte",
        "versao_mais_nova",
        "versao_do_app",
        "checar_essenciais",
        "ajustes_do_driver_nvidia",
        "medicoes_automaticas",
        "o_que_mudou",
        "relatorio_da_ultima_partida",
        "diagnostico_da_partida",
        "receita_da_partida",
        "resultado_para_compartilhar",
        "perfil_do_pc",
        "conferir_o_proprio_trabalho",
        "onde_os_jogos_moram",
        "por_que_o_fps_esta_baixo",
        "o_que_nao_fazemos",
        "o_que_o_otimiza_altera",
        "protocolo_de_grupos",
        "nota_do_jogo",
        "conflitos_entre_ajustes",
        "conta_que_esta_rodando",
        "niveis_de_otimizacao",
        "passo_a_passo_da_bios",
        "ficha_da_bios",
        "abertura_pronta",
        "quedas_de_desempenho",
        "reiniciar_na_bios",
        // Nova Geração, Onda 1: Tela, Rede e Latência.
        "tela_diagnostico",
        "tela_troca_pendente",
        "tela_confirmar_troca",
        "tela_voltar_troca",
        "rede_rota",
        "rede_rota_do_jogo",
        "rede_fila",
        "fivem_servidores_ping",
        "rede_quem_usa",
        "rede_prioridade_estado",
        "rede_prioridade_desligar",
        "escala_nitidez",
        "rede_historico",
        "energia_calibrar_cancelar",
        "energia_salvar_calibrado_para_o_jogo",
        "cs2_painel",
        "bandeja_humor",
        "discord_disponivel",
        "comunidade_participar",
        "comunidade_parecidos",
        "nuvem_guardar",
        "nuvem_restaurar",
        "cs2_desligar",
        "rede_wifi",
        "rede_varredura_wifi",
        "rede_varredura_wifi_desligar",
        "dpc_capturar",
        "dpc_cancelar",
        "mouse_taxa",
        "banda_painel",
        "banda_desligar",
    ];

    /// Alteram o computador. Sem licença, recusam.
    const EXIGEM_LICENCA: &[&str] = &[
        "tela_trocar_hz",
        "rede_varredura_wifi_ligar",
        "banda_ligar",
        "rede_provar_placa",
        "rede_prioridade_ligar",
        "prender_jogo_nos_nucleos",
        "gerador_ligar",
        "cpuset_testar",
        "testar_causa_da_travada",
        "energia_testar_candidato",
        "energia_calibrar",
        "energia_aplicar_calibrado",
        "cs2_ligar",
        "energia_aplicar",
        "energia_modo_dinamico",
        "aplicar_plano_otimiza",
        "reparar_plano_otimiza",
        "set_automatic_pagefile",
        "clean_shader_cache",
        "set_gpu_preference",
        "fix_readiness",
        "set_persistent_priority",
        "set_game_mode",
        "clean_fivem",
        "set_scheduled_task",
        "set_service_start",
        "remove_store_app",
        "create_restore_point",
        "enable_system_protection",
        "set_startup_enabled",
        "apply_game_profile",
        "apply_optimization",
        "refazer_mudanca",
        "optimize_now",
        "provar_o_otimizar",
        "set_max_refresh_rate",
        "religar_essenciais",
        "aplicar_ajuste_nvidia",
        "limitar_fps_nvidia",
        "aplicar_ajuste_amd",
        "aplicar_ajuste_intel",
        "mira_ligar",
        "cores_aplicar",
        "frame_boost_ligar",
        "turbo_ligar",
        "turbo_perfil_do_jogo_pedir",
        "turbo_provar",
        "config_motor_aplicar",
        "frame_boost_travar_base",
        "jogo_liso_aplicar",
        "jogo_liso_depois_da_partida",
        "memoria_fechar",
        "limpeza_executar",
        "energia_de_jogo_definir",
        "overlay_ligar",
        "contador_fivem_ligar",
        "fov_fivem_definir",
    ];

    /// Só a parte de produção: os testes contêm as palavras procuradas. De TODOS os arquivos de `src/commands/`, lidos
    /// da pasta e não de uma lista: um arquivo de área novo que a guarda não lesse deixaria comando sem classificação
    /// de licença, e sem `(async)`, passar calado.
    fn producao() -> &'static str {
        static PRODUCAO: std::sync::OnceLock<String> = std::sync::OnceLock::new();

        PRODUCAO.get_or_init(|| {
            let pasta = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("src")
                .join("commands");

            let mut arquivos: Vec<std::path::PathBuf> = std::fs::read_dir(&pasta)
                .unwrap_or_else(|e| panic!("não consegui ler {:?}: {}", pasta, e))
                .flatten()
                .map(|entrada| entrada.path())
                .filter(|caminho| caminho.extension().and_then(|e| e.to_str()) == Some("rs"))
                .collect();

            // `read_dir` não promete ordem; fixada, a mensagem de uma falha é a mesma em toda rodada.
            arquivos.sort();

            assert!(
                arquivos.len() > 1,
                "só {} arquivo em {:?}; a guarda estaria olhando a pasta errada",
                arquivos.len(),
                pasta
            );

            let mut juntos = String::new();

            for caminho in &arquivos {
                let fonte = std::fs::read_to_string(caminho)
                    .unwrap_or_else(|e| panic!("não consegui ler {:?}: {}", caminho, e));

                let corte = fonte
                    .split("#[cfg(test)]")
                    .next()
                    .expect("split devolve ao menos um pedaço");

                // Este módulo de testes mora no `mod.rs`: sem o corte lá, a guarda acharia as próprias palavras.
                if caminho.file_name().and_then(|n| n.to_str()) == Some("mod.rs") {
                    assert!(
                        corte.len() < fonte.len(),
                        "não achei onde a produção termina; a guarda estaria olhando o \
                         arquivo errado"
                    );
                }

                juntos.push_str(corte);
                juntos.push('\n');
            }

            juntos
        })
    }

    /// Comando síncrono sem `(async)` roda na thread da janela: um PowerShell lento ali congelava a aba BIOS por
    /// 130 s (reproduzido) até o Windows ou a pessoa fechar o programa.
    #[test]
    fn nenhum_comando_roda_na_thread_da_janela() {
        let marca = concat!("#[tauri::", "command");
        let na_janela: Vec<String> = producao()
            .split(marca)
            .skip(1)
            .filter(|bloco| bloco.starts_with(']'))
            .filter_map(|bloco| {
                bloco
                    .lines()
                    .find(|l| l.contains("fn "))
                    .map(str::to_string)
            })
            .filter(|assinatura| !assinatura.contains("async fn "))
            .collect();

        assert!(
            na_janela.is_empty(),
            "comando síncrono sem `(async)`, roda na thread da janela: {:?}",
            na_janela
        );
    }

    fn comandos() -> Vec<&'static str> {
        let marca = concat!("#[tauri::", "command");

        producao()
            .split(marca)
            .skip(1)
            // A marca aceita argumento: os de reparo são `(async)`, e sem isto ficariam invisíveis à guarda.
            .filter(|bloco| bloco.starts_with(']') || bloco.starts_with("(async)]"))
            .map(|bloco| {
                let assinatura = bloco
                    .lines()
                    .find(|l| l.contains("pub fn ") || l.contains("pub async fn "))
                    .expect("todo comando tem uma assinatura logo abaixo da marca");

                assinatura
                    .split("fn ")
                    .nth(1)
                    // `<`: comando genérico no runtime (os que recebem `AppHandle` e entram no contrato tipado).
                    .and_then(|resto| resto.split(['(', '<']).next())
                    .expect("nome do comando")
                    .trim()
            })
            .collect()
    }

    /// Onde a assinatura do comando começa, genérico (`fn nome<R: Runtime>(`) ou não.
    fn inicio_do_comando(fonte: &str, nome: &str) -> Option<usize> {
        fonte
            .find(&format!("fn {}(", nome))
            .or_else(|| fonte.find(&format!("fn {}<", nome)))
    }

    /// Chutar pintaria a placa de verde para quem tem Radeon.
    #[test]
    fn a_marca_da_placa_sai_do_nome() {
        for (nome, esperado) in [
            ("NVIDIA GeForce GTX 1650", "nvidia"),
            ("NVIDIA GeForce RTX 4070 Ti", "nvidia"),
            ("AMD Radeon RX 6600", "amd"),
            ("Radeon(TM) Graphics", "amd"),
            ("Intel(R) UHD Graphics 630", "intel"),
            ("Intel(R) Arc(TM) A750", "intel"),
            ("Microsoft Basic Display Adapter", "desconhecida"),
            ("", "desconhecida"),
        ] {
            assert_eq!(
                super::marca_da_placa(nome),
                esperado,
                "`{}` foi classificada errado",
                nome
            );
        }
    }

    #[test]
    fn nenhum_comando_fica_sem_classificacao() {
        for nome in comandos() {
            let livre = LIVRES.contains(&nome);
            let paga = EXIGEM_LICENCA.contains(&nome);

            assert!(
                livre || paga,
                "o comando `{}` não está classificado. Decida: ele LÊ o \
                 computador (vai para LIVRES) ou ALTERA (vai para \
                 EXIGEM_LICENCA e ganha a linha `licenca::exigir()?`)?",
                nome
            );

            assert!(!(livre && paga), "`{}` está nas duas listas", nome);
        }
    }

    #[test]
    fn quem_altera_o_computador_pede_licenca() {
        let fonte = producao();

        for nome in EXIGEM_LICENCA {
            let inicio = inicio_do_comando(fonte, nome)
                .unwrap_or_else(|| panic!("comando `{}` sumiu do arquivo", nome));

            let corpo = &fonte[inicio..];
            let abre = corpo.find('{').expect("corpo do comando");

            // A guarda é a PRIMEIRA coisa do corpo: depois já se mexeu no computador de quem não pagou.
            let primeiras = corpo[abre + 1..]
                .lines()
                .find(|l| !l.trim().is_empty())
                .unwrap_or("");

            assert!(
                primeiras.contains("licenca::exigir()?"),
                "`{}` altera o computador e não confere a licença na primeira \
                 linha. Achei: `{}`",
                nome,
                primeiras.trim()
            );
        }
    }

    #[test]
    fn quem_so_le_nao_pede_licenca() {
        // Diagnóstico com licença esvaziaria a tela de compra.
        let fonte = producao();

        for nome in LIVRES {
            let Some(inicio) = inicio_do_comando(fonte, nome) else {
                continue;
            };

            // O fim é a primeira linha só com `}`: o arquivo é CRLF, e "\n}\n" nunca casaria.
            let corpo: String = fonte[inicio..]
                .lines()
                .take_while(|l| l.trim_end() != "}")
                .collect::<Vec<_>>()
                .join("\n");

            assert!(
                !corpo.contains("licenca::exigir()?"),
                "`{}` está na lista dos livres e mesmo assim exige licença",
                nome
            );
        }
    }

    #[test]
    fn as_duas_listas_cobrem_todos_os_comandos() {
        let achados = comandos().len();
        let classificados = LIVRES.len() + EXIGEM_LICENCA.len();

        assert_eq!(
            achados, classificados,
            "{} comandos no arquivo, {} nas listas",
            achados, classificados
        );
    }

    /// Com `decorations: false` a barra é nosso HTML chamando comandos do Tauri, e no Tauri 2 comando não declarado
    /// em `capabilities/default.json` falha CALADO (o duplo-clique usa `internal_toggle_maximize`, outra permissão).
    #[test]
    fn a_barra_da_janela_tem_todas_as_permissoes_que_usa() {
        let permissoes = include_str!("../../capabilities/default.json");

        for comando in [
            "core:window:allow-close",
            "core:window:allow-minimize",
            "core:window:allow-toggle-maximize",
            "core:window:allow-start-dragging",
            "core:window:allow-internal-toggle-maximize",
            "core:window:allow-is-maximized",
        ] {
            assert!(
                permissoes.contains(comando),
                "a barra da janela usa `{}` e a permissão não está declarada;                  o botão vai falhar sem dizer nada",
                comando
            );
        }
    }

    /// Literais de string com o texto ANTES deles, onde mora o operador. Em `char`, não byte: há acento.
    fn literais_com_contexto(linha: &str) -> Vec<(String, String)> {
        let chars: Vec<char> = linha.chars().collect();
        let mut achados = Vec::new();
        let mut i = 0;

        while i < chars.len() {
            let abre = chars[i];
            if abre == '"' || abre == '\'' || abre == '`' {
                let contexto: String = chars[..i].iter().collect();
                let mut j = i + 1;
                let mut conteudo = String::new();

                while j < chars.len() {
                    if chars[j] == '\\' && j + 1 < chars.len() {
                        j += 2;
                        continue;
                    }
                    if chars[j] == abre {
                        break;
                    }
                    conteudo.push(chars[j]);
                    j += 1;
                }

                let resto: String = chars.get(j + 1..).unwrap_or(&[]).iter().collect();
                achados.push((contexto, conteudo, resto));
                i = j + 1;
            } else {
                i += 1;
            }
        }

        achados
            .into_iter()
            .map(|(contexto, conteudo, _resto)| (contexto, conteudo))
            .collect()
    }

    /// Prosa do backend: tem espaço ou termina em pontuação. Rótulo que a tela inventou é uma palavra só.
    fn parece_prosa_do_backend(literal: &str) -> bool {
        if literal.trim().is_empty() {
            return false;
        }
        literal.contains(' ') || literal.trim_end().ends_with(['.', '!', '?'])
    }

    /// Igualdade, prefixo, substring, posição ou `case`: o repertório que já causou o defeito três vezes.
    fn termina_em_operador_de_decisao(contexto: &str) -> bool {
        let c = contexto.trim_end();
        c.ends_with("===")
            || c.ends_with("!==")
            || c.ends_with(".startsWith(")
            || c.ends_with(".endsWith(")
            || c.ends_with(".includes(")
            || c.ends_with(".indexOf(")
            || c.trim_start().ends_with("case")
    }

    /// Tira comentários antes (senão a guarda reprova por CITAÇÃO do antipadrão), com o mesmo estado de aspas de
    /// `literais_com_contexto` para não confundir `"https://"`. Mantém as quebras de linha para os números baterem.
    fn remover_comentarios(fonte: &str) -> String {
        let chars: Vec<char> = fonte.chars().collect();
        let mut saida = String::with_capacity(chars.len());
        let mut i = 0;
        let mut aspas: Option<char> = None;

        while i < chars.len() {
            let c = chars[i];

            if let Some(abre) = aspas {
                saida.push(c);
                if c == '\\' && i + 1 < chars.len() {
                    saida.push(chars[i + 1]);
                    i += 2;
                    continue;
                }
                if c == abre {
                    aspas = None;
                }
                i += 1;
                continue;
            }

            if c == '"' || c == '\'' || c == '`' {
                aspas = Some(c);
                saida.push(c);
                i += 1;
                continue;
            }

            if c == '/' && chars.get(i + 1) == Some(&'/') {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
                continue;
            }

            if c == '/' && chars.get(i + 1) == Some(&'*') {
                i += 2;
                while i < chars.len() && !(chars[i] == '*' && chars.get(i + 1) == Some(&'/')) {
                    if chars[i] == '\n' {
                        saida.push('\n');
                    }
                    i += 1;
                }
                i = (i + 2).min(chars.len());
                continue;
            }

            saida.push(c);
            i += 1;
        }

        saida
    }

    /// Compartilhada pela guarda real (os arquivos de `ui/src`) e pelo teste com fontes sintéticas.
    fn achados_de_prosa_comparada(fonte: &str) -> Vec<String> {
        let sem_comentarios = remover_comentarios(fonte);
        let mut achados = Vec::new();

        for (numero, linha) in sem_comentarios.lines().enumerate() {
            for (contexto, literal) in literais_com_contexto(linha) {
                if termina_em_operador_de_decisao(&contexto) && parece_prosa_do_backend(&literal) {
                    achados.push(format!(
                        "linha {}: `{}\"{}\"…`",
                        numero + 1,
                        contexto.trim_start(),
                        literal
                    ));
                }
            }
        }

        achados
    }

    #[test]
    fn a_tela_nao_decide_cor_comparando_texto_do_backend() {
        // O defeito voltou três vezes (`Corrigiu` escondendo, `CorrigiuEmParte` verde por prefixo, desfecho por
        // igualdade): procura a FORMA, literal com cara de prosa comparado por operador de decisão.
        // Toda a interface (`../ui/src`, `.ts` e `.svelte`), lida da pasta: um componente novo não escapa. `tipos.ts`
        // fica de fora: é gerado do Rust e não decide nada.
        let raiz = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("ui")
            .join("src");
        let mut pendentes = vec![raiz.clone()];
        let mut arquivos = Vec::new();
        while let Some(pasta) = pendentes.pop() {
            for entrada in std::fs::read_dir(&pasta)
                .unwrap_or_else(|e| panic!("não consegui ler {:?}: {}", pasta, e))
                .flatten()
            {
                let caminho = entrada.path();
                if caminho.is_dir() {
                    pendentes.push(caminho);
                } else if matches!(caminho.extension().and_then(|e| e.to_str()), Some("ts" | "svelte"))
                    && caminho.file_name().and_then(|n| n.to_str()) != Some("tipos.ts")
                {
                    arquivos.push(caminho);
                }
            }
        }
        arquivos.sort();
        assert!(
            arquivos.len() > 20,
            "só {} arquivos em {:?}; a guarda estaria olhando a pasta errada",
            arquivos.len(),
            raiz
        );

        let mut achados = Vec::new();
        for caminho in &arquivos {
            let fonte = std::fs::read_to_string(caminho)
                .unwrap_or_else(|e| panic!("não consegui ler {:?}: {}", caminho, e));
            let nome = caminho.strip_prefix(&raiz).unwrap_or(caminho).display().to_string();
            achados.extend(achados_de_prosa_comparada(&fonte).into_iter().map(|a| format!("{}: {}", nome, a)));
        }

        assert!(
            achados.is_empty(),
            "a tela voltou a decidir por texto do backend:\n{}",
            achados.join("\n")
        );
    }

    #[test]
    fn a_guarda_da_prosa_reconhece_o_defeito_original() {
        // Canário da própria guarda.
        assert!(parece_prosa_do_backend("Terminou."));
        assert!(parece_prosa_do_backend("Interrompida por você."));
        assert!(parece_prosa_do_backend("Corrigiu "));
        assert!(!parece_prosa_do_backend("Applied"));
        assert!(!parece_prosa_do_backend("VerificarArquivos"));

        assert!(termina_em_operador_de_decisao("desfecho === "));
        assert!(termina_em_operador_de_decisao("resultado.startsWith("));
        assert!(termina_em_operador_de_decisao("    case "));
        assert!(!termina_em_operador_de_decisao("const titulo = "));
    }

    /// Comentário citando o antipadrão não reprova; o mesmo texto como código dispara.
    #[test]
    fn comentario_que_cita_prosa_do_backend_nao_dispara_mas_o_codigo_equivalente_dispara() {
        let comentado = "// antes era: desfecho === \"Terminou.\"\n\
             // devolve \"Corrigiu 2 arquivos.\" quando conserta em parte\n\
             const x = 1;\n";
        assert!(
            achados_de_prosa_comparada(comentado).is_empty(),
            "um comentário citando a prosa antiga não pode reprovar a guarda"
        );

        let codigo = "const cor = desfecho === \"Terminou.\" ? \"ok\" : \"error\";\n";
        assert!(
            !achados_de_prosa_comparada(codigo).is_empty(),
            "o mesmo texto, fora de comentário, precisa continuar disparando"
        );
    }

    #[test]
    fn remover_comentarios_nao_confunde_barra_dentro_de_string() {
        let fonte =
            "const url = \"https://exemplo.com/caminho\"; // comentario de verdade\nconst y = 2;";
        let limpo = remover_comentarios(fonte);

        assert!(
            limpo.contains("https://exemplo.com/caminho"),
            "apagou parte da string, tratando a barra dela como comentário: {:?}",
            limpo
        );
        assert!(
            !limpo.contains("comentario de verdade"),
            "não tirou o comentário de linha de verdade: {:?}",
            limpo
        );
    }

    #[test]
    fn remover_comentarios_preserva_a_contagem_de_linhas() {
        let fonte = "linha1\n/* bloco\nde duas\nlinhas */\nlinha5";
        let limpo = remover_comentarios(fonte);

        assert_eq!(limpo.lines().count(), fonte.lines().count());
    }

    #[test]
    fn o_comando_do_rbar_esta_registrado() {
        // Comando fora do `generate_handler!` só falha na máquina do cliente.
        let lib = std::fs::read_to_string(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("src")
                .join("lib.rs"),
        )
        .expect("lib.rs");
        assert!(
            lib.contains("analyze_rbar"),
            "analyze_rbar fora do generate_handler!"
        );
    }
}
