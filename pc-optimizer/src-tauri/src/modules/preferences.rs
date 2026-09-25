use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Um intervalo de 0 ocuparia a CPU que o programa deveria liberar.
const INTERVALOS_VALIDOS: [u32; 3] = [1, 2, 5];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub restore_point_before_batch: bool,

    pub metrics_interval_seconds: u32,

    /// DESLIGADO por padrão: mudar o sistema sem a pessoa pedir é o que este produto critica nos outros.
    #[serde(default)]
    pub auto_game_mode: bool,

    pub show_unavailable: bool,

    /// LIGADO por padrão, ao contrário do modo jogo: medir só escuta os eventos do Windows e não muda nada.
    pub medir_quadros_sozinho: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Preferences {
            restore_point_before_batch: true,
            metrics_interval_seconds: 2,
            auto_game_mode: false,
            show_unavailable: true,
            medir_quadros_sozinho: true,
        }
    }
}

impl Preferences {
    /// O JSON pode ter sido editado à mão, e um intervalo inválido travaria a interface num laço de leitura.
    fn sanitize(mut self) -> Self {
        if !INTERVALOS_VALIDOS.contains(&self.metrics_interval_seconds) {
            self.metrics_interval_seconds = Preferences::default().metrics_interval_seconds;
        }
        self
    }

    fn path() -> PathBuf {
        let base = std::env::var("APPDATA")
            .or_else(|_| std::env::var("HOME"))
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));

        base.join("pc-optimizer").join("preferences.json")
    }

    /// Arquivo ausente ou corrompido devolve o padrão, nunca erro: não pode impedir o programa de abrir.
    pub fn load() -> Self {
        fs::read_to_string(Self::path())
            .ok()
            .and_then(|raw| serde_json::from_str::<Preferences>(&raw).ok())
            .unwrap_or_default()
            .sanitize()
    }

    pub fn save(&self) -> Result<(), String> {
        let preferencias = self.clone().sanitize();
        let path = Self::path();

        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|e| format!("Falha ao criar a pasta: {}", e))?;
        }

        let raw = serde_json::to_string_pretty(&preferencias)
            .map_err(|e| format!("Falha ao serializar preferências: {}", e))?;

        fs::write(&path, raw).map_err(|e| format!("Falha ao gravar preferências: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn padrao_protege_o_usuario() {
        let padrao = Preferences::default();

        assert!(padrao.restore_point_before_batch);
        assert!(padrao.show_unavailable);
    }

    #[test]
    fn intervalo_invalido_volta_ao_padrao() {
        let mut p = Preferences::default();

        p.metrics_interval_seconds = 0;
        assert_eq!(p.clone().sanitize().metrics_interval_seconds, 2);

        p.metrics_interval_seconds = 999;
        assert_eq!(p.clone().sanitize().metrics_interval_seconds, 2);
    }

    #[test]
    fn intervalos_da_lista_sao_preservados() {
        for intervalo in INTERVALOS_VALIDOS {
            let mut p = Preferences::default();
            p.metrics_interval_seconds = intervalo;
            assert_eq!(p.sanitize().metrics_interval_seconds, intervalo);
        }
    }

    #[test]
    fn json_incompleto_completa_com_o_padrao() {
        // `serde(default)`: uma preferência nova, ausente no arquivo de quem já usa, não derruba a leitura.
        let parcial: Preferences = serde_json::from_str(r#"{"show_unavailable": false}"#).unwrap();

        assert!(!parcial.show_unavailable);
        assert!(parcial.restore_point_before_batch);
        assert_eq!(parcial.metrics_interval_seconds, 2);
    }

    #[test]
    fn json_corrompido_nao_derruba_o_programa() {
        assert!(serde_json::from_str::<Preferences>("{isso nao e json}").is_err());
    }

    #[test]
    fn preferencias_gravadas_antes_da_2_0_continuam_carregando() {
        // `game_mode_avisado` saiu na 2.0, mas continua no arquivo de quem já usava: chave que sobrou não pode derrubar
        // a leitura.
        let antigo: Preferences =
            serde_json::from_str(r#"{"auto_game_mode": true, "game_mode_avisado": true}"#)
                .expect("preferências antigas precisam continuar legíveis");

        assert!(antigo.auto_game_mode);
    }

    #[test]
    fn quem_atualiza_da_1_9_passa_a_ter_a_medicao_sozinha_ligada() {
        let antigo: Preferences =
            serde_json::from_str(r#"{"auto_game_mode": true, "show_unavailable": false}"#)
                .expect("preferências da 1.9 continuam legíveis");

        assert!(antigo.medir_quadros_sozinho);
        assert!(antigo.auto_game_mode);
        assert!(!antigo.show_unavailable);
    }
}
