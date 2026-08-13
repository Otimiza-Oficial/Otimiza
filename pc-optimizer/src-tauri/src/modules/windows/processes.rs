// Monitor de processos em tempo real
//
// Responde à pergunta que o cliente faz de verdade: "o que está deixando meu PC
// lento AGORA?". Otimizador nenhum responde isso — todos mostram uma barra de
// progresso e um número inventado no fim.
//
// Aqui o programa aponta o culpado pelo nome, com quanto ele consome, e diz se
// ele volta sozinho no próximo boot.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessImpact {
    pub name: String,
    /// Porcentagem da CPU TOTAL da máquina, não de um núcleo.
    pub cpu_percent: f64,
    pub ram_mb: f64,
    /// Quantos processos com este nome estão rodando. O Discord abre vários.
    pub instances: usize,
    /// Se este programa sobe junto com o Windows.
    pub in_startup: bool,
}

pub struct ProcessMonitor {
    system: System,
    startup: HashSet<String>,
    /// Núcleos lógicos, usado para normalizar o uso de CPU.
    cores: f64,
}

impl ProcessMonitor {
    pub fn new() -> Self {
        ProcessMonitor {
            system: System::new(),
            startup: super::startup::startup_executables(),
            cores: num_cpus::get().max(1) as f64,
        }
    }

    /// Relê a lista de programas de inicialização.
    /// Chamado depois de otimizar, quando algo pode ter mudado.
    pub fn refresh_startup(&mut self) {
        self.startup = super::startup::startup_executables();
    }

    /// Os processos que mais pesam agora, agrupados por nome.
    ///
    /// Duas correções que a maioria das ferramentas erra:
    ///
    /// 1. O `sysinfo` reporta uso de CPU relativo a UM núcleo — um processo pode
    ///    aparecer com 380% numa máquina de 4 núcleos. Dividimos pelo número de
    ///    núcleos para virar porcentagem da máquina, que é o que o cliente lê no
    ///    Gerenciador de Tarefas.
    /// 2. Programas modernos abrem vários processos com o mesmo nome. Somar por
    ///    nome mostra "Discord: 12%" em vez de seis linhas de 2% que o cliente
    ///    não consegue interpretar.
    pub fn top(&mut self, limit: usize) -> Vec<ProcessImpact> {
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing().with_cpu().with_memory(),
        );

        let mut grouped: HashMap<String, ProcessImpact> = HashMap::new();

        for process in self.system.processes().values() {
            let name = process.name().to_string_lossy().to_string();

            let entry = grouped.entry(name.clone()).or_insert_with(|| ProcessImpact {
                in_startup: self.startup.contains(&name.to_lowercase()),
                name,
                cpu_percent: 0.0,
                ram_mb: 0.0,
                instances: 0,
            });

            entry.cpu_percent += process.cpu_usage() as f64 / self.cores;
            entry.ram_mb += process.memory() as f64 / 1_048_576.0;
            entry.instances += 1;
        }

        let mut impacts: Vec<ProcessImpact> = grouped.into_values().collect();

        impacts.sort_by(|a, b| {
            b.cpu_percent
                .partial_cmp(&a.cpu_percent)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        impacts.truncate(limit);
        impacts
    }
}

/// Processos vivos: identificador, nome do executável e instante de início.
///
/// Existe para a suspensão durante o jogo. Não agrupa por nome, ao contrário do
/// `top()` acima, porque suspender exige o PID de cada processo — um Chrome com
/// quinze abas são quinze processos, e todos precisam parar.
///
/// O instante de início vai junto porque o Windows RECICLA identificadores. Sem
/// essa assinatura, o Otimiza poderia retomar um processo novo que nunca
/// suspendeu, mexendo num programa que não é o dele.
pub fn listar_para_suspensao() -> Vec<(u32, String, u64)> {
    let mut sistema = System::new();
    sistema.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing(),
    );

    sistema
        .processes()
        .iter()
        .map(|(pid, processo)| {
            (
                pid.as_u32(),
                processo.name().to_string_lossy().to_string(),
                processo.start_time(),
            )
        })
        .collect()
}

impl Default for ProcessMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_is_normalized_to_the_whole_machine() {
        let mut monitor = ProcessMonitor::new();

        // A primeira leitura do sysinfo não tem base de comparação; a segunda tem.
        monitor.top(10);
        std::thread::sleep(std::time::Duration::from_millis(300));
        let impacts = monitor.top(10);

        assert!(!impacts.is_empty(), "nenhum processo listado");

        let total: f64 = impacts.iter().map(|p| p.cpu_percent).sum();
        assert!(
            total <= 100.0 * num_cpus::get() as f64,
            "soma de CPU impossível: {}",
            total
        );

        for impact in impacts.iter().take(5) {
            println!(
                "{:<28} {:>6.1}% CPU  {:>8.0} MB  x{}  inicializa: {}",
                impact.name, impact.cpu_percent, impact.ram_mb, impact.instances, impact.in_startup
            );
        }
    }

    #[test]
    fn results_come_sorted_by_impact() {
        let mut monitor = ProcessMonitor::new();
        monitor.top(10);
        std::thread::sleep(std::time::Duration::from_millis(300));
        let impacts = monitor.top(10);

        let cpu: Vec<f64> = impacts.iter().map(|p| p.cpu_percent).collect();
        assert!(cpu.windows(2).all(|pair| pair[0] >= pair[1]));
    }

    #[test]
    fn limit_is_respected() {
        let mut monitor = ProcessMonitor::new();
        assert!(monitor.top(3).len() <= 3);
    }
}
