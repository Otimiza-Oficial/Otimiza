// Medições de quadros automáticas durante as partidas (quase ninguém lembra de medir). Com o jogo em primeiro
// plano há alguns minutos e o app como administrador, escuta vinte segundos pelo canal de `frames.rs`, no máximo
// uma vez a cada vinte minutos. NÃO compara medições entre si: foram feitas em lugares diferentes do jogo, e
// comparar menu com rua movimentada é fabricar prova.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MedicaoAutomatica {
    pub jogo: String,
    pub quando: u64,
    pub fps: f64,
    pub low_1pct: f64,
    pub engasgos_por_minuto: f64,
    pub segundos: f64,
    pub confiavel: bool,
    pub mudancas_aplicadas: usize,
    /// `None` em medição antiga.
    #[serde(default)]
    pub ambiente: Option<crate::modules::deriva::Ambiente>,

    // O RITMO, além do resumo: FPS maior com ritmo pior não é melhora. Todos com `serde(default)`: medição antiga
    // continua lendo, com os campos ausentes em vez de zerados.
    /// Diferente da mediana de propósito: o afastamento entre as duas é o sintoma.
    #[serde(default)]
    pub frametime_medio_ms: Option<f64>,
    #[serde(default)]
    pub frametime_p95_ms: Option<f64>,
    #[serde(default)]
    pub frametime_p99_ms: Option<f64>,
    /// Lido pela NVML na mesma janela: com o jogo fechado a placa já esfriou e não diz nada. `None` em medição antiga
    /// e sem placa NVIDIA.
    #[serde(default)]
    pub placa: Option<crate::core::sensores::ResumoGpu>,

    /// Só vale na mesma janela: uso de CPU depois do jogo fechar não diz nada da partida.
    #[serde(default)]
    pub cpu_uso_pct: Option<f64>,
    /// Vale pelo par: quadros baixos com os DOIS sobrando aponta para motor do jogo, teto ou espera.
    #[serde(default)]
    pub gpu_uso_pct: Option<f64>,

    // Shader e asset do disco dão o mesmo buraco no frametime. Guarda a proporção do cruzamento, não a série
    // (milhares de carimbos não cabem em sessenta medições).
    #[serde(default)]
    pub trancos_com_disco_pct: Option<f64>,
    /// Proporção sem denominador esconde que o total era dois.
    #[serde(default)]
    pub trancos_medidos: Option<usize>,

    /// Separa os dois lados da vigília do governador em `modules::portao`. `None`: não se aplica, mudou no meio ou antiga.
    #[serde(default)]
    pub governador: Option<crate::modules::portao::GovernadorNaPartida>,
}

pub const GUARDADAS: usize = 60;

/// Menos de vinte segundos e o 1% pior não significa nada.
pub const SEGUNDOS_DE_MEDICAO: u64 = 20;

/// Os primeiros minutos são carregamento e menu, a centenas de quadros.
pub const SEGUNDOS_ANTES_DA_PRIMEIRA: u64 = 180;

pub const SEGUNDOS_ENTRE_MEDICOES: u64 = 20 * 60;

/// Mais espaçado que o vigia do modo jogo: a detecção passa pelo PowerShell, e medir não pode pesar no jogo.
pub const SEGUNDOS_ENTRE_OLHADAS: u64 = 30;

fn caminho() -> PathBuf {
    let base = std::env::var("APPDATA")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));

    base.join("pc-optimizer").join("medicoes.json")
}

/// Arquivo inexistente é vazio; existente e ilegível é `Err`, nunca a lista vazia fingindo resposta.
pub fn ler_de(caminho: &Path) -> Result<Vec<MedicaoAutomatica>, String> {
    match fs::read_to_string(caminho) {
        Ok(bruto) => serde_json::from_str(&bruto)
            .map_err(|e| format!("o histórico de medições está ilegível: {}", e)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(format!("não consegui ler o histórico de medições: {}", e)),
    }
}

pub fn ler() -> Result<Vec<MedicaoAutomatica>, String> {
    ler_de(&caminho())
}

/// Histórico ilegível NÃO é sobrescrito: gravar por cima apagaria tudo o que já foi medido.
pub fn registrar_em(caminho: &Path, medicao: MedicaoAutomatica) -> Result<(), String> {
    let mut todas = ler_de(caminho)?;
    todas.push(medicao);

    let excesso = todas.len().saturating_sub(GUARDADAS);
    todas.drain(..excesso);

    if let Some(pasta) = caminho.parent() {
        fs::create_dir_all(pasta)
            .map_err(|e| format!("não consegui criar a pasta de dados: {}", e))?;
    }

    let bruto = serde_json::to_string(&todas)
        .map_err(|e| format!("não consegui preparar o histórico de medições: {}", e))?;

    fs::write(caminho, bruto)
        .map_err(|e| format!("não consegui gravar o histórico de medições: {}", e))
}

pub fn registrar(medicao: MedicaoAutomatica) -> Result<(), String> {
    registrar_em(&caminho(), medicao)
}

pub fn hora_de_medir(aberto_ha: u64, desde_a_ultima: Option<u64>) -> bool {
    match desde_a_ultima {
        None => aberto_ha >= SEGUNDOS_ANTES_DA_PRIMEIRA,
        Some(segundos) => segundos >= SEGUNDOS_ENTRE_MEDICOES,
    }
}

/// Relógio de fora, para cada transição ser provada em teste sem esperar.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Acompanhamento {
    pid: Option<u32>,
    desde: u64,
    ultima: Option<u64>,
}

impl Acompanhamento {
    /// Sem jogo, ou com OUTRO jogo, a contagem recomeça.
    pub fn observar(&mut self, pid: Option<u32>, agora: u64) -> bool {
        match pid {
            None => {
                *self = Acompanhamento::default();
                false
            }
            Some(pid) if self.pid != Some(pid) => {
                *self = Acompanhamento {
                    pid: Some(pid),
                    desde: agora,
                    ultima: None,
                };
                false
            }
            Some(_) => hora_de_medir(
                agora.saturating_sub(self.desde),
                self.ultima.map(|ultima| agora.saturating_sub(ultima)),
            ),
        }
    }

    /// Marca também a que falhou: recusa do anticheat tentada a cada meio minuto seria trabalho à toa na partida.
    pub fn tentado(&mut self, agora: u64) {
        self.ultima = Some(agora);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exemplo(fps: f64) -> MedicaoAutomatica {
        MedicaoAutomatica {
            jogo: "FiveM_b3258_GTAProcess.exe".to_string(),
            quando: 1_757_600_000,
            fps,
            low_1pct: fps / 2.0,
            engasgos_por_minuto: 3.0,
            segundos: 20.0,
            placa: None,
            confiavel: true,
            mudancas_aplicadas: 4,
            ambiente: None,
            frametime_medio_ms: Some(16.7),
            frametime_p95_ms: Some(22.0),
            frametime_p99_ms: Some(31.0),
            cpu_uso_pct: Some(48.0),
            gpu_uso_pct: Some(72.0),
            trancos_com_disco_pct: Some(20.0),
            trancos_medidos: Some(15),
            governador: None,
        }
    }

    fn pasta_de_teste(nome: &str) -> PathBuf {
        let unico = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);

        std::env::temp_dir().join(format!(
            "otimiza-medicoes-{}-{}-{}",
            nome,
            std::process::id(),
            unico
        ))
    }

    #[test]
    fn a_primeira_medicao_espera_o_carregamento() {
        assert!(!hora_de_medir(0, None));
        assert!(!hora_de_medir(SEGUNDOS_ANTES_DA_PRIMEIRA - 1, None));
        assert!(hora_de_medir(SEGUNDOS_ANTES_DA_PRIMEIRA, None));
    }

    #[test]
    fn depois_da_primeira_espera_o_intervalo() {
        assert!(!hora_de_medir(3600, Some(SEGUNDOS_ENTRE_MEDICOES - 1)));
        assert!(hora_de_medir(3600, Some(SEGUNDOS_ENTRE_MEDICOES)));
    }

    #[test]
    fn o_acompanhamento_mede_so_o_mesmo_jogo_aberto_sem_parar() {
        let mut vigia = Acompanhamento::default();

        assert!(!vigia.observar(Some(42), 1000));
        assert!(!vigia.observar(Some(42), 1000 + SEGUNDOS_ANTES_DA_PRIMEIRA - 30));

        assert!(vigia.observar(Some(42), 1000 + SEGUNDOS_ANTES_DA_PRIMEIRA));
        vigia.tentado(1000 + SEGUNDOS_ANTES_DA_PRIMEIRA);

        assert!(!vigia.observar(Some(42), 1000 + SEGUNDOS_ANTES_DA_PRIMEIRA + 60));

        let depois = 1000 + SEGUNDOS_ANTES_DA_PRIMEIRA + SEGUNDOS_ENTRE_MEDICOES;
        assert!(vigia.observar(Some(42), depois));
    }

    #[test]
    fn sair_do_jogo_ou_trocar_de_jogo_recomeca_a_contagem() {
        let mut vigia = Acompanhamento::default();
        assert!(!vigia.observar(Some(42), 0));
        assert!(vigia.observar(Some(42), SEGUNDOS_ANTES_DA_PRIMEIRA));

        assert!(!vigia.observar(Some(7), SEGUNDOS_ANTES_DA_PRIMEIRA + 30));
        assert!(!vigia.observar(Some(7), SEGUNDOS_ANTES_DA_PRIMEIRA + 60));

        assert!(!vigia.observar(None, 10_000));
        assert_eq!(vigia, Acompanhamento::default());
    }

    #[test]
    fn historico_que_nao_existe_e_vazio_e_ilegivel_e_erro() {
        let pasta = pasta_de_teste("leitura");
        let arquivo = pasta.join("medicoes.json");

        assert_eq!(ler_de(&arquivo), Ok(Vec::new()));

        fs::create_dir_all(&pasta).unwrap();
        fs::write(&arquivo, "isto não é json").unwrap();
        let resultado = ler_de(&arquivo);
        let _ = fs::remove_dir_all(&pasta);

        assert!(resultado.is_err(), "arquivo ilegível virou lista vazia");
    }

    #[test]
    fn o_historico_guarda_so_as_ultimas() {
        let pasta = pasta_de_teste("teto");
        let arquivo = pasta.join("medicoes.json");

        for indice in 0..(GUARDADAS + 5) {
            registrar_em(&arquivo, exemplo(indice as f64)).unwrap();
        }

        let todas = ler_de(&arquivo).unwrap();
        let _ = fs::remove_dir_all(&pasta);

        assert_eq!(todas.len(), GUARDADAS);
        assert_eq!(todas.first().unwrap().fps, 5.0);
        assert_eq!(todas.last().unwrap().fps, (GUARDADAS + 4) as f64);
    }

    #[test]
    fn historico_ilegivel_nao_e_apagado_por_uma_medicao_nova() {
        let pasta = pasta_de_teste("ilegivel");
        let arquivo = pasta.join("medicoes.json");
        fs::create_dir_all(&pasta).unwrap();
        fs::write(&arquivo, "corrompido").unwrap();

        let resultado = registrar_em(&arquivo, exemplo(60.0));
        let conteudo = fs::read_to_string(&arquivo).unwrap();
        let _ = fs::remove_dir_all(&pasta);

        assert!(resultado.is_err());
        assert_eq!(
            conteudo, "corrompido",
            "a medição nova apagou o que estava lá"
        );
    }
}
