// A prova do botão "Otimizar" no jogo do cliente, com o jogo ABERTO: troca entre o plano de energia de antes (A) e o
// do Otimiza (B) em rodadas A B B A B A A B e compara pelo critério único de `modules::repeticoes`. Medir "tudo antes,
// tudo depois" põe o aquecimento da máquina e o ponto do jogo no lado do depois; alternar espalha os dois pelos
// dois lados, e a ordem (Thue-Morse) anula a deriva em linha reta.
//
// Só o plano entra porque é o único ajuste do lote que muda FPS com o jogo aberto: o Game DVR vale da próxima vez
// que o jogo abre, e o que é de inicialização, do próximo reinício. O resto do lote não é julgado aqui, e a tela
// diz isso. Piorou: o plano é desfeito. Nunca menos FPS.

use serde::{Deserialize, Serialize};

use crate::modules::medicoes::GeracaoNaPartida;
use crate::modules::repeticoes::{self, Diferenca, Resumo};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Lado {
    SemOtimiza,
    ComOtimiza,
}

/// Com a máquina esquentando em linha reta, cada lado recebe o mesmo tanto de começo e de fim: as posições somam
/// igual (0+3+5+6 = 1+2+4+7). Com seis rodadas isso é impossível (a soma é ímpar), por isso oito.
pub const ORDEM: [Lado; 8] = [
    Lado::SemOtimiza,
    Lado::ComOtimiza,
    Lado::ComOtimiza,
    Lado::SemOtimiza,
    Lado::ComOtimiza,
    Lado::SemOtimiza,
    Lado::SemOtimiza,
    Lado::ComOtimiza,
];

/// Descartado: o primeiro minuto de jogo ainda compila shader e enche cache.
pub const AQUECIMENTO_S: u64 = 60;

/// Depois de trocar o plano, até o processador obedecer.
pub const ESPERA_DEPOIS_DE_TROCAR_S: u64 = 3;

/// 30 s a 23 FPS já dá os mil quadros do 1%.
pub const SEGUNDOS_POR_RODADA_MIN: u64 = 30;
pub const SEGUNDOS_POR_RODADA_MAX: u64 = 60;

pub fn duracao_estimada_s(segundos_por_rodada: u64) -> u64 {
    AQUECIMENTO_S + ORDEM.len() as u64 * (segundos_por_rodada + ESPERA_DEPOIS_DE_TROCAR_S)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rodada {
    pub lado: Lado,
    /// Só os quadros que o jogo desenhou.
    pub fps_do_jogo: f64,
    /// `None` abaixo de mil quadros (a regra única do 1%).
    pub low_1pct: Option<f64>,
    /// Pelo PresentMon; `None` pelo canal antigo.
    pub fps_exibido: Option<f64>,
    pub quadros: usize,
    pub geracao: Option<GeracaoNaPartida>,
    pub configuracao_do_jogo: Option<String>,
    /// Falso quando o PresentMon falhou e mediu o canal antigo, que conta tudo o que foi apresentado.
    pub pelo_presentmon: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Desfecho {
    /// Mais FPS do jogo ou 1% pior mais alto, além do ruído, sem piora no outro.
    Ganhou,
    /// Menos FPS ou 1% pior mais baixo, além do ruído. O plano é desfeito.
    Piorou,
    /// As rodadas não distinguem os dois lados. NÃO é "não faz nada": é o que esta medição consegue dizer.
    Indistinguivel,
    /// Configuração gráfica ou gerador de quadros mudou no meio, as rodadas vieram de medidores diferentes, ou faltou
    /// rodada.
    SemComparacaoJusta,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Resultado {
    pub jogo: String,
    pub quando: u64,
    pub segundos_por_rodada: u64,
    pub rodadas: Vec<Rodada>,
    pub fps_sem: Option<Resumo>,
    pub fps_com: Option<Resumo>,
    pub low_sem: Option<Resumo>,
    pub low_com: Option<Resumo>,
    pub diferenca_fps: Option<Diferenca>,
    pub diferenca_low: Option<Diferenca>,
    pub desfecho: Desfecho,
    /// A frase para a tela; a tela escolhe cor e ícone pelo `desfecho`, nunca por este texto.
    pub leitura: String,
    /// O lado A foi o Equilibrado do Windows porque o plano de antes não existe mais.
    pub contra_o_equilibrado: bool,
    /// Falso só quando piorou: o plano do Otimiza foi desfeito.
    pub ficou_com_otimiza: bool,
    /// Preenchido quando o desfazer depois de piorar falhou: a pessoa precisa saber que o plano ficou.
    #[serde(default)]
    pub falha_ao_desfazer: Option<String>,
}

fn do_lado(rodadas: &[Rodada], lado: Lado) -> impl Iterator<Item = &Rodada> {
    rodadas.iter().filter(move |r| r.lado == lado)
}

/// Dois valores `Some` diferentes: `None` é "não deu para ler", não "mudou".
fn mudou<T: PartialEq>(valores: impl Iterator<Item = Option<T>>) -> bool {
    let lidos: Vec<T> = valores.flatten().collect();
    lidos.windows(2).any(|par| par[0] != par[1])
}

fn piora(d: &Option<Diferenca>) -> bool {
    matches!(d, Some(Diferenca::Real { delta, .. }) if *delta < 0.0)
}

fn ganho(d: &Option<Diferenca>) -> bool {
    matches!(d, Some(Diferenca::Real { delta, .. }) if *delta > 0.0)
}

/// Pura: recebe as rodadas medidas e devolve o veredito, para cada caso ser provado sem jogo aberto.
pub fn decidir(
    jogo: &str,
    quando: u64,
    segundos_por_rodada: u64,
    rodadas: Vec<Rodada>,
    contra_o_equilibrado: bool,
) -> Resultado {
    let fps = |lado| do_lado(&rodadas, lado).map(|r| r.fps_do_jogo).collect::<Vec<f64>>();
    let fps_sem = repeticoes::resumir("sem", &fps(Lado::SemOtimiza));
    let fps_com = repeticoes::resumir("com", &fps(Lado::ComOtimiza));

    // 1% só com TODAS as rodadas tendo amostra: média de metade das rodadas compararia momentos diferentes.
    let low = |lado| -> Option<Vec<f64>> { do_lado(&rodadas, lado).map(|r| r.low_1pct).collect() };
    let low_sem = low(Lado::SemOtimiza).and_then(|v| repeticoes::resumir("sem", &v));
    let low_com = low(Lado::ComOtimiza).and_then(|v| repeticoes::resumir("com", &v));

    let diferenca_fps = match (&fps_sem, &fps_com) {
        (Some(a), Some(b)) => Some(repeticoes::comparar(a, b)),
        _ => None,
    };
    let diferenca_low = match (&low_sem, &low_com) {
        (Some(a), Some(b)) => Some(repeticoes::comparar(a, b)),
        _ => None,
    };

    let faltou_rodada = [Lado::SemOtimiza, Lado::ComOtimiza]
        .iter()
        .any(|l| do_lado(&rodadas, *l).count() < repeticoes::REPETICOES_MINIMAS);
    let mudou_a_configuracao = mudou(rodadas.iter().map(|r| r.configuracao_do_jogo.clone()));
    let mudou_a_geracao = mudou(rodadas.iter().map(|r| r.geracao));
    let mudou_o_medidor = mudou(rodadas.iter().map(|r| Some(r.pelo_presentmon)));

    let (desfecho, leitura) = if mudou_a_configuracao {
        (
            Desfecho::SemComparacaoJusta,
            "A configuração gráfica do jogo mudou no meio do teste. Um lado rodou com gráfico \
             diferente do outro, e a diferença seria da qualidade, não do Otimiza."
                .to_string(),
        )
    } else if mudou_a_geracao {
        (
            Desfecho::SemComparacaoJusta,
            "Um gerador de quadros ligou ou desligou no meio do teste. Com ele, o FPS medido \
             não compara os dois planos."
                .to_string(),
        )
    } else if mudou_o_medidor {
        (
            Desfecho::SemComparacaoJusta,
            "O PresentMon falhou em parte das rodadas e elas foram medidas pelo canal antigo, que conta os \
             quadros de outro jeito. Rodadas de medidores diferentes não se comparam."
                .to_string(),
        )
    } else if faltou_rodada {
        (
            Desfecho::SemComparacaoJusta,
            format!(
                "Faltaram rodadas: são precisas pelo menos {} de cada lado para existir margem de erro.",
                repeticoes::REPETICOES_MINIMAS
            ),
        )
    } else if piora(&diferenca_fps) || piora(&diferenca_low) {
        (
            Desfecho::Piorou,
            "Com o plano do Otimiza o jogo rodou pior nesta máquina, além do ruído. O plano foi \
             desfeito e o seu voltou."
                .to_string(),
        )
    } else if ganho(&diferenca_fps) || ganho(&diferenca_low) {
        (
            Desfecho::Ganhou,
            "Com o plano do Otimiza o jogo rodou melhor nesta máquina, e a diferença passou do \
             ruído nas rodadas alternadas. O plano fica."
                .to_string(),
        )
    } else {
        (
            Desfecho::Indistinguivel,
            "As rodadas não separaram os dois planos: a diferença ficou dentro do ruído do jogo. \
             Não piorou além do ruído, então o plano fica; o ganho dele aqui não foi comprovado."
                .to_string(),
        )
    };

    Resultado {
        jogo: jogo.to_string(),
        quando,
        segundos_por_rodada,
        rodadas,
        fps_sem,
        fps_com,
        low_sem,
        low_com,
        diferenca_fps,
        diferenca_low,
        ficou_com_otimiza: desfecho != Desfecho::Piorou,
        desfecho,
        leitura,
        contra_o_equilibrado,
        falha_ao_desfazer: None,
    }
}

/// Piorou e o Windows não deixou voltar: a frase e o estado dizem isso, em vez de "foi desfeito".
pub fn marcar_falha_ao_desfazer(r: &mut Resultado, erro: String) {
    r.ficou_com_otimiza = true;
    r.leitura = format!(
        "Com o plano do Otimiza o jogo rodou pior nesta máquina, além do ruído, mas o Windows não deixou \
         voltar ao plano de antes ({}). Escolha outro plano em Opções de Energia do Windows, ou desfaça \
         \"Plano de energia OTIMIZA\" na lista de otimizações se ele ainda aparecer lá.",
        erro
    );
    r.falha_ao_desfazer = Some(erro);
}

fn pasta() -> std::path::PathBuf {
    std::env::var("APPDATA")
        .or_else(|_| std::env::var("HOME"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join("pc-optimizer")
}

pub fn guardar(r: &Resultado) -> Result<(), String> {
    let destino = pasta().join("prova_alternada.json");
    if let Some(dir) = destino.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("não consegui criar a pasta: {}", e))?;
    }
    let bruto = serde_json::to_string_pretty(r).map_err(|e| format!("não consegui serializar: {}", e))?;
    std::fs::write(destino, bruto).map_err(|e| format!("não consegui gravar a prova: {}", e))
}

pub fn guardada() -> Option<Resultado> {
    std::fs::read_to_string(pasta().join("prova_alternada.json"))
        .ok()
        .and_then(|b| serde_json::from_str(&b).ok())
}

#[cfg(target_os = "windows")]
pub use maquina::*;

#[cfg(target_os = "windows")]
mod maquina {
    use super::*;
    use crate::modules::changelog::{ChangeLog, ChangeRecord};
    use crate::modules::windows::{configjogo, frames, planoenergia, power, presentmon};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    static EM_ANDAMENTO: AtomicBool = AtomicBool::new(false);

    /// A medição automática fica de fora enquanto isto roda: uma partida meio A, meio B iria para o histórico.
    pub fn em_andamento() -> bool {
        EM_ANDAMENTO.load(Ordering::SeqCst)
    }

    struct Vez;
    impl Drop for Vez {
        fn drop(&mut self) {
            EM_ANDAMENTO.store(false, Ordering::SeqCst);
        }
    }

    #[derive(Debug, Clone)]
    pub struct Planos {
        pub sem: String,
        pub com: String,
        pub contra_o_equilibrado: bool,
    }

    /// A = o plano que estava antes do Otimiza (o `desfazer` volta para ele); B = o OTIMIZA, que precisa estar ATIVO.
    pub fn planos(log: &ChangeLog) -> Result<Planos, String> {
        let Some(entrada) = log.applied().iter().find(|e| e.optimization_id == "plano_otimiza") else {
            return Err(
                "O plano de energia do Otimiza não está aplicado. Aperte \"Otimizar agora\" primeiro: \
                 a prova compara o plano do Otimiza com o seu, com o jogo aberto."
                    .to_string(),
            );
        };
        let lista = planoenergia::listar_planos()?;
        let nosso = planoenergia::achar_na_lista(&lista, planoenergia::NOME_DO_PLANO)
            .ok_or("O plano OTIMIZA não existe mais nesta máquina. Aperte \"Otimizar agora\" de novo.")?;
        let ativo = power::active_scheme()?;
        if !ativo.eq_ignore_ascii_case(&nosso) {
            return Err(
                "O plano ativo agora não é o OTIMIZA (o modo dinâmico ou outro programa trocou). \
                 A prova só compara com o plano do Otimiza ativo."
                    .to_string(),
            );
        }

        let anterior = entrada.changes.iter().find_map(|c| match c {
            ChangeRecord::PowerPlan { previous_guid } => Some(previous_guid.clone()),
            _ => None,
        });
        let existe = |guid: &str| lista.iter().any(|(g, _)| g.eq_ignore_ascii_case(guid));

        match anterior.filter(|g| !g.eq_ignore_ascii_case(&nosso) && existe(g)) {
            Some(sem) => Ok(Planos { sem, com: nosso, contra_o_equilibrado: false }),
            None => Ok(Planos {
                sem: planoenergia::EQUILIBRADO_GUID.to_string(),
                com: nosso,
                contra_o_equilibrado: true,
            }),
        }
    }

    fn marcador() -> std::path::PathBuf {
        pasta().join("prova_alternada_em_andamento.txt")
    }

    /// O programa caiu no meio: o plano do Otimiza volta a ser o ativo, como estava antes do teste.
    pub fn recuperar_na_abertura() -> Option<Result<(), String>> {
        let guid = std::fs::read_to_string(marcador()).ok()?;
        let r = ativar(guid.trim());
        let _ = std::fs::remove_file(marcador());
        Some(r)
    }

    fn ativar(guid: &str) -> Result<(), String> {
        power::set_active_scheme(guid)?;
        let ativo = power::active_scheme()?;
        if !ativo.eq_ignore_ascii_case(guid) {
            return Err(format!("O Windows aceitou trocar o plano, mas o ativo continua {}.", ativo));
        }
        Ok(())
    }

    #[derive(Debug, Clone, Serialize)]
    pub struct Passo {
        /// 0 é o aquecimento; 1 a 8, as rodadas.
        pub indice: usize,
        pub total: usize,
        pub lado: Option<Lado>,
        pub segundos: u64,
    }

    fn jogo_aberto(pid: u32) -> bool {
        let mut s = sysinfo::System::new();
        s.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[sysinfo::Pid::from_u32(pid)]), true);
        s.process(sysinfo::Pid::from_u32(pid)).is_some()
    }

    fn medir(pid: u32, nome: &str, lado: Lado, segundos: u64) -> Result<Rodada, String> {
        let configuracao_do_jogo = configjogo::impressao_da_configuracao(nome);
        match presentmon::medir_como_antes(pid, nome, segundos as u32) {
            Ok((_, r)) => Ok(Rodada {
                lado,
                fps_do_jogo: r.fps_do_jogo_medio,
                low_1pct: r.low_1pct,
                fps_exibido: Some(r.fps_exibido),
                quadros: r.quadros_do_jogo,
                geracao: Some(crate::modules::medicoes::geracao_agora(Some(&r))),
                configuracao_do_jogo,
                pelo_presentmon: true,
            }),
            Err(erro) => {
                crate::utils::Logger::warn(&format!("prova alternada: PresentMon não mediu ({}); canal antigo", erro));
                let m = frames::medir(pid, nome, segundos)?;
                Ok(Rodada {
                    lado,
                    fps_do_jogo: m.fps,
                    low_1pct: m.detalhe_confiavel.then_some(m.low_1pct),
                    fps_exibido: None,
                    quadros: m.frames as usize,
                    geracao: Some(crate::modules::medicoes::geracao_agora(None)),
                    configuracao_do_jogo,
                    pelo_presentmon: false,
                })
            }
        }
    }

    fn rodadas(
        pid: u32,
        nome: &str,
        segundos: u64,
        planos: &Planos,
        emitir: &dyn Fn(Passo),
    ) -> Result<Vec<Rodada>, String> {
        let total = ORDEM.len();
        emitir(Passo { indice: 0, total, lado: None, segundos: AQUECIMENTO_S });
        for _ in 0..AQUECIMENTO_S {
            std::thread::sleep(Duration::from_secs(1));
            if !jogo_aberto(pid) {
                return Err("O jogo fechou durante o aquecimento. Nada foi medido.".to_string());
            }
        }

        let mut feitas = Vec::with_capacity(total);
        for (i, lado) in ORDEM.iter().enumerate() {
            let guid = match lado {
                Lado::SemOtimiza => &planos.sem,
                Lado::ComOtimiza => &planos.com,
            };
            ativar(guid)?;
            emitir(Passo { indice: i + 1, total, lado: Some(*lado), segundos });
            std::thread::sleep(Duration::from_secs(ESPERA_DEPOIS_DE_TROCAR_S));
            if !jogo_aberto(pid) {
                return Err("O jogo fechou no meio do teste. O plano do Otimiza voltou a ser o ativo.".to_string());
            }
            let rodada = medir(pid, nome, *lado, segundos)?;
            // Fechou durante a rodada: o que foi medido pode ser o menu de saída.
            if !jogo_aberto(pid) {
                return Err("O jogo fechou no meio do teste. O plano do Otimiza voltou a ser o ativo.".to_string());
            }
            crate::utils::Logger::info(&format!(
                "prova alternada: rodada {}/{} {:?}: {:.1} FPS do jogo, 1% {:?}, {} quadros",
                i + 1,
                total,
                lado,
                rodada.fps_do_jogo,
                rodada.low_1pct,
                rodada.quadros
            ));
            feitas.push(rodada);
        }
        Ok(feitas)
    }

    /// Depois de piorar. Pelo histórico primeiro (desfaz e apaga o OTIMIZA); e o plano ATIVO é que decide: sem
    /// `PowerPlan` no histórico (o OTIMIZA já era o ativo ao aplicar) o desfazer não troca nada, e comparar contra o
    /// Equilibrado deixaria o pior ativo.
    pub fn voltar_ao_de_antes(planos: &Planos, log: &mut ChangeLog) -> Result<(), String> {
        let pelo_historico = crate::modules::windows::WindowsOptimizer::new().revert("plano_otimiza", log);
        if power::active_scheme().is_ok_and(|a| a.eq_ignore_ascii_case(&planos.sem)) {
            return Ok(());
        }
        ativar(&planos.sem).map_err(|e| match pelo_historico {
            Err(h) => format!("{h}; {e}"),
            Ok(_) => e,
        })
    }

    /// Termina sempre com o plano do Otimiza ativo (quem decide desfazer é o chamador, pelo `desfecho`).
    pub fn executar(
        pid: u32,
        nome: &str,
        segundos_por_rodada: u64,
        planos: &Planos,
        emitir: &dyn Fn(Passo),
    ) -> Result<Vec<Rodada>, String> {
        if EM_ANDAMENTO.swap(true, Ordering::SeqCst) {
            return Err("Já há uma prova rodando.".to_string());
        }
        let _vez = Vez;

        std::fs::create_dir_all(pasta()).map_err(|e| format!("não consegui criar a pasta: {}", e))?;
        std::fs::write(marcador(), &planos.com).map_err(|e| format!("não consegui marcar o teste: {}", e))?;

        let segundos = segundos_por_rodada.clamp(SEGUNDOS_POR_RODADA_MIN, SEGUNDOS_POR_RODADA_MAX);
        let resultado = rodadas(pid, nome, segundos, planos, emitir);

        let volta = ativar(&planos.com);
        let _ = std::fs::remove_file(marcador());
        if let Err(e) = volta {
            crate::utils::Logger::warn(&format!("prova alternada: o plano do Otimiza não voltou: {}", e));
            return Err(format!(
                "O teste terminou, mas o Windows não reativou o plano do Otimiza ({}). \
                 Aperte \"Otimizar agora\" de novo.",
                e
            ));
        }
        resultado
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rodada(lado: Lado, fps: f64, low: Option<f64>) -> Rodada {
        Rodada {
            lado,
            fps_do_jogo: fps,
            low_1pct: low,
            fps_exibido: Some(fps),
            quadros: (fps * 60.0) as usize,
            geracao: Some(GeracaoNaPartida::NenhumaVisivel),
            configuracao_do_jogo: Some("abc".into()),
            pelo_presentmon: true,
        }
    }

    /// Na ordem de `ORDEM`, cada lado consumindo os seus valores na sequência.
    fn todas(sem: [f64; 4], com: [f64; 4]) -> Vec<Rodada> {
        let (mut s, mut c) = (sem.iter(), com.iter());
        ORDEM
            .iter()
            .map(|lado| {
                let v = *match lado {
                    Lado::SemOtimiza => s.next(),
                    Lado::ComOtimiza => c.next(),
                }
                .unwrap();
                rodada(*lado, v, Some(v * 0.7))
            })
            .collect()
    }

    #[test]
    fn a_ordem_equilibra_a_deriva_e_tem_repeticoes_dos_dois_lados() {
        let sem = ORDEM.iter().filter(|l| **l == Lado::SemOtimiza).count();
        let com = ORDEM.iter().filter(|l| **l == Lado::ComOtimiza).count();
        assert_eq!(sem, com);
        assert!(sem >= repeticoes::REPETICOES_MINIMAS);

        // Soma das posições igual dos dois lados: deriva em linha reta pesa o mesmo em cada um.
        let posicoes = |lado| ORDEM.iter().enumerate().filter(|(_, l)| **l == lado).map(|(i, _)| i).sum::<usize>();
        assert_eq!(posicoes(Lado::SemOtimiza), posicoes(Lado::ComOtimiza));
    }

    #[test]
    fn ganho_claro_fica_com_o_otimiza() {
        let r = decidir("FiveM.exe", 0, 60, todas([70.0, 71.0, 70.5, 70.8], [78.0, 79.0, 78.5, 78.2]), false);
        assert_eq!(r.desfecho, Desfecho::Ganhou);
        assert!(r.ficou_com_otimiza);
    }

    #[test]
    fn piora_clara_desfaz() {
        let r = decidir("FiveM.exe", 0, 60, todas([78.0, 79.0, 78.5, 78.2], [70.0, 71.0, 70.5, 70.8]), false);
        assert_eq!(r.desfecho, Desfecho::Piorou);
        assert!(!r.ficou_com_otimiza);
    }

    #[test]
    fn media_igual_com_1pct_pior_desfaz() {
        let mut rodadas = todas([70.0, 71.0, 70.5, 70.8], [70.2, 70.8, 70.6, 70.7]);
        for r in rodadas.iter_mut().filter(|r| r.lado == Lado::ComOtimiza) {
            r.low_1pct = Some(30.0 + r.fps_do_jogo / 100.0);
        }
        let r = decidir("FiveM.exe", 0, 60, rodadas, false);
        assert_eq!(r.desfecho, Desfecho::Piorou, "{:?}", r.diferenca_low);
    }

    #[test]
    fn ruido_nao_vira_ganho_nem_piora() {
        let r = decidir("FiveM.exe", 0, 60, todas([70.0, 76.0, 72.0, 74.0], [73.0, 69.0, 75.0, 71.0]), false);
        assert_eq!(r.desfecho, Desfecho::Indistinguivel);
        assert!(r.ficou_com_otimiza, "não piorou: o plano fica");
    }

    #[test]
    fn configuracao_que_mudou_recusa_a_comparacao() {
        let mut rodadas = todas([70.0, 71.0, 70.5, 70.8], [90.0, 91.0, 90.5, 90.8]);
        rodadas[7].configuracao_do_jogo = Some("outra".into());
        let r = decidir("FiveM.exe", 0, 60, rodadas, false);
        assert_eq!(r.desfecho, Desfecho::SemComparacaoJusta);
        assert!(r.ficou_com_otimiza);
    }

    #[test]
    fn gerador_que_ligou_recusa_a_comparacao() {
        let mut rodadas = todas([70.0, 71.0, 70.5, 70.8], [140.0, 141.0, 140.5, 140.8]);
        rodadas[2].geracao = Some(GeracaoNaPartida::LosslessScaling);
        let r = decidir("FiveM.exe", 0, 60, rodadas, false);
        assert_eq!(r.desfecho, Desfecho::SemComparacaoJusta);
    }

    #[test]
    fn configuracao_ilegivel_nao_e_mudanca() {
        let mut rodadas = todas([70.0, 71.0, 70.5, 70.8], [78.0, 79.0, 78.5, 78.2]);
        rodadas[1].configuracao_do_jogo = None;
        let r = decidir("FiveM.exe", 0, 60, rodadas, false);
        assert_eq!(r.desfecho, Desfecho::Ganhou);
    }

    #[test]
    fn medidores_misturados_recusam_a_comparacao() {
        let mut rodadas = todas([70.0, 71.0, 70.5, 70.8], [78.0, 79.0, 78.5, 78.2]);
        rodadas[3].pelo_presentmon = false;
        let r = decidir("FiveM.exe", 0, 60, rodadas, false);
        assert_eq!(r.desfecho, Desfecho::SemComparacaoJusta);
    }

    #[test]
    fn piora_que_nao_desfez_nao_diz_que_desfez() {
        let mut r = decidir("FiveM.exe", 0, 60, todas([78.0, 79.0, 78.5, 78.2], [70.0, 71.0, 70.5, 70.8]), false);
        marcar_falha_ao_desfazer(&mut r, "acesso negado".into());
        assert_eq!(r.desfecho, Desfecho::Piorou);
        assert!(r.ficou_com_otimiza);
        assert!(!r.leitura.contains("foi desfeito"));
        assert!(r.leitura.contains("acesso negado"));
    }

    #[test]
    fn rodada_faltando_nao_conclui() {
        let mut rodadas = todas([70.0, 71.0, 70.5, 70.8], [78.0, 79.0, 78.5, 78.2]);
        rodadas.truncate(5); // dois do lado sem: abaixo do mínimo
        let r = decidir("FiveM.exe", 0, 60, rodadas, false);
        assert_eq!(r.desfecho, Desfecho::SemComparacaoJusta);
    }

    #[test]
    fn sem_1pct_numa_rodada_o_1pct_fica_de_fora() {
        let mut rodadas = todas([70.0, 71.0, 70.5, 70.8], [78.0, 79.0, 78.5, 78.2]);
        rodadas[0].low_1pct = None;
        let r = decidir("FiveM.exe", 0, 60, rodadas, false);
        assert!(r.low_sem.is_none());
        assert!(r.diferenca_low.is_none());
        assert_eq!(r.desfecho, Desfecho::Ganhou);
    }

    #[test]
    fn a_duracao_estimada_inclui_o_aquecimento() {
        assert_eq!(duracao_estimada_s(45), 60 + 8 * 48);
    }
}
