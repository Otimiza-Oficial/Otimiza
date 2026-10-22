// TURBO (3.5). Um botão: mede a máquina e o jogo, escolhe a receita pelo GARGALO, aplica e lembra por jogo. As peças já
// existiam soltas (Frame Boost, escala, energia de jogo, memória para o jogo); o Turbo só decide qual delas serve:
//
//   placa no limite      → escala (FPS real a mais) e, se ainda faltar para o monitor, geração
//   processador no limite → geração e energia de jogo (a escala não ajuda: a placa já sobra)
//   memória acabando     → a memória primeiro (oferecer fechar; nunca fecha sozinho) e a energia
//   nada no limite       → não liga nada e diz que o jogo já está no limite desta máquina
//
// Jogo competitivo não ganha geração por padrão: ela acrescenta atraso. O nível da escala sai do FPS medido, como
// ESTIMATIVA até a medida do depois.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::modules::windows::bottleneck::Limite;
use crate::modules::windows::geracao::escala::Nivel;

/// Onde o jogo trava, nas palavras do Turbo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Gargalo {
    Placa,
    Processador,
    Memoria,
    /// Nada no limite: o jogo já roda o que esta máquina consegue.
    Nenhum,
    /// O jogo estava parado (menu, pausa) durante a medida.
    SemCarga,
}

/// Pura. VRAM cheia conta como placa (menos pixels também é menos memória de vídeo); disco e "não identificado" não
/// têm receita no Turbo.
pub fn gargalo_de(l: Limite) -> Gargalo {
    match l {
        Limite::Gpu | Limite::MemoriaVideo => Gargalo::Placa,
        Limite::CpuUmNucleo | Limite::CpuTodos => Gargalo::Processador,
        Limite::MemoriaRam => Gargalo::Memoria,
        Limite::SemCarga => Gargalo::SemCarga,
        Limite::Disco | Limite::NaoIdentificado => Gargalo::Nenhum,
    }
}

/// Jogos de tiro e de reflexo: o atraso da geração pesa mais que a fluidez.
const COMPETITIVOS: &[&str] = &["cs2", "valorant", "fortnite", "cod", "apex", "rocketleague"];

pub fn competitivo(id_do_jogo: &str) -> bool {
    COMPETITIVOS.contains(&id_do_jogo)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Receita {
    pub gargalo: Gargalo,
    /// O Frame Boost: escala (nível) e multiplicador (1 = só escala, 0 = Auto, 2+ = fixo). `None` = não liga.
    pub escala: Option<Nivel>,
    pub multiplicador: Option<u8>,
    pub energia: bool,
    /// Oferecer fechar o que ocupa memória (a tela lista; o Turbo nunca fecha sozinho).
    pub memoria: bool,
    /// O que o Turbo vai fazer e por quê, para a tela.
    pub explicacao: String,
}

/// Pura: o FPS do jogo renderizando no nível, preso na placa. O FPS cresce quase na proporção dos pixels, mas não
/// inteiro (geometria, processador e pós-processamento não diminuem): expoente 0,7, para não prometer demais.
pub fn fps_no_nivel(fps: f64, nivel: Nivel) -> f64 {
    fps * nivel.fator().powi(2).powf(0.7)
}

/// Pura: o nível mais bonito que alcança `alvo` (FPS do jogo). Sem FPS medido, Qualidade; nenhum alcança, Desempenho.
pub fn nivel_automatico(fps: Option<f64>, alvo: f64) -> Nivel {
    let Some(fps) = fps.filter(|f| *f > 0.0) else {
        return Nivel::Qualidade;
    };
    Nivel::TODOS
        .iter()
        .copied()
        .find(|n| fps_no_nivel(fps, *n) >= alvo)
        .unwrap_or(Nivel::Desempenho)
}

/// Pura: a receita para o gargalo medido. `fps` é o do jogo na medida (sem ele, o nível padrão); `hz` o do monitor.
pub fn montar(gargalo: Gargalo, fps: Option<f64>, hz: Option<u32>, competitivo: bool) -> Receita {
    let hz = hz.filter(|h| *h > 0).unwrap_or(60) as f64;
    let (escala, multiplicador, energia, memoria, explicacao) = match gargalo {
        Gargalo::Placa => {
            // Só escala tem de encher o monitor sozinha; com geração, metade dele já vira o monitor cheio no 2×.
            let so_escala = nivel_automatico(fps, hz);
            let precisa_gerar = !competitivo && fps.is_some_and(|f| fps_no_nivel(f, so_escala) < hz * 0.9);
            if precisa_gerar {
                let nivel = nivel_automatico(fps, hz / 2.0);
                (
                    Some(nivel),
                    Some(0),
                    true,
                    false,
                    "A placa de vídeo está no limite: o jogo passa a renderizar menos pixels (FPS real a mais) e a \
                     geração de quadros completa o monitor."
                        .to_string(),
                )
            } else {
                (
                    Some(so_escala),
                    Some(1),
                    true,
                    false,
                    if competitivo {
                        "A placa de vídeo está no limite: o jogo passa a renderizar menos pixels. Jogo competitivo: \
                         sem geração de quadros, para não acrescentar atraso."
                    } else {
                        "A placa de vídeo está no limite: o jogo passa a renderizar menos pixels, e isso já basta \
                         para o monitor. Sem geração, sem atraso a mais."
                    }
                    .to_string(),
                )
            }
        }
        Gargalo::Processador => (
            None,
            (!competitivo).then_some(0),
            true,
            false,
            if competitivo {
                "O processador está no limite: diminuir a resolução não ajudaria (a placa já sobra). O processador \
                 passa a subir de frequência sem hesitar com o jogo aberto. Jogo competitivo: sem geração."
            } else {
                "O processador está no limite: diminuir a resolução não ajudaria (a placa já sobra). A geração de \
                 quadros enche o monitor e o processador passa a subir de frequência sem hesitar."
            }
            .to_string(),
        ),
        Gargalo::Memoria => (
            None,
            None,
            true,
            true,
            "A memória está acabando: é ela que trava o jogo ao carregar cenário. Feche o que não precisa (a lista \
             abaixo) e meça de novo."
                .to_string(),
        ),
        Gargalo::Nenhum => (
            None,
            None,
            false,
            false,
            "Nada está no limite: o jogo já roda o que esta máquina consegue. O Turbo não liga nada à toa.".to_string(),
        ),
        Gargalo::SemCarga => (
            None,
            None,
            false,
            false,
            "O jogo estava parado durante a medida (menu ou pausa). Jogue e toque em Turbo de novo.".to_string(),
        ),
    };
    Receita {
        gargalo,
        escala,
        multiplicador,
        energia,
        memoria,
        explicacao,
    }
}

/// A receita que deu certo num jogo, nesta máquina: na próxima vez, um clique e sem medir de novo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Perfil {
    pub receita: Receita,
    /// O FPS do jogo na medida (o "antes").
    pub fps_antes: Option<f64>,
    pub medido_em: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
struct Perfis {
    /// Pelo id do jogo (`historico::id_do_jogo`).
    jogos: BTreeMap<String, Perfil>,
}

fn caminho() -> PathBuf {
    std::env::var("APPDATA")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("pc-optimizer")
        .join("turbo.json")
}

fn ler_em(c: &Path) -> Perfis {
    fs::read_to_string(c)
        .ok()
        .and_then(|b| serde_json::from_str(&b).ok())
        .unwrap_or_default()
}

fn gravar_em(c: &Path, p: &Perfis) -> Result<(), String> {
    if let Some(d) = c.parent() {
        fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    let texto = serde_json::to_string_pretty(p).map_err(|e| e.to_string())?;
    fs::write(c, texto).map_err(|e| e.to_string())
}

pub fn perfil(id_do_jogo: &str) -> Option<Perfil> {
    ler_em(&caminho()).jogos.get(id_do_jogo).cloned()
}

pub fn lembrar(id_do_jogo: &str, perfil: Perfil) -> Result<(), String> {
    let c = caminho();
    let mut p = ler_em(&c);
    p.jogos.insert(id_do_jogo.to_string(), perfil);
    gravar_em(&c, &p)
}

pub fn esquecer(id_do_jogo: &str) -> Result<(), String> {
    let c = caminho();
    let mut p = ler_em(&c);
    if p.jogos.remove(id_do_jogo).is_some() {
        gravar_em(&c, &p)?;
    }
    Ok(())
}

/// Para a tela.
#[derive(Debug, Clone, Serialize)]
pub struct Painel {
    pub jogo: Option<String>,
    pub competitivo: bool,
    /// O que ficou lembrado para este jogo.
    pub perfil: Option<Perfil>,
    /// O que o Turbo fez da última vez que ligou (nesta abertura do Otimiza).
    pub passos: Vec<String>,
    pub ligado: bool,
}

#[cfg(target_os = "windows")]
static PASSOS: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
/// O Turbo ligou a energia de jogo (e não a pessoa): desligar o Turbo devolve só o que ele ligou.
#[cfg(target_os = "windows")]
static LIGOU_A_ENERGIA: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
#[cfg(target_os = "windows")]
static LIGADO: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(target_os = "windows")]
pub fn painel() -> Painel {
    use std::sync::atomic::Ordering;
    let jogo = super::sessao::atual();
    let id = jogo
        .as_ref()
        .map(|j| super::historico::id_do_jogo(&j.executavel).0);
    Painel {
        jogo: jogo.as_ref().map(|j| j.nome.clone()),
        competitivo: id.as_deref().is_some_and(competitivo),
        perfil: id.as_deref().and_then(perfil),
        passos: PASSOS.lock().map(|p| p.clone()).unwrap_or_default(),
        ligado: LIGADO.load(Ordering::SeqCst),
    }
}

/// Mede 10 s (gargalo pelos contadores do Windows e FPS pelo PresentMon, ao mesmo tempo) e monta a receita. Bloqueia.
#[cfg(target_os = "windows")]
pub fn medir() -> Result<(Receita, Option<f64>), String> {
    use crate::modules::windows::{bottleneck, presentmon};
    let jogo = super::sessao::atual().ok_or("Abra o jogo primeiro: o Turbo mede o jogo aberto.")?;
    let pid = jogo.pid;
    let fps = std::thread::spawn(move || {
        presentmon::capturar(pid, 10)
            .and_then(|q| presentmon::resumir(&q))
            .ok()
            .map(|r| r.fps_do_jogo_medio)
            .filter(|f| *f > 0.0)
    });
    let relatorio = bottleneck::analisar(10);
    let fps = fps.join().ok().flatten();
    let (id, _) = super::historico::id_do_jogo(&jogo.executavel);
    let hz = super::frameboost::hz_do_jogo(pid);
    crate::utils::Logger::info(&format!(
        "turbo: {} · {:?} (CPU {:.0}%, núcleo {:.0}%, placa {:.0}%) · {:?} fps · {:?} Hz",
        id, relatorio.limite, relatorio.cpu_total, relatorio.cpu_max_core, relatorio.gpu_percent, fps, hz
    ));
    Ok((
        montar(gargalo_de(relatorio.limite), fps, hz, competitivo(&id)),
        fps,
    ))
}

/// Aplica a receita e devolve o que fez, passo a passo. Um passo que falha (anticheat, sem administrador) vira nota e
/// os outros seguem: o Turbo nunca deixa de fazer o que pode por causa do que não pode.
#[cfg(target_os = "windows")]
pub fn aplicar(app: &tauri::AppHandle, r: &Receita) -> Vec<String> {
    use std::sync::atomic::Ordering;
    let mut passos = vec![r.explicacao.clone()];
    if r.energia {
        let ja = super::energia_jogo::ler().ligado;
        if ja {
            passos.push("Energia de jogo: já estava ligada.".to_string());
        } else {
            match super::energia_jogo::definir(true) {
                Ok(_) => {
                    LIGOU_A_ENERGIA.store(true, Ordering::SeqCst);
                    passos.push("Energia de jogo: ligada.".to_string());
                }
                Err(e) => passos.push(format!("Energia de jogo: {}", e)),
            }
        }
    }
    if let Some(m) = r.multiplicador {
        match super::frameboost::ligar(app, m, false, r.escala) {
            Ok(_) => passos.push(match (r.escala, m) {
                (Some(n), 1) => format!("Escala {:?}: ligada, sem geração.", n),
                (Some(n), _) => format!("Escala {:?} e geração de quadros: ligadas.", n),
                (None, _) => "Geração de quadros: ligada.".to_string(),
            }),
            Err(e) => passos.push(format!("Frame Boost: {}", e)),
        }
    }
    if r.memoria {
        passos.push("Memória: veja abaixo o que ocupa e feche o que não precisa.".to_string());
    }
    LIGADO.store(true, Ordering::SeqCst);
    if let Ok(mut p) = PASSOS.lock() {
        *p = passos.clone();
    }
    passos
}

/// Desfaz o que o Turbo ligou: o Frame Boost e, se foi ele que ligou, a energia de jogo.
#[cfg(target_os = "windows")]
pub fn desligar() -> Result<(), String> {
    use std::sync::atomic::Ordering;
    super::frameboost::desligar()?;
    if LIGOU_A_ENERGIA.swap(false, Ordering::SeqCst) {
        super::energia_jogo::definir(false)?;
    }
    LIGADO.store(false, Ordering::SeqCst);
    if let Ok(mut p) = PASSOS.lock() {
        p.clear();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cada_limite_tem_o_seu_gargalo() {
        assert_eq!(gargalo_de(Limite::Gpu), Gargalo::Placa);
        assert_eq!(gargalo_de(Limite::MemoriaVideo), Gargalo::Placa);
        assert_eq!(gargalo_de(Limite::CpuUmNucleo), Gargalo::Processador);
        assert_eq!(gargalo_de(Limite::CpuTodos), Gargalo::Processador);
        assert_eq!(gargalo_de(Limite::MemoriaRam), Gargalo::Memoria);
        assert_eq!(gargalo_de(Limite::SemCarga), Gargalo::SemCarga);
        assert_eq!(gargalo_de(Limite::NaoIdentificado), Gargalo::Nenhum);
    }

    #[test]
    fn o_nivel_e_o_mais_bonito_que_alcanca() {
        // 60 fps num monitor de 144: a Ultra (1,3×) dá ~87, a Qualidade (1,5×) ~106, o Equilibrado ~126 e o
        // Desempenho ~158. Só o Desempenho chega a 144.
        assert_eq!(nivel_automatico(Some(60.0), 144.0), Nivel::Desempenho);
        assert_eq!(nivel_automatico(Some(60.0), 72.0), Nivel::UltraQualidade);
        assert_eq!(nivel_automatico(Some(60.0), 100.0), Nivel::Qualidade);
        assert_eq!(nivel_automatico(None, 144.0), Nivel::Qualidade, "sem medida, o padrão");
    }

    #[test]
    fn o_ganho_estimado_e_menor_que_o_dos_pixels() {
        let f = fps_no_nivel(100.0, Nivel::Qualidade);
        assert!(f > 150.0 && f < 225.0, "{}", f);
    }

    #[test]
    fn placa_no_limite_ganha_escala_e_geracao_se_faltar() {
        let r = montar(Gargalo::Placa, Some(40.0), Some(144), false);
        assert!(r.escala.is_some());
        assert_eq!(r.multiplicador, Some(0), "40 fps não enche 144 Hz só com escala: Auto");
        assert!(r.energia);
    }

    #[test]
    fn placa_no_limite_perto_do_monitor_fica_so_com_escala() {
        let r = montar(Gargalo::Placa, Some(110.0), Some(144), false);
        assert_eq!(r.multiplicador, Some(1), "a escala já leva 110 a 144: sem geração");
        assert!(r.escala.is_some());
    }

    #[test]
    fn competitivo_nunca_ganha_geracao() {
        let r = montar(Gargalo::Placa, Some(40.0), Some(240), true);
        assert_eq!(r.multiplicador, Some(1));
        let r = montar(Gargalo::Processador, Some(40.0), Some(240), true);
        assert_eq!(r.multiplicador, None);
        assert!(r.energia);
    }

    #[test]
    fn processador_no_limite_nao_ganha_escala() {
        let r = montar(Gargalo::Processador, Some(45.0), Some(180), false);
        assert_eq!(r.escala, None, "a placa já sobra: menos pixels não dá FPS");
        assert_eq!(r.multiplicador, Some(0));
        assert!(r.energia);
    }

    #[test]
    fn memoria_acabando_oferece_a_memoria_e_nao_liga_o_frame_boost() {
        let r = montar(Gargalo::Memoria, Some(45.0), Some(60), false);
        assert!(r.memoria);
        assert_eq!(r.multiplicador, None);
    }

    #[test]
    fn nada_no_limite_nao_liga_nada() {
        for g in [Gargalo::Nenhum, Gargalo::SemCarga] {
            let r = montar(g, Some(200.0), Some(144), false);
            assert_eq!((r.escala, r.multiplicador, r.energia, r.memoria), (None, None, false, false));
            assert!(!r.explicacao.is_empty());
        }
    }

    #[test]
    fn os_competitivos_estao_no_catalogo() {
        for id in COMPETITIVOS {
            assert!(
                super::super::jogos::CATALOGO.iter().any(|j| j.id == *id),
                "{} não está no catálogo de jogos",
                id
            );
        }
        assert!(!competitivo("fivem"));
    }

    #[test]
    fn o_perfil_vai_e_volta_pelo_arquivo() {
        let pasta = std::env::temp_dir().join(format!("otimiza-turbo-{}", std::process::id()));
        let c = pasta.join("turbo.json");
        let mut p = Perfis::default();
        p.jogos.insert(
            "cs2".to_string(),
            Perfil {
                receita: montar(Gargalo::Placa, Some(90.0), Some(144), true),
                fps_antes: Some(90.0),
                medido_em: 1,
            },
        );
        gravar_em(&c, &p).unwrap();
        assert_eq!(ler_em(&c), p);
        let _ = fs::remove_dir_all(&pasta);
    }

    #[test]
    fn arquivo_ausente_ou_estragado_nao_tem_perfil() {
        let pasta = std::env::temp_dir().join(format!("otimiza-turbo-ruim-{}", std::process::id()));
        assert_eq!(ler_em(&pasta.join("nao-existe.json")), Perfis::default());
        fs::create_dir_all(&pasta).unwrap();
        fs::write(pasta.join("turbo.json"), "{ estragado").unwrap();
        assert_eq!(ler_em(&pasta.join("turbo.json")), Perfis::default());
        let _ = fs::remove_dir_all(&pasta);
    }
}
