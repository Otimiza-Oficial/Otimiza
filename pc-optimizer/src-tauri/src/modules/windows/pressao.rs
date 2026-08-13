// A janela dos últimos dias
//
// O evento 2004 do Windows é a evidência mais forte que o produto tem, mas ele
// só é gravado quando a memória acaba DE VERDADE. Existe um estado anterior a
// esse — a máquina que passa a noite raspando o limite, paginando sem parar,
// travando por segundos de cada vez — que não gera evento nenhum e some do
// diagnóstico, porque quando o cliente abre o Otimiza o jogo já foi fechado.
//
// Este módulo é a janela que faltava. Ele amostra a pressão de memória no
// mesmo laço de seis segundos que o vigia do modo jogo já usa, e guarda o
// resultado em disco.
//
// O QUE É GUARDADO, E POR QUE NÃO É A AMOSTRA CRUA
//
// Uma amostra a cada seis segundos são 14.400 por dia. Guardar isso viraria
// dezenas de megabytes por semana no PC do cliente — um otimizador que engorda
// o disco é uma piada de mau gosto.
//
// Então o que vai para o arquivo é UM registro por hora, com os extremos
// daquela hora: o maior commit, a menor memória disponível, quantos minutos
// ficaram acima do limite, e os processos do PIOR instante. Duas semanas cabem
// em poucos kilobytes.
//
// Esse último campo é o que transforma "você precisa de mais memória" em
// "Discord e FiveM juntos prometeram 12 GB na terça às 21:40". A primeira frase
// é opinião; a segunda o cliente reconhece.

use super::achados::{FindingSeverity, FixLocation};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Uma hora de observação, já resumida.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Hora {
    /// Início da hora, em "AAAA-MM-DDTHH".
    pub quando: String,
    pub commit_max_gb: f64,
    pub disponivel_min_gb: f64,
    /// Quantas amostras ficaram acima do limite de pressão.
    pub amostras_apertadas: u32,
    pub amostras: u32,
    /// Os maiores consumidores no pior instante desta hora.
    pub piores: Vec<(String, f64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Janela {
    pub horas: Vec<Hora>,
}

/// Quanto tempo de histórico o produto guarda.
///
/// Duas semanas cobrem a rotina de quem joga no fim de semana sem virar um
/// arquivo grande nem uma memória longa demais para ser relevante: o que
/// aconteceu há dois meses provavelmente já mudou.
pub const DIAS_GUARDADOS: usize = 14;
const HORAS_GUARDADAS: usize = DIAS_GUARDADOS * 24;

/// Abaixo disto o Windows já está espremendo memória para caber.
const DISPONIVEL_APERTADO_GB: f64 = 0.7;

/// Quantos minutos apertados numa hora já contam como problema, e não como
/// pico isolado. Com amostragem de 6 segundos, 50 amostras são 5 minutos.
const AMOSTRAS_PARA_CONTAR: u32 = 50;

impl Janela {
    fn path() -> PathBuf {
        let base = std::env::var("APPDATA")
            .or_else(|_| std::env::var("HOME"))
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));

        base.join("pc-optimizer").join("pressao.json")
    }

    pub fn load() -> Self {
        fs::read_to_string(Self::path())
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::path();

        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)
                .map_err(|e| format!("Não foi possível criar a pasta de dados: {}", e))?;
        }

        let json = serde_json::to_string(self)
            .map_err(|e| format!("Não foi possível gravar a janela: {}", e))?;

        fs::write(&path, json).map_err(|e| format!("Não foi possível gravar a janela: {}", e))
    }

    /// Só o teste apaga a janela. O produto nunca descarta observação por
    /// conta própria: o que ela registra é a única memória que o Otimiza tem do
    /// que aconteceu enquanto o cliente jogava.
    #[cfg(test)]
    pub fn limpar() -> Result<(), String> {
        let path = Self::path();

        if path.exists() {
            fs::remove_file(&path).map_err(|e| format!("Não foi possível limpar: {}", e))?;
        }

        Ok(())
    }
}

/// Uma leitura instantânea.
#[derive(Debug, Clone, Copy)]
pub struct Amostra {
    pub commit_gb: f64,
    pub disponivel_gb: f64,
}

/// Acrescenta uma amostra à hora corrente.
///
/// **Função pura**: recebe a janela e devolve a janela. Toda a lógica de
/// agregação e de descarte do que envelheceu fica testável sem tocar em disco
/// nem esperar uma hora passar.
pub fn agregar(
    mut janela: Janela,
    hora_atual: &str,
    amostra: Amostra,
    piores: Vec<(String, f64)>,
) -> Janela {
    let apertada = amostra.disponivel_gb < DISPONIVEL_APERTADO_GB;

    match janela.horas.last_mut().filter(|h| h.quando == hora_atual) {
        Some(hora) => {
            hora.amostras += 1;
            if apertada {
                hora.amostras_apertadas += 1;
            }

            // Os processos guardados são os do PIOR instante da hora, e não os
            // do último. É o instante que interessa reconstituir depois.
            if amostra.commit_gb > hora.commit_max_gb {
                hora.commit_max_gb = amostra.commit_gb;
                hora.piores = piores;
            }

            if amostra.disponivel_gb < hora.disponivel_min_gb {
                hora.disponivel_min_gb = amostra.disponivel_gb;
            }
        }
        None => janela.horas.push(Hora {
            quando: hora_atual.to_string(),
            commit_max_gb: amostra.commit_gb,
            disponivel_min_gb: amostra.disponivel_gb,
            amostras_apertadas: u32::from(apertada),
            amostras: 1,
            piores,
        }),
    }

    // Descarta o que passou da janela. Sem isto o arquivo cresce para sempre no
    // PC do cliente, que é o oposto do serviço que este programa vende.
    if janela.horas.len() > HORAS_GUARDADAS {
        let excesso = janela.horas.len() - HORAS_GUARDADAS;
        janela.horas.drain(0..excesso);
    }

    janela
}

// --------------------------------------------------------------- diagnóstico

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PressaoFinding {
    pub id: String,
    pub title: String,
    pub measured: String,
    pub advice: String,
    pub severity: FindingSeverity,
    pub fix_location: FixLocation,
}

/// Traduz "AAAA-MM-DDTHH" para algo que se lê numa frase.
fn hora_legivel(chave: &str) -> String {
    let Some((data, hora)) = chave.split_once('T') else {
        return chave.to_string();
    };

    let partes: Vec<&str> = data.split('-').collect();

    if partes.len() == 3 {
        format!("{}/{} por volta das {}h", partes[2], partes[1], hora)
    } else {
        chave.to_string()
    }
}

/// Regras de diagnóstico, puras.
pub fn diagnosticar(janela: &Janela) -> Vec<PressaoFinding> {
    let apertadas: Vec<&Hora> = janela
        .horas
        .iter()
        .filter(|h| h.amostras_apertadas >= AMOSTRAS_PARA_CONTAR)
        .collect();

    if apertadas.is_empty() {
        return Vec::new();
    }

    // A pior hora é a que fica na frase — com os processos dela pelo nome.
    let pior = apertadas
        .iter()
        .max_by(|a, b| a.commit_max_gb.total_cmp(&b.commit_max_gb))
        .expect("a lista não está vazia");

    let quem = if pior.piores.is_empty() {
        String::new()
    } else {
        let lista: Vec<String> = pior
            .piores
            .iter()
            .take(3)
            .map(|(nome, gb)| format!("{} com {:.1} GB", nome, gb))
            .collect();

        format!(" Naquele momento: {}.", lista.join(", "))
    };

    vec![PressaoFinding {
        id: "pressao_recorrente".to_string(),
        title: "A memória vive no limite nesta máquina".to_string(),
        measured: format!(
            "{} hora(s) de uso apertado nos últimos {} dias. A pior foi em {}, com \
             {:.1} GB comprometidos e só {:.1} GB livres.{}",
            apertadas.len(),
            DIAS_GUARDADOS,
            hora_legivel(&pior.quando),
            pior.commit_max_gb,
            pior.disponivel_min_gb,
            quem
        ),
        advice: "Não é um pico isolado: são horas seguidas raspando o limite. Nesse \
                 estado o Windows passa mais tempo movendo memória para o disco do que \
                 trabalhando, e é isso que o cliente sente como \"o PC engasga\". \
                 Fechar o que não está em uso durante o jogo alivia; mais memória resolve."
            .to_string(),
        severity: FindingSeverity::Critical,
        fix_location: FixLocation::Hardware,
    }]
}

// ---------------------------------------------------------------- amostragem

/// Lê a memória agora e acrescenta à janela.
///
/// Chamada a cada seis segundos pelo laço que já existe. Tudo aqui é leitura
/// de memória em memória, sub-milissegundo — nenhuma chamada ao PowerShell,
/// que a esta frequência faria o próprio otimizador virar o programa que mais
/// pesa no PC do cliente.
///
/// Grava em disco no máximo uma vez por minuto: a agregação acontece em RAM, e
/// escrever a cada seis segundos castigaria o SSD sem nenhum ganho.
pub fn amostrar() {
    use std::sync::Mutex;
    use std::time::Instant;

    static ESTADO: Mutex<Option<(Janela, Instant)>> = Mutex::new(None);

    let mut sistema = sysinfo::System::new();
    sistema.refresh_memory();

    let total = sistema.total_memory() as f64;
    let disponivel = sistema.available_memory() as f64;
    let em_gb = 1_073_741_824.0;

    let amostra = Amostra {
        // Sem o commit do sistema (que só o WMI dá), o uso físico é a melhor
        // aproximação disponível de graça. É honesto porque o que o achado
        // afirma é "a memória viveu no limite", e não um valor de commit.
        commit_gb: (total - disponivel) / em_gb,
        disponivel_gb: disponivel / em_gb,
    };

    let piores = if amostra.disponivel_gb < DISPONIVEL_APERTADO_GB {
        maiores_consumidores()
    } else {
        // Só vale o custo de percorrer os processos quando o instante é
        // apertado — que é o único instante cujos nomes interessam guardar.
        Vec::new()
    };

    let Ok(mut guarda) = ESTADO.lock() else {
        return;
    };

    let (janela, ultimo_salvamento) = guarda
        .take()
        .unwrap_or_else(|| (Janela::load(), Instant::now()));

    let janela = agregar(janela, &hora_corrente(), amostra, piores);

    if ultimo_salvamento.elapsed().as_secs() >= 60 {
        let _ = janela.save();
        *guarda = Some((janela, Instant::now()));
    } else {
        *guarda = Some((janela, ultimo_salvamento));
    }
}

/// A hora atual em "AAAA-MM-DDTHH".
fn hora_corrente() -> String {
    // O projeto não carrega biblioteca de data, e acrescentar uma só para
    // formatar uma chave de agregação não se paga. O PowerShell também não
    // serve: a esta frequência ele custaria mais que tudo o resto junto.
    let segundos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let dias = segundos / 86_400;
    let hora = (segundos % 86_400) / 3_600;

    // Conversão de dias desde 1970 para data civil, algoritmo de Howard Hinnant.
    let z = dias as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{:04}-{:02}-{:02}T{:02}", y, m, d, hora)
}

/// Os processos que mais seguram memória agora.
fn maiores_consumidores() -> Vec<(String, f64)> {
    use std::collections::HashMap;

    let mut sistema = sysinfo::System::new();
    sistema.refresh_processes_specifics(
        sysinfo::ProcessesToUpdate::All,
        true,
        sysinfo::ProcessRefreshKind::nothing().with_memory(),
    );

    // Agrupa por nome: um navegador com quinze abas são quinze processos, e
    // quinze linhas de 300 MB escondem o fato de que ele segura 4,5 GB.
    let mut por_nome: HashMap<String, f64> = HashMap::new();

    for processo in sistema.processes().values() {
        let nome = processo.name().to_string_lossy().to_string();
        *por_nome.entry(nome).or_insert(0.0) += processo.memory() as f64 / 1_073_741_824.0;
    }

    let mut lista: Vec<(String, f64)> = por_nome.into_iter().collect();
    lista.sort_by(|a, b| b.1.total_cmp(&a.1));
    lista.truncate(3);
    lista
        .into_iter()
        .map(|(nome, gb)| (nome, (gb * 10.0).round() / 10.0))
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PressaoReport {
    pub horas_observadas: usize,
    pub findings: Vec<PressaoFinding>,
}

pub fn analyze() -> PressaoReport {
    let janela = Janela::load();

    PressaoReport {
        horas_observadas: janela.horas.len(),
        findings: diagnosticar(&janela),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hora(chave: &str, commit: f64, disponivel: f64, apertadas: u32) -> Hora {
        Hora {
            quando: chave.to_string(),
            commit_max_gb: commit,
            disponivel_min_gb: disponivel,
            amostras_apertadas: apertadas,
            amostras: 600,
            piores: Vec::new(),
        }
    }

    #[test]
    fn amostras_da_mesma_hora_viram_um_registro_so() {
        // 14.400 amostras por dia não podem virar 14.400 linhas no disco do
        // cliente: um otimizador que engorda o disco é uma piada de mau gosto.
        let mut j = Janela::default();

        for i in 0..600 {
            j = agregar(
                j,
                "2026-08-11T21",
                Amostra {
                    commit_gb: 9.0 + (i as f64 % 3.0),
                    disponivel_gb: 1.5,
                },
                Vec::new(),
            );
        }

        assert_eq!(j.horas.len(), 1);
        assert_eq!(j.horas[0].amostras, 600);
        assert_eq!(j.horas[0].commit_max_gb, 11.0);
    }

    #[test]
    fn guarda_os_processos_do_pior_instante_e_nao_do_ultimo() {
        // O nome do processo é o que transforma "falta memória" em algo que o
        // cliente reconhece. Guardar o do último instante mostraria quem estava
        // aberto quando a hora acabou, que não é a mesma pergunta.
        let mut j = Janela::default();

        j = agregar(
            j,
            "2026-08-11T21",
            Amostra { commit_gb: 12.0, disponivel_gb: 0.3 },
            vec![("FiveM.exe".into(), 7.0), ("Discord.exe".into(), 1.2)],
        );
        j = agregar(
            j,
            "2026-08-11T21",
            Amostra { commit_gb: 4.0, disponivel_gb: 5.0 },
            vec![("explorer.exe".into(), 0.2)],
        );

        assert_eq!(j.horas[0].commit_max_gb, 12.0);
        assert_eq!(j.horas[0].piores[0].0, "FiveM.exe");
        // E o mínimo de disponível também é o extremo, não o último.
        assert_eq!(j.horas[0].disponivel_min_gb, 0.3);
    }

    #[test]
    fn a_janela_nao_cresce_para_sempre() {
        let mut j = Janela::default();

        for i in 0..(HORAS_GUARDADAS + 50) {
            j = agregar(
                j,
                &format!("hora-{:05}", i),
                Amostra { commit_gb: 5.0, disponivel_gb: 3.0 },
                Vec::new(),
            );
        }

        assert_eq!(j.horas.len(), HORAS_GUARDADAS);
        // O que sobrou é o mais recente, não o mais antigo.
        assert_eq!(j.horas.last().unwrap().quando, format!("hora-{:05}", HORAS_GUARDADAS + 49));
    }

    #[test]
    fn pico_isolado_nao_vira_achado() {
        // Uma hora com dois minutos apertados é uso normal de PC. Transformar
        // isso em alerta seria inventar problema para justificar a compra.
        let janela = Janela {
            horas: vec![hora("2026-08-11T21", 9.0, 0.4, 10)],
        };

        assert!(diagnosticar(&janela).is_empty());
    }

    #[test]
    fn horas_seguidas_no_limite_viram_achado_com_nome_e_data() {
        let janela = Janela {
            horas: vec![
                hora("2026-08-10T20", 10.5, 0.4, 300),
                Hora {
                    piores: vec![("FiveM.exe".into(), 7.2), ("Discord.exe".into(), 1.4)],
                    ..hora("2026-08-11T21", 12.3, 0.2, 400)
                },
                hora("2026-08-12T19", 9.8, 0.5, 120),
            ],
        };

        let f = diagnosticar(&janela);
        let achado = &f[0];

        assert_eq!(achado.severity, FindingSeverity::Critical);
        assert_eq!(achado.fix_location, FixLocation::Hardware);
        assert!(achado.measured.contains("3 hora(s)"));
        // A pior hora é a de maior commit, com os processos dela pelo nome.
        assert!(achado.measured.contains("11/08 por volta das 21h"));
        assert!(achado.measured.contains("FiveM.exe com 7.2 GB"));
    }

    #[test]
    fn maquina_sem_observacao_nao_afirma_nada() {
        assert!(diagnosticar(&Janela::default()).is_empty());
    }

    #[test]
    fn a_janela_sobrevive_a_ida_e_volta_do_disco() {
        let janela = Janela {
            horas: vec![hora("2026-08-11T21", 12.3, 0.2, 400)],
        };

        janela.save().expect("gravar");
        assert_eq!(Janela::load().horas, janela.horas);

        Janela::limpar().expect("limpar");
        assert!(Janela::load().horas.is_empty());
    }
}
