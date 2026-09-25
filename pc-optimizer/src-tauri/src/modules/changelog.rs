// Registro de cada mudança aplicada, com o valor ANTERIOR gravado antes do novo: sem ele não existe desfazer
// honesto.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum PreviousValue {
    Absent,
    /// Reverter apaga o valor E a chave criada.
    AbsentKey,
    Dword(u32),
    Text(String),
    /// É assim que o Windows guarda o estado dos programas de inicialização, como o Gerenciador de Tarefas faz.
    Binary(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ChangeRecord {
    RegistryValue {
        hive: String,
        path: String,
        name: String,
        previous: PreviousValue,
    },
    ServiceStartType {
        service: String,
        previous: String,
    },
    PowerPlan {
        previous_guid: String,
    },
    Hibernation {
        previously_enabled: bool,
    },
    /// Sem valor anterior o plano herdava o padrão do Windows: reverter é apagar.
    PowerSetting {
        scheme: String,
        subgroup: String,
        setting: String,
        previous: PreviousValue,
        /// `None` = mudança gravada antes de o produto escrever a bateria: reverte só a tomada, sem inventar valor.
        #[serde(default)]
        previous_dc: Option<PreviousValue>,
    },
    MemoryCompression {
        previously_enabled: bool,
    },
    BootLimits {
        removed: Vec<(String, String)>,
    },
    ReservedStorage {
        previously_enabled: bool,
    },
    RefreshRate {
        device: String,
        previous_hz: u32,
    },
    ScheduledTask {
        path: String,
        name: String,
        previously_enabled: bool,
    },

    /// O arquivo INTEIRO: o jogo reescreve, reordena e muda o formato, e devolver só as chaves mexidas geraria um
    /// arquivo que não é nem o de antes nem o de agora.
    GameConfig {
        /// Caminho completo: a pasta do jogo pode ter sido movida ou desinstalada.
        caminho: String,
        /// `None` quando o arquivo não existia: desfazer é apagá-lo.
        anterior: Option<String>,
        jogo: String,
    },

    /// Texto porque o anterior tem dois estados: um número escolhido (volta escrito) ou o padrão de fábrica (volta
    /// pela restauração da NVAPI). A tradução mora no `nvdriver.rs`.
    DriverNvidia {
        opcao: String,
        valor_anterior: String,
    },

    /// Perfil criado pelo Otimiza é apagado inteiro; perfil que já existia tem cada ajuste devolvido.
    PerfilNvidia {
        executavel: String,
        perfil: String,
        perfil_criado: bool,
        anteriores: Vec<(String, String)>,
    },

    /// `perfil_criado` decide o desfazer: criado pelo Otimiza é apagado inteiro (pelo nome que só ele usa); o que já
    /// existia só tem o limite devolvido.
    LimiteNvidia {
        executavel: String,
        fps: u32,
        perfil_criado: bool,
        valor_anterior: String,
    },
}

impl ChangeRecord {
    /// Mostrar exatamente o que se mexeu, ao vivo, em vez de "otimizado!".
    pub fn describe(&self) -> String {
        match self {
            ChangeRecord::RegistryValue {
                path,
                name,
                previous,
                ..
            } => {
                let key = path.rsplit('\\').next().unwrap_or(path);
                format!(
                    "registro · {}\\{} (antes: {})",
                    key,
                    name,
                    previous.describe()
                )
            }
            ChangeRecord::ServiceStartType { service, previous } => {
                // Serve aos dois sentidos: desligar um serviço e religar um essencial que o Windows modificado trouxe desligado.
                if previous == "disabled" {
                    format!("serviço · {} religado (antes: desativado)", service)
                } else {
                    format!("serviço · {} desativado (antes: {})", service, previous)
                }
            }
            ChangeRecord::PowerPlan { .. } => "plano de energia trocado".to_string(),
            ChangeRecord::Hibernation { .. } => "hibernação desligada".to_string(),
            ChangeRecord::PowerSetting {
                setting, previous, ..
            } => {
                let short: String = setting.chars().take(8).collect();
                format!(
                    "energia · ajuste {} (antes: {})",
                    short,
                    previous.describe()
                )
            }
            ChangeRecord::MemoryCompression { .. } => "compressão de memória desligada".to_string(),
            ChangeRecord::ReservedStorage { .. } => "armazenamento reservado liberado".to_string(),
            ChangeRecord::ScheduledTask {
                name,
                previously_enabled,
                ..
            } => format!(
                "tarefa agendada · {} (antes: {})",
                name,
                if *previously_enabled {
                    "ligada"
                } else {
                    "desligada"
                }
            ),
            ChangeRecord::RefreshRate {
                device,
                previous_hz,
            } => {
                let curto = device.rsplit('\\').next().unwrap_or(device);
                format!("monitor · {} (antes: {} Hz)", curto, previous_hz)
            }
            ChangeRecord::BootLimits { removed } => {
                let keys: Vec<&str> = removed.iter().map(|(key, _)| key.as_str()).collect();
                format!("boot · limites removidos: {}", keys.join(", "))
            }
            ChangeRecord::DriverNvidia {
                opcao,
                valor_anterior,
            } => format!(
                "driver NVIDIA · {} (antes: {})",
                opcao,
                if valor_anterior == crate::modules::windows::nvdriver::ANTERIOR_PADRAO {
                    "o padrão do driver".to_string()
                } else {
                    valor_anterior.clone()
                }
            ),
            ChangeRecord::PerfilNvidia { executavel, perfil, perfil_criado, anteriores } => format!(
                "driver NVIDIA · {} no perfil {} ({} ajuste(s){})",
                executavel,
                perfil,
                anteriores.len(),
                if *perfil_criado { ", perfil criado pelo Otimiza" } else { "" }
            ),
            ChangeRecord::LimiteNvidia {
                executavel,
                fps,
                perfil_criado,
                valor_anterior,
            } => format!(
                "driver NVIDIA · {} limitado a {} FPS ({})",
                executavel,
                fps,
                if *perfil_criado {
                    "perfil criado pelo Otimiza".to_string()
                } else if valor_anterior == crate::modules::windows::nvdriver::ANTERIOR_PADRAO
                    || valor_anterior == "0"
                {
                    "antes: sem limite".to_string()
                } else {
                    format!("antes: {} FPS", valor_anterior)
                }
            ),
            ChangeRecord::GameConfig { jogo, anterior, .. } => format!(
                "{} · configuração alterada ({})",
                jogo,
                match anterior {
                    Some(texto) => format!("cópia de {} bytes guardada", texto.len()),
                    None => "o arquivo não existia".to_string(),
                }
            ),
        }
    }
}

impl PreviousValue {
    fn describe(&self) -> String {
        match self {
            PreviousValue::Absent | PreviousValue::AbsentKey => "não existia".to_string(),
            PreviousValue::Dword(value) => value.to_string(),
            PreviousValue::Text(value) if value.is_empty() => "vazio".to_string(),
            PreviousValue::Text(value) => value.clone(),
            PreviousValue::Binary(bytes) => match bytes.first() {
                Some(2) => "habilitado".to_string(),
                Some(3) => "desabilitado".to_string(),
                _ => "valor binário".to_string(),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppliedOptimization {
    pub optimization_id: String,
    pub name: String,
    pub timestamp: u64,
    pub changes: Vec<ChangeRecord>,
}

/// Vazio pode ser "nada aplicado" ou "não sei o que foi aplicado"; o segundo PRECISA aparecer na tela.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "estado")]
pub enum LeituraDoHistorico {
    #[default]
    Ok,
    /// `guardado_em`: para onde o ilegível foi movido. Não é apagado: pode ser a única cópia, e um humano lê JSON
    /// truncado.
    Ilegivel {
        motivo: String,
        guardado_em: Option<String>,
    },
}

/// Nome fixo não serve: duas gravações simultâneas disputavam o temporário e a segunda falhava com "não
/// encontrado". PID mais nanossegundos separa.
pub(crate) fn caminho_temporario(destino: &Path) -> PathBuf {
    let marca = format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    );

    destino.with_extension(format!("novo-{}", marca))
}

pub struct ChangeLog {
    path: PathBuf,
    entries: Vec<AppliedOptimization>,
    leitura: LeituraDoHistorico,
}

impl ChangeLog {
    /// Não devolve erro, para não travar o app, mas não finge que ilegível é vazio (ver `LeituraDoHistorico`).
    pub fn load() -> Self {
        let path = Self::storage_path();
        let (entries, leitura) = Self::ler(&path);

        ChangeLog {
            path,
            entries,
            leitura,
        }
    }

    /// Nome único pelo mesmo motivo do `in_memory` dos testes daqui.
    #[cfg(test)]
    pub(crate) fn em_memoria() -> Self {
        use std::sync::atomic::{AtomicU32, Ordering};
        static PROXIMO: AtomicU32 = AtomicU32::new(0);
        let numero = PROXIMO.fetch_add(1, Ordering::Relaxed);
        ChangeLog {
            path: std::env::temp_dir().join(format!(
                "pc-optimizer-teste-externo-{}-{}.json",
                std::process::id(),
                numero
            )),
            entries: Vec::new(),
            leitura: LeituraDoHistorico::Ok,
        }
    }

    pub fn leitura(&self) -> &LeituraDoHistorico {
        &self.leitura
    }

    /// Caminho por parâmetro: o real está em `%APPDATA%`, e o teste escreveria no perfil de quem roda a esteira.
    fn ler(path: &Path) -> (Vec<AppliedOptimization>, LeituraDoHistorico) {
        let bruto = match fs::read_to_string(path) {
            Ok(bruto) => bruto,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return (Vec::new(), LeituraDoHistorico::Ok)
            }
            Err(e) => {
                return (
                    Vec::new(),
                    LeituraDoHistorico::Ilegivel {
                        motivo: format!("não consegui abrir o arquivo: {}", e),
                        guardado_em: None,
                    },
                )
            }
        };

        match serde_json::from_str(&bruto) {
            Ok(entries) => (entries, LeituraDoHistorico::Ok),
            Err(e) => {
                let guardado = Self::guardar_ilegivel(path);

                (
                    Vec::new(),
                    LeituraDoHistorico::Ilegivel {
                        motivo: format!("o arquivo não é um histórico válido: {}", e),
                        guardado_em: guardado,
                    },
                )
            }
        }
    }

    /// Sem isto, a próxima gravação passa por cima da última pista do que estava aplicado.
    fn guardar_ilegivel(path: &Path) -> Option<String> {
        let carimbo = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let destino = path.with_extension(format!("json.ilegivel-{}", carimbo));

        fs::rename(path, &destino)
            .ok()
            .map(|_| destino.to_string_lossy().to_string())
    }

    fn storage_path() -> PathBuf {
        let base = std::env::var("APPDATA")
            .or_else(|_| std::env::var("HOME"))
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));

        base.join("pc-optimizer").join("changes.json")
    }

    fn persist(&self) -> Result<(), String> {
        Self::gravar(&self.path, &self.entries)
    }

    /// ATÔMICA: um JSON truncado por queda de energia era lido como vazio, e "Desfazer tudo" dizia que não havia
    /// nada com as mudanças ainda aplicadas.
    fn gravar(path: &Path, entries: &[AppliedOptimization]) -> Result<(), String> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|e| format!("Failed to create data dir: {}", e))?;
        }

        let raw = serde_json::to_string_pretty(entries)
            .map_err(|e| format!("Failed to serialize change log: {}", e))?;

        let temporario = caminho_temporario(path);

        // `sync_all` antes do rename: `fs::write` volta com o conteúdo no cache, e a queda podia gravar o rename sem o
        // conteúdo.
        {
            use std::io::Write;

            let mut arquivo = fs::File::create(&temporario)
                .map_err(|e| format!("Failed to write change log: {}", e))?;

            arquivo
                .write_all(raw.as_bytes())
                .map_err(|e| format!("Failed to write change log: {}", e))?;

            arquivo
                .sync_all()
                .map_err(|e| format!("Failed to flush change log: {}", e))?;
        }

        fs::rename(&temporario, path).map_err(|e| format!("Failed to replace change log: {}", e))
    }

    /// Substitui o registro anterior da mesma otimização, para guardar o estado original mais antigo ainda não
    /// revertido.
    pub fn record(&mut self, entry: AppliedOptimization) -> Result<(), String> {
        if !self
            .entries
            .iter()
            .any(|e| e.optimization_id == entry.optimization_id)
        {
            self.entries.push(entry);
            self.persist()?;
        }
        Ok(())
    }

    pub fn take(&mut self, optimization_id: &str) -> Result<Option<AppliedOptimization>, String> {
        match self
            .entries
            .iter()
            .position(|e| e.optimization_id == optimization_id)
        {
            Some(index) => {
                let entry = self.entries.remove(index);
                self.persist()?;
                Ok(Some(entry))
            }
            None => Ok(None),
        }
    }

    pub fn applied(&self) -> &[AppliedOptimization] {
        &self.entries
    }

    pub fn is_applied(&self, optimization_id: &str) -> bool {
        self.entries
            .iter()
            .any(|e| e.optimization_id == optimization_id)
    }
}

pub fn now_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests_1_8 {
    use super::*;

    fn pasta(nome: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("otimiza-teste-{}", nome));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("criar pasta de teste");
        dir
    }

    #[test]
    fn arquivo_ausente_e_historico_vazio_de_verdade() {
        let caminho = pasta("ausente").join("changes.json");

        let (entradas, leitura) = ChangeLog::ler(&caminho);

        assert!(entradas.is_empty());
        assert_eq!(leitura, LeituraDoHistorico::Ok);
    }

    #[test]
    fn arquivo_truncado_nao_vira_historico_vazio_em_silencio() {
        let dir = pasta("truncado");
        let caminho = dir.join("changes.json");

        fs::write(&caminho, r#"[{"optimization_id":"algo","name":"Te"#).unwrap();

        let (entradas, leitura) = ChangeLog::ler(&caminho);

        assert!(
            entradas.is_empty(),
            "sem conseguir ler, nao ha o que listar"
        );

        match leitura {
            LeituraDoHistorico::Ilegivel { guardado_em, .. } => {
                let guardado = guardado_em.expect("o arquivo ilegivel precisa ser guardado");

                assert!(
                    std::path::Path::new(&guardado).exists(),
                    "o arquivo ilegivel foi perdido: ele pode ser a unica pista do \
                     que o cliente tem aplicado"
                );

                assert!(
                    !caminho.exists(),
                    "o caminho precisa ficar livre, senao a proxima gravacao passa \
                     por cima da unica copia"
                );
            }
            LeituraDoHistorico::Ok => {
                panic!("arquivo truncado foi lido como historico valido e vazio")
            }
        }
    }

    #[test]
    fn arquivo_valido_e_lido_inteiro() {
        let caminho = pasta("valido").join("changes.json");

        ChangeLog::gravar(&caminho, &[sample("um"), sample("dois")]).unwrap();

        let (entradas, leitura) = ChangeLog::ler(&caminho);

        assert_eq!(entradas.len(), 2);
        assert_eq!(leitura, LeituraDoHistorico::Ok);
    }

    #[test]
    fn a_gravacao_nao_deixa_arquivo_temporario_para_tras() {
        let dir = pasta("temporario");
        let caminho = dir.join("changes.json");

        ChangeLog::gravar(&caminho, &[sample("um")]).unwrap();

        let sobraram: Vec<String> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();

        assert_eq!(sobraram, vec!["changes.json".to_string()]);
    }

    #[test]
    fn gravar_por_cima_substitui_o_conteudo() {
        let caminho = pasta("substitui").join("changes.json");

        ChangeLog::gravar(&caminho, &[sample("antigo")]).unwrap();
        ChangeLog::gravar(&caminho, &[sample("novo")]).unwrap();

        let (entradas, _) = ChangeLog::ler(&caminho);

        assert_eq!(entradas.len(), 1);
        assert_eq!(entradas[0].optimization_id, "novo");
    }

    fn sample(id: &str) -> AppliedOptimization {
        AppliedOptimization {
            optimization_id: id.to_string(),
            name: "Teste".to_string(),
            timestamp: 0,
            changes: vec![ChangeRecord::PowerPlan {
                previous_guid: "abc".to_string(),
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(id: &str) -> AppliedOptimization {
        AppliedOptimization {
            optimization_id: id.to_string(),
            name: "Teste".to_string(),
            timestamp: 0,
            changes: vec![ChangeRecord::PowerPlan {
                previous_guid: "abc".to_string(),
            }],
        }
    }

    /// Um arquivo por chamada: com nome fixo os testes em paralelo disputavam o arquivo e falhavam de vez em quando.
    fn in_memory() -> ChangeLog {
        use std::sync::atomic::{AtomicU32, Ordering};

        static PROXIMO: AtomicU32 = AtomicU32::new(0);
        let numero = PROXIMO.fetch_add(1, Ordering::Relaxed);

        ChangeLog {
            path: std::env::temp_dir().join(format!("pc-optimizer-test-changes-{}.json", numero)),
            entries: Vec::new(),
            leitura: LeituraDoHistorico::Ok,
        }
    }

    #[test]
    fn servico_religado_nao_aparece_como_desativado() {
        let religado = ChangeRecord::ServiceStartType {
            service: "PlugPlay".to_string(),
            previous: "disabled".to_string(),
        };
        assert_eq!(
            religado.describe(),
            "serviço · PlugPlay religado (antes: desativado)"
        );

        let desligado = ChangeRecord::ServiceStartType {
            service: "SysMain".to_string(),
            previous: "auto".to_string(),
        };
        assert_eq!(
            desligado.describe(),
            "serviço · SysMain desativado (antes: auto)"
        );
    }

    #[test]
    fn records_and_reports_applied() {
        let mut log = in_memory();
        log.record(sample("power_plan")).unwrap();

        assert!(log.is_applied("power_plan"));
        assert_eq!(log.applied().len(), 1);
    }

    #[test]
    fn recording_twice_keeps_original_entry() {
        let mut log = in_memory();
        log.record(sample("power_plan")).unwrap();
        log.record(sample("power_plan")).unwrap();

        assert_eq!(log.applied().len(), 1);
    }

    #[test]
    fn take_removes_entry_so_it_can_be_reverted_once() {
        let mut log = in_memory();
        log.record(sample("power_plan")).unwrap();

        assert!(log.take("power_plan").unwrap().is_some());
        assert!(!log.is_applied("power_plan"));
        assert!(log.take("power_plan").unwrap().is_none());
    }

    #[test]
    fn taking_unknown_optimization_is_not_an_error() {
        let mut log = in_memory();
        assert!(log.take("never_applied").unwrap().is_none());
    }

    /// "Padrão do driver" e um número escolhido não podem ser trocados na tela: o cliente acharia que o Otimiza
    /// apagou a configuração dele.
    #[test]
    fn a_linha_do_driver_nvidia_diz_o_que_existia_antes() {
        let do_padrao = ChangeRecord::DriverNvidia {
            opcao: "vsync".to_string(),
            valor_anterior: crate::modules::windows::nvdriver::ANTERIOR_PADRAO.to_string(),
        }
        .describe();

        assert!(do_padrao.contains("vsync"), "{}", do_padrao);
        assert!(do_padrao.contains("padrão do driver"), "{}", do_padrao);
        assert!(
            !do_padrao.contains(crate::modules::windows::nvdriver::ANTERIOR_PADRAO),
            "o codigo interno vazou para a tela: {}",
            do_padrao
        );

        let de_escolha = ChangeRecord::DriverNvidia {
            opcao: "energia".to_string(),
            valor_anterior: "3".to_string(),
        }
        .describe();

        assert!(de_escolha.contains("energia"), "{}", de_escolha);
        assert!(de_escolha.contains('3'), "{}", de_escolha);
        assert!(
            !de_escolha.contains("padrão do driver"),
            "o valor que o cliente tinha escolhido virou 'padrao': {}",
            de_escolha
        );
    }
}
