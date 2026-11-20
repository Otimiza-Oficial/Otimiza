// Registro de cada mudança aplicada, com o valor ANTERIOR gravado antes do novo: sem ele não existe desfazer
// honesto.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
#[cfg_attr(test, derive(specta::Type))]
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

    /// Ajuste do driver AMD pelo ADLX. O anterior está no formato de `amddriver::codificar` ("1", "0", "1:141").
    DriverAmd {
        opcao: String,
        valor_anterior: String,
    },

    /// Ajuste do driver Intel pelo IGCL. O anterior está no formato de `inteldriver::codificar` ("e:1", "l:1:141").
    DriverIntel {
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
        /// O caminho pelo qual o perfil foi achado (o perfil que o driver usa). Ausente nos gravados antes (perfil achado só pelo nome).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        caminho: Option<String>,
    },
    /// A fila de 1 quadro do driver NVIDIA no perfil do jogo, junto com o Frame Boost (1.0.7). O desfazer devolve o valor
    /// e não apaga o perfil (o teto da base pode estar nele).
    FilaNvidia {
        executavel: String,
        valor_anterior: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        caminho: Option<String>,
    },
    /// Smooth Motion ligado no perfil do jogo (experimental). O desfazer devolve o valor e não apaga o perfil.
    SmoothMotionNvidia {
        executavel: String,
        perfil_criado: bool,
        valor_anterior: String,
    },

    /// Elemento do `bcdedit` (`windows::pacote`). `None`: não existia, e o desfazer apaga.
    BootValor {
        nome: String,
        anterior: Option<String>,
    },
    /// Parâmetro do `Set-NetTCPSetting` no modelo "Internet".
    RedeTcp {
        parametro: String,
        anterior: String,
    },
    /// Parâmetro do `Set-NetOffloadGlobalSetting`.
    RedeOffload {
        parametro: String,
        anterior: String,
    },
    /// Propriedade avançada da placa de rede que o teste com prova manteve (`plataforma::placa_prova`, R2).
    PropriedadeDaPlaca {
        placa: String,
        palavra: String,
        anterior: String,
    },
    /// As placas de rede em que o recurso estava ligado: o desfazer liga de novo só nelas.
    RedeAdaptador {
        recurso: String,
        placas: Vec<String>,
    },
    /// O modo de vídeo INTEIRO lido antes da troca de taxa (`plataforma::hz`): o desfazer devolve resolução, cor,
    /// taxa, orientação e posição como estavam, não "a taxa de antes na resolução de agora" (`RefreshRate`).
    ModoDeTela {
        dispositivo: String,
        anterior: crate::plataforma::hz::ModoDeVideo,
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
            ChangeRecord::DriverAmd {
                opcao,
                valor_anterior,
            } => format!(
                "driver AMD · {} (antes: {})",
                opcao,
                match crate::modules::windows::amddriver::decodificar(valor_anterior) {
                    Some(crate::modules::windows::amddriver::Anterior::Chave(true)) =>
                        "ligado".to_string(),
                    Some(crate::modules::windows::amddriver::Anterior::Chave(false)) =>
                        "desligado".to_string(),
                    Some(crate::modules::windows::amddriver::Anterior::Limite { ligado, fps }) =>
                        format!(
                            "{} FPS, {}",
                            fps,
                            if ligado { "ligado" } else { "desligado" }
                        ),
                    None => valor_anterior.clone(),
                }
            ),
            ChangeRecord::DriverIntel {
                opcao,
                valor_anterior,
            } => {
                format!("driver Intel · {} (antes: {})", opcao, valor_anterior)
            }
            ChangeRecord::PerfilNvidia {
                executavel,
                perfil,
                perfil_criado,
                anteriores,
            } => format!(
                "driver NVIDIA · {} no perfil {} ({} ajuste(s){})",
                executavel,
                perfil,
                anteriores.len(),
                if *perfil_criado {
                    ", perfil criado pelo Otimiza"
                } else {
                    ""
                }
            ),
            ChangeRecord::SmoothMotionNvidia { executavel, .. } => {
                format!("driver NVIDIA · Smooth Motion ligado em {}", executavel)
            }
            ChangeRecord::FilaNvidia { executavel, .. } => {
                format!("driver NVIDIA · fila de 1 quadro em {} (menos atraso no Frame Boost)", executavel)
            }
            ChangeRecord::BootValor { nome, anterior } => format!(
                "boot · {} (antes: {})",
                nome,
                anterior.as_deref().unwrap_or("padrão do Windows")
            ),
            ChangeRecord::RedeTcp { parametro, anterior } => {
                format!("rede · TCP {} (antes: {})", parametro, anterior)
            }
            ChangeRecord::RedeOffload { parametro, anterior } => {
                format!("rede · {} (antes: {})", parametro, anterior)
            }
            ChangeRecord::RedeAdaptador { recurso, placas } => {
                format!("rede · {} desligado em {}", recurso, placas.join(", "))
            }
            ChangeRecord::PropriedadeDaPlaca { placa, palavra, anterior } => {
                format!("placa de rede {} · {} (antes: {})", placa, palavra, anterior)
            }
            ChangeRecord::ModoDeTela {
                dispositivo,
                anterior,
            } => {
                let curto = dispositivo.rsplit('\\').next().unwrap_or(dispositivo);
                format!(
                    "monitor · {} (antes: {}x{} a {} Hz)",
                    curto, anterior.largura, anterior.altura, anterior.hz
                )
            }
            ChangeRecord::LimiteNvidia {
                executavel,
                fps,
                perfil_criado,
                valor_anterior,
                ..
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
#[cfg_attr(test, derive(specta::Type))]
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

/// Por que o histórico não foi gravado (1.0.12, A8): erro com tipo em vez de texto solto, para quem chama distinguir o
/// caso e a tela receber a frase em português. Vira `String` sozinho no `?` dos comandos (`From`).
#[derive(Debug)]
pub enum ErroDoHistorico {
    /// A pasta de dados (`%APPDATA%\pc-optimizer`) não pôde ser criada.
    Pasta(std::io::Error),
    /// O histórico em memória não virou JSON (não deveria acontecer: os tipos são todos serializáveis).
    Serializar(serde_json::Error),
    /// O temporário não foi escrito nem forçado ao disco (disco cheio, antivírus, permissão).
    Gravar(std::io::Error),
    /// O temporário não substituiu o arquivo (outro programa com o arquivo aberto).
    Trocar(std::io::Error),
}

impl std::fmt::Display for ErroDoHistorico {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErroDoHistorico::Pasta(e) => write!(f, "não consegui criar a pasta do histórico de mudanças: {}", e),
            ErroDoHistorico::Serializar(e) => write!(f, "não consegui montar o histórico de mudanças: {}", e),
            ErroDoHistorico::Gravar(e) => write!(f, "não consegui gravar o histórico de mudanças: {}", e),
            ErroDoHistorico::Trocar(e) => write!(f, "não consegui substituir o histórico de mudanças: {}", e),
        }
    }
}

impl std::error::Error for ErroDoHistorico {}

impl From<ErroDoHistorico> for String {
    fn from(e: ErroDoHistorico) -> String {
        e.to_string()
    }
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

/// O DONO ÚNICO do histórico (1.0.12, A1). Antes, quem quisesse gravar relia o `changes.json` do disco por conta
/// própria (`ChangeLog::load()` no Frame Boost, na prova da placa de rede, no Turbo) e gravava a cópia dele por cima
/// da do app: um vigia que gravasse no meio perdia o registro de um dos dois, e com ele o desfazer. Agora existe UMA
/// instância, atrás desta trava; só quem segura a trava recebe `&mut ChangeLog`, e só com ele se grava.
///
/// Nos testes começa vazio num arquivo temporário (`caminho_do_dono`): o real está em `%APPDATA%`, o perfil de quem
/// roda a esteira.
static HISTORICO: std::sync::LazyLock<std::sync::Arc<tokio::sync::Mutex<ChangeLog>>> =
    std::sync::LazyLock::new(|| {
        #[cfg(test)]
        let _ = fs::remove_file(ChangeLog::caminho_do_dono());
        std::sync::Arc::new(tokio::sync::Mutex::new(ChangeLog::carregar_do_disco()))
    });

/// A trava do histórico. No código assíncrono: `historico().lock().await` (ou `lock_owned()` para levar a guarda a um
/// `spawn_blocking`).
pub fn historico() -> std::sync::Arc<tokio::sync::Mutex<ChangeLog>> {
    HISTORICO.clone()
}

/// Para código SÍNCRONO fora do runtime (`spawn_blocking`, threads próprias, o desinstalador): espera a trava e empresta
/// o histórico. Não chame de dentro de uma função `async` (o tokio entra em pânico) nem com a trava já presa no mesmo
/// caminho (esperaria por ela para sempre): nesses casos, quem segura a trava passa o `&mut ChangeLog` adiante.
pub fn com_historico<T>(f: impl FnOnce(&mut ChangeLog) -> T) -> T {
    let trava = historico();
    let mut log = trava.blocking_lock();
    f(&mut log)
}

/// Uma cópia SÓ DE LEITURA do histórico, para quem só consulta (o painel do Frame Boost, o teto gravado, o relatório
/// de suporte). Não grava: `Deref` dá só `&ChangeLog`, e gravar pede `&mut`. Com a trava livre, a cópia vem da memória;
/// presa (alguém gravando agora, ou o próprio chamador segurando a trava mais acima), vem da última versão confirmada
/// no disco, que toda gravação deixa inteira (temporário + `rename`). Nunca espera a trava, então serve de qualquer
/// lugar, até de dentro de quem a segura.
pub struct FotoDoHistorico(ChangeLog);

impl std::ops::Deref for FotoDoHistorico {
    type Target = ChangeLog;
    fn deref(&self) -> &ChangeLog {
        &self.0
    }
}

pub fn foto() -> FotoDoHistorico {
    let trava = historico();
    let copia = match trava.try_lock() {
        Ok(log) => log.copia(),
        Err(_) => {
            // Sem `guardar_ilegivel`: quem só lê não move o arquivo do dono.
            let path = ChangeLog::caminho_do_dono();
            let (entries, leitura) = ChangeLog::ler_sem_mexer(&path);
            ChangeLog {
                path,
                entries,
                leitura,
            }
        }
    };
    FotoDoHistorico(copia)
}

impl ChangeLog {
    fn copia(&self) -> ChangeLog {
        ChangeLog {
            path: self.path.clone(),
            entries: self.entries.clone(),
            leitura: self.leitura.clone(),
        }
    }

    /// Não devolve erro, para não travar o app, mas não finge que ilegível é vazio (ver `LeituraDoHistorico`). Privado:
    /// só o dono (`HISTORICO`) e a cópia de leitura (`foto`) carregam do disco, e a cópia não grava.
    fn carregar_do_disco() -> Self {
        let path = Self::caminho_do_dono();
        let (entries, leitura) = Self::ler(&path);

        ChangeLog {
            path,
            entries,
            leitura,
        }
    }

    /// O `changes.json` DE VERDADE desta máquina, fora do dono: só para os testes `#[ignore]` que mexem no PC de quem
    /// roda e precisam desfazer o que o app gravou. Com o Otimiza fechado.
    #[cfg(test)]
    pub(crate) fn do_disco_real() -> Self {
        let base = std::env::var("APPDATA")
            .or_else(|_| std::env::var("HOME"))
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));
        let path = base.join("pc-optimizer").join("changes.json");
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
        Self::ler_e_guardar(path, true)
    }

    /// A leitura da cópia de leitura (`foto`): ilegível é dito, mas o arquivo fica onde está (só o dono o tira do
    /// caminho, antes de gravar por cima).
    fn ler_sem_mexer(path: &Path) -> (Vec<AppliedOptimization>, LeituraDoHistorico) {
        Self::ler_e_guardar(path, false)
    }

    fn ler_e_guardar(path: &Path, guardar: bool) -> (Vec<AppliedOptimization>, LeituraDoHistorico) {
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
                let guardado = if guardar { Self::guardar_ilegivel(path) } else { None };

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

    #[cfg(test)]
    fn caminho_do_dono() -> PathBuf {
        std::env::temp_dir().join(format!("pc-optimizer-teste-dono-{}.json", std::process::id()))
    }

    #[cfg(not(test))]
    fn caminho_do_dono() -> PathBuf {
        let base = std::env::var("APPDATA")
            .or_else(|_| std::env::var("HOME"))
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));

        base.join("pc-optimizer").join("changes.json")
    }

    fn persist(&self) -> Result<(), ErroDoHistorico> {
        Self::gravar(&self.path, &self.entries)
    }

    /// ATÔMICA: um JSON truncado por queda de energia era lido como vazio, e "Desfazer tudo" dizia que não havia
    /// nada com as mudanças ainda aplicadas.
    fn gravar(path: &Path, entries: &[AppliedOptimization]) -> Result<(), ErroDoHistorico> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(ErroDoHistorico::Pasta)?;
        }

        let raw = serde_json::to_string_pretty(entries).map_err(ErroDoHistorico::Serializar)?;

        let temporario = caminho_temporario(path);

        // `sync_all` antes do rename: `fs::write` volta com o conteúdo no cache, e a queda podia gravar o rename sem o
        // conteúdo.
        {
            use std::io::Write;

            let escrever = || -> std::io::Result<()> {
                let mut arquivo = fs::File::create(&temporario)?;
                arquivo.write_all(raw.as_bytes())?;
                arquivo.sync_all()
            };
            if let Err(e) = escrever() {
                let _ = fs::remove_file(&temporario);
                return Err(ErroDoHistorico::Gravar(e));
            }
        }

        fs::rename(&temporario, path).map_err(|e| {
            let _ = fs::remove_file(&temporario);
            ErroDoHistorico::Trocar(e)
        })
    }

    /// Muda dois registros em memória e grava UMA vez: sem estado intermediário no disco. `false` se um dos dois falta.
    pub fn editar_dois(
        &mut self,
        a: &str,
        b: &str,
        f: impl FnOnce(&mut AppliedOptimization, &mut AppliedOptimization),
    ) -> Result<bool, ErroDoHistorico> {
        let (Some(i), Some(j)) = (
            self.entries.iter().position(|e| e.optimization_id == a),
            self.entries.iter().position(|e| e.optimization_id == b),
        ) else {
            return Ok(false);
        };
        if i == j {
            return Ok(false);
        }
        let (menor, maior) = (i.min(j), i.max(j));
        let (esquerda, direita) = self.entries.split_at_mut(maior);
        let (x, y) = (&mut esquerda[menor], &mut direita[0]);
        if i < j {
            f(x, y);
        } else {
            f(y, x);
        }
        self.persist()?;
        Ok(true)
    }

    /// Substitui o registro anterior da mesma otimização, para guardar o estado original mais antigo ainda não
    /// revertido.
    pub fn record(&mut self, entry: AppliedOptimization) -> Result<(), ErroDoHistorico> {
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

    pub fn take(&mut self, optimization_id: &str) -> Result<Option<AppliedOptimization>, ErroDoHistorico> {
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

    /// A cópia de leitura diz que está ilegível, mas não move o arquivo: só o dono tira do caminho.
    #[test]
    fn quem_so_le_nao_move_o_arquivo_ilegivel() {
        let caminho = pasta("ilegivel-so-le").join("changes.json");
        fs::write(&caminho, "{ truncado").unwrap();

        let (entradas, leitura) = ChangeLog::ler_sem_mexer(&caminho);

        assert!(entradas.is_empty());
        assert!(matches!(leitura, LeituraDoHistorico::Ilegivel { guardado_em: None, .. }));
        assert!(caminho.exists(), "a leitura moveu o arquivo do dono");
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

/// O dono único (1.0.12, A1): duas gravações ao mesmo tempo não apagam uma à outra, e quem só lê nunca grava.
#[cfg(test)]
mod testes_do_dono {
    use super::*;

    fn registro(id: &str) -> AppliedOptimization {
        AppliedOptimization {
            optimization_id: id.to_string(),
            name: "Teste do dono".to_string(),
            timestamp: 0,
            changes: vec![ChangeRecord::PowerPlan {
                previous_guid: id.to_string(),
            }],
        }
    }

    /// O caso da 1.0.9: o Frame Boost gravava (Smooth Motion) numa cópia lida do disco enquanto um vigia gravava a
    /// memória do app; o último a gravar apagava o registro do outro. Pelo dono, as duas threads esperam a vez e as
    /// duas mudanças ficam, na memória E no disco.
    #[test]
    fn duas_escritas_ao_mesmo_tempo_nao_se_apagam() {
        let ids: Vec<String> = (0..16).map(|i| format!("dono-concorrente-{}", i)).collect();
        let largada = std::sync::Arc::new(std::sync::Barrier::new(ids.len()));
        let threads: Vec<_> = ids
            .iter()
            .cloned()
            .map(|id| {
                let largada = largada.clone();
                std::thread::spawn(move || {
                    largada.wait();
                    com_historico(|log| log.record(registro(&id))).expect("gravar pelo dono");
                })
            })
            .collect();
        for t in threads {
            t.join().expect("a thread de gravação caiu");
        }

        com_historico(|log| {
            for id in &ids {
                assert!(log.is_applied(id), "{} sumiu da memória", id);
            }
        });
        let (no_disco, leitura) = ChangeLog::ler(&ChangeLog::caminho_do_dono());
        assert_eq!(leitura, LeituraDoHistorico::Ok);
        for id in &ids {
            assert!(
                no_disco.iter().any(|e| &e.optimization_id == id),
                "{} sumiu do disco: a gravação de outra thread passou por cima",
                id
            );
        }
        com_historico(|log| {
            for id in &ids {
                log.take(id).expect("desfazer pelo dono");
            }
        });
    }

    /// O mesmo, com o lado assíncrono (um comando segurando a trava e levando a guarda para um `spawn_blocking`) e um
    /// vigia síncrono querendo gravar no meio: o vigia espera, e nenhum registro se perde.
    #[test]
    fn o_comando_com_a_trava_e_o_vigia_esperam_a_vez() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("runtime de teste");
        rt.block_on(async {
            let guarda = historico().lock_owned().await;
            let vigia = std::thread::spawn(|| {
                com_historico(|log| log.record(registro("dono-vigia"))).expect("o vigia grava pelo dono");
            });
            let comando = tokio::task::spawn_blocking(move || {
                let mut log = guarda;
                // Enquanto o comando segura a trava, o vigia não consegue gravar.
                std::thread::sleep(std::time::Duration::from_millis(100));
                assert!(!log.is_applied("dono-vigia"), "o vigia gravou sem a trava");
                log.record(registro("dono-comando")).expect("o comando grava pelo dono");
            });
            comando.await.expect("o comando caiu");
            tokio::task::spawn_blocking(move || vigia.join().expect("o vigia caiu"))
                .await
                .expect("esperar o vigia");
        });
        com_historico(|log| {
            assert!(log.is_applied("dono-comando"));
            assert!(log.is_applied("dono-vigia"));
            log.take("dono-comando").expect("desfazer");
            log.take("dono-vigia").expect("desfazer");
        });
    }

    /// Quem só lê (o painel do Frame Boost, chamado de dentro de quem segura a trava) não espera a trava: lê a última
    /// versão gravada no disco, e não trava para sempre.
    #[test]
    fn a_foto_nao_espera_a_trava_e_ve_o_que_foi_gravado() {
        com_historico(|log| log.record(registro("dono-foto"))).expect("gravar");
        let trava = historico();
        let presa = trava.blocking_lock();
        let foto = foto();
        assert!(foto.is_applied("dono-foto"), "com a trava presa, a foto vem do disco");
        drop(presa);
        assert!(super::foto().is_applied("dono-foto"), "com a trava livre, a foto vem da memória");
        com_historico(|log| log.take("dono-foto")).expect("desfazer");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fila_do_frame_boost_aparece_no_historico_com_o_jogo() {
        let c = ChangeRecord::FilaNvidia {
            executavel: "FiveM_GTAProcess.exe".to_string(),
            valor_anterior: "0".to_string(),
            caminho: None,
        };
        assert!(c.describe().contains("fila de 1 quadro em FiveM_GTAProcess.exe"));
        assert_eq!(crate::modules::windows::nvdriver::id_da_fila("FiveM_GTAProcess.exe"), "fila_nvidia:fivem_gtaprocess.exe");
        // Registro gravado sem o caminho (formato curto) volta a ser lido.
        let json = serde_json::to_string(&c).unwrap();
        assert!(!json.contains("caminho"));
        let _: ChangeRecord = serde_json::from_str(&json).unwrap();
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
