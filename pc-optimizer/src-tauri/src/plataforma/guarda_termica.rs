// GUARDA TÉRMICA (Plano 1.1, Onda 3b). Com um jogo aberto, olha a cada 15 s se o processador ou a placa estão sendo
// FREADOS por temperatura, e o Oti avisa antes de o jogo começar a travar — a queda de FPS por calor vem aos poucos e a
// pessoa culpa o jogo, a internet ou o Otimiza.
//
// Só lê, sem administrador: o processador pelo contador do Windows `Performance Limit Flags` (bit 0 = limite térmico,
// o mesmo da calibração de energia); a placa NVIDIA pela NVML (temperatura e motivo do clock segurado). Placa de outra
// marca e PC sem o contador ficam sem guarda, em silêncio: aviso sem leitura seria inventado.
//
// Não avisa por um pico: precisa de 2 das últimas 4 leituras. E não repete o mesmo aviso antes de 10 minutos.

use serde::Serialize;

/// Leituras olhadas de cada vez (4 × 15 s = 1 minuto).
pub const JANELA: usize = 4;
pub const SEGUNDOS_ENTRE_LEITURAS: u64 = 15;
const SEGUNDOS_ENTRE_AVISOS_IGUAIS: u64 = 10 * 60;
/// Placa acima disso ainda não está freando, mas está perto (a maioria freia entre 83 e 87 °C).
const PLACA_QUENTE_C: u32 = 84;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Leitura {
    pub cpu_freio_termico: Option<bool>,
    pub placa_c: Option<u32>,
    pub placa_freio_termico: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum TipoDeAviso {
    ProcessadorFreado,
    PlacaFreada,
    PlacaQuente,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct AvisoTermico {
    pub tipo: TipoDeAviso,
    pub frase: String,
    /// `perigo` (já está freando) ou `atencao` (perto).
    pub tom: String,
    pub jogo: Option<String>,
}

fn quantas(janela: &[Leitura], f: impl Fn(&Leitura) -> bool) -> usize {
    janela.iter().filter(|l| f(l)).count()
}

/// **Pura.** O aviso das últimas leituras (a mais grave primeiro), ou nenhum.
pub fn avaliar(janela: &[Leitura], jogo: Option<&str>) -> Option<AvisoTermico> {
    let ultimas = &janela[janela.len().saturating_sub(JANELA)..];
    let no_jogo = jogo.map(|j| format!(" no {j}")).unwrap_or_default();
    let aviso = |tipo, frase: String, tom: &str| AvisoTermico {
        tipo,
        frase,
        tom: tom.to_string(),
        jogo: jogo.map(String::from),
    };

    if quantas(ultimas, |l| l.cpu_freio_termico == Some(true)) >= 2 {
        return Some(aviso(
            TipoDeAviso::ProcessadorFreado,
            format!(
                "O processador está sendo freado por temperatura{no_jogo}. O FPS vai cair aos poucos: confira se a \
                 saída de ar está livre e, no notebook, use uma base ou levante a parte de trás."
            ),
            "perigo",
        ));
    }
    if quantas(ultimas, |l| l.placa_freio_termico == Some(true)) >= 2 {
        let c = ultimas.iter().filter_map(|l| l.placa_c).max();
        return Some(aviso(
            TipoDeAviso::PlacaFreada,
            format!(
                "A placa de vídeo está sendo freada por temperatura{no_jogo}{}. Limpe a poeira das ventoinhas ou \
                 limite o FPS um pouco acima do seu monitor.",
                c.map(|c| format!(" ({c} °C)")).unwrap_or_default()
            ),
            "perigo",
        ));
    }
    let quentes: Vec<u32> = ultimas
        .iter()
        .filter_map(|l| l.placa_c)
        .filter(|c| *c >= PLACA_QUENTE_C)
        .collect();
    if quentes.len() >= 2 {
        return Some(aviso(
            TipoDeAviso::PlacaQuente,
            format!(
                "A placa de vídeo chegou a {} °C{no_jogo}, perto de onde ela começa a se frear. Fico de olho.",
                quentes.iter().max().copied().unwrap_or(PLACA_QUENTE_C)
            ),
            "atencao",
        ));
    }
    None
}

/// **Pura.** Avisa de novo só depois do intervalo, ou quando o aviso fica mais grave.
pub fn deve_avisar(novo: TipoDeAviso, ultimo: Option<(TipoDeAviso, u64)>, agora_s: u64) -> bool {
    match ultimo {
        None => true,
        Some((tipo, quando)) => {
            let mais_grave = tipo == TipoDeAviso::PlacaQuente && novo != TipoDeAviso::PlacaQuente;
            mais_grave || tipo != novo || agora_s.saturating_sub(quando) >= SEGUNDOS_ENTRE_AVISOS_IGUAIS
        }
    }
}

/// Liga a vigia em segundo plano: dorme enquanto não há jogo, e com jogo lê a cada 15 s. Avisos pelo evento
/// `oti:aviso-termico`.
#[cfg(target_os = "windows")]
/// Plano de refino, E5: ajuste que faz a máquina gastar mais (plano de energia, placa no clock máximo) liga a guarda
/// também SEM jogo aberto por este tempo. Ganhar 3% e perder 20% quando esquenta é travada disfarçada.
pub const VIGIA_DEPOIS_DE_AJUSTE_S: u64 = 30 * 60;
/// Os ajustes do catálogo que aumentam o consumo e o calor.
pub const AJUSTES_QUE_ESQUENTAM: &[&str] = &["plano_otimiza", "placa_no_clock_maximo"];

static VIGIAR_ATE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
/// O nome do ajuste que pediu a vigília, para o aviso dizer de onde veio o calor.
static AJUSTE_VIGIADO: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

fn agora_s() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Chamado pelo motor do catálogo depois de aplicar `id`: se ele esquenta, a guarda vigia os próximos 30 minutos.
pub fn depois_de_aplicar(id: &str, nome: &str) {
    if !AJUSTES_QUE_ESQUENTAM.contains(&id) {
        return;
    }
    VIGIAR_ATE.store(agora_s() + VIGIA_DEPOIS_DE_AJUSTE_S, std::sync::atomic::Ordering::SeqCst);
    *AJUSTE_VIGIADO.lock().unwrap_or_else(|e| e.into_inner()) = Some(nome.to_string());
}

/// **Pura.** A guarda olha agora? Com jogo aberto sempre; sem jogo, só dentro da vigília depois de um ajuste.
pub fn deve_vigiar(jogo_aberto: bool, agora: u64, vigiar_ate: u64) -> bool {
    jogo_aberto || agora < vigiar_ate
}

pub fn vigiar(app: tauri::AppHandle) {
    use tauri::Emitter;
    std::thread::Builder::new()
        .name("guarda-termica".into())
        .spawn(move || {
            let mut janela: Vec<Leitura> = Vec::new();
            let mut ultimo: Option<(TipoDeAviso, u64)> = None;
            loop {
                std::thread::sleep(std::time::Duration::from_secs(SEGUNDOS_ENTRE_LEITURAS));
                let jogo = super::sessao::atual();
                let ate = VIGIAR_ATE.load(std::sync::atomic::Ordering::SeqCst);
                if !deve_vigiar(jogo.is_some(), agora_s(), ate) {
                    janela.clear();
                    continue;
                }
                let cpu = crate::modules::windows::motorenergia_maquina::amostra_agora();
                let placa = crate::modules::windows::nvml::amostrar_com_motivos(true);
                janela.push(Leitura {
                    cpu_freio_termico: cpu.and_then(|a| a.flags).map(|f| f & 1 == 1),
                    placa_c: placa.and_then(|p| p.temperatura_c),
                    placa_freio_termico: placa.and_then(|p| p.motivos).map(|m| {
                        m & (crate::core::sensores::MOTIVO_TERMICO_SW | crate::core::sensores::MOTIVO_TERMICO_HW) != 0
                    }),
                });
                if janela.len() > JANELA {
                    janela.remove(0);
                }
                let nome = jogo.as_ref().map(|j| j.nome.clone());
                if let Some(mut aviso) = avaliar(&janela, nome.as_deref()) {
                    // Sem jogo, o calor veio na vigília de um ajuste: o aviso diz qual, para a pessoa poder desfazer.
                    if jogo.is_none() {
                        if let Some(ajuste) = AJUSTE_VIGIADO.lock().unwrap_or_else(|e| e.into_inner()).clone() {
                            aviso.frase = format!(
                                "{} Começou depois do ajuste \"{}\": se continuar, desfaça ele no Histórico.",
                                aviso.frase, ajuste
                            );
                        }
                    }
                    let agora = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    if deve_avisar(aviso.tipo, ultimo, agora) {
                        ultimo = Some((aviso.tipo, agora));
                        let _ = app.emit("oti:aviso-termico", aviso);
                    }
                }
            }
        })
        .ok();
}

#[cfg(test)]
mod testes {
    use super::*;

    fn l(cpu: Option<bool>, c: Option<u32>, placa: Option<bool>) -> Leitura {
        Leitura { cpu_freio_termico: cpu, placa_c: c, placa_freio_termico: placa }
    }

    #[test]
    fn sem_jogo_so_vigia_na_meia_hora_depois_do_ajuste() {
        assert!(deve_vigiar(true, 1000, 0));
        assert!(deve_vigiar(false, 1000, 1000 + VIGIA_DEPOIS_DE_AJUSTE_S));
        assert!(!deve_vigiar(false, 1000 + VIGIA_DEPOIS_DE_AJUSTE_S, 1000 + VIGIA_DEPOIS_DE_AJUSTE_S));
        assert!(!deve_vigiar(false, 1000, 0));
    }

    #[test]
    fn os_ajustes_que_esquentam_existem_no_catalogo() {
        for id in AJUSTES_QUE_ESQUENTAM {
            assert!(crate::modules::windows::catalog::find(id).is_some(), "{id} não está no catálogo");
        }
    }

    #[test]
    fn um_pico_so_nao_avisa() {
        let j = [l(Some(true), Some(70), Some(false)), l(Some(false), Some(70), Some(false))];
        assert_eq!(avaliar(&j, None), None);
    }

    #[test]
    fn processador_freado_duas_vezes_avisa_como_perigo_e_cita_o_jogo() {
        let j = [l(Some(true), None, None), l(Some(false), None, None), l(Some(true), None, None)];
        let a = avaliar(&j, Some("FiveM")).unwrap();
        assert_eq!(a.tipo, TipoDeAviso::ProcessadorFreado);
        assert_eq!(a.tom, "perigo");
        assert!(a.frase.contains("no FiveM"));
    }

    #[test]
    fn placa_freada_diz_a_temperatura() {
        let j = [l(None, Some(86), Some(true)), l(None, Some(88), Some(true))];
        let a = avaliar(&j, None).unwrap();
        assert_eq!(a.tipo, TipoDeAviso::PlacaFreada);
        assert!(a.frase.contains("88 °C"), "{}", a.frase);
    }

    #[test]
    fn placa_quente_sem_freio_e_so_atencao() {
        let j = [l(None, Some(85), Some(false)), l(None, Some(84), Some(false)), l(None, Some(70), Some(false))];
        let a = avaliar(&j, None).unwrap();
        assert_eq!((a.tipo, a.tom.as_str()), (TipoDeAviso::PlacaQuente, "atencao"));
    }

    #[test]
    fn sem_leitura_nao_inventa_aviso() {
        let j = [Leitura::default(); 4];
        assert_eq!(avaliar(&j, Some("CS2")), None);
    }

    #[test]
    fn so_olha_as_ultimas_quatro() {
        let mut j = vec![l(Some(true), None, None); 3];
        j.extend([l(Some(false), None, None); 4]);
        assert_eq!(avaliar(&j, None), None);
    }

    #[test]
    fn nao_repete_o_mesmo_aviso_antes_de_dez_minutos_mas_avisa_se_piorar() {
        let ultimo = Some((TipoDeAviso::PlacaQuente, 1000));
        assert!(!deve_avisar(TipoDeAviso::PlacaQuente, ultimo, 1300));
        assert!(deve_avisar(TipoDeAviso::PlacaQuente, ultimo, 1000 + 600));
        assert!(deve_avisar(TipoDeAviso::PlacaFreada, ultimo, 1001));
        assert!(deve_avisar(TipoDeAviso::ProcessadorFreado, None, 0));
    }
}
