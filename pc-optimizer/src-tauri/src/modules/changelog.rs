// Change Log
// Registra cada mudança aplicada no sistema para permitir rollback granular.
//
// Toda otimização reversível grava aqui o valor ANTERIOR antes de escrever o novo.
// Sem esse registro não existe "desfazer" honesto — apenas a promessa dele.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

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
        /// O valor anterior no modo bateria.
        ///
        /// `Option` com padrão porque existe `changes.json` em máquina de
        /// cliente gravado antes de o produto passar a escrever a bateria. Ler
        /// um desses arquivos precisa continuar funcionando: `None` significa
        /// "esta mudança é antiga e só mexeu na tomada", e a reversão respeita
        /// isso em vez de inventar um valor para a bateria.
        #[serde(default)]
        previous_dc: Option<PreviousValue>,
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

    /// Um ajuste do driver da NVIDIA, aplicado pela NVAPI.
    ///
    /// POR QUE ESTA VARIANTE GUARDA TEXTO, E NÃO UM NÚMERO
    ///
    /// O valor anterior de um ajuste da NVIDIA tem DOIS estados que precisam
    /// caber no mesmo campo: um número que o cliente já tinha escolhido, ou "o
    /// padrão de fábrica do driver". Os dois são reversíveis, mas por caminhos
    /// diferentes — o número volta escrito, o padrão volta pela chamada de
    /// restauração da própria NVAPI, que é o que permitiu este pilar existir.
    ///
    /// A tradução dos dois sentidos mora no `nvdriver.rs`, junto da chamada que
    /// os usa; aqui fica só o texto que atravessa o disco.
    DriverNvidia {
        /// O identificador do ajuste no catálogo do `nvdriver.rs`.
        opcao: String,
        /// O valor que existia antes, ou `nvdriver::ANTERIOR_PADRAO` quando o
        /// que existia antes era o padrão de fábrica.
        valor_anterior: String,
    },

    /// O limite de quadros de UM jogo, no perfil do executável dele no driver
    /// da NVIDIA.
    ///
    /// `perfil_criado` decide o desfazer: o perfil que o Otimiza criou é
    /// apagado inteiro, achado pelo nome que só o Otimiza usa; o perfil que já
    /// existia só tem o limite devolvido ao que era.
    LimiteNvidia {
        executavel: String,
        fps: u32,
        perfil_criado: bool,
        /// Como no `DriverNvidia`: um número, ou `nvdriver::ANTERIOR_PADRAO`.
        valor_anterior: String,
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
                // O mesmo registro serve aos dois sentidos: desligar um serviço
                // que subia, e religar um essencial que o Windows modificado
                // trouxe desligado — este, com "antes: disabled".
                if previous == "disabled" {
                    format!("serviço · {} religado (antes: desativado)", service)
                } else {
                    format!("serviço · {} desativado (antes: {})", service, previous)
                }
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
                    // 0 é o limitador desligado no driver.
                    "antes: sem limite".to_string()
                } else {
                    format!("antes: {} FPS", valor_anterior)
                }
            ),
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

/// Por que o histórico está do jeito que está.
///
/// Vazio tem dois significados, e confundi-los é a diferença entre "não há
/// nada aplicado" e "não sei o que foi aplicado". O segundo caso PRECISA
/// aparecer na tela: sem ele, o produto diz que não há nada a desfazer sobre
/// uma máquina que pode estar cheia de mudanças aplicadas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "estado")]
pub enum LeituraDoHistorico {
    /// Li o arquivo, ou ele não existe. Vazio aqui é vazio de verdade.
    #[default]
    Ok,
    /// O arquivo existia e não deu para ler. Vazio aqui é DESCONHECIMENTO.
    ///
    /// `guardado_em` é para onde o arquivo ilegível foi movido. Ele não é
    /// apagado: pode ser a única cópia do que o cliente tem aplicado, e um
    /// humano ainda consegue ler JSON truncado.
    Ilegivel {
        motivo: String,
        guardado_em: Option<String>,
    },
}


/// Um nome de arquivo temporário que ninguém mais vai usar.
///
/// NOME FIXO NÃO SERVE, e o teste provou antes do cliente: dois caminhos
/// gravando ao mesmo tempo disputam o mesmo temporário, o primeiro a renomear
/// leva o arquivo embora, e o segundo falha com "não encontrado".
///
/// No produto isso aconteceria com duas janelas abertas, ou com um `persist`
/// disparado enquanto outro ainda não terminou. Identificador do processo mais
/// o relógio em nanossegundos separa os dois casos sem custo.
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

/// Histórico persistente de otimizações aplicadas.
pub struct ChangeLog {
    path: PathBuf,
    entries: Vec<AppliedOptimization>,
    leitura: LeituraDoHistorico,
}

impl ChangeLog {
    /// Carrega o histórico do disco.
    ///
    /// NÃO devolve erro, para não travar o app — mas também não finge que
    /// arquivo ilegível é arquivo vazio. Ver `LeituraDoHistorico`.
    pub fn load() -> Self {
        let path = Self::storage_path();
        let (entries, leitura) = Self::ler(&path);

        ChangeLog { path, entries, leitura }
    }

    /// A leitura deu certo, ou o vazio é desconhecimento?
    pub fn leitura(&self) -> &LeituraDoHistorico {
        &self.leitura
    }

    /// Lê o arquivo, separando "não existe" de "não consegui ler".
    ///
    /// Função com o caminho por parâmetro de propósito: o caminho de verdade
    /// sai de `%APPDATA%`, e um teste que dependesse disso escreveria no perfil
    /// de quem roda a esteira.
    fn ler(path: &Path) -> (Vec<AppliedOptimization>, LeituraDoHistorico) {
        let bruto = match fs::read_to_string(path) {
            Ok(bruto) => bruto,
            // Arquivo ausente é o estado normal de quem nunca aplicou nada.
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
                // JSON truncado é o resultado esperado de uma queda no meio da
                // escrita — foi para isso que `persist` virou atômica. Aqui
                // trata-se do arquivo que JÁ ficou assim.
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

    /// Move o arquivo ilegível para o lado, em vez de deixá-lo ser sobrescrito.
    ///
    /// Sem isto, a primeira gravação seguinte passa por cima dele e a última
    /// pista do que estava aplicado na máquina do cliente some para sempre.
    /// Mover também libera o caminho, para o produto seguir funcionando.
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

    /// Grava o histórico de forma ATÔMICA: escreve ao lado e renomeia por cima.
    ///
    /// A gravação direta com `fs::write` deixava uma janela real de perda: uma
    /// queda de energia, ou o processo morto no meio, produzia um JSON truncado
    /// — e o arquivo truncado era lido depois como histórico vazio. Todas as
    /// otimizações voltavam a aparecer como disponíveis, "Desfazer tudo"
    /// respondia que não havia nada a fazer, e as mudanças continuavam
    /// aplicadas no registro do cliente. A promessa central do produto morria
    /// em silêncio, por causa de uma tomada.
    ///
    /// Com temporário + `rename`, o arquivo de destino ou é o antigo inteiro ou
    /// é o novo inteiro. No Windows o `rename` do Rust substitui o destino.
    ///
    /// Caminho por parâmetro pelo mesmo motivo de `ler`: testabilidade.
    fn gravar(path: &Path, entries: &[AppliedOptimization]) -> Result<(), String> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|e| format!("Failed to create data dir: {}", e))?;
        }

        let raw = serde_json::to_string_pretty(entries)
            .map_err(|e| format!("Failed to serialize change log: {}", e))?;

        let temporario = caminho_temporario(path);

        fs::write(&temporario, raw)
            .map_err(|e| format!("Failed to write change log: {}", e))?;

        fs::rename(&temporario, path)
            .map_err(|e| format!("Failed to replace change log: {}", e))
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
mod tests_1_8 {
    use super::*;

    /// Uma pasta só deste teste, dentro do temporário do sistema.
    ///
    /// O caminho de verdade sai de `%APPDATA%`, e um teste que escrevesse lá
    /// mexeria no histórico real de quem roda a esteira — inclusive no do dono.
    fn pasta(nome: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("otimiza-teste-{}", nome));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("criar pasta de teste");
        dir
    }

    /// Arquivo que não existe é o estado normal de quem nunca aplicou nada.
    /// Vazio aqui é vazio de verdade, e não pode virar alarme.
    #[test]
    fn arquivo_ausente_e_historico_vazio_de_verdade() {
        let caminho = pasta("ausente").join("changes.json");

        let (entradas, leitura) = ChangeLog::ler(&caminho);

        assert!(entradas.is_empty());
        assert_eq!(leitura, LeituraDoHistorico::Ok);
    }

    /// O caso que este conserto existe para pegar.
    ///
    /// Antes, um JSON truncado — o resultado de uma queda no meio da escrita —
    /// era lido como histórico vazio. Todas as otimizações voltavam a aparecer
    /// como disponíveis e "Desfazer tudo" dizia que não havia nada a fazer,
    /// enquanto as mudanças seguiam aplicadas no registro do cliente.
    #[test]
    fn arquivo_truncado_nao_vira_historico_vazio_em_silencio() {
        let dir = pasta("truncado");
        let caminho = dir.join("changes.json");

        // Exatamente o que sobra de um `fs::write` interrompido.
        fs::write(&caminho, r#"[{"optimization_id":"algo","name":"Te"#).unwrap();

        let (entradas, leitura) = ChangeLog::ler(&caminho);

        assert!(entradas.is_empty(), "sem conseguir ler, nao ha o que listar");

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

    /// Histórico válido continua sendo lido normalmente.
    #[test]
    fn arquivo_valido_e_lido_inteiro() {
        let caminho = pasta("valido").join("changes.json");

        ChangeLog::gravar(&caminho, &[sample("um"), sample("dois")]).unwrap();

        let (entradas, leitura) = ChangeLog::ler(&caminho);

        assert_eq!(entradas.len(), 2);
        assert_eq!(leitura, LeituraDoHistorico::Ok);
    }

    /// A gravação não pode deixar o temporário para trás: ele seria lido como
    /// lixo por quem abrisse a pasta, e ocuparia espaço para sempre.
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

    /// Gravar por cima de um histórico que já existe substitui o conteúdo
    /// inteiro — é o que a troca por `rename` precisa continuar fazendo.
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

    /// ChangeLog em memória, sem tocar no disco do usuário durante os testes.
    fn in_memory() -> ChangeLog {
        ChangeLog {
            path: std::env::temp_dir().join("pc-optimizer-test-changes.json"),
            entries: Vec::new(),
            // Comeca vazio de verdade, e nao por nao ter conseguido ler.
            leitura: LeituraDoHistorico::Ok,
        }
    }

    #[test]
    fn servico_religado_nao_aparece_como_desativado() {
        // O mesmo registro serve para desligar um serviço e para religar um
        // essencial. Religar o Plug and Play e escrever "desativado" no
        // acompanhamento ao vivo seria dizer ao cliente o contrário do que
        // aconteceu.
        let religado = ChangeRecord::ServiceStartType {
            service: "PlugPlay".to_string(),
            previous: "disabled".to_string(),
        };
        assert_eq!(religado.describe(), "serviço · PlugPlay religado (antes: desativado)");

        let desligado = ChangeRecord::ServiceStartType {
            service: "SysMain".to_string(),
            previous: "auto".to_string(),
        };
        assert_eq!(desligado.describe(), "serviço · SysMain desativado (antes: auto)");
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

    /// A linha que o cliente le sobre um ajuste do driver da NVIDIA precisa
    /// dizer O QUE ERA ANTES em portugues, e nao cuspir o codigo interno.
    ///
    /// Os dois casos sao opostos e nao podem ser trocados: "o padrao do driver"
    /// e um estado de fabrica; um numero e uma escolha que o cliente ja tinha
    /// feito. Confundir os dois na tela faria o cliente achar que o Otimiza
    /// apagou a configuracao dele -- ou o contrario.
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
