// "Restaurar meu PC" (MASTER-PLAN, item 6): desfaz TUDO o que o Otimiza mudou, e não só o que está no histórico.
// Até a 3.2 o "Desfazer tudo" passava só pelo `changelog`; ficavam para trás o plano do motor de energia (backup
// próprio), as regras de núcleos por jogo (reaplicadas sozinhas a cada partida) e os modos automáticos, que
// continuariam mexendo no PC depois de "desfazer tudo". Também é o que o desinstalador chama (`--restaurar-tudo`).
//
// Ordem: primeiro desliga o que age sozinho (senão o modo jogo ou o modo dinâmico reaplicariam no meio), depois as
// peças com estado próprio, o histórico, e no fim confere se sobrou o plano OTIMIZA.
//
// Os perfis de energia por jogo NÃO são apagados: só agem com o modo dinâmico, que é desligado aqui, e são medições
// do cliente; apagar jogaria dado fora sem desfazer nada no Windows.
//
// Nunca diz "nada a fazer" sobre o que não conseguiu ler: histórico ou backup ilegível é falha, com o motivo.

use serde::Serialize;

use crate::modules::changelog::ChangeLog;
use crate::modules::optimizer::OptimizationOutcome;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Parte {
    /// Modo jogo automático, modo jogo ligado e modo dinâmico do motor de energia.
    Automaticos,
    /// O plano que o motor de energia aplicou, com o backup dele.
    MotorDeEnergia,
    /// Regras de núcleos por jogo (`cpuset.json`), que se reaplicam a cada partida.
    NucleosPorJogo,
    /// Tudo o que está no histórico de mudanças.
    Historico,
    /// Conferência final: o plano OTIMIZA não pode ter sobrado.
    PlanoOtimiza,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "estado", content = "motivo")]
pub enum Desfecho {
    Voltou,
    NadaAFazer,
    Falhou(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct ResultadoDaParte {
    pub parte: Parte,
    pub desfecho: Desfecho,
    /// O que foi feito, em frases curtas, para a tela e o log.
    pub feito: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Relatorio {
    pub partes: Vec<ResultadoDaParte>,
    /// Item a item do histórico, como o "Desfazer tudo" sempre mostrou.
    pub itens: Vec<OptimizationOutcome>,
    pub tudo_voltou: bool,
}

/// O que se leu de cada peça antes de começar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Leitura {
    Nada,
    Algo,
    Ilegivel,
}

/// **Pura.** O que há para fazer. O motor só entra sem o plano OTIMIZA do botão no histórico: com ele, desfazer o
/// plano pelo histórico já esquece o motor (`esquecer_o_plano`), e restaurar o motor antes refaria o OTIMIZA só
/// para apagá-lo. Leitura ilegível ENTRA: a parte falha dizendo por quê, em vez de sumir como "nada a fazer".
pub fn o_que_fazer(automaticos_ligados: bool, motor: Leitura, otimiza_no_historico: bool, nucleos: Leitura, historico: Leitura) -> Vec<Parte> {
    let mut partes = Vec::new();
    if automaticos_ligados {
        partes.push(Parte::Automaticos);
    }
    if motor == Leitura::Ilegivel || (motor == Leitura::Algo && !otimiza_no_historico) {
        partes.push(Parte::MotorDeEnergia);
    }
    if nucleos != Leitura::Nada {
        partes.push(Parte::NucleosPorJogo);
    }
    if historico != Leitura::Nada {
        partes.push(Parte::Historico);
    }
    partes
}

/// **Pura.** Só "voltou" quando nenhuma parte falhou e nenhum item do histórico ficou.
pub fn tudo_voltou(partes: &[ResultadoDaParte], itens: &[OptimizationOutcome]) -> bool {
    partes.iter().all(|p| !matches!(p.desfecho, Desfecho::Falhou(_))) && itens.iter().all(|i| i.success)
}

/// Antes de mexer: com uma operação interrompida, o valor de antes dela está SÓ no diário (`transacao`), e desfazer
/// por cima o apagaria. Com a prova alternada rodando, ela reativaria o OTIMIZA no meio.
#[cfg(target_os = "windows")]
fn impedimento() -> Option<String> {
    match crate::modules::transacao::pendente() {
        Ok(None) => {}
        Ok(Some(_)) => {
            return Some(
                "Há uma mudança que ficou pela metade. Abra o Otimiza e resolva o aviso de recuperação antes de \
                 desfazer tudo: o valor de antes dela só está guardado ali."
                    .to_string(),
            )
        }
        Err(e) => return Some(format!("Não consegui ler o diário de mudanças pela metade ({}); nada foi desfeito.", e)),
    }
    if crate::modules::provaalternada::em_andamento() {
        return Some("A prova do Otimizar está rodando. Espere ela terminar e desfaça tudo depois.".to_string());
    }
    if super::investigacao::em_andamento() {
        return Some("O teste da travada está rodando. Espere ele terminar (ele devolve tudo) e desfaça depois.".to_string());
    }
    None
}

#[cfg(target_os = "windows")]
pub fn restaurar_tudo<F>(log: &mut ChangeLog, on_step: F) -> Result<Relatorio, String>
where
    F: FnMut(crate::modules::optimizer::BatchStep),
{
    use super::{cpuset, gamemode, motorenergia_maquina as motor};
    use crate::modules::changelog::LeituraDoHistorico;
    use crate::modules::preferences::Preferences;
    use crate::utils::Logger;

    if let Some(motivo) = impedimento() {
        Logger::warn(&format!("restaurar tudo recusado: {}", motivo));
        return Err(motivo);
    }

    let mut preferencias = Preferences::load();
    let dinamico = motor::dinamico().ligado;
    let modo_jogo = gamemode::status(log).active;
    let teste_da_travada = super::investigacao::ha_acalmado();
    let regras = cpuset::ler();
    let estado_do_motor = motor::estado_do_backup();
    let historico_ilegivel = match log.leitura() {
        LeituraDoHistorico::Ilegivel { motivo, guardado_em } => Some(format!(
            "o histórico de mudanças está ilegível ({}){}; o que ele guardava não pôde ser desfeito por aqui",
            motivo,
            guardado_em.as_deref().map(|c| format!(", cópia em {}", c)).unwrap_or_default()
        )),
        _ => None,
    };
    let planejado = o_que_fazer(
        preferencias.auto_game_mode || dinamico || modo_jogo || teste_da_travada,
        match &estado_do_motor {
            Ok(false) => Leitura::Nada,
            Ok(true) => Leitura::Algo,
            Err(_) => Leitura::Ilegivel,
        },
        log.is_applied("plano_otimiza"),
        if regras.is_empty() { Leitura::Nada } else { Leitura::Algo },
        if historico_ilegivel.is_some() {
            Leitura::Ilegivel
        } else if log.applied().is_empty() {
            Leitura::Nada
        } else {
            Leitura::Algo
        },
    );
    Logger::info(&format!("restaurar tudo: {:?}", planejado));

    let mut partes = Vec::new();
    let mut itens = Vec::new();
    let mut on_step = on_step;
    for parte in [Parte::Automaticos, Parte::MotorDeEnergia, Parte::NucleosPorJogo, Parte::Historico] {
        if !planejado.contains(&parte) {
            partes.push(ResultadoDaParte { parte, desfecho: Desfecho::NadaAFazer, feito: Vec::new() });
            continue;
        }
        let mut feito = Vec::new();
        let mut falhas = Vec::new();
        match parte {
            Parte::Automaticos => {
                if preferencias.auto_game_mode {
                    preferencias.auto_game_mode = false;
                    match preferencias.save() {
                        Ok(()) => feito.push("Modo jogo automático desligado.".to_string()),
                        Err(e) => falhas.push(format!("modo jogo automático: {}", e)),
                    }
                }
                if modo_jogo {
                    match gamemode::desativar(log) {
                        Ok(frase) => feito.push(frase),
                        Err(e) => falhas.push(e),
                    }
                }
                if teste_da_travada {
                    match super::investigacao::devolver_mantido() {
                        Ok(Some(frase)) => feito.push(format!("Teste da travada: {}", frase)),
                        Ok(None) => {}
                        Err(e) => falhas.push(format!("teste da travada: {}", e)),
                    }
                }
                if dinamico {
                    match motor::definir_dinamico(false) {
                        Ok(_) => feito.push("Modo dinâmico do motor de energia desligado.".to_string()),
                        Err(e) => falhas.push(format!("modo dinâmico: {}", e)),
                    }
                }
            }
            Parte::MotorDeEnergia => match &estado_do_motor {
                Err(e) => falhas.push(format!("backup do motor de energia ilegível ({}); o plano dele pode ter ficado", e)),
                Ok(_) => match motor::restaurar_anterior(false) {
                    Ok(r) if r.divergentes.is_empty() => feito.push("O plano de energia de antes do motor voltou.".to_string()),
                    Ok(r) => falhas.push(format!("o plano voltou, mas {} ajuste(s) não: {}", r.divergentes.len(), r.divergentes.join(", "))),
                    // O plano de antes pode não existir mais: o padrão do Windows é o único "antes" que resta.
                    Err(e) => match motor::restaurar_padrao_windows() {
                        Ok(_) => feito.push(format!(
                            "O plano de antes do motor não voltou ({}); ficou o Equilibrado, o padrão do Windows.",
                            e
                        )),
                        Err(e2) => falhas.push(format!("motor de energia: {}; e o padrão do Windows também não: {}", e, e2)),
                    },
                },
            },
            Parte::NucleosPorJogo => {
                for exe in regras.keys() {
                    match cpuset::esquecer(exe) {
                        Ok(()) => feito.push(format!("Regra de núcleos de {} esquecida.", exe)),
                        Err(e) => falhas.push(format!("{}: {}", exe, e)),
                    }
                }
            }
            Parte::Historico => {
                if let Some(motivo) = &historico_ilegivel {
                    falhas.push(motivo.clone());
                } else {
                    itens = super::WindowsOptimizer::new().revert_all(log, &mut on_step);
                    let nao_voltaram = itens.iter().filter(|i| !i.success).count();
                    feito.push(format!("{} item(ns) do histórico desfeito(s).", itens.len() - nao_voltaram));
                    if nao_voltaram > 0 {
                        falhas.push(format!("{} item(ns) do histórico não voltaram", nao_voltaram));
                    }
                }
            }
            Parte::PlanoOtimiza => {}
        }
        let desfecho = if falhas.is_empty() { Desfecho::Voltou } else { Desfecho::Falhou(falhas.join("; ")) };
        Logger::info(&format!("restaurar tudo: {:?} → {:?}", parte, desfecho));
        partes.push(ResultadoDaParte { parte, desfecho, feito });
    }

    // O desfazer do plano só anota no log quando o Windows recusa apagar: aqui a sobra aparece.
    let conferencia = match motor::sobrou_o_plano_otimiza() {
        Ok(false) => Desfecho::NadaAFazer,
        Ok(true) => Desfecho::Falhou(
            "o plano de energia OTIMIZA ainda existe no Windows; apague-o em Opções de energia se não o quiser".to_string(),
        ),
        Err(e) => Desfecho::Falhou(format!("não consegui conferir se o plano OTIMIZA sobrou: {}", e)),
    };
    Logger::info(&format!("restaurar tudo: conferência do plano OTIMIZA → {:?}", conferencia));
    partes.push(ResultadoDaParte { parte: Parte::PlanoOtimiza, desfecho: conferencia, feito: Vec::new() });

    let tudo = tudo_voltou(&partes, &itens);
    Ok(Relatorio { partes, itens, tudo_voltou: tudo })
}

/// O desinstalador chama `pc-optimizer.exe --restaurar-tudo --dados "<APPDATA de quem instalou>"`.
pub const ARGUMENTO: &str = "--restaurar-tudo";
const ARGUMENTO_DADOS: &str = "--dados";

/// Onde o desinstalador lê o código de saída (`ExecShellWait` não o devolve), dentro da pasta de dados.
pub const ARQUIVO_DO_RESULTADO: &str = "restauracao-resultado.txt";

#[derive(Debug, PartialEq, Eq)]
pub enum Pedido {
    /// Abertura normal do programa.
    Nenhum,
    Restaurar { dados: Option<std::path::PathBuf> },
    Invalido(String),
}

/// **Pura.** Só olha quando o primeiro argumento é o nosso: qualquer outra abertura segue como sempre. Nada além de
/// `--dados <pasta absoluta>` é aceito depois dele.
pub fn ler_argumentos(argumentos: &[String]) -> Pedido {
    match argumentos {
        [a] if a == ARGUMENTO => Pedido::Restaurar { dados: None },
        [a, d, pasta] if a == ARGUMENTO && d == ARGUMENTO_DADOS => {
            let caminho = std::path::PathBuf::from(pasta);
            if caminho.is_absolute() {
                Pedido::Restaurar { dados: Some(caminho) }
            } else {
                Pedido::Invalido(format!("{} precisa de uma pasta absoluta", ARGUMENTO_DADOS))
            }
        }
        [a, ..] if a == ARGUMENTO => Pedido::Invalido(format!("uso: {} [{} <pasta>]", ARGUMENTO, ARGUMENTO_DADOS)),
        _ => Pedido::Nenhum,
    }
}

/// **Pura.** A mesma pasta de dados, sem diferença de maiúscula nem de barra no fim (NTFS não diferencia).
pub fn mesma_pasta(a: &std::path::Path, b: &std::path::Path) -> bool {
    let normal = |p: &std::path::Path| p.to_string_lossy().trim_end_matches(['\\', '/']).to_lowercase();
    normal(a) == normal(b)
}

/// Pelo desinstalador: restaura sem abrir a janela, mostra o resultado numa caixa de mensagem e grava o código em
/// `ARQUIVO_DO_RESULTADO` para o desinstalador decidir se continua. Códigos: 0 tudo voltou, 1 parte não voltou,
/// 2 sem administrador, 3 o Otimiza está aberto, 4 outra conta, 5 recusado antes de mexer.
#[cfg(target_os = "windows")]
pub fn pela_linha_de_comando(dados: Option<std::path::PathBuf>) -> i32 {
    let codigo = restaurar_pela_linha_de_comando(&dados);
    // Só na pasta de dados de quem pediu, e só quando ela é a nossa (código 4 não escreve na pasta de outra conta).
    if let (Some(pasta), true) = (&dados, codigo != 4) {
        let _ = std::fs::write(pasta.join("pc-optimizer").join(ARQUIVO_DO_RESULTADO), codigo.to_string());
    }
    codigo
}

#[cfg(target_os = "windows")]
fn restaurar_pela_linha_de_comando(dados: &Option<std::path::PathBuf>) -> i32 {
    use crate::utils::Logger;

    // Quem autorizou pelo UAC pode ser OUTRA conta (administrador digitando a senha para um usuário comum): aí o
    // histórico e o registro do usuário seriam os dela, e "restaurar" mexeria no PC errado.
    if let (Some(pedida), Ok(nossa)) = (dados, std::env::var("APPDATA")) {
        if !mesma_pasta(pedida, std::path::Path::new(&nossa)) {
            avisar(
                "O Otimiza não desfez nada: a permissão foi dada por outra conta do Windows, e o que ele mudou está \
                 na conta de quem o usou.",
            );
            return 4;
        }
    }
    if !super::registry::is_elevated() {
        avisar("O Otimiza não desfez nada: é preciso permitir como administrador.");
        return 2;
    }
    if outro_otimiza_aberto() {
        avisar("O Otimiza não desfez nada: ele ainda está aberto.");
        return 3;
    }

    let mut log = ChangeLog::load();
    let r = match restaurar_tudo(&mut log, |_| {}) {
        Ok(r) => r,
        Err(motivo) => {
            avisar(&format!("O Otimiza não desfez nada. {}", motivo));
            return 5;
        }
    };
    Logger::info(&format!("restaurar tudo pela desinstalação: tudo voltou = {}", r.tudo_voltou));
    let feito: Vec<String> = r.partes.iter().flat_map(|p| p.feito.clone()).collect();
    let falhas: Vec<String> = r
        .partes
        .iter()
        .filter_map(|p| match &p.desfecho {
            Desfecho::Falhou(m) => Some(m.clone()),
            _ => None,
        })
        .chain(r.itens.iter().filter(|i| !i.success).map(|i| format!("{}: {}", i.name, i.message)))
        .collect();
    let texto = if feito.is_empty() && falhas.is_empty() {
        "O Otimiza não tinha nenhuma mudança ativa neste PC.".to_string()
    } else if r.tudo_voltou {
        format!("Tudo o que o Otimiza mudou voltou ao que era.\n\n{}", feito.join("\n"))
    } else {
        format!("Parte não voltou:\n{}\n\nO que voltou:\n{}", falhas.join("\n"), feito.join("\n"))
    };
    avisar(&texto);
    if r.tudo_voltou {
        0
    } else {
        1
    }
}

#[cfg(target_os = "windows")]
fn outro_otimiza_aberto() -> bool {
    let Some(nosso) = std::env::current_exe().ok().and_then(|c| c.file_name().map(|n| n.to_owned())) else {
        return false;
    };
    let eu = sysinfo::Pid::from_u32(std::process::id());
    let mut s = sysinfo::System::new();
    s.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    s.processes().iter().any(|(pid, p)| *pid != eu && p.name().eq_ignore_ascii_case(&nosso))
}

#[cfg(target_os = "windows")]
fn avisar(texto: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONINFORMATION, MB_OK};
    let largo = |s: &str| s.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
    let (corpo, titulo) = (largo(texto), largo("Otimiza"));
    unsafe {
        MessageBoxW(std::ptr::null_mut(), corpo.as_ptr(), titulo.as_ptr(), MB_OK | MB_ICONINFORMATION);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn abertura_normal_nao_e_pedido() {
        assert_eq!(ler_argumentos(&[]), Pedido::Nenhum);
        assert_eq!(ler_argumentos(&args(&["--qualquer"])), Pedido::Nenhum);
    }

    #[test]
    fn o_desinstalador_pede_com_ou_sem_a_pasta() {
        assert_eq!(ler_argumentos(&args(&[ARGUMENTO])), Pedido::Restaurar { dados: None });
        assert_eq!(
            ler_argumentos(&args(&[ARGUMENTO, "--dados", r"C:\Users\x\AppData\Roaming"])),
            Pedido::Restaurar { dados: Some(r"C:\Users\x\AppData\Roaming".into()) }
        );
    }

    #[test]
    fn argumento_a_mais_ou_pasta_relativa_e_recusado() {
        assert!(matches!(ler_argumentos(&args(&[ARGUMENTO, "--dados", "relativa"])), Pedido::Invalido(_)));
        assert!(matches!(ler_argumentos(&args(&[ARGUMENTO, "--outra"])), Pedido::Invalido(_)));
    }

    #[test]
    fn a_pasta_de_dados_compara_sem_maiuscula_nem_barra() {
        use std::path::Path;
        assert!(mesma_pasta(Path::new(r"C:\Users\X\AppData\Roaming\"), Path::new(r"c:\users\x\appdata\roaming")));
        assert!(!mesma_pasta(Path::new(r"C:\Users\admin\AppData\Roaming"), Path::new(r"C:\Users\x\AppData\Roaming")));
    }

    #[test]
    fn maquina_limpa_nao_tem_nada_a_fazer() {
        assert!(o_que_fazer(false, Leitura::Nada, false, Leitura::Nada, Leitura::Nada).is_empty());
    }

    #[test]
    fn cada_peca_com_estado_proprio_entra() {
        assert_eq!(
            o_que_fazer(true, Leitura::Algo, false, Leitura::Algo, Leitura::Algo),
            vec![Parte::Automaticos, Parte::MotorDeEnergia, Parte::NucleosPorJogo, Parte::Historico]
        );
    }

    #[test]
    fn com_o_otimiza_no_historico_o_motor_sai_pelo_historico() {
        let partes = o_que_fazer(false, Leitura::Algo, true, Leitura::Nada, Leitura::Algo);
        assert_eq!(partes, vec![Parte::Historico], "restaurar o motor antes refaria o OTIMIZA só para apagá-lo");
    }

    #[test]
    fn o_que_nao_se_leu_entra_para_falhar_e_nao_some() {
        let partes = o_que_fazer(false, Leitura::Ilegivel, true, Leitura::Nada, Leitura::Ilegivel);
        assert_eq!(partes, vec![Parte::MotorDeEnergia, Parte::Historico]);
    }

    fn parte(desfecho: Desfecho) -> ResultadoDaParte {
        ResultadoDaParte { parte: Parte::Historico, desfecho, feito: Vec::new() }
    }

    #[test]
    fn uma_falha_em_qualquer_parte_nao_e_tudo_voltou() {
        assert!(tudo_voltou(&[parte(Desfecho::Voltou), parte(Desfecho::NadaAFazer)], &[]));
        assert!(!tudo_voltou(&[parte(Desfecho::Voltou), parte(Desfecho::Falhou("x".into()))], &[]));
        let falhou = OptimizationOutcome::failed("a", "A", "não voltou".to_string());
        assert!(!tudo_voltou(&[parte(Desfecho::Voltou)], &[falhou]));
    }
}
