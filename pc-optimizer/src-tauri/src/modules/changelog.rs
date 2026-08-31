// Change Log
// Registra cada mudança aplicada no sistema para permitir rollback granular.
//
// Toda otimização reversível grava aqui o valor ANTERIOR antes de escrever o novo.
// Sem esse registro não existe "desfazer" honesto — apenas a promessa dele.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Valor que existia antes da otimização ser aplicada.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum PreviousValue {
    /// A chave existia, mas o valor não — reverter significa apagar o valor.
    Absent,
    /// Nem a chave existia. Reverter apaga o valor E a chave criada,
    /// para não deixar sujeira no registro do cliente.
    AbsentKey,
    Dword(u32),
    Text(String),
    /// Valor bruto (REG_BINARY). O Windows guarda o estado dos programas de
    /// inicialização assim, e é o único jeito de mexer nisso do modo que o
    /// Gerenciador de Tarefas mexe.
    Binary(Vec<u8>),
}

/// Uma mudança atômica no sistema, com informação suficiente para desfazê-la.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
        /// Tipo de inicialização anterior, no formato do `sc config` (auto/demand/disabled/delayed-auto)
        previous: String,
    },
    PowerPlan {
        previous_guid: String,
    },
    Hibernation {
        previously_enabled: bool,
    },
    /// Ajuste fino do plano de energia. O valor anterior pode não existir: nesse
    /// caso o plano estava herdando o padrão do Windows, e reverter é apagá-lo.
    PowerSetting {
        scheme: String,
        subgroup: String,
        setting: String,
        previous: PreviousValue,
    },
    MemoryCompression {
        previously_enabled: bool,
    },
    /// Limites de inicialização removidos, guardados para poder voltar.
    BootLimits {
        removed: Vec<(String, String)>,
    },
    /// Armazenamento Reservado do Windows ligado ou desligado.
    ReservedStorage {
        previously_enabled: bool,
    },
    /// Taxa de atualização de um monitor.
    ///
    /// Guarda o dispositivo e a frequência anterior. Sem isto o cliente que
    /// não gostou do resultado — ou cuja tela ficou instável — não teria
    /// caminho de volta pelo produto.
    RefreshRate {
        device: String,
        previous_hz: u32,
    },
    /// Tarefa agendada ligada ou desligada.
    ScheduledTask {
        path: String,
        name: String,
        previously_enabled: bool,
    },

    /// Um arquivo de configuração de jogo, guardado INTEIRO antes de ser
    /// alterado.
    ///
    /// POR QUE O ARQUIVO INTEIRO, E NÃO AS CHAVES QUE MUDARAM
    ///
    /// Todas as outras variantes guardam o valor anterior de uma coisa só, e
    /// isso funciona porque registro e serviço são pares de chave e valor com
    /// dono conhecido. Um arquivo de configuração de jogo não é: o próprio jogo
    /// reescreve o arquivo quando quer, reordena as linhas, acrescenta chaves
    /// que não existiam na versão passada e muda o formato entre atualizações.
    ///
    /// Guardando só as chaves mexidas, "desfazer" depois de o jogo ter
    /// reescrito o arquivo devolveria um valor antigo para dentro de uma
    /// estrutura nova — e o resultado seria um arquivo que nem é o de antes nem
    /// o de agora. O arquivo inteiro é a única coisa que garante que voltar
    /// significa voltar.
    ///
    /// Custa alguns kilobytes por mudança. É o preço mais barato deste projeto.
    GameConfig {
        /// Caminho completo, para o desfazer não depender de reencontrar a
        /// pasta do jogo — que pode ter sido movida ou desinstalada.
        caminho: String,
        /// O conteúdo que existia antes. `None` quando o arquivo não existia:
        /// desfazer, nesse caso, é apagá-lo.
        anterior: Option<String>,
        /// Nome do jogo, só para a descrição que o cliente lê.
        jogo: String,
    },
}

impl ChangeRecord {
    /// Descrição em português do que foi alterado, para o cliente acompanhar ao
    /// vivo em vez de confiar numa barra de progresso.
    ///
    /// Mostrar exatamente o que se mexeu é o que separa uma ferramenta de
    /// confiança de uma caixa preta que diz "otimizado!".
    pub fn describe(&self) -> String {
        match self {
            ChangeRecord::RegistryValue { path, name, previous, .. } => {
                let key = path.rsplit('\\').next().unwrap_or(path);
                format!("registro · {}\\{} (antes: {})", key, name, previous.describe())
            }
            ChangeRecord::ServiceStartType { service, previous } => {
                format!("serviço · {} desativado (antes: {})", service, previous)
            }
            ChangeRecord::PowerPlan { .. } => "plano de energia trocado".to_string(),
            ChangeRecord::Hibernation { .. } => "hibernação desligada".to_string(),
            ChangeRecord::PowerSetting { setting, previous, .. } => {
                let short: String = setting.chars().take(8).collect();
                format!("energia · ajuste {} (antes: {})", short, previous.describe())
            }
            ChangeRecord::MemoryCompression { .. } => "compressão de memória desligada".to_string(),
            ChangeRecord::ReservedStorage { .. } => {
                "armazenamento reservado liberado".to_string()
            }
            ChangeRecord::ScheduledTask { name, previously_enabled, .. } => format!(
                "tarefa agendada · {} (antes: {})",
                name,
                if *previously_enabled { "ligada" } else { "desligada" }
            ),
            ChangeRecord::RefreshRate { device, previous_hz } => {
                let curto = device.rsplit('\\').next().unwrap_or(device);
                format!("monitor · {} (antes: {} Hz)", curto, previous_hz)
            }
            ChangeRecord::BootLimits { removed } => {
                let keys: Vec<&str> = removed.iter().map(|(key, _)| key.as_str()).collect();
                format!("boot · limites removidos: {}", keys.join(", "))
            }
            ChangeRecord::GameConfig { jogo, anterior, .. } => format!(
                "{} · configuração alterada ({})",
                jogo,
                match anterior {
                    // O tamanho é a prova visível de que há para onde voltar.
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
            // O primeiro byte é o que decide habilitado (0x02) ou desabilitado (0x03).
            PreviousValue::Binary(bytes) => match bytes.first() {
                Some(2) => "habilitado".to_string(),
                Some(3) => "desabilitado".to_string(),
                _ => "valor binário".to_string(),
            },
        }
    }
}

/// Uma otimização aplicada e todas as mudanças que ela causou.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppliedOptimization {
    pub optimization_id: String,
    pub name: String,
    pub timestamp: u64,
    pub changes: Vec<ChangeRecord>,
}

/// Histórico persistente de otimizações aplicadas.
pub struct ChangeLog {
    path: PathBuf,
    entries: Vec<AppliedOptimization>,
}

impl ChangeLog {
    /// Carrega o histórico do disco. Um arquivo ausente ou corrompido resulta em
    /// histórico vazio — nunca em erro, para não travar o app.
    pub fn load() -> Self {
        let path = Self::storage_path();
        let entries = fs::read_to_string(&path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default();

        ChangeLog { path, entries }
    }

    fn storage_path() -> PathBuf {
        let base = std::env::var("APPDATA")
            .or_else(|_| std::env::var("HOME"))
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));

        base.join("pc-optimizer").join("changes.json")
    }

    fn persist(&self) -> Result<(), String> {
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir).map_err(|e| format!("Failed to create data dir: {}", e))?;
        }

        let raw = serde_json::to_string_pretty(&self.entries)
            .map_err(|e| format!("Failed to serialize change log: {}", e))?;

        fs::write(&self.path, raw).map_err(|e| format!("Failed to write change log: {}", e))
    }

    /// Registra uma otimização aplicada. Substitui um registro anterior da mesma
    /// otimização para que o histórico guarde sempre o estado original mais antigo
    /// que ainda não foi revertido.
    pub fn record(&mut self, entry: AppliedOptimization) -> Result<(), String> {
        if !self.entries.iter().any(|e| e.optimization_id == entry.optimization_id) {
            self.entries.push(entry);
            self.persist()?;
        }
        Ok(())
    }

    /// Remove e devolve o registro de uma otimização, para que ela possa ser revertida.
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

    /// Otimizações atualmente aplicadas.
    pub fn applied(&self) -> &[AppliedOptimization] {
        &self.entries
    }

    /// Verifica se uma otimização específica está aplicada.
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

    /// ChangeLog em memória, sem tocar no disco do usuário durante os testes.
    fn in_memory() -> ChangeLog {
        ChangeLog {
            path: std::env::temp_dir().join("pc-optimizer-test-changes.json"),
            entries: Vec::new(),
        }
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
}
