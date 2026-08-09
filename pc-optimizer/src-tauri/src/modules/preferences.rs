// Preferências do usuário
//
// Cada preferência aqui muda comportamento real do programa. Interruptor que não
// altera nada é enfeite, e enfeite numa ferramenta de sistema é o começo da
// desconfiança: se um botão mente, por que os números não mentiriam?

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Intervalos aceitos para a leitura das métricas, em segundos.
/// Fora dessa lista o valor é ignorado — um intervalo de 0 ocuparia a CPU que o
/// programa deveria estar liberando.
const INTERVALOS_VALIDOS: [u32; 3] = [1, 2, 5];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    /// Tentar criar ponto de restauração antes do lote "Otimizar agora".
    ///
    /// Ligado por padrão. Criar o ponto leva dezenas de segundos, então quem
    /// otimiza várias máquinas por dia costuma querer desligar.
    pub restore_point_before_batch: bool,

    /// De quantos em quantos segundos as métricas e os processos são lidos.
    /// Ler mais rápido custa CPU do próprio programa — num PC fraco isso conta.
    pub metrics_interval_seconds: u32,

    /// Ligar o plano de alto desempenho sozinho quando um jogo abre, e
    /// desligar quando ele fecha.
    ///
    /// DESLIGADO por padrão, e é importante que continue assim. Um programa que
    /// muda configuração do sistema por conta própria, sem a pessoa pedir, é
    /// exatamente o que este produto critica nos outros — mesmo quando a
    /// mudança é boa. Quem quiser, liga aqui sabendo o que vai acontecer.
    #[serde(default)]
    pub auto_game_mode: bool,

    /// Mostrar na lista o que não se aplica a esta máquina.
    ///
    /// Ligado por padrão: saber que o programa se recusou a oferecer algo, e por
    /// quê, é parte do valor. Quem já entendeu pode desligar para reduzir ruído.
    pub show_unavailable: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Preferences {
            restore_point_before_batch: true,
            metrics_interval_seconds: 2,
            // Desligado: mexer no sistema sem a pessoa pedir precisa ser escolha dela.
            auto_game_mode: false,
            show_unavailable: true,
        }
    }
}

impl Preferences {
    /// Corrige valores fora da faixa em vez de confiar no arquivo.
    /// O JSON pode ter sido editado à mão, e um intervalo inválido travaria a
    /// interface num laço de leitura.
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

    /// Lê as preferências. Arquivo ausente ou corrompido devolve o padrão —
    /// nunca erro, para não impedir o programa de abrir.
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

        // O ponto de restauração vem ligado: quem não sabe que existe é
        // exatamente quem mais precisa dele.
        assert!(padrao.restore_point_before_batch);
        // E o que não se aplica aparece, porque a recusa explicada é o produto.
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
        // `serde(default)` garante que uma preferência nova, ainda ausente no
        // arquivo de quem já usa o programa, não derrube a leitura inteira.
        let parcial: Preferences = serde_json::from_str(r#"{"show_unavailable": false}"#).unwrap();

        assert!(!parcial.show_unavailable);
        assert!(parcial.restore_point_before_batch);
        assert_eq!(parcial.metrics_interval_seconds, 2);
    }

    #[test]
    fn json_corrompido_nao_derruba_o_programa() {
        assert!(serde_json::from_str::<Preferences>("{isso nao e json}").is_err());
        // `load` engole o erro e devolve o padrão; o teste acima garante que o
        // erro existe para ser engolido.
    }
}
